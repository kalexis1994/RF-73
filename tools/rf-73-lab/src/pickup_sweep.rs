//! Bounded offline experiment; never applies a fitted geometry to the plugin.
use rf_73_analysis::{AudioClip, compare};
use rf_73_dsp::{Engine, FIRST_NOTE, LAST_NOTE, Profile};
use serde::Serialize;
use std::{collections::BTreeMap, error::Error, path::Path};

pub const HELP: &str = "Pickup sweep:
  sweep-pickup REFERENCE.wav --output REPORT.json --note N --velocity V --seconds S
    --gaps-mm 1,1.5,2 --offsets-mm 0.25,0.5,0.75
    [--reference-start S] [--model-start S] [--channel 0]
Select corresponding held-note regions. No automatic alignment or resampling.
Duration: 0.128..4 s; model start: 0..2 s; normalized velocity: 0.01..1.
Grid: 1..5 unique values per axis, at most 25 candidates.
Ranks 128 ms spectra (64 ms hop) after ONE whole-region RMS gain correction.
Reports original level/error, every window score and near ties. Does not change the plugin.
";

#[derive(Clone, Serialize)]
struct Options {
    note: u8,
    velocity: f64,
    seconds: f64,
    reference_start_seconds: f64,
    model_start_seconds: f64,
    gaps_mm: Vec<f64>,
    offsets_mm: Vec<f64>,
}

#[derive(Serialize)]
struct WindowScore {
    start_seconds: f64,
    requested_samples: usize,
    spectral_start_seconds: Option<f64>,
    spectral_observation_samples: Option<usize>,
    raw_spectral_error_db: Option<f64>,
    level_matched_spectral_error_db: Option<f64>,
}

#[derive(Serialize)]
pub(super) struct Metrics {
    pub objective_db: f64,
    raw_objective_db: f64,
    candidate_level_minus_reference_db: f64,
    candidate_gain_to_match_reference: f64,
    applied_candidate_gain: f64,
    applied_gain_normalized_rmse: f64,
    raw_normalized_rmse: Option<f64>,
    level_matched_normalized_rmse: Option<f64>,
    scored_windows: usize,
    windows: Vec<WindowScore>,
}

#[derive(Serialize)]
struct Candidate {
    gap_mm: f64,
    offset_mm: f64,
    metrics: Option<Metrics>,
    rejection_reason: Option<String>,
}

pub(super) fn grid(value: &str, low: f64, high: f64) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut values = Vec::new();
    for token in value.split(',') {
        let number: f64 = token.trim().parse()?;
        if !number.is_finite()
            || !(low..=high).contains(&number)
            || values.contains(&number)
            || values.len() == 5
        {
            return Err(
                format!("grid needs 1..5 distinct finite values within {low}..{high}").into(),
            );
        }
        values.push(number);
    }
    Ok(values)
}

/// Selection diagnostics, not confidence estimates; grid input order is irrelevant.
pub(super) fn selection_limits(
    gap: f64,
    offset: f64,
    gaps: &[f64],
    offsets: &[f64],
) -> Vec<String> {
    let mut limits = Vec::new();
    for (name, selected, values, low, high) in [
        ("gap", gap, gaps, 0.5, 5.0),
        ("offset", offset, offsets, -3.0, 3.0),
    ] {
        if values.len() == 1 {
            limits.push(format!("{name}_fixed"));
        } else {
            if selected == values.iter().copied().fold(f64::INFINITY, f64::min) {
                limits.push(format!("{name}_grid_min"));
            }
            if selected == values.iter().copied().fold(f64::NEG_INFINITY, f64::max) {
                limits.push(format!("{name}_grid_max"));
            }
        }
        if selected == low {
            limits.push(format!("{name}_profile_min"));
        }
        if selected == high {
            limits.push(format!("{name}_profile_max"));
        }
    }
    limits
}

fn validate(options: &Options) -> Result<(), Box<dyn Error>> {
    if !(FIRST_NOTE..=LAST_NOTE).contains(&options.note)
        || !options.velocity.is_finite()
        || !(0.01..=1.0).contains(&options.velocity)
        || !options.seconds.is_finite()
        || !(0.128..=4.0).contains(&options.seconds)
        || !options.reference_start_seconds.is_finite()
        || options.reference_start_seconds < 0.0
        || !options.model_start_seconds.is_finite()
        || !(0.0..=2.0).contains(&options.model_start_seconds)
    {
        return Err("invalid note, velocity, duration or region start".into());
    }
    Ok(())
}

pub(super) fn render(
    rate: u32,
    frames: usize,
    start: usize,
    note: u8,
    velocity: f64,
    profile: Profile,
) -> Result<AudioClip, Box<dyn Error>> {
    let mut engine = Engine::new(rate as f64, profile)?;
    if !engine.note_on(0, note, velocity) {
        return Err("model rejected the strike".into());
    }
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..start + frames {
        let sample = engine.next_sample();
        if !sample.is_finite() || engine.faults() != 0 {
            return Err("candidate render encountered a numerical fault".into());
        }
        if frame >= start {
            samples.push(sample as f64);
        }
    }
    Ok(AudioClip::from_samples(rate, samples)?)
}

fn rms(samples: &[f64]) -> f64 {
    (samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64).sqrt()
}

fn score(reference: &AudioClip, candidate: &AudioClip) -> Result<Metrics, Box<dyn Error>> {
    score_with_gain(reference, candidate, None)
}

pub(super) fn score_with_gain(
    reference: &AudioClip,
    candidate: &AudioClip,
    fixed_gain: Option<f64>,
) -> Result<Metrics, Box<dyn Error>> {
    let rate = reference.metadata().sample_rate;
    let size = (rate as f64 * 0.128).round() as usize;
    if reference.samples().len() < size
        || reference.samples().len() != candidate.samples().len()
        || rate != candidate.metadata().sample_rate
    {
        return Err("scoring requires equal-rate, equal-length regions of at least 128 ms".into());
    }
    let whole = compare(reference, candidate, 0.0)?;
    let matching_gain = whole
        .candidate_gain_to_match_reference
        .ok_or("candidate or reference has insufficient signal energy")?;
    let gain = fixed_gain.unwrap_or(matching_gain);
    if !gain.is_finite() || gain <= 0.0 {
        return Err("applied gain must be finite and positive".into());
    }
    let hop = (rate as f64 * 0.064).round() as usize;
    let mut starts: Vec<_> = (0..=reference.samples().len() - size)
        .step_by(hop)
        .collect();
    // Cover the selected tail even if it does not land on the regular hop grid.
    let last = reference.samples().len() - size;
    if starts.last() != Some(&last) {
        starts.push(last);
    }
    let mut windows = Vec::new();
    let (mut raw_sum, mut matched_sum, mut count) = (0.0, 0.0, 0);
    for start in starts {
        let a = &reference.samples()[start..start + size];
        let b = &candidate.samples()[start..start + size];
        let (raw, matched, spectral_start, observed) = if rms(a) <= 1e-12 {
            (None, None, None, None)
        } else {
            let a = AudioClip::from_samples(rate, a.to_vec())?;
            let b = AudioClip::from_samples(rate, b.to_vec())?;
            let corrected =
                AudioClip::from_samples(rate, b.samples().iter().map(|s| s * gain).collect())?;
            let raw = compare(&a, &b, 0.0)?;
            (
                raw.log_spectral_distance_db,
                compare(&a, &corrected, 0.0)?.log_spectral_distance_db,
                Some(start as f64 / rate as f64 + raw.spectral_start_seconds),
                Some(raw.spectral_observation_samples),
            )
        };
        if let (Some(a), Some(b)) = (raw, matched) {
            raw_sum += a * a;
            matched_sum += b * b;
            count += 1;
        }
        windows.push(WindowScore {
            start_seconds: start as f64 / rate as f64,
            requested_samples: size,
            spectral_start_seconds: spectral_start,
            spectral_observation_samples: observed,
            raw_spectral_error_db: raw,
            level_matched_spectral_error_db: matched,
        });
    }
    if count == 0 {
        return Err("no reference windows with measurable energy".into());
    }
    Ok(Metrics {
        objective_db: (matched_sum / count as f64).sqrt(),
        raw_objective_db: (raw_sum / count as f64).sqrt(),
        candidate_level_minus_reference_db: whole
            .candidate_level_minus_reference_db
            .expect("positive gain"),
        candidate_gain_to_match_reference: matching_gain,
        applied_candidate_gain: gain,
        applied_gain_normalized_rmse: rms(&reference
            .samples()
            .iter()
            .zip(candidate.samples())
            .map(|(a, b)| a - gain * b)
            .collect::<Vec<_>>())
            / rms(reference.samples()),
        raw_normalized_rmse: whole.raw_normalized_rmse,
        level_matched_normalized_rmse: whole.level_matched_normalized_rmse,
        scored_windows: count,
        windows,
    })
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 2 {
        return Err(HELP.into());
    }
    let rest = &args[2..];
    if !rest.len().is_multiple_of(2) {
        return Err("sweep options require a flag and value".into());
    }
    let mut flags = BTreeMap::new();
    for pair in rest.as_chunks::<2>().0 {
        let flag = pair[0].as_str();
        if !matches!(
            flag,
            "--output"
                | "--note"
                | "--velocity"
                | "--seconds"
                | "--gaps-mm"
                | "--offsets-mm"
                | "--reference-start"
                | "--model-start"
                | "--channel"
        ) || flags.insert(flag, pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or duplicate option: {flag}").into());
        }
    }
    let required = |flag| {
        flags
            .get(flag)
            .copied()
            .ok_or_else(|| format!("{flag} is required"))
    };
    let output = Path::new(required("--output")?);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let options = Options {
        note: required("--note")?.parse()?,
        velocity: required("--velocity")?.parse()?,
        seconds: required("--seconds")?.parse()?,
        gaps_mm: grid(required("--gaps-mm")?, 0.5, 5.0)?,
        offsets_mm: grid(required("--offsets-mm")?, -3.0, 3.0)?,
        reference_start_seconds: flags
            .get("--reference-start")
            .map_or(Ok(0.0), |v| v.parse())?,
        model_start_seconds: flags.get("--model-start").map_or(Ok(0.0), |v| v.parse())?,
    };
    validate(&options)?;
    let input = AudioClip::open(
        &args[1],
        flags
            .get("--channel")
            .map(|v| v.parse::<u16>())
            .transpose()?,
    )?;
    let rate = input.metadata().sample_rate;
    if ![44100, 48000, 96000, 192000].contains(&rate) {
        return Err("sweep requires a 44100, 48000, 96000 or 192000 Hz reference".into());
    }
    if options.reference_start_seconds > input.duration() {
        return Err("reference start exceeds recording duration".into());
    }
    let frames = (options.seconds * rate as f64).round() as usize;
    let reference_start = (options.reference_start_seconds * rate as f64).round() as usize;
    let model_start = (options.model_start_seconds * rate as f64).round() as usize;
    let reference_samples = input
        .samples()
        .get(reference_start..reference_start + frames)
        .ok_or("reference region exceeds recording duration")?;
    if rms(reference_samples) <= 1e-12 {
        return Err("reference region has insufficient signal energy".into());
    }
    let reference = AudioClip::from_samples(rate, reference_samples.to_vec())?;
    let mut candidates = Vec::new();
    for &gap in &options.gaps_mm {
        for &offset in &options.offsets_mm {
            let profile = Profile {
                pickup_gap_m: gap / 1000.0,
                pickup_offset_m: offset / 1000.0,
                ..Profile::default()
            };
            let result = render(
                rate,
                frames,
                model_start,
                options.note,
                options.velocity,
                profile,
            )
            .and_then(|candidate| score(&reference, &candidate));
            let (metrics, rejection_reason) = match result {
                Ok(metrics) => (Some(metrics), None),
                Err(error) => (None, Some(error.to_string())),
            };
            candidates.push(Candidate {
                gap_mm: gap,
                offset_mm: offset,
                metrics,
                rejection_reason,
            });
        }
    }
    let mut ranking: Vec<_> = (0..candidates.len())
        .filter(|&i| candidates[i].metrics.is_some())
        .collect();
    ranking.sort_by(|&a, &b| {
        candidates[a]
            .metrics
            .as_ref()
            .unwrap()
            .objective_db
            .total_cmp(&candidates[b].metrics.as_ref().unwrap().objective_db)
            .then(a.cmp(&b))
    });
    let best = ranking.first().copied();
    let limits = best.map(|i| {
        selection_limits(
            candidates[i].gap_mm,
            candidates[i].offset_mm,
            &options.gaps_mm,
            &options.offsets_mm,
        )
    });
    let near_best: Vec<_> = best.map_or_else(Vec::new, |index| {
        let limit = candidates[index].metrics.as_ref().unwrap().objective_db + 0.01;
        ranking
            .iter()
            .copied()
            .filter(|&i| candidates[i].metrics.as_ref().unwrap().objective_db <= limit)
            .collect()
    });
    let base = Profile::default();
    super::analysis::write_report(
        output,
        &serde_json::json!({
            "schema_version":1, "method":"pickup-grid-v1; global-rms-gain; rms-of-window-log-spectral-errors",
            "model":"research-0.1.1-uncalibrated", "reference":args[1], "reference_audio":input.metadata(), "options":options,
            "reference_start_frame":reference_start,"model_start_frame":model_start,"compared_frames":frames,
            "fixed_profile":{"hammer_mass_kg":base.hammer_mass_kg,"modal_mass_kg":base.modal_mass_kg,
                "contact_stiffness":base.contact_stiffness,"maximum_hammer_speed_m_s":base.maximum_hammer_speed_m_s,"decay_seconds":base.decay_seconds},
            "near_best_tolerance_db":0.01,"best_candidate_index":best,"near_best_candidate_indices":near_best,
            "selection_limits":limits,
            "ranking_indices":ranking,"candidates":candidates,
        }),
    )?;
    println!("Pickup sweep: {}", output.display());
    if let Some(i) = best {
        println!(
            "candidates={} best_gap_mm={} best_offset_mm={} objective_db={:.6} near_best_count={}",
            candidates.len(),
            candidates[i].gap_mm,
            candidates[i].offset_mm,
            candidates[i].metrics.as_ref().unwrap().objective_db,
            near_best.len()
        );
    } else {
        return Err("no candidate could be scored; inspect rejection reasons in the report".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_grids_are_rejected_before_rendering() {
        for bad in ["", "NaN", "inf", "1,1.0", "1,", "0.4", "6", "0.5,1,2,3,4,5"] {
            assert!(grid(bad, 0.5, 5.0).is_err(), "{bad}");
        }
        assert_eq!(grid("1,1.5,2", 0.5, 5.0).unwrap(), vec![1.0, 1.5, 2.0]);
    }

    #[test]
    fn selection_limits_distinguish_edges_fixed_axes_and_interior() {
        assert_eq!(
            selection_limits(0.5, 0.25, &[1.5, 0.5, 1.0], &[0.5, 0.0, 0.25]),
            vec!["gap_grid_min", "gap_profile_min"]
        );
        assert!(selection_limits(1.0, 0.25, &[1.5, 0.5, 1.0], &[0.5, 0.0, 0.25]).is_empty());
        assert_eq!(
            selection_limits(1.0, 3.0, &[1.0], &[-3.0, 3.0]),
            vec!["gap_fixed", "offset_grid_max", "offset_profile_max"]
        );
    }

    #[test]
    fn one_global_gain_preserves_time_varying_level_errors() {
        let a: Vec<_> = (0..48000)
            .map(|i| 0.2 * (std::f64::consts::TAU * 220.0 * i as f64 / 48000.0).sin())
            .collect();
        let constant = AudioClip::from_samples(48000, a.iter().map(|v| v * 0.5).collect()).unwrap();
        let varying = AudioClip::from_samples(
            48000,
            a.iter()
                .enumerate()
                .map(|(i, v)| v * if i < 24000 { 0.5 } else { 1.5 })
                .collect(),
        )
        .unwrap();
        let reference = AudioClip::from_samples(48000, a).unwrap();
        assert!(score(&reference, &constant).unwrap().objective_db < 1e-10);
        let frozen = score_with_gain(&reference, &constant, Some(1.0)).unwrap();
        assert!(frozen.objective_db > 0.1);
        assert!((frozen.applied_gain_normalized_rmse - 0.5).abs() < 1e-12);
        assert_eq!(frozen.applied_candidate_gain, 1.0);
        assert!(score(&reference, &varying).unwrap().objective_db > 0.1);
    }

    #[test]
    fn unmeasurable_and_mismatched_regions_return_errors() {
        let silence = AudioClip::from_samples(48000, vec![0.0; 6144]).unwrap();
        let short = AudioClip::from_samples(48000, vec![0.1; 128]).unwrap();
        let signal = AudioClip::from_samples(48000, vec![0.1; 6144]).unwrap();
        assert!(score(&silence, &signal).is_err());
        assert!(score(&signal, &silence).is_err());
        assert!(score(&short, &short).is_err());
        assert!(score(&signal, &short).is_err());
    }
}
