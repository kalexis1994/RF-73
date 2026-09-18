//! Offline symmetric FIR isolation with explicit real-data support.
use crate::{AudioClip, AudioError, ShortEnvelope, ShortEnvelopeOptions, measure_short_envelope};
use serde::Serialize;
use std::f64::consts::{PI, TAU};

#[derive(Serialize)]
pub struct BandEnvelopeFilter {
    pub method: &'static str,
    pub center_hz: f64,
    pub half_bandwidth_hz: f64,
    pub half_support_samples: usize,
    pub kernel_samples: usize,
    pub source_start_sample: usize,
    pub source_end_sample_exclusive: usize,
    pub filtered_start_sample: usize,
    pub filtered_end_sample_exclusive: usize,
    pub multiply_accumulate_count: usize,
    pub carrier_stationary_gains: Vec<f64>,
}

#[derive(Serialize)]
pub struct BandEnvelope {
    pub schema_version: u32,
    pub filter: BandEnvelopeFilter,
    /// Times and demodulated phases refer to the original clip, not the crop.
    pub measurement: ShortEnvelope,
    pub scope: &'static str,
}

const HALF_BANDWIDTH: f64 = 300.0;
const HALF_SUPPORT_SECONDS: f64 = 0.016;
const MAX_WORK: usize = 100_000_000;

fn response(kernel: &[f64], frequency: f64, rate: f64) -> f64 {
    let half = kernel.len() / 2;
    kernel
        .iter()
        .enumerate()
        .map(|(i, h)| h * (TAU * frequency * (i as f64 - half as f64) / rate).cos())
        .sum()
}

fn kernel(center: f64, rate: f64, half: usize) -> Vec<f64> {
    let mut taps: Vec<_> = (0..=2 * half)
        .map(|i| {
            let n = i as f64 - half as f64;
            let lowpass = if n == 0.0 {
                2.0 * HALF_BANDWIDTH / rate
            } else {
                (TAU * HALF_BANDWIDTH * n / rate).sin() / (PI * n)
            };
            let window =
                0.42 + 0.5 * (PI * n / half as f64).cos() + 0.08 * (TAU * n / half as f64).cos();
            2.0 * lowpass * (TAU * center * n / rate).cos() * window
        })
        .collect();
    let gain = response(&taps, center, rate);
    for tap in &mut taps {
        *tap /= gain;
    }
    taps
}

/// Fixed 32 ms Blackman-windowed bandpass, followed by unchanged short regression.
/// Requires the whole source interval plus 16 ms of real samples on each side.
pub fn measure_band_envelope(
    clip: &AudioClip,
    options: ShortEnvelopeOptions,
) -> Result<BandEnvelope, AudioError> {
    let rate = clip.metadata().sample_rate as f64;
    let Some(&center) = options.frequencies_hz.first() else {
        return Err(AudioError("band envelope requires a target carrier".into()));
    };
    // Validate bounded work before allocating taps or invoking the estimator.
    if !center.is_finite()
        || center <= 600.0
        || center + 600.0 >= 0.45 * rate
        || !(1..=3).contains(&options.frequencies_hz.len())
        || options.frequencies_hz.iter().enumerate().any(|(i, f)| {
            !f.is_finite() || (*f - center).abs() > 200.0 || options.frequencies_hz[..i].contains(f)
        })
        || !options.start_seconds.is_finite()
        || !options.end_seconds.is_finite()
        || options.start_seconds < 0.0
        || options.end_seconds > clip.duration()
        || options.end_seconds <= options.start_seconds
        || options.end_seconds - options.start_seconds > 0.3
        || !(0.016..=0.064).contains(&options.window_seconds)
        || !options.hop_seconds.is_finite()
        || options.hop_seconds < options.window_seconds / 8.0
        || options.hop_seconds > options.window_seconds / 2.0
    {
        return Err(AudioError(
            "invalid band-envelope interval, carriers or short-window support".into(),
        ));
    }
    let half = (rate * HALF_SUPPORT_SECONDS).round() as usize;
    let start = (rate * options.start_seconds).ceil() as usize;
    let end = (rate * options.end_seconds).floor() as usize;
    let size = (rate * options.window_seconds).round() as usize;
    let hop = (rate * options.hop_seconds).round() as usize;
    if half == 0
        || hop == 0
        || end.saturating_sub(start) < size
        || start < half
        || end
            .checked_add(half)
            .is_none_or(|n| n > clip.samples().len())
    {
        return Err(AudioError("band envelope requires complete windows and real source samples on both sides; no padding".into()));
    }
    let kernel_size = 2 * half + 1;
    let work = (end - start)
        .checked_mul(kernel_size)
        .ok_or_else(|| AudioError("band-envelope work overflow".into()))?;
    let frames = (end - start - size) / hop + 1;
    if work > MAX_WORK || frames > 64 || frames.checked_mul(size).is_none_or(|n| n > 1_000_000) {
        return Err(AudioError(
            "band-envelope work exceeds FIR or regression limit".into(),
        ));
    }
    let taps = kernel(center, rate, half);
    let source_full_scale = clip.samples()[start - half..end + half]
        .iter()
        .any(|x| x.abs() >= 1.0);
    let filtered = (start..end)
        .map(|n| {
            // Centered convolution: even taps make correlation and convolution equal.
            taps.iter()
                .zip(&clip.samples()[n - half..=n + half])
                .map(|(h, x)| h * x)
                .sum()
        })
        .collect();
    let cropped = AudioClip::from_samples(clip.metadata().sample_rate, filtered)?;
    let mut local_options = options.clone();
    local_options.start_seconds = 0.0;
    local_options.end_seconds = cropped.duration();
    let mut measurement = measure_short_envelope(&cropped, local_options)?;
    let origin = start as f64 / rate;
    let shift = -TAU * center * origin;
    let phase_delta = measurement.points.first().map_or(0.0, |p| {
        (p.unwrapped_phase_radians + shift + PI).rem_euclid(TAU) - PI - p.unwrapped_phase_radians
    });
    let (s, c) = shift.sin_cos();
    for point in &mut measurement.points {
        let [re, im] = point.coefficient;
        point.coefficient = [re * c - im * s, re * s + im * c];
        point.unwrapped_phase_radians += phase_delta;
        point.center_seconds += origin;
    }
    measurement.options = options;
    if source_full_scale {
        measurement
            .rejection_reasons
            .push("source_full_scale_in_filter_support");
        measurement.qualified = false;
    }
    Ok(BandEnvelope {
        schema_version: 1,
        filter: BandEnvelopeFilter {
            method: "centered-blackman-sinc-bandpass-v1",
            center_hz: center,
            half_bandwidth_hz: HALF_BANDWIDTH,
            half_support_samples: half,
            kernel_samples: kernel_size,
            source_start_sample: start - half,
            source_end_sample_exclusive: end + half,
            filtered_start_sample: start,
            filtered_end_sample_exclusive: end,
            multiply_accumulate_count: work,
            carrier_stationary_gains: measurement
                .options
                .frequencies_hz
                .iter()
                .map(|f| response(&taps, *f, rate))
                .collect(),
        },
        measurement,
        scope: "Offline noncausal centered FIR, no zero padding or onset inference. Integer-cropped real source support includes both halos. Output times and demodulated phase use original-file coordinates. Unit gain only at the stationary target: decaying envelopes and edges can change amplitude/phase. Filtering colors noise; downstream residual-based margin is descriptive, not a confidence interval. Unchanged short regression gates are conditional on this observation filter and declared carriers. No mechanical-loss, source-mode or realtime claim.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> ShortEnvelopeOptions {
        ShortEnvelopeOptions {
            frequencies_hz: vec![1620.0, 1568.0],
            start_seconds: 0.02,
            end_seconds: 0.18,
            window_seconds: 0.032,
            hop_seconds: 0.008,
        }
    }

    #[test]
    fn filter_is_even_normalized_and_rejects_distant_carriers_at_native_rates() {
        for rate in [44100.0, 48000.0, 96000.0] {
            let half = (rate * HALF_SUPPORT_SECONDS).round() as usize;
            let taps = kernel(1620.0, rate, half);
            assert!(
                taps.iter()
                    .zip(taps.iter().rev())
                    .all(|(a, b)| (a - b).abs() < 1e-15)
            );
            assert!((response(&taps, 1620.0, rate) - 1.0).abs() < 1e-12);
            assert!((response(&taps, 1568.0, rate) - 1.0).abs() < 0.001);
            for f in [0.0, 196.35, 392.7, 589.05, 3000.0] {
                assert!(response(&taps, f, rate).abs() < 1e-5, "rate={rate}, f={f}");
            }
        }
    }

    #[test]
    fn isolated_exponential_preserves_absolute_phase_rate_and_matches_complex_filter_gain() {
        for rate in [44100, 48000, 96000] {
            let clip = AudioClip::from_samples(
                rate,
                (0..rate / 4)
                    .map(|i| {
                        let t = i as f64 / rate as f64;
                        0.002 * (-8.0 * t).exp() * (TAU * 1620.0 * t + 0.73).cos()
                    })
                    .collect(),
            )
            .unwrap();
            let taps = kernel(
                1620.0,
                rate as f64,
                (rate as f64 * HALF_SUPPORT_SECONDS).round() as usize,
            );
            // Independent exponential eigenfunction identity for centered convolution.
            let half = taps.len() / 2;
            let gain = taps.iter().enumerate().fold([0.0, 0.0], |mut acc, (i, h)| {
                let tau = (i as f64 - half as f64) / rate as f64;
                acc[0] += h * (-8.0 * tau).exp() * (TAU * 1620.0 * tau).cos();
                acc[1] += h * (-8.0 * tau).exp() * (TAU * 1620.0 * tau).sin();
                acc
            });
            let expected_clip = AudioClip::from_samples(
                rate,
                (0..rate / 4)
                    .map(|i| {
                        let t = i as f64 / rate as f64;
                        0.002
                            * (-8.0 * t).exp()
                            * gain[0].hypot(gain[1])
                            * (TAU * 1620.0 * t + 0.73 + gain[1].atan2(gain[0])).cos()
                    })
                    .collect(),
            )
            .unwrap();
            for start in [0.02, 0.020013] {
                let mut o = options();
                o.start_seconds = start;
                let expected = measure_short_envelope(&expected_clip, o.clone()).unwrap();
                let r = measure_band_envelope(&clip, o).unwrap();
                assert!(r.measurement.qualified);
                let f = r.measurement.provisional_fit.as_ref().unwrap();
                assert!((f.amplitude_decay_per_second - 8.0).abs() < 0.001);
                assert!(f.carrier_offset_hz.abs() < 0.001);
                // Compare to the exact filtered eigenfunction through the same approximate
                // regression. Do not mistake polynomial phase bias for FIR/crop error.
                assert_eq!(r.measurement.points.len(), expected.points.len());
                for (p, expected) in r.measurement.points.iter().zip(&expected.points) {
                    assert!((p.center_seconds - expected.center_seconds).abs() < 1e-12);
                    assert!((p.amplitude_dbfs - expected.amplitude_dbfs).abs() < 1e-8);
                    assert!(
                        (p.unwrapped_phase_radians - expected.unwrapped_phase_radians).abs() < 1e-8
                    );
                    let difference = (p.coefficient[0] - expected.coefficient[0])
                        .hypot(p.coefficient[1] - expected.coefficient[1]);
                    assert!(
                        difference / expected.coefficient[0].hypot(expected.coefficient[1]) < 1e-8
                    );
                }
            }
        }
    }

    #[test]
    fn unsupported_intervals_and_carriers_fail_and_filter_cannot_hide_source_clipping() {
        let mut samples = vec![0.0; 12000];
        samples[300] = 1.0; // In the real-data halo, outside the measured interval.
        let clip = AudioClip::from_samples(48000, samples).unwrap();
        let r = measure_band_envelope(&clip, options()).unwrap();
        assert!(!r.measurement.qualified);
        assert!(
            r.measurement
                .rejection_reasons
                .contains(&"source_full_scale_in_filter_support")
        );
        for (start, end) in [(0.0, 0.18), (0.1, 0.25), (f64::NAN, 0.18), (0.02, 0.6)] {
            let mut o = options();
            o.start_seconds = start;
            o.end_seconds = end;
            assert!(measure_band_envelope(&clip, o).is_err());
        }
        for frequencies in [
            vec![],
            vec![196.0],
            vec![1620.0, 982.0],
            vec![1620.0, 1620.0],
            vec![f64::NAN],
        ] {
            let mut o = options();
            o.frequencies_hz = frequencies;
            assert!(measure_band_envelope(&clip, o).is_err());
        }
        let huge_rate = AudioClip::from_samples(192000, vec![0.0; 192000 / 4]).unwrap();
        assert!(
            measure_band_envelope(&huge_rate, options())
                .err()
                .unwrap()
                .0
                .contains("work exceeds")
        );
    }
}
