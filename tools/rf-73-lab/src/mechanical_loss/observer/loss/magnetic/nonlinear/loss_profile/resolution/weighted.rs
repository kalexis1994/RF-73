//! Noise-consistent local resolution with explicit optimizer qualification.
use super::*;

pub const HELP: &str = "Weighted magnetic loss resolution:
  magnetic-weighted-loss-resolution --input WEIGHTING_RECEIPT.json --output REPORT.json
Constant-voltage state refits; local radii withheld for incomplete optimizer evidence.
";
const BLOB: &str = "72ef1d0838548765e3955eaf673da4ed82ad0494";
const SHA256: &str = "c0629403f9187efc05f69c22a90ae6b777ba0e400fb69ddc5d46cdc97c4085ca";

fn completed(status: Option<&str>) -> bool {
    matches!(status, Some("residual_converged" | "no_descent_step"))
}

pub(super) fn qualify(diagnosis: &mut Value, row: &SourceRow, evaluations: &[Value], sigma: f64) {
    let source_complete = row.fit.attempts.len() == 2
        && row.fit.selected_loss_start_index < row.fit.attempts.len()
        && row
            .fit
            .attempts
            .iter()
            .all(|a| completed(a.status.as_deref()));
    let local_complete = evaluations.len() == 11
        && evaluations.iter().all(|e| {
            let Some(starts) = e["state_starts"].as_array() else {
                return false;
            };
            starts.len() == 3
                && e["selected_state_start_index"]
                    .as_u64()
                    .is_some_and(|i| i < starts.len() as u64)
                && starts
                    .iter()
                    .all(|s| completed(s["status"].as_str()) && s["error"].is_null())
                && e["error"].is_null()
        });
    let non_improving = diagnosis["alternatives"].as_array().is_some_and(|a| {
        a.len() == 10
            && a.iter()
                .all(|v| v["objective_change"].as_f64().is_some_and(|d| d >= 0.0))
    });
    let status = if sigma == 0.0 {
        "withheld_no_noise_scale"
    } else if diagnosis["center_replayed"] != true {
        "withheld_failed_center_replay"
    } else if !source_complete || !local_complete {
        "withheld_incomplete_optimizer"
    } else if !non_improving {
        "withheld_improving_alternative"
    } else if diagnosis["linearized_log_radius_for_one_noise_unit"]
        .as_f64()
        .is_none_or(|r| !r.is_finite() || r <= 0.0)
    {
        "withheld_unresolved_sensitivity"
    } else {
        "descriptive_local_radius"
    };
    if status != "descriptive_local_radius" {
        diagnosis["linearized_log_radius_for_one_noise_unit"] = Value::Null;
    }
    diagnosis["local_radius_status"] = json!(status);
    diagnosis["optimizer_qualification"] = json!({
        "source_outer_starts":row.fit.attempts,
        "selected_source_outer_start_index":row.fit.selected_loss_start_index,
        "source_all_starts_complete":source_complete,
        "local_all_starts_complete":local_complete,
        "all_alternatives_non_improving":non_improving,
        "global_optimum_certified":false,
        "interpretation":"All starts, including unselected ones, must avoid errors, unknown statuses and budget limits for a descriptive radius. No-descent counts as a completed numerical attempt, not proof of stationarity or an optimum. Source outer statuses and new local inner statuses are checked; historical inner candidates are not requalified. Signal distances and singular values remain descriptive even when the radius is withheld. No confidence or coverage claim."
    });
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 5 || args[1] != "--input" || args[3] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[4]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let input: Source = serde_json::from_slice(&pinned_bytes_for(Path::new(&args[2]), BLOB)?)?;
    if input.schema_version != 1
        || input.experiment != "nonlinear-magnetic-loss-weighting-v1"
        || input.weighting.as_deref() != Some("constant_voltage")
        || input.cases.len() != 6
    {
        return Err("unsupported weighted loss receipt schema".into());
    }
    let mut report = study(&input, Weighting::ConstantVoltage)?;
    report["experiment"] = json!("nonlinear-magnetic-weighted-loss-resolution-v1");
    report["source_git_blob_sha1"] = json!(BLOB);
    report["source_sha256"] = json!(SHA256);
    report["weighting"] = json!(Weighting::ConstantVoltage);
    report["protocol"] = json!(
        "Frozen before first run. Pin nonlinear-magnetic-loss-weighting-v1 bytes; read only schema/weighting, sample rate, noise condition, estimated scales, training RMSE and outer-start completion statuses/index. Regenerate the same six mechanical histories and five noise conditions each. No outer search, reference loss errors, state truth, prior fitted state or held-out scores enter the diagnostic. Refit all 18 continuous coordinates from measured training halves at each center and ten alternatives, with the original three starts and budgets. Every residual/Jacobian/seed uses constant-voltage weighting 1/norm(concatenated training voltage). Require center RMSE replay within absolute 1e-8. Axial log offsets +/-ln(1.01) form centered profiled derivatives; axial +/-ln(1.05) and weakest raw-voltage direction +/-ln(1.05) explore alternatives. Multiply normalized residuals by the common training norm to recover voltage; report pooled and raw singular values and noise-scaled values using injected sigma. D=norm(alternative prediction-center prediction)/sigma is a total vector distance, not waveform RMS. Smallest singular value uses reorthogonalized QR and det(R)/largest. Local radius sigma/minimum raw singular value is descriptive and withheld for sigma=0, failed replay, any source outer or local inner error/unknown stop/budget limit (even unselected), an improving alternative, or unresolved sensitivity. Every candidate, start status and raw signal distance is retained. No retuning or superiority/coverage gate."
    );
    report["scope"] = json!(
        "Training-only sensitivity aligned with the constant-voltage objective. Oracle sigma, exact sensor geometry/law/gain and synthetic mechanics remain supplied. A completed no-descent attempt does not establish stationarity, global uniqueness or calibrated uncertainty. No confidence intervals or statistical coverage inferred from local radii or alternative distances. Historical inner-search paths are not requalified here. No production DSP, preset, recording, audio asset, host launch or listening test."
    );
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("weighted loss resolution retained failed replay controls".into());
    }
    println!("Weighted loss resolution: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (SourceRow, Value, Vec<Value>) {
        let row = SourceRow {
            snr_db: Some(20.0),
            seed: 17,
            fit: SourceFit {
                estimated_scales: [1.0, 1.0],
                training_relative_rmse: 0.1,
                selected_loss_start_index: 1,
                attempts: vec![
                    SourceAttempt {
                        status: Some("no_descent_step".into()),
                    },
                    SourceAttempt {
                        status: Some("residual_converged".into()),
                    },
                ],
            },
        };
        let diagnosis = json!({"center_replayed":true,"linearized_log_radius_for_one_noise_unit":0.02,
            "raw_voltage_profile_singular_values":[2.0,1.0],"alternatives":vec![json!({"objective_change":0.01});10]});
        let evaluations = vec![
            json!({"selected_state_start_index":1,"state_starts":vec![json!({"status":"no_descent_step"});3]});
            11
        ];
        (row, diagnosis, evaluations)
    }
    #[test]
    fn weighted_radius_withholds_unselected_budget_failures_and_keeps_raw_sensitivity() {
        for bad in [
            Some("iteration_limit"),
            Some("unresolved_profile_derivative"),
            None,
        ] {
            let (mut row, mut d, e) = fixture();
            row.fit.attempts[0].status = bad.map(str::to_owned);
            qualify(&mut d, &row, &e, 0.1);
            assert_eq!(d["local_radius_status"], "withheld_incomplete_optimizer");
            assert!(d["linearized_log_radius_for_one_noise_unit"].is_null());
            assert_eq!(d["raw_voltage_profile_singular_values"], json!([2.0, 1.0]));
        }
        let (row, mut d, mut e) = fixture();
        e[10]["state_starts"][0]["status"] = json!("iteration_limit");
        qualify(&mut d, &row, &e, 0.1);
        assert_eq!(d["local_radius_status"], "withheld_incomplete_optimizer");
    }
    #[test]
    fn weighted_radius_distinguishes_zero_noise_bad_replay_and_improving_alternatives() {
        let (row, mut d, e) = fixture();
        qualify(&mut d, &row, &e, 0.1);
        assert_eq!(d["local_radius_status"], "descriptive_local_radius");
        assert_eq!(d["linearized_log_radius_for_one_noise_unit"], 0.02);
        assert_eq!(
            d["optimizer_qualification"]["global_optimum_certified"],
            false
        );
        let (row, mut d, e) = fixture();
        qualify(&mut d, &row, &e, 0.0);
        assert_eq!(d["local_radius_status"], "withheld_no_noise_scale");
        let (row, mut d, e) = fixture();
        d["center_replayed"] = json!(false);
        qualify(&mut d, &row, &e, 0.1);
        assert_eq!(d["local_radius_status"], "withheld_failed_center_replay");
        let (row, mut d, e) = fixture();
        d["alternatives"][0]["objective_change"] = json!(-0.001);
        qualify(&mut d, &row, &e, 0.1);
        assert_eq!(d["local_radius_status"], "withheld_improving_alternative");
    }
}
