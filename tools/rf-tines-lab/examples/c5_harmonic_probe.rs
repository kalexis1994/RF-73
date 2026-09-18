//! Frozen C5-only source qualification; no fitting or held-out audio.
use rf_tines_analysis::{AudioClip, detect_timbre_onset, pitch_anchor, probe_second_harmonic};
use serde_json::json;
use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: c5_harmonic_probe SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY".into());
    }
    let out = Path::new(&args[2]);
    fs::create_dir(out)?;
    let mut rows = Vec::new();
    for layer in ["p", "mp", "mf", "f"] {
        let file = format!("C4-{layer}.wav");
        let clip = AudioClip::open(Path::new(&args[1]).join(&file), None)?;
        let onset = detect_timbre_onset(&clip)?;
        let anchor = pitch_anchor(&clip, 72)?;
        let frequency = anchor
            .frequency_hz
            .ok_or("C5 pitch anchor is not qualified")?;
        for (window, offsets, durations) in [
            ("attack", [0.0, 0.002, 0.005], [0.080, 0.096, 0.112]),
            ("body", [0.23, 0.25, 0.27], [0.30, 0.35, 0.40]),
        ] {
            for offset in offsets {
                for duration in durations {
                    rows.push(
                        json!({"file":file,"layer":layer,"window":window,"offset_seconds":offset,
                        "onset_seconds":onset,"anchor_hz":frequency,
                        "probe":probe_second_harmonic(&clip,onset+offset,duration,frequency)?}),
                    );
                }
            }
        }
    }
    let receipt = json!({"scope":"C5 development recordings only; no parameter fitting or plugin change",
        "method":"Hann FFT; H2 peak around twice qualified H1; background annulus 3..8 resolution units; exploratory 24 dB median and 12 dB p90 margins",
        "stability_policy":"Report all windows and ranges; no aggregate pass threshold inferred after observing data",
        "observations":rows});
    fs::write(
        out.join("observations.json"),
        serde_json::to_string_pretty(&receipt)?,
    )?;
    println!("Wrote 72 C5 H2 observations to {}", out.display());
    Ok(())
}
