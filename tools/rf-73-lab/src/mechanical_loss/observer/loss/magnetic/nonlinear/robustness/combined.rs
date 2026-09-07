//! Paired noise/sensor-error experiment with fixed mechanics and inference gates.
use super::*;

pub const HELP: &str = "Combined magnetic state study:
  magnetic-state-combined --output REPORT.json
Paired noise-only, sensor-only and combined errors; known losses and timing.
";

fn combined_conditions() -> Vec<Condition> {
    let original = conditions();
    let sensors: Vec<_> = original
        .iter()
        .filter(|c| c.snr_db.is_none())
        .copied()
        .collect();
    let noise: Vec<_> = original
        .iter()
        .filter(|c| c.snr_db.is_some_and(|v| v <= 40.0))
        .copied()
        .collect();
    let mut rows = sensors.clone();
    for n in noise {
        for s in &sensors {
            rows.push(Condition {
                name: if s.name == "matched" {
                    "additive_noise"
                } else {
                    s.name
                },
                snr_db: n.snr_db,
                seed: n.seed,
                ..*s
            });
        }
    }
    rows
}

fn has_mismatch(c: &Value) -> bool {
    c["gap_scale"] != 1.0 || c["offset_scale"] != 1.0 || c["swap_law"] == true
}
fn same_geometry(a: &Value, b: &Value) -> bool {
    ["gap_scale", "offset_scale", "swap_law"]
        .iter()
        .all(|k| a[*k] == b[*k])
}
fn state_error(row: &Value) -> Option<f64> {
    row["fit"]["windows"]
        .as_array()?
        .iter()
        .map(|w| w["state_energy_norm_relative_rmse"].as_f64())
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .reduce(f64::max)
}
fn paired_rows(rows: &[Value]) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut pairs = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let c = &row["condition"];
        if c["snr_db"].is_null() || !has_mismatch(c) {
            continue;
        }
        let locate = |noise_only: bool| -> Result<usize, Box<dyn Error>> {
            let matches: Vec<_> = rows
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    if candidate["true_sensor"] != row["true_sensor"] {
                        return false;
                    }
                    let other = &candidate["condition"];
                    if noise_only {
                        !has_mismatch(other)
                            && other["snr_db"] == c["snr_db"]
                            && other["seed"] == c["seed"]
                    } else {
                        other["snr_db"].is_null() && same_geometry(other, c)
                    }
                })
                .map(|(j, _)| j)
                .collect();
            if matches.len() != 1 {
                return Err("combined condition needs exactly one paired control".into());
            }
            Ok(matches[0])
        };
        let noise_index = locate(true)?;
        let sensor_index = locate(false)?;
        let noise = &rows[noise_index];
        let sensor = &rows[sensor_index];
        if row["noise_standard_deviation"] != noise["noise_standard_deviation"] {
            return Err("paired noise amplitude differs".into());
        }
        let complete = [row, noise, sensor]
            .iter()
            .all(|r| r["fit"]["selected_start_index"].is_number());
        let summary = if complete {
            let accepted = row["fit"]["oracle_prediction_consistent"] == true;
            let wrong = row["fit"]["state_within_one_percent"] == false;
            json!({"status":"compared",
                "noise_only_state_max_error":state_error(noise),"sensor_only_state_max_error":state_error(sensor),
                "combined_state_max_error":state_error(row),
                "noise_only_prediction_consistent":noise["fit"]["oracle_prediction_consistent"],
                "noise_only_state_within_one_percent":noise["fit"]["state_within_one_percent"],
                "sensor_only_prediction_consistent":sensor["fit"]["oracle_prediction_consistent"],
                "combined_prediction_consistent":accepted,"combined_state_within_one_percent":!wrong,
                "mismatch_masked_by_noise":sensor["fit"]["oracle_prediction_consistent"] == false && accepted,
                "prediction_consistent_state_error":accepted && wrong,
                "new_hidden_state_error_vs_noise_only":accepted && wrong && noise["fit"]["oracle_prediction_consistent"] == true && noise["fit"]["state_within_one_percent"] == true})
        } else {
            json!({"status":"withheld_incomplete_pair","scope":"Inspect retained input/fit errors; no success or negative result imputed."})
        };
        pairs.push(json!({"combined_row_index":i,"noise_only_row_index":noise_index,"sensor_only_row_index":sensor_index,"comparison":summary}));
    }
    Ok(pairs)
}

fn study() -> Result<Value, Box<dyn Error>> {
    let mut cases = study_cases(&combined_conditions(), "Combined magnetic state")?;
    for case in &mut cases {
        let rows = case["observations"]
            .as_array()
            .ok_or("missing combined observations")?;
        case["paired_comparisons"] = json!(paired_rows(rows)?);
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-state-combined-v1",
        "controls_passed":cases.iter().all(|c|c["controls_passed"] == true),"cases":cases,
        "protocol":"Frozen before first run. Six known-loss trajectories at 48/96 kHz, two laws and baseline/close sensors. Six assumed sensor conditions (matched, gap +/-5%, offset +/-5%, swapped field law), crossed with no noise and 40/20 dB Gaussian noise at seeds 17/71: 30 rows per sensor, 720 total. Same sigma from first clean training window, same continuous state, optimizer, training-only selection and held-out diagnostic gates as nonlinear-magnetic-state-robustness-v1. Noise-only and each combined condition use identical true signal and identical noise samples; sensor-only controls use the same true signal without noise. All 24 noiseless matched controls require original strict recovery. Each of 480 combined rows references exactly one noise-only and one sensor-only row in the same trajectory/sensor case. Out-of-range assumptions and failed fits are retained, with incomplete comparisons withheld. All completed comparisons report both state and voltage outcomes; descriptive masking means a sensor-only prediction rejection becomes a combined prediction acceptance. New hidden state error additionally requires a prediction-consistent noise-only state within 1%, but combined state above 1%. Gates and budgets unchanged; these oracle diagnostics require actual injected noise and synthetic state truth, and never select fits.",
        "scope":"Initial-state inference with exact mechanical losses and timing; not joint parameter estimation. Two paired noise realizations are not confidence intervals or a universal SNR cutoff. Radial gap and transverse offset only; longitudinal sensor placement fixed. Field-law swap also changes gain. Centered sign symmetry, real field calibration and antialiasing remain unresolved; no production equation, preset, audio asset or host change."}),
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
        return Err("combined magnetic state study retained failed controls".into());
    }
    println!("Combined magnetic state: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_sensor_conditions_preserve_exact_paired_noise() {
        let clean = [vec![1.0; 80], vec![0.1; 80]];
        let conditions = combined_conditions();
        assert_eq!(conditions.len(), 30);
        for c in &conditions {
            if c.snr_db.is_none() {
                continue;
            }
            let control = conditions
                .iter()
                .find(|n| n.name == "additive_noise" && n.snr_db == c.snr_db && n.seed == c.seed)
                .unwrap();
            assert_eq!(corrupt(&clean, *c), corrupt(&clean, *control));
        }
    }
    #[test]
    fn combined_pairing_rejects_missing_or_duplicate_controls() {
        let make = |name: &str, snr: Option<f64>, gap: f64| json!({"true_sensor":"s","condition":{"name":name,"snr_db":snr,"seed":17,"gap_scale":gap,"offset_scale":1.0,"swap_law":false},"noise_standard_deviation":0.01,"fit":{"error":"fixture failure"}});
        let combined = make("assumed_gap", Some(40.0), 1.05);
        let noise = make("additive_noise", Some(40.0), 1.0);
        let sensor = make("assumed_gap", None, 1.05);
        assert!(paired_rows(&[combined.clone(), noise.clone()]).is_err());
        assert!(
            paired_rows(&[
                combined.clone(),
                noise.clone(),
                sensor.clone(),
                noise.clone()
            ])
            .is_err()
        );
        let result = paired_rows(&[combined, noise, sensor]).unwrap();
        assert_eq!(
            result[0]["comparison"]["status"],
            "withheld_incomplete_pair"
        );
    }
}
