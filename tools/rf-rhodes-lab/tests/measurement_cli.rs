use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("rf-rhodes-cli-{}-{unique}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
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
