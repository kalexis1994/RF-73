//! Frequency-only observations for reference preparation, never modal identification.
use crate::{AudioClip, AudioError, spectrum::Spectrum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PitchWindow {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub frequency_hz: Option<f64>,
    pub amplitude_dbfs: Option<f64>,
    pub margin_above_band_background_db: Option<f64>,
    pub competing_peak_frequency_hz: Option<f64>,
    pub rejection_reasons: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct PitchAnchor {
    pub qualified: bool,
    pub frequency_hz: Option<f64>,
    pub temporal_span_cents: Option<f64>,
    pub search_band_hz: [f64; 2],
    pub observation_resolution_hz: f64,
    pub windows: Vec<PitchWindow>,
    pub rejection_reasons: Vec<&'static str>,
}

/// Three nonoverlapping 512 ms windows starting at file time 0.25 seconds.
/// Broad +/-400-cent search avoids locking onto a weak expected-note neighbor.
/// A qualified peak is still an output component, not a proven mechanical mode.
pub fn pitch_anchor(clip: &AudioClip, note: u8) -> Result<PitchAnchor, AudioError> {
    if !(40..=90).contains(&note) {
        return Err(AudioError("pitch-anchor note must be 40..90".into()));
    }
    let rate = clip.metadata.sample_rate as f64;
    let size = (0.512 * rate).round() as usize;
    let start = (0.25 * rate).round() as usize;
    if clip.samples.len() < start + 3 * size {
        return Err(AudioError(
            "pitch anchor requires all three complete 512 ms windows after 0.25 s".into(),
        ));
    }
    let expected = 440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0);
    let band = [
        expected / 2.0_f64.powf(1.0 / 3.0),
        expected * 2.0_f64.powf(1.0 / 3.0),
    ];
    let mut windows = Vec::new();
    for index in 0..3 {
        let from = start + index * size;
        let spectrum =
            Spectrum::for_tracking(&clip.samples[from..from + size], clip.metadata.sample_rate);
        let lo = (band[0] / spectrum.spacing).ceil() as usize;
        let hi = (band[1] / spectrum.spacing).floor() as usize;
        let mut background = spectrum.amplitudes[lo..=hi].to_vec();
        background.sort_by(f64::total_cmp);
        let background = background[background.len() / 2].max(1e-150);
        let mut peaks: Vec<_> = (lo..=hi)
            .filter(|&i| {
                spectrum.amplitudes[i] >= spectrum.amplitudes[i - 1]
                    && spectrum.amplitudes[i] > spectrum.amplitudes[i + 1]
                    && spectrum.amplitudes[i] > 1e-12
            })
            .map(|i| spectrum.peak(i))
            .filter(|p| (band[0]..=band[1]).contains(&p.frequency_hz))
            .collect();
        peaks.sort_by(|a, b| b.amplitude_dbfs.total_cmp(&a.amplitude_dbfs));
        let peak = peaks.first();
        let margin = peak.map(|p| p.amplitude_dbfs - 20.0 * background.log10());
        let rival = peak.and_then(|p| {
            peaks
                .iter()
                .skip(1)
                .find(|q| q.amplitude_dbfs >= p.amplitude_dbfs - 20.0)
        });
        let mut reasons = Vec::new();
        if peak.is_none() {
            reasons.push("no_peak");
        }
        if margin.is_none_or(|m| m < 24.0) {
            reasons.push("insufficient_band_background_margin");
        }
        if rival.is_some() {
            reasons.push("competing_peak_within_20_db");
        }
        if peak.is_some_and(|p| {
            p.frequency_hz - band[0] < 2.0 * spectrum.resolution
                || band[1] - p.frequency_hz < 2.0 * spectrum.resolution
        }) {
            reasons.push("peak_at_search_boundary");
        }
        windows.push(PitchWindow {
            start_seconds: from as f64 / rate,
            end_seconds: (from + size) as f64 / rate,
            frequency_hz: peak.map(|p| p.frequency_hz),
            amplitude_dbfs: peak.map(|p| p.amplitude_dbfs),
            margin_above_band_background_db: margin,
            competing_peak_frequency_hz: rival.map(|p| p.frequency_hz),
            rejection_reasons: reasons,
        });
    }
    let frequencies: Vec<_> = windows.iter().filter_map(|w| w.frequency_hz).collect();
    let span = (frequencies.len() == 3).then(|| {
        1200.0
            * (frequencies.iter().copied().fold(0.0_f64, f64::max)
                / frequencies.iter().copied().fold(f64::INFINITY, f64::min))
            .log2()
    });
    let mut reasons = Vec::new();
    if windows.iter().any(|w| !w.rejection_reasons.is_empty()) {
        reasons.push("unqualified_window");
    }
    if span.is_none_or(|s| !s.is_finite() || s > 5.0) {
        reasons.push("temporal_frequency_span_exceeds_5_cents");
    }
    let frequency = reasons
        .is_empty()
        .then(|| (frequencies.iter().map(|f| f.ln()).sum::<f64>() / 3.0).exp());
    Ok(PitchAnchor {
        qualified: frequency.is_some(),
        frequency_hz: frequency,
        temporal_span_cents: span,
        search_band_hz: band,
        observation_resolution_hz: rate / size as f64,
        windows,
        rejection_reasons: reasons,
    })
}
