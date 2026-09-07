//! Profile two unknown loss scales with a nonlinear continuous-state refit.
pub mod noise;
pub mod resolution;
pub mod weighting;
use super::*;

pub const HELP: &str = "Nonlinear magnetic loss profile:
  magnetic-loss-profile --output REPORT.json
Known production pickup geometry, unknown structural/damper losses; training-only nested fits.
";
const LOSS_STARTS: [[f64; 2]; 2] = [[0.5, 1.5], [1.5, 0.5]];
const LOG_STEP: f64 = 1e-3;
const OUTER_ITERATIONS: usize = 16;
const TARGET: f64 = 1e-8;

struct Candidate {
    scales: [f64; 2],
    state: Vec<f64>,
    residual: Vec<f64>,
    objective: f64,
    evaluation_index: usize,
}

struct StateFit {
    state: Vec<f64>,
    residual: Vec<f64>,
    starts: Vec<Value>,
    selected: usize,
}

// The inverse accepts only assumed operators, sensor geometry and training
// voltages. Neither true losses, reference motion nor held-out data are fields.
struct Profile<'a> {
    spectrum: &'a ModalSpectrum,
    structural: &'a Matrix,
    damper: &'a Matrix,
    sensor: Sensor,
    rate: u32,
    weighting: Weighting,
    training: [Vec<f64>; 2],
    evaluations: Vec<Value>,
}
impl Profile<'_> {
    fn candidate(&mut self, scales: [f64; 2]) -> Result<Candidate, Box<dyn Error>> {
        let index = self.evaluations.len();
        let result = self.evaluate(scales);
        match result {
            Ok(StateFit {
                state,
                residual,
                starts,
                selected,
            }) => {
                let objective = dot(&residual, &residual);
                self.evaluations
                    .push(json!({"scales":scales,"objective":objective,
                    "selected_state_start_index":selected,"state_starts":starts}));
                Ok(Candidate {
                    scales,
                    state,
                    residual,
                    objective,
                    evaluation_index: index,
                })
            }
            Err(e) => {
                self.evaluations
                    .push(json!({"scales":scales,"error":e.to_string()}));
                Err(e)
            }
        }
    }
    fn evaluate(&self, scales: [f64; 2]) -> Result<StateFit, Box<dyn Error>> {
        if scales
            .iter()
            .any(|x| !x.is_finite() || *x < LOWER || *x > UPPER)
        {
            return Err("loss candidate outside 0.25..2.0".into());
        }
        let t = templates(
            self.spectrum,
            self.structural,
            self.damper,
            scales[0],
            scales[1],
            self.rate,
        )?;
        let data = Training::weighted(&t, &self.training, self.weighting)?;
        let seed = data.seed(self.sensor)?;
        let mut starts = Vec::new();
        let mut best: Option<(usize, Solution)> = None;
        for factor in STARTS {
            match optimize(
                &data,
                self.sensor,
                seed.iter().map(|x| factor * x).collect(),
            ) {
                Ok(solution) => {
                    starts.push(json!({"factor":factor,"training_relative_rmse":solution.training_relative_rmse,
                        "status":solution.status,"iterations":solution.iterations}));
                    if best.as_ref().is_none_or(|(_, b)| {
                        solution.training_relative_rmse < b.training_relative_rmse
                    }) {
                        best = Some((starts.len() - 1, solution));
                    }
                }
                Err(e) => starts.push(json!({"factor":factor,"error":e.to_string()})),
            }
        }
        let Some((selected, solution)) = best else {
            return Err(format!("all profiled state starts failed: {}", json!(starts)).into());
        };
        let residual = data.residual(self.sensor, &solution.initial_state);
        Ok(StateFit {
            state: solution.initial_state,
            residual,
            starts,
            selected,
        })
    }
}

fn log_to_scales(logs: [f64; 2]) -> [f64; 2] {
    logs.map(|x| x.exp().clamp(LOWER, UPPER))
}
struct Outer {
    candidate: Candidate,
    status: &'static str,
    history: Vec<Value>,
}
fn solve(profile: &mut Profile<'_>, start: [f64; 2]) -> Result<Outer, Box<dyn Error>> {
    let mut current = profile.candidate(start)?;
    let mut damping = 1e-3_f64;
    let mut status = "iteration_limit";
    let mut history = vec![json!({"initial_evaluation_index":current.evaluation_index})];
    for iteration in 0..OUTER_ITERATIONS {
        if current.objective.sqrt() < TARGET {
            status = "residual_converged";
            break;
        }
        let logs = current.scales.map(f64::ln);
        let mut columns = Vec::new();
        let mut differences = Vec::new();
        for j in 0..2 {
            let mut a = logs;
            let mut b = logs;
            a[j] = (a[j] - LOG_STEP).max(LOWER.ln());
            b[j] = (b[j] + LOG_STEP).min(UPPER.ln());
            let minus = profile.candidate(log_to_scales(a))?;
            let plus = profile.candidate(log_to_scales(b))?;
            columns.push(
                plus.residual
                    .iter()
                    .zip(&minus.residual)
                    .map(|(p, m)| (p - m) / (b[j] - a[j]))
                    .collect::<Vec<_>>(),
            );
            differences.push(json!({"parameter":j,"minus_evaluation_index":minus.evaluation_index,"plus_evaluation_index":plus.evaluation_index,"log_span":b[j]-a[j]}));
        }
        let norms: Vec<_> = columns.iter().map(|c| dot(c, c).sqrt()).collect();
        if norms.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            status = "unresolved_profile_derivative";
            break;
        }
        let mut accepted = false;
        for trial in 0..8 {
            let augmented: Dense = columns
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    c.iter()
                        .map(|x| x / norms[j])
                        .chain((0..2).map(|i| if i == j { damping.sqrt() } else { 0.0 }))
                        .collect()
                })
                .collect();
            let rhs: Vec<_> = current
                .residual
                .iter()
                .map(|x| -x)
                .chain([0.0; 2])
                .collect();
            let step = infer(&augmented, &rhs)?;
            let proposal = core::array::from_fn(|j| {
                (logs[j] + step.state[j] / norms[j]).clamp(LOWER.ln(), UPPER.ln())
            });
            let next = profile.candidate(log_to_scales(proposal))?;
            let improved = next.objective < current.objective;
            history.push(json!({"iteration":iteration+1,"trial":trial,"damping":damping,"derivative_evaluations":differences,
                "from_evaluation_index":current.evaluation_index,"proposal_evaluation_index":next.evaluation_index,"accepted":improved}));
            if improved {
                current = next;
                damping = (damping * 0.2).max(1e-12);
                accepted = true;
                break;
            }
            damping = (damping * 10.0).min(1e12);
        }
        if !accepted {
            status = "no_descent_step";
            break;
        }
    }
    if current.objective.sqrt() < TARGET {
        status = "residual_converged";
    }
    Ok(Outer {
        candidate: current,
        status,
        history,
    })
}

fn recover(profile: &mut Profile<'_>) -> Result<(Candidate, Vec<Value>, usize), Box<dyn Error>> {
    let mut best: Option<(usize, Candidate)> = None;
    let mut attempts = Vec::new();
    for start in LOSS_STARTS {
        match solve(profile, start) {
            Ok(result) => {
                let c = result.candidate;
                attempts.push(json!({"start":start,"status":result.status,"scales":c.scales,
                    "objective":c.objective,"evaluation_index":c.evaluation_index,"history":result.history}));
                if best.as_ref().is_none_or(|(_, b)| c.objective < b.objective) {
                    best = Some((attempts.len() - 1, c));
                }
            }
            Err(e) => attempts.push(json!({"start":start,"error":e.to_string()})),
        }
    }
    let Some((selected, candidate)) = best else {
        return Err(format!("all loss starts failed: {}", json!(attempts)).into());
    };
    Ok((candidate, attempts, selected))
}

fn study() -> Result<Value, Box<dyn Error>> {
    let p = prepare(perturbations()[0])?;
    let sensor = sensors()?
        .into_iter()
        .find(|s| s.law == "production" && s.geometry == "baseline")
        .ok_or("missing production baseline sensor")?;
    let settings = [
        (0.63, 1.37, 48000),
        (0.63, 1.37, 96000),
        (1.13, 0.57, 48000),
        (1.13, 0.57, 96000),
        (1.47, 0.91, 48000),
        (1.47, 0.91, 96000),
        (2.2, 0.91, 48000),
    ];
    let mut cases = Vec::new();
    for (alpha, beta, rate) in settings {
        let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
            pickup: Vec::new(),
            truth: Vec::new(),
            contact_free: true,
        });
        let mut voltages: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
        let take = simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
            for (i, (start, end)) in [(0.02, 0.10), (0.14, 0.22)].into_iter().enumerate() {
                if tick >= (start * tick_rate as f64).round() as usize
                    && tick < (end * tick_rate as f64).round() as usize
                {
                    traces[i].contact_free &= !before.contact_active && !after.contact_active;
                    if tick % (tick_rate / rate as usize) == 0 {
                        traces[i].pickup.push(before.pickup_velocity_m_s);
                        traces[i].truth.push(reference_state(&p.spectrum, before));
                        voltages[i].push(
                            sensor
                                .voltage(before.pickup_displacement_m, before.pickup_velocity_m_s),
                        );
                    }
                }
            }
        })?;
        if !take.diagnostics["last_contact_seconds"]
            .as_f64()
            .is_some_and(|v| v < 0.02)
            || traces.iter().any(|t| {
                !t.contact_free || t.pickup.len() != (0.08 * f64::from(rate)).round() as usize
            })
        {
            return Err("invalid loss profile history".into());
        }
        let mut profile = Profile {
            spectrum: &p.spectrum,
            structural: &p.structural,
            damper: &p.damper,
            sensor,
            rate,
            training: core::array::from_fn(|i| voltages[i][..voltages[i].len() / 2].to_vec()),
            weighting: Weighting::RelativeWindows,
            evaluations: Vec::new(),
        };
        let fit = match recover(&mut profile) {
            Ok((candidate, attempts, selected)) => {
                let scales = candidate.scales;
                let t = templates(
                    &p.spectrum,
                    &p.structural,
                    &p.damper,
                    scales[0],
                    scales[1],
                    rate,
                )?;
                let validation = score(&t, sensor, &candidate.state, &traces, &voltages);
                let boundary = scales.iter().any(|x| x - LOWER < 1e-5 || UPPER - x < 1e-5);
                let prediction = !boundary
                    && validation["windows"].as_array().is_some_and(|ws| {
                        ws.iter().all(|w| {
                            w["held_out_voltage_relative_rmse"]
                                .as_f64()
                                .is_some_and(|e| e < 1e-6)
                        })
                    });
                let relative = [
                    (scales[0] / alpha - 1.0).abs(),
                    (scales[1] / beta - 1.0).abs(),
                ];
                json!({"estimated_scales":scales,"training_relative_rmse":candidate.objective.sqrt(),"selected_loss_start_index":selected,
                    "attempts":attempts,"validation":validation,"boundary_limited":boundary,"prediction_consistent":prediction,
                    "relative_loss_errors":relative,"both_losses_within_one_percent":relative.iter().all(|e|*e<0.01)})
            }
            Err(e) => json!({"error":e.to_string()}),
        };
        let negative = alpha > UPPER;
        let passed = take.diagnostics["mechanical_checks_passed"] == true
            && if negative {
                fit["estimated_scales"].is_array()
                    && fit["prediction_consistent"] == false
                    && fit["both_losses_within_one_percent"] == false
            } else {
                fit["prediction_consistent"] == true
                    && fit["both_losses_within_one_percent"] == true
                    && fit["validation"]["known_state_recovery"] == true
            };
        cases.push(json!({"sample_rate":rate,"reference_scales_for_scoring_only":[alpha,beta],"negative_out_of_range_control":negative,"control_passed":passed,
            "diagnostics":take.diagnostics,"fit":fit,"profile_evaluations":profile.evaluations}));
        println!(
            "Nonlinear magnetic loss profile: completed {rate} Hz, reference scales ({alpha}, {beta})"
        );
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-loss-profile-v1","controls_passed":cases.iter().all(|c|c["control_passed"]==true),"sensor":sensor,"cases":cases,
        "protocol":"Frozen before first run. Production law and baseline geometry only, noiseless. Six off-grid loss pairs/rates from preceding studies plus an out-of-range structural scale 2.2 at 48 kHz with damper 0.91. Exact mechanical mass/stiffness and sensor geometry supplied, losses unknown. Profile owns training voltage only: same two 40 ms training halves, one continuous state across 0.14 s event. Every candidate rebuilds off/on operators and refits all 18 initial coordinates from three rest-linearized seed factors, using unchanged inner optimizer. Joint outer search in log loss scales bounded 0.25..2; fixed starts (0.5,1.5),(1.5,0.5), at most 16 iterations, central log differences +/-1e-3 clipped at bounds with actual denominator, augmented column-scaled QR, initial damping 1e-3 times 0.2/10 after accepted/rejected trials, bounds 1e-12..1e12, eight trials per iteration, strict descent and training relative RMSE <1e-8 stop. Finite differences also refit state. Select minimum training objective only; retain every candidate and compact inner start outcomes and outer histories. Held-out prediction/state and reference losses used only after selection. Six positives require held-out voltage <1e-6 and state <1e-4 in both windows, both loss errors <1%, and no boundary hit within 1e-5. Out-of-range negative requires a completed fit with prediction rejection and failed two-loss recovery; no range extension.",
        "scope":"Controlled nonlinear unknown-loss recovery; two starts do not prove global uniqueness or confidence. No noise, geometry error, unknown field law, real recording or antialiasing qualification here. Prior paired noise/geometry ambiguity remains applicable. No production DSP, preset, audio asset or host change."}),
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
        return Err("nonlinear magnetic loss profile retained failed controls".into());
    }
    println!("Nonlinear magnetic loss profile: {}", output.display());
    Ok(())
}
