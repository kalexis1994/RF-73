use rf_73_analysis::{AudioClip, PairedDecayStatus, PartialComparisonOptions, compare_partials};
use std::{collections::BTreeMap, error::Error, path::Path};

pub const HELP: &str = "Partial comparison:
  compare-partials REFERENCE.wav CANDIDATE.wav --output REPORT.json --seconds S
    [--reference-start S] [--candidate-start S] [--partial-window-ms 128]
    [--match-cents 50] [--reference-channel 0] [--candidate-channel 0]
Select equal-duration regions of uninterrupted sustain at corresponding note ages.
Starts default to zero. No automatic time alignment, resampling or gain changes.
Window: 32/128/512/1024 ms. Matching tolerance: 0..100 cents.
Decay differences require qualified fits with identical intervals and complete pairing.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    let rest = &args[3..];
    if !rest.len().is_multiple_of(2) {
        return Err("partial comparison options require a flag and value".into());
    }
    for pair in rest.as_chunks::<2>().0 {
        let flag = pair[0].as_str();
        if !matches!(
            flag,
            "--output"
                | "--seconds"
                | "--reference-start"
                | "--candidate-start"
                | "--partial-window-ms"
                | "--match-cents"
                | "--reference-channel"
                | "--candidate-channel"
        ) || flags.insert(flag, pair[1].as_str()).is_some()
        {
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
    let options = PartialComparisonOptions {
        seconds: flags
            .get("--seconds")
            .ok_or("--seconds is required to select an uninterrupted sustain region")?
            .parse()?,
        reference_start_seconds: flags
            .get("--reference-start")
            .map_or(Ok(0.0), |v| v.parse())?,
        candidate_start_seconds: flags
            .get("--candidate-start")
            .map_or(Ok(0.0), |v| v.parse())?,
        partial_window_ms: flags
            .get("--partial-window-ms")
            .map_or(Ok(128), |v| v.parse())?,
        match_cents: flags.get("--match-cents").map_or(Ok(50.0), |v| v.parse())?,
    };
    let reference = AudioClip::open(
        &args[1],
        flags
            .get("--reference-channel")
            .map(|v| v.parse::<u16>())
            .transpose()?,
    )?;
    let candidate = AudioClip::open(
        &args[2],
        flags
            .get("--candidate-channel")
            .map(|v| v.parse::<u16>())
            .transpose()?,
    )?;
    let report = compare_partials(&reference, &candidate, options)?;
    super::analysis::write_report(
        output,
        &serde_json::json!({
            "reference": args[1], "candidate": args[2],
            "reference_audio": reference.metadata(), "candidate_audio": candidate.metadata(),
            "partial_comparison": report,
        }),
    )?;
    println!("Partial comparison: {}", output.display());
    println!(
        "pairs={} matched_observations={} qualified_decay_pairs={} detection_complete={} level_delta_db={:?}",
        report.matches.len(),
        report.reference_observations.matched,
        report
            .matches
            .iter()
            .filter(|m| m.decay.status == PairedDecayStatus::Qualified)
            .count(),
        report.detection_complete,
        report.candidate_level_minus_reference_db
    );
    println!(
        "reference_no_counterpart={} candidate_no_counterpart={} reference_ambiguous={} candidate_ambiguous={}",
        report.reference_observations.no_counterpart,
        report.candidate_observations.no_counterpart,
        report.reference_observations.ambiguous_match,
        report.candidate_observations.ambiguous_match
    );
    Ok(())
}
