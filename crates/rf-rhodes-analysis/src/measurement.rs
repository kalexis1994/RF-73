use crate::{
    AudioClip, AudioError, AudioMetadata, db, rms,
    spectrum::{Partial, SpectralPeak, SpectralSnapshot, Spectrum},
    tracking::PartialTracking,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub struct AnalysisOptions {
    /// Expected note, used only as a bounded fundamental-search hint.
    pub note: u8,
    /// Explicit end of the uninterrupted sustain region, relative to file start.
    /// None disables natural-decay estimation rather than fitting a key release.
    pub sustain_end_seconds: Option<f64>,
    /// Independent tracking observation: 32, 128, 512 or 1024 ms; hop is one quarter.
    /// Harmonic summaries retain their original 128 ms observation.
    pub partial_window_ms: u32,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            note: 57,
            sustain_end_seconds: None,
            partial_window_ms: 128,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EnvelopePoint {
    pub center_seconds: f64,
    pub rms_dbfs: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct DecayEstimate {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub slope_db_per_second: f64,
    pub r_squared: f64,
    pub fitted_drop_db: f64,
    pub extrapolated_t60_seconds: Option<f64>,
    pub points: usize,
}

#[derive(Debug, Serialize)]
pub struct PartialFrame {
    pub center_seconds: f64,
    pub harmonics: Vec<Partial>,
}

#[derive(Debug, Serialize)]
pub struct Analysis {
    pub schema_version: u32,
    pub method: &'static str,
    pub audio: AudioMetadata,
    pub duration_seconds: f64,
    pub expected_note: u8,
    pub expected_fundamental_hz: f64,
    pub peak: f64,
    pub peak_dbfs: Option<f64>,
    pub rms_dbfs: Option<f64>,
    pub dc_offset: f64,
    pub samples_at_or_above_full_scale: usize,
    pub onset_seconds: Option<f64>,
    pub envelope_peak_seconds: Option<f64>,
    pub fundamental: Option<SpectralPeak>,
    pub tuning_error_cents: Option<f64>,
    pub sustain_end_seconds: Option<f64>,
    pub decay: Option<DecayEstimate>,
    pub envelope: Vec<EnvelopePoint>,
    pub spectra: Vec<SpectralSnapshot>,
    pub partial_window_seconds: f64,
    pub partial_tracks: Vec<PartialFrame>,
    pub inharmonic_tracking: PartialTracking,
}

pub fn analyze(clip: &AudioClip, options: AnalysisOptions) -> Result<Analysis, AudioError> {
    if ![32, 128, 512, 1024].contains(&options.partial_window_ms) {
        return Err(AudioError(
            "partial window must be 32, 128, 512 or 1024 ms".into(),
        ));
    }
    if options.note > 127 {
        return Err(AudioError("expected note must be MIDI 0..127".into()));
    }
    if clip.samples.len() < 128 {
        return Err(AudioError("analysis needs at least 128 samples".into()));
    }
    if let Some(end) = options.sustain_end_seconds
        && (!end.is_finite() || end <= 0.0 || end > clip.duration())
    {
        return Err(AudioError(
            "sustain end must be finite, positive and inside the recording".into(),
        ));
    }
    let samples = &clip.samples;
    let rate = clip.metadata.sample_rate;
    let fs = rate as f64;
    let peak = samples.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    let onset = onset(samples);
    let mut envelope = Vec::new();
    let envelope_size = ((fs * 0.020).round() as usize).min(samples.len());
    let hop = (fs * 0.005).round() as usize;
    for start in (0..=samples.len() - envelope_size).step_by(hop.max(1)) {
        envelope.push(EnvelopePoint {
            center_seconds: (start as f64 + envelope_size as f64 / 2.0) / fs,
            rms_dbfs: db(rms(&samples[start..start + envelope_size])),
        });
    }
    let onset_seconds = onset.map(|i| i as f64 / fs);
    let envelope_peak = onset_seconds.and_then(|start| {
        envelope
            .iter()
            .filter(|p| p.center_seconds >= start && p.center_seconds <= start + 0.5)
            .filter_map(|p| p.rms_dbfs.map(|db| (p.center_seconds, db)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    });
    let expected = 440.0 * 2.0_f64.powf((options.note as f64 - 69.0) / 12.0);
    let mut spectra = Vec::new();
    let mut fundamental = None;
    let mut fundamental_size = 0;
    if let Some(onset) = onset {
        for (offset, seconds) in [(0.0, 0.032), (0.0, 0.096), (0.25, 0.35)] {
            let start = onset + (offset * fs) as usize;
            if start >= samples.len() {
                continue;
            }
            let size = ((seconds * fs) as usize)
                .min(samples.len() - start)
                .min(32_768);
            if size < 128 {
                continue;
            }
            let spectrum = Spectrum::new(&samples[start..start + size], rate);
            // Prefer the longest observation; require four cycles of the target.
            if size >= fundamental_size && size as f64 / fs * expected >= 4.0 {
                let candidate = spectrum.strongest_near(expected, expected * 0.06);
                if candidate.is_some() {
                    fundamental = candidate;
                    fundamental_size = size;
                }
            }
            spectra.push(spectrum.snapshot(start as f64 / fs, expected));
        }
    }
    let decay = match (envelope_peak, onset_seconds, options.sustain_end_seconds) {
        (Some((time, peak_db)), Some(onset), Some(end)) => fit_decay(
            &envelope,
            (time + 0.05).max(onset + 0.1),
            end - envelope_size as f64 / (2.0 * fs),
            peak_db - 60.0,
        ),
        _ => None,
    };
    let partial_size = ((fs * 0.128).round() as usize).min(32_768);
    let partial_hop = (fs * 0.032).round() as usize;
    let mut partial_tracks = Vec::new();
    let tracking_size = (fs * options.partial_window_ms as f64 / 1000.0).round() as usize;
    let tracking_hop = (fs * options.partial_window_ms as f64 / 4000.0).round() as usize;
    let mut inharmonic_tracking = PartialTracking::new(tracking_size, tracking_hop, rate);
    if samples.len() >= partial_size && onset.is_some() {
        for start in (0..=samples.len() - partial_size).step_by(partial_hop.max(1)) {
            let spectrum = Spectrum::new(&samples[start..start + partial_size], rate);
            if options.partial_window_ms == 128 {
                inharmonic_tracking
                    .push(&spectrum, (start as f64 + partial_size as f64 / 2.0) / fs);
            }
            let harmonics = (1..=6)
                .filter(|&n| n as f64 * expected < fs / 2.0)
                .map(|n| Partial {
                    harmonic: n,
                    target_hz: n as f64 * expected,
                    peak: spectrum.strongest_near(
                        n as f64 * expected,
                        (2.0 * spectrum.spacing)
                            .max(n as f64 * expected * 0.015)
                            .min(expected * 0.2),
                    ),
                })
                .collect();
            partial_tracks.push(PartialFrame {
                center_seconds: (start as f64 + partial_size as f64 / 2.0) / fs,
                harmonics,
            });
        }
    }
    if options.partial_window_ms != 128 && samples.len() >= tracking_size && onset.is_some() {
        for start in (0..=samples.len() - tracking_size).step_by(tracking_hop) {
            let spectrum = Spectrum::for_tracking(&samples[start..start + tracking_size], rate);
            inharmonic_tracking.push(&spectrum, (start as f64 + tracking_size as f64 / 2.0) / fs);
        }
    }
    inharmonic_tracking.finish(onset_seconds, options.sustain_end_seconds);
    Ok(Analysis {
        schema_version: 2,
        method: "hann-v1; amplitude-onset-40db; rms20ms-hop5ms; harmonic-track128ms-hop32ms; free-peaks-v1",
        audio: clip.metadata.clone(),
        duration_seconds: clip.duration(),
        expected_note: options.note,
        expected_fundamental_hz: expected,
        peak,
        peak_dbfs: db(peak),
        rms_dbfs: db(rms(samples)),
        dc_offset: samples.iter().sum::<f64>() / samples.len() as f64,
        samples_at_or_above_full_scale: samples.iter().filter(|x| x.abs() >= 1.0).count(),
        onset_seconds,
        envelope_peak_seconds: envelope_peak.map(|p| p.0),
        tuning_error_cents: fundamental
            .as_ref()
            .map(|p| 1200.0 * (p.frequency_hz / expected).log2()),
        fundamental,
        sustain_end_seconds: options.sustain_end_seconds,
        decay,
        envelope,
        spectra,
        partial_window_seconds: partial_size as f64 / fs,
        partial_tracks,
        inharmonic_tracking,
    })
}

pub(crate) fn onset(samples: &[f64]) -> Option<usize> {
    let peak = samples.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    if peak < 1e-12 {
        return None;
    }
    samples
        .iter()
        .position(|x| x.abs() >= (peak * 0.01).max(1e-12))
}

fn fit_decay(points: &[EnvelopePoint], start: f64, end: f64, floor: f64) -> Option<DecayEstimate> {
    let values: Vec<_> = points
        .iter()
        .filter(|p| p.center_seconds >= start && p.center_seconds <= end)
        .filter_map(|p| {
            p.rms_dbfs
                .filter(|db| *db > floor)
                .map(|db| (p.center_seconds, db))
        })
        .collect();
    if values.len() < 10 {
        return None;
    }
    let x_mean = values.iter().map(|v| v.0).sum::<f64>() / values.len() as f64;
    let y_mean = values.iter().map(|v| v.1).sum::<f64>() / values.len() as f64;
    let xx = values.iter().map(|v| (v.0 - x_mean).powi(2)).sum::<f64>();
    let yy = values.iter().map(|v| (v.1 - y_mean).powi(2)).sum::<f64>();
    let xy = values
        .iter()
        .map(|v| (v.0 - x_mean) * (v.1 - y_mean))
        .sum::<f64>();
    if xx < 1e-12 || yy < 1e-12 {
        return None;
    }
    let slope = xy / xx;
    let r_squared = (xy * xy / (xx * yy)).clamp(0.0, 1.0);
    let first = values.first()?.0;
    let last = values.last()?.0;
    let drop = -slope * (last - first);
    Some(DecayEstimate {
        start_seconds: first,
        end_seconds: last,
        slope_db_per_second: slope,
        r_squared,
        fitted_drop_db: drop,
        extrapolated_t60_seconds: (slope < -1e-6
            && r_squared >= 0.95
            && drop >= 5.0
            && last - first >= 0.1)
            .then(|| -60.0 / slope),
        points: values.len(),
    })
}
