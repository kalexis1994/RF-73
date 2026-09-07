//! Offline initial-state inference from one mechanical pickup-velocity history.
//! Geometry AND damping are supplied; no unknown losses are estimated here.
use super::*;
use rf_73_dsp::{ModalProbe, ModalSpectrum};
use std::f64::consts::TAU;

type Dense = Vec<Vec<f64>>;
pub const HELP: &str = "Dynamic pickup state study:
  pickup-state --output REPORT.json
Known geometry and damping; one pickup-velocity history, fitted initial state.
Held-out prediction, deterministic noise and omitted-mode controls; no audio calibration.
";

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn apply(a: &Dense, x: &[f64]) -> Vec<f64> {
    a.iter().map(|r| dot(r, x)).collect()
}
fn multiply(a: &Dense, b: &Dense) -> Dense {
    a.iter()
        .map(|row| {
            (0..b.len())
                .map(|j| row.iter().enumerate().map(|(k, v)| v * b[k][j]).sum())
                .collect()
        })
        .collect()
}

// Independent laboratory propagator in energy-scaled modal coordinates.
// A small Taylor exponential plus squaring avoids integration phase error at 15 kHz.
fn transition(g: &Dense, dt: f64) -> Result<Dense, Box<dyn Error>> {
    if g.is_empty()
        || g.iter()
            .any(|r| r.len() != g.len() || r.iter().any(|x| !x.is_finite()))
        || !dt.is_finite()
        || dt <= 0.0
    {
        return Err("invalid observer generator or time step".into());
    }
    let norm = g
        .iter()
        .map(|r| r.iter().map(|x| x.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let mut h = dt;
    let mut squares = 0;
    while norm * h > 1.0 / 32.0 {
        h *= 0.5;
        squares += 1;
        if squares > 32 {
            return Err("observer exponential scaling exceeded 32".into());
        }
    }
    let mut sum: Dense = (0..g.len())
        .map(|i| (0..g.len()).map(|j| f64::from(i == j)).collect())
        .collect();
    let mut term = sum.clone();
    for order in 1..=18 {
        term = multiply(&term, g);
        for (s, t) in sum.iter_mut().flatten().zip(term.iter_mut().flatten()) {
            *t *= h / f64::from(order);
            *s += *t;
        }
    }
    for _ in 0..squares {
        sum = multiply(&sum, &sum);
    }
    if sum.iter().flatten().any(|x| !x.is_finite()) {
        return Err("non-finite observer transition".into());
    }
    Ok(sum)
}

struct Observer {
    step: Dense,
    port: Vec<f64>,
}
impl Observer {
    fn prepare(
        s: &ModalSpectrum,
        c: &Matrix,
        count: usize,
        rate: u32,
    ) -> Result<Self, Box<dyn Error>> {
        if !(1..=9).contains(&count) || rate == 0 {
            return Err("invalid observer mode count or rate".into());
        }
        let mut g = vec![vec![0.0; 2 * count]; 2 * count];
        let mut port = vec![0.0; 2 * count];
        for (i, mode) in s.modes[..count].iter().enumerate() {
            let omega = TAU * mode.frequency_hz;
            g[i][i + count] = omega;
            g[i + count][i] = -omega;
            port[i + count] = mode.pickup_weight;
            for (j, other) in s.modes[..count].iter().enumerate() {
                // Retain every off-diagonal damping term; damping is not proportional.
                let force: [f64; 9] = core::array::from_fn(|k| dot(&c[k], &other.shape));
                g[i + count][j + count] = -dot(&mode.shape, &force);
            }
        }
        Ok(Self {
            step: transition(&g, 1.0 / f64::from(rate))?,
            port,
        })
    }
    fn columns(&self, samples: usize) -> Dense {
        let mut columns = vec![Vec::with_capacity(samples); self.port.len()];
        let mut row = self.port.clone();
        for _ in 0..samples {
            for (col, y) in columns.iter_mut().zip(&row) {
                col.push(*y);
            }
            row = (0..row.len())
                .map(|j| {
                    row.iter()
                        .enumerate()
                        .map(|(i, v)| v * self.step[i][j])
                        .sum()
                })
                .collect();
        }
        columns
    }
}

struct Estimate {
    state: Vec<f64>,
    minimum_normalized_qr_pivot: f64,
}

// Only observation templates and scalar training samples enter this inverse.
// No state, clean target, held-out sample, noise seed or fitted-loss truth is accepted.
fn infer(columns: &Dense, samples: &[f64]) -> Result<Estimate, Box<dyn Error>> {
    let count = columns.len();
    if count == 0
        || samples.len() < count
        || samples.iter().any(|x| !x.is_finite())
        || columns
            .iter()
            .any(|c| c.len() != samples.len() || c.iter().any(|x| !x.is_finite()))
    {
        return Err("invalid pickup observation matrix or samples".into());
    }
    let norms: Vec<_> = columns.iter().map(|c| dot(c, c).sqrt()).collect();
    if norms.iter().any(|n| !n.is_finite() || *n <= 0.0) {
        return Err("unobservable pickup state column".into());
    }
    let mut q: Dense = Vec::new();
    let mut r = vec![vec![0.0; count]; count];
    let mut minimum = 1.0_f64;
    for (j, col) in columns.iter().enumerate() {
        let mut v: Vec<_> = col.iter().map(|x| x / norms[j]).collect();
        for _ in 0..2 {
            for (i, basis) in q.iter().enumerate() {
                let projection = dot(basis, &v);
                r[i][j] += projection;
                for (x, b) in v.iter_mut().zip(basis) {
                    *x -= projection * b;
                }
            }
        }
        let pivot = dot(&v, &v).sqrt();
        if !pivot.is_finite() || pivot < 1e-8 {
            return Err("pickup state observation is rank deficient".into());
        }
        minimum = minimum.min(pivot);
        r[j][j] = pivot;
        q.push(v.iter().map(|x| x / pivot).collect());
    }
    let mut state = vec![0.0; count];
    for i in (0..count).rev() {
        state[i] = (dot(&q[i], samples) - dot(&r[i][i + 1..], &state[i + 1..])) / r[i][i];
    }
    for (x, norm) in state.iter_mut().zip(norms) {
        *x /= norm;
    }
    if state.iter().any(|x| !x.is_finite()) {
        return Err("non-finite inferred pickup state".into());
    }
    Ok(Estimate {
        state,
        minimum_normalized_qr_pivot: minimum,
    })
}

// Deterministic, zero-mean uniform pseudorandom perturbation, with unit sample RMS.
fn noise(count: usize) -> Vec<f64> {
    let mut seed = 0x73c0ffee_u32;
    let mut values: Vec<_> = (0..count)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            f64::from(seed) / f64::from(u32::MAX) - 0.5
        })
        .collect();
    let mean = values.iter().sum::<f64>() / count as f64;
    for x in &mut values {
        *x -= mean;
    }
    let rms = (dot(&values, &values) / count as f64).sqrt();
    for x in &mut values {
        *x /= rms;
    }
    values
}

fn reference_state(s: &ModalSpectrum, p: &ModalProbe) -> [f64; 18] {
    let mq: [f64; 9] = core::array::from_fn(|i| dot(&s.mass_matrix[i], &p.position));
    let mv: [f64; 9] = core::array::from_fn(|i| dot(&s.mass_matrix[i], &p.velocity));
    core::array::from_fn(|i| {
        let mode = &s.modes[i % 9];
        if i < 9 {
            TAU * mode.frequency_hz * dot(&mode.shape, &mq)
        } else {
            dot(&mode.shape, &mv)
        }
    })
}
struct Trace {
    pickup: Vec<f64>,
    truth: Vec<[f64; 18]>,
    contact_free: bool,
}

fn evaluate(observer: &Observer, estimate: &Estimate, trace: &Trace, training: &[f64]) -> Value {
    let count = estimate.state.len() / 2;
    let split = training.len();
    let mut state = estimate.state.clone();
    let mut error = [0.0; 2];
    let mut signal = [0.0; 2];
    let mut state_error = 0.0;
    let mut retained_error = 0.0;
    let mut state_signal = 0.0;
    let mut retained_signal = 0.0;
    let mut per_mode_error = [0.0; 9];
    let mut per_mode_signal = [0.0; 9];
    let mut noisy_fit_error = 0.0;
    for (index, (y, truth)) in trace.pickup.iter().zip(&trace.truth).enumerate() {
        let prediction = dot(&observer.port, &state);
        let region = usize::from(index >= split);
        error[region] += (prediction - y).powi(2);
        signal[region] += y * y;
        if index < split {
            noisy_fit_error += (prediction - training[index]).powi(2);
        } else {
            for mode in 0..9 {
                let (q, v) = if mode < count {
                    (state[mode], state[mode + count])
                } else {
                    (0.0, 0.0)
                };
                let e = (q - truth[mode]).powi(2) + (v - truth[mode + 9]).powi(2);
                let power = truth[mode].powi(2) + truth[mode + 9].powi(2);
                per_mode_error[mode] += e;
                per_mode_signal[mode] += power;
                state_error += e;
                state_signal += power;
                if mode < count {
                    retained_error += e;
                    retained_signal += power;
                }
            }
        }
        state = apply(&observer.step, &state);
    }
    json!({
        "minimum_normalized_qr_pivot":estimate.minimum_normalized_qr_pivot,
        "inferred_initial_energy_coordinates":estimate.state,
        "training_clean_pickup_relative_rmse":(error[0]/signal[0]).sqrt(),
        "training_noisy_pickup_rmse_over_clean_rms":(noisy_fit_error/signal[0]).sqrt(),
        "held_out_clean_pickup_relative_rmse":(error[1]/signal[1]).sqrt(),
        "held_out_full_state_energy_norm_relative_rmse":(state_error/state_signal).sqrt(),
        "held_out_retained_state_energy_norm_relative_rmse":(retained_error/retained_signal).sqrt(),
        "held_out_per_mode_energy_norm_relative_rmse":per_mode_error.iter().zip(per_mode_signal).map(|(e,s)| (e/s).sqrt()).collect::<Vec<_>>()
    })
}

fn study() -> Result<Value, Box<dyn Error>> {
    let s = ModalSpectrum::prepare(TineGeometry::default(), ModalAssemblyProfile::default())?;
    let baseline = ModalAssembly::new(
        48000.0,
        TineGeometry::default(),
        ModalAssemblyProfile::default(),
        ModalIntegration::Refined {
            contact_substeps: 64,
        },
    )?;
    let c0 = baseline.damping_matrix(false);
    let on = baseline.damping_matrix(true);
    let windows = [(0.02, 0.06, 0.10), (0.14, 0.18, 0.22)];
    let mut cases = Vec::new();
    for (structural, damper) in [(0.5, 1.5), (1.0, 0.5), (1.5, 1.0)] {
        for rate in [48000, 96000] {
            let mut traces: Vec<_> = windows
                .iter()
                .map(|_| Trace {
                    pickup: Vec::new(),
                    truth: Vec::new(),
                    contact_free: true,
                })
                .collect();
            let take = simulate_observed(
                rate,
                structural,
                damper,
                |tick, tick_rate, before, after, _| {
                    for (trace, (start, _, end)) in traces.iter_mut().zip(windows) {
                        if tick >= (start * tick_rate as f64).round() as usize
                            && tick < (end * tick_rate as f64).round() as usize
                        {
                            trace.contact_free &= !before.contact_active && !after.contact_active;
                            if tick % (tick_rate / rate as usize) == 0 {
                                trace.pickup.push(before.pickup_velocity_m_s);
                                trace.truth.push(reference_state(&s, before));
                            }
                        }
                    }
                },
            )?;
            let mut results = Vec::new();
            for (window, (trace, (start, split, end))) in traces.iter().zip(windows).enumerate() {
                let training_count = ((split - start) * f64::from(rate)).round() as usize;
                let expected_count = ((end - start) * f64::from(rate)).round() as usize;
                if trace.pickup.len() != expected_count || !trace.contact_free {
                    return Err("invalid observer contact-free history".into());
                }
                let clean = &trace.pickup[..training_count];
                let rms = (dot(clean, clean) / training_count as f64).sqrt();
                if !rms.is_finite() || rms <= 0.0 {
                    return Err("unexcited pickup history".into());
                }
                let perturbation = noise(training_count);
                let c: Matrix = core::array::from_fn(|i| {
                    core::array::from_fn(|j| {
                        structural * c0[i][j]
                            + if window == 1 {
                                damper * (on[i][j] - c0[i][j])
                            } else {
                                0.0
                            }
                    })
                });
                for count in [3, 6, 9] {
                    let observer = Observer::prepare(&s, &c, count, rate)?;
                    let columns = observer.columns(training_count);
                    for level in [0.0, 0.001, 0.01] {
                        let training: Vec<_> = clean
                            .iter()
                            .zip(&perturbation)
                            .map(|(y, n)| y + level * rms * n)
                            .collect();
                        let fit = match infer(&columns, &training) {
                            Ok(estimate) => evaluate(&observer, &estimate, trace, &training),
                            Err(e) => json!({"error":e.to_string()}),
                        };
                        let required = count == 9 && level == 0.0;
                        let passed = fit["held_out_clean_pickup_relative_rmse"]
                            .as_f64()
                            .is_some_and(|x| x < 1e-6)
                            && fit["held_out_full_state_energy_norm_relative_rmse"]
                                .as_f64()
                                .is_some_and(|x| x < 1e-5);
                        results.push(json!({"start_seconds":start,"split_seconds":split,"end_seconds_exclusive":end,
                            "damper_on":window==1,"contact_free":trace.contact_free,"training_samples":training_count,
                            "held_out_samples":expected_count-training_count,"retained_modes":count,"training_noise_relative_rms":level,
                            "training_clean_pickup_rms_m_s":rms,"required_control":required,"required_control_passed":if required {Some(passed)} else {None},"fit":fit}));
                    }
                }
            }
            cases.push(json!({"known_structural_scale":structural,"known_damper_scale":damper,"diagnostics":take.diagnostics,
                "controls_passed":take.diagnostics["mechanical_checks_passed"]==true && results.iter().filter(|r|r["required_control"]==true).all(|r|r["required_control_passed"]==true),"observations":results}));
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"dynamic-pickup-state-v1",
        "controls_passed":cases.iter().all(|c|c["controls_passed"]==true),"cases":cases,
        "modes":s.modes.iter().map(|m|json!({"frequency_hz":m.frequency_hz,"pickup_weight":m.pickup_weight})).collect::<Vec<_>>(),
        "protocol":"Frozen before first run. Same six controlled loss-study strikes at 48/96 kHz. Scalar mechanical pickup velocity sampled at output rate, not displacement or differentiated data. Fit initial state on [0.02,0.06) and separately [0.14,0.18); predict unused [0.06,0.10) and [0.18,0.22) without refit. Contact-free histories, known binary damper status within each window. Known M,K AND actual C. Lowest 3/6/9 undamped modes; x=[omega*z,zdot], all projected damping couplings retained. Independent scaled Taylor matrix exponential, normalized two-pass QR; reject pivots <1e-8. Same zero-mean unit-RMS LCG sequence (seed 0x73c0ffee, multiplier 1664525, increment 1013904223) per training length, injected only into training at 0/0.001/0.01 times clean training RMS. Ground truth used only to generate observations and score errors. Required nine-mode noiseless controls: held-out clean pickup relative RMSE <1e-6, full-state energy-norm relative RMSE <1e-5, original mechanical checks pass. Reduced/noisy cases descriptive, every failure retained. No threshold or window tuning.",
        "scope":"Offline initial-state batch inference with exact geometry, mass, stiffness, damping, observation gain and event timing. Not loss estimation: actual loss scales are supplied to the observer. Energy norm uses omega-weighted modal displacement and modal velocity, not relative energy scalar error. Per-mode errors include omitted modes as zero reconstruction. Mechanical pickup velocity is not nonlinear magnetic voltage or processed source-bank audio. One untuned geometry and elastic single-mass hammer. No production model, audio asset, host or preset changes."}),
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
        return Err("dynamic pickup state study retained failed controls".into());
    }
    println!("Dynamic pickup state: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exponential_matches_damped_oscillator_and_time_refinement() {
        let omega = 19000.0;
        let loss = 17.0;
        let dt = 1.0 / 48000.0;
        let g = vec![vec![0.0, omega], vec![-omega, -2.0 * loss]];
        let step = transition(&g, dt).unwrap();
        let half = transition(&g, dt / 2.0).unwrap();
        let twice = multiply(&half, &half);
        let wd = (omega * omega - loss * loss).sqrt();
        let factor = (-loss * dt).exp();
        let sine = (wd * dt).sin() / wd;
        let cosine = (wd * dt).cos();
        let exact = [
            [factor * (cosine + loss * sine), factor * omega * sine],
            [-factor * omega * sine, factor * (cosine - loss * sine)],
        ];
        for i in 0..2 {
            for j in 0..2 {
                assert!((step[i][j] - exact[i][j]).abs() < 1e-12);
                assert!((step[i][j] - twice[i][j]).abs() < 1e-12);
            }
        }
        assert!(transition(&g, f64::NAN).is_err());
        assert!(transition(&vec![vec![0.0]], -1.0).is_err());
    }
    #[test]
    fn pickup_history_resolves_instantaneously_invisible_displacement() {
        let omega = 310.0;
        let observer = Observer {
            step: transition(&vec![vec![0.0, omega], vec![-omega, 0.0]], 1.0 / 48000.0).unwrap(),
            port: vec![0.0, 2.0],
        };
        let columns = observer.columns(1000);
        // Analytic scalar observations, independent of the template propagation.
        let samples: Vec<_> = (0..1000)
            .map(|i| -2.0 * 0.004 * (omega * f64::from(i) / 48000.0).sin())
            .collect();
        let fit = infer(&columns, &samples).unwrap();
        assert!((fit.state[0] - 0.004).abs() < 1e-12);
        assert!(fit.state[1].abs() < 1e-12);
        assert_eq!(samples[0], 0.0);
        assert!(infer(&observer.columns(1), &[0.0]).is_err());
    }
    #[test]
    fn inverse_rejects_missing_rank_nonfinite_and_malformed_observations() {
        for columns in [
            vec![vec![0.0; 3]],
            vec![vec![1.0, 2.0, 3.0], vec![2.0, 4.0, 6.0]],
            vec![vec![f64::NAN; 3]],
            vec![vec![1.0; 2]],
        ] {
            assert!(infer(&columns, &[1.0, 2.0, 3.0]).is_err());
        }
        assert!(infer(&vec![vec![1.0; 3]], &[1.0, f64::INFINITY, 3.0]).is_err());
        let n = noise(1920);
        assert_eq!(n, noise(1920));
        assert!(n.iter().sum::<f64>().abs() < 1e-12);
        assert!((dot(&n, &n) / n.len() as f64 - 1.0).abs() < 1e-12);
    }
}
