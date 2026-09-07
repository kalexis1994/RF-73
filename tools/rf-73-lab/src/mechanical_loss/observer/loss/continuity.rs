//! Carry the off-window inferred state through the known damper event.
use super::geometry::{classify, compact, perturbations, prepare};
use super::*;

pub const HELP: &str = "Continuous-state pickup loss study:
  pickup-loss-continuity --output REPORT.json
One off-window initial state propagated through the known damper event; no on-window reset.
Paired comparison with independent-window fits on the same position-error grid.
";

fn advance(step: &Dense, mut ticks: usize, initial: &[f64]) -> Vec<f64> {
    let mut power = step.clone();
    let mut state = initial.to_vec();
    while ticks > 0 {
        if ticks % 2 == 1 {
            state = apply(&power, &state);
        }
        ticks /= 2;
        if ticks > 0 {
            power = multiply(&power, &power);
        }
    }
    state
}

fn on_candidate(
    model: &Model<'_>,
    alpha: f64,
    beta: f64,
    off: &Candidate,
    samples: &[f64],
) -> Result<Candidate, Box<dyn Error>> {
    // The initial state lives at 0.02 s; advance to 0.14 s without observing
    // any held-out or gap samples. Enabling a viscous damper applies no impulse.
    let ticks = (0.12 * f64::from(model.rate)).round() as usize;
    let event_state = advance(&off.observer.step, ticks, &off.estimate.state);
    let damping = core::array::from_fn(|i| {
        core::array::from_fn(|j| alpha * model.structural[i][j] + beta * model.damper[i][j])
    });
    let observer = Observer::prepare(model.spectrum, &damping, 9, model.rate)?;
    let norm = dot(samples, samples).sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return Err("continuous loss inference needs finite excited training".into());
    }
    let mut state = event_state.clone();
    let normalized_residual = samples
        .iter()
        .map(|y| {
            let residual = (dot(&observer.port, &state) - y) / norm;
            state = apply(&observer.step, &state);
            residual
        })
        .collect::<Vec<_>>();
    if normalized_residual.iter().any(|v| !v.is_finite()) {
        return Err("non-finite continuous-state prediction".into());
    }
    Ok(Candidate {
        observer,
        estimate: Estimate {
            state: event_state,
            minimum_normalized_qr_pivot: off.estimate.minimum_normalized_qr_pivot,
        },
        normalized_residual,
    })
}

fn recover_continuous(
    model: &Model<'_>,
    training: &[Vec<f64>; 2],
) -> Result<Recovery, Box<dyn Error>> {
    let structural = search(|a| Ok(model.candidate(a, 1.0, false, &training[0])?.objective()))?;
    let off = model.candidate(structural.scale, 1.0, false, &training[0])?;
    let damper =
        search(|b| Ok(on_candidate(model, structural.scale, b, &off, &training[1])?.objective()))?;
    let on = on_candidate(model, structural.scale, damper.scale, &off, &training[1])?;
    Ok(Recovery {
        structural,
        damper,
        candidates: [off, on],
    })
}

fn continuous_sensitivity(
    model: &Model<'_>,
    training: &[Vec<f64>; 2],
    alpha: f64,
    beta: f64,
) -> Result<Value, Box<dyn Error>> {
    let residual = |a, b| -> Result<Vec<f64>, Box<dyn Error>> {
        let off = model.candidate(a, b, false, &training[0])?;
        let on = on_candidate(model, a, b, &off, &training[1])?;
        Ok(off
            .normalized_residual
            .into_iter()
            .chain(on.normalized_residual)
            .collect())
    };
    let mut derivatives: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for (parameter, derivative) in derivatives.iter_mut().enumerate() {
        let plus = residual(
            alpha * if parameter == 0 { 1.01 } else { 1.0 },
            beta * if parameter == 1 { 1.01 } else { 1.0 },
        )?;
        let minus = residual(
            alpha * if parameter == 0 { 0.99 } else { 1.0 },
            beta * if parameter == 1 { 0.99 } else { 1.0 },
        )?;
        *derivative = plus
            .iter()
            .zip(minus)
            .map(|(p, m)| (p - m) / (0.02 * 2.0_f64.sqrt()))
            .collect();
    }
    let aa = dot(&derivatives[0], &derivatives[0]);
    let bb = dot(&derivatives[1], &derivatives[1]);
    let ab = dot(&derivatives[0], &derivatives[1]);
    let largest = 0.5 * (aa + bb + ((aa - bb).powi(2) + 4.0 * ab * ab).sqrt());
    let smallest = if largest > 0.0 {
        ((aa * bb - ab * ab) / largest).max(0.0)
    } else {
        0.0
    };
    let ratio = if largest > 0.0 {
        (smallest / largest).sqrt()
    } else {
        0.0
    };
    Ok(
        json!({"relative_parameter_perturbation":0.01,"profile_residual_singular_values":[largest.sqrt(),smallest.sqrt()],
        "minimum_to_maximum_singular_ratio":ratio,"locally_resolved":smallest.sqrt()>1e-8 && ratio>1e-4,
        "scope":"Central +/-1% scale perturbations. Refit the initial state from off training only, propagate it through the event, and concatenate normalized off/on training residuals divided by sqrt(2). No on-state refit. Local log-scale sensitivity, not confidence or global uniqueness."}),
    )
}

fn continuous_outcome(model: &Model<'_>, traces: &[Trace; 2]) -> Result<Value, Box<dyn Error>> {
    let training: [Vec<f64>; 2] =
        core::array::from_fn(|i| traces[i].pickup[..traces[i].pickup.len() / 2].to_vec());
    // Recovery sees training slices and assumed operators, not truth or held-out data.
    let result = recover_continuous(model, &training)?;
    let mut windows: Vec<_> = result
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| evaluate(&c.observer, &c.estimate, &traces[i], &training[i]))
        .collect();
    let on = windows[1].as_object_mut().unwrap();
    let pivot = on.remove("minimum_normalized_qr_pivot").unwrap();
    on.insert(
        "source_off_state_fit_minimum_normalized_qr_pivot".into(),
        pivot,
    );
    on.insert(
        "state_origin".into(),
        json!("Propagated from 0.02 s through the 0.14 s damper event; no on-window state fit."),
    );
    let local = continuous_sensitivity(
        model,
        &training,
        result.structural.scale,
        result.damper.scale,
    )?;
    let consistent = !result.structural.boundary_limited
        && !result.damper.boundary_limited
        && local["locally_resolved"] == true
        && windows.iter().all(|w| {
            w["held_out_clean_pickup_relative_rmse"]
                .as_f64()
                .is_some_and(|v| v < 0.005)
        });
    Ok(compact(
        json!({"estimated_structural_scale":result.structural.scale,"estimated_damper_scale":result.damper.scale,
        "structural_profile":result.structural,"conditional_damper_profile":result.damper,"windows":windows,
        "local_sensitivity":local,"prediction_consistent":consistent,"fitted_initial_state_count":1,
        "scope":"Sequential scales with a single initial state at 0.02 s. Off training determines structural loss and state, both frozen before searching damper loss. Propagation to 0.14 s reads no signal in the intervening gap. The same event state starts every on candidate. This is not joint optimization of both losses and initial state."}),
    ))
}

fn reference_summary(fit: &Value, alpha: f64, beta: f64) -> Value {
    json!({"estimated_structural_scale":fit["estimated_structural_scale"],"estimated_damper_scale":fit["estimated_damper_scale"],
        "prediction_consistent":fit["prediction_consistent"],"classification":classify(fit,alpha,beta),
        "held_out_pickup_relative_rmse":fit["windows"].as_array().map(|w|w.iter().map(|w|w["held_out_clean_pickup_relative_rmse"].clone()).collect::<Vec<_>>()),
        "error":fit["error"]})
}

fn study() -> Result<Value, Box<dyn Error>> {
    let prepared = perturbations()
        .into_iter()
        .map(prepare)
        .collect::<Result<Vec<_>, _>>()?;
    let s = &prepared[0].spectrum;
    let mut cases = Vec::new();
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
                pickup: Vec::new(),
                truth: Vec::new(),
                contact_free: true,
            });
            let take =
                simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
                    for (trace, (start, end)) in traces.iter_mut().zip([(0.02, 0.10), (0.14, 0.22)])
                    {
                        if tick >= (start * tick_rate as f64).round() as usize
                            && tick < (end * tick_rate as f64).round() as usize
                        {
                            trace.contact_free &= !before.contact_active && !after.contact_active;
                            if tick % (tick_rate / rate as usize) == 0 {
                                trace.pickup.push(before.pickup_velocity_m_s);
                                trace.truth.push(reference_state(s, before));
                            }
                        }
                    }
                })?;
            if traces.iter().any(|t| {
                !t.contact_free || t.pickup.len() != (0.08 * f64::from(rate)).round() as usize
            }) {
                return Err("invalid continuous loss history".into());
            }
            // The previously verified simulator separates before both windows;
            // also require the unobserved propagation interval to contain no contact.
            let gap_contact_free = take.diagnostics["last_contact_seconds"]
                .as_f64()
                .is_some_and(|t| t < 0.02);
            let mut observations = Vec::new();
            for prepared in &prepared {
                let p = prepared.perturbation;
                let model = Model {
                    spectrum: &prepared.spectrum,
                    structural: prepared.structural,
                    damper: prepared.damper,
                    rate,
                };
                let fit = if prepared.invariants_passed && gap_contact_free {
                    match continuous_outcome(&model, &traces) {
                        Ok(f) => f,
                        Err(e) => json!({"error":e.to_string()}),
                    }
                } else {
                    json!({"error":"invalid continuous mechanics assumptions"})
                };
                let independent = match outcome(&model, &traces, 0.0) {
                    Ok(f) => reference_summary(&f, alpha, beta),
                    Err(e) => json!({"error":e.to_string()}),
                };
                let classification = classify(&fit, alpha, beta);
                let required = p.family == "matched";
                let passed = classification["known_scale_relative_errors"]
                    .as_array()
                    .is_some_and(|v| {
                        v.len() == 2 && v.iter().all(|x| x.as_f64().is_some_and(|e| e < 0.001))
                    })
                    && fit["prediction_consistent"] == true
                    && fit["windows"].as_array().is_some_and(|w| {
                        w.len() == 2
                            && w.iter().all(|w| {
                                w["held_out_clean_pickup_relative_rmse"]
                                    .as_f64()
                                    .is_some_and(|e| e < 1e-5)
                            })
                    });
                observations.push(json!({"perturbation":p,"damper_offset_mm":p.damper_offset*75.0,"pickup_offset_mm":p.pickup_offset*75.0,
                    "operator_invariants_passed":prepared.invariants_passed,"required_control":required,"required_control_passed":if required{Some(passed)}else{None},
                    "classification":classification,"fit":fit,"independent_window_reference":independent}));
            }
            cases.push(json!({"known_structural_scale":alpha,"known_damper_scale":beta,"diagnostics":take.diagnostics,"propagation_gap_contact_free":gap_contact_free,
                "controls_passed":gap_contact_free && take.diagnostics["mechanical_checks_passed"]==true && observations.iter().all(|o|o["operator_invariants_passed"]==true && (o["required_control"]!=true || o["required_control_passed"]==true)),"observations":observations}));
            println!("Continuous pickup loss: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    let rows: Vec<_> = cases
        .iter()
        .flat_map(|c| c["observations"].as_array().unwrap())
        .collect();
    let count = |f: fn(&Value) -> bool| rows.iter().filter(|o| f(o)).count();
    Ok(
        json!({"schema_version":1,"experiment":"continuous-pickup-loss-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),
        "summary":{"paired_observations":rows.len(),"continuous_prediction_consistent":count(|o|o["fit"]["prediction_consistent"]==true),"continuous_scale_recovery":count(|o|o["classification"]["known_scale_recovery_within_one_percent"]==true),
            "continuous_consistent_but_biased":count(|o|o["classification"]["prediction_consistent_but_biased"]==true),"independent_prediction_consistent":count(|o|o["independent_window_reference"]["prediction_consistent"]==true),
            "independent_consistent_but_biased":count(|o|o["independent_window_reference"]["classification"]["prediction_consistent_but_biased"]==true),"fit_errors":count(|o|o["fit"]["error"].is_string())},"cases":cases,
        "protocol":"Frozen before first run. Same six noiseless off-grid loss trajectories and 17 assumed-operator position variants as pickup-loss-geometry. Fit structural scale and initial state on [0.02,0.06); freeze both, propagate from 0.02 to the known 0.14 s event, then search damper scale using [0.14,0.18) with the propagated state and no reset or on-state refit. Reject contact anywhere in the propagation gap. Propagate one sample-step matrix by integer binary exponentiation to the event. All on candidates start with identical q,v; the viscous event supplies no impulse. Held-out [0.06,0.10) and [0.18,0.22) remain unused. Same 51-evaluation bounded searches, compact 17-node histories and numerical operator checks. Local sensitivity refits off state only at +/-1% scales and propagates it through the event. Same numerical gates: each held-out pickup RMSE <0.005, interior losses, local singular minimum >1e-8 and ratio >1e-4. Required matched controls additionally need both scale errors <0.001, both pickup errors <1e-5 and mechanical checks. Rerun the independent-window inverse on each exact same trajectory for a paired comparison. Every perturbation outcome retained; no gate retuning.",
        "scope":"One initial state and known damper timing, sequential conditional scale estimation, not a joint state/parameter optimizer. Geometry remains assumed, with fixed longitudinal-error grid. No guarantee that state continuity alone identifies pickup/contact position. Independent and continuous sensitivities correspond to their own nuisance-state constraints. Clean synthetic mechanical velocity, not magnetic voltage or noisy recordings. No production changes, waveform assets or host launch."}),
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
        return Err("continuous pickup loss study retained failed controls".into());
    }
    println!("Continuous pickup loss: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn analytic(omega: f64, loss: f64, t: f64, x: &[f64]) -> Vec<f64> {
        let w = (omega * omega - loss * loss).sqrt();
        let sine = (w * t).sin() / w;
        let cosine = (w * t).cos();
        let f = (-loss * t).exp();
        vec![
            f * ((cosine + loss * sine) * x[0] + omega * sine * x[1]),
            f * (-omega * sine * x[0] + (cosine - loss * sine) * x[1]),
        ]
    }
    #[test]
    fn switched_damping_preserves_event_state_and_matches_analytic_piecewise_motion() {
        let omega = 400.0;
        let initial = [0.01, -0.02];
        for rate in [48000, 96000] {
            let step = |loss| {
                transition(
                    &vec![vec![0.0, omega], vec![-omega, -2.0 * loss]],
                    1.0 / f64::from(rate),
                )
                .unwrap()
            };
            let event = advance(
                &step(3.0),
                (0.12 * f64::from(rate)).round() as usize,
                &initial,
            );
            let exact_event = analytic(omega, 3.0, 0.12, &initial);
            for (a, b) in event.iter().zip(&exact_event) {
                assert!((a - b).abs() < 1e-12);
            }
            assert_eq!(advance(&step(17.0), 0, &event), event);
            let after = advance(
                &step(17.0),
                (0.04 * f64::from(rate)).round() as usize,
                &event,
            );
            let exact = analytic(omega, 17.0, 0.04, &exact_event);
            for (a, b) in after.iter().zip(exact) {
                assert!((a - b).abs() < 1e-12);
            }
            // Restarting from the original state at the event would be incorrect.
            let reset = analytic(omega, 17.0, 0.04, &initial);
            assert!((after[0] - reset[0]).abs() > 1e-3);
        }
    }
    #[test]
    fn every_damper_candidate_starts_from_the_same_off_inferred_state() {
        let p = prepare(perturbations()[0]).unwrap();
        let model = Model {
            spectrum: &p.spectrum,
            structural: p.structural,
            damper: p.damper,
            rate: 48000,
        };
        let samples: Vec<_> = (0..1920).map(|i| (f64::from(i) * 0.017).sin()).collect();
        let off = model.candidate(0.7, 1.0, false, &samples).unwrap();
        let a = on_candidate(&model, 0.7, LOWER, &off, &samples).unwrap();
        let b = on_candidate(&model, 0.7, UPPER, &off, &samples).unwrap();
        assert_eq!(a.estimate.state, b.estimate.state);
        assert_eq!(a.normalized_residual[0], b.normalized_residual[0]);
        assert_ne!(a.normalized_residual[100], b.normalized_residual[100]);
        assert!(on_candidate(&model, 0.7, 1.0, &off, &vec![0.0; 1920]).is_err());
    }
}
