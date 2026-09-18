//! Nonlinear initial-state fitting with known mechanics and sensor geometry.
pub mod loss_profile;
pub mod robustness;
use super::*;

pub const HELP: &str = "Nonlinear magnetic state study:
  magnetic-state --output REPORT.json
Known mechanical losses and sensor geometry; one continuous initial state, analytic Jacobian.
Three fixed starts, training-only selection, held-out prediction and centered sign ambiguity.
";
const STARTS: [f64; 3] = [0.5, 1.0, 1.5];
const ITERATIONS: usize = 40;

impl Sensor {
    fn gradient(self, x: f64, v: f64) -> [f64; 2] {
        let z = (self.offset_m + x) / self.gap_m;
        let (a, p) = if self.law == "production" {
            (0.015, 1.5)
        } else {
            (0.045, 2.5)
        };
        let dx = a / (self.gap_m * self.gap_m) * (1.0 + (1.0 - 2.0 * p) * z * z)
            / (1.0 + z * z).powf(p + 1.0)
            * v;
        [dx, self.voltage(x, 1.0)]
    }
}

fn power(step: &Dense, mut exponent: usize) -> Dense {
    let mut result: Dense = (0..step.len())
        .map(|i| (0..step.len()).map(|j| f64::from(i == j)).collect())
        .collect();
    let mut base = step.clone();
    while exponent > 0 {
        if exponent % 2 == 1 {
            result = multiply(&result, &base);
        }
        exponent /= 2;
        if exponent > 0 {
            base = multiply(&base, &base);
        }
    }
    result
}
fn row_times(row: &[f64], m: &Dense) -> Vec<f64> {
    (0..row.len())
        .map(|j| row.iter().enumerate().map(|(i, x)| x * m[i][j]).sum())
        .collect()
}
struct Window {
    displacement: Dense,
    velocity: Dense,
}
struct Templates {
    windows: [Window; 2],
    off: Observer,
    on: Observer,
    event: Dense,
}
fn templates(
    s: &ModalSpectrum,
    c0: &Matrix,
    d: &Matrix,
    alpha: f64,
    beta: f64,
    rate: u32,
) -> Result<Templates, Box<dyn Error>> {
    let c = core::array::from_fn(|i| core::array::from_fn(|j| alpha * c0[i][j]));
    let cd = core::array::from_fn(|i| core::array::from_fn(|j| c[i][j] + beta * d[i][j]));
    let off = Observer::prepare(s, &c, 9, rate)?;
    let on = Observer::prepare(s, &cd, 9, rate)?;
    let event = power(&off.step, (0.12 * f64::from(rate)).round() as usize);
    let count = (0.08 * f64::from(rate)).round() as usize;
    let port_q: Vec<_> = (0..18)
        .map(|i| {
            if i < 9 {
                s.modes[i].pickup_weight / (TAU * s.modes[i].frequency_hz)
            } else {
                0.0
            }
        })
        .collect();
    let windows = core::array::from_fn(|index| {
        let operator = if index == 0 { &off } else { &on };
        let mut q = port_q.clone();
        let mut v = operator.port.clone();
        let mut window = Window {
            displacement: Vec::with_capacity(count),
            velocity: Vec::with_capacity(count),
        };
        for _ in 0..count {
            window.displacement.push(if index == 0 {
                q.clone()
            } else {
                row_times(&q, &event)
            });
            window.velocity.push(if index == 0 {
                v.clone()
            } else {
                row_times(&v, &event)
            });
            q = row_times(&q, &operator.step);
            v = row_times(&v, &operator.step);
        }
        window
    });
    Ok(Templates {
        windows,
        off,
        on,
        event,
    })
}

// Training owns only selected observations and their operator rows. No truth,
// held-out voltage, optimizer start chosen from state, or fitted-loss truth.
#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Weighting {
    RelativeWindows,
    ConstantVoltage,
}

struct Training {
    q: Dense,
    v: Dense,
    y: Vec<f64>,
    weights: Vec<f64>,
}
impl Training {
    fn new(t: &Templates, observations: &[Vec<f64>; 2]) -> Result<Self, Box<dyn Error>> {
        Self::weighted(t, observations, Weighting::RelativeWindows)
    }
    fn weighted(
        t: &Templates,
        observations: &[Vec<f64>; 2],
        weighting: Weighting,
    ) -> Result<Self, Box<dyn Error>> {
        let mut data = Self {
            q: Vec::new(),
            v: Vec::new(),
            y: Vec::new(),
            weights: Vec::new(),
        };
        for (window, y) in t.windows.iter().zip(observations) {
            if y.is_empty() || y.len() > window.velocity.len() || y.iter().any(|v| !v.is_finite()) {
                return Err("invalid nonlinear training observations".into());
            }
            let norm = dot(y, y).sqrt();
            if !norm.is_finite() || norm <= 0.0 {
                return Err("unexcited nonlinear training window".into());
            }
            for (i, value) in y.iter().enumerate() {
                data.q.push(window.displacement[i].clone());
                data.v.push(window.velocity[i].clone());
                data.y.push(*value);
                data.weights.push(1.0 / (2.0_f64.sqrt() * norm));
            }
        }
        if weighting == Weighting::ConstantVoltage {
            // One training-only scalar preserves voltage least-squares minima
            // without dividing by oracle sigma (also defined for noiseless data).
            let norm = dot(&data.y, &data.y).sqrt();
            if !norm.is_finite() || norm <= 0.0 {
                return Err("invalid pooled training voltage norm".into());
            }
            data.weights.fill(1.0 / norm);
        }
        Ok(data)
    }
    fn residual(&self, s: Sensor, state: &[f64]) -> Vec<f64> {
        self.q
            .iter()
            .zip(&self.v)
            .zip(&self.y)
            .zip(&self.weights)
            .map(|(((q, v), y), w)| (y - s.voltage(dot(q, state), dot(v, state))) * w)
            .collect()
    }
    fn jacobian(&self, s: Sensor, state: &[f64]) -> Dense {
        let mut columns: Dense = (0..18).map(|_| Vec::with_capacity(self.y.len())).collect();
        for ((q, v), w) in self.q.iter().zip(&self.v).zip(&self.weights) {
            let [dx, dv] = s.gradient(dot(q, state), dot(v, state));
            for (j, col) in columns.iter_mut().enumerate() {
                col.push((dx * q[j] + dv * v[j]) * w);
            }
        }
        columns
    }
    fn seed(&self, s: Sensor) -> Result<Vec<f64>, Box<dyn Error>> {
        let columns = self.jacobian(s, &[0.0; 18]);
        let values = self
            .y
            .iter()
            .zip(&self.weights)
            .map(|(y, w)| y * w)
            .collect::<Vec<_>>();
        Ok(infer(&columns, &values)?.state)
    }
}

#[derive(Serialize)]
struct Solution {
    initial_state: Vec<f64>,
    training_relative_rmse: f64,
    status: &'static str,
    iterations: usize,
    history: Vec<Value>,
}
fn optimize(data: &Training, s: Sensor, start: Vec<f64>) -> Result<Solution, Box<dyn Error>> {
    if start.len() != 18 || start.iter().any(|x| !x.is_finite()) {
        return Err("invalid nonlinear initial state".into());
    }
    let mut state = start;
    let mut residual = data.residual(s, &state);
    let mut objective = dot(&residual, &residual);
    if !objective.is_finite() {
        return Err("non-finite nonlinear objective".into());
    }
    let mut history = vec![json!({"iteration":0,"objective":objective})];
    let mut lambda = 1e-3_f64;
    let mut status = "iteration_limit";
    let mut iterations = 0;
    for iteration in 0..ITERATIONS {
        if objective.sqrt() < 1e-9 {
            status = "residual_converged";
            break;
        }
        let columns = data.jacobian(s, &state);
        let norms: Vec<_> = columns.iter().map(|c| dot(c, c).sqrt()).collect();
        if norms.iter().any(|n| !n.is_finite() || *n <= 0.0) {
            return Err("zero or non-finite nonlinear Jacobian column".into());
        }
        let mut accepted = false;
        for trial in 0..8 {
            let augmented: Dense = columns
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    c.iter()
                        .map(|x| x / norms[j])
                        .chain((0..18).map(|i| if i == j { lambda.sqrt() } else { 0.0 }))
                        .collect()
                })
                .collect();
            let values: Vec<_> = residual.iter().copied().chain([0.0; 18]).collect();
            let step = infer(&augmented, &values)?;
            let proposal: Vec<_> = state
                .iter()
                .zip(&step.state)
                .zip(&norms)
                .map(|((x, d), n)| x + d / n)
                .collect();
            let candidate = data.residual(s, &proposal);
            let cost = dot(&candidate, &candidate);
            let improved =
                cost.is_finite() && proposal.iter().all(|x| x.is_finite()) && cost < objective;
            history.push(json!({"iteration":iteration+1,"trial":trial,"damping":lambda,"candidate_objective":if cost.is_finite(){Some(cost)}else{None},"accepted":improved}));
            if improved {
                state = proposal;
                residual = candidate;
                objective = cost;
                lambda = (lambda * 0.2).max(1e-12);
                accepted = true;
                break;
            }
            lambda = (lambda * 10.0).min(1e12);
        }
        iterations = iteration + 1;
        if !accepted {
            status = "no_descent_step";
            break;
        }
    }
    if objective.sqrt() < 1e-9 {
        status = "residual_converged";
    }
    Ok(Solution {
        initial_state: state,
        training_relative_rmse: objective.sqrt(),
        status,
        iterations,
        history,
    })
}

fn score(
    t: &Templates,
    s: Sensor,
    state: &[f64],
    traces: &[Trace; 2],
    voltages: &[Vec<f64>; 2],
) -> Value {
    let mut windows = Vec::new();
    for index in 0..2 {
        let operator = if index == 0 { &t.off } else { &t.on };
        let mut motion = if index == 0 {
            state.to_vec()
        } else {
            apply(&t.event, state)
        };
        let mut error = 0.0;
        let mut signal = 0.0;
        let mut state_error = 0.0;
        let mut state_signal = 0.0;
        let count = voltages[index].len();
        for (i, (y, truth)) in voltages[index].iter().zip(&traces[index].truth).enumerate() {
            if i >= count / 2 {
                let q = dot(&t.windows[index].displacement[i], state);
                let v = dot(&t.windows[index].velocity[i], state);
                error += (s.voltage(q, v) - y).powi(2);
                signal += y * y;
                state_error += motion
                    .iter()
                    .zip(truth)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>();
                state_signal += dot(truth, truth);
            }
            motion = apply(&operator.step, &motion);
        }
        windows.push(json!({"held_out_voltage_relative_rmse":(error/signal).sqrt(),"held_out_state_energy_norm_relative_rmse":(state_error/state_signal).sqrt()}));
    }
    json!({"windows":windows,"known_state_recovery":windows.iter().all(|w|w["held_out_voltage_relative_rmse"].as_f64().is_some_and(|v|v<1e-6) && w["held_out_state_energy_norm_relative_rmse"].as_f64().is_some_and(|v|v<1e-4))})
}

fn fit_state(
    t: &Templates,
    s: Sensor,
    traces: &[Trace; 2],
    voltages: &[Vec<f64>; 2],
) -> Result<Value, Box<dyn Error>> {
    fit_state_weighted(t, s, traces, voltages, Weighting::RelativeWindows)
}

fn fit_state_weighted(
    t: &Templates,
    s: Sensor,
    traces: &[Trace; 2],
    voltages: &[Vec<f64>; 2],
    weighting: Weighting,
) -> Result<Value, Box<dyn Error>> {
    let observations = core::array::from_fn(|i| voltages[i][..voltages[i].len() / 2].to_vec());
    let data = match weighting {
        Weighting::RelativeWindows => Training::new(t, &observations)?,
        Weighting::ConstantVoltage => Training::weighted(t, &observations, weighting)?,
    };
    let seed = data.seed(s)?;
    let mut attempts = Vec::new();
    let mut best: Option<(usize, Solution)> = None;
    for factor in STARTS {
        match optimize(&data, s, seed.iter().map(|x| factor * x).collect()) {
            Ok(solution) => {
                let validation = score(t, s, &solution.initial_state, traces, voltages);
                attempts.push(
                    json!({"start_factor":factor,"optimization":solution,"validation":validation}),
                );
                // Selection depends only on training objective, never validation.
                if best
                    .as_ref()
                    .is_none_or(|(_, b)| solution.training_relative_rmse < b.training_relative_rmse)
                {
                    best = Some((attempts.len() - 1, solution));
                }
            }
            Err(e) => attempts.push(json!({"start_factor":factor,"error":e.to_string()})),
        }
    }
    let Some((index, solution)) = best else {
        return Ok(json!({"error":"all nonlinear state starts failed","attempts":attempts}));
    };
    let validation = score(t, s, &solution.initial_state, traces, voltages);
    Ok(
        json!({"selected_start_index":index,"selection":"Minimum training objective only; all start outcomes retained.","validation":validation,"attempts":attempts}),
    )
}

fn study() -> Result<Value, Box<dyn Error>> {
    let prepared = prepare(perturbations()[0])?;
    let sensors = sensors()?;
    let mut cases = Vec::new();
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let t = templates(
                &prepared.spectrum,
                &prepared.structural,
                &prepared.damper,
                alpha,
                beta,
                rate,
            )?;
            let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
                pickup: Vec::new(),
                truth: Vec::new(),
                contact_free: true,
            });
            let mut positions: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
            let take =
                simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
                    for (i, (start, end)) in [(0.02, 0.10), (0.14, 0.22)].into_iter().enumerate() {
                        if tick >= (start * tick_rate as f64).round() as usize
                            && tick < (end * tick_rate as f64).round() as usize
                        {
                            traces[i].contact_free &=
                                !before.contact_active && !after.contact_active;
                            if tick % (tick_rate / rate as usize) == 0 {
                                traces[i].pickup.push(before.pickup_velocity_m_s);
                                traces[i]
                                    .truth
                                    .push(reference_state(&prepared.spectrum, before));
                                positions[i].push(before.pickup_displacement_m);
                            }
                        }
                    }
                })?;
            let contact_free = take.diagnostics["last_contact_seconds"]
                .as_f64()
                .is_some_and(|v| v < 0.02)
                && traces
                    .iter()
                    .all(|tr| tr.contact_free && tr.pickup.len() == t.windows[0].velocity.len());
            if !contact_free {
                return Err("invalid nonlinear state history".into());
            }
            let mut observations = Vec::new();
            for s in &sensors {
                let voltages: [Vec<f64>; 2] = core::array::from_fn(|i| {
                    positions[i]
                        .iter()
                        .zip(&traces[i].pickup)
                        .map(|(x, v)| s.voltage(*x, *v))
                        .collect()
                });
                let required = s.geometry == "baseline" || s.geometry == "centered";
                let fit = if s.offset_m == 0.0 {
                    let mut error = 0.0;
                    let mut signal = 0.0;
                    for i in 0..2 {
                        for ((x, v), y) in
                            positions[i].iter().zip(&traces[i].pickup).zip(&voltages[i])
                        {
                            error += (s.voltage(-x, -v) - y).powi(2);
                            signal += y * y;
                        }
                    }
                    json!({"status":"withheld_sign_ambiguity","sign_symmetry_relative_rmse":(error/signal).sqrt(),"voltage_energy":signal,
                        "scope":"For a centered pickup V(-x,-v)=V(x,v). Linear mechanics preserves the sign-reversed state through the event, so voltage alone does not select absolute state sign without additional strike information. Zero-state Jacobian also vanishes. No unique-state fit claimed."})
                } else {
                    match fit_state(&t, *s, &traces, &voltages) {
                        Ok(f) => f,
                        Err(e) => json!({"error":e.to_string()}),
                    }
                };
                let passed = if s.offset_m == 0.0 {
                    fit["sign_symmetry_relative_rmse"]
                        .as_f64()
                        .is_some_and(|v| v < 1e-12)
                        && fit["voltage_energy"].as_f64().is_some_and(|v| v > 0.0)
                } else {
                    fit["validation"]["known_state_recovery"] == true
                };
                observations.push(json!({"sensor":s,"required_control":required,"required_control_passed":if required{Some(passed)}else{None},"fit":fit}));
            }
            cases.push(json!({"supplied_structural_scale":alpha,"supplied_damper_scale":beta,"diagnostics":take.diagnostics,"controls_passed":take.diagnostics["mechanical_checks_passed"]==true && observations.iter().filter(|o|o["required_control"]==true).all(|o|o["required_control_passed"]==true),"observations":observations}));
            println!("Nonlinear magnetic state: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-state-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),"cases":cases,
        "protocol":"Frozen before first run. Same six known-loss mechanical trajectories and six sensors as magnetic-pickup-loss, point-sampled at 48/96 kHz. Here true loss scales, M,K, observation geometry and gain are supplied; only one 18-coordinate initial state at 0.02 s is estimated. Known propagation through the 0.14 s viscous event. Joint training on [0.02,0.06) and [0.14,0.18), each residual normalized by its signal norm and sqrt(2); held-out [0.06,0.10) and [0.18,0.22) never select starts or update state. Analytic magnetic Jacobian times mechanical displacement/velocity templates. Rest-linearized least-squares seed; fixed factors 0.5,1,1.5. Levenberg-Marquardt scaled columns, augmented reorthogonalized QR, lambda initially 1e-3, times 0.2 after descent / times 10 after rejection, bounded 1e-12..1e12. At most 40 iterations with eight trials each, stop at training relative RMS <1e-9 or no descent. Select lowest training objective, retain every start and scored outcome. Twelve baseline controls require both held-out voltage relative RMSEs <1e-6 and state energy-norm errors <1e-4; close geometry descriptive. Twelve centered controls verify nonzero voltage with sign symmetry residual <1e-12 and withhold unique-state inference. No gate or start tuning.",
        "scope":"Nonlinear initial-state inverse with known physical loss scales and sensor law, not unknown-loss calibration. Current magnetic laws are uncalibrated proxies. Multistart agreement does not prove global uniqueness; centered sign symmetry needs additional information. Direct samples can contain aliased harmonics; no antialiasing, real-bank calibration, production DSP or audio change. No host launch."}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = study()?;
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("nonlinear magnetic state study retained failed controls".into());
    }
    println!("Nonlinear magnetic state: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_voltage_weights_preserve_pooled_error_and_ignore_held_out_rows() {
        let p = prepare(perturbations()[0]).unwrap();
        let mut t = templates(&p.spectrum, &p.structural, &p.damper, 0.7, 1.3, 48000).unwrap();
        let y = [vec![2.0; 20], vec![5.0; 60]];
        let pooled = Training::weighted(&t, &y, Weighting::ConstantVoltage).unwrap();
        let relative = Training::new(&t, &y).unwrap();
        let norm = (20.0_f64 * 4.0 + 60.0 * 25.0).sqrt();
        assert!(
            pooled
                .weights
                .iter()
                .all(|w| (*w - 1.0 / norm).abs() < 1e-15)
        );
        assert_ne!(relative.weights[0], relative.weights[20]);
        let sensor = sensors().unwrap()[0];
        let state: Vec<_> = (0..18).map(|i| 1e-3 * (f64::from(i) + 0.3).sin()).collect();
        let raw: f64 = pooled
            .q
            .iter()
            .zip(&pooled.v)
            .zip(&pooled.y)
            .map(|((q, v), y)| (y - sensor.voltage(dot(q, &state), dot(v, &state))).powi(2))
            .sum();
        let r = pooled.residual(sensor, &state);
        assert!((dot(&r, &r) - raw / norm.powi(2)).abs() < 1e-12);
        for (window, values) in t.windows.iter_mut().zip(&y) {
            for q in &mut window.displacement[values.len()..] {
                q.fill(f64::NAN);
            }
            for v in &mut window.velocity[values.len()..] {
                v.fill(f64::NAN);
            }
        }
        let unaffected = Training::weighted(&t, &y, Weighting::ConstantVoltage).unwrap();
        assert_eq!(r, unaffected.residual(sensor, &state));
        for bad in [
            [vec![], vec![1.0]],
            [vec![f64::NAN], vec![1.0]],
            [vec![0.0], vec![1.0]],
        ] {
            assert!(Training::weighted(&t, &bad, Weighting::ConstantVoltage).is_err());
        }
    }

    #[test]
    fn constant_voltage_jacobian_matches_weighted_residual_differences() {
        let p = prepare(perturbations()[0]).unwrap();
        let t = templates(&p.spectrum, &p.structural, &p.damper, 0.7, 1.3, 48000).unwrap();
        let data = Training::weighted(
            &t,
            &[vec![2.0; 40], vec![5.0; 40]],
            Weighting::ConstantVoltage,
        )
        .unwrap();
        let sensor = sensors().unwrap()[0];
        let state: Vec<_> = (0..18).map(|i| 1e-3 * (f64::from(i) + 0.3).sin()).collect();
        let columns = data.jacobian(sensor, &state);
        for j in 0..18 {
            let mut plus = state.clone();
            let mut minus = state.clone();
            plus[j] += 1e-7;
            minus[j] -= 1e-7;
            let rp = data.residual(sensor, &plus);
            let rm = data.residual(sensor, &minus);
            for i in 0..rp.len() {
                let derivative = (rm[i] - rp[i]) / 2e-7;
                assert!((columns[j][i] - derivative).abs() < 1e-7 * columns[j][i].abs().max(1.0));
            }
        }
    }

    #[test]
    fn magnetic_jacobian_matches_independent_centered_differences() {
        for s in sensors().unwrap() {
            for x in [-0.0004, -0.0001, 0.0, 0.0003] {
                for v in [-0.2, 0.1] {
                    let [dx, dv] = s.gradient(x, v);
                    let h = 1e-9;
                    let q = (s.voltage(x + h, v) - s.voltage(x - h, v)) / (2.0 * h);
                    let vel = (s.voltage(x, v + h) - s.voltage(x, v - h)) / (2.0 * h);
                    assert!((dx - q).abs() < 2e-6 * dx.abs().max(1.0));
                    assert!((dv - vel).abs() < 2e-6 * dv.abs().max(1.0));
                }
            }
        }
    }
    #[test]
    fn continuous_templates_match_direct_state_propagation() {
        let p = prepare(perturbations()[0]).unwrap();
        let t = templates(&p.spectrum, &p.structural, &p.damper, 0.7, 1.3, 48000).unwrap();
        let initial: Vec<_> = (0..18).map(|i| 1e-3 * (f64::from(i) + 0.3).sin()).collect();
        let port_q: Vec<_> = (0..18)
            .map(|i| {
                if i < 9 {
                    p.spectrum.modes[i].pickup_weight / (TAU * p.spectrum.modes[i].frequency_hz)
                } else {
                    0.0
                }
            })
            .collect();
        for index in 0..2 {
            let op = if index == 0 { &t.off } else { &t.on };
            let mut state = if index == 0 {
                initial.clone()
            } else {
                apply(&t.event, &initial)
            };
            for i in 0..t.windows[index].velocity.len() {
                assert!(
                    (dot(&t.windows[index].displacement[i], &initial) - dot(&port_q, &state)).abs()
                        < 1e-12
                );
                assert!(
                    (dot(&t.windows[index].velocity[i], &initial) - dot(&op.port, &state)).abs()
                        < 1e-10
                );
                state = apply(&op.step, &state);
            }
        }
    }
}
