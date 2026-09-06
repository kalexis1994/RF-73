//! Prepare a frequency target from declared training takes; validation never fits it.
use rf_73_analysis::{AudioClip, PitchAnchor, pitch_anchor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::{Cursor, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

pub const HELP: &str = "Frequency reference preparation:
  prepare-pitch-reference MANIFEST.json --output REPORT.json [--candidate PATH.wav]
Strict schema-1 manifest; 3..8 takes, >=2 training and >=1 validation.
Git verifies pinned blob identities before the same bytes are decoded.
Three 512 ms observations after 0.25 s; broad +/-400-cent search, no gain fit.
Training alone fits the equal-take log-frequency mean. Validation only evaluates.
Optional candidate needs >=1.786 s; no WAVs, DSP parameters or decay fits generated.
";

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Training,
    Validation,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Take {
    id: String,
    file: String,
    git_blob_sha1: String,
    role: Role,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    note: u8,
    source: String,
    source_revision: String,
    instrument: String,
    processing: String,
    capture_gain: String,
    prior_exposure: String,
    takes: Vec<Take>,
}
impl Manifest {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        let mut ids = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        if self.schema_version != 1
            || !(40..=90).contains(&self.note)
            || !(3..=8).contains(&self.takes.len())
            || [
                &self.source,
                &self.source_revision,
                &self.instrument,
                &self.processing,
                &self.capture_gain,
                &self.prior_exposure,
            ]
            .iter()
            .any(|s| s.trim().is_empty())
            || self
                .takes
                .iter()
                .filter(|t| t.role == Role::Training)
                .count()
                < 2
            || !self.takes.iter().any(|t| t.role == Role::Validation)
            || self.takes.iter().any(|t| {
                t.id.trim().is_empty()
                    || t.file.trim().is_empty()
                    || !ids.insert(&t.id)
                    || t.git_blob_sha1.len() != 40
                    || !t.git_blob_sha1.bytes().all(|b| b.is_ascii_hexdigit())
                    || !hashes.insert(t.git_blob_sha1.to_lowercase())
            })
        {
            return Err("invalid pitch manifest, duplicate content, or missing training/validation provenance".into());
        }
        Ok(())
    }
}

pub(crate) fn verified_audio(path: &Path, expected: &str) -> Result<AudioClip, Box<dyn Error>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(32_000_001)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 32_000_000 {
        return Err("pitch reference file exceeds 32 MB".into());
    }
    let mut child = Command::new("git")
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing hash input pipe")?
        .write_all(&bytes)?;
    let result = child.wait_with_output()?;
    if !result.status.success()
        || String::from_utf8(result.stdout)?.trim() != expected.to_lowercase()
    {
        return Err(format!(
            "reference blob mismatch or git hash failure: {}",
            path.display()
        )
        .into());
    }
    Ok(AudioClip::read(Cursor::new(bytes), None)?)
}

fn target(anchors: &[(Role, PitchAnchor)]) -> Option<f64> {
    let training: Vec<_> = anchors
        .iter()
        .filter(|(r, _)| *r == Role::Training)
        .collect();
    if training.len() < 2
        || training
            .iter()
            .any(|(_, a)| !a.qualified || a.frequency_hz.is_none_or(|f| !f.is_finite() || f <= 0.0))
    {
        return None;
    }
    Some(
        (training
            .iter()
            .map(|(_, a)| a.frequency_hz.unwrap().ln())
            .sum::<f64>()
            / training.len() as f64)
            .exp(),
    )
}
fn residual(anchor: &PitchAnchor, target: Option<f64>) -> Value {
    let cents = anchor
        .frequency_hz
        .zip(target)
        .map(|(f, t)| 1200.0 * (f / t).log2());
    json!({"frequency_error_cents":cents,"consistent_with_5_cent_pilot_limit":
        anchor.qualified && cents.is_some_and(|c|c.abs()<=5.0)})
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 && args.len() != 6
        || args.get(2).is_none_or(|s| s != "--output")
        || args.len() == 6 && args[4] != "--candidate"
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|e| e != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let input = Path::new(&args[1]);
    let mut bytes = Vec::new();
    fs::File::open(input)?.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("pitch manifest exceeds 64 KiB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    let parent = input.parent().unwrap_or(Path::new("."));
    let mut anchors = Vec::new();
    for take in &manifest.takes {
        let clip = verified_audio(&parent.join(&take.file), &take.git_blob_sha1)?;
        let anchor = pitch_anchor(&clip, manifest.note)?;
        println!(
            "Pitch anchor {}: qualified={}, frequency={:?}",
            take.id, anchor.qualified, anchor.frequency_hz
        );
        anchors.push((take.role, anchor));
    }
    let frozen = target(&anchors);
    let rows: Vec<_> = manifest
        .takes
        .iter()
        .zip(&anchors)
        .map(|(t, (_, a))| {
            json!({"take":t,
        "anchor":a,"frozen_target_residual":residual(a,frozen)})
        })
        .collect();
    let qualified = frozen.is_some()
        && rows
            .iter()
            .all(|r| r["frozen_target_residual"]["consistent_with_5_cent_pilot_limit"] == true);
    let candidate = if args.len() == 6 {
        let clip = AudioClip::open(&args[5], None)?;
        let anchor = pitch_anchor(&clip, manifest.note)?;
        json!({"file":args[5],"anchor":anchor,"frozen_target_residual":residual(&anchor,frozen),
            "role":"Diagnostic only; never participates in reference target fitting or qualification"})
    } else {
        Value::Null
    };
    let report = json!({"schema_version":1,"experiment":"frequency-reference-preparation-v1",
        "reference_qualification_passed":qualified,"training_target_hz":frozen,"manifest":manifest,
        "takes":rows,"candidate":candidate,
        "protocol":{"search_half_width_cents":400,"window_seconds":0.512,"start_seconds":0.25,"window_count":3,
            "minimum_band_background_margin_db":24,"competitor_margin_db":20,"maximum_temporal_span_cents":5,
            "maximum_between_take_error_cents":5,"target":"Equal-take arithmetic mean in log frequency; training only"},
        "scope":"Frequency-only target from processed output recordings. Broad-band dominant local peak is not a proven fundamental or structural eigenmode. Unresolved close components can remain hidden. Reciprocal window duration and temporal spread are not confidence intervals. Reused layers are a declared protocol split, not blind independent data. No decay/material/geometry/gain/velocity identification, DSP mutation, WAV generation or listening claim. Git blob identity is verified on the exact decoded bytes; candidate is diagnostic and never fitted."});
    crate::analysis::write_report(output, &report)?;
    if !qualified {
        return Err("reference target unqualified; report retained".into());
    }
    println!(
        "Frozen frequency target: {:.6} Hz; report {}",
        frozen.unwrap(),
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn anchor(f: f64) -> PitchAnchor {
        PitchAnchor {
            qualified: true,
            frequency_hz: Some(f),
            temporal_span_cents: Some(0.0),
            search_band_hz: [150.0, 250.0],
            observation_resolution_hz: 1.953125,
            windows: Vec::new(),
            rejection_reasons: Vec::new(),
        }
    }
    #[test]
    fn validation_cannot_select_target_and_training_failures_are_not_dropped() {
        let mut data = vec![
            (Role::Training, anchor(196.0)),
            (Role::Training, anchor(197.0)),
            (Role::Validation, anchor(230.0)),
        ];
        let frozen = target(&data).unwrap();
        assert!((frozen - (196.0_f64 * 197.0).sqrt()).abs() < 1e-10);
        data[2].1.frequency_hz = Some(170.0);
        data[2].1.qualified = false;
        assert_eq!(target(&data), Some(frozen));
        data[0].1.qualified = false;
        assert!(target(&data).is_none());
    }
}
