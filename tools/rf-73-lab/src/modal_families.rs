//! Cross-take frequency recurrence, independent of proposed structural frequencies.
use rf_73_analysis::{ModalObservation, observe_modes};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, error::Error, fs::File, io::Read, path::Path};

pub const HELP: &str = "Cross-take spectral families:
  observe-families MANIFEST.json --output REPORT.json
Strict schema-1 manifest, one MIDI note, 3..8 distinct pinned reference WAVs.
Native 32/128 ms attack and 512 ms body windows; no proposed model frequencies.
Retains every peak, harmonic/capacity limits, chained and duplicate ambiguities.
Recurrence and low-order frequency relations do not identify mechanical modes.
";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Take {
    id: String,
    file: String,
    git_blob_sha1: String,
    fundamental_hz: f64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    note: u8,
    provenance: String,
    prior_exposure: String,
    takes: Vec<Take>,
}
impl Manifest {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        let mut ids = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        let nominal = 440.0 * 2.0_f64.powf((self.note as f64 - 69.0) / 12.0);
        if self.schema_version != 1
            || !(28..=100).contains(&self.note)
            || !(3..=8).contains(&self.takes.len())
            || self.provenance.trim().is_empty()
            || self.prior_exposure.trim().is_empty()
            || self.takes.iter().any(|t| {
                t.id.trim().is_empty()
                    || t.file.trim().is_empty()
                    || !ids.insert(&t.id)
                    || t.git_blob_sha1.len() != 40
                    || !t.git_blob_sha1.bytes().all(|b| b.is_ascii_hexdigit())
                    || !hashes.insert(t.git_blob_sha1.to_lowercase())
                    || !t.fundamental_hz.is_finite()
                    || t.fundamental_hz <= 0.0
                    || (1200.0 * (t.fundamental_hz / nominal).log2()).abs() > 100.0
            })
        {
            return Err(
                "invalid family manifest, note anchors, duplicate input or missing provenance"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Serialize)]
struct Member {
    take_index: usize,
    peak_index: usize,
    frequency_hz: f64,
    nearest_harmonic: u32,
    harmonic_distance_hz: f64,
    harmonic_overlap: bool,
    detector_ambiguous: bool,
    capacity_limited: bool,
}
#[derive(Serialize)]
struct Family {
    frequency_range_hz: [f64; 2],
    mean_frequency_hz: f64,
    chained_or_duplicate: bool,
    reliable_take_count: usize,
    isolated_take_count: usize,
    status: &'static str,
    members: Vec<Member>,
}

fn group(mut peaks: Vec<Member>, tolerance: f64) -> Vec<Family> {
    peaks.sort_by(|a, b| {
        a.frequency_hz
            .total_cmp(&b.frequency_hz)
            .then(a.take_index.cmp(&b.take_index))
    });
    let mut groups: Vec<Vec<Member>> = Vec::new();
    for p in peaks {
        if groups
            .last()
            .is_none_or(|g| p.frequency_hz - g.last().unwrap().frequency_hz > tolerance)
        {
            groups.push(Vec::new());
        }
        groups.last_mut().unwrap().push(p);
    }
    groups
        .into_iter()
        .map(|members| {
            let range = [
                members[0].frequency_hz,
                members.last().unwrap().frequency_hz,
            ];
            let mut takes = BTreeSet::new();
            let duplicate = members.iter().any(|p| !takes.insert(p.take_index));
            let ambiguous = duplicate || range[1] - range[0] > tolerance;
            let reliable: BTreeSet<_> = members
                .iter()
                .filter(|m| !m.detector_ambiguous && !m.capacity_limited)
                .map(|m| m.take_index)
                .collect();
            let isolated: BTreeSet<_> = members
                .iter()
                .filter(|m| !m.detector_ambiguous && !m.capacity_limited && !m.harmonic_overlap)
                .map(|m| m.take_index)
                .collect();
            Family {
                frequency_range_hz: range,
                mean_frequency_hz: members.iter().map(|m| m.frequency_hz).sum::<f64>()
                    / members.len() as f64,
                chained_or_duplicate: ambiguous,
                reliable_take_count: reliable.len(),
                isolated_take_count: isolated.len(),
                status: if ambiguous {
                    "ambiguous_component"
                } else if isolated.len() >= 3 {
                    "recurrent_nonharmonic_candidate"
                } else if reliable.len() >= 3 {
                    "recurrent_with_harmonic_overlap"
                } else {
                    "insufficient_reliable_recurrence"
                },
                members,
            }
        })
        .collect()
}

// Check relations in the same individual takes, not just rounded family averages.
// A relation is descriptive and cannot distinguish nonlinear mixing from coincidence.
fn relations(families: &[Family], fundamentals: &[f64], tolerance: f64) -> Vec<Value> {
    let mut rows = Vec::new();
    for (i, a) in families
        .iter()
        .enumerate()
        .filter(|(_, a)| a.status == "recurrent_nonharmonic_candidate")
    {
        for (j, b) in families
            .iter()
            .enumerate()
            .skip(i + 1)
            .filter(|(_, b)| b.status == "recurrent_nonharmonic_candidate")
        {
            for (kind, n) in [
                ("fundamental_offset", 1),
                ("fundamental_offset", 2),
                ("fundamental_offset", 3),
                ("fundamental_offset", 4),
                ("frequency_multiple", 2),
                ("frequency_multiple", 3),
            ] {
                let mut support = Vec::new();
                for x in a
                    .members
                    .iter()
                    .filter(|m| !m.detector_ambiguous && !m.capacity_limited && !m.harmonic_overlap)
                {
                    if let Some(y) = b.members.iter().find(|m| {
                        m.take_index == x.take_index
                            && !m.detector_ambiguous
                            && !m.capacity_limited
                            && !m.harmonic_overlap
                    }) {
                        let residual = if kind == "fundamental_offset" {
                            y.frequency_hz - x.frequency_hz - n as f64 * fundamentals[x.take_index]
                        } else {
                            y.frequency_hz - n as f64 * x.frequency_hz
                        };
                        // Propagate both observation tolerances, including the multiplier.
                        let limit = if kind == "fundamental_offset" {
                            2.0 * tolerance
                        } else {
                            (n + 1) as f64 * tolerance
                        };
                        if residual.abs() <= limit {
                            support.push(json!({"take_index":x.take_index,"residual_hz":residual,"tolerance_hz":limit}));
                        }
                    }
                }
                if support.len() >= 3 {
                    rows.push(json!({"lower_family_index":i,"upper_family_index":j,"kind":kind,"order":n,"support":support}));
                }
            }
        }
    }
    rows
}

fn summarize(observations: &[ModalObservation], fundamentals: &[f64]) -> Vec<Value> {
    (0..3).map(|w| {
        // Conservative common association width across rounded native-rate windows.
        let tolerance=observations.iter().map(|o|o.windows[w].minimum_separation_hz/2.0).fold(0.0_f64,f64::max);
        let mut peaks=Vec::new();
        for (take_index,o) in observations.iter().enumerate() {
            let window=&o.windows[w];
            for (peak_index,p) in window.accepted_peaks.iter().enumerate() {
                let harmonic=(p.frequency_hz/o.fundamental_hz).round().max(1.0) as u32;
                let distance=(p.frequency_hz-harmonic as f64*o.fundamental_hz).abs();
                peaks.push(Member {take_index,peak_index,frequency_hz:p.frequency_hz,nearest_harmonic:harmonic,
                    harmonic_distance_hz:distance,harmonic_overlap:distance<window.minimum_separation_hz,
                    detector_ambiguous:p.ambiguous_neighbor,capacity_limited:window.capacity_limited || p.capacity_limited});
            }
        }
        let families=group(peaks,tolerance);
        let relations=relations(&families,fundamentals,tolerance);
        let families: Vec<_>=families.iter().map(|f| {
            let mut row=serde_json::to_value(f).expect("finite detected family");
            row["input_evidence"]=json!(observations.iter().enumerate().map(|(i,o)| {
                let window=&o.windows[w];
                let members: Vec<_>=f.members.iter().filter(|m|m.take_index==i).collect();
                let status=if window.capacity_limited {"capacity_limited"}
                    else if f.frequency_range_hz[0]-tolerance<window.detection_band_hz[0] || f.frequency_range_hz[1]+tolerance>window.detection_band_hz[1] {"outside_complete_search_band"}
                    else if members.is_empty() {"no_accepted_peak_in_component"}
                    else if f.chained_or_duplicate || members.iter().any(|m|m.detector_ambiguous) {"ambiguous_component"}
                    else if members.iter().any(|m|m.harmonic_overlap) {"harmonic_overlap"}
                    else {"isolated_frequency_candidate"};
                json!({"take_index":i,"status":status})
            }).collect::<Vec<_>>());
            row
        }).collect();
        json!({"label":observations[0].windows[w].label,"association_tolerance_hz":tolerance,
            "families":families,"low_order_frequency_relations":relations,
            "input_window_limits":observations.iter().enumerate().map(|(i,o)|json!({"take_index":i,
                "capacity_limited":o.windows[w].capacity_limited,"detection_band_hz":o.windows[w].detection_band_hz})).collect::<Vec<_>>()})
    }).collect()
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let mut bytes = Vec::new();
    File::open(&args[1])?.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("family manifest exceeds 64 KiB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    let mut observations = Vec::new();
    let mut inputs = Vec::new();
    for t in &manifest.takes {
        let clip = crate::pitch_reference::verified_audio(Path::new(&t.file), &t.git_blob_sha1)?;
        // Target is used only by the legacy observation wrapper; families use all
        // independently detected peaks, never the proposed-mode associations.
        let o = observe_modes(&clip, &[t.fundamental_hz], t.fundamental_hz)?;
        inputs.push(json!({"id":t.id,"audio":clip.metadata(),"windows":o.windows.iter().map(|w|json!({
            "label":w.label,"start_frame":w.start_frame,"minimum_separation_hz":w.minimum_separation_hz,
            "capacity_limited":w.capacity_limited,"detector":w.detector,"accepted_peaks":w.accepted_peaks})).collect::<Vec<_>>()}));
        observations.push(o);
    }
    let windows = summarize(
        &observations,
        &manifest
            .takes
            .iter()
            .map(|t| t.fundamental_hz)
            .collect::<Vec<_>>(),
    );
    crate::analysis::write_report(
        output,
        &json!({"schema_version":1,"experiment":"cross-take-spectral-families-v1",
        "manifest":manifest,"inputs":inputs,"windows":windows,
        "protocol":"Independent native-rate peaks; one reciprocal duration association width, connected frequency components. Reject transitive chains wider than that width or duplicate takes as ambiguous; at least three distinct uncapped unambiguous nonharmonic detections for recurrence. Capacity-limited inputs remain explicit and never count as reliable evidence. Relations require support within the same three or more takes. No structural proposals or geometry fit.",
        "scope":"Exploratory single-note recurrence, not modal identification, independent validation, natural decay, calibrated amplitude or cross-note confirmation. Integer harmonics and selected low-order relations do not enumerate all nonlinear mixtures; frequency coincidence does not prove mixing. Non-detection is not physical absence; missing and capped takes remain visible."}),
    )?;
    println!("Spectral family report: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_rate_windows_preserve_resolution_limits_despite_different_gains() {
        let mut observations = Vec::new();
        for (rate, gain) in [(44100, 1.0), (48000, 0.5), (96000, 0.2)] {
            let samples = (0..rate)
                .map(|i| {
                    let t = i as f64 / rate as f64;
                    gain * (0.2 * (std::f64::consts::TAU * 200.0 * t).sin()
                        + 0.04 * (std::f64::consts::TAU * 1353.0 * t).sin()
                        + 0.02 * (std::f64::consts::TAU * 3207.0 * t).sin())
                })
                .collect();
            let clip = rf_73_analysis::AudioClip::from_samples(rate, samples).unwrap();
            observations.push(observe_modes(&clip, &[200.0], 200.0).unwrap());
        }
        let rows = summarize(&observations, &[200.0; 3]);
        for window in &rows[1..] {
            for expected in [1353.0, 3207.0] {
                // 3207 is only 7 Hz from H16: unresolved at 128 ms (15.625 Hz),
                // separated at 512 ms (3.90625 Hz). Detection alone is insufficient.
                let status = if expected == 3207.0 && window["label"] == "attack_128_ms" {
                    "recurrent_with_harmonic_overlap"
                } else {
                    "recurrent_nonharmonic_candidate"
                };
                assert!(
                    window["families"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["status"] == status
                            && (f["mean_frequency_hz"].as_f64().unwrap() - expected).abs() < 0.2),
                    "{window}"
                );
            }
        }
    }
    fn member(take: usize, f: f64) -> Member {
        Member {
            take_index: take,
            peak_index: 0,
            frequency_hz: f,
            nearest_harmonic: 7,
            harmonic_distance_hz: 50.0,
            harmonic_overlap: false,
            detector_ambiguous: false,
            capacity_limited: false,
        }
    }
    #[test]
    fn recurrence_requires_distinct_reliable_inputs_and_rejects_transitive_chains() {
        assert_eq!(
            group(
                vec![member(0, 1425.0), member(1, 1425.2), member(2, 1425.5)],
                8.0
            )[0]
            .status,
            "recurrent_nonharmonic_candidate"
        );
        for peaks in [
            vec![member(0, 1400.0), member(1, 1407.0), member(2, 1414.0)],
            vec![member(0, 1425.0), member(0, 1426.0), member(2, 1425.5)],
        ] {
            assert!(group(peaks, 8.0)[0].chained_or_duplicate);
        }
        let mut peaks = vec![member(0, 1425.0), member(1, 1425.2), member(2, 1425.5)];
        peaks[2].capacity_limited = true;
        assert_eq!(group(peaks.clone(), 8.0)[0].isolated_take_count, 2);
        peaks[2].capacity_limited = false;
        peaks[2].harmonic_overlap = true;
        assert_eq!(
            group(peaks, 8.0)[0].status,
            "recurrent_with_harmonic_overlap"
        );
        assert!(group(Vec::new(), 8.0).is_empty());
    }
    #[test]
    fn relations_require_same_take_support_and_report_frequency_mixing_ambiguity() {
        let mut peaks = Vec::new();
        for i in 0..3 {
            peaks.push(member(i, 1425.0));
            peaks.push(member(i, 1625.0));
            peaks.push(member(i, 2850.0));
        }
        let families = group(peaks, 8.0);
        let r = relations(&families, &[200.0; 3], 8.0);
        assert!(
            r.iter()
                .any(|r| r["kind"] == "fundamental_offset" && r["order"] == 1)
        );
        assert!(
            r.iter()
                .any(|r| r["kind"] == "frequency_multiple" && r["order"] == 2)
        );
        let mut split = vec![member(0, 1425.0), member(1, 1425.0), member(2, 1425.0)];
        split.extend([member(3, 1625.0), member(4, 1625.0), member(5, 1625.0)]);
        assert!(relations(&group(split, 8.0), &[200.0; 6], 8.0).is_empty());
    }
    #[test]
    fn manifest_rejects_duplicate_bytes_unknown_fields_and_mixed_notes() {
        let text = include_str!("../../../references/g3-spectral-families.manifest.json");
        let mut m: Manifest = serde_json::from_str(text).unwrap();
        m.validate().unwrap();
        m.takes[1].git_blob_sha1 = m.takes[0].git_blob_sha1.clone();
        assert!(m.validate().is_err());
        let mut m: Manifest = serde_json::from_str(text).unwrap();
        m.takes[1].fundamental_hz *= 2.0;
        assert!(m.validate().is_err());
        let mut v: Value = serde_json::from_str(text).unwrap();
        v["unexpected"] = json!(true);
        assert!(serde_json::from_value::<Manifest>(v).is_err());
    }
}
