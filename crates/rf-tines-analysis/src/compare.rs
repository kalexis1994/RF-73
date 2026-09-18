use crate::{AudioClip, AudioError, db, measurement::onset, rms, spectrum::Spectrum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub schema_version: u32,
    pub sample_rate: u32,
    pub reference_frames: usize,
    pub candidate_frames: usize,
    /// Positive means candidate is delayed; advance candidate by this amount.
    pub candidate_delay_samples: i32,
    pub alignment_limit_samples: i32,
    pub alignment_at_limit: bool,
    pub alignment_correlation: Option<f64>,
    pub compared_frames: usize,
    pub candidate_gain_to_match_reference: Option<f64>,
    pub candidate_level_minus_reference_db: Option<f64>,
    pub raw_normalized_rmse: Option<f64>,
    pub level_matched_normalized_rmse: Option<f64>,
    pub log_spectral_distance_db: Option<f64>,
    pub level_matched_log_spectral_distance_db: Option<f64>,
    pub spectral_floor_below_reference_peak_db: f64,
    pub spectral_observation_samples: usize,
    /// Relative to the start of the aligned overlap, before the Hann window.
    pub spectral_start_seconds: f64,
}

pub fn compare(
    reference: &AudioClip,
    candidate: &AudioClip,
    align_ms: f64,
) -> Result<Comparison, AudioError> {
    if reference.metadata.sample_rate != candidate.metadata.sample_rate {
        return Err(AudioError(
            "comparison requires equal sample rates; no implicit resampling is performed".into(),
        ));
    }
    if !align_ms.is_finite() || !(0.0..=100.0).contains(&align_ms) {
        return Err(AudioError(
            "alignment search must be finite and within 0..100 ms".into(),
        ));
    }
    if reference.samples.len().min(candidate.samples.len()) < 128 {
        return Err(AudioError(
            "comparison needs at least 128 samples per recording".into(),
        ));
    }
    let rate = reference.metadata.sample_rate;
    let bound = (align_ms * rate as f64 / 1000.0).round() as i32;
    let (lag, correlation) = align(&reference.samples, &candidate.samples, rate, bound);
    let (a, b) = overlap(&reference.samples, &candidate.samples, lag);
    if a.len() < 128 {
        return Err(AudioError("too little overlap after alignment".into()));
    }
    let a_rms = rms(a);
    let b_rms = rms(b);
    let gain = (a_rms > 1e-12 && b_rms > 1e-12).then(|| a_rms / b_rms);
    let nrmse = |gain: f64| {
        (a_rms > 1e-12).then(|| {
            (a.iter()
                .zip(b)
                .map(|(x, y)| (x - gain * y).powi(2))
                .sum::<f64>()
                / a.len() as f64)
                .sqrt()
                / a_rms
        })
    };
    let spectral_start = onset(a).unwrap_or(0).min(a.len() - 128);
    let spectral_size = (a.len() - spectral_start).min(32_768);
    let sa = Spectrum::new(&a[spectral_start..spectral_start + spectral_size], rate);
    let sb = Spectrum::new(&b[spectral_start..spectral_start + spectral_size], rate);
    let reference_peak = sa.amplitudes.iter().copied().fold(0.0, f64::max);
    let floor = db(reference_peak).map(|db| db - 80.0);
    let spectral_error = |gain_db: f64| {
        floor.map(|floor| {
            let mut sum = 0.0;
            let mut count = 0;
            for i in 1..sa.amplitudes.len() - 1 {
                let frequency = i as f64 * sa.spacing;
                if frequency < 20.0 {
                    continue;
                }
                let a_db = sa.amplitude_db(frequency).unwrap_or(floor).max(floor);
                let b_db =
                    (sb.amplitude_db(frequency).unwrap_or(floor - gain_db) + gain_db).max(floor);
                sum += (a_db - b_db).powi(2);
                count += 1;
            }
            (sum / count as f64).sqrt()
        })
    };
    Ok(Comparison {
        schema_version: 1,
        sample_rate: rate,
        reference_frames: reference.samples.len(),
        candidate_frames: candidate.samples.len(),
        candidate_delay_samples: lag,
        alignment_limit_samples: bound,
        alignment_at_limit: bound > 0 && lag.abs() == bound,
        alignment_correlation: correlation,
        compared_frames: a.len(),
        candidate_gain_to_match_reference: gain,
        candidate_level_minus_reference_db: gain.and_then(|g| db(1.0 / g)),
        raw_normalized_rmse: nrmse(1.0),
        level_matched_normalized_rmse: gain.and_then(nrmse),
        log_spectral_distance_db: spectral_error(0.0),
        level_matched_log_spectral_distance_db: gain.and_then(db).and_then(spectral_error),
        spectral_floor_below_reference_peak_db: 80.0,
        spectral_observation_samples: spectral_size,
        spectral_start_seconds: spectral_start as f64 / rate as f64,
    })
}

fn overlap<'a>(a: &'a [f64], b: &'a [f64], lag: i32) -> (&'a [f64], &'a [f64]) {
    let a_start = (-lag).max(0) as usize;
    let b_start = lag.max(0) as usize;
    let length = a
        .len()
        .saturating_sub(a_start)
        .min(b.len().saturating_sub(b_start));
    if length == 0 {
        return (&a[..0], &b[..0]);
    }
    (&a[a_start..a_start + length], &b[b_start..b_start + length])
}

fn align(a: &[f64], b: &[f64], rate: u32, bound: i32) -> (i32, Option<f64>) {
    let Some(start) = onset(a) else {
        return (0, None);
    };
    let end = (start + (rate as usize * 15 / 100)).min(a.len());
    let score = |lag: i32, stride: usize| {
        let (mut aa, mut bb, mut ab, mut count) = (0.0, 0.0, 0.0, 0);
        for i in (start..end).step_by(stride) {
            let j = i as i64 + lag as i64;
            // Out-of-file samples are zero padded; do not reward tiny overlaps.
            // Box-average the coarse grid instead of aliasing isolated samples.
            let mut x = 0.0;
            let mut y = 0.0;
            for offset in 0..stride {
                x += a.get(i + offset).copied().unwrap_or(0.0);
                let target = j + offset as i64;
                if target >= 0 {
                    y += b.get(target as usize).copied().unwrap_or(0.0);
                }
            }
            x /= stride as f64;
            y /= stride as f64;
            aa += x * x;
            bb += y * y;
            ab += x * y;
            count += 1;
        }
        if count < 8 || aa < 1e-24 || bb < 1e-24 {
            None
        } else {
            Some(ab / (aa * bb).sqrt())
        }
    };
    let mut best = 0;
    let mut best_score = score(0, 16).unwrap_or(-2.0);
    let prefer = |value: f64, old: f64, lag: i32, best: i32| {
        value > old + 1e-10 || ((value - old).abs() <= 1e-10 && lag.abs() < best.abs())
    };
    for lag in (-bound..=bound).step_by(16) {
        if let Some(value) = score(lag, 16)
            && prefer(value, best_score, lag, best)
        {
            best = lag;
            best_score = value;
        }
    }
    let coarse = best;
    best_score = score(best, 1).unwrap_or(-2.0);
    for lag in (coarse - 16).max(-bound)..=(coarse + 16).min(bound) {
        if let Some(value) = score(lag, 1)
            && prefer(value, best_score, lag, best)
        {
            best = lag;
            best_score = value;
        }
    }
    if best_score < -1.0 {
        (0, None)
    } else {
        (best, Some(best_score.clamp(-1.0, 1.0)))
    }
}
