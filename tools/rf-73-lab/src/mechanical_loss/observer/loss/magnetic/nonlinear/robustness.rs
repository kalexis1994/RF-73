//! Fixed noise and sensor mismatch qualification; mechanical losses remain known.
use super::*;

pub const HELP: &str = "Magnetic state robustness study:
  magnetic-state-robustness --output REPORT.json
Fixed additive noise and assumed sensor mismatch; known losses, held-out state checks.
";

#[derive(Clone, Copy, Serialize)]
struct Condition {
    name: &'static str,
    snr_db: Option<f64>,
    seed: u64,
    gap_scale: f64,
    offset_scale: f64,
    swap_law: bool,
}
fn conditions() -> Vec<Condition> {
    let matched = Condition {
        name: "matched",
        snr_db: None,
        seed: 0,
        gap_scale: 1.0,
        offset_scale: 1.0,
        swap_law: false,
    };
    let mut rows = vec![matched];
    for snr in [60.0, 40.0, 20.0] {
        for seed in [17, 71] {
            rows.push(Condition {
                name: "additive_noise",
                snr_db: Some(snr),
                seed,
                ..matched
            });
        }
    }
    for scale in [0.95, 1.05] {
        rows.push(Condition {
            name: "assumed_gap",
            gap_scale: scale,
            ..matched
        });
        rows.push(Condition {
            name: "assumed_offset",
            offset_scale: scale,
            ..matched
        });
    }
    rows.push(Condition {
        name: "assumed_field_law",
        swap_law: true,
        ..matched
    });
    rows
}
fn assumed_sensor(s: Sensor, c: Condition) -> Result<Sensor, Box<dyn Error>> {
    let gap_m = s.gap_m * c.gap_scale;
    let offset_m = s.offset_m * c.offset_scale;
    Ok(Sensor {
        law: if c.swap_law {
            if s.law == "production" {
                "point_pole_proxy"
            } else {
                "production"
            }
        } else {
            s.law
        },
        gap_m,
        offset_m,
        pickup: MagneticPickup::new(gap_m, offset_m)?,
        ..s
    })
}

// SplitMix64 and Box-Muller: deterministic independent Gaussian samples. The
// scale is derived only from the first clean training window and is held fixed
// across both full windows. No full-window renormalization or sample recycling.
struct Noise(u64);
impl Noise {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        ((z >> 12) as f64 + 0.5) / 4503599627370496.0
    }
    fn normal(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (TAU * self.uniform()).cos()
    }
}
fn corrupt(clean: &[Vec<f64>; 2], c: Condition) -> ([Vec<f64>; 2], f64) {
    let sigma = c.snr_db.map_or(0.0, |snr| {
        let train = &clean[0][..clean[0].len() / 2];
        (dot(train, train) / train.len() as f64).sqrt() * 10.0_f64.powf(-snr / 20.0)
    });
    let mut noise = Noise(c.seed);
    (
        core::array::from_fn(|i| {
            clean[i]
                .iter()
                .map(|y| y + sigma * noise.normal())
                .collect()
        }),
        sigma,
    )
}

fn outcome(
    t: &Templates,
    s: Sensor,
    traces: &[Trace; 2],
    clean: &[Vec<f64>; 2],
    measured: &[Vec<f64>; 2],
) -> Result<Value, Box<dyn Error>> {
    let fit = fit_state(t, s, traces, measured)?;
    let Some(selected) = fit["selected_start_index"].as_u64() else {
        return Ok(fit);
    };
    let attempts = fit["attempts"]
        .as_array()
        .ok_or("missing nonlinear attempts")?;
    let state: Vec<f64> = serde_json::from_value(
        attempts[selected as usize]["optimization"]["initial_state"].clone(),
    )?;
    let clean_score = score(t, s, &state, traces, clean);
    let mut windows = Vec::new();
    for i in 0..2 {
        let start = clean[i].len() / 2;
        let measured_energy = dot(&measured[i][start..], &measured[i][start..]);
        let noise_energy: f64 = clean[i][start..]
            .iter()
            .zip(&measured[i][start..])
            .map(|(a, b)| (a - b).powi(2))
            .sum();
        let noise_relative = (noise_energy / measured_energy).sqrt();
        let threshold = (1.25 * noise_relative).max(1e-6);
        let observed = fit["validation"]["windows"][i]["held_out_voltage_relative_rmse"]
            .as_f64()
            .ok_or("missing voltage score")?;
        windows.push(json!({"clean_voltage_relative_rmse":clean_score["windows"][i]["held_out_voltage_relative_rmse"],
            "state_energy_norm_relative_rmse":clean_score["windows"][i]["held_out_state_energy_norm_relative_rmse"],
            "measured_voltage_relative_rmse":observed,"injected_noise_relative_rmse":noise_relative,
            "oracle_prediction_threshold":threshold,"oracle_prediction_consistent":observed < threshold}));
    }
    let prediction = windows
        .iter()
        .all(|w| w["oracle_prediction_consistent"] == true);
    let state_within_one_percent = windows.iter().all(|w| {
        w["state_energy_norm_relative_rmse"]
            .as_f64()
            .is_some_and(|e| e < 0.01)
    });
    // Compact all start outcomes instead of duplicating complete LM histories.
    let summaries: Vec<_> = attempts
        .iter()
        .map(|a| {
            json!({"start_factor":a["start_factor"],"error":a["error"],
        "status":a["optimization"]["status"],"iterations":a["optimization"]["iterations"],
        "training_relative_rmse":a["optimization"]["training_relative_rmse"],
        "held_out":a["validation"]})
        })
        .collect();
    Ok(
        json!({"selected_start_index":selected,"attempts":summaries,"windows":windows,
        "oracle_prediction_consistent":prediction,"state_within_one_percent":state_within_one_percent,
        "prediction_consistent_but_state_biased":prediction && !state_within_one_percent,
        "strict_matched_recovery":clean_score["known_state_recovery"]}),
    )
}

fn study() -> Result<Value, Box<dyn Error>> {
    let p = prepare(perturbations()[0])?;
    let sensors: Vec<_> = sensors()?
        .into_iter()
        .filter(|s| s.offset_m != 0.0)
        .collect();
    let mut cases = Vec::new();
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let t = templates(&p.spectrum, &p.structural, &p.damper, alpha, beta, rate)?;
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
                                traces[i].truth.push(reference_state(&p.spectrum, before));
                                positions[i].push(before.pickup_displacement_m);
                            }
                        }
                    }
                })?;
            if !take.diagnostics["last_contact_seconds"]
                .as_f64()
                .is_some_and(|v| v < 0.02)
                || traces
                    .iter()
                    .any(|tr| !tr.contact_free || tr.pickup.len() != t.windows[0].velocity.len())
            {
                return Err("invalid robustness mechanical history".into());
            }
            let mut rows = Vec::new();
            for sensor in &sensors {
                let clean = core::array::from_fn(|i| {
                    positions[i]
                        .iter()
                        .zip(&traces[i].pickup)
                        .map(|(x, v)| sensor.voltage(*x, *v))
                        .collect()
                });
                for condition in conditions() {
                    let assumed = match assumed_sensor(*sensor, condition) {
                        Ok(s) => s,
                        Err(e) => {
                            rows.push(json!({"true_sensor":sensor,"assumed_sensor":null,"condition":condition,
                                "requested_gap_m":sensor.gap_m * condition.gap_scale,
                                "requested_offset_m":sensor.offset_m * condition.offset_scale,
                                "noise_standard_deviation":0.0,"required_control":condition.name == "matched",
                                "fit":{"status":"withheld_invalid_sensor","error":e.to_string()}}));
                            continue;
                        }
                    };
                    let (measured, sigma) = corrupt(&clean, condition);
                    let fit = match outcome(&t, assumed, &traces, &clean, &measured) {
                        Ok(v) => v,
                        Err(e) => json!({"error":e.to_string()}),
                    };
                    rows.push(json!({"true_sensor":sensor,"assumed_sensor":assumed,"condition":condition,"noise_standard_deviation":sigma,
                        "required_control":condition.name == "matched","fit":fit}));
                }
            }
            let passed = take.diagnostics["mechanical_checks_passed"] == true
                && rows
                    .iter()
                    .filter(|r| r["required_control"] == true)
                    .all(|r| r["fit"]["strict_matched_recovery"] == true);
            cases.push(json!({"sample_rate":rate,"supplied_structural_scale":alpha,"supplied_damper_scale":beta,"controls_passed":passed,"diagnostics":take.diagnostics,"observations":rows}));
            println!("Magnetic state robustness: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-state-robustness-v1","controls_passed":cases.iter().all(|c|c["controls_passed"] == true),"cases":cases,
        "protocol":"Frozen before first run. Six known-loss trajectories, two laws at baseline and close geometry, 12 conditions each: noiseless matched; additive Gaussian noise at 60/40/20 dB with seeds 17 and 71; assumed gap +/-5%; assumed offset +/-5%; swapped field law. No combined perturbations. Noise sigma uses only RMS of first clean training window and stays constant across both windows. SplitMix64/Box-Muller; paired conditions reuse standardized noise. Same nonlinear fitter, three fixed starts, continuous event state, training-only selection and held-out windows as nonlinear-magnetic-state-v1. No budgets or gates changed. All 24 matched controls require original strict state/voltage recovery; all perturbed cases descriptive. State recovery within 1% requires both held-out state energy-norm relative errors <0.01. Oracle voltage consistency requires each measured held-out relative RMSE < max(1e-6, 1.25 * actual injected held-out noise relative RMS). This synthetic diagnostic uses known injected noise, is never supplied to optimization/selection, and is not a deployable acceptance rule. All start outcomes and failures retained compactly.",
        "scope":"Known mechanical losses and timing. Assumed radial gap, transverse offset and field law vary; longitudinal geometry stays fixed. Two noise seeds do not establish confidence intervals. Centered sign ambiguity remains in previous study. No real recording, unknown-loss recovery, antialiasing, production DSP, audio asset or host change."}),
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
        return Err("magnetic state robustness retained failed controls".into());
    }
    println!("Magnetic state robustness: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaussian_noise_is_repeatable_and_has_expected_moments() {
        let mut a = Noise(17);
        let mut b = Noise(17);
        let mut sum = 0.0;
        let mut squares = 0.0;
        for _ in 0..100000 {
            let x = a.normal();
            assert_eq!(x, b.normal());
            sum += x;
            squares += x * x;
        }
        assert!((sum / 100000.0).abs() < 0.015);
        assert!((squares / 100000.0 - 1.0).abs() < 0.02);
    }
    #[test]
    fn held_out_voltage_cannot_change_noise_scale_or_training_samples() {
        let clean = [vec![2.0; 100], vec![1.0; 100]];
        let condition = conditions()[1];
        let (first, sigma) = corrupt(&clean, condition);
        let mut changed = clean.clone();
        for window in &mut changed {
            window[50..].fill(1e6);
        }
        let (second, other_sigma) = corrupt(&changed, condition);
        assert_eq!(sigma, 0.002);
        assert_eq!(sigma, other_sigma);
        for i in 0..2 {
            assert_eq!(first[i][..50], second[i][..50]);
        }
        let noiseless = corrupt(&clean, conditions()[0]);
        assert_eq!(noiseless.0, clean);
        assert_eq!(noiseless.1, 0.0);
    }
}
