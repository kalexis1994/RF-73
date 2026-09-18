//! Short, explicit spectral observations without inferred strike velocities or decay fits.
use crate::{
    AudioClip, AudioError, db, rms,
    spectrum::{SpectralSnapshot, Spectrum},
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct ToneComparisonOptions {
    pub note: u8,
    pub reference_start_seconds: f64,
    pub candidate_start_seconds: f64,
}

impl Default for ToneComparisonOptions {
    fn default() -> Self {
        Self {
            note: 57,
            reference_start_seconds: 0.0,
            candidate_start_seconds: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HarmonicBalance {
    pub harmonic: u32,
    pub reference_relative_to_fundamental_db: Option<f64>,
    pub candidate_relative_to_fundamental_db: Option<f64>,
    pub candidate_minus_reference_balance_db: Option<f64>,
    pub candidate_minus_reference_raw_db: Option<f64>,
    pub candidate_minus_reference_cents: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ToneWindow {
    pub label: &'static str,
    pub reference_start_frame: usize,
    pub candidate_start_frame: usize,
    pub reference_rms_dbfs: Option<f64>,
    pub candidate_rms_dbfs: Option<f64>,
    pub candidate_minus_reference_rms_db: Option<f64>,
    /// Four expected cycles are required before H1 can normalize harmonic balance.
    pub enough_fundamental_cycles: bool,
    pub reference: SpectralSnapshot,
    pub candidate: SpectralSnapshot,
    pub harmonics: Vec<HarmonicBalance>,
}

#[derive(Debug, Serialize)]
pub struct ToneComparison {
    pub schema_version: u32,
    pub method: &'static str,
    pub note: u8,
    pub sample_rate: u32,
    pub reference_anchor_frame: usize,
    pub candidate_anchor_frame: usize,
    pub windows: Vec<ToneWindow>,
}

pub fn compare_tone(
    reference: &AudioClip,
    candidate: &AudioClip,
    options: ToneComparisonOptions,
) -> Result<ToneComparison, AudioError> {
    let rate = reference.metadata().sample_rate;
    if rate != candidate.metadata().sample_rate || options.note > 127 {
        return Err(AudioError(
            "tone comparison needs equal sample rates and a MIDI note 0..127".into(),
        ));
    }
    let anchor = |clip: &AudioClip, seconds: f64| {
        if !seconds.is_finite() || seconds < 0.0 || seconds > clip.duration() {
            return Err(AudioError(
                "tone anchor must lie inside its recording".into(),
            ));
        }
        Ok((seconds * rate as f64).round() as usize)
    };
    let a_start = anchor(reference, options.reference_start_seconds)?;
    let b_start = anchor(candidate, options.candidate_start_seconds)?;
    let expected = 440.0 * 2.0_f64.powf((options.note as f64 - 69.0) / 12.0);
    let mut windows = Vec::new();
    for (label, offset, seconds) in [
        ("attack_32_ms", 0.0, 0.032),
        ("attack_96_ms", 0.0, 0.096),
        ("body_350_ms", 0.25, 0.35),
    ] {
        let offset = (offset * rate as f64).round() as usize;
        let size = (seconds * rate as f64).round() as usize;
        let a_at = a_start + offset;
        let b_at = b_start + offset;
        let a = reference
            .samples()
            .get(a_at..a_at + size)
            .ok_or_else(|| AudioError("reference lacks a complete requested tone window".into()))?;
        let b = candidate
            .samples()
            .get(b_at..b_at + size)
            .ok_or_else(|| AudioError("candidate lacks a complete requested tone window".into()))?;
        // Full observation, including 67,200 body samples at 192 kHz.
        let sa = Spectrum::for_tracking(a, rate).snapshot(a_at as f64 / rate as f64, expected);
        let sb = Spectrum::for_tracking(b, rate).snapshot(b_at as f64 / rate as f64, expected);
        let enough_cycles = size as f64 / rate as f64 * expected >= 4.0;
        let fundamental = |s: &SpectralSnapshot| {
            enough_cycles
                .then(|| {
                    s.harmonics
                        .first()
                        .and_then(|h| h.peak.as_ref())
                        .map(|p| p.amplitude_dbfs)
                })
                .flatten()
        };
        let a_h1 = fundamental(&sa);
        let b_h1 = fundamental(&sb);
        let harmonics = sa
            .harmonics
            .iter()
            .zip(&sb.harmonics)
            .map(|(a, b)| {
                let a_rel = a
                    .peak
                    .as_ref()
                    .zip(a_h1)
                    .map(|(p, h1)| p.amplitude_dbfs - h1);
                let b_rel = b
                    .peak
                    .as_ref()
                    .zip(b_h1)
                    .map(|(p, h1)| p.amplitude_dbfs - h1);
                let paired = a.peak.as_ref().zip(b.peak.as_ref());
                HarmonicBalance {
                    harmonic: a.harmonic,
                    reference_relative_to_fundamental_db: a_rel,
                    candidate_relative_to_fundamental_db: b_rel,
                    candidate_minus_reference_balance_db: b_rel.zip(a_rel).map(|(b, a)| b - a),
                    candidate_minus_reference_raw_db: paired
                        .map(|(a, b)| b.amplitude_dbfs - a.amplitude_dbfs),
                    candidate_minus_reference_cents: paired
                        .map(|(a, b)| 1200.0 * (b.frequency_hz / a.frequency_hz).log2()),
                }
            })
            .collect();
        let a_rms = db(rms(a));
        let b_rms = db(rms(b));
        windows.push(ToneWindow {
            label,
            reference_start_frame: a_at,
            candidate_start_frame: b_at,
            reference_rms_dbfs: a_rms,
            candidate_rms_dbfs: b_rms,
            candidate_minus_reference_rms_db: b_rms.zip(a_rms).map(|(b, a)| b - a),
            enough_fundamental_cycles: enough_cycles,
            reference: sa,
            candidate: sb,
            harmonics,
        });
    }
    Ok(ToneComparison {
        schema_version: 1,
        method: "tone-v1; explicit-full-windows; harmonic-balance-relative-to-H1; no-decay-fit",
        note: options.note,
        sample_rate: rate,
        reference_anchor_frame: a_start,
        candidate_anchor_frame: b_start,
        windows,
    })
}
