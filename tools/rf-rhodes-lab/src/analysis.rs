use rf_rhodes_analysis::{AnalysisOptions, AudioClip, analyze, compare};
use serde::Serialize;
use std::{collections::BTreeMap, error::Error, io::Write, path::Path};

pub const HELP: &str = "Measurements:
  analyze INPUT.wav --output REPORT.json [--note 57] [--channel 0] [--sustain-end 1.8]
  compare REFERENCE.wav CANDIDATE.wav --output REPORT.json [--align-ms 20]
          [--reference-channel 0] [--candidate-channel 0]
WAV: PCM 8/16/24/32 or float32, 8..192 kHz, at most 60 seconds.
Multichannel files require explicit zero-based channel selection; no downmixing.
Comparison requires equal sample rates. Reports never overwrite existing files.
Decay is estimated only when --sustain-end marks the end of an uninterrupted
sustain region in seconds from file start. Extrapolated T60 is not measured T60.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let positional = if args[0] == "analyze" { 2 } else { 3 };
    if args.len() < positional {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    let remaining = &args[positional..];
    if !remaining.len().is_multiple_of(2) {
        return Err("measurement options require a flag and value".into());
    }
    for pair in remaining.as_chunks::<2>().0 {
        let flag = pair[0].as_str();
        let allowed = if args[0] == "analyze" {
            matches!(flag, "--output" | "--note" | "--channel" | "--sustain-end")
        } else {
            matches!(
                flag,
                "--output" | "--align-ms" | "--reference-channel" | "--candidate-channel"
            )
        };
        if !allowed || flags.insert(flag, pair[1].as_str()).is_some() {
            return Err(format!("unknown or duplicate option: {flag}").into());
        }
    }
    let output = Path::new(
        flags
            .get("--output")
            .ok_or("--output REPORT.json is required")?,
    );
    if output.extension().is_none_or(|s| s != "json") {
        return Err("report must use .json extension".into());
    }
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }
    if args[0] == "analyze" {
        let channel = flags
            .get("--channel")
            .map(|value| value.parse::<u16>())
            .transpose()?;
        let clip = AudioClip::open(&args[1], channel)?;
        let options = AnalysisOptions {
            note: flags.get("--note").map_or(Ok(57), |v| v.parse())?,
            sustain_end_seconds: flags
                .get("--sustain-end")
                .map(|v| v.parse::<f64>())
                .transpose()?,
        };
        let report = analyze(&clip, options)?;
        write_report(
            output,
            &serde_json::json!({"input": args[1], "analysis": report}),
        )?;
        println!("Analysis: {}", output.display());
        println!(
            "duration={:.3}s peak={:.6} fundamental_hz={:?} tuning_cents={:?} extrapolated_t60_s={:?}",
            report.duration_seconds,
            report.peak,
            report.fundamental.as_ref().map(|p| p.frequency_hz),
            report.tuning_error_cents,
            report
                .decay
                .as_ref()
                .and_then(|d| d.extrapolated_t60_seconds)
        );
    } else {
        let reference_channel = flags
            .get("--reference-channel")
            .map(|v| v.parse::<u16>())
            .transpose()?;
        let candidate_channel = flags
            .get("--candidate-channel")
            .map(|v| v.parse::<u16>())
            .transpose()?;
        let reference = AudioClip::open(&args[1], reference_channel)?;
        let candidate = AudioClip::open(&args[2], candidate_channel)?;
        let align_ms = flags
            .get("--align-ms")
            .map_or(Ok(20.0), |v| v.parse::<f64>())?;
        let report = compare(&reference, &candidate, align_ms)?;
        write_report(
            output,
            &serde_json::json!({"reference": args[1], "candidate": args[2],
            "reference_audio": reference.metadata(), "candidate_audio": candidate.metadata(), "comparison": report}),
        )?;
        println!("Comparison: {}", output.display());
        println!(
            "delay_samples={} level_delta_db={:?} raw_nrmse={:?} level_matched_nrmse={:?}",
            report.candidate_delay_samples,
            report.candidate_level_minus_reference_db,
            report.raw_normalized_rmse,
            report.level_matched_normalized_rmse
        );
    }
    Ok(())
}

pub(crate) fn write_report(path: &Path, report: &impl Serialize) -> Result<(), Box<dyn Error>> {
    // Serialize before opening the output so serialization failure creates no file.
    let bytes = serde_json::to_vec_pretty(report)?;
    let mut file = super::new_file(path)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(())
}
