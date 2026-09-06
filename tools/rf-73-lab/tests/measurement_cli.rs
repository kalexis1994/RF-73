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
fn geometry_study_preserves_outputs_and_rejects_unqualified_reference() {
    let scratch = Scratch::new();
    fs::write(scratch.0.join("study.json"), b"preserve").unwrap();
    let args = [
        "sweep-tuned-geometry",
        "missing.json",
        "--output",
        "study.json",
    ];
    let out = scratch.run(&args);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("new .json"));
    assert_eq!(fs::read(scratch.0.join("study.json")).unwrap(), b"preserve");
    fs::write(scratch.0.join("invalid.json"), b"{}").unwrap();
    let out = scratch.run(&[
        "sweep-tuned-geometry",
        "invalid.json",
        "--output",
        "new.json",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("qualified G3"));
    assert!(!scratch.0.join("new.json").exists());
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
