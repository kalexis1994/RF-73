//! Paired loss recovery with equal sample weights for constant voltage noise.
use super::*;

pub const HELP: &str = "Magnetic loss weighting comparison:
  magnetic-loss-weighting --input NOISE_RECEIPT.json --output REPORT.json
Pinned relative-window baseline; new constant-voltage loss and state fits.
";

fn difference(a: &Value, b: &Value) -> Option<f64> {
    Some(a.as_f64()? - b.as_f64()?)
}

fn compare(current: &Value, baseline: &Value) -> Value {
    if !current["fit"]["validation"].is_object()
        || !baseline["fit"]["validation"].is_object()
        || !current["known_loss_state_control"]["windows"].is_array()
        || !baseline["known_loss_state_control"]["windows"].is_array()
    {
        return json!({"status":"withheld_incomplete_fit"});
    }
    let a = &current["fit"]["validation"];
    let b = &baseline["fit"]["validation"];
    let loss: Vec<_> = (0..2)
        .map(|i| difference(&a["relative_loss_errors"][i], &b["relative_loss_errors"][i]))
        .collect();
    let windows: Vec<_> = (0..2).map(|i| {
        let aw = &a["windows"][i];
        let bw = &b["windows"][i];
        json!({
            "clean_voltage_relative_rmse_change":difference(&aw["clean_voltage_relative_rmse"], &bw["clean_voltage_relative_rmse"]),
            "measured_voltage_relative_rmse_change":difference(&aw["measured_voltage_relative_rmse"], &bw["measured_voltage_relative_rmse"]),
            "state_energy_norm_relative_rmse_change":difference(&aw["state_energy_norm_relative_rmse"], &bw["state_energy_norm_relative_rmse"]),
            "known_loss_state_energy_norm_relative_rmse_change":difference(
                &current["known_loss_state_control"]["windows"][i]["state_energy_norm_relative_rmse"],
                &baseline["known_loss_state_control"]["windows"][i]["state_energy_norm_relative_rmse"])
        })
    }).collect();
    json!({"status":"compared","relative_loss_error_change":loss,"windows":windows,
        "new_loss_recovery":a["both_losses_within_one_percent"]==true && b["both_losses_within_one_percent"]==false,
        "lost_loss_recovery":a["both_losses_within_one_percent"]==false && b["both_losses_within_one_percent"]==true,
        "new_state_recovery":a["state_within_one_percent"]==true && b["state_within_one_percent"]==false,
        "lost_state_recovery":a["state_within_one_percent"]==false && b["state_within_one_percent"]==true})
}

fn attach_baseline(report: &mut Value, baseline: &Value) -> Result<(), Box<dyn Error>> {
    let cases = report["cases"]
        .as_array_mut()
        .ok_or("missing weighted cases")?;
    let prior = baseline["cases"]
        .as_array()
        .ok_or("missing baseline cases")?;
    if cases.len() != 6 || prior.len() != cases.len() {
        return Err("weighting case count mismatch".into());
    }
    for (case, old) in cases.iter_mut().zip(prior) {
        if case["sample_rate"] != old["sample_rate"]
            || case["reference_scales_for_scoring_only"] != old["reference_scales_for_scoring_only"]
        {
            return Err("weighting case identity mismatch".into());
        }
        let rows = case["observations"]
            .as_array_mut()
            .ok_or("missing weighted rows")?;
        let old_rows = old["observations"]
            .as_array()
            .ok_or("missing baseline rows")?;
        if rows.len() != 5 || rows.len() != old_rows.len() {
            return Err("weighting observation count mismatch".into());
        }
        for (row, old_row) in rows.iter_mut().zip(old_rows) {
            for key in [
                "snr_db",
                "seed",
                "noise_standard_deviation",
                "required_control",
            ] {
                if row[key] != old_row[key] {
                    return Err(format!("weighting observation identity mismatch: {key}").into());
                }
            }
            row["paired_weighting_comparison"] = compare(row, old_row);
            row["relative_window_baseline"] = json!({
                "estimated_scales":old_row["fit"]["estimated_scales"],
                "training_relative_rmse":old_row["fit"]["training_relative_rmse"],
                "validation":old_row["fit"]["validation"],
                "loss_start_agreement":old_row["fit"]["loss_start_agreement"],
                "known_loss_state_control":old_row["known_loss_state_control"]
            });
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 5 || args[1] != "--input" || args[3] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[4]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    // Pin bytes before expensive work; previous fits/scores never enter the inverse.
    let bytes = resolution::pinned_bytes(Path::new(&args[2]))?;
    let mut report = noise::study_weighted(Weighting::ConstantVoltage)?;
    let baseline: Value = serde_json::from_slice(&bytes)?;
    attach_baseline(&mut report, &baseline)?;
    report["experiment"] = json!("nonlinear-magnetic-loss-weighting-v1");
    report["source_git_blob_sha1"] = json!(resolution::SOURCE_BLOB);
    report["source_sha256"] = json!(resolution::SOURCE_SHA256);
    report["weighting"] = json!(Weighting::ConstantVoltage);
    report["protocol"] = json!(
        "Frozen before first run. Regenerate the same six mechanical fixtures, windows, rates and five noise conditions from the pinned nonlinear-magnetic-loss-noise-v1 receipt (30 paired observations). Shared simulation and noise generator. Replace relative-window residual weights 1/(sqrt(2)*norm(y_window)) by the single training-only scalar 1/norm(concatenated training voltage). Equal sample weights are proportional to inverse noise sigma for constant voltage noise; sigma is not supplied to the inverse, and noiseless controls remain defined. Apply the same weighting to seed, residual and analytic Jacobian in every continuous-state refit, to the outer loss residual, and to the known-loss state comparator. Unchanged starts, bounds, finite differences, iteration/trial budgets and normalized residual stop thresholds. Fit objective is pooled voltage relative RMSE, not a whitened chi-square or comparable to the prior window-relative objective. Select only by each fit's own training objective. Deserialize previous results only after all new fits finish. Verify pair rate, reference fixture, SNR, seed and sigma. Preserve separate clean/measured held-out voltage, full state, loss errors, outer start agreement, failures and candidate histories. All comparison deltas are new minus baseline: negative error change is improvement. Six noiseless controls must pass unchanged strict recovery gates; noisy comparisons are descriptive with no retuning or required superiority."
    );
    report["scope"] = json!(
        "Synthetic constant additive voltage noise with known mechanics except two losses and exact sensor geometry/law/gain. Two seeds do not establish statistical coverage. Equal sample weights maximize the Gaussian likelihood up to a constant in this controlled noise model, but local optimizer outcomes do not certify a global optimum. Common training normalization affects absolute stopping scale; budgets and numerical thresholds are retained. No confidence intervals, source calibration, production DSP, presets, audio assets, host launch or listening test."
    );
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("magnetic weighting study retained failed controls".into());
    }
    println!("Magnetic loss weighting: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighting_comparison_withholds_failed_fits() {
        assert_eq!(
            compare(&json!({}), &json!({}))["status"],
            "withheld_incomplete_fit"
        );
        assert!((difference(&json!(0.01), &json!(0.03)).unwrap() + 0.02).abs() < 1e-15);
        assert_eq!(difference(&Value::Null, &json!(0.03)), None);
    }

    #[test]
    fn weighting_pairing_rejects_changed_identity_or_missing_rows() {
        let row = json!({"snr_db":20.0,"seed":17,"noise_standard_deviation":0.1,"required_control":false});
        let case = json!({"sample_rate":48000,"reference_scales_for_scoring_only":[0.63,1.37],"observations":vec![row;5]});
        let baseline = json!({"cases":vec![case;6]});
        let mut same = baseline.clone();
        attach_baseline(&mut same, &baseline).unwrap();
        assert_eq!(
            same["cases"][0]["observations"][0]["paired_weighting_comparison"]["status"],
            "withheld_incomplete_fit"
        );
        for key in [
            "snr_db",
            "seed",
            "noise_standard_deviation",
            "required_control",
        ] {
            let mut changed = baseline.clone();
            changed["cases"][0]["observations"][0][key] = Value::Null;
            assert!(attach_baseline(&mut changed, &baseline).is_err());
        }
        let mut changed = baseline.clone();
        changed["cases"][0]["sample_rate"] = json!(96000);
        assert!(attach_baseline(&mut changed, &baseline).is_err());
        let mut changed = baseline.clone();
        changed["cases"][0]["observations"] = json!([]);
        assert!(attach_baseline(&mut changed, &baseline).is_err());
    }
}
