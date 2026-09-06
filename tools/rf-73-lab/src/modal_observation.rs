//! Observe pinned audio at its native rate against explicit proposed resonances.
use rf_73_analysis::observe_modes;
use std::{collections::BTreeMap, error::Error, path::Path};

pub const HELP: &str = "Modal spectral evidence:
  observe-modes INPUT.wav --blob-sha1 HASH --fundamental HZ --modes HZ,HZ,...
    --output REPORT.json
Git verifies the exact bytes decoded. Supply 1..16 increasing modes, 20..20000 Hz.
Full 32/128 ms attack windows and 512 ms from 250 ms, at the native sample rate.
No forced peak fit, onset alignment, resampling, decay fit or amplitude changes.
Reports harmonic/neighbor confusion and non-detections; proximity is not identity.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 10 {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    for [key, value] in args[2..].as_chunks::<2>().0 {
        if !["--blob-sha1", "--fundamental", "--modes", "--output"].contains(&key.as_str())
            || flags.insert(key.as_str(), value.as_str()).is_some()
        {
            return Err(HELP.into());
        }
    }
    let output = Path::new(flags["--output"]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let hash = flags["--blob-sha1"];
    if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected a 40-digit Git blob SHA-1".into());
    }
    let fundamental = flags["--fundamental"].parse::<f64>()?;
    let targets = flags["--modes"]
        .split(',')
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()?;
    let clip = crate::pitch_reference::verified_audio(Path::new(&args[1]), hash)?;
    let observation = observe_modes(&clip, &targets, fundamental)?;
    crate::analysis::write_report(
        output,
        &serde_json::json!({"input":args[1],"git_blob_sha1":hash.to_lowercase(),
        "audio":clip.metadata(),"observation":observation,
        "scope":"Frequency candidates near proposed undamped modes. Detector thresholds and window duration limit non-detections. Spectral coincidence does not identify a mechanical mode or establish instrument fidelity; gain, excitation and processing are not inferred."}),
    )?;
    println!("Modal observation: {}", output.display());
    Ok(())
}
