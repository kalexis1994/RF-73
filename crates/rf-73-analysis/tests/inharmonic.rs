use core::f64::consts::TAU;
use rf_73_analysis::{Analysis, AnalysisOptions, AudioClip, DecayRejection, PartialTrack, analyze};

fn measure(
    rate: u32,
    seconds: f64,
    end: Option<f64>,
    mut sample: impl FnMut(f64) -> f64,
) -> Analysis {
    let clip = AudioClip::from_samples(
        rate,
        (0..(seconds * rate as f64) as usize)
            .map(|i| sample(i as f64 / rate as f64))
            .collect(),
    )
    .unwrap();
    analyze(
        &clip,
        AnalysisOptions {
            note: 57,
            sustain_end_seconds: end,
            ..Default::default()
        },
    )
    .unwrap()
}

fn oscillation(frequency: f64, t: f64, t60: f64) -> f64 {
    (TAU * frequency * t).sin() * (-1000.0_f64.ln() * t / t60).exp()
}

fn longest_near(report: &Analysis, frequency: f64) -> &PartialTrack {
    report
        .inharmonic_tracking
        .tracks
        .iter()
        .filter(|t| (t.observations[0].frequency_hz - frequency).abs() < 2.0)
        .max_by_key(|t| t.observations.len())
        .expect("expected spectral track")
}

#[test]
fn recovers_noninteger_partials_and_independent_decay_across_rates() {
    for rate in [8_000, 44_100, 48_000, 192_000] {
        let report = measure(rate, 1.6, Some(1.55), |t| {
            0.4 * oscillation(220.37, t, 5.0) + 0.16 * oscillation(731.23, t, 2.5)
        });
        assert_eq!(report.schema_version, 2);
        for (frequency, t60) in [(220.37, 5.0), (731.23, 2.5)] {
            let track = longest_near(&report, frequency);
            assert!(track.observations.len() > 25);
            assert!(
                track
                    .observations
                    .iter()
                    .all(|p| (p.frequency_hz - frequency).abs() < 0.15)
            );
            let decay = &track.decay;
            assert!(
                decay.rejection_reasons.is_empty(),
                "rate={rate} f={frequency}: {decay:?}"
            );
            let estimate = decay.estimate.as_ref().unwrap();
            assert!((estimate.extrapolated_t60_seconds.unwrap() - t60).abs() < 0.04);
        }
        // A real inharmonic track must not inherit the nearest harmonic's label.
        assert!(report.partial_tracks[0].harmonics.iter().all(|h| {
            h.peak
                .as_ref()
                .is_none_or(|p| (p.frequency_hz - 731.23).abs() > 10.0)
        }));
    }
}

#[test]
fn isolated_off_bin_tone_does_not_turn_hann_sidelobes_into_tracks() {
    let report = measure(16_000, 0.8, None, |t| 0.5 * (TAU * 731.23 * t).sin());
    assert_eq!(
        report.inharmonic_tracking.tracks.len(),
        1,
        "{:?}",
        report.inharmonic_tracking.tracks
    );
    assert_eq!(
        longest_near(&report, 731.23).decay.rejection_reasons,
        vec![DecayRejection::SustainBoundaryRequired]
    );
}

#[test]
fn noise_and_silence_do_not_receive_persistent_modes_or_decay() {
    let mut state = 0x12345678_u32;
    let noise = measure(16_000, 1.5, Some(1.4), |_| {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        0.05 * (state as f64 / u32::MAX as f64 - 0.5)
    });
    assert!(
        noise
            .inharmonic_tracking
            .tracks
            .iter()
            .all(|t| t.observations.len() < 3)
    );
    assert!(
        noise
            .inharmonic_tracking
            .tracks
            .iter()
            .all(|t| t.decay.estimate.is_none())
    );
    for value in [0.0, 0.2] {
        let silent = measure(16_000, 0.5, None, |_| value);
        assert!(silent.inharmonic_tracking.tracks.is_empty());
    }
}

#[test]
fn a_partial_fading_into_noise_stops_instead_of_following_the_floor() {
    let mut state = 12345_u32;
    let report = measure(16_000, 2.5, Some(2.4), |t| {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        0.3 * oscillation(731.23, t, 1.8) + 0.01 * (state as f64 / u32::MAX as f64 - 0.5)
    });
    let track = longest_near(&report, 731.23);
    assert!(track.observations.last().unwrap().center_seconds < 1.7);
    assert!(
        track
            .observations
            .iter()
            .all(|p| p.margin_above_background_db >= 18.0)
    );
    if let Some(t60) = track
        .decay
        .estimate
        .as_ref()
        .and_then(|d| d.extrapolated_t60_seconds)
    {
        assert!((t60 - 1.8).abs() < 0.15);
    }
}

#[test]
fn resolved_neighbors_keep_separate_frequencies_and_unresolved_beating_is_unqualified() {
    let separated = measure(16_000, 1.5, Some(1.45), |t| {
        0.3 * oscillation(700.0, t, 4.0) + 0.3 * oscillation(735.0, t, 4.0)
    });
    assert!(longest_near(&separated, 700.0).observations.len() > 30);
    assert!(longest_near(&separated, 735.0).observations.len() > 30);
    let unresolved = measure(16_000, 1.5, Some(1.45), |t| {
        0.3 * (TAU * 700.0 * t).sin() + 0.3 * (TAU * 702.0 * t).sin()
    });
    assert!(unresolved.inharmonic_tracking.minimum_separation_hz > 2.0);
    assert!(
        unresolved
            .inharmonic_tracking
            .tracks
            .iter()
            .any(|t| t.observations.iter().any(|p| p.ambiguous_neighbor))
    );
    assert!(unresolved.inharmonic_tracking.tracks.iter().all(|t| {
        t.decay
            .estimate
            .as_ref()
            .is_none_or(|d| d.extrapolated_t60_seconds.is_none())
    }));
    // Away from cancellations the main peak is blended; these data cannot resolve two modes.
    assert_eq!(unresolved.inharmonic_tracking.frames[0].accepted_peaks, 1);
    assert!(
        (unresolved.inharmonic_tracking.tracks[0].observations[0].frequency_hz - 701.0).abs() < 0.2
    );
}

#[test]
fn missing_frames_split_tracks_and_short_attacks_remain_visible() {
    let report = measure(16_000, 1.6, Some(1.55), |t| {
        let gate = if t < 0.08 || t > 0.6 { 1.0 } else { 0.0 };
        gate * 0.3 * oscillation(731.23, t, 4.0)
    });
    let tracks: Vec<_> = report
        .inharmonic_tracking
        .tracks
        .iter()
        .filter(|t| (t.observations[0].frequency_hz - 731.23).abs() < 3.0)
        .collect();
    assert!(tracks.len() >= 2);
    let attack = tracks
        .iter()
        .find(|t| t.observations[0].center_seconds < 0.1)
        .unwrap();
    assert!(attack.decay.estimate.is_none());
    for track in tracks {
        assert!(
            track
                .observations
                .windows(2)
                .all(|p| p[1].frame == p[0].frame + 1)
        );
    }
}

#[test]
fn full_windows_exclude_release_from_partial_decay() {
    let report = measure(16_000, 2.0, Some(1.0), |t| {
        if t < 1.0 {
            0.4 * oscillation(731.23, t, 4.0)
        } else {
            0.0
        }
    });
    let fit = longest_near(&report, 731.23)
        .decay
        .estimate
        .as_ref()
        .unwrap();
    assert!(fit.end_seconds + report.inharmonic_tracking.window_seconds / 2.0 <= 1.0);
    assert!((fit.extrapolated_t60_seconds.unwrap() - 4.0).abs() < 0.04);
}

#[test]
fn flat_rising_and_two_stage_envelopes_reject_extrapolation() {
    for case in 0..3 {
        let report = measure(16_000, 2.0, Some(1.95), |t| {
            let envelope = match case {
                0 => 0.2,
                1 => 0.1 + 0.1 * t,
                _ => 0.4 * 10.0_f64.powf(-(20.0 * t.min(0.9) + 3.0 * (t - 0.9).max(0.0)) / 20.0),
            };
            envelope * (TAU * 731.23 * t).sin()
        });
        let decay = &longest_near(&report, 731.23).decay;
        assert!(!decay.rejection_reasons.is_empty());
        assert!(
            decay
                .estimate
                .as_ref()
                .unwrap()
                .extrapolated_t60_seconds
                .is_none()
        );
        if case == 2 {
            assert!(
                decay
                    .rejection_reasons
                    .contains(&DecayRejection::InconsistentSlopes)
            );
        }
    }
}

#[test]
fn short_clips_return_empty_tracking_with_explicit_resolution() {
    let report = measure(16_000, 0.01, None, |t| (TAU * 700.0 * t).sin());
    assert!(report.inharmonic_tracking.frames.is_empty());
    assert!(report.inharmonic_tracking.tracks.is_empty());
    assert_eq!(report.inharmonic_tracking.window_seconds, 0.128);
}
