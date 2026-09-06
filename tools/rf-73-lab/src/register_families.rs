//! Cross-note descriptive hypotheses, with qualified per-take pitch anchors.
use rf_73_analysis::{observe_modes, pitch_anchor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};

pub const HELP: &str = "Cross-note spectral hypotheses:
  observe-register MANIFEST.json --output REPORT.json
Strict schema-1 manifest: 2..5 distinct notes, 3..8 pinned takes per note.
Measure per-take pitch, retain all source windows and recurrence limits.
Compare reference-note families under fixed-Hz and constant-ratio predictions.
Report absent/ambiguous correspondences; no modal identity, geometry or gain fit.
";
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Take {
    id: String,
    file: String,
    git_blob_sha1: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Group {
    note: u8,
    takes: Vec<Take>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    reference_note: u8,
    provenance: String,
    selection: String,
    groups: Vec<Group>,
}
impl Manifest {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        let (mut notes, mut ids, mut hashes) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
        if self.schema_version != 1
            || !(2..=5).contains(&self.groups.len())
            || self.provenance.trim().is_empty()
            || self.selection.trim().is_empty()
            || self.groups.iter().any(|g| {
                !(40..=90).contains(&g.note)
                    || !notes.insert(g.note)
                    || !(3..=8).contains(&g.takes.len())
                    || g.takes.iter().any(|t| {
                        t.id.trim().is_empty()
                            || t.file.trim().is_empty()
                            || !ids.insert(&t.id)
                            || t.git_blob_sha1.len() != 40
                            || !t.git_blob_sha1.bytes().all(|b| b.is_ascii_hexdigit())
                            || !hashes.insert(t.git_blob_sha1.to_lowercase())
                    })
            })
            || !notes.contains(&self.reference_note)
        {
            return Err(
                "invalid register manifest, duplicate note/content or missing reference/provenance"
                    .into(),
            );
        }
        Ok(())
    }
}

fn center(f: &Value) -> f64 {
    let reliable: Vec<_> = f["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| {
            m["capacity_limited"] == false
                && m["detector_ambiguous"] == false
                && m["harmonic_overlap"] == false
        })
        .collect();
    reliable
        .iter()
        .map(|m| m["frequency_hz"].as_f64().unwrap())
        .sum::<f64>()
        / reliable.len() as f64
}
fn correspondences(reference: &Value, other: &Value, scale: f64) -> Vec<Value> {
    let a: Vec<_> = reference["families"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, f)| f["status"] == "recurrent_nonharmonic_candidate")
        .collect();
    let b: Vec<_> = other["families"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, f)| f["status"] == "recurrent_nonharmonic_candidate")
        .collect();
    let tolerance = reference["association_tolerance_hz"].as_f64().unwrap() * scale
        + other["association_tolerance_hz"].as_f64().unwrap();
    let candidates: Vec<Vec<_>> = a
        .iter()
        .map(|(_, x)| {
            b.iter()
                .filter(|(_, y)| (center(y) - center(x) * scale).abs() <= tolerance)
                .map(|(j, y)| (*j, center(y)))
                .collect()
        })
        .collect();
    let mut reverse = BTreeMap::new();
    for list in &candidates {
        for (j, _) in list {
            *reverse.entry(*j).or_insert(0usize) += 1;
        }
    }
    a.iter().zip(candidates).map(|((i,x),list)| {
        let prediction=center(x)*scale;
        let status=if list.is_empty() {"no_recurrent_candidate_under_rule"} else if list.len()>1 || reverse[&list[0].0]>1 {"ambiguous_frequency_correspondence"} else {"single_frequency_correspondence"};
        json!({"reference_family_index":i,"reference_reliable_center_hz":center(x),"predicted_frequency_hz":prediction,"tolerance_hz":tolerance,"status":status,
            "candidates":list.iter().map(|(j,f)|json!({"family_index":j,"reliable_center_hz":f,"residual_hz":f-prediction,"shared_by_reference_families":reverse[j]})).collect::<Vec<_>>()})
    }).collect()
}
fn compare(groups: &[Value], reference_note: u8) -> Vec<Value> {
    let reference = groups.iter().find(|g| g["note"] == reference_note).unwrap();
    groups.iter().filter(|g|g["note"]!=reference_note).map(|g| {
        if reference["all_pitch_anchors_qualified"]!=true || g["all_pitch_anchors_qualified"]!=true {
            return json!({"note":g["note"],"status":"withheld_unqualified_pitch","windows":[]});
        }
        let scale=g["geometric_mean_fundamental_hz"].as_f64().unwrap()/reference["geometric_mean_fundamental_hz"].as_f64().unwrap();
        json!({"note":g["note"],"status":"descriptive_hypotheses_only","fundamental_ratio":scale,
            "windows":(0..3).map(|w|json!({"label":reference["windows"][w]["label"],
                "fixed_hz":correspondences(&reference["windows"][w],&g["windows"][w],1.0),
                "constant_ratio":correspondences(&reference["windows"][w],&g["windows"][w],scale)})).collect::<Vec<_>>()})
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
        return Err("register manifest exceeds 64 KiB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    let mut groups = Vec::new();
    for g in &manifest.groups {
        let (mut observations, mut fundamentals, mut inputs) = (Vec::new(), Vec::new(), Vec::new());
        for t in &g.takes {
            let clip =
                crate::pitch_reference::verified_audio(Path::new(&t.file), &t.git_blob_sha1)?;
            let anchor = pitch_anchor(&clip, g.note)?;
            let observation = if anchor.qualified {
                let f = anchor
                    .frequency_hz
                    .ok_or("qualified anchor missing frequency")?;
                fundamentals.push(f);
                let o = observe_modes(&clip, &[f], f)?;
                let row=json!(o.windows.iter().map(|w|json!({"label":w.label,"start_frame":w.start_frame,"observed_samples":w.spectrum.observed_samples,
                    "minimum_separation_hz":w.minimum_separation_hz,"capacity_limited":w.capacity_limited,"detector":w.detector,"accepted_peaks":w.accepted_peaks})).collect::<Vec<_>>());
                observations.push(o);
                row
            } else {
                Value::Null
            };
            inputs.push(json!({"id":t.id,"audio":clip.metadata(),"pitch_anchor":anchor,"observation_windows":observation}));
        }
        let qualified = fundamentals.len() == g.takes.len();
        let windows = if qualified {
            json!(crate::modal_families::summarize(
                &observations,
                &fundamentals
            ))
        } else {
            Value::Null
        };
        let mean = qualified.then(|| {
            (fundamentals.iter().map(|f| f.ln()).sum::<f64>() / fundamentals.len() as f64).exp()
        });
        println!("Note {}: all pitch anchors qualified={qualified}", g.note);
        groups.push(json!({"note":g.note,"all_pitch_anchors_qualified":qualified,"geometric_mean_fundamental_hz":mean,"inputs":inputs,"windows":windows}));
    }
    let qualified = groups
        .iter()
        .all(|g| g["all_pitch_anchors_qualified"] == true);
    let comparisons = compare(&groups, manifest.reference_note);
    crate::analysis::write_report(
        output,
        &json!({"schema_version":1,"experiment":"cross-note-spectral-hypotheses-v1","manifest":manifest,"all_pitch_anchors_qualified":qualified,
        "groups":groups,"comparisons":comparisons,
        "protocol":"Per-take qualified three-window pitch anchors; unchanged independent family detector and recurrence gates. Fixed-Hz and constant-ratio predictions use means of reliable family members and geometric means of all qualified per-note fundamentals. Frequency tolerances add both reciprocal durations, scaling the reference for the ratio hypothesis. Multiple and shared candidates are ambiguous. Failed pitch qualification withholds the entire group's family inference, never silently drops takes.",
        "scope":"Descriptive cross-note probes on one processed instrument, not mode identity, a linear modal scaling law, independent-instrument validation, calibrated gain or a geometry fit. Nonmatches may reflect finite windows, detector capacity, nonlinear pickup, processing or real inharmonic geometry. Constant-ratio failure cannot rule out a mechanical family; fixed-Hz recurrence cannot prove electrical hum. Anchor uncertainty is not a statistical confidence interval. All source takes and window limitations are retained."}),
    )?;
    if !qualified {
        return Err("register report retained unqualified pitch anchors; comparisons withheld where necessary".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(freqs: &[f64]) -> Value {
        json!({"association_tolerance_hz":1.0,"families":freqs.iter().map(|f|json!({"status":"recurrent_nonharmonic_candidate","members":(0..3).map(|_|json!({"frequency_hz":f,"capacity_limited":false,"detector_ambiguous":false,"harmonic_overlap":false})).collect::<Vec<_>>()})).collect::<Vec<_>>()})
    }
    #[test]
    fn fixed_and_scaled_hypotheses_remain_separate_and_missing_is_explicit() {
        let a = window(&[100.0, 1400.0]);
        let b = window(&[100.0, 2100.0]);
        let fixed = correspondences(&a, &b, 1.0);
        let scaled = correspondences(&a, &b, 1.5);
        assert_eq!(fixed[0]["status"], "single_frequency_correspondence");
        assert_eq!(fixed[1]["status"], "no_recurrent_candidate_under_rule");
        assert_eq!(scaled[0]["status"], "no_recurrent_candidate_under_rule");
        assert_eq!(scaled[1]["status"], "single_frequency_correspondence");
    }
    #[test]
    fn shared_matches_are_ambiguous_and_unqualified_groups_are_withheld() {
        let a = window(&[100.0, 103.0]);
        let b = window(&[101.5]);
        assert!(
            correspondences(&a, &b, 1.0)
                .iter()
                .all(|r| r["status"] == "ambiguous_frequency_correspondence")
        );
        let groups = [
            json!({"note":55,"all_pitch_anchors_qualified":true}),
            json!({"note":59,"all_pitch_anchors_qualified":false}),
        ];
        assert_eq!(
            compare(&groups, 55)[0]["status"],
            "withheld_unqualified_pitch"
        );
    }
    #[test]
    fn strict_register_manifest_rejects_duplicate_notes_content_and_missing_reference() {
        let text = include_str!("../../../references/register-families.manifest.json");
        let mut m: Manifest = serde_json::from_str(text).unwrap();
        m.validate().unwrap();
        m.groups[0].note = m.groups[1].note;
        assert!(m.validate().is_err());
        let mut m: Manifest = serde_json::from_str(text).unwrap();
        m.groups[0].takes[0].git_blob_sha1 = m.groups[1].takes[0].git_blob_sha1.clone();
        assert!(m.validate().is_err());
        let mut m: Manifest = serde_json::from_str(text).unwrap();
        m.reference_note = 60;
        assert!(m.validate().is_err());
    }
}
