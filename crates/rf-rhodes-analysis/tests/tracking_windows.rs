use core::f64::consts::TAU;
use rf_rhodes_analysis::{AnalysisOptions, AudioClip, analyze};

fn clip(rate: u32, seconds: f64, sample: impl Fn(f64) -> f64) -> AudioClip {
    AudioClip::from_samples(
        rate,
        (0..(rate as f64 * seconds) as usize)
            .map(|i| sample(i as f64 / rate as f64))
            .collect(),
    )
    .unwrap()
}

#[test]
fn long_observations_resolve_close_tones_without_changing_harmonic_summaries() {
    let audio = clip(16_000, 2.5, |t| {
        0.3 * (TAU * 700.3 * t).sin() + 0.3 * (TAU * 705.7 * t).sin()
    });
    let short = analyze(&audio, AnalysisOptions::default()).unwrap();
    let long = analyze(
        &audio,
        AnalysisOptions {
            partial_window_ms: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    let tracking = &long.inharmonic_tracking;
    assert_eq!(tracking.minimum_separation_hz, 1.953125);
    assert_eq!(short.inharmonic_tracking.minimum_separation_hz, 15.625);
    for frequency in [700.3, 705.7] {
        let track = tracking
            .tracks
            .iter()
            .find(|track| {
                track
                    .observations
                    .iter()
                    .all(|p| (p.frequency_hz - frequency).abs() < 0.04)
            })
            .expect("resolved close tone");
        assert_eq!(track.observations.len(), tracking.frames.len());
        assert!(track.observations.iter().all(|p| !p.ambiguous_neighbor));
    }
    assert_eq!(long.partial_window_seconds, short.partial_window_seconds);
    for (a, b) in long.partial_tracks.iter().zip(&short.partial_tracks) {
        assert_eq!(a.center_seconds, b.center_seconds);
        for (a, b) in a.harmonics.iter().zip(&b.harmonics) {
            assert_eq!(
                a.peak.as_ref().map(|p| p.frequency_hz),
                b.peak.as_ref().map(|p| p.frequency_hz)
            );
            assert_eq!(
                a.peak.as_ref().map(|p| p.amplitude_dbfs),
                b.peak.as_ref().map(|p| p.amplitude_dbfs)
            );
        }
    }
}

#[test]
fn high_rate_long_window_uses_samples_beyond_the_old_fft_cap() {
    // The first 32768 samples are silent. A silently truncated FFT would miss the tone.
    let audio = clip(192_000, 1.024, |t| {
        if t < 0.3 {
            0.0
        } else {
            0.5 * (TAU * 731.23 * t).sin()
        }
    });
    let report = analyze(
        &audio,
        AnalysisOptions {
            partial_window_ms: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    let tracking = &report.inharmonic_tracking;
    assert_eq!(tracking.observed_samples, 196608);
    assert_eq!(tracking.fft_size, 524288);
    assert_eq!(tracking.frames.len(), 1);
    assert_eq!(tracking.window_seconds, 1.024);
    assert_eq!(tracking.observation_resolution_hz, 192000.0 / 196608.0);
    assert_eq!(tracking.bin_spacing_hz, 192000.0 / 524288.0);
    assert!(
        tracking
            .tracks
            .iter()
            .any(|t| (t.observations[0].frequency_hz - 731.23).abs() < 0.1)
    );
}

#[test]
fn short_observations_show_a_brief_attack_in_a_short_recording() {
    let audio = clip(48_000, 0.096, |t| {
        0.4 * (TAU * 1378.72 * t).sin() * (-t / 0.015).exp()
    });
    let default = analyze(&audio, AnalysisOptions::default()).unwrap();
    assert!(default.inharmonic_tracking.frames.is_empty());
    let short = analyze(
        &audio,
        AnalysisOptions {
            partial_window_ms: 32,
            sustain_end_seconds: Some(0.09),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(short.inharmonic_tracking.frames.len(), 9);
    assert_eq!(short.inharmonic_tracking.minimum_separation_hz, 62.5);
    assert!(
        short
            .inharmonic_tracking
            .tracks
            .iter()
            .any(|t| (t.observations[0].frequency_hz - 1378.72).abs() < 1.0)
    );
    assert!(
        short
            .inharmonic_tracking
            .tracks
            .iter()
            .all(|t| t.decay.estimate.is_none())
    );
}

#[test]
fn long_window_decay_excludes_release_and_preserves_known_slope() {
    let audio = clip(8_000, 5.0, |t| {
        if t >= 4.7 {
            0.0
        } else {
            0.4 * (TAU * 731.23 * t).sin() * (-1000.0_f64.ln() * t / 8.0).exp()
        }
    });
    for window in [512, 1024] {
        let report = analyze(
            &audio,
            AnalysisOptions {
                partial_window_ms: window,
                sustain_end_seconds: Some(4.7),
                ..Default::default()
            },
        )
        .unwrap();
        let tracking = &report.inharmonic_tracking;
        let track = tracking
            .tracks
            .iter()
            .max_by_key(|t| t.observations.len())
            .unwrap();
        assert!(
            track.decay.rejection_reasons.is_empty(),
            "{:?}",
            track.decay
        );
        let decay = track.decay.estimate.as_ref().unwrap();
        assert!((decay.extrapolated_t60_seconds.unwrap() - 8.0).abs() < 0.03);
        assert!(decay.end_seconds + tracking.window_seconds / 2.0 <= 4.7);
    }
}

#[test]
fn invalid_windows_fail_and_short_files_never_shorten_the_requested_window() {
    let audio = clip(44_100, 0.2, |t| 0.4 * (TAU * 220.0 * t).sin());
    for window in [0, 1, 31, 64, 513, 2048, u32::MAX] {
        assert!(
            analyze(
                &audio,
                AnalysisOptions {
                    partial_window_ms: window,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    for window in [512, 1024] {
        let report = analyze(
            &audio,
            AnalysisOptions {
                partial_window_ms: window,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(report.inharmonic_tracking.frames.is_empty());
        assert_eq!(
            report.inharmonic_tracking.observed_samples,
            (44100.0 * window as f64 / 1000.0).round() as usize
        );
    }
}
