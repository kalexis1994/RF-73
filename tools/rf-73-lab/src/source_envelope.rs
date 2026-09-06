//! Pinned source-family envelope observations; failed gates are evidence too.
use rf_73_analysis::{AudioClip, ComponentEnvelope, EnvelopeOptions, measure_component_envelope};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    error::Error,
    fs::File,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
};

pub const HELP: &str = "Pinned source envelopes:
  observe-source-envelopes MANIFEST.json --output REPORT.json
Strict schema-1 selection manifest, pinned register receipt and all takes of one note.
Fixed prior attack peaks and pitch anchors; no frequency/interval fitting.
Native 128/256 ms envelopes, retained missing/rejected cases and conditional rate relations.
No source editing, natural-sustain assumption, geometry/loss calibration or playback.
";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    evidence_file: String,
    evidence_git_blob_sha1: String,
    note: u8,
    lower_family_band_hz: [f64; 2],
    upper_family_band_hz: [f64; 2],
    start_seconds: f64,
    end_seconds: f64,
    selection: String,
    path_base: String,
}
impl Manifest {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        let valid_band = |b: [f64; 2]| {
            b.iter().all(|x| x.is_finite())
                && b[0] >= 100.0
                && b[1] <= 20000.0
                && b[1] > b[0]
                && b[1] - b[0] <= 40.0
        };
        if self.schema_version != 1
            || !(40..=90).contains(&self.note)
            || self.evidence_file.trim().is_empty()
            || !valid_hash(&self.evidence_git_blob_sha1)
            || !valid_band(self.lower_family_band_hz)
            || !valid_band(self.upper_family_band_hz)
            || self.lower_family_band_hz[1] >= self.upper_family_band_hz[0]
            || !self.start_seconds.is_finite()
            || !self.end_seconds.is_finite()
            || self.start_seconds < 0.0
            || self.end_seconds - self.start_seconds < 0.8
            || self.end_seconds - self.start_seconds > 3.0
            || self.end_seconds > 60.0
            || self.selection.trim().is_empty()
            || self.path_base != "repository_working_directory"
        {
            return Err("invalid source-envelope manifest or missing selection provenance".into());
        }
        Ok(())
    }
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("source-envelope input exceeds byte limit".into());
    }
    Ok(bytes)
}
fn verified_evidence(path: &Path, expected: &str) -> Result<Evidence, Box<dyn Error>> {
    let bytes = read_bounded(path, 2_000_000)?;
    let mut child = Command::new("git")
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing evidence hash pipe")?
        .write_all(&bytes)?;
    let result = child.wait_with_output()?;
    if !result.status.success()
        || String::from_utf8(result.stdout)?.trim() != expected.to_lowercase()
    {
        return Err("source evidence blob mismatch or git hash failure".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
// Typed projection of the versioned register receipt; unrelated fields remain allowed.
#[derive(Deserialize)]
struct Evidence {
    schema_version: u32,
    experiment: String,
    manifest: PriorManifest,
    groups: Vec<Group>,
}
#[derive(Deserialize)]
struct PriorManifest {
    groups: Vec<PriorGroup>,
}
#[derive(Deserialize)]
struct PriorGroup {
    note: u8,
    takes: Vec<Take>,
}
#[derive(Deserialize, Serialize)]
struct Take {
    id: String,
    file: String,
    git_blob_sha1: String,
}
#[derive(Deserialize)]
struct Group {
    note: u8,
    inputs: Vec<Input>,
}
#[derive(Deserialize)]
struct Input {
    id: String,
    pitch_anchor: Anchor,
    observation_windows: Vec<Window>,
}
#[derive(Deserialize)]
struct Anchor {
    frequency_hz: Option<f64>,
    qualified: bool,
}
#[derive(Deserialize, Serialize)]
struct Window {
    label: String,
    observed_samples: usize,
    capacity_limited: bool,
    accepted_peaks: Vec<Peak>,
}
#[derive(Deserialize, Serialize)]
struct Peak {
    frequency_hz: f64,
    ambiguous_neighbor: bool,
    capacity_limited: bool,
}

fn group(evidence: &Evidence, note: u8) -> Result<(&PriorGroup, &Group), Box<dyn Error>> {
    if evidence.schema_version != 1 || evidence.experiment != "cross-note-spectral-hypotheses-v1" {
        return Err("unsupported source evidence schema/experiment".into());
    }
    let priors: Vec<_> = evidence
        .manifest
        .groups
        .iter()
        .filter(|g| g.note == note)
        .collect();
    let groups: Vec<_> = evidence.groups.iter().filter(|g| g.note == note).collect();
    if priors.len() != 1 || groups.len() != 1 {
        return Err("source evidence needs one matching note group".into());
    }
    let (prior, group) = (priors[0], groups[0]);
    let mut ids = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    if !(3..=8).contains(&prior.takes.len())
        || prior.takes.len() != group.inputs.len()
        || prior.takes.iter().any(|t| {
            t.id.trim().is_empty()
                || t.file.trim().is_empty()
                || !ids.insert(&t.id)
                || !valid_hash(&t.git_blob_sha1)
                || !hashes.insert(t.git_blob_sha1.to_lowercase())
        })
    {
        return Err("invalid, duplicated or incomplete source take group".into());
    }
    let input_ids: BTreeSet<_> = group.inputs.iter().map(|i| &i.id).collect();
    if input_ids != ids || input_ids.len() != group.inputs.len() {
        return Err("source take IDs do not match pinned observations".into());
    }
    Ok((prior, group))
}

struct Selection {
    frequency: Option<f64>,
    excluded_peak: Option<usize>,
    reasons: Vec<&'static str>,
}
fn select(input: &Input, window: &Window, band: Option<[f64; 2]>) -> Selection {
    let mut reasons = Vec::new();
    if window.capacity_limited {
        reasons.push("source_window_capacity_limited");
    }
    let (frequency, excluded_peak) = if let Some([low, high]) = band {
        let peaks: Vec<_> = window
            .accepted_peaks
            .iter()
            .enumerate()
            .filter(|(_, p)| (low..=high).contains(&p.frequency_hz))
            .collect();
        if peaks.len() != 1 {
            reasons.push(if peaks.is_empty() {
                "missing_prior_peak"
            } else {
                "ambiguous_prior_peaks"
            });
            (None, None)
        } else {
            let (index, p) = peaks[0];
            if p.ambiguous_neighbor {
                reasons.push("source_peak_ambiguous");
            }
            if p.capacity_limited {
                reasons.push("source_peak_capacity_limited");
            }
            (Some(p.frequency_hz), Some(index))
        }
    } else if input.pitch_anchor.qualified {
        let frequency = input.pitch_anchor.frequency_hz;
        // The fundamental anchor was measured separately. Associate only a unique
        // attack peak within 8 Hz; retain every other observed peak as a neighbor.
        let matches: Vec<_> = window
            .accepted_peaks
            .iter()
            .enumerate()
            .filter(|(_, p)| frequency.is_some_and(|f| (p.frequency_hz - f).abs() <= 8.0))
            .collect();
        if matches.len() > 1 {
            reasons.push("ambiguous_fundamental_neighbor_association");
        }
        if matches.len() == 1 {
            let peak = matches[0].1;
            if peak.ambiguous_neighbor {
                reasons.push("source_peak_ambiguous");
            }
            if peak.capacity_limited && !window.capacity_limited {
                reasons.push("source_peak_capacity_limited");
            }
        }
        (frequency, (matches.len() == 1).then(|| matches[0].0))
    } else {
        reasons.push("unqualified_pitch_anchor");
        (None, None)
    };
    Selection {
        frequency,
        excluded_peak,
        reasons,
    }
}

fn consensus(
    measurements: &[ComponentEnvelope],
    selection: &Selection,
) -> (Option<f64>, Vec<&'static str>) {
    let mut reasons = Vec::new();
    if !selection.reasons.is_empty() {
        reasons.push("prior_selection_unqualified");
    }
    if measurements.len() != 2 {
        reasons.push("missing_window_measurements");
    }
    if measurements
        .iter()
        .any(|m| !m.qualified || m.provisional_fit.is_none())
    {
        reasons.push("envelope_gate_rejected");
    }
    if !reasons.is_empty() {
        return (None, reasons);
    }
    let a = measurements[0]
        .provisional_fit
        .as_ref()
        .unwrap()
        .amplitude_decay_per_second;
    let b = measurements[1]
        .provisional_fit
        .as_ref()
        .unwrap()
        .amplitude_decay_per_second;
    if (a - b).abs() > 0.2_f64.max(0.1 * a.abs().max(b.abs())) {
        reasons.push("window_rate_disagreement");
        (None, reasons)
    } else {
        (Some((a + b) * 0.5), reasons)
    }
}

fn observe(clip: &AudioClip, input: &Input, m: &Manifest) -> Result<Value, Box<dyn Error>> {
    let windows: Vec<_> = input
        .observation_windows
        .iter()
        .filter(|w| w.label == "attack_128_ms")
        .collect();
    if windows.len() != 1 {
        return Err("source needs exactly one attack-128 observation".into());
    }
    let window = windows[0];
    if window.accepted_peaks.len() > 32
        || window.observed_samples == 0
        || window.observed_samples != (0.128 * clip.metadata().sample_rate as f64).round() as usize
        || window.accepted_peaks.iter().any(|p| {
            !p.frequency_hz.is_finite()
                || p.frequency_hz < 20.0
                || p.frequency_hz >= clip.metadata().sample_rate as f64 / 2.0
        })
        || input
            .pitch_anchor
            .frequency_hz
            .is_some_and(|f| !f.is_finite() || f <= 0.0)
        || (input.pitch_anchor.qualified && input.pitch_anchor.frequency_hz.is_none())
    {
        return Err("invalid source peak window or pitch anchor".into());
    }
    let mut components = Vec::new();
    let mut rates = Vec::new();
    let mut frequencies = Vec::new();
    for (label, band) in [
        ("fundamental", None),
        ("lower_family", Some(m.lower_family_band_hz)),
        ("upper_family", Some(m.upper_family_band_hz)),
    ] {
        let selection = select(input, window, band);
        let neighbors: Vec<_> = window
            .accepted_peaks
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != selection.excluded_peak)
            .map(|(_, p)| p.frequency_hz)
            .collect();
        let mut measurements = Vec::new();
        if let Some(frequency) = selection.frequency {
            for duration in [0.128, 0.256] {
                measurements.push(measure_component_envelope(
                    clip,
                    EnvelopeOptions {
                        frequency_hz: frequency,
                        start_seconds: m.start_seconds,
                        end_seconds: m.end_seconds,
                        window_seconds: duration,
                        hop_seconds: duration / 4.0,
                        known_neighbor_frequencies_hz: neighbors.clone(),
                    },
                )?);
            }
        }
        let (rate, reasons) = consensus(&measurements, &selection);
        rates.push(rate);
        frequencies.push(selection.frequency);
        components.push(json!({"label": label, "selected_frequency_hz": selection.frequency,
            "excluded_prior_peak_index": selection.excluded_peak, "selection_rejections": selection.reasons,
            "known_neighbor_frequencies_hz": neighbors, "measurements": measurements,
            "cross_window_rejections": reasons, "conditional_amplitude_decay_per_second": rate}));
    }
    let frequency_residual = frequencies[2]
        .zip(frequencies[1])
        .zip(frequencies[0])
        .map(|((upper, lower), fundamental)| upper - lower - fundamental);
    let rate_residual = rates[2]
        .zip(rates[1])
        .zip(rates[0])
        .map(|((upper, lower), fundamental)| upper - lower - fundamental);
    Ok(
        json!({"audio": clip.metadata(), "prior_attack_window": window,
        "prior_fundamental_hz": input.pitch_anchor.frequency_hz, "components": components,
        "conditional_weak_mixing_relation": {"frequency_residual_hz": frequency_residual,
            "amplitude_decay_sum_residual_per_second": rate_residual,
            "status": if rate_residual.is_some() {"conditional_observation_only"} else {"withheld_unqualified_or_missing_component"},
            "scope": "Upper minus lower minus fundamental. No fit or causal acceptance threshold. Relation presumes these output components correspond to a weak-mixing pair; no modal identity, displacement phase or natural loss is established."}}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let bytes = read_bounded(Path::new(&args[1]), 65536)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    let evidence = verified_evidence(
        Path::new(&manifest.evidence_file),
        &manifest.evidence_git_blob_sha1,
    )?;
    let (prior, group) = group(&evidence, manifest.note)?;
    let mut takes = Vec::new();
    for take in &prior.takes {
        let input = group
            .inputs
            .iter()
            .find(|i| i.id == take.id)
            .ok_or("missing source observation")?;
        let clip =
            crate::pitch_reference::verified_audio(Path::new(&take.file), &take.git_blob_sha1)?;
        let observation = observe(&clip, input, &manifest)?;
        takes.push(json!({"take": take, "observation": observation}));
        println!("Source envelopes: {}", take.id);
    }
    crate::analysis::write_report(
        output,
        &json!({"schema_version": 1, "experiment": "pinned-source-envelopes-v1",
        "manifest": manifest, "takes": takes,
        "protocol": "Retain every take in the pinned note group. Fundamental uses its qualified prior anchor; families use a unique attack-128 peak in each frozen band. Every other attack peak is a declared neighbor. A unique fundamental attack peak within 8 Hz is excluded as self. Missing/ambiguous/capped selections are retained. No carrier refinement. Both 128/256 ms envelopes must pass unchanged gates, selection must be reliable and rates must agree within max(0.2 /s, 10% of larger absolute rate) to report their descriptive mean. Mixing rate residual withheld unless all three components qualify.",
        "scope": "Processed harp-output recordings with unknown gain, strike speed and note-off. Intervals are file offsets, not guaranteed natural sustain. Fixed prior carrier uncertainty can trigger frequency rejection. Prior peak lists do not prove isolation throughout the interval. Conditional envelope agreement is not mechanical modal identity, calibrated loss, a listening result or proof of pickup mixing. No physical parameter or audio changes. All referenced bytes are verified before decoding; failed selections and gates are data, not command failures."}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(peaks: Vec<f64>) -> Input {
        Input {
            id: "test".into(),
            pitch_anchor: Anchor {
                frequency_hz: Some(196.0),
                qualified: true,
            },
            observation_windows: vec![Window {
                label: "attack_128_ms".into(),
                observed_samples: 6144,
                capacity_limited: false,
                accepted_peaks: peaks
                    .into_iter()
                    .map(|f| Peak {
                        frequency_hz: f,
                        ambiguous_neighbor: false,
                        capacity_limited: false,
                    })
                    .collect(),
            }],
        }
    }
    #[test]
    fn selection_retains_missing_ambiguous_and_capped_evidence() {
        let mut i = input(vec![196.1, 1425.0, 1428.0]);
        let s = select(&i, &i.observation_windows[0], Some([1410.0, 1440.0]));
        assert_eq!(s.frequency, None);
        assert!(s.reasons.contains(&"ambiguous_prior_peaks"));
        assert!(
            select(&i, &i.observation_windows[0], Some([1605.0, 1635.0]))
                .reasons
                .contains(&"missing_prior_peak")
        );
        i.observation_windows[0].accepted_peaks.pop();
        i.observation_windows[0].capacity_limited = true;
        let s = select(&i, &i.observation_windows[0], Some([1410.0, 1440.0]));
        assert_eq!(s.frequency, Some(1425.0));
        assert!(s.reasons.contains(&"source_window_capacity_limited"));
        let s = select(&i, &i.observation_windows[0], None);
        assert_eq!(s.excluded_peak, Some(0));
        assert_eq!(s.frequency, Some(196.0));
    }
    #[test]
    fn manifest_rejects_invalid_bounds_hashes_and_unknown_fields() {
        let source = include_str!("../../../references/source-envelope.manifest.json");
        let m: Manifest = serde_json::from_str(source).unwrap();
        m.validate().unwrap();
        for (key, value) in [
            ("note", json!(5)),
            ("end_seconds", json!(0.2)),
            ("lower_family_band_hz", json!([1410.0, 1600.0])),
            ("evidence_git_blob_sha1", json!("bad")),
            ("selection", json!("")),
        ] {
            let mut v: Value = serde_json::from_str(source).unwrap();
            v[key] = value;
            assert!(
                serde_json::from_value::<Manifest>(v)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let mut v: Value = serde_json::from_str(source).unwrap();
        v["unknown"] = json!(1);
        assert!(serde_json::from_value::<Manifest>(v).is_err());
    }

    #[test]
    fn consensus_withholds_failed_selection_window_and_disagreeing_rates() {
        let clip = AudioClip::from_samples(
            48000,
            (0..96000)
                .map(|i| {
                    let t = i as f64 / 48000.0;
                    0.2 * (-3.0 * t).exp() * (std::f64::consts::TAU * 1425.0 * t).cos()
                })
                .collect(),
        )
        .unwrap();
        let mut measurements: Vec<_> = [0.128, 0.256]
            .into_iter()
            .map(|w| {
                measure_component_envelope(
                    &clip,
                    EnvelopeOptions {
                        frequency_hz: 1425.0,
                        start_seconds: 0.1,
                        end_seconds: 1.5,
                        window_seconds: w,
                        hop_seconds: w / 4.0,
                        known_neighbor_frequencies_hz: vec![],
                    },
                )
                .unwrap()
            })
            .collect();
        let mut selection = Selection {
            frequency: Some(1425.0),
            excluded_peak: Some(0),
            reasons: vec![],
        };
        assert!((consensus(&measurements, &selection).0.unwrap() - 3.0).abs() < 0.001);
        assert!(consensus(&measurements[..1], &selection).0.is_none());
        selection.reasons.push("source_window_capacity_limited");
        assert!(consensus(&measurements, &selection).0.is_none());
        selection.reasons.clear();
        measurements[1].qualified = false;
        assert!(consensus(&measurements, &selection).0.is_none());
        measurements[1].qualified = true;
        measurements[1]
            .provisional_fit
            .as_mut()
            .unwrap()
            .amplitude_decay_per_second = 5.0;
        let result = consensus(&measurements, &selection);
        assert!(result.0.is_none());
        assert!(result.1.contains(&"window_rate_disagreement"));
    }

    #[test]
    fn evidence_requires_unique_matching_ids_notes_and_content() {
        let make = || {
            serde_json::from_value::<Evidence>(json!({"schema_version": 1,
                "experiment": "cross-note-spectral-hypotheses-v1",
                "manifest": {"groups": [{"note":55,"takes":(0..3).map(|i|json!({"id":format!("t{i}"),"file":format!("{i}.wav"),"git_blob_sha1":format!("{i:040x}")})).collect::<Vec<_>>()}]},
                "groups":[{"note":55,"inputs":(0..3).map(|i|json!({"id":format!("t{i}"),"pitch_anchor":{"frequency_hz":196.0,"qualified":true},"observation_windows":[]})).collect::<Vec<_>>()}]
            })).unwrap()
        };
        assert!(group(&make(), 55).is_ok());
        assert!(group(&make(), 59).is_err());
        let mut e = make();
        e.groups[0].inputs[0].id = "t1".into();
        assert!(group(&e, 55).is_err());
        let mut e = make();
        e.manifest.groups[0].takes[0].git_blob_sha1 =
            e.manifest.groups[0].takes[1].git_blob_sha1.clone();
        assert!(group(&e, 55).is_err());
        let mut e = make();
        e.schema_version = 2;
        assert!(group(&e, 55).is_err());
    }
}
