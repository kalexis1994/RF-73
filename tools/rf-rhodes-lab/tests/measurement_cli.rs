use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);
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
            let path = std::env::temp_dir().join(format!(
                "rf-rhodes-cli-{}-{unique}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("failed to reserve test directory: {error}"),
            }
        }
        panic!("could not reserve a unique test directory");
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rf-rhodes-lab"))
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
