//! Local-background evidence for a weak harmonic; never changes fit scoring.
use crate::{AudioClip, AudioError, spectrum::Spectrum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HarmonicProbe {
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub resolution_hz: f64,
    pub frequency_hz: Option<f64>,
    pub relative_to_h1_db: Option<f64>,
    pub median_margin_db: Option<f64>,
    pub p90_margin_db: Option<f64>,
    pub legacy_detected: bool,
    pub supported: bool,
    pub rejection_reasons: Vec<&'static str>,
}

/// Complete Hann window, local H2 peak without the snapshot's global -60 dB
/// cutoff. Background uses bins 3..8 observation resolutions from expected H2.
/// Exploratory support requires 24 dB median / 12 dB p90 margin and frequency
/// agreement within max(one observation resolution, 0.2% of H2).
pub fn probe_second_harmonic(
    clip: &AudioClip,
    start: f64,
    duration: f64,
    fundamental: f64,
) -> Result<HarmonicProbe, AudioError> {
    let rate = f64::from(clip.metadata().sample_rate);
    if !start.is_finite()
        || start < 0.0
        || !duration.is_finite()
        || !(0.064..=0.512).contains(&duration)
        || !fundamental.is_finite()
        || !(400.0..=600.0).contains(&fundamental)
        || !(16000.0..=192000.0).contains(&rate)
    {
        return Err(AudioError("H2 probe requires a finite C5-range frequency, start >=0, 64..512 ms and rate 16..192 kHz".into()));
    }
    let from = (start * rate).round() as usize;
    let count = (duration * rate).round() as usize;
    let samples = from
        .checked_add(count)
        .and_then(|end| clip.samples().get(from..end))
        .ok_or_else(|| AudioError("H2 probe requires the complete observation".into()))?;
    let s = Spectrum::for_tracking(samples, clip.metadata().sample_rate);
    let target = 2.0 * fundamental;
    let tolerance = (2.0 * s.spacing).max(target * 0.015).min(fundamental * 0.2);
    let local_peak = |target: f64, tolerance: f64| {
        (1..s.amplitudes.len() - 1)
            .filter(|&i| {
                (i as f64 * s.spacing - target).abs() <= tolerance
                    && s.amplitudes[i] > 1e-12
                    && s.amplitudes[i] >= s.amplitudes[i - 1]
                    && s.amplitudes[i] > s.amplitudes[i + 1]
            })
            .max_by(|&a, &b| s.amplitudes[a].total_cmp(&s.amplitudes[b]))
            .map(|i| s.peak(i))
    };
    let h1 = local_peak(fundamental, tolerance / 2.0);
    let h2 = local_peak(target, tolerance);
    let mut background: Vec<_> = s
        .amplitudes
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            (3.0 * s.resolution..=8.0 * s.resolution)
                .contains(&(*i as f64 * s.spacing - target).abs())
        })
        .map(|(_, a)| *a)
        .collect();
    background.sort_by(f64::total_cmp);
    let median = 20.0 * background[background.len() / 2].max(1e-150).log10();
    let p90 = 20.0
        * background[(background.len() - 1) * 9 / 10]
            .max(1e-150)
            .log10();
    let median_margin = h2.as_ref().map(|p| p.amplitude_dbfs - median);
    let p90_margin = h2.as_ref().map(|p| p.amplitude_dbfs - p90);
    let mut reasons = Vec::new();
    if h1.is_none() {
        reasons.push("missing_h1");
    }
    if h2.is_none() {
        reasons.push("missing_h2");
    }
    if median_margin.is_none_or(|v| v < 24.0) {
        reasons.push("median_margin_below_24_db");
    }
    if p90_margin.is_none_or(|v| v < 12.0) {
        reasons.push("p90_margin_below_12_db");
    }
    if h2
        .as_ref()
        .is_none_or(|p| (p.frequency_hz - target).abs() > s.resolution.max(target * 0.002))
    {
        reasons.push("frequency_mismatch");
    }
    Ok(HarmonicProbe {
        start_seconds: from as f64 / rate,
        duration_seconds: count as f64 / rate,
        resolution_hz: s.resolution,
        frequency_hz: h2.as_ref().map(|p| p.frequency_hz),
        relative_to_h1_db: h2
            .as_ref()
            .zip(h1.as_ref())
            .map(|(b, a)| b.amplitude_dbfs - a.amplitude_dbfs),
        median_margin_db: median_margin,
        p90_margin_db: p90_margin,
        legacy_detected: s.strongest_near(target, tolerance).is_some(),
        supported: reasons.is_empty(),
        rejection_reasons: reasons,
    })
}
