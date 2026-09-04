//! Shared-geometry fit with a frozen gain and separately evaluated held-out takes.
use super::pickup_sweep::{Metrics, grid, render, score_with_gain};
use rf_rhodes_analysis::AudioClip;
use rf_rhodes_dsp::{FIRST_NOTE, LAST_NOTE, Profile};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fs::File, io::Read, path::Path};

pub const HELP: &str = "Pickup reference set:
  fit-pickup-set MANIFEST.json --output REPORT.json
Strict schema-1 manifest: 3..8 takes, at least two distinct fit note/velocity pairs
and one held-out pair. One capture gain shared by the entire reference set.
At most 25 geometries. Fit takes alone select geometry AND gain; held-out takes
evaluate only that frozen choice. No automatic alignment or plugin modification.
See docs/PICKUP-SET.md for manifest fields, bounds and interpretation.
";

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Fit,
    Validation,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Take {
    id: String,
    file: String,
    role: Role,
    note: u8,
    velocity: f64,
    velocity_basis: String,
    reference_start_seconds: f64,
    model_start_seconds: f64,
    seconds: f64,
    sustain_end_seconds: f64,
    channel: Option<u16>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    source: String,
    source_revision: String,
    license: String,
    instrument: String,
    processing: String,
    capture_gain: String,
    gaps_mm: Vec<f64>,
    offsets_mm: Vec<f64>,
    takes: Vec<Take>,
}

struct Prepared {
    audio: AudioClip,
    model_start: usize,
    receipt: serde_json::Value,
}

#[derive(Serialize)]
struct TakeScore {
    take_id: String,
    metrics: Metrics,
}

#[derive(Serialize)]
struct Fit {
    objective_db: f64,
    applied_gain: f64,
    takes: Vec<TakeScore>,
}

#[derive(Serialize)]
struct Candidate {
    gap_mm: f64,
    offset_mm: f64,
    fit: Option<Fit>,
    rejection_reason: Option<String>,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty()
}

fn validate(m: &Manifest) -> Result<(), Box<dyn Error>> {
    if m.schema_version != 1
        || !(3..=8).contains(&m.takes.len())
        || [
            &m.source,
            &m.source_revision,
            &m.license,
            &m.instrument,
            &m.processing,
            &m.capture_gain,
        ]
        .iter()
        .any(|s| !nonempty(s))
    {
        return Err("schema 1 requires 3..8 takes and explicit provenance/capture fields".into());
    }
    for (values, low, high) in [(&m.gaps_mm, 0.5, 5.0), (&m.offsets_mm, -3.0, 3.0)] {
        grid(
            &values
                .iter()
                .map(f64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            low,
            high,
        )?;
    }
    let mut ids = BTreeSet::new();
    let mut fit_pairs = BTreeSet::new();
    for t in &m.takes {
        if !nonempty(&t.id)
            || !nonempty(&t.file)
            || !nonempty(&t.velocity_basis)
            || !ids.insert(&t.id)
            || !(FIRST_NOTE..=LAST_NOTE).contains(&t.note)
            || !t.velocity.is_finite()
            || !(0.01..=1.0).contains(&t.velocity)
            || !t.seconds.is_finite()
            || !(0.128..=4.0).contains(&t.seconds)
            || !t.reference_start_seconds.is_finite()
            || t.reference_start_seconds < 0.0
            || !t.model_start_seconds.is_finite()
            || !(0.0..=2.0).contains(&t.model_start_seconds)
            || !t.sustain_end_seconds.is_finite()
            || t.reference_start_seconds + t.seconds > t.sustain_end_seconds
        {
            return Err(format!("invalid take or held-note region: {}", t.id).into());
        }
        if t.role == Role::Fit {
            fit_pairs.insert((t.note, t.velocity.to_bits()));
        }
    }
    if fit_pairs.len() < 2 || !m.takes.iter().any(|t| t.role == Role::Validation) {
        return Err(
            "need two distinct fit note/velocity pairs and at least one validation take".into(),
        );
    }
    if m.takes
        .iter()
        .any(|t| t.role == Role::Validation && fit_pairs.contains(&(t.note, t.velocity.to_bits())))
    {
        return Err("validation note/velocity pairs must be held out from fitting".into());
    }
    Ok(())
}

fn energy(audio: &AudioClip) -> f64 {
    audio.samples().iter().map(|s| s * s).sum::<f64>() / audio.samples().len() as f64
}

fn prepare(m: &Manifest, root: &Path) -> Result<Vec<Prepared>, Box<dyn Error>> {
    let mut prepared = Vec::new();
    let mut paths = BTreeSet::new();
    let mut rate = None;
    for t in &m.takes {
        let path = root.join(&t.file).canonicalize()?;
        // Reject reuse even for different channels/regions: these are separate takes.
        let key = path.to_string_lossy().into_owned();
        let key = if cfg!(windows) {
            key.to_lowercase()
        } else {
            key
        };
        if !paths.insert(key) {
            return Err("each take must use a distinct reference file".into());
        }
        let input = AudioClip::open(&path, t.channel)?;
        let hz = input.metadata().sample_rate;
        if ![44100, 48000, 96000, 192000].contains(&hz) || rate.is_some_and(|r| r != hz) {
            return Err("all takes need the same supported model sample rate".into());
        }
        rate = Some(hz);
        if t.sustain_end_seconds > input.duration() {
            return Err(format!("sustain boundary exceeds recording: {}", t.id).into());
        }
        let start = (t.reference_start_seconds * hz as f64).round() as usize;
        let frames = (t.seconds * hz as f64).round() as usize;
        let end = start.checked_add(frames).ok_or("region overflow")?;
        if end > (t.sustain_end_seconds * hz as f64).round() as usize {
            return Err("rounded region crosses sustain boundary".into());
        }
        let samples = input
            .samples()
            .get(start..end)
            .ok_or("region exceeds recording")?;
        let audio = AudioClip::from_samples(hz, samples.to_vec())?;
        if energy(&audio) <= 1e-24 {
            return Err(format!("silent reference: {}", t.id).into());
        }
        let model_start = (t.model_start_seconds * hz as f64).round() as usize;
        prepared.push(Prepared { audio, model_start, receipt: serde_json::json!({
            "take_id": t.id, "resolved_file": path, "audio": input.metadata(),
            "reference_start_frame": start, "model_start_frame": model_start, "compared_frames": frames,
        }) });
    }
    Ok(prepared)
}

fn candidate_audio(t: &Take, p: &Prepared, profile: Profile) -> Result<AudioClip, Box<dyn Error>> {
    render(
        p.audio.metadata().sample_rate,
        p.audio.samples().len(),
        p.model_start,
        t.note,
        t.velocity,
        profile,
    )
}

fn aggregate(scores: &[TakeScore]) -> f64 {
    (scores
        .iter()
        .map(|s| s.metrics.objective_db.powi(2))
        .sum::<f64>()
        / scores.len() as f64)
        .sqrt()
}

fn fit(m: &Manifest, prepared: &[Prepared], profile: Profile) -> Result<Fit, Box<dyn Error>> {
    let mut audio = Vec::new();
    let (mut reference_energy, mut candidate_energy) = (0.0, 0.0);
    for (t, p) in m
        .takes
        .iter()
        .zip(prepared)
        .filter(|(t, _)| t.role == Role::Fit)
    {
        let candidate = candidate_audio(t, p, profile)?;
        if energy(&candidate) <= 1e-24 {
            return Err(format!("silent candidate: {}", t.id).into());
        }
        reference_energy += energy(&p.audio);
        candidate_energy += energy(&candidate);
        audio.push((t, p, candidate));
    }
    // Equal take weighting, independent of region length; no held-out energy enters.
    let gain = (reference_energy / candidate_energy).sqrt();
    let mut takes = Vec::new();
    for (t, p, candidate) in audio {
        takes.push(TakeScore {
            take_id: t.id.clone(),
            metrics: score_with_gain(&p.audio, &candidate, Some(gain))?,
        });
    }
    Ok(Fit {
        objective_db: aggregate(&takes),
        applied_gain: gain,
        takes,
    })
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let path = Path::new(&args[1]).canonicalize()?;
    let mut bytes = Vec::new();
    File::open(&path)?.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err("manifest exceeds 64 KiB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    validate(&manifest)?;
    let prepared = prepare(&manifest, path.parent().ok_or("manifest has no parent")?)?;
    let mut candidates = Vec::new();
    for &gap in &manifest.gaps_mm {
        for &offset in &manifest.offsets_mm {
            let profile = Profile {
                pickup_gap_m: gap / 1000.0,
                pickup_offset_m: offset / 1000.0,
                ..Profile::default()
            };
            let (fit, rejection_reason) = match fit(&manifest, &prepared, profile) {
                Ok(result) => (Some(result), None),
                Err(e) => (None, Some(e.to_string())),
            };
            candidates.push(Candidate {
                gap_mm: gap,
                offset_mm: offset,
                fit,
                rejection_reason,
            });
        }
    }
    let mut ranking: Vec<_> = (0..candidates.len())
        .filter(|&i| candidates[i].fit.is_some())
        .collect();
    ranking.sort_by(|&a, &b| {
        candidates[a]
            .fit
            .as_ref()
            .unwrap()
            .objective_db
            .total_cmp(&candidates[b].fit.as_ref().unwrap().objective_db)
            .then(a.cmp(&b))
    });
    let best = ranking.first().copied();
    let near_best: Vec<_> = ranking
        .iter()
        .copied()
        .filter(|&i| {
            candidates[i].fit.as_ref().unwrap().objective_db
                <= candidates[ranking[0]].fit.as_ref().unwrap().objective_db + 0.01
        })
        .collect();
    let mut validation = Vec::new();
    let mut validation_errors = Vec::new();
    if let Some(index) = best {
        let selected = &candidates[index];
        let profile = Profile {
            pickup_gap_m: selected.gap_mm / 1000.0,
            pickup_offset_m: selected.offset_mm / 1000.0,
            ..Profile::default()
        };
        let gain = selected.fit.as_ref().unwrap().applied_gain;
        for (t, p) in manifest
            .takes
            .iter()
            .zip(&prepared)
            .filter(|(t, _)| t.role == Role::Validation)
        {
            match candidate_audio(t, p, profile)
                .and_then(|audio| score_with_gain(&p.audio, &audio, Some(gain)))
            {
                Ok(metrics) => validation.push(TakeScore {
                    take_id: t.id.clone(),
                    metrics,
                }),
                Err(e) => validation_errors
                    .push(serde_json::json!({"take_id":t.id,"reason":e.to_string()})),
            }
        }
    }
    let complete = best.is_some() && validation_errors.is_empty() && !validation.is_empty();
    let base = Profile::default();
    super::analysis::write_report(
        output,
        &serde_json::json!({
            "schema_version":1, "method":"pickup-set-v1; fit-only-pooled-rms-gain; equal-take-window-rms-error",
            "model":"research-0.1.1-uncalibrated", "manifest":manifest, "manifest_path":path,
            "inputs":prepared.iter().map(|p| &p.receipt).collect::<Vec<_>>(),
            "fixed_profile":{"hammer_mass_kg":base.hammer_mass_kg,"modal_mass_kg":base.modal_mass_kg,
                "contact_stiffness":base.contact_stiffness,"maximum_hammer_speed_m_s":base.maximum_hammer_speed_m_s,"decay_seconds":base.decay_seconds},
            "best_candidate_index":best,"ranking_indices":ranking,"near_best_candidate_indices":near_best,
            "near_best_tolerance_db":0.01,"candidates":candidates,"validation":validation,
            "validation_objective_db":if complete { Some(aggregate(&validation)) } else { None },
            "validation_errors":validation_errors,"evaluation_complete":complete,
        }),
    )?;
    println!("Pickup set report: {}", output.display());
    if !complete {
        return Err("fit or validation could not be completed; inspect report".into());
    }
    let selected = &candidates[best.unwrap()];
    println!(
        "best_gap_mm={} best_offset_mm={} fit_objective_db={:.6} validation_objective_db={:.6}",
        selected.gap_mm,
        selected.offset_mm,
        selected.fit.as_ref().unwrap().objective_db,
        aggregate(&validation)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooled_gain_preserves_differences_between_strike_intensities() {
        let take = |velocity| Take {
            id: format!("v-{velocity}"),
            file: "unused.wav".into(),
            role: Role::Fit,
            note: 57,
            velocity,
            velocity_basis: "Synthetic input".into(),
            reference_start_seconds: 0.1,
            model_start_seconds: 0.1,
            seconds: 0.256,
            sustain_end_seconds: 0.5,
            channel: None,
        };
        let manifest = Manifest {
            schema_version: 1,
            source: "Synthetic".into(),
            source_revision: "Test".into(),
            license: "Test".into(),
            instrument: "Research".into(),
            processing: "Test gains".into(),
            capture_gain: "Controlled".into(),
            gaps_mm: vec![1.5],
            offsets_mm: vec![0.5],
            takes: vec![take(0.3), take(0.7)],
        };
        let prepared = |gains: [f64; 2]| {
            manifest
                .takes
                .iter()
                .zip(gains)
                .map(|(t, gain)| {
                    let clip =
                        render(48000, 12288, 4800, t.note, t.velocity, Profile::default()).unwrap();
                    Prepared {
                        audio: AudioClip::from_samples(
                            48000,
                            clip.samples().iter().map(|s| s * gain).collect(),
                        )
                        .unwrap(),
                        model_start: 4800,
                        receipt: serde_json::Value::Null,
                    }
                })
                .collect::<Vec<_>>()
        };
        let uniform = fit(&manifest, &prepared([2.0, 2.0]), Profile::default()).unwrap();
        assert!((uniform.applied_gain - 2.0).abs() < 1e-12);
        assert!(uniform.objective_db < 1e-10);
        let varying = fit(&manifest, &prepared([2.0, 4.0]), Profile::default()).unwrap();
        assert!(varying.applied_gain > 2.0 && varying.applied_gain < 4.0);
        assert!(varying.objective_db > 0.1);
    }
}
