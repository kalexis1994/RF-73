use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn electromechanical_receipt_qualifies_reciprocity_load_controls_and_output_convergence() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/electromechanical-validation.json");
    let p: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(p["passed"], true);
    assert_eq!(p["steps_per_frame"], serde_json::json!([64, 128, 256]));
    assert_eq!(p["cases"].as_array().unwrap().len(), 8);
    for case in p["cases"].as_array().unwrap() {
        assert_eq!(case["passed"], true);
        let zero = case["case"] == "zero_flux";
        let open = case["case"] == "open_capacitive";
        assert_eq!(case["takes"].as_array().unwrap().len(), 3);
        if !zero {
            assert!(
                case["feedback_velocity_rms_difference_m_s"]
                    .as_f64()
                    .unwrap()
                    > 1e-12
            );
        }
        for take in case["takes"].as_array().unwrap() {
            assert_eq!(take["passed"], true);
            assert_eq!(take["heat_monotone"], true);
            for (field, limit) in [
                ("max_relative_total_balance_defect", 1e-8),
                ("max_relative_exchange_defect", 1e-10),
                ("max_relative_circuit_balance_defect", 1e-8),
                ("max_stationary_drive_relative_energy_growth", 1e-10),
            ] {
                assert!(take[field].as_f64().unwrap() < limit);
            }
            assert!(take["mechanical_contact_entries"][0].as_u64().unwrap() >= 2);
            assert!(take["maximum_coupling_iterations"].as_u64().unwrap() <= 16);
            if zero {
                for field in [
                    "peak_filtered_voltage_v",
                    "peak_reaction_force_n",
                    "coil_heat_j",
                ] {
                    assert_eq!(take[field], 0.0);
                }
            } else {
                assert!(take["peak_filtered_voltage_v"].as_f64().unwrap() > 1e-4);
                assert!(take["peak_reaction_force_n"].as_f64().unwrap() > 1e-8);
                assert!(take["coil_heat_j"].as_f64().unwrap() > 0.0);
                assert!(take["mechanical_pickup_work_j"].as_f64().unwrap() < 0.0);
            }
            if zero || open {
                assert_eq!(take["load_heat_j"], 0.0);
            } else {
                assert!(take["load_heat_j"].as_f64().unwrap() > 0.0);
            }
        }
        for comparison in ["coarse_vs_fine", "medium_vs_fine"] {
            let windows = case[comparison]["windows"].as_array().unwrap();
            assert_eq!(windows.len(), 4);
            for window in windows {
                for error in window["relative_rmse_voltage_current_vertical_horizontal"]
                    .as_array()
                    .unwrap()
                {
                    assert!(error.as_f64().unwrap() < 0.01);
                }
            }
        }
    }
}

#[test]
fn electromechanical_cli_rejects_bad_options_and_preserves_both_render_outputs() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("keep.json"), b"receipt").unwrap();
    fs::write(scratch.0.join("audio.wav"), b"audio").unwrap();
    for args in [
        vec!["electromechanical", "--output", "keep.json"],
        vec!["electromechanical-render", "--output", "keep.wav"],
        vec!["electromechanical-render", "--output", "audio.wav"],
        vec!["electromechanical"],
        vec!["electromechanical", "--output", "bad.wav"],
        vec!["electromechanical", "--output", "bad.json", "--unknown"],
        vec!["electromechanical", "--bad", "bad.json"],
        vec!["electromechanical-render"],
        vec!["electromechanical-render", "--output", "bad.json"],
        vec![
            "electromechanical-render",
            "--output",
            "bad.wav",
            "--refined",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
    }
    assert_eq!(fs::read(scratch.0.join("keep.json")).unwrap(), b"receipt");
    assert_eq!(fs::read(scratch.0.join("audio.wav")).unwrap(), b"audio");
    for gain in ["NaN", "inf", "0", "-0.1", "2", "invalid"] {
        assert!(
            !scratch
                .run(&[
                    "electromechanical-render",
                    "--output",
                    "bad.wav",
                    "--gain",
                    gain
                ])
                .status
                .success()
        );
    }
    assert!(
        !scratch
            .run(&["electromechanical-render", "--output", "bad.wav", "--gain"])
            .status
            .success()
    );
    for absent in ["keep.wav", "audio.json", "bad.wav", "bad.json"] {
        assert!(!scratch.0.join(absent).exists());
    }
}

#[test]
fn polarized_action_receipt_closes_each_plane_and_preserves_symmetry_controls() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/polarized-action-validation.json");
    let p: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(p["passed"], true);
    assert_eq!(p["steps_per_frame"], serde_json::json!([64, 128, 256]));
    assert_eq!(p["cases"].as_array().unwrap().len(), 8);
    for case in p["cases"].as_array().unwrap() {
        assert_eq!(case["passed"], true);
        for take in case["takes"].as_array().unwrap() {
            assert_eq!(take["passed"], true);
            assert!(take["max_relative_balance_defect"].as_f64().unwrap() < 1e-8);
            for defect in take["max_relative_plane_work_defect"].as_array().unwrap() {
                assert!(defect.as_f64().unwrap() < 1e-8);
            }
            if case["case"] == "isotropic" || case["case"] == "aligned_anisotropy" {
                assert_eq!(take["peak_displacement_xy_m"][1], 0.0);
                assert_eq!(take["plane_coupling_work_j"][1], 0.0);
            } else {
                assert!(take["peak_displacement_xy_m"][1].as_f64().unwrap() > 1e-9);
                assert!(take["orbit_covariance_rank"].as_f64().unwrap() > 1e-3);
            }
            if case["case"] == "rotated_boundary" {
                assert_eq!(take["plane_contact_work_j"][1], 0.0);
                let coupling = take["plane_coupling_work_j"][1].as_f64().unwrap();
                let stored = take["final_plane_diagonal_energy_j"][1].as_f64().unwrap();
                let heat = take["plane_diagonal_heat_j"][1].as_f64().unwrap();
                assert!(coupling > 0.0);
                assert!((coupling - stored - heat).abs() < coupling * 1e-7);
            }
        }
        for comparison in ["coarse_vs_fine", "medium_vs_fine"] {
            for window in case[comparison]["windows"].as_array().unwrap() {
                for error in window["velocity_xy_relative_rmse"].as_array().unwrap() {
                    assert!(error.as_f64().unwrap() < 0.01);
                }
                for error in window["hammer_arm_rmse_m"].as_array().unwrap() {
                    assert!(error.as_f64().unwrap() < 1e-5);
                }
            }
        }
    }
}

#[test]
fn polarized_action_cli_rejects_invalid_options_and_existing_reports() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("keep.json"), b"preserve").unwrap();
    let out = scratch.run(&["polarized-action", "--output", "keep.json"]);
    assert!(!out.status.success());
    assert_eq!(fs::read(scratch.0.join("keep.json")).unwrap(), b"preserve");
    for args in [
        vec!["polarized-action"],
        vec!["polarized-action", "--output", "bad.wav"],
        vec!["polarized-action", "--output", "bad.json", "--unknown"],
        vec!["polarized-action", "--bad", "bad.json"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn action_cycle_cli_rejects_invalid_options_and_preserves_existing_reports() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("keep.json"), b"preserve").unwrap();
    let out = scratch.run(&[
        "action-cycle",
        "--output",
        "keep.json",
        "--fast-drive",
        "--reference",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("new .json file"));
    assert_eq!(fs::read(scratch.0.join("keep.json")).unwrap(), b"preserve");
    for args in [
        vec!["action-cycle"],
        vec!["action-cycle", "--output", "bad.wav"],
        vec!["action-cycle", "--output", "bad.json", "--unknown"],
        vec![
            "action-cycle",
            "--output",
            "bad.json",
            "--reference",
            "--refined",
        ],
        vec![
            "action-cycle",
            "--output",
            "bad.json",
            "--fast-drive",
            "--fast-drive",
        ],
        vec!["action-cycle", "--bad", "bad.json"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn action_cycle_receipts_preserve_failed_studies_and_reproduce_overlapping_resolution() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../references");
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&fs::read(root.join(name)).unwrap()).unwrap()
    };
    let slow = read("persistent-action-cycle-validation.json");
    let fast = read("persistent-action-cycle-fast-refined-validation.json");
    let reference = read("persistent-action-cycle-reference-validation.json");
    assert_eq!(slow["passed"], false);
    assert_eq!(fast["passed"], false);
    assert_eq!(reference["passed"], true);
    assert_eq!(
        reference["steps_per_frame"],
        serde_json::json!([512, 1024, 2048])
    );
    assert_eq!(reference["drive_speed_m_s"], 1.5);
    assert_eq!(reference["profile"], fast["profile"]);
    assert_eq!(reference["cases"].as_array().unwrap().len(), 16);
    for (a, b) in fast["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["cases"].as_array().unwrap())
    {
        for field in ["length_m", "sample_rate", "gesture"] {
            assert_eq!(a[field], b[field]);
        }
        assert_eq!(a["takes"][2], b["takes"][0]);
        assert_eq!(b["passed"], true);
        for take in b["takes"].as_array().unwrap() {
            assert_eq!(take["passed"], true);
            assert_eq!(take["behavior_passed"], true);
            assert_eq!(take["heat_monotone"], true);
            assert!(take["max_relative_balance_defect"].as_f64().unwrap() < 1e-8);
            assert!(take["max_relative_contact_work_defect"].as_f64().unwrap() < 1e-9);
            assert!(take["maximum_solver_sweeps"].as_u64().unwrap() <= 64);
            if b["gesture"] == "slack_bridle" {
                assert!(take["simultaneous_hammer_felt_steps"].as_u64().unwrap() > 0);
            }
        }
        for comparison in ["coarse_vs_fine", "medium_vs_fine"] {
            for window in b[comparison]["windows"].as_array().unwrap() {
                assert!(window["pickup_velocity_relative_rmse"].as_f64().unwrap() < 0.01);
                assert!(window["hammer_position_rmse_m"].as_f64().unwrap() < 1e-5);
                assert!(window["arm_position_rmse_m"].as_f64().unwrap() < 1e-5);
            }
        }
    }
}

#[test]
fn felt_damper_preserves_failed_coarse_evidence_and_qualifies_refinement() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../references");
    let coarse: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("moving-felt-damper-validation.json")).unwrap())
            .unwrap();
    let fine: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("moving-felt-damper-refined-validation.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(coarse["passed"], false);
    assert_eq!(fine["passed"], true);
    assert_eq!(fine["steps_per_frame"], serde_json::json!([32, 64, 128]));
    assert!(
        fine["protocol"]
            .as_str()
            .unwrap()
            .contains("original 16/32/64 matrix failed")
    );
    assert_eq!(fine["cases"].as_array().unwrap().len(), 24);
    let mut failed = 0;
    for (a, b) in coarse["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fine["cases"].as_array().unwrap())
    {
        for key in ["length_m", "sample_rate", "gesture"] {
            assert_eq!(a[key], b[key]);
        }
        if a["passed"] == false {
            failed += 1;
        }
        assert_eq!(b["passed"], true);
        // Independent runs at overlapping resolutions must preserve the complete summary.
        assert_eq!(a["takes"][1], b["takes"][0]);
        assert_eq!(a["takes"][2], b["takes"][1]);
        for take in b["takes"].as_array().unwrap() {
            assert_eq!(take["passed"], true);
            assert_eq!(take["heat_monotone"], true);
            assert!(take["minimum_force_n"].as_f64().unwrap() >= 0.0);
            assert!(take["max_relative_balance_defect"].as_f64().unwrap() < 1e-8);
            if b["gesture"] == "held" {
                assert_eq!(take["contact_entries"], 0);
                assert_eq!(take["felt_heat_j"], 0.0);
            } else {
                assert!(take["contact_entries"].as_u64().unwrap() > 0);
                assert!(take["felt_heat_j"].as_f64().unwrap() > 0.0);
            }
        }
        for key in ["coarse_vs_fine", "medium_vs_fine"] {
            assert_eq!(b[key]["passed"], true);
            assert_eq!(b[key]["windows"].as_array().unwrap().len(), 3);
        }
    }
    assert_eq!(failed, 1);
}

#[test]
fn felt_damper_cli_rejects_invalid_options_and_existing_outputs() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("keep.json"), b"preserve").unwrap();
    let out = scratch.run(&["felt-damper", "--output", "keep.json", "--refined"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("new .json file"));
    assert_eq!(fs::read(scratch.0.join("keep.json")).unwrap(), b"preserve");
    for args in [
        vec!["felt-damper"],
        vec!["felt-damper", "--output", "bad.wav"],
        vec!["felt-damper", "--output", "bad.json", "--unknown"],
        vec!["felt-damper", "--bad", "bad.json"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn magnetic_weighted_loss_resolution_replays_and_qualifies_every_start() {
    let scratch = Scratch::new();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/nonlinear-magnetic-loss-weighting-validation.json");
    let bytes = fs::read(source).unwrap();
    fs::write(scratch.0.join("source.json"), &bytes).unwrap();
    let args = [
        "magnetic-weighted-loss-resolution",
        "--input",
        "source.json",
        "--output",
        "resolution.json",
    ];
    scratch.success(&args);
    let report = scratch.json("resolution.json");
    let source: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        report["experiment"],
        "nonlinear-magnetic-weighted-loss-resolution-v1"
    );
    assert_eq!(report["weighting"], "constant_voltage");
    assert_eq!(
        report["source_git_blob_sha1"],
        "72ef1d0838548765e3955eaf673da4ed82ad0494"
    );
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["cases"].as_array().unwrap().len(), 6);
    let mut zero_noise = 0;
    let mut source_limited = 0;
    for (ci, case) in report["cases"].as_array().unwrap().iter().enumerate() {
        assert_eq!(case["sample_rate"], source["cases"][ci]["sample_rate"]);
        assert_eq!(case["observations"].as_array().unwrap().len(), 5);
        for (ri, row) in case["observations"].as_array().unwrap().iter().enumerate() {
            let d = &row["diagnosis"];
            let old = &source["cases"][ci]["observations"][ri];
            assert_eq!(d["center_replayed"], true);
            assert!(
                d["source_training_rmse_absolute_difference"]
                    .as_f64()
                    .unwrap()
                    < 1e-8
            );
            assert!(d["relative_window_profile_singular_values"].is_null());
            let norm = d["training_voltage_l2_norm"].as_f64().unwrap();
            for i in 0..2 {
                let normalized = d["pooled_relative_profile_singular_values"][i]
                    .as_f64()
                    .unwrap();
                let raw = d["raw_voltage_profile_singular_values"][i]
                    .as_f64()
                    .unwrap();
                assert!((normalized * norm / raw - 1.0).abs() < 1e-10);
                let a = d["center_scales"][i].as_f64().unwrap();
                let b = old["fit"]["estimated_scales"][i].as_f64().unwrap();
                assert!((a / b - 1.0).abs() < 1e-14);
            }
            let qualification = &d["optimizer_qualification"];
            assert_eq!(qualification["global_optimum_certified"], false);
            assert_eq!(
                qualification["selected_source_outer_start_index"],
                old["fit"]["selected_loss_start_index"]
            );
            let incomplete = old["fit"]["attempts"].as_array().unwrap().iter().any(|a| {
                !matches!(
                    a["status"].as_str(),
                    Some("residual_converged" | "no_descent_step")
                )
            });
            assert_eq!(qualification["source_all_starts_complete"], !incomplete);
            let sigma = row["noise_standard_deviation"].as_f64().unwrap();
            if sigma == 0.0 {
                zero_noise += 1;
                assert_eq!(d["local_radius_status"], "withheld_no_noise_scale");
                assert!(d["oracle_noise_scaled_singular_values"].is_null());
                assert!(d["linearized_log_radius_for_one_noise_unit"].is_null());
            } else if incomplete {
                source_limited += 1;
                assert_eq!(d["local_radius_status"], "withheld_incomplete_optimizer");
                assert!(d["linearized_log_radius_for_one_noise_unit"].is_null());
            } else if d["local_radius_status"] == "descriptive_local_radius" {
                assert_eq!(qualification["local_all_starts_complete"], true);
                assert_eq!(qualification["all_alternatives_non_improving"], true);
                let radius = d["linearized_log_radius_for_one_noise_unit"]
                    .as_f64()
                    .unwrap();
                let small = d["raw_voltage_profile_singular_values"][1]
                    .as_f64()
                    .unwrap();
                assert!((radius * small / sigma - 1.0).abs() < 1e-12);
            } else {
                assert!(d["linearized_log_radius_for_one_noise_unit"].is_null());
            }
            let evaluations = row["profile_evaluations"].as_array().unwrap();
            assert_eq!(evaluations.len(), 11);
            for e in evaluations {
                let starts = e["state_starts"].as_array().unwrap();
                assert_eq!(starts.len(), 3);
                let selected = e["selected_state_start_index"].as_u64().unwrap() as usize;
                let best = starts[selected]["training_relative_rmse"].as_f64().unwrap();
                assert!(
                    starts
                        .iter()
                        .all(|s| best <= s["training_relative_rmse"].as_f64().unwrap())
                );
            }
            let alternatives = d["alternatives"].as_array().unwrap();
            assert_eq!(alternatives.len(), 10);
            for a in alternatives {
                let i = a["evaluation_index"].as_u64().unwrap() as usize;
                assert_eq!(a["scales"], evaluations[i]["scales"]);
                assert_eq!(a["training_objective"], evaluations[i]["objective"]);
                if sigma > 0.0 {
                    let distance = a["prediction_change_voltage_l2"].as_f64().unwrap() / sigma;
                    assert!(
                        (a["oracle_noise_scaled_prediction_distance"]
                            .as_f64()
                            .unwrap()
                            / distance
                            - 1.0)
                            .abs()
                            < 1e-12
                    );
                    assert_eq!(a["within_one_noise_unit"], distance < 1.0);
                } else {
                    assert!(a["oracle_noise_scaled_prediction_distance"].is_null());
                    assert!(a["within_one_noise_unit"].is_null());
                }
            }
        }
    }
    assert_eq!(zero_noise, 6);
    assert_eq!(source_limited, 1);
    let before = fs::read(scratch.0.join("resolution.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(before, fs::read(scratch.0.join("resolution.json")).unwrap());
    let mut modified = bytes.clone();
    modified.push(b' ');
    fs::write(scratch.0.join("modified.json"), modified).unwrap();
    let out = scratch.run(&[
        "magnetic-weighted-loss-resolution",
        "--input",
        "modified.json",
        "--output",
        "bad.json",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("pinned evidence"));
    assert!(!scratch.0.join("bad.json").exists());
    assert_eq!(fs::read(scratch.0.join("source.json")).unwrap(), bytes);
    for args in [
        vec!["magnetic-weighted-loss-resolution"],
        vec![
            "magnetic-weighted-loss-resolution",
            "--input",
            "source.json",
            "--output",
            "bad.wav",
        ],
        vec![
            "magnetic-weighted-loss-resolution",
            "--bad",
            "source.json",
            "--output",
            "bad.json",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn magnetic_loss_weighting_preflights_output_and_pinned_input() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("keep.json"), b"preserve").unwrap();
    let result = scratch.run(&[
        "magnetic-loss-weighting",
        "--input",
        "missing.json",
        "--output",
        "keep.json",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("new .json file"));
    assert_eq!(fs::read(scratch.0.join("keep.json")).unwrap(), b"preserve");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/nonlinear-magnetic-loss-noise-validation.json");
    let mut bytes = fs::read(source).unwrap();
    bytes.push(b' ');
    fs::write(scratch.0.join("modified.json"), &bytes).unwrap();
    let result = scratch.run(&[
        "magnetic-loss-weighting",
        "--input",
        "modified.json",
        "--output",
        "bad.json",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("pinned evidence"));
    assert!(!scratch.0.join("bad.json").exists());
    assert_eq!(fs::read(scratch.0.join("modified.json")).unwrap(), bytes);
    for args in [
        vec!["magnetic-loss-weighting"],
        vec![
            "magnetic-loss-weighting",
            "--input",
            "modified.json",
            "--output",
            "bad.wav",
        ],
        vec![
            "magnetic-loss-weighting",
            "--unknown",
            "modified.json",
            "--output",
            "bad.json",
        ],
        vec![
            "magnetic-loss-weighting",
            "--input",
            "modified.json",
            "--output",
            "bad.json",
            "extra",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
    let help = scratch.run(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("magnetic-loss-weighting --input"));
}

#[test]
fn magnetic_loss_weighting_receipt_preserves_pairs_and_training_selection() {
    fn same_evidence(a: &serde_json::Value, b: &serde_json::Value) {
        use serde_json::Value;
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                // Baseline scores undergo an extra JSON parse/serialize cycle.
                // Retain a relative machine-precision allowance, not an error gate.
                let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
                assert!((a - b).abs() <= 8.0 * f64::EPSILON * a.abs().max(b.abs()));
            }
            (Value::Array(a), Value::Array(b)) => {
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b) {
                    same_evidence(a, b);
                }
            }
            (Value::Object(a), Value::Object(b)) => {
                assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
                for (key, a) in a {
                    same_evidence(a, &b[key]);
                }
            }
            _ => assert_eq!(a, b),
        }
    }
    // The expensive full command is run once to produce the tracked receipt.
    // Recheck its evidence without repeating the complete outer-search matrix.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../references");
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("nonlinear-magnetic-loss-weighting-validation.json")).unwrap(),
    )
    .unwrap();
    let old: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("nonlinear-magnetic-loss-noise-validation.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["experiment"], "nonlinear-magnetic-loss-weighting-v1");
    assert_eq!(report["weighting"], "constant_voltage");
    assert_eq!(
        report["source_git_blob_sha1"],
        "5e32ac1255599fd8aae280eb6656d14921c0f174"
    );
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["cases"].as_array().unwrap().len(), 6);
    let mut controls = 0;
    for (case, prior) in report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(old["cases"].as_array().unwrap())
    {
        assert_eq!(case["sample_rate"], prior["sample_rate"]);
        assert_eq!(
            case["reference_scales_for_scoring_only"],
            prior["reference_scales_for_scoring_only"]
        );
        assert_eq!(case["observations"].as_array().unwrap().len(), 5);
        for (row, previous) in case["observations"]
            .as_array()
            .unwrap()
            .iter()
            .zip(prior["observations"].as_array().unwrap())
        {
            for key in ["snr_db", "seed", "noise_standard_deviation"] {
                assert_eq!(row[key], previous[key]);
            }
            assert_eq!(row["weighting"], "constant_voltage");
            same_evidence(
                &row["relative_window_baseline"]["validation"],
                &previous["fit"]["validation"],
            );
            same_evidence(
                &row["relative_window_baseline"]["known_loss_state_control"],
                &previous["known_loss_state_control"],
            );
            assert_eq!(row["paired_weighting_comparison"]["status"], "compared");
            if row["required_control"] == true {
                controls += 1;
                assert_eq!(row["required_control_passed"], true);
            } else {
                assert!(row["required_control_passed"].is_null());
            }
            let fit = &row["fit"];
            let attempts = fit["attempts"].as_array().unwrap();
            assert_eq!(attempts.len(), 2);
            let selected = fit["selected_loss_start_index"].as_u64().unwrap() as usize;
            let objective = attempts[selected]["objective"].as_f64().unwrap();
            assert!(
                attempts
                    .iter()
                    .all(|a| objective <= a["objective"].as_f64().unwrap())
            );
            let evaluations = row["profile_evaluations"].as_array().unwrap();
            for a in attempts {
                let index = a["evaluation_index"].as_u64().unwrap() as usize;
                assert_eq!(a["scales"], evaluations[index]["scales"]);
                assert_eq!(a["objective"], evaluations[index]["objective"]);
            }
            for e in evaluations {
                let starts = e["state_starts"].as_array().unwrap();
                assert_eq!(starts.len(), 3);
                let selected = e["selected_state_start_index"].as_u64().unwrap() as usize;
                let best = starts[selected]["training_relative_rmse"].as_f64().unwrap();
                assert!(
                    starts
                        .iter()
                        .all(|s| best <= s["training_relative_rmse"].as_f64().unwrap())
                );
            }
            for i in 0..2 {
                let expected = fit["validation"]["relative_loss_errors"][i]
                    .as_f64()
                    .unwrap()
                    - previous["fit"]["validation"]["relative_loss_errors"][i]
                        .as_f64()
                        .unwrap();
                let actual = row["paired_weighting_comparison"]["relative_loss_error_change"][i]
                    .as_f64()
                    .unwrap();
                assert!((actual - expected).abs() < 1e-14);
                for field in [
                    "clean_voltage_relative_rmse",
                    "measured_voltage_relative_rmse",
                    "state_energy_norm_relative_rmse",
                ] {
                    let expected = fit["validation"]["windows"][i][field].as_f64().unwrap()
                        - previous["fit"]["validation"]["windows"][i][field]
                            .as_f64()
                            .unwrap();
                    let actual = row["paired_weighting_comparison"]["windows"][i]
                        [format!("{field}_change")]
                    .as_f64()
                    .unwrap();
                    assert!((actual - expected).abs() < 1e-14);
                }
            }
        }
    }
    assert_eq!(controls, 6);
}

#[test]
fn magnetic_loss_resolution_pins_evidence_replays_centers_and_withholds_zero_noise() {
    let scratch = Scratch::new();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/nonlinear-magnetic-loss-noise-validation.json");
    let bytes = fs::read(source).unwrap();
    fs::write(scratch.0.join("source.json"), &bytes).unwrap();
    let args = [
        "magnetic-loss-resolution",
        "--input",
        "source.json",
        "--output",
        "resolution.json",
    ];
    scratch.success(&args);
    let report = scratch.json("resolution.json");
    assert_eq!(
        report["experiment"],
        "nonlinear-magnetic-loss-resolution-v1"
    );
    assert_eq!(report["controls_passed"], true);
    assert_eq!(
        report["source_git_blob_sha1"],
        "5e32ac1255599fd8aae280eb6656d14921c0f174"
    );
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut noiseless = 0;
    for case in cases {
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 5);
        for row in rows {
            let d = &row["diagnosis"];
            assert_eq!(d["center_replayed"], true);
            let alternatives = d["alternatives"].as_array().unwrap();
            assert_eq!(alternatives.len(), 10);
            let evaluations = row["profile_evaluations"].as_array().unwrap();
            assert_eq!(evaluations.len(), 11);
            for eval in evaluations {
                let starts = eval["state_starts"].as_array().unwrap();
                assert_eq!(starts.len(), 3);
                let best=starts[eval["selected_state_start_index"].as_u64().unwrap() as usize]["training_relative_rmse"].as_f64().unwrap();
                for start in starts {
                    if let Some(other) = start["training_relative_rmse"].as_f64() {
                        assert!(best <= other);
                    } else {
                        assert!(start["error"].is_string());
                    }
                }
            }
            let sigma = row["noise_standard_deviation"].as_f64().unwrap();
            if sigma == 0.0 {
                noiseless += 1;
                assert_eq!(d["noise_resolution_status"], "withheld_no_noise_scale");
                assert!(d["oracle_noise_scaled_singular_values"].is_null());
                assert!(d["linearized_log_radius_for_one_noise_unit"].is_null());
            }
            let singular = d["raw_voltage_profile_singular_values"].as_array().unwrap();
            assert!(singular[0].as_f64().unwrap() >= singular[1].as_f64().unwrap());
            let center = d["center_scales"].as_array().unwrap();
            for alt in alternatives {
                assert!(alt["evaluation_index"].as_u64().unwrap() < 11);
                for j in 0..2 {
                    let expected = center[j].as_f64().unwrap()
                        * alt["log_scale_offset"][j].as_f64().unwrap().exp();
                    assert!((alt["scales"][j].as_f64().unwrap() - expected).abs() < 1e-14);
                }
                if sigma > 0.0 {
                    let distance = alt["prediction_change_voltage_l2"].as_f64().unwrap() / sigma;
                    let stored = alt["oracle_noise_scaled_prediction_distance"]
                        .as_f64()
                        .unwrap();
                    assert!((distance - stored).abs() < 1e-12 * distance.max(1.0));
                    assert_eq!(alt["within_one_noise_unit"], stored < 1.0);
                } else {
                    assert!(alt["within_one_noise_unit"].is_null());
                }
            }
        }
    }
    assert_eq!(noiseless, 6);
    let saved = fs::read(scratch.0.join("resolution.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("resolution.json")).unwrap());
    let mut modified = bytes.clone();
    modified.push(b' ');
    fs::write(scratch.0.join("modified.json"), modified).unwrap();
    assert!(
        !scratch
            .run(&[
                "magnetic-loss-resolution",
                "--input",
                "modified.json",
                "--output",
                "rejected.json"
            ])
            .status
            .success()
    );
    assert!(!scratch.0.join("rejected.json").exists());
    assert_eq!(bytes, fs::read(scratch.0.join("source.json")).unwrap());
}

#[test]
fn noisy_magnetic_losses_keep_truth_separate_and_pair_identical_noise() {
    let scratch = Scratch::new();
    let args = ["magnetic-loss-noise", "--output", "noise.json"];
    scratch.success(&args);
    let report = scratch.json("noise.json");
    assert_eq!(report["experiment"], "nonlinear-magnetic-loss-noise-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut controls = 0;
    let mut noisy = 0;
    for case in cases {
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 5);
        for row in rows {
            if row["required_control"] == true {
                controls += 1;
                assert_eq!(row["required_control_passed"], true);
                assert!(row["snr_db"].is_null());
                assert_eq!(row["noise_standard_deviation"], 0.0);
            } else {
                noisy += 1;
                assert!(row["required_control_passed"].is_null());
                assert!(row["noise_standard_deviation"].as_f64().unwrap() > 0.0);
            }
            let fit = &row["fit"];
            let known = &row["known_loss_state_control"];
            if let Some(selected) = fit["selected_loss_start_index"].as_u64() {
                let attempts = fit["attempts"].as_array().unwrap();
                assert_eq!(attempts.len(), 2);
                let best = attempts[selected as usize]["objective"].as_f64().unwrap();
                for attempt in attempts {
                    if let Some(cost) = attempt["objective"].as_f64() {
                        assert!(best <= cost);
                        assert_eq!(
                            attempt["relative_loss_errors_for_scoring_only"]
                                .as_array()
                                .unwrap()
                                .len(),
                            2
                        );
                    } else {
                        assert!(attempt["error"].is_string());
                    }
                }
                let v = &fit["validation"];
                let errors = v["relative_loss_errors"].as_array().unwrap();
                assert_eq!(
                    v["both_losses_within_one_percent"],
                    errors.iter().all(|e| e.as_f64().unwrap() < 0.01)
                );
                assert_eq!(
                    v["prediction_consistent_loss_error"],
                    v["prediction_consistent"] == true
                        && v["both_losses_within_one_percent"] == false
                );
                assert_eq!(
                    fit["agreement_hides_loss_error"],
                    fit["loss_start_agreement"]["within_one_percent"] == true
                        && v["prediction_consistent_loss_error"] == true
                );
                if row["paired_state_comparison"]["status"] == "compared" {
                    for i in 0..2 {
                        assert_eq!(
                            v["windows"][i]["injected_noise_relative_rmse"],
                            known["windows"][i]["injected_noise_relative_rmse"]
                        );
                    }
                    assert_eq!(
                        row["paired_state_comparison"]["new_hidden_state_error_when_freeing_losses"],
                        known["state_within_one_percent"] == true
                            && v["prediction_consistent"] == true
                            && v["state_within_one_percent"] == false
                    );
                } else {
                    assert!(known["error"].is_string());
                }
            } else {
                assert!(fit["error"].is_string());
            }
            assert!(!row["profile_evaluations"].as_array().unwrap().is_empty());
        }
    }
    assert_eq!((controls, noisy), (6, 24));
    let saved = fs::read(scratch.0.join("noise.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("noise.json")).unwrap());
    for args in [
        vec!["magnetic-loss-noise"],
        vec!["magnetic-loss-noise", "--output", "bad.wav"],
        vec![
            "magnetic-loss-noise",
            "--output",
            "bad.json",
            "--truth",
            "1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
    }
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn nonlinear_loss_profile_recovers_unknown_scales_and_retains_bounded_failure() {
    let scratch = Scratch::new();
    let args = ["magnetic-loss-profile", "--output", "profile.json"];
    scratch.success(&args);
    let report = scratch.json("profile.json");
    assert_eq!(report["experiment"], "nonlinear-magnetic-loss-profile-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    let mut positives = 0;
    let mut negatives = 0;
    for case in cases {
        assert_eq!(case["control_passed"], true);
        let fit = &case["fit"];
        if case["negative_out_of_range_control"] == true {
            negatives += 1;
            assert_eq!(fit["prediction_consistent"], false);
            assert_eq!(fit["both_losses_within_one_percent"], false);
        } else {
            positives += 1;
            assert_eq!(fit["validation"]["known_state_recovery"], true);
            assert_eq!(fit["both_losses_within_one_percent"], true);
            assert_eq!(fit["boundary_limited"], false);
        }
        let evaluations = case["profile_evaluations"].as_array().unwrap();
        for evaluation in evaluations {
            for x in evaluation["scales"].as_array().unwrap() {
                assert!((0.25..=2.0).contains(&x.as_f64().unwrap()));
            }
            if let Some(selected) = evaluation["selected_state_start_index"].as_u64() {
                let starts = evaluation["state_starts"].as_array().unwrap();
                assert_eq!(starts.len(), 3);
                let best = starts[selected as usize]["training_relative_rmse"]
                    .as_f64()
                    .unwrap();
                for start in starts {
                    if let Some(cost) = start["training_relative_rmse"].as_f64() {
                        assert!(best <= cost);
                    } else {
                        assert!(start["error"].is_string());
                    }
                }
            } else {
                assert!(evaluation["error"].is_string());
            }
        }
        let attempts = fit["attempts"].as_array().unwrap();
        assert_eq!(attempts.len(), 2);
        let selected = fit["selected_loss_start_index"].as_u64().unwrap() as usize;
        let best = attempts[selected]["objective"].as_f64().unwrap();
        for attempt in attempts {
            if let Some(cost) = attempt["objective"].as_f64() {
                assert!(best <= cost);
                for step in attempt["history"].as_array().unwrap() {
                    if let Some(index) = step["from_evaluation_index"].as_u64() {
                        let next = step["proposal_evaluation_index"].as_u64().unwrap();
                        assert_eq!(
                            step["accepted"],
                            evaluations[next as usize]["objective"].as_f64().unwrap()
                                < evaluations[index as usize]["objective"].as_f64().unwrap()
                        );
                        for difference in step["derivative_evaluations"].as_array().unwrap() {
                            assert!(difference["log_span"].as_f64().unwrap() > 0.0);
                            assert!(
                                difference["plus_evaluation_index"].as_u64().unwrap()
                                    < evaluations.len() as u64
                            );
                            assert!(
                                difference["minus_evaluation_index"].as_u64().unwrap()
                                    < evaluations.len() as u64
                            );
                        }
                    }
                }
            } else {
                assert!(attempt["error"].is_string());
            }
        }
    }
    assert_eq!((positives, negatives), (6, 1));
    let saved = fs::read(scratch.0.join("profile.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("profile.json")).unwrap());
    for args in [
        vec!["magnetic-loss-profile"],
        vec!["magnetic-loss-profile", "--output", "bad.wav"],
        vec![
            "magnetic-loss-profile",
            "--output",
            "bad.json",
            "--truth",
            "1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
    }
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn combined_magnetic_state_keeps_paired_controls_and_withheld_geometry() {
    let scratch = Scratch::new();
    let args = ["magnetic-state-combined", "--output", "combined.json"];
    scratch.success(&args);
    let report = scratch.json("combined.json");
    assert_eq!(report["experiment"], "nonlinear-magnetic-state-combined-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut controls = 0;
    let mut withheld = 0;
    let mut incomplete = 0;
    for case in cases {
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 120);
        for row in rows {
            if row["required_control"] == true {
                controls += 1;
                assert_eq!(row["fit"]["strict_matched_recovery"], true);
            }
            if row["fit"]["status"] == "withheld_invalid_sensor" {
                withheld += 1;
                assert!(row["requested_gap_m"].as_f64().unwrap() < 0.0005);
            }
            if let Some(selected) = row["fit"]["selected_start_index"].as_u64() {
                let attempts = row["fit"]["attempts"].as_array().unwrap();
                assert_eq!(attempts.len(), 3);
                let cost = attempts[selected as usize]["training_relative_rmse"]
                    .as_f64()
                    .unwrap();
                for attempt in attempts {
                    if let Some(other) = attempt["training_relative_rmse"].as_f64() {
                        assert!(cost <= other);
                    } else {
                        assert!(attempt["error"].is_string());
                    }
                }
            } else {
                assert!(row["fit"]["error"].is_string());
            }
        }
        let pairs = case["paired_comparisons"].as_array().unwrap();
        assert_eq!(pairs.len(), 80);
        for pair in pairs {
            let combined = &rows[pair["combined_row_index"].as_u64().unwrap() as usize];
            let noise = &rows[pair["noise_only_row_index"].as_u64().unwrap() as usize];
            let sensor = &rows[pair["sensor_only_row_index"].as_u64().unwrap() as usize];
            assert_eq!(combined["true_sensor"], noise["true_sensor"]);
            assert_eq!(combined["true_sensor"], sensor["true_sensor"]);
            assert_eq!(combined["condition"]["seed"], noise["condition"]["seed"]);
            assert_eq!(
                combined["condition"]["snr_db"],
                noise["condition"]["snr_db"]
            );
            assert_eq!(
                combined["noise_standard_deviation"],
                noise["noise_standard_deviation"]
            );
            assert!(sensor["condition"]["snr_db"].is_null());
            for field in ["gap_scale", "offset_scale", "swap_law"] {
                assert_eq!(combined["condition"][field], sensor["condition"][field]);
            }
            let comparison = &pair["comparison"];
            if comparison["status"] == "withheld_incomplete_pair" {
                incomplete += 1;
                assert!(
                    [combined, noise, sensor]
                        .iter()
                        .any(|r| r["fit"]["error"].is_string())
                );
            } else {
                assert_eq!(comparison["status"], "compared");
                let accepted = combined["fit"]["oracle_prediction_consistent"] == true;
                let wrong = combined["fit"]["state_within_one_percent"] == false;
                assert_eq!(
                    comparison["mismatch_masked_by_noise"],
                    sensor["fit"]["oracle_prediction_consistent"] == false && accepted
                );
                assert_eq!(
                    comparison["prediction_consistent_state_error"],
                    accepted && wrong
                );
                assert_eq!(
                    comparison["new_hidden_state_error_vs_noise_only"],
                    accepted
                        && wrong
                        && noise["fit"]["oracle_prediction_consistent"] == true
                        && noise["fit"]["state_within_one_percent"] == true
                );
            }
        }
    }
    assert_eq!((controls, withheld, incomplete), (24, 60, 48));
    let saved = fs::read(scratch.0.join("combined.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("combined.json")).unwrap());
    for args in [
        vec!["magnetic-state-combined"],
        vec!["magnetic-state-combined", "--output", "bad.wav"],
        vec![
            "magnetic-state-combined",
            "--output",
            "bad.json",
            "--truth",
            "1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
    }
    assert!(!scratch.0.join("bad.json").exists());
    assert!(!scratch.0.join("bad.wav").exists());
}

#[test]
fn magnetic_state_robustness_retains_noise_mismatch_and_training_selection() {
    let scratch = Scratch::new();
    let args = ["magnetic-state-robustness", "--output", "robustness.json"];
    scratch.success(&args);
    let report = scratch.json("robustness.json");
    assert_eq!(
        report["experiment"],
        "nonlinear-magnetic-state-robustness-v1"
    );
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut controls = 0;
    let mut noisy = 0;
    let mut mismatch = 0;
    let mut withheld = 0;
    for case in cases {
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 48);
        for row in rows {
            if row["required_control"] == true {
                controls += 1;
                assert_eq!(row["fit"]["strict_matched_recovery"], true);
            } else if row["condition"]["snr_db"].is_number() {
                noisy += 1;
                assert!(row["noise_standard_deviation"].as_f64().unwrap() > 0.0);
                assert_eq!(row["true_sensor"], row["assumed_sensor"]);
            } else {
                mismatch += 1;
                assert_eq!(row["noise_standard_deviation"], 0.0);
                assert_ne!(row["true_sensor"], row["assumed_sensor"]);
            }
            let fit = &row["fit"];
            if fit["status"] == "withheld_invalid_sensor" {
                withheld += 1;
                assert_eq!(row["condition"]["name"], "assumed_gap");
                assert_eq!(row["true_sensor"]["geometry"], "close");
                assert!(row["requested_gap_m"].as_f64().unwrap() < 0.0005);
            }
            if let Some(index) = fit["selected_start_index"].as_u64() {
                let attempts = fit["attempts"].as_array().unwrap();
                assert_eq!(attempts.len(), 3);
                let selected = attempts[index as usize]["training_relative_rmse"]
                    .as_f64()
                    .unwrap();
                for attempt in attempts {
                    if let Some(other) = attempt["training_relative_rmse"].as_f64() {
                        assert!(selected <= other);
                    } else {
                        assert!(attempt["error"].is_string());
                    }
                }
                let windows = fit["windows"].as_array().unwrap();
                assert_eq!(windows.len(), 2);
                assert_eq!(
                    fit["state_within_one_percent"],
                    windows
                        .iter()
                        .all(|w| w["state_energy_norm_relative_rmse"].as_f64().unwrap() < 0.01)
                );
                for w in windows {
                    let threshold =
                        (1.25 * w["injected_noise_relative_rmse"].as_f64().unwrap()).max(1e-6);
                    // JSON round trips can shift the recomputed product by an ULP.
                    let stored = w["oracle_prediction_threshold"].as_f64().unwrap();
                    assert!((stored - threshold).abs() <= 8.0 * f64::EPSILON * threshold);
                }
            } else {
                assert!(fit["error"].is_string());
            }
        }
    }
    assert_eq!((controls, noisy, mismatch), (24, 144, 120));
    assert_eq!(withheld, 12);
    let saved = fs::read(scratch.0.join("robustness.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("robustness.json")).unwrap());
    assert!(
        !scratch
            .run(&["magnetic-state-robustness", "--output", "bad.wav"])
            .status
            .success()
    );
    assert!(!scratch.0.join("bad.wav").exists());
}

#[test]
fn nonlinear_magnetic_state_uses_training_selection_and_retains_centered_ambiguity() {
    let scratch = Scratch::new();
    let args = ["magnetic-state", "--output", "state.json"];
    scratch.success(&args);
    let report = scratch.json("state.json");
    assert_eq!(report["experiment"], "nonlinear-magnetic-state-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut centered = 0;
    let mut baseline = 0;
    for case in cases {
        assert_eq!(case["controls_passed"], true);
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        for row in rows {
            if row["required_control"] == true {
                assert_eq!(row["required_control_passed"], true);
            }
            let fit = &row["fit"];
            if row["sensor"]["geometry"] == "centered" {
                centered += 1;
                assert_eq!(fit["status"], "withheld_sign_ambiguity");
                assert!(fit["voltage_energy"].as_f64().unwrap() > 0.0);
                assert!(fit["sign_symmetry_relative_rmse"].as_f64().unwrap() < 1e-12);
                continue;
            }
            if row["sensor"]["geometry"] == "baseline" {
                baseline += 1;
                assert_eq!(fit["validation"]["known_state_recovery"], true);
            }
            let attempts = fit["attempts"].as_array().unwrap();
            assert_eq!(attempts.len(), 3);
            if fit["error"].is_null() {
                let selected = fit["selected_start_index"].as_u64().unwrap() as usize;
                let objective = attempts[selected]["optimization"]["training_relative_rmse"]
                    .as_f64()
                    .unwrap();
                for attempt in attempts {
                    if let Some(other) = attempt["optimization"]["training_relative_rmse"].as_f64()
                    {
                        assert!(objective <= other);
                    }
                }
            }
            for attempt in attempts {
                if let Some(history) = attempt["optimization"]["history"].as_array() {
                    let mut previous = history[0]["objective"].as_f64().unwrap();
                    for step in &history[1..] {
                        if step["accepted"] == true {
                            let next = step["candidate_objective"].as_f64().unwrap();
                            assert!(next < previous);
                            previous = next;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(centered, 12);
    assert_eq!(baseline, 12);
    let saved = fs::read(scratch.0.join("state.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("state.json")).unwrap());
    for args in [
        vec!["magnetic-state"],
        vec!["magnetic-state", "--output", "bad.wav"],
        vec!["magnetic-state", "--output", "bad.json", "--truth", "1"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn magnetic_loss_keeps_linear_controls_nonlinear_failures_and_centered_withholding() {
    let scratch = Scratch::new();
    let args = ["magnetic-pickup-loss", "--output", "magnetic.json"];
    scratch.success(&args);
    let report = scratch.json("magnetic.json");
    assert_eq!(report["experiment"], "magnetic-observation-loss-v1");
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["summary"]["observations"], 66);
    assert_eq!(report["summary"]["positive_controls"], 30);
    assert_eq!(report["summary"]["centered_withheld"], 12);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 11);
        for row in rows {
            if row["required_control"] == true {
                assert_eq!(row["required_control_passed"], true);
            }
            if row["status"] == "withheld_zero_rest_sensitivity" {
                assert_eq!(row["sensor"]["geometry"], "centered");
                assert_eq!(row["rest_sensitivity_per_velocity"], 0.0);
                assert!(row["reason"].as_str().unwrap().contains("rest sensitivity"));
                assert!(row["fit"].is_null());
                for stats in row["nonlinear_forward_diagnostics"].as_array().unwrap() {
                    assert!(stats["voltage_proxy_rms"].as_f64().unwrap() > 0.0);
                    assert_eq!(stats["rest_linearization_relative_rmse"], 1.0);
                }
            } else if row["status"] == "fitted" {
                assert_eq!(row["fit"]["fitted_initial_state_count"], 1);
                for window in row["fit"]["windows"].as_array().unwrap() {
                    assert!(
                        window["held_out_clean_observation_relative_rmse"]
                            .as_f64()
                            .is_some()
                    );
                    assert!(window["held_out_clean_pickup_relative_rmse"].is_null());
                    if row["required_control"] == true {
                        assert!(
                            window["held_out_clean_observation_relative_rmse"]
                                .as_f64()
                                .unwrap()
                                < 1e-5
                        );
                    }
                }
            }
            if row["observation"] != "mechanical_velocity" {
                assert_eq!(row["observation_units"], "uncalibrated_voltage_proxy");
                assert_eq!(
                    row["nonlinear_forward_diagnostics"]
                        .as_array()
                        .unwrap()
                        .len(),
                    2
                );
            }
        }
    }
    let saved = fs::read(scratch.0.join("magnetic.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("magnetic.json")).unwrap());
    for args in [
        vec!["magnetic-pickup-loss"],
        vec!["magnetic-pickup-loss", "--output", "bad.wav"],
        vec![
            "magnetic-pickup-loss",
            "--output",
            "bad.json",
            "--gain",
            "1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn continuous_loss_carries_one_state_and_pairs_every_position_case() {
    let scratch = Scratch::new();
    let args = ["pickup-loss-continuity", "--output", "continuous.json"];
    scratch.success(&args);
    let report = scratch.json("continuous.json");
    assert_eq!(report["experiment"], "continuous-pickup-loss-v1");
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["summary"]["paired_observations"], 102);
    // The unchanged independent path must reproduce the established grid.
    assert_eq!(report["summary"]["independent_prediction_consistent"], 100);
    assert_eq!(report["summary"]["independent_consistent_but_biased"], 46);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut required = 0;
    let mut biased = 0;
    for case in cases {
        assert_eq!(case["propagation_gap_contact_free"], true);
        assert_eq!(case["controls_passed"], true);
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 17);
        for row in rows {
            assert_eq!(row["operator_invariants_passed"], true);
            if row["required_control"] == true {
                required += 1;
                assert_eq!(row["required_control_passed"], true);
            }
            if row["classification"]["prediction_consistent_but_biased"] == true {
                biased += 1;
            }
            let fit = &row["fit"];
            if fit["error"].is_null() {
                assert_eq!(fit["fitted_initial_state_count"], 1);
                assert_eq!(
                    fit["estimated_structural_scale"],
                    row["independent_window_reference"]["estimated_structural_scale"]
                );
                assert_eq!(fit["windows"].as_array().unwrap().len(), 2);
                assert!(fit["windows"][1]["minimum_normalized_qr_pivot"].is_null());
                assert!(
                    fit["windows"][1]["source_off_state_fit_minimum_normalized_qr_pivot"]
                        .as_f64()
                        .unwrap()
                        > 1e-8
                );
                for name in ["structural_profile", "conditional_damper_profile"] {
                    assert_eq!(fit[name]["evaluation_count"], 51);
                    assert_eq!(
                        fit[name]["coarse_evaluations"].as_array().unwrap().len(),
                        17
                    );
                }
            }
        }
    }
    assert_eq!(required, 6);
    assert_eq!(
        report["summary"]["continuous_consistent_but_biased"],
        biased
    );
    let saved = fs::read(scratch.0.join("continuous.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("continuous.json")).unwrap());
    for args in [
        vec!["pickup-loss-continuity"],
        vec!["pickup-loss-continuity", "--output", "bad.wav"],
        vec![
            "pickup-loss-continuity",
            "--output",
            "bad.json",
            "--reset",
            "1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn small_position_errors_retain_every_fit_and_keep_prediction_separate_from_truth() {
    let scratch = Scratch::new();
    let args = ["pickup-loss-geometry", "--output", "geometry.json"];
    scratch.success(&args);
    let report = scratch.json("geometry.json");
    assert_eq!(report["experiment"], "pickup-loss-position-errors-v1");
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["summary"]["observations"], 102);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut biased = 0;
    for case in cases {
        assert_eq!(case["controls_passed"], true);
        let rows = case["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 17);
        for (family, count) in [
            ("matched", 1),
            ("damper_only", 6),
            ("pickup_only", 6),
            ("combined", 4),
        ] {
            assert_eq!(
                rows.iter()
                    .filter(|r| r["perturbation"]["family"] == family)
                    .count(),
                count
            );
        }
        for row in rows {
            assert_eq!(row["operator_invariants_passed"], true);
            if row["required_control"] == true {
                assert_eq!(row["required_control_passed"], true);
            } else {
                assert!(row["required_control_passed"].is_null());
            }
            let c = &row["classification"];
            assert_eq!(
                c["prediction_consistent_but_biased"],
                row["fit"]["prediction_consistent"] == true
                    && c["known_scale_recovery_within_one_percent"] == false
                    && c["known_scale_relative_errors"].is_array()
            );
            if c["prediction_consistent_but_biased"] == true {
                biased += 1;
            }
            if row["fit"]["error"].is_null() {
                for name in ["structural_profile", "conditional_damper_profile"] {
                    let p = &row["fit"][name];
                    assert_eq!(p["evaluation_count"], 51);
                    assert_eq!(p["coarse_evaluations"].as_array().unwrap().len(), 17);
                    assert!(p["evaluations"].is_null());
                }
            }
        }
    }
    assert_eq!(
        report["summary"]["prediction_consistent_but_biased"],
        biased
    );
    let saved = fs::read(scratch.0.join("geometry.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("geometry.json")).unwrap());
    for args in [
        vec!["pickup-loss-geometry"],
        vec!["pickup-loss-geometry", "--output", "bad.wav"],
        vec![
            "pickup-loss-geometry",
            "--output",
            "bad.json",
            "--offset",
            "0.1",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn pickup_loss_recovers_off_grid_scales_without_truth_in_search_and_keeps_controls() {
    let scratch = Scratch::new();
    let args = ["pickup-loss", "--output", "loss.json"];
    scratch.success(&args);
    let report = scratch.json("loss.json");
    assert_eq!(report["experiment"], "profiled-pickup-loss-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        assert_eq!(case["controls_passed"], true);
        let observations = case["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 3);
        for (observation, label) in observations.iter().zip([
            "matched_noiseless",
            "matched_noise_1pct",
            "wrong_damper_position",
        ]) {
            assert_eq!(observation["observation"], label);
            if observation["required_control"] == true {
                assert_eq!(observation["required_control_passed"], true);
                assert_eq!(
                    observation["fit"]["known_scale_recovery_within_one_percent"],
                    true
                );
                for error in observation["fit"]["known_scale_relative_errors"]
                    .as_array()
                    .unwrap()
                {
                    assert!(error.as_f64().unwrap() < 0.001);
                }
            } else {
                assert!(observation["required_control_passed"].is_null());
            }
            let fit = &observation["fit"];
            if fit["error"].is_null() {
                assert_eq!(fit["windows"].as_array().unwrap().len(), 2);
                for name in ["structural_profile", "conditional_damper_profile"] {
                    let evaluations = fit[name]["evaluations"].as_array().unwrap();
                    assert_eq!(evaluations.len(), 51);
                    for e in evaluations {
                        assert!((0.25..=2.0).contains(&e[0].as_f64().unwrap()));
                        assert!(e[1].as_f64().unwrap() >= 0.0);
                    }
                }
                assert!(
                    fit["local_sensitivity"]["minimum_to_maximum_singular_ratio"]
                        .as_f64()
                        .is_some_and(|r| (0.0..=1.0).contains(&r))
                );
                assert!(fit["prediction_consistent"].is_boolean());
            }
        }
    }
    let saved = fs::read(scratch.0.join("loss.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("loss.json")).unwrap());
    for args in [
        vec!["pickup-loss"],
        vec!["pickup-loss", "--output", "bad.wav"],
        vec!["pickup-loss", "--output", "bad.json", "--truth", "1"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn pickup_history_infers_states_with_held_out_prediction_and_retains_reductions() {
    let scratch = Scratch::new();
    let args = ["pickup-state", "--output", "state.json"];
    scratch.success(&args);
    let report = scratch.json("state.json");
    assert_eq!(report["experiment"], "dynamic-pickup-state-v1");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut required = 0;
    for case in cases {
        assert_eq!(case["controls_passed"], true);
        let observations = case["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 18);
        for damped in [false, true] {
            for count in [3, 6, 9] {
                for noise in [0.0, 0.001, 0.01] {
                    assert_eq!(
                        observations
                            .iter()
                            .filter(|o| o["damper_on"] == damped
                                && o["retained_modes"] == count
                                && o["training_noise_relative_rms"] == noise)
                            .count(),
                        1
                    );
                }
            }
        }
        for observation in observations {
            assert_eq!(observation["contact_free"], true);
            assert_eq!(
                observation["training_samples"],
                observation["held_out_samples"]
            );
            if observation["required_control"] == true {
                required += 1;
                assert_eq!(observation["required_control_passed"], true);
                assert!(
                    observation["fit"]["held_out_clean_pickup_relative_rmse"]
                        .as_f64()
                        .unwrap()
                        < 1e-6
                );
                assert!(
                    observation["fit"]["held_out_full_state_energy_norm_relative_rmse"]
                        .as_f64()
                        .unwrap()
                        < 1e-5
                );
            } else {
                assert!(observation["required_control_passed"].is_null());
            }
            if observation["fit"]["error"].is_null() {
                assert_eq!(
                    observation["fit"]["held_out_per_mode_energy_norm_relative_rmse"]
                        .as_array()
                        .unwrap()
                        .len(),
                    9
                );
                assert_eq!(
                    observation["fit"]["inferred_initial_energy_coordinates"]
                        .as_array()
                        .unwrap()
                        .len(),
                    observation["retained_modes"].as_u64().unwrap() as usize * 2
                );
            }
        }
    }
    assert_eq!(required, 12);
    let saved = fs::read(scratch.0.join("state.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("state.json")).unwrap());
    for args in [
        vec!["pickup-state"],
        vec!["pickup-state", "--output", "bad.wav"],
        vec!["pickup-state", "--output", "bad.json", "--modes", "3"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn reduced_mechanical_loss_keeps_shared_trajectory_controls_and_all_observations() {
    let scratch = Scratch::new();
    let args = ["reduced-mechanical-loss", "--output", "reduced.json"];
    scratch.success(&args);
    let report = scratch.json("reduced.json");
    assert_eq!(report["controls_passed"], true);
    assert_eq!(report["modes"].as_array().unwrap().len(), 9);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        assert_eq!(case["full_state_rows_match_original"], true);
        let observations = case["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 6);
        for (index, label) in [
            "full_state",
            "lowest_1_modes",
            "lowest_3_modes",
            "lowest_6_modes",
            "lowest_9_modes",
            "instantaneous_pickup_lift",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(observations[index]["observation"], label);
            assert_eq!(observations[index]["rows"].as_array().unwrap().len(), 6);
            assert!(observations[index]["fit"]["internally_consistent"].is_boolean());
            assert!(observations[index]["fit"]["known_scale_recovery"].is_boolean());
        }
        for index in [0, 4] {
            assert_eq!(observations[index]["fit"]["internally_consistent"], true);
            assert_eq!(observations[index]["fit"]["known_scale_recovery"], true);
        }
    }
    let original = fs::read(scratch.0.join("reduced.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(original, fs::read(scratch.0.join("reduced.json")).unwrap());
    for args in [
        vec!["reduced-mechanical-loss"],
        vec!["reduced-mechanical-loss", "--output", "bad.wav"],
        vec![
            "reduced-mechanical-loss",
            "--output",
            "bad.json",
            "--modes",
            "3",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn mechanical_loss_recovers_known_scales_with_held_out_rows_and_output_protection() {
    let scratch = Scratch::new();
    let args = ["mechanical-loss", "--output", "loss.json"];
    scratch.success(&args);
    let report = scratch.json("loss.json");
    assert_eq!(report["all_cases_qualified"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        assert_eq!(case["qualified"], true);
        assert_eq!(case["rows"].as_array().unwrap().len(), 6);
        assert_eq!(case["fit_row_indices"], serde_json::json!([0, 1, 3]));
        assert_eq!(case["held_out_row_indices"], serde_json::json!([2, 4, 5]));
        for row in case["rows"].as_array().unwrap() {
            assert_eq!(row["contact_free"], true);
        }
        assert!(
            case["omitted_damper_control"]["relative_energy_rmse"]
                .as_f64()
                .unwrap()
                > 0.01
        );
        assert!(case["held_out_relative_energy_rmse"].as_f64().unwrap() < 0.005);
        for name in ["structural", "damper"] {
            let actual = case[format!("estimated_{name}_scale")].as_f64().unwrap();
            let expected = case[format!("known_{name}_scale")].as_f64().unwrap();
            assert!((actual / expected - 1.0).abs() < 0.01);
        }
    }
    let saved = fs::read(scratch.0.join("loss.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("loss.json")).unwrap());
    for args in [
        vec!["mechanical-loss"],
        vec!["mechanical-loss", "--output", "bad.wav"],
        vec!["mechanical-loss", "--output", "bad.json", "--hold", "0.1"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn band_event_grid_retains_all_outcomes_without_identifying_natural_sustain() {
    let scratch = Scratch::new();
    let args = ["study-band-events", "--output", "events.json"];
    scratch.success(&args);
    let report = scratch.json("events.json");
    assert_eq!(report["controls_passed"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 162);
    for rate in [44100, 48000, 96000] {
        for event in ["onset", "release", "loss_increase", "loss_decrease"] {
            let matching: Vec<_> = cases
                .iter()
                .filter(|c| c["sample_rate_hz"] == rate && c["event"] == event)
                .collect();
            assert_eq!(matching.len(), 13);
            assert_eq!(matching.first().unwrap()["requested_event_seconds"], 0.0);
            assert_eq!(matching.last().unwrap()["requested_event_seconds"], 0.204);
        }
    }
    for case in cases {
        assert_eq!(case["measurements"].as_array().unwrap().len(), 2);
        assert_eq!(
            case["natural_sustain_status"],
            "not_identified_by_measurement"
        );
        assert_eq!(
            case["accepted_event_in_measurement_interval"],
            case["paired_qualified"] == true
                && case["ground_truth_region"] == "measurement_interval"
        );
        if case["paired_qualified"] == true {
            assert!(case["paired_rejections"].as_array().unwrap().is_empty());
            for m in case["measurements"].as_array().unwrap() {
                assert_eq!(m["qualified"], true);
            }
        } else {
            assert!(case["conditional_amplitude_decay_per_second"].is_null());
        }
    }
    let saved = fs::read(scratch.0.join("events.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("events.json")).unwrap());
    for args in [
        vec!["study-band-events"],
        vec!["study-band-events", "--output", "bad.wav"],
        vec!["study-band-events", "--output", "bad.json", "--time", "0.1"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn band_envelope_study_retains_onset_false_acceptance_and_requires_both_windows() {
    let scratch = Scratch::new();
    let args = ["validate-band-envelope", "--output", "study.json"];
    let out = scratch.run(&args);
    // The original per-window onset expectation fails: this is retained evidence,
    // not a reason to relax a gate or silently mark the study as fully validated.
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("retained failed expectations"));
    let report = scratch.json("study.json");
    assert_eq!(report["all_expectations_passed"], false);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 66);
    let failures: Vec<_> = cases
        .iter()
        .filter(|c| c["expectation_passed"] != true)
        .collect();
    assert_eq!(failures.len(), 3);
    for failure in failures {
        assert_eq!(failure["probe"], "onset_inside_interval");
        assert_eq!(
            failure["filtered"]["measurement"]["options"]["window_seconds"],
            0.064
        );
        assert_eq!(failure["filtered"]["measurement"]["qualified"], true);
    }
    // Apply the already-declared source pilot agreement criterion descriptively.
    // This does not erase the failed individual-window expectations above.
    for [a, b] in cases.as_chunks::<2>().0 {
        assert_eq!(a["probe"], b["probe"]);
        assert_eq!(a["sample_rate_hz"], b["sample_rate_hz"]);
        let ma = &a["filtered"]["measurement"];
        let mb = &b["filtered"]["measurement"];
        let rates = ma["provisional_fit"]["amplitude_decay_per_second"]
            .as_f64()
            .zip(mb["provisional_fit"]["amplitude_decay_per_second"].as_f64());
        let pair_qualified = ma["qualified"] == true
            && mb["qualified"] == true
            && rates
                .is_some_and(|(x, y)| (x - y).abs() <= 0.5_f64.max(0.15 * x.abs().max(y.abs())));
        assert_eq!(
            pair_qualified,
            a["expected_rejection"].is_null(),
            "{}",
            a["probe"]
        );
    }
    let original = fs::read(scratch.0.join("study.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(original, fs::read(scratch.0.join("study.json")).unwrap());
    for args in [
        vec!["validate-band-envelope"],
        vec![
            "validate-band-envelope",
            "--output",
            "bad.json",
            "--unknown",
        ],
        vec!["validate-band-envelope", "--output", "bad.wav"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn short_envelope_study_and_wav_observations_preserve_outputs_and_sources() {
    let scratch = Scratch::new();
    scratch.success(&["validate-short-envelope", "--output", "study.json"]);
    let report = scratch.json("study.json");
    assert_eq!(report["all_expectations_passed"], true);
    assert_eq!(report["cases"].as_array().unwrap().len(), 18);
    scratch.success(&[
        "render",
        "--output",
        "source.wav",
        "--seconds",
        "0.25",
        "--hold",
        "0.2",
    ]);
    let original = fs::read(scratch.0.join("source.wav")).unwrap();
    let args = [
        "short-envelope",
        "source.wav",
        "--output",
        "observation.json",
        "--frequencies-hz",
        "1620,1568",
        "--start",
        "0.02",
        "--end",
        "0.18",
    ];
    scratch.success(&args);
    let observation = scratch.json("observation.json");
    assert_eq!(
        observation["measurement"]["method"],
        "joint-quadratic-carrier-envelope-v1"
    );
    assert!(
        !observation["measurement"]["points"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let before = fs::read(scratch.0.join("observation.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("observation.json")).unwrap()
    );
    for args in [
        vec![
            "validate-short-envelope",
            "--output",
            "bad.json",
            "--unknown",
            "1",
        ],
        vec![
            "validate-short-envelope",
            "--output",
            "bad.json",
            "--output",
            "other.json",
        ],
        vec!["short-envelope", "source.wav", "--output", "bad.json"],
        vec![
            "short-envelope",
            "source.wav",
            "--output",
            "bad.json",
            "--frequencies-hz",
            "1620,1620",
            "--start",
            "0.02",
            "--end",
            "0.18",
        ],
        vec![
            "short-envelope",
            "source.wav",
            "--output",
            "bad.json",
            "--frequencies-hz",
            "NaN",
            "--start",
            "0.02",
            "--end",
            "0.18",
        ],
        vec!["validate-short-envelope", "--output", "bad.wav"],
    ] {
        assert!(!scratch.run(&args).status.success());
        for name in ["bad.json", "other.json", "bad.wav"] {
            assert!(!scratch.0.join(name).exists());
        }
    }
    assert_eq!(original, fs::read(scratch.0.join("source.wav")).unwrap());
}

#[test]
fn source_envelopes_keep_missing_components_and_verify_receipt_and_audio_bytes() {
    check_pinned_source_envelopes(false);
}

#[test]
fn short_source_envelopes_keep_missing_components_and_verify_receipt_and_audio_bytes() {
    check_pinned_source_envelopes(true);
}

fn check_pinned_source_envelopes(short: bool) {
    let command = if short {
        "observe-short-source-envelopes"
    } else {
        "observe-source-envelopes"
    };
    let scratch = Scratch::new();
    let hash = |name: &str| {
        let out = Command::new("git")
            .current_dir(&scratch.0)
            .args(["hash-object", "--no-filters", "--", name])
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    let mut takes = Vec::new();
    let mut inputs = Vec::new();
    let mut original = Vec::new();
    for (index, velocity) in ["0.3", "0.6", "0.9"].into_iter().enumerate() {
        let file = format!("source-{index}.wav");
        scratch.success(&[
            "render",
            "--output",
            &file,
            "--note",
            "55",
            "--velocity",
            velocity,
            "--seconds",
            if short { "0.25" } else { "2" },
            "--hold",
            if short { "0.2" } else { "1.5" },
        ]);
        original.push(fs::read(scratch.0.join(&file)).unwrap());
        takes.push(serde_json::json!({"id":format!("take-{index}"),"file":file,"git_blob_sha1":hash(&file)}));
        inputs.push(serde_json::json!({"id":format!("take-{index}"),"pitch_anchor":{"qualified":true,"frequency_hz":196.0},
            "observation_windows":[{"label":"attack_128_ms","observed_samples":6144,"capacity_limited":false,
                "accepted_peaks":([196.1,393.0,589.0].map(|f|serde_json::json!({"frequency_hz":f,"ambiguous_neighbor":false,"capacity_limited":false})))}]}));
    }
    let receipt = serde_json::json!({"schema_version":1,"experiment":"cross-note-spectral-hypotheses-v1",
        "manifest":{"groups":[{"note":55,"takes":takes}]},"groups":[{"note":55,"inputs":inputs}]});
    fs::write(
        scratch.0.join("evidence.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let mut manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../references/source-envelope.manifest.json"
    ))
    .unwrap();
    manifest["evidence_file"] = serde_json::json!("evidence.json");
    manifest["evidence_git_blob_sha1"] = serde_json::json!(hash("evidence.json"));
    if short {
        manifest["start_seconds"] = serde_json::json!(0.02);
        manifest["end_seconds"] = serde_json::json!(0.18);
    }
    fs::write(
        scratch.0.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let args = [command, "manifest.json", "--output", "result.json"];
    scratch.success(&args);
    let result = scratch.json("result.json");
    assert_eq!(result["takes"].as_array().unwrap().len(), 3);
    for take in result["takes"].as_array().unwrap() {
        let obs = &take["observation"];
        if short {
            let fundamental = &obs["components"][0];
            assert_eq!(
                fundamental["declared_frequencies_hz"],
                serde_json::json!([196.0, 393.0, 589.0])
            );
            assert_eq!(fundamental["measurements"].as_array().unwrap().len(), 2);
            assert_eq!(
                fundamental["measurements"][0]["options"]["window_seconds"],
                0.032
            );
            assert_eq!(
                fundamental["measurements"][1]["options"]["window_seconds"],
                0.064
            );
        }
        assert_eq!(
            obs["conditional_weak_mixing_relation"]["amplitude_decay_sum_residual_per_second"],
            serde_json::Value::Null
        );
        for c in &obs["components"].as_array().unwrap()[1..] {
            assert_eq!(c["selected_frequency_hz"], serde_json::Value::Null);
            assert_eq!(c["selection_rejections"][0], "missing_prior_peak");
            assert!(c["measurements"].as_array().unwrap().is_empty());
        }
    }
    let saved = fs::read(scratch.0.join("result.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(saved, fs::read(scratch.0.join("result.json")).unwrap());
    for (i, bytes) in original.iter().enumerate() {
        assert_eq!(
            *bytes,
            fs::read(scratch.0.join(format!("source-{i}.wav"))).unwrap()
        );
    }
    fs::write(scratch.0.join("source-0.wav"), b"changed source").unwrap();
    let invalid = [command, "manifest.json", "--output", "bad.json"];
    let out = scratch.run(&invalid);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("blob mismatch"));
    assert!(!scratch.0.join("bad.json").exists());
    fs::write(scratch.0.join("source-0.wav"), &original[0]).unwrap();
    fs::write(scratch.0.join("evidence.json"), b"changed evidence").unwrap();
    let out = scratch.run(&invalid);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("evidence blob mismatch"));
    assert!(!scratch.0.join("bad.json").exists());
    manifest["unknown"] = serde_json::json!(true);
    fs::write(
        scratch.0.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(!scratch.run(&invalid).status.success());
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn component_envelope_roundtrip_validation_and_output_protection() {
    let scratch = Scratch::new();
    scratch.success(&["validate-envelope", "--output", "validation.json"]);
    let report = scratch.json("validation.json");
    assert_eq!(report["all_expectations_passed"], true);
    assert_eq!(report["cases"].as_array().unwrap().len(), 18);
    let path = scratch.0.join("source.wav");
    // Independent PCM16 fixture; the lab writer produces float WAVs.
    let mut pcm = Vec::from(&b"RIFF"[..]);
    pcm.extend_from_slice(&(36_u32 + 96000 * 2).to_le_bytes());
    pcm.extend_from_slice(b"WAVEfmt ");
    pcm.extend_from_slice(&16_u32.to_le_bytes());
    pcm.extend_from_slice(&1_u16.to_le_bytes());
    pcm.extend_from_slice(&1_u16.to_le_bytes());
    pcm.extend_from_slice(&48000_u32.to_le_bytes());
    pcm.extend_from_slice(&96000_u32.to_le_bytes());
    pcm.extend_from_slice(&2_u16.to_le_bytes());
    pcm.extend_from_slice(&16_u16.to_le_bytes());
    pcm.extend_from_slice(b"data");
    pcm.extend_from_slice(&(96000_u32 * 2).to_le_bytes());
    for i in 0..96000 {
        let t = i as f64 / 48000.0;
        let x = 0.2 * (-3.0 * t).exp() * (std::f64::consts::TAU * 1426.7578125 * t + 0.73).cos();
        pcm.extend_from_slice(&((x * 32767.0).round() as i16).to_le_bytes());
    }
    fs::write(&path, pcm).unwrap();
    let source = fs::read(&path).unwrap();
    let args = [
        "component-envelope",
        "source.wav",
        "--output",
        "measurement.json",
        "--frequency-hz",
        "1426.7578125",
        "--start",
        "0.1",
        "--end",
        "1.5",
    ];
    scratch.success(&args);
    let measurement = scratch.json("measurement.json");
    assert_eq!(measurement["measurement"]["qualified"], true);
    assert!(
        (measurement["measurement"]["provisional_fit"]["amplitude_decay_per_second"]
            .as_f64()
            .unwrap()
            - 3.0)
            .abs()
            < 0.01
    );
    let before = fs::read(scratch.0.join("measurement.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("measurement.json")).unwrap()
    );
    for invalid in [
        vec![
            "validate-envelope",
            "--output",
            "bad.json",
            "--unknown",
            "1",
        ],
        vec![
            "validate-envelope",
            "--output",
            "bad.json",
            "--output",
            "other.json",
        ],
        vec!["component-envelope", "source.wav", "--output", "bad.json"],
        vec![
            "component-envelope",
            "source.wav",
            "--output",
            "bad.json",
            "--frequency-hz",
            "NaN",
            "--start",
            "0.1",
            "--end",
            "1.5",
        ],
        vec![
            "component-envelope",
            "source.wav",
            "--output",
            "bad.json",
            "--unknown",
            "1",
        ],
        vec!["validate-envelope", "--output", "bad.wav"],
    ] {
        assert!(!scratch.run(&invalid).status.success());
        for name in ["bad.json", "other.json", "bad.wav"] {
            assert!(!scratch.0.join(name).exists());
        }
    }
    assert_eq!(source, fs::read(&path).unwrap());
}

#[test]
fn register_evidence_preserves_outputs_and_rejects_changed_source_bytes() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("report.json"), b"preserve").unwrap();
    assert!(
        !scratch
            .run(&[
                "observe-register",
                "missing.json",
                "--output",
                "report.json"
            ])
            .status
            .success()
    );
    assert_eq!(
        fs::read(scratch.0.join("report.json")).unwrap(),
        b"preserve"
    );
    fs::write(scratch.0.join("bad.json"), b"{}").unwrap();
    assert!(
        !scratch
            .run(&["observe-register", "bad.json", "--output", "new.json"])
            .status
            .success()
    );
    assert!(!scratch.0.join("new.json").exists());
    let mut m: serde_json::Value = serde_json::from_str(include_str!(
        "../../../references/register-families.manifest.json"
    ))
    .unwrap();
    m["groups"][0]["takes"][0]["file"] = serde_json::json!("altered.wav");
    fs::write(scratch.0.join("altered.wav"), b"changed source bytes").unwrap();
    fs::write(
        scratch.0.join("manifest.json"),
        serde_json::to_vec(&m).unwrap(),
    )
    .unwrap();
    let out = scratch.run(&["observe-register", "manifest.json", "--output", "new.json"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("blob mismatch"));
    assert!(!scratch.0.join("new.json").exists());
}

#[test]
fn spectral_families_preserve_outputs_and_reject_invalid_or_mismatched_inputs() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("output.json"), b"preserve").unwrap();
    assert!(
        !scratch
            .run(&[
                "observe-families",
                "missing.json",
                "--output",
                "output.json"
            ])
            .status
            .success()
    );
    assert_eq!(
        fs::read(scratch.0.join("output.json")).unwrap(),
        b"preserve"
    );
    fs::write(scratch.0.join("manifest.json"), b"{}").unwrap();
    assert!(
        !scratch
            .run(&["observe-families", "manifest.json", "--output", "new.json"])
            .status
            .success()
    );
    assert!(!scratch.0.join("new.json").exists());
    let mut m: serde_json::Value = serde_json::from_str(include_str!(
        "../../../references/g3-spectral-families.manifest.json"
    ))
    .unwrap();
    m["takes"][0]["file"] = serde_json::json!("source.wav");
    fs::write(scratch.0.join("source.wav"), b"incorrect bytes").unwrap();
    fs::write(
        scratch.0.join("manifest.json"),
        serde_json::to_vec(&m).unwrap(),
    )
    .unwrap();
    let out = scratch.run(&["observe-families", "manifest.json", "--output", "new.json"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("blob mismatch"));
    assert!(!scratch.0.join("new.json").exists());
}

#[test]
fn geometry_study_preserves_outputs_and_rejects_unqualified_reference() {
    for command in [
        "sweep-tuned-geometry",
        "sweep-spring-span",
        "sweep-tine-taper",
        "sweep-tine-transition",
    ] {
        let scratch = Scratch::new();
        fs::write(scratch.0.join("study.json"), b"preserve").unwrap();
        let args = [command, "missing.json", "--output", "study.json"];
        let out = scratch.run(&args);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("new .json"));
        assert_eq!(fs::read(scratch.0.join("study.json")).unwrap(), b"preserve");
        fs::write(scratch.0.join("invalid.json"), b"{}").unwrap();
        let out = scratch.run(&[command, "invalid.json", "--output", "new.json"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("qualified G3"));
        assert!(!scratch.0.join("new.json").exists());
    }
}

#[test]
fn modal_observation_verifies_bytes_keeps_native_windows_and_preserves_output() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "source.wav",
        "--note",
        "55",
        "--seconds",
        "1",
        "--hold",
        "0.9",
    ]);
    let hash = Command::new("git")
        .args(["hash-object", "--", "source.wav"])
        .current_dir(&scratch.0)
        .output()
        .unwrap();
    assert!(hash.status.success());
    let hash = String::from_utf8(hash.stdout).unwrap();
    let args = [
        "observe-modes",
        "source.wav",
        "--blob-sha1",
        hash.trim(),
        "--fundamental",
        "196.4",
        "--modes",
        "196.4,1361.9,3686.4",
        "--output",
        "observation.json",
    ];
    scratch.success(&args);
    let report = scratch.json("observation.json");
    assert_eq!(report["git_blob_sha1"], hash.trim());
    assert_eq!(report["observation"]["sample_rate"], 48000);
    assert_eq!(
        report["observation"]["windows"][2]["spectrum"]["observed_samples"],
        24576
    );
    let preserved = fs::read(scratch.0.join("observation.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        fs::read(scratch.0.join("observation.json")).unwrap(),
        preserved
    );
    fs::write(scratch.0.join("source.wav"), b"different bytes").unwrap();
    let mut mismatch = args;
    mismatch[9] = "mismatch.json";
    let result = scratch.run(&mismatch);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("blob mismatch"));
    assert!(!scratch.0.join("mismatch.json").exists());
    mismatch[2] = "--unknown";
    assert!(!scratch.run(&mismatch).status.success());
}
impl Scratch {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // Windows clock resolution can give parallel tests the same timestamp.
        // Reserve each directory atomically; never adopt an existing directory.
        for _ in 0..64 {
            let id = SCRATCH_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("rf-73-cli-{}-{unique}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("failed to reserve test directory: {error}"),
            }
        }
        panic!("could not reserve a unique test directory");
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rf-73-lab"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn success(&self, args: &[&str]) {
        let result = self.run(args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fn json(&self, name: &str) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.0.join(name)).unwrap()).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn pitch_reference_roundtrip_verifies_content_and_preserves_outputs() {
    let scratch = Scratch::new();
    let mut takes = Vec::new();
    for (i, velocity) in ["0.2", "0.5", "0.8"].into_iter().enumerate() {
        let file = format!("take-{i}.wav");
        scratch.success(&[
            "render",
            "--output",
            &file,
            "--note",
            "55",
            "--velocity",
            velocity,
            "--seconds",
            "2",
            "--hold",
            "1.9",
        ]);
        let hash = Command::new("git")
            .args(["hash-object", "--", &file])
            .current_dir(&scratch.0)
            .output()
            .unwrap();
        assert!(hash.status.success());
        takes.push(serde_json::json!({"id":format!("take-{i}"),"file":file,
            "git_blob_sha1":String::from_utf8(hash.stdout).unwrap().trim(),
            "role":if i==2 {"validation"} else {"training"}}));
    }
    let mut manifest = serde_json::json!({"schema_version":1,"note":55,"source":"Synthetic test",
        "source_revision":"generated","instrument":"Production research engine","processing":"None",
        "capture_gain":"Fixed","prior_exposure":"Synthetic regression only","takes":takes});
    let manifest_path = scratch.0.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let args = [
        "prepare-pitch-reference",
        "manifest.json",
        "--output",
        "target.json",
    ];
    scratch.success(&args);
    let report = scratch.json("target.json");
    assert_eq!(report["reference_qualification_passed"], true);
    assert!((report["training_target_hz"].as_f64().unwrap() - 195.9977).abs() < 0.1);
    let saved = fs::read(scratch.0.join("target.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(fs::read(scratch.0.join("target.json")).unwrap(), saved);
    manifest["takes"][0]["git_blob_sha1"] =
        serde_json::json!("0000000000000000000000000000000000000000");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(
        !scratch
            .run(&[
                "prepare-pitch-reference",
                "manifest.json",
                "--output",
                "bad.json"
            ])
            .status
            .success()
    );
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn hammer_comparison_preserves_every_destination_and_rejects_invalid_work() {
    for name in [
        "study.wav",
        "study-elastic.wav",
        "study-rate.wav",
        "study.json",
    ] {
        let scratch = Scratch::new();
        fs::write(scratch.0.join(name), b"preserve existing output").unwrap();
        assert!(
            !scratch
                .run(&["compare-modal-hammers", "--output", "study.wav"])
                .status
                .success()
        );
        assert_eq!(
            fs::read(scratch.0.join(name)).unwrap(),
            b"preserve existing output"
        );
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
    }
    let scratch = Scratch::new();
    for tail in [["--seconds", "NaN"], ["--speed", "2"], ["--note", "57"]] {
        assert!(
            !scratch
                .run(&[
                    "compare-modal-hammers",
                    "--output",
                    "study.wav",
                    tail[0],
                    tail[1]
                ])
                .status
                .success()
        );
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
    }
}

#[test]
fn modal_audio_rejects_invalid_options_and_preserves_both_output_paths() {
    let scratch = Scratch::new();
    for args in [
        vec![
            "render-memory-modal",
            "--output",
            "preview.wav",
            "--seconds",
            "NaN",
        ],
        vec![
            "render-memory-modal",
            "--output",
            "preview.wav",
            "--hold",
            "2",
        ],
        vec![
            "render-memory-modal",
            "--output",
            "preview.wav",
            "--note",
            "57",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
    }
    assert!(!scratch.0.join("preview.wav").exists());
    assert!(!scratch.0.join("preview.json").exists());
    for (name, other) in [("a.wav", "a.json"), ("b.json", "b.wav")] {
        fs::write(scratch.0.join(name), b"preserve existing output").unwrap();
        let wav = if name.ends_with("wav") { name } else { other };
        assert!(
            !scratch
                .run(&["render-memory-modal", "--output", wav])
                .status
                .success()
        );
        assert_eq!(
            fs::read(scratch.0.join(name)).unwrap(),
            b"preserve existing output"
        );
        assert!(!scratch.0.join(other).exists());
    }
}

#[test]
fn listening_wavs_match_global_rms_stay_below_ceiling_and_preserve_outputs() {
    let scratch = Scratch::new();
    let args = ["pickup-listening", "--output", "study"];
    scratch.success(&args);
    let report = scratch.json("study/report.json");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["faults"], 0);
    assert_eq!(
        report["diagnostics"]["isolated_keys"]
            .as_array()
            .unwrap()
            .len(),
        73
    );
    assert_eq!(report["frames"], 24 * 44100);
    let mut observed_rms = Vec::new();
    for name in ["current", "close-original", "close-point-pole"] {
        let path = format!("study/{name}.wav");
        scratch.success(&["inspect", &path]);
        let bytes = fs::read(scratch.0.join(path)).unwrap();
        let mut power = 0.0;
        for chunk in bytes[58..].as_chunks::<4>().0 {
            let value = f32::from_le_bytes(*chunk) as f64;
            assert!(value.is_finite() && value.abs() <= 10.0_f64.powf(-6.0 / 20.0));
            power += value * value;
        }
        observed_rms.push((power / (24.0 * 44100.0)).sqrt());
    }
    for rms in &observed_rms[1..] {
        assert!((rms / observed_rms[0] - 1.0).abs() < 1e-7);
    }
    let before = fs::read(scratch.0.join("study/report.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("study/report.json")).unwrap()
    );
    for flags in [
        vec!["--ceiling-dbfs", "0"],
        vec!["--ceiling-dbfs", "NaN"],
        vec!["--gap-mm", "0"],
        vec!["--offset-mm", "inf"],
        vec!["--sample-rate", "44000"],
        vec!["--unknown", "1"],
        vec!["--measure-only", "--measure-only"],
        vec!["--gap-mm", "1", "--gap-mm", "2"],
        vec!["--gap-mm"],
    ] {
        let mut args = vec!["pickup-listening", "--output", "invalid"];
        args.extend(flags);
        assert!(!scratch.run(&args).status.success(), "{args:?}");
        assert!(!scratch.0.join("invalid").exists());
    }
}

#[test]
fn pickup_convergence_reports_finite_reference_and_frozen_path_without_overwrite() {
    let scratch = Scratch::new();
    let args = [
        "converge-pickup",
        "--output",
        "pickup-convergence.json",
        "--note",
        "55",
        "--seconds",
        "0.05",
        "--gap-mm",
        "0.5",
        "--offset-mm",
        "0.25",
    ];
    scratch.success(&args);
    let report = scratch.json("pickup-convergence.json");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["reference"]["internal_steps"], 128);
    assert_eq!(report["filter_delay_output_samples"], 15.75);
    assert_eq!(report["faults"], 0);
    let rows = report["comparisons"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[4]["internal_steps"], 64);
    assert_eq!(rows[5]["path"], "frozen_128x_trajectory_sampled_at_4x");
    assert!(rows[5]["mechanics"].is_null());
    for row in rows {
        assert_eq!(row["laws"].as_array().unwrap().len(), 2);
        for law in row["laws"].as_array().unwrap() {
            for window in ["full", "attack_32_ms", "after_attack"] {
                assert!(law[window]["raw_nrmse"].as_f64().unwrap().is_finite());
            }
        }
    }
    for (fine, coarse) in rows[4]["laws"]
        .as_array()
        .unwrap()
        .iter()
        .zip(rows[0]["laws"].as_array().unwrap())
    {
        assert!(
            fine["full"]["raw_nrmse"].as_f64().unwrap()
                < coarse["full"]["raw_nrmse"].as_f64().unwrap() * 0.2
        );
    }
    let before = fs::read(scratch.0.join("pickup-convergence.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("pickup-convergence.json")).unwrap()
    );
    scratch.success(&[
        "converge-pickup",
        "--output",
        "fine.json",
        "--reference-steps",
        "256",
        "--note",
        "100",
        "--velocity",
        "0.2",
        "--seconds",
        "0.05",
    ]);
    let fine_report = scratch.json("fine.json");
    assert_eq!(fine_report["reference"]["internal_steps"], 256);
    assert_eq!(fine_report["frozen_reference_stride"], 64);
    let fine_rows = fine_report["comparisons"].as_array().unwrap();
    assert_eq!(fine_rows.len(), 7);
    assert_eq!(fine_rows[5]["internal_steps"], 128);
    for (fine, coarse) in fine_rows[5]["laws"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fine_rows[4]["laws"].as_array().unwrap())
    {
        assert!(
            fine["full"]["raw_nrmse"].as_f64().unwrap()
                < coarse["full"]["raw_nrmse"].as_f64().unwrap() * 0.4
        );
    }
    for flags in [
        vec!["--reference-steps", "64"],
        vec!["--reference-steps", "512"],
        vec!["--seconds", "0.049"],
        vec!["--seconds", "0.251"],
        vec!["--seconds", "NaN"],
        vec!["--velocity", "0.001"],
        vec!["--gap-mm", "0"],
        vec!["--offset-mm", "inf"],
        vec!["--note", "101"],
        vec!["--sample-rate", "44000"],
        vec!["--unknown", "1"],
        vec!["--note", "55", "--note", "57"],
        vec!["--note"],
    ] {
        let mut args = vec!["converge-pickup", "--output", "invalid.json"];
        args.extend(flags);
        assert!(!scratch.run(&args).status.success(), "{args:?}");
        assert!(!scratch.0.join("invalid.json").exists());
    }
}

#[test]
fn mechanical_pickup_pair_preserves_production_wav_and_all_existing_outputs() {
    let scratch = Scratch::new();
    let options = [
        "--note",
        "55",
        "--sample-rate",
        "44100",
        "--seconds",
        "0.8",
        "--hold",
        "0.7",
    ];
    let mut args = vec!["render-pickup-pair", "--output", "pair.wav"];
    args.extend(options);
    scratch.success(&args);
    let mut regular = vec!["render", "--output", "regular.wav"];
    regular.extend(options);
    scratch.success(&regular);
    assert_eq!(
        fs::read(scratch.0.join("pair.wav")).unwrap(),
        fs::read(scratch.0.join("regular.wav")).unwrap()
    );
    scratch.success(&["inspect", "pair-point-pole.wav"]);
    let report = scratch.json("pair-pickup-pair.json");
    assert_eq!(report["faults"], 0);
    assert_eq!(report["mechanics"]["internal_samples"], 141120);
    assert_eq!(
        report["tone_comparison"]["windows"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(
        report["point_pole_levels"]["rms"].as_f64().unwrap()
            > report["production_levels"]["rms"].as_f64().unwrap()
    );
    let before = fs::read(scratch.0.join("pair-pickup-pair.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("pair-pickup-pair.json")).unwrap()
    );
    for existing in ["blocked-point-pole.wav", "blocked-pickup-pair.json"] {
        fs::write(scratch.0.join(existing), "preserve").unwrap();
        assert!(
            !scratch
                .run(&["render-pickup-pair", "--output", "blocked.wav"])
                .status
                .success()
        );
        assert!(!scratch.0.join("blocked.wav").exists());
        assert_eq!(fs::read(scratch.0.join(existing)).unwrap(), b"preserve");
        fs::remove_file(scratch.0.join(existing)).unwrap();
    }
    for flags in [
        vec!["--trace"],
        vec!["--seconds", "10.1"],
        vec!["--seconds", "0.64", "--hold", "0.6"],
        vec!["--hold", "0.59"],
        vec!["--velocity", "NaN"],
        vec!["--gap-mm", "0"],
        vec!["--offset-mm", "4"],
        vec!["--note", "27"],
        vec!["--velocity", "0.5", "--velocity", "0.6"],
    ] {
        let mut args = vec!["render-pickup-pair", "--output", "invalid.wav"];
        args.extend(flags);
        assert!(!scratch.run(&args).status.success(), "{args:?}");
        assert!(!scratch.0.join("invalid.wav").exists());
        assert!(!scratch.0.join("invalid-point-pole.wav").exists());
        assert!(!scratch.0.join("invalid-pickup-pair.json").exists());
    }
}

#[test]
fn pickup_transfer_reports_both_laws_and_rejects_invalid_or_existing_output() {
    let scratch = Scratch::new();
    let args = [
        "pickup-transfer",
        "--output",
        "transfer.json",
        "--period-frames",
        "32",
        "--amplitudes-mm",
        "0.25",
    ];
    scratch.success(&args);
    let report = scratch.json("transfer.json");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["frequency_hz"], 1378.125);
    assert_eq!(report["last_retained_harmonic"], 13);
    let rows = report["observations"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["law"], "production");
    assert_eq!(rows[1]["law"], "research_point_pole");
    assert_eq!(rows[0]["sampling_errors"].as_array().unwrap().len(), 5);
    assert_eq!(rows[1]["reference_harmonics"].as_array().unwrap().len(), 12);
    let before = fs::read(scratch.0.join("transfer.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(before, fs::read(scratch.0.join("transfer.json")).unwrap());
    for flags in [
        vec!["--period-frames", "15"],
        vec!["--period-frames", "513"],
        vec!["--sample-rate", "44000"],
        vec!["--gap-mm", "NaN"],
        vec!["--offset-mm", "inf"],
        vec!["--amplitudes-mm", "0"],
        vec!["--amplitudes-mm", "0.1,0.1"],
        vec!["--amplitudes-mm", "0.1,0.2,0.3,0.4,0.5,0.6"],
        vec!["--amplitudes-mm", "NaN"],
        vec!["--gap-mm", "1", "--gap-mm", "2"],
        vec!["--unknown", "1"],
        vec!["--period-frames"],
    ] {
        let mut args = vec!["pickup-transfer", "--output", "invalid.json"];
        args.extend(flags);
        assert!(!scratch.run(&args).status.success(), "{args:?}");
        assert!(!scratch.0.join("invalid.json").exists());
    }
}

#[test]
fn tone_comparison_roundtrip_needs_no_sustain_claim_and_preserves_output() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "tone.wav",
        "--seconds",
        "0.8",
        "--hold",
        "0.7",
    ]);
    let args = [
        "compare-tone",
        "tone.wav",
        "tone.wav",
        "--note",
        "57",
        "--output",
        "tone-comparison.json",
    ];
    scratch.success(&args);
    let report = scratch.json("tone-comparison.json");
    let tone = &report["tone_comparison"];
    assert_eq!(tone["schema_version"], 1);
    assert_eq!(tone["windows"].as_array().unwrap().len(), 3);
    assert!(tone.get("decay").is_none());
    assert_eq!(
        tone["windows"][2]["harmonics"][0]["candidate_minus_reference_balance_db"],
        0.0
    );
    let before = fs::read(scratch.0.join("tone-comparison.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        before,
        fs::read(scratch.0.join("tone-comparison.json")).unwrap()
    );
    for flags in [
        vec![],
        vec!["--note", "128"],
        vec!["--note", "57", "--candidate-start", "0.3"],
        vec!["--note", "57", "--unknown", "1"],
        vec!["--note", "57", "--note", "55"],
    ] {
        let mut invalid = vec![
            "compare-tone",
            "tone.wav",
            "tone.wav",
            "--output",
            "bad.json",
        ];
        invalid.extend(flags);
        assert!(!scratch.run(&invalid).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn pickup_set_freezes_fit_geometry_and_gain_before_validation() {
    let scratch = Scratch::new();
    for (file, velocity, gap) in [
        ("soft.wav", "0.3", "2"),
        ("loud.wav", "0.7", "2"),
        ("held.wav", "0.5", "2"),
        ("different.wav", "0.5", "1"),
    ] {
        scratch.success(&[
            "render",
            "--output",
            file,
            "--note",
            "57",
            "--velocity",
            velocity,
            "--gap-mm",
            gap,
            "--offset-mm",
            "0.75",
            "--seconds",
            "0.7",
            "--hold",
            "0.6",
        ]);
    }
    let take = |id: &str, file: &str, role: &str, velocity: f64| {
        serde_json::json!({
            "id":id,"file":file,"role":role,"note":57,"velocity":velocity,
            "velocity_basis":"Known synthetic model input", "reference_start_seconds":0.1,
            "model_start_seconds":0.1,"seconds":0.4,"sustain_end_seconds":0.6,
        })
    };
    let mut manifest = serde_json::json!({
        "schema_version":1,"source":"Local synthetic fixture","source_revision":"Model 0.1.1",
        "license":"Project-generated test signal","instrument":"RF-73 research model",
        "processing":"None","capture_gain":"Fixed engine output gain",
        "gaps_mm":[1.5,2],"offsets_mm":[0.5,0.75],
        "takes":[take("soft", "soft.wav", "fit", 0.3), take("loud", "loud.wav", "fit", 0.7),
            take("held", "held.wav", "validation", 0.5)]
    });
    fs::write(
        scratch.0.join("set.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    scratch.success(&["fit-pickup-set", "set.json", "--output", "fit.json"]);
    let report = scratch.json("fit.json");
    assert_eq!(report["best_candidate_index"], 3);
    assert_eq!(report["validation_objective_db"], 0.0);
    assert_eq!(report["candidates"][3]["fit"]["applied_gain"], 1.0);
    assert_eq!(report["validation"].as_array().unwrap().len(), 1);
    let before = fs::read(scratch.0.join("fit.json")).unwrap();
    assert!(
        !scratch
            .run(&["fit-pickup-set", "set.json", "--output", "fit.json"])
            .status
            .success()
    );
    assert_eq!(before, fs::read(scratch.0.join("fit.json")).unwrap());

    // Deliberately change only the held-out geometry. It must not affect fitting.
    manifest["takes"][2]["file"] = "different.wav".into();
    fs::write(
        scratch.0.join("set.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    scratch.success(&[
        "fit-pickup-set",
        "set.json",
        "--output",
        "validation-changed.json",
    ]);
    let changed = scratch.json("validation-changed.json");
    assert_eq!(changed["candidates"], report["candidates"]);
    assert_eq!(changed["ranking_indices"], report["ranking_indices"]);
    assert!(changed["validation_objective_db"].as_f64().unwrap() > 0.05);
    assert_eq!(
        changed["validation"][0]["metrics"]["applied_candidate_gain"],
        1.0
    );
    assert!(
        changed["validation"][0]["metrics"]["applied_gain_normalized_rmse"]
            .as_f64()
            .unwrap()
            > 0.1
    );

    for invalid in [
        {
            let mut m = manifest.clone();
            m["takes"][2]["velocity"] = 0.3.into();
            m
        },
        {
            let mut m = manifest.clone();
            m["takes"][2]["file"] = "soft.wav".into();
            m
        },
        {
            let mut m = manifest.clone();
            m["takes"][0]["sustain_end_seconds"] = 0.2.into();
            m
        },
        {
            let mut m = manifest.clone();
            m["processing"] = "".into();
            m
        },
        {
            let mut m = manifest.clone();
            m["takes"][1]["role"] = "validation".into();
            m
        },
        {
            let mut m = manifest.clone();
            m["schema_version"] = 2.into();
            m
        },
        {
            let mut m = manifest.clone();
            m["gaps_mm"] = serde_json::json!([1, 1]);
            m
        },
        {
            let mut m = manifest.clone();
            m["unknown_option"] = true.into();
            m
        },
    ] {
        fs::write(
            scratch.0.join("invalid.json"),
            serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap();
        assert!(
            !scratch
                .run(&["fit-pickup-set", "invalid.json", "--output", "bad.json"])
                .status
                .success()
        );
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn pickup_sweep_recovers_known_geometry_and_preserves_files() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "reference.wav",
        "--seconds",
        "0.8",
        "--hold",
        "0.7",
        "--note",
        "57",
        "--velocity",
        "0.7",
        "--gap-mm",
        "2",
        "--offset-mm",
        "0.75",
    ]);
    let reference_before = fs::read(scratch.0.join("reference.wav")).unwrap();
    let args = [
        "sweep-pickup",
        "reference.wav",
        "--output",
        "sweep.json",
        "--note",
        "57",
        "--velocity",
        "0.7",
        "--seconds",
        "0.5",
        "--reference-start",
        "0.1",
        "--model-start",
        "0.1",
        "--gaps-mm",
        "1.5,2",
        "--offsets-mm",
        "0.5,0.75",
    ];
    scratch.success(&args);
    let report = scratch.json("sweep.json");
    assert_eq!(report["best_candidate_index"], 3);
    assert_eq!(report["ranking_indices"].as_array().unwrap().len(), 4);
    assert_eq!(report["reference_start_frame"], 4800);
    assert_eq!(report["model_start_frame"], 4800);
    assert_eq!(report["compared_frames"], 24000);
    let metrics = &report["candidates"][3]["metrics"];
    assert_eq!(metrics["objective_db"], 0.0);
    assert_eq!(metrics["raw_normalized_rmse"], 0.0);
    assert_eq!(metrics["scored_windows"], 7);
    assert_eq!(metrics["windows"][0]["requested_samples"], 6144);
    assert!(
        metrics["windows"][0]["spectral_observation_samples"]
            .as_u64()
            .unwrap()
            > 6000
    );
    let report_before = fs::read(scratch.0.join("sweep.json")).unwrap();
    assert!(!scratch.run(&args).status.success());
    assert_eq!(
        report_before,
        fs::read(scratch.0.join("sweep.json")).unwrap()
    );
    assert_eq!(
        reference_before,
        fs::read(scratch.0.join("reference.wav")).unwrap()
    );

    for (flag, value) in [
        ("--gaps-mm", "1,1.0"),
        ("--offsets-mm", "NaN"),
        ("--seconds", "0.01"),
        ("--note", "101"),
        ("--velocity", "0"),
        ("--reference-start", "0.4"),
        ("--model-start", "3"),
        ("--channel", "1"),
    ] {
        let mut invalid = args.to_vec();
        invalid[3] = "bad.json";
        if let Some(index) = invalid.iter().position(|v| *v == flag) {
            invalid[index + 1] = value;
        } else {
            invalid.extend([flag, value]);
        }
        assert!(!scratch.run(&invalid).status.success(), "{flag} {value}");
        assert!(!scratch.0.join("bad.json").exists());
    }
    let mut missing = args.to_vec();
    missing[3] = "bad.json";
    missing.drain(6..8); // The strike velocity must be supplied explicitly.
    assert!(!scratch.run(&missing).status.success());
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn partial_comparison_roundtrip_and_invalid_regions_preserve_files() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "a.wav",
        "--seconds",
        "1.2",
        "--hold",
        "1.1",
    ]);
    scratch.success(&[
        "compare-partials",
        "a.wav",
        "a.wav",
        "--output",
        "partials.json",
        "--seconds",
        "1",
    ]);
    let report = scratch.json("partials.json");
    let comparison = &report["partial_comparison"];
    assert_eq!(comparison["schema_version"], 1);
    assert_eq!(comparison["candidate_level_minus_reference_db"], 0.0);
    assert_eq!(comparison["reference_region"]["end_frame_exclusive"], 48000);
    assert!(!comparison["matches"].as_array().unwrap().is_empty());
    assert!(
        comparison["matches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["rms_frequency_error_cents"] == 0.0 && p["rms_level_error_db"] == 0.0)
    );
    let before = fs::read(scratch.0.join("partials.json")).unwrap();
    assert!(
        !scratch
            .run(&[
                "compare-partials",
                "a.wav",
                "a.wav",
                "--output",
                "partials.json",
                "--seconds",
                "1"
            ])
            .status
            .success()
    );
    assert_eq!(before, fs::read(scratch.0.join("partials.json")).unwrap());
    for flags in [
        vec![],
        vec!["--seconds", "NaN"],
        vec!["--seconds", "2"],
        vec!["--seconds", "1", "--candidate-start", "0.3"],
        vec!["--seconds", "1", "--match-cents", "200"],
        vec!["--seconds", "1", "--seconds", "1"],
        vec!["--seconds", "1", "--unknown", "1"],
    ] {
        let mut args = vec!["compare-partials", "a.wav", "a.wav", "--output", "bad.json"];
        args.extend(flags);
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn convergence_reports_reference_and_refinement_without_overwriting() {
    let scratch = Scratch::new();
    scratch.success(&[
        "converge",
        "--output",
        "convergence.json",
        "--seconds",
        "0.05",
    ]);
    let report = scratch.json("convergence.json");
    assert_eq!(report["reference"]["substeps"], 64);
    assert_eq!(report["comparisons"].as_array().unwrap().len(), 4);
    assert_eq!(report["frames"], 2400);
    for row in report["comparisons"].as_array().unwrap() {
        assert_eq!(row["full_audio"]["candidate_delay_samples"], 0);
        assert_eq!(row["full_audio"]["compared_frames"], 2400);
        assert!(row["mechanics"]["separation_seconds"].as_f64().unwrap() > 0.0);
        assert!(
            row["displacement_normalized_rmse"]
                .as_f64()
                .unwrap()
                .is_finite()
        );
    }
    assert!(
        report["comparisons"][3]["displacement_normalized_rmse"]
            .as_f64()
            .unwrap()
            < report["comparisons"][0]["displacement_normalized_rmse"]
                .as_f64()
                .unwrap()
    );
    let before = fs::read(scratch.0.join("convergence.json")).unwrap();
    assert!(
        !scratch
            .run(&["converge", "--output", "convergence.json"])
            .status
            .success()
    );
    assert_eq!(
        before,
        fs::read(scratch.0.join("convergence.json")).unwrap()
    );
    for args in [
        vec!["converge", "--output", "bad.json", "--velocity", "NaN"],
        vec!["converge", "--output", "bad.json", "--velocity", "1e-100"],
        vec!["converge", "--output", "bad.json", "--seconds", "10"],
        vec![
            "converge", "--output", "bad.json", "--note", "57", "--note", "58",
        ],
        vec!["converge", "--output", "bad.json", "--sample-rate", "8000"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn rendered_audio_can_be_analyzed_and_compared_without_overwriting() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "a.wav",
        "--seconds",
        "0.4",
        "--hold",
        "0.3",
    ]);
    scratch.success(&[
        "analyze",
        "a.wav",
        "--output",
        "analysis.json",
        "--note",
        "57",
        "--sustain-end",
        "0.29",
    ]);
    let report = scratch.json("analysis.json");
    assert_eq!(report["analysis"]["schema_version"], 2);
    assert!(
        !report["analysis"]["inharmonic_tracking"]["tracks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        report["analysis"]["inharmonic_tracking"]["minimum_separation_hz"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(
        report["analysis"]["fundamental"]["frequency_hz"]
            .as_f64()
            .unwrap()
            > 200.0
    );
    scratch.success(&["compare", "a.wav", "a.wav", "--output", "comparison.json"]);
    scratch.success(&[
        "analyze",
        "a.wav",
        "--output",
        "short-window.json",
        "--partial-window-ms",
        "32",
    ]);
    let short_window = scratch.json("short-window.json");
    assert_eq!(
        short_window["analysis"]["inharmonic_tracking"]["observed_samples"],
        1536
    );
    assert_eq!(
        short_window["analysis"]["inharmonic_tracking"]["window_seconds"],
        0.032
    );
    assert!(
        short_window["analysis"]["inharmonic_tracking"]["frames"]
            .as_array()
            .unwrap()
            .len()
            > report["analysis"]["inharmonic_tracking"]["frames"]
                .as_array()
                .unwrap()
                .len()
    );
    let report = scratch.json("comparison.json");
    assert_eq!(report["comparison"]["candidate_delay_samples"], 0);
    assert_eq!(report["comparison"]["raw_normalized_rmse"], 0.0);
    let before = fs::read(scratch.0.join("analysis.json")).unwrap();
    assert!(
        !scratch
            .run(&["analyze", "a.wav", "--output", "analysis.json"])
            .status
            .success()
    );
    assert_eq!(before, fs::read(scratch.0.join("analysis.json")).unwrap());
    for args in [
        vec![
            "analyze", "a.wav", "--output", "bad.json", "--note", "57", "--note", "58",
        ],
        vec![
            "analyze",
            "a.wav",
            "--output",
            "bad.json",
            "--sustain-end",
            "NaN",
        ],
        vec!["analyze", "a.wav", "--output", "bad.json", "--unknown", "1"],
        vec!["analyze", "missing.wav", "--output", "bad.json"],
        vec![
            "analyze",
            "a.wav",
            "--output",
            "bad.json",
            "--partial-window-ms",
            "0",
        ],
        vec![
            "analyze",
            "a.wav",
            "--output",
            "bad.json",
            "--partial-window-ms",
            "NaN",
        ],
        vec![
            "analyze",
            "a.wav",
            "--output",
            "bad.json",
            "--partial-window-ms",
            "2048",
        ],
        vec![
            "analyze",
            "a.wav",
            "--output",
            "bad.json",
            "--partial-window-ms",
            "32",
            "--partial-window-ms",
            "128",
        ],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
    }
}

#[test]
fn comparison_rate_mismatch_fails_without_creating_a_report() {
    let scratch = Scratch::new();
    scratch.success(&[
        "render",
        "--output",
        "a.wav",
        "--seconds",
        "0.1",
        "--hold",
        "0.05",
    ]);
    scratch.success(&[
        "render",
        "--output",
        "b.wav",
        "--seconds",
        "0.1",
        "--hold",
        "0.05",
        "--sample-rate",
        "44100",
    ]);
    let result = scratch.run(&["compare", "a.wav", "b.wav", "--output", "bad.json"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("equal sample rates"));
    assert!(!scratch.0.join("bad.json").exists());
}

#[test]
fn spring_tuning_preflights_every_output_and_rejects_invalid_reference() {
    let scratch = Scratch::new();
    for file in ["pair.wav", "pair-before.wav", "pair.json"] {
        fs::write(scratch.0.join(file), b"preserve").unwrap();
        let out = scratch.run(&["tune-modal-pitch", "missing.json", "--output", "pair.wav"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("refusing to overwrite"));
        assert_eq!(fs::read(scratch.0.join(file)).unwrap(), b"preserve");
        fs::remove_file(scratch.0.join(file)).unwrap();
    }
    fs::write(scratch.0.join("invalid.json"), b"{}").unwrap();
    let out = scratch.run(&["tune-modal-pitch", "invalid.json", "--output", "pair.wav"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("qualified G3"));
    assert!(!scratch.0.join("pair.wav").exists());
    assert!(!scratch.0.join("pair-before.wav").exists());
    assert!(!scratch.0.join("pair.json").exists());
}

#[test]
fn pickup_mixing_rejects_invalid_arguments_and_preserves_existing_output() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("existing.json"), b"preserve").unwrap();
    let result = scratch.run(&["pickup-mixing", "--output", "existing.json"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("new .json file"));
    assert_eq!(
        fs::read(scratch.0.join("existing.json")).unwrap(),
        b"preserve"
    );
    for args in [
        vec!["pickup-mixing"],
        vec!["pickup-mixing", "--unknown", "bad.json"],
        vec!["pickup-mixing", "--output", "bad.json", "--unknown"],
        vec!["pickup-mixing", "--output", "bad.wav"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}

#[test]
fn pickup_decay_reports_controls_and_preserves_existing_output() {
    let scratch = Scratch::new();
    scratch.success(&["pickup-decay", "--output", "decay.json"]);
    let report = scratch.json("decay.json");
    assert_eq!(report["all_cases_qualified"], true);
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        assert_eq!(case["qualified"], true);
        assert_eq!(case["observations"].as_array().unwrap().len(), 6);
        assert_eq!(case["decay_fits"].as_array().unwrap().len(), 2);
        assert!(case["max_refinement_relative_error"].as_f64().unwrap() < 1e-8);
    }
    let before = fs::read(scratch.0.join("decay.json")).unwrap();
    assert!(
        !scratch
            .run(&["pickup-decay", "--output", "decay.json"])
            .status
            .success()
    );
    assert_eq!(before, fs::read(scratch.0.join("decay.json")).unwrap());
    for args in [
        vec!["pickup-decay"],
        vec!["pickup-decay", "--unknown", "bad.json"],
        vec!["pickup-decay", "--output", "bad.json", "--unknown"],
        vec!["pickup-decay", "--output", "bad.wav"],
    ] {
        assert!(!scratch.run(&args).status.success());
        assert!(!scratch.0.join("bad.json").exists());
        assert!(!scratch.0.join("bad.wav").exists());
    }
}
