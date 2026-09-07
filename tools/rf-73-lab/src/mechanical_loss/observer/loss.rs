//! Profile unknown viscous scales while eliminating nuisance initial states.
use super::*;

pub const HELP: &str = "Unknown pickup loss study:
  pickup-loss --output REPORT.json
Fit structural loss from damper-off history, then damper loss with structural loss frozen.
Training-only state fits; held-out prediction, noise, local sensitivity and wrong-operator control.
";
const LOWER: f64 = 0.25;
const UPPER: f64 = 2.0;
const GRID_STEPS: usize = 16;
const REFINEMENTS: usize = 32;

#[derive(Serialize)]
struct Search {
    scale: f64,
    objective: f64,
    boundary_limited: bool,
    final_bracket: [f64; 2],
    evaluations: Vec<[f64; 2]>,
}

// Bounded coarse scan, then golden-section refinement of the best coarse bracket.
// No known scale, clean signal, held-out interval or previous-case answer is accepted.
fn search(
    mut objective: impl FnMut(f64) -> Result<f64, Box<dyn Error>>,
) -> Result<Search, Box<dyn Error>> {
    let mut evaluations = Vec::new();
    let mut sample = |x: f64| -> Result<f64, Box<dyn Error>> {
        let value = objective(x)?;
        if !value.is_finite() || value < 0.0 {
            return Err("non-finite or negative profile objective".into());
        }
        evaluations.push([x, value]);
        Ok(value)
    };
    let step = (UPPER - LOWER) / GRID_STEPS as f64;
    let mut best_index = 0;
    let mut best = f64::INFINITY;
    for index in 0..=GRID_STEPS {
        let value = sample(LOWER + step * index as f64)?;
        if value < best {
            best = value;
            best_index = index;
        }
    }
    let mut left = LOWER + step * best_index.saturating_sub(1) as f64;
    let mut right = LOWER + step * (best_index + 1).min(GRID_STEPS) as f64;
    let ratio = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut a = right - ratio * (right - left);
    let mut b = left + ratio * (right - left);
    let mut fa = sample(a)?;
    let mut fb = sample(b)?;
    for _ in 0..REFINEMENTS {
        if fa <= fb {
            right = b;
            b = a;
            fb = fa;
            a = right - ratio * (right - left);
            fa = sample(a)?;
        } else {
            left = a;
            a = b;
            fa = fb;
            b = left + ratio * (right - left);
            fb = sample(b)?;
        }
    }
    let best = *evaluations
        .iter()
        .min_by(|a, b| a[1].total_cmp(&b[1]))
        .unwrap();
    Ok(Search {
        scale: best[0],
        objective: best[1],
        boundary_limited: best[0] - LOWER < 1e-5 || UPPER - best[0] < 1e-5,
        final_bracket: [left, right],
        evaluations,
    })
}

struct Model<'a> {
    spectrum: &'a ModalSpectrum,
    structural: Matrix,
    damper: Matrix,
    rate: u32,
}
struct Candidate {
    observer: Observer,
    estimate: Estimate,
    normalized_residual: Vec<f64>,
}
impl Candidate {
    fn objective(&self) -> f64 {
        dot(&self.normalized_residual, &self.normalized_residual)
    }
}
impl Model<'_> {
    fn candidate(
        &self,
        alpha: f64,
        beta: f64,
        damped: bool,
        training: &[f64],
    ) -> Result<Candidate, Box<dyn Error>> {
        if !alpha.is_finite() || alpha <= 0.0 || !beta.is_finite() || beta <= 0.0 {
            return Err("invalid loss scale candidate".into());
        }
        let c = core::array::from_fn(|i| {
            core::array::from_fn(|j| {
                alpha * self.structural[i][j]
                    + if damped {
                        beta * self.damper[i][j]
                    } else {
                        0.0
                    }
            })
        });
        let observer = Observer::prepare(self.spectrum, &c, 9, self.rate)?;
        let columns = observer.columns(training.len());
        let estimate = infer(&columns, training)?;
        let norm = dot(training, training).sqrt();
        if !norm.is_finite() || norm <= 0.0 {
            return Err("loss inference needs an excited pickup history".into());
        }
        let normalized_residual = training
            .iter()
            .enumerate()
            .map(|(i, y)| {
                (columns
                    .iter()
                    .zip(&estimate.state)
                    .map(|(col, x)| col[i] * x)
                    .sum::<f64>()
                    - y)
                    / norm
            })
            .collect();
        Ok(Candidate {
            observer,
            estimate,
            normalized_residual,
        })
    }
}

struct Recovery {
    structural: Search,
    damper: Search,
    candidates: [Candidate; 2],
}
fn recover(model: &Model<'_>, training: &[Vec<f64>; 2]) -> Result<Recovery, Box<dyn Error>> {
    let structural = search(|a| Ok(model.candidate(a, 1.0, false, &training[0])?.objective()))?;
    let damper = search(|b| {
        Ok(model
            .candidate(structural.scale, b, true, &training[1])?
            .objective())
    })?;
    let candidates = [
        model.candidate(structural.scale, damper.scale, false, &training[0])?,
        model.candidate(structural.scale, damper.scale, true, &training[1])?,
    ];
    Ok(Recovery {
        structural,
        damper,
        candidates,
    })
}

// Refit nuisance states at every perturbation. This is local sensitivity of the
// profiled residual, not a confidence interval or a global identifiability proof.
fn sensitivity(
    model: &Model<'_>,
    training: &[Vec<f64>; 2],
    alpha: f64,
    beta: f64,
) -> Result<Value, Box<dyn Error>> {
    let mut derivatives: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for (parameter, derivative) in derivatives.iter_mut().enumerate() {
        for (index, samples) in training.iter().enumerate() {
            let plus = model.candidate(
                alpha * if parameter == 0 { 1.01 } else { 1.0 },
                beta * if parameter == 1 { 1.01 } else { 1.0 },
                index == 1,
                samples,
            )?;
            let minus = model.candidate(
                alpha * if parameter == 0 { 0.99 } else { 1.0 },
                beta * if parameter == 1 { 0.99 } else { 1.0 },
                index == 1,
                samples,
            )?;
            derivative.extend(
                plus.normalized_residual
                    .iter()
                    .zip(minus.normalized_residual)
                    .map(|(p, m)| (p - m) / (0.02 * 2.0_f64.sqrt())),
            );
        }
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
        "scope":"Central +/-1% parameter perturbations, nuisance initial states refitted each time, two training residual vectors normalized by their observed signal norms and sqrt(2). Derivatives approximate log-scale sensitivity. Local diagnostic only; no confidence interval or global uniqueness claim. Not used to select parameters."}),
    )
}

fn outcome(model: &Model<'_>, traces: &[Trace; 2], level: f64) -> Result<Value, Box<dyn Error>> {
    let training: [Vec<f64>; 2] = core::array::from_fn(|index| {
        let count = traces[index].pickup.len() / 2;
        let clean = &traces[index].pickup[..count];
        let rms = (dot(clean, clean) / count as f64).sqrt();
        clean
            .iter()
            .zip(noise(count))
            .map(|(y, n)| y + level * rms * n)
            .collect()
    });
    // From this boundary onward, recovery receives no trace or ground-truth scale.
    let result = recover(model, &training)?;
    let windows: Vec<_> = result
        .candidates
        .iter()
        .enumerate()
        .map(|(index, c)| evaluate(&c.observer, &c.estimate, &traces[index], &training[index]))
        .collect();
    let boundary = result.structural.boundary_limited || result.damper.boundary_limited;
    let local = sensitivity(
        model,
        &training,
        result.structural.scale,
        result.damper.scale,
    )?;
    let prediction_consistent = !boundary
        && local["locally_resolved"] == true
        && windows.iter().all(|w| {
            w["held_out_clean_pickup_relative_rmse"]
                .as_f64()
                .is_some_and(|x| x < 0.005)
        });
    Ok(
        json!({"estimated_structural_scale":result.structural.scale,"estimated_damper_scale":result.damper.scale,
        "structural_profile":result.structural,"conditional_damper_profile":result.damper,"windows":windows,
        "local_sensitivity":local,"prediction_consistent":prediction_consistent,
        "scope":"Structural estimate uses damper-off training only; damper estimate conditions on that fixed estimate. Separate nuisance initial states, nine modes. Prediction consistency uses clean synthetic held-out pickup, interior parameters and local sensitivity; it does not certify physical parameter correctness."}),
    )
}

fn study() -> Result<Value, Box<dyn Error>> {
    let s = ModalSpectrum::prepare(TineGeometry::default(), ModalAssemblyProfile::default())?;
    let assembly = |position| {
        ModalAssembly::new(
            48000.0,
            TineGeometry::default(),
            ModalAssemblyProfile {
                damper_position: position,
                ..ModalAssemblyProfile::default()
            },
            ModalIntegration::Refined {
                contact_substeps: 64,
            },
        )
    };
    let base = assembly(0.8)?;
    let wrong = assembly(0.7)?;
    let c0 = base.damping_matrix(false);
    let d0 = base.damping_matrix(true);
    let d_wrong = wrong.damping_matrix(true);
    let windows = [(0.02, 0.10), (0.14, 0.22)];
    let mut cases = Vec::new();
    // Off-grid truth values deliberately differ from both grid nodes and bounds.
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
                pickup: Vec::new(),
                truth: Vec::new(),
                contact_free: true,
            });
            let take =
                simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
                    for (trace, (start, end)) in traces.iter_mut().zip(windows) {
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
                })?;
            if traces.iter().any(|t| {
                !t.contact_free || t.pickup.len() != (0.08 * f64::from(rate)).round() as usize
            }) {
                return Err("invalid contact-free loss history".into());
            }
            let mut observations = Vec::new();
            for (label, level, position, on) in [
                ("matched_noiseless", 0.0, 0.8, &d0),
                ("matched_noise_1pct", 0.01, 0.8, &d0),
                ("wrong_damper_position", 0.0, 0.7, &d_wrong),
            ] {
                let model = Model {
                    spectrum: &s,
                    structural: c0,
                    damper: core::array::from_fn(|i| core::array::from_fn(|j| on[i][j] - c0[i][j])),
                    rate,
                };
                let mut fit = match outcome(&model, &traces, level) {
                    Ok(fit) => fit,
                    Err(e) => json!({"error":e.to_string()}),
                };
                let scale_errors: Option<[f64; 2]> = fit["estimated_structural_scale"]
                    .as_f64()
                    .zip(fit["estimated_damper_scale"].as_f64())
                    .map(|(a, b)| [(a / alpha - 1.0).abs(), (b / beta - 1.0).abs()]);
                let recovery = scale_errors.is_some_and(|e| e.iter().all(|x| *x < 0.01));
                fit["known_scale_relative_errors"] = json!(scale_errors);
                fit["known_scale_recovery_within_one_percent"] = json!(recovery);
                let required = label == "matched_noiseless";
                let passed = scale_errors.is_some_and(|e| e.iter().all(|x| *x < 0.001))
                    && fit["prediction_consistent"] == true
                    && fit["windows"].as_array().is_some_and(|w| {
                        w.len() == 2
                            && w.iter().all(|w| {
                                w["held_out_clean_pickup_relative_rmse"]
                                    .as_f64()
                                    .is_some_and(|e| e < 1e-5)
                            })
                    });
                observations.push(json!({"observation":label,"training_noise_relative_rms":level,"assumed_damper_position":position,
                    "required_control":required,"required_control_passed":if required {Some(passed)} else {None},"fit":fit}));
            }
            cases.push(json!({"known_structural_scale":alpha,"known_damper_scale":beta,"actual_damper_position":0.8,"diagnostics":take.diagnostics,
                "controls_passed":take.diagnostics["mechanical_checks_passed"]==true && observations.iter().filter(|o|o["required_control"]==true).all(|o|o["required_control_passed"]==true),"observations":observations}));
            println!("Pickup loss: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"profiled-pickup-loss-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),"cases":cases,
        "search_bounds":[LOWER,UPPER],"coarse_intervals":GRID_STEPS,"golden_refinements":REFINEMENTS,
        "fit_windows_seconds":[[0.02,0.06],[0.14,0.18]],"held_out_windows_seconds":[[0.06,0.10],[0.18,0.22]],
        "protocol":"Frozen before first run. Off-grid truth pairs (0.63,1.37),(1.13,0.57),(1.47,0.91), 48/96 kHz, same default untuned geometry/elastic hammer and binary damper event as mechanical-loss. Scalar pickup velocity, nine-mode exact free evolution. Search structural scale using only off training, then conditional damper scale using only on training with structural estimate frozen. Every candidate refits 18 nuisance initial-state coordinates by normalized reorthogonalized QR. Candidate objective is squared residual / observed training signal squared norm. Each search scans 17 nodes in [0.25,2], refines best coarse bracket by 32 golden iterations; includes endpoints and retains all 51 evaluations. No truth scale, held-out data or clean signal enters recovery. Noiseless matched model, 1% RMS deterministic training noise, and noiseless wrong damper position 0.7 instead of 0.8 share each trajectory. Same observer noise sequence; held-out signal stays clean. Central +/-1% refitted residual derivatives measure local two-parameter sensitivity, not confidence or global uniqueness. Prediction consistency: both held-out pickup errors <0.005, parameters away from bounds by >=1e-5, minimum sensitivity singular value >1e-8 and singular ratio >1e-4. Only matched noiseless controls require both scale errors <0.001, both held-out pickup errors <1e-5 and mechanical checks. Other results, biased fits and errors remain descriptive without retuning.",
        "scope":"Synthetic recovery of two global viscous multipliers with known mass, stiffness, damping shapes, linear mechanical observation gain and damper timing. Initial states are fitted independently per window. Sequential conditional estimates, not a joint maximum-likelihood fit. A finite coarse scan cannot prove a unique global optimum. Wrong-operator prediction may remain plausible while physical loss estimates are biased. No nonlinear magnetic pickup inversion, processed-bank calibration, independent modal losses, realtime integration or production changes."}),
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
        return Err("profiled pickup loss study retained failed controls".into());
    }
    println!("Profiled pickup loss: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_profile_resolves_off_grid_minimum_and_retains_boundary_controls() {
        let fit = search(|x| Ok((x - 0.7312345).powi(2))).unwrap();
        assert!((fit.scale - 0.7312345).abs() < 1e-7);
        assert!(!fit.boundary_limited);
        assert_eq!(fit.evaluations.len(), 51);
        assert!(
            fit.evaluations
                .iter()
                .all(|e| e[0] >= LOWER && e[0] <= UPPER)
        );
        for target in [0.0, 3.0] {
            let fit = search(|x| Ok((x - target).powi(2))).unwrap();
            assert!(fit.boundary_limited);
            assert_eq!(fit.scale, if target == 0.0 { LOWER } else { UPPER });
        }
        assert!(search(|_| Ok(f64::NAN)).is_err());
        assert!(search(|_| Ok(-1.0)).is_err());
        assert!(search(|_| Err("candidate failed".into())).is_err());
    }
    #[test]
    fn damper_scale_is_unobservable_without_damper_contact() {
        let spectrum =
            ModalSpectrum::prepare(TineGeometry::default(), ModalAssemblyProfile::default())
                .unwrap();
        let base = ModalAssembly::new(
            48000.0,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            ModalIntegration::Refined {
                contact_substeps: 64,
            },
        )
        .unwrap();
        let structural = base.damping_matrix(false);
        let on = base.damping_matrix(true);
        let model = Model {
            spectrum: &spectrum,
            structural,
            damper: core::array::from_fn(|i| core::array::from_fn(|j| on[i][j] - structural[i][j])),
            rate: 48000,
        };
        let training: Vec<_> = (0..1920).map(|i| (0.017 * f64::from(i)).sin()).collect();
        let a = model.candidate(0.7, LOWER, false, &training).unwrap();
        let b = model.candidate(0.7, UPPER, false, &training).unwrap();
        assert_eq!(a.normalized_residual, b.normalized_residual);
        assert_eq!(a.estimate.state, b.estimate.state);
        assert!(model.candidate(0.7, 1.0, false, &vec![0.0; 1920]).is_err());
    }
}
