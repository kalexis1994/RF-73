//! Fixed position-error grid; prediction acceptance is not parameter validation.
use super::*;

pub const HELP: &str = "Pickup loss position-error study:
  pickup-loss-geometry --output REPORT.json
Small longitudinal damper/pickup position errors, alone and combined, with fixed loss gates.
Noiseless shared trajectories; retain prediction-consistent but biased fits. Not magnetic gap.
";

#[derive(Clone, Copy, Serialize)]
pub(super) struct Perturbation {
    pub(super) family: &'static str,
    pub(super) damper_offset: f64,
    pub(super) pickup_offset: f64,
}
pub(super) fn perturbations() -> Vec<Perturbation> {
    let mut values = vec![Perturbation {
        family: "matched",
        damper_offset: 0.0,
        pickup_offset: 0.0,
    }];
    for offset in [-0.01, -0.005, -0.001, 0.001, 0.005, 0.01] {
        values.push(Perturbation {
            family: "damper_only",
            damper_offset: offset,
            pickup_offset: 0.0,
        });
        values.push(Perturbation {
            family: "pickup_only",
            damper_offset: 0.0,
            pickup_offset: offset,
        });
    }
    for damper in [-0.005, 0.005] {
        for pickup in [-0.005, 0.005] {
            values.push(Perturbation {
                family: "combined",
                damper_offset: damper,
                pickup_offset: pickup,
            });
        }
    }
    values
}

pub(super) struct Prepared {
    pub(super) perturbation: Perturbation,
    pub(super) spectrum: ModalSpectrum,
    pub(super) structural: Matrix,
    pub(super) damper: Matrix,
    pub(super) invariants_passed: bool,
}
pub(super) fn prepare(p: Perturbation) -> Result<Prepared, Box<dyn Error>> {
    let geometry = TineGeometry {
        pickup_position: 0.98 + p.pickup_offset,
        ..TineGeometry::default()
    };
    let profile = ModalAssemblyProfile {
        damper_position: 0.8 + p.damper_offset,
        ..ModalAssemblyProfile::default()
    };
    let spectrum = ModalSpectrum::prepare(geometry, profile)?;
    let baseline_spectrum =
        ModalSpectrum::prepare(TineGeometry::default(), ModalAssemblyProfile::default())?;
    let assembly = |g, profile| {
        ModalAssembly::new(
            48000.0,
            g,
            profile,
            ModalIntegration::Refined {
                contact_substeps: 64,
            },
        )
    };
    let changed = assembly(geometry, profile)?;
    let baseline = assembly(TineGeometry::default(), ModalAssemblyProfile::default())?;
    let structural = changed.damping_matrix(false);
    let on = changed.damping_matrix(true);
    // These offsets must not silently become mass/stiffness/tuning changes.
    // Unchanged eigenshapes also permit direct state scoring in the shared basis.
    let invariants_passed = changed.mass_matrix() == baseline.mass_matrix()
        && changed.stiffness_matrix() == baseline.stiffness_matrix()
        && structural == baseline.damping_matrix(false)
        && spectrum
            .modes
            .iter()
            .zip(&baseline_spectrum.modes)
            .all(|(a, b)| a.frequency_hz == b.frequency_hz && a.shape == b.shape);
    Ok(Prepared {
        perturbation: p,
        spectrum,
        structural,
        damper: core::array::from_fn(|i| core::array::from_fn(|j| on[i][j] - structural[i][j])),
        invariants_passed,
    })
}

// Keep every fit and coarse node, but not 34 local refinement evaluations per
// scale. The fitted result, search bounds/bracket and evaluation count remain.
pub(super) fn compact(mut fit: Value) -> Value {
    for name in ["structural_profile", "conditional_damper_profile"] {
        if let Some(profile) = fit[name].as_object_mut()
            && let Some(Value::Array(mut evaluations)) = profile.remove("evaluations")
        {
            profile.insert("evaluation_count".into(), json!(evaluations.len()));
            evaluations.truncate(GRID_STEPS + 1);
            profile.insert("coarse_evaluations".into(), json!(evaluations));
        }
    }
    fit
}

pub(super) fn classify(fit: &Value, alpha: f64, beta: f64) -> Value {
    let errors = fit["estimated_structural_scale"]
        .as_f64()
        .zip(fit["estimated_damper_scale"].as_f64())
        .map(|(a, b)| [(a / alpha - 1.0).abs(), (b / beta - 1.0).abs()]);
    let recovery = errors.is_some_and(|e| e.iter().all(|x| *x < 0.01));
    let consistent = fit["prediction_consistent"] == true;
    json!({"known_scale_relative_errors":errors,"known_scale_recovery_within_one_percent":recovery,
        "prediction_consistent_but_biased":consistent && errors.is_some() && !recovery})
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
                return Err("invalid position-error contact-free history".into());
            }
            let mut observations = Vec::new();
            for prepared in &prepared {
                let p = prepared.perturbation;
                let model = Model {
                    spectrum: &prepared.spectrum,
                    structural: prepared.structural,
                    damper: prepared.damper,
                    rate,
                };
                let fit = if prepared.invariants_passed {
                    match outcome(&model, &traces, 0.0) {
                        Ok(fit) => compact(fit),
                        Err(e) => json!({"error":e.to_string()}),
                    }
                } else {
                    json!({"error":"position perturbation changed mechanical invariants"})
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
                    "assumed_damper_position":0.8+p.damper_offset,"assumed_pickup_position":0.98+p.pickup_offset,
                    "operator_invariants_passed":prepared.invariants_passed,"required_control":required,"required_control_passed":if required {Some(passed)} else {None},"classification":classification,"fit":fit}));
            }
            cases.push(json!({"known_structural_scale":alpha,"known_damper_scale":beta,"actual_damper_position":0.8,"actual_pickup_position":0.98,
                "diagnostics":take.diagnostics,"controls_passed":take.diagnostics["mechanical_checks_passed"]==true && observations.iter().all(|o|o["operator_invariants_passed"]==true && (o["required_control"]!=true || o["required_control_passed"]==true)),"observations":observations}));
            println!("Pickup loss positions: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    let rows: Vec<_> = cases
        .iter()
        .flat_map(|c| c["observations"].as_array().unwrap())
        .collect();
    let count = |predicate: fn(&Value) -> bool| rows.iter().filter(|o| predicate(o)).count();
    Ok(
        json!({"schema_version":1,"experiment":"pickup-loss-position-errors-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),
        "summary":{"observations":rows.len(),"prediction_consistent":count(|o|o["fit"]["prediction_consistent"]==true),"known_scale_recovery":count(|o|o["classification"]["known_scale_recovery_within_one_percent"]==true),"prediction_consistent_but_biased":count(|o|o["classification"]["prediction_consistent_but_biased"]==true),"fit_errors":count(|o|o["fit"]["error"].is_string())},"cases":cases,
        "search_bounds":[LOWER,UPPER],"coarse_intervals":GRID_STEPS,"golden_refinements":REFINEMENTS,"training_noise_relative_rms":0.0,
        "fit_windows_seconds":[[0.02,0.06],[0.14,0.18]],"held_out_windows_seconds":[[0.06,0.10],[0.18,0.22]],
        "protocol":"Frozen before first run. Same six profiled-pickup-loss trajectories, true scales (0.63,1.37),(1.13,0.57),(1.47,0.91) at 48/96 kHz. Seventeen assumed operators per trajectory: matched; damper-only and pickup-only offsets -0.01,-0.005,-0.001,+0.001,+0.005,+0.01; all four combined +/-0.005 pairs. Positions are longitudinal fractions of the 75 mm tine, true damper 0.8 and pickup 0.98; not magnetic gap or vertical alignment. Perturb assumptions only, not generated observations. Require mass, stiffness, structural damping, undamped eigenshapes and frequencies unchanged. Pickup offsets change only observation weights; damper offsets change the spatial damper operator. Noiseless training isolates operator bias. Reuse unchanged sequential structural/conditional-damper searches, nuisance-state QR, two fixed fit/held-out windows, sensitivity and prediction gates. Every search still performs 51 evaluations; receipt keeps 17 coarse nodes, evaluation count, selected result and final bracket. Prediction-consistent-but-biased means existing prediction consistency passes while either known scale error is >=1%; ground truth only scores. Six matched controls require scale errors <0.001, both held-out pickup errors <1e-5 and original mechanical checks. All perturbation results, rejections and fit errors retained descriptively; no threshold tuning or inferred manufacturing tolerance.",
        "scope":"Synthetic sensitivity to small assumed longitudinal observation/contact position errors, nine modes and two unknown global viscous multipliers. Independent nuisance states can absorb some observation mismatch. Conditional loss sensitivity measures resolution within each assumed model, not geometry correctness. No guarantee for unsampled offsets, other keys, noise combinations, contact materials or real recordings. Magnetic flux/voltage conversion remains untested by this command. No production equations, presets, audio assets or host changes."}),
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
        return Err("pickup loss position study retained failed controls".into());
    }
    println!("Pickup loss positions: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn longitudinal_offsets_change_only_the_intended_port_or_damping_operator() {
        let all = perturbations()
            .into_iter()
            .map(prepare)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(all.len(), 17);
        let base = &all[0];
        for p in &all {
            assert!(p.invariants_passed);
            let pickup_same = p
                .spectrum
                .modes
                .iter()
                .zip(&base.spectrum.modes)
                .all(|(a, b)| a.pickup_weight == b.pickup_weight);
            assert_eq!(pickup_same, p.perturbation.pickup_offset == 0.0);
            assert_eq!(p.damper == base.damper, p.perturbation.damper_offset == 0.0);
        }
    }
    #[test]
    fn prediction_acceptance_does_not_hide_parameter_bias_or_count_failed_fits() {
        let fit = json!({"prediction_consistent":true,"estimated_structural_scale":1.0,"estimated_damper_scale":1.02});
        let classified = classify(&fit, 1.0, 1.0);
        assert_eq!(classified["prediction_consistent_but_biased"], true);
        assert_eq!(classified["known_scale_recovery_within_one_percent"], false);
        assert_eq!(
            classify(&fit, 1.0, 1.02)["prediction_consistent_but_biased"],
            false
        );
        assert_eq!(
            classify(&json!({"error":"rank deficient"}), 1.0, 1.0)["prediction_consistent_but_biased"],
            false
        );
    }
}
