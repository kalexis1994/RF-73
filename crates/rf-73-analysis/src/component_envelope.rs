//! Bounded, fixed-frequency complex demodulation. A component is not a mode.
use crate::{AudioClip, AudioError};
use serde::Serialize;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Serialize)]
pub struct EnvelopeOptions {
    pub frequency_hz: f64,
    /// Caller-selected observation interval, not an inferred sustain boundary.
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub window_seconds: f64,
    pub hop_seconds: f64,
    /// Independently identified neighbors. An empty list does not prove isolation.
    pub known_neighbor_frequencies_hz: Vec<f64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeRejection {
    InsufficientSpan,
    KnownUnresolvedNeighbor,
    LowMargin,
    FullScaleSamples,
    InsufficientDecay,
    NonExponentialAmplitude,
    InconsistentSlopes,
    FrequencyMismatch,
    UnstablePhase,
}

#[derive(Serialize)]
pub struct ComponentEnvelopePoint {
    pub center_seconds: f64,
    /// Peak coefficient after removing the supplied carrier frequency globally.
    pub coefficient: [f64; 2],
    pub amplitude_dbfs: f64,
    /// Median of ten guard projections, not a calibrated broadband noise RMS.
    pub guard_background_dbfs: f64,
    pub margin_db: f64,
    pub unwrapped_phase_radians: f64,
}

#[derive(Debug, Serialize)]
pub struct ComponentEnvelopeFit {
    pub amplitude_decay_per_second: f64,
    pub fitted_drop_db: f64,
    pub amplitude_residual_rms_db: f64,
    pub early_decay_per_second: f64,
    pub late_decay_per_second: f64,
    pub carrier_offset_hz: f64,
    pub phase_residual_rms_radians: f64,
}

#[derive(Serialize)]
pub struct ComponentEnvelope {
    pub schema_version: u32,
    pub method: &'static str,
    pub options: EnvelopeOptions,
    pub observed_samples: usize,
    pub actual_window_seconds: f64,
    pub actual_hop_seconds: f64,
    pub minimum_separation_hz: f64,
    pub points: Vec<ComponentEnvelopePoint>,
    /// Descriptive fit retained even on rejection. Never use without the gates.
    pub provisional_fit: Option<ComponentEnvelopeFit>,
    pub qualified: bool,
    pub rejection_reasons: Vec<EnvelopeRejection>,
    pub scope: &'static str,
}

fn line(t: &[f64], y: &[f64]) -> (f64, f64) {
    let mean_t = t.iter().sum::<f64>() / t.len() as f64;
    let mean_y = y.iter().sum::<f64>() / y.len() as f64;
    let variance = t.iter().map(|x| (x - mean_t).powi(2)).sum::<f64>();
    let slope = t
        .iter()
        .zip(y)
        .map(|(t, y)| (t - mean_t) * (y - mean_y))
        .sum::<f64>()
        / variance;
    let rms = (t
        .iter()
        .zip(y)
        .map(|(t, y)| (y - mean_y - slope * (t - mean_t)).powi(2))
        .sum::<f64>()
        / t.len() as f64)
        .sqrt();
    (slope, rms)
}

pub fn measure_component_envelope(
    clip: &AudioClip,
    options: EnvelopeOptions,
) -> Result<ComponentEnvelope, AudioError> {
    let rate = clip.metadata().sample_rate as f64;
    let values = [
        options.frequency_hz,
        options.start_seconds,
        options.end_seconds,
        options.window_seconds,
        options.hop_seconds,
    ];
    if values.iter().any(|v| !v.is_finite())
        || options.start_seconds < 0.0
        || options.end_seconds > clip.duration()
        || options.end_seconds <= options.start_seconds
        || !(0.064..=0.512).contains(&options.window_seconds)
        || options.hop_seconds < options.window_seconds / 8.0
        || options.hop_seconds > options.window_seconds
        || options.known_neighbor_frequencies_hz.len() > 32
        || options
            .known_neighbor_frequencies_hz
            .iter()
            .any(|f| !f.is_finite() || *f <= 0.0 || *f >= rate / 2.0)
    {
        return Err(AudioError(
            "invalid envelope interval, window/hop or neighbor list".into(),
        ));
    }
    let size = (options.window_seconds * rate).round() as usize;
    let hop = (options.hop_seconds * rate).round() as usize;
    let start = (options.start_seconds * rate).ceil() as usize;
    let end = (options.end_seconds * rate).floor() as usize;
    let resolution = rate / size as f64;
    if end.saturating_sub(start) < size
        || options.frequency_hz <= 9.0 * resolution
        || options.frequency_hz >= rate / 2.0 - 9.0 * resolution
    {
        return Err(AudioError(
            "envelope needs a complete window and carrier clear of DC/Nyquist guards".into(),
        ));
    }
    let frames = (end - start - size) / hop + 1;
    if frames > 128 || frames * size > 8_000_000 {
        return Err(AudioError(
            "envelope work exceeds 128 windows / 8 million observed samples".into(),
        ));
    }
    let mut reasons = Vec::new();
    if options
        .known_neighbor_frequencies_hz
        .iter()
        .any(|f| (f - options.frequency_hz).abs() < 2.0 * resolution)
    {
        reasons.push(EnvelopeRejection::KnownUnresolvedNeighbor);
    }
    if clip.samples()[start..end].iter().any(|x| x.abs() >= 1.0) {
        reasons.push(EnvelopeRejection::FullScaleSamples);
    }
    let weights: Vec<_> = (0..size)
        .map(|i| 0.5 - 0.5 * (TAU * i as f64 / (size - 1) as f64).cos())
        .collect();
    let weight_sum = weights.iter().sum::<f64>();
    let frequencies: Vec<_> = std::iter::once(options.frequency_hz)
        .chain([-1.0, 1.0].into_iter().flat_map(|sign| {
            (4..=8).map(move |k| options.frequency_hz + sign * k as f64 * resolution)
        }))
        .collect();
    let kernels: Vec<Vec<_>> = frequencies
        .iter()
        .map(|f| {
            weights
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let (s, c) = (TAU * f * i as f64 / rate).sin_cos();
                    [w * c, -w * s]
                })
                .collect()
        })
        .collect();
    let mut points: Vec<ComponentEnvelopePoint> = Vec::with_capacity(frames);
    for frame in 0..frames {
        let offset = start + frame * hop;
        let samples = &clip.samples()[offset..offset + size];
        let mean = samples
            .iter()
            .zip(&weights)
            .map(|(x, w)| x * w)
            .sum::<f64>()
            / weight_sum;
        let coefficients: Vec<[f64; 2]> = kernels
            .iter()
            .map(|kernel| {
                let mut c = [0.0; 2];
                for (x, k) in samples.iter().zip(kernel) {
                    c[0] += (x - mean) * k[0];
                    c[1] += (x - mean) * k[1];
                }
                [2.0 * c[0] / weight_sum, 2.0 * c[1] / weight_sum]
            })
            .collect();
        let mut guards: Vec<_> = coefficients[1..].iter().map(|c| c[0].hypot(c[1])).collect();
        guards.sort_by(f64::total_cmp);
        let background = ((guards[4] + guards[5]) * 0.5).max(1e-12);
        let amplitude = coefficients[0][0].hypot(coefficients[0][1]).max(1e-12);
        let (s, c) = (TAU * options.frequency_hz * offset as f64 / rate).sin_cos();
        let [re, im] = coefficients[0];
        let coefficient = [re * c + im * s, im * c - re * s];
        let mut phase = coefficient[1].atan2(coefficient[0]);
        if let Some(previous) = points.last() {
            let delta = (phase - previous.unwrapped_phase_radians + PI).rem_euclid(TAU) - PI;
            phase = previous.unwrapped_phase_radians + delta;
        }
        points.push(ComponentEnvelopePoint {
            center_seconds: (offset as f64 + (size - 1) as f64 * 0.5) / rate,
            coefficient,
            amplitude_dbfs: 20.0 * amplitude.log10(),
            guard_background_dbfs: 20.0 * background.log10(),
            margin_db: 20.0 * (amplitude / background).log10(),
            unwrapped_phase_radians: phase,
        });
    }
    if points
        .iter()
        .any(|p| p.margin_db < 18.0 || p.amplitude_dbfs < -180.0)
    {
        reasons.push(EnvelopeRejection::LowMargin);
    }
    let span = points.last().unwrap().center_seconds - points[0].center_seconds;
    let fit = if points.len() < 8 || span < 0.4 {
        reasons.push(EnvelopeRejection::InsufficientSpan);
        None
    } else {
        let times: Vec<_> = points.iter().map(|p| p.center_seconds).collect();
        let levels: Vec<_> = points.iter().map(|p| p.amplitude_dbfs).collect();
        let phases: Vec<_> = points.iter().map(|p| p.unwrapped_phase_radians).collect();
        let (slope, residual) = line(&times, &levels);
        let (phase_slope, phase_residual) = line(&times, &phases);
        let half = points.len() / 2;
        let rate_scale = -std::f64::consts::LN_10 / 20.0;
        let early = rate_scale * line(&times[..half], &levels[..half]).0;
        let late = rate_scale * line(&times[half..], &levels[half..]).0;
        let decay = rate_scale * slope;
        if -slope * span < 3.0 {
            reasons.push(EnvelopeRejection::InsufficientDecay);
        }
        if residual > 0.5 {
            reasons.push(EnvelopeRejection::NonExponentialAmplitude);
        }
        if (early - late).abs() > 0.2_f64.max(0.1 * decay.abs()) {
            reasons.push(EnvelopeRejection::InconsistentSlopes);
        }
        if (phase_slope / TAU).abs() > 0.5 {
            reasons.push(EnvelopeRejection::FrequencyMismatch);
        }
        if phase_residual > 0.05 {
            reasons.push(EnvelopeRejection::UnstablePhase);
        }
        Some(ComponentEnvelopeFit {
            amplitude_decay_per_second: decay,
            fitted_drop_db: -slope * span,
            amplitude_residual_rms_db: residual,
            early_decay_per_second: early,
            late_decay_per_second: late,
            carrier_offset_hz: phase_slope / TAU,
            phase_residual_rms_radians: phase_residual,
        })
    };
    Ok(ComponentEnvelope {
        schema_version: 1,
        method: "fixed-carrier-hann-envelope-v1",
        options,
        observed_samples: size,
        actual_window_seconds: size as f64 / rate,
        actual_hop_seconds: hop as f64 / rate,
        minimum_separation_hz: 2.0 * resolution,
        points,
        provisional_fit: fit,
        qualified: reasons.is_empty(),
        rejection_reasons: reasons,
        scope: "Conditional component-envelope observation, not mechanical modal identity or natural-decay calibration. Caller supplies frequency, interval and known neighbors. Unknown unresolved mixtures can evade the gates. Guard median is a local spectral heuristic, not measured broadband SNR. Overlapping windows are correlated; no confidence interval or T60 extrapolation. Finite Hann windows bias absolute amplitude and phase; rate and carrier slope are validated on controlled probes.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> EnvelopeOptions {
        EnvelopeOptions {
            frequency_hz: 1426.7578125,
            start_seconds: 0.1,
            end_seconds: 1.5,
            window_seconds: 0.128,
            hop_seconds: 0.032,
            known_neighbor_frequencies_hz: vec![],
        }
    }
    fn clip(rate: u32, decay: f64, shift: f64) -> AudioClip {
        AudioClip::from_samples(
            rate,
            (0..rate * 2)
                .map(|i| {
                    let t = i as f64 / rate as f64;
                    0.1 + 0.25
                        * (-decay * t).exp()
                        * (TAU * (1426.7578125 + shift) * t + 0.73).cos()
                })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn recovers_decay_and_carrier_offset_at_native_rates() {
        for rate in [44100, 48000, 96000] {
            let r = measure_component_envelope(&clip(rate, 3.0, 0.2), options()).unwrap();
            assert!(r.qualified, "{:?}", r.rejection_reasons);
            let fit = r.provisional_fit.unwrap();
            assert!((fit.amplitude_decay_per_second - 3.0).abs() < 0.001);
            assert!((fit.carrier_offset_hz - 0.2).abs() < 0.001);
            assert!(fit.phase_residual_rms_radians < 0.001);
        }
    }
    #[test]
    fn rejects_known_overlap_stationary_carrier_error_and_silence() {
        let mut opt = options();
        opt.known_neighbor_frequencies_hz
            .push(opt.frequency_hz + 4.0);
        assert!(
            measure_component_envelope(&clip(48000, 3.0, 0.0), opt)
                .unwrap()
                .rejection_reasons
                .contains(&EnvelopeRejection::KnownUnresolvedNeighbor)
        );
        let steady = measure_component_envelope(&clip(48000, 0.0, 0.0), options()).unwrap();
        assert!(
            steady
                .rejection_reasons
                .contains(&EnvelopeRejection::InsufficientDecay)
        );
        let shifted = measure_component_envelope(&clip(48000, 3.0, 2.0), options()).unwrap();
        assert!(
            shifted
                .rejection_reasons
                .contains(&EnvelopeRejection::FrequencyMismatch)
        );
        let silent = AudioClip::from_samples(48000, vec![0.0; 96000]).unwrap();
        let r = measure_component_envelope(&silent, options()).unwrap();
        assert!(r.rejection_reasons.contains(&EnvelopeRejection::LowMargin));
        assert!(!r.qualified);
    }
    #[test]
    fn absolute_phase_is_preserved_and_gain_does_not_change_decay() {
        let source = clip(48000, 3.0, 0.0);
        let quiet =
            AudioClip::from_samples(48000, source.samples().iter().map(|x| x * 0.1).collect())
                .unwrap();
        let a = measure_component_envelope(&source, options()).unwrap();
        let b = measure_component_envelope(&quiet, options()).unwrap();
        for (x, y) in a.points.iter().zip(&b.points) {
            assert!((x.unwrapped_phase_radians - 0.73).abs() < 0.001);
            assert!((x.amplitude_dbfs - y.amplitude_dbfs - 20.0).abs() < 1e-8);
        }
        assert!(
            (a.provisional_fit.unwrap().amplitude_decay_per_second
                - b.provisional_fit.unwrap().amplitude_decay_per_second)
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn retains_rejection_for_full_scale_and_short_observation() {
        let source = clip(48000, 3.0, 0.0);
        let loud =
            AudioClip::from_samples(48000, source.samples().iter().map(|x| x * 8.0).collect())
                .unwrap();
        let result = measure_component_envelope(&loud, options()).unwrap();
        assert!(
            result
                .rejection_reasons
                .contains(&EnvelopeRejection::FullScaleSamples)
        );
        let mut short = options();
        short.end_seconds = 0.35;
        let result = measure_component_envelope(&source, short).unwrap();
        assert!(
            result
                .rejection_reasons
                .contains(&EnvelopeRejection::InsufficientSpan)
        );
        assert!(result.provisional_fit.is_none());
    }

    #[test]
    fn qualification_does_not_prove_absence_of_undeclared_close_components() {
        let o = options();
        let source = AudioClip::from_samples(
            48000,
            (0..96000)
                .map(|i| {
                    let t = i as f64 / 48000.0;
                    0.2 * (-3.0 * t).exp()
                        * ((TAU * o.frequency_hz * t + 0.73).cos()
                            + 0.5 * (TAU * (o.frequency_hz + 0.05) * t + 0.73).cos())
                })
                .collect(),
        )
        .unwrap();
        let result = measure_component_envelope(&source, o.clone()).unwrap();
        assert!(result.qualified, "{:?}", result.rejection_reasons);
        let mut declared = o;
        declared
            .known_neighbor_frequencies_hz
            .push(declared.frequency_hz + 0.05);
        let result = measure_component_envelope(&source, declared).unwrap();
        assert!(
            result
                .rejection_reasons
                .contains(&EnvelopeRejection::KnownUnresolvedNeighbor)
        );
    }
    #[test]
    fn rejects_invalid_and_excessive_work_without_truncation() {
        let source = clip(48000, 3.0, 0.0);
        for (field, value) in [(0, f64::NAN), (1, -0.1), (2, 3.0), (3, 0.0), (4, 0.001)] {
            let mut o = options();
            match field {
                0 => o.frequency_hz = value,
                1 => o.start_seconds = value,
                2 => o.end_seconds = value,
                3 => o.window_seconds = value,
                _ => o.hop_seconds = value,
            }
            assert!(measure_component_envelope(&source, o).is_err());
        }
        let mut o = options();
        o.frequency_hz = 20.0;
        assert!(measure_component_envelope(&source, o).is_err());
        let mut o = options();
        o.window_seconds = 0.064;
        o.hop_seconds = 0.008;
        assert!(measure_component_envelope(&source, o).is_err());
        let mut o = options();
        o.known_neighbor_frequencies_hz = vec![1000.0; 33];
        assert!(measure_component_envelope(&source, o).is_err());
        let mut o = options();
        o.end_seconds = 0.2;
        assert!(measure_component_envelope(&source, o).is_err());
    }
}
