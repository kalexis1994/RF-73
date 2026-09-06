use rf_73_analysis::{AudioClip, ToneComparisonOptions, compare_tone};
use std::{collections::BTreeMap, error::Error, path::Path};

pub const HELP: &str = "Tone comparison:
  compare-tone REFERENCE.wav CANDIDATE.wav --note N --output REPORT.json
    [--reference-start S] [--candidate-start S]
    [--reference-channel 0] [--candidate-channel 0]
Explicit anchors (default 0), then 32/96 ms attack and 250..600 ms body windows.
Reports raw spectra and harmonic balance relative to each fundamental.
No inferred velocity, onset alignment, resampling, decay fit or aggregate ranking.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 || !(args.len() - 3).is_multiple_of(2) {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    for pair in args[3..].as_chunks::<2>().0 {
        let flag = pair[0].as_str();
        if !matches!(
            flag,
            "--note"
                | "--output"
                | "--reference-start"
                | "--candidate-start"
                | "--reference-channel"
                | "--candidate-channel"
        ) || flags.insert(flag, pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or duplicate tone option: {flag}").into());
        }
    }
    let output = Path::new(flags.get("--output").ok_or("--output is required")?);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let options = ToneComparisonOptions {
        note: flags.get("--note").ok_or("--note is required")?.parse()?,
        reference_start_seconds: flags
            .get("--reference-start")
            .map_or(Ok(0.0), |s| s.parse())?,
        candidate_start_seconds: flags
            .get("--candidate-start")
            .map_or(Ok(0.0), |s| s.parse())?,
    };
    let a = AudioClip::open(
        &args[1],
        flags
            .get("--reference-channel")
            .map(|s| s.parse::<u16>())
            .transpose()?,
    )?;
    let b = AudioClip::open(
        &args[2],
        flags
            .get("--candidate-channel")
            .map(|s| s.parse::<u16>())
            .transpose()?,
    )?;
    let comparison = compare_tone(&a, &b, options)?;
    super::analysis::write_report(
        output,
        &serde_json::json!({
            "reference": args[1], "candidate": args[2], "reference_audio":a.metadata(),
            "candidate_audio":b.metadata(), "tone_comparison":comparison,
        }),
    )?;
    println!("Tone comparison: {}", output.display());
    Ok(())
}
