//! Paired known/unknown loss inference from identical noisy magnetic histories.
use super::*;

pub const HELP: &str = "Noisy nonlinear magnetic loss study:
  magnetic-loss-noise --output REPORT.json
Known geometry, unknown losses; paired fixed-loss state control on identical noise.
";

fn start_agreement(attempts: &[Value]) -> Value {
    let complete = attempts
        .iter()
        .filter_map(|a| {
            let s = a["scales"].as_array()?;
            Some([s[0].as_f64()?, s[1].as_f64()?])
        })
        .collect::<Vec<_>>();
    if complete.len() != 2 {
        return json!({"status":"withheld_incomplete_starts"});
    }
    let spread: [f64; 2] = core::array::from_fn(|j| {
        (complete[0][j] - complete[1][j]).abs() / complete[0][j].abs().max(complete[1][j].abs())
    });
    json!({"status":"compared","relative_spread":spread,"within_one_percent":spread.iter().all(|x|*x<0.01)})
}

struct Observation<'a> {
    traces: &'a [Trace; 2],
    clean: &'a [Vec<f64>; 2],
    measured: &'a [Vec<f64>; 2],
    reference_scales: [f64; 2],
}
fn validation(
    t: &Templates,
    sensor: Sensor,
    c: &Candidate,
    observation: &Observation<'_>,
) -> Value {
    let clean = score(t, sensor, &c.state, observation.traces, observation.clean);
    let measured = score(
        t,
        sensor,
        &c.state,
        observation.traces,
        observation.measured,
    );
    let mut windows = Vec::new();
    for i in 0..2 {
        let start = observation.clean[i].len() / 2;
        let signal = dot(
            &observation.measured[i][start..],
            &observation.measured[i][start..],
        );
        let noise: f64 = observation.clean[i][start..]
            .iter()
            .zip(&observation.measured[i][start..])
            .map(|(a, b)| (a - b).powi(2))
            .sum();
        let noise_relative = (noise / signal).sqrt();
        let threshold = (1.25 * noise_relative).max(1e-6);
        let error = measured["windows"][i]["held_out_voltage_relative_rmse"].as_f64();
        windows.push(json!({"clean_voltage_relative_rmse":clean["windows"][i]["held_out_voltage_relative_rmse"],
            "state_energy_norm_relative_rmse":clean["windows"][i]["held_out_state_energy_norm_relative_rmse"],
            "measured_voltage_relative_rmse":error,"injected_noise_relative_rmse":noise_relative,
            "oracle_prediction_threshold":threshold,"oracle_prediction_consistent":error.is_some_and(|e|e<threshold)}));
    }
    let relative: [f64; 2] =
        core::array::from_fn(|j| (c.scales[j] / observation.reference_scales[j] - 1.0).abs());
    let boundary = c
        .scales
        .iter()
        .any(|x| x - LOWER < 1e-5 || UPPER - x < 1e-5);
    let prediction = !boundary
        && windows
            .iter()
            .all(|w| w["oracle_prediction_consistent"] == true);
    let loss = relative.iter().all(|e| *e < 0.01);
    json!({"windows":windows,"boundary_limited":boundary,"prediction_consistent":prediction,
        "relative_loss_errors":relative,"both_losses_within_one_percent":loss,
        "state_within_one_percent":windows.iter().all(|w|w["state_energy_norm_relative_rmse"].as_f64().is_some_and(|e|e<0.01)),
        "prediction_consistent_loss_error":prediction && !loss,"strict_state_recovery":clean["known_state_recovery"]})
}

fn study() -> Result<Value, Box<dyn Error>> {
    let p = prepare(perturbations()[0])?;
    let sensor = sensors()?
        .into_iter()
        .find(|s| s.law == "production" && s.geometry == "baseline")
        .ok_or("missing baseline sensor")?;
    let conditions = [
        (None, 0),
        (Some(40.0), 17),
        (Some(40.0), 71),
        (Some(20.0), 17),
        (Some(20.0), 71),
    ];
    let mut cases = Vec::new();
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
                pickup: Vec::new(),
                truth: Vec::new(),
                contact_free: true,
            });
            let mut clean: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
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
                                traces[i].truth.push(reference_state(&p.spectrum, before));
                                clean[i].push(sensor.voltage(
                                    before.pickup_displacement_m,
                                    before.pickup_velocity_m_s,
                                ));
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
                return Err("invalid noisy loss history".into());
            }
            let mut rows = Vec::new();
            for (snr, seed) in conditions {
                let (measured, sigma) = robustness::additive_voltage_noise(&clean, snr, seed);
                let mut profile = Profile {
                    spectrum: &p.spectrum,
                    structural: &p.structural,
                    damper: &p.damper,
                    sensor,
                    rate,
                    training: core::array::from_fn(|i| {
                        measured[i][..measured[i].len() / 2].to_vec()
                    }),
                    evaluations: Vec::new(),
                };
                // Finish unknown-loss selection before preparing the oracle comparator.
                let fit = match recover(&mut profile) {
                    Ok((candidate, mut attempts, selected)) => {
                        let t = templates(
                            &p.spectrum,
                            &p.structural,
                            &p.damper,
                            candidate.scales[0],
                            candidate.scales[1],
                            rate,
                        )?;
                        let scored = validation(
                            &t,
                            sensor,
                            &candidate,
                            &Observation {
                                traces: &traces,
                                clean: &clean,
                                measured: &measured,
                                reference_scales: [alpha, beta],
                            },
                        );
                        for attempt in &mut attempts {
                            if let Some(scales) = attempt["scales"].as_array() {
                                let errors = [
                                    (scales[0].as_f64().ok_or("missing structural scale")? / alpha
                                        - 1.0)
                                        .abs(),
                                    (scales[1].as_f64().ok_or("missing damper scale")? / beta
                                        - 1.0)
                                        .abs(),
                                ];
                                attempt["relative_loss_errors_for_scoring_only"] = json!(errors);
                            }
                        }
                        let agreement = start_agreement(&attempts);
                        json!({"estimated_scales":candidate.scales,"training_relative_rmse":candidate.objective.sqrt(),
                            "selected_loss_start_index":selected,"attempts":attempts,"validation":scored,"loss_start_agreement":agreement,
                            "agreement_hides_loss_error":agreement["within_one_percent"]==true && scored["prediction_consistent_loss_error"]==true})
                    }
                    Err(e) => json!({"error":e.to_string()}),
                };
                let known_t = templates(&p.spectrum, &p.structural, &p.damper, alpha, beta, rate)?;
                let known = match robustness::outcome(&known_t, sensor, &traces, &clean, &measured)
                {
                    Ok(v) => v,
                    Err(e) => json!({"error":e.to_string()}),
                };
                let pair = if fit["validation"].is_object() && known["windows"].is_array() {
                    json!({"status":"compared","known_loss_state_within_one_percent":known["state_within_one_percent"],
                        "unknown_loss_state_within_one_percent":fit["validation"]["state_within_one_percent"],
                        "new_hidden_state_error_when_freeing_losses":known["state_within_one_percent"]==true && fit["validation"]["prediction_consistent"]==true && fit["validation"]["state_within_one_percent"]==false})
                } else {
                    json!({"status":"withheld_incomplete_fit"})
                };
                let required = snr.is_none();
                let passed = fit["validation"]["prediction_consistent"] == true
                    && fit["validation"]["strict_state_recovery"] == true
                    && fit["validation"]["both_losses_within_one_percent"] == true
                    && known["strict_matched_recovery"] == true;
                rows.push(json!({"snr_db":snr,"seed":seed,"noise_standard_deviation":sigma,"required_control":required,
                    "required_control_passed":if required{Some(passed)}else{None},"fit":fit,"known_loss_state_control":known,"paired_state_comparison":pair,
                    "profile_evaluations":profile.evaluations}));
                println!(
                    "Noisy magnetic loss: {rate} Hz, reference ({alpha}, {beta}), SNR {snr:?}, seed {seed}"
                );
            }
            cases.push(json!({"sample_rate":rate,"reference_scales_for_scoring_only":[alpha,beta],"diagnostics":take.diagnostics,
                "controls_passed":take.diagnostics["mechanical_checks_passed"]==true && rows.iter().filter(|r|r["required_control"]==true).all(|r|r["required_control_passed"]==true),"observations":rows}));
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-loss-noise-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),"sensor":sensor,"cases":cases,
        "protocol":"Frozen before first run. Six known-geometry production-baseline trajectories from nonlinear-magnetic-loss-profile-v1. Each has a noiseless control and constant-voltage Gaussian noise at nominal 40/20 dB, seeds 17/71: 30 fits. Reuse SplitMix64/Box-Muller with sigma derived solely from the first clean training window; same sample sequence as prior robustness studies. Unknown-loss inverse and all inner/outer starts, bounds, finite differences, budgets and objective weights unchanged. It owns measured training voltage only. After selection, evaluate clean/measured held-out voltage, full state and relative loss errors separately. Oracle noise-aware prediction gate is each measured held-out RMSE < max(1e-6,1.25*actual injected noise relative RMS), plus no boundary hit; this uses synthetic noise, not a recording acceptance rule. State and both loss errors each scored at 1%; only six noiseless controls require original strict state/voltage gates and loss recovery. A separate state-only inverse receives true losses on the identical measured waveform after unknown-loss selection; paired flags distinguish newly hidden state error when freeing losses. Outer-start agreement uses abs(a-b)/max(abs(a),abs(b)) <1% for each scale; no claim of confidence from agreement. Both starts retain reference loss errors for scoring only. Failures and incomplete comparisons retained; no retuning from noise results.",
        "scope":"Known geometry and law, unknown losses; no source calibration or inferred confidence intervals. Two noise realizations and three loss pairs are limited coverage. SNR nominal at first training interval only, later damped signal may be noise-dominated. Sensor uncertainty, gain/timing errors and antialiasing remain unqualified. No production DSP, preset, audio asset or host change."}),
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
        return Err("noisy magnetic loss study retained failed controls".into());
    }
    println!("Noisy magnetic loss: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loss_start_agreement_is_symmetric_and_withholds_missing_results() {
        let a = json!({"scales":[0.5,1.5]});
        let b = json!({"scales":[0.504,1.49]});
        assert_eq!(
            start_agreement(&[a.clone(), b.clone()])["within_one_percent"],
            true
        );
        assert_eq!(
            start_agreement(&[a.clone(), b.clone()]),
            start_agreement(&[b, a.clone()])
        );
        assert_eq!(
            start_agreement(&[a.clone(), json!({"error":"failed"})])["status"],
            "withheld_incomplete_starts"
        );
        assert_eq!(
            start_agreement(&[a, json!({"scales":[0.6,1.5]})])["within_one_percent"],
            false
        );
    }
}
