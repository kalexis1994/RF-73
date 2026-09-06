use core::f64::consts::TAU;
use rf_73_analysis::{
    AudioClip, PairedDecayStatus, PartialComparison, PartialComparisonOptions, compare_partials,
};

fn clip(seconds: f64, sample: impl Fn(f64) -> f64) -> AudioClip {
    AudioClip::from_samples(
        16_000,
        (0..(seconds * 16000.0) as usize)
            .map(|i| sample(i as f64 / 16000.0))
            .collect(),
    )
    .unwrap()
}

fn tone(frequency: f64, t: f64, t60: f64) -> f64 {
    0.4 * (TAU * frequency * t).sin() * (-1000.0_f64.ln() * t / t60).exp()
}

fn options(seconds: f64) -> PartialComparisonOptions {
    PartialComparisonOptions {
        seconds,
        ..Default::default()
    }
}

fn assert_counts(report: &PartialComparison) {
    for counts in [
        &report.reference_observations,
        &report.candidate_observations,
    ] {
        assert_eq!(
            counts.total,
            counts.matched + counts.no_counterpart + counts.ambiguous_match + counts.excluded
        );
    }
    assert_eq!(
        report.reference_observations.matched,
        report.candidate_observations.matched
    );
    assert_eq!(
        report.reference_observations.matched,
        report
            .matches
            .iter()
            .map(|p| p.paired_center_seconds.len())
            .sum::<usize>()
    );
}

#[test]
fn identity_and_gain_preserve_raw_levels_without_false_errors() {
    let a = clip(2.0, |t| tone(731.23, t, 4.0));
    for gain in [1.0, 0.5, 2.0] {
        let b = clip(2.0, |t| gain * tone(731.23, t, 4.0));
        let report = compare_partials(&a, &b, options(2.0)).unwrap();
        assert_counts(&report);
        assert_eq!(report.matches.len(), 1);
        assert!(report.detection_complete);
        let pair = &report.matches[0];
        assert_eq!(pair.mean_candidate_minus_reference_cents, 0.0);
        assert!((pair.mean_candidate_minus_reference_db - 20.0 * gain.log10()).abs() < 1e-10);
        assert!(
            pair.level_matched_mean_candidate_minus_reference_db
                .unwrap()
                .abs()
                < 1e-10
        );
        assert!(pair.level_matched_rms_level_error_db.unwrap() < 1e-5);
        assert_eq!(pair.decay.status, PairedDecayStatus::Qualified);
        assert!(
            pair.decay
                .candidate_minus_reference_t60_seconds
                .unwrap()
                .abs()
                < 1e-10
        );
    }
}

#[test]
fn signed_pitch_and_decay_differences_recover_known_changes() {
    let a = clip(2.4, |t| tone(731.23, t, 4.0));
    let b = clip(2.4, |t| tone(731.23 * 2.0_f64.powf(20.0 / 1200.0), t, 6.0));
    let report = compare_partials(&a, &b, options(2.4)).unwrap();
    assert_counts(&report);
    let pair = report
        .matches
        .iter()
        .max_by_key(|p| p.paired_center_seconds.len())
        .unwrap();
    assert!((pair.mean_candidate_minus_reference_cents - 20.0).abs() < 0.1);
    assert!((pair.rms_frequency_error_cents - 20.0).abs() < 0.1);
    assert_eq!(pair.decay.status, PairedDecayStatus::Qualified);
    assert!((pair.decay.candidate_minus_reference_t60_seconds.unwrap() - 2.0).abs() < 0.04);
    assert!(
        (pair
            .decay
            .candidate_minus_reference_slope_db_per_second
            .unwrap()
            - 5.0)
            .abs()
            < 0.05
    );
}

#[test]
fn unmatched_components_and_silence_stay_explicit() {
    let a = clip(1.5, |t| tone(220.0, t, 4.0) + tone(731.23, t, 4.0));
    let b = clip(1.5, |t| tone(220.0, t, 4.0) + tone(1100.0, t, 4.0));
    let report = compare_partials(&a, &b, options(1.5)).unwrap();
    assert_counts(&report);
    assert!(report.reference_observations.no_counterpart > 20);
    assert!(report.candidate_observations.no_counterpart > 20);
    assert!(
        report
            .matches
            .iter()
            .all(|p| (p.mean_reference_frequency_hz - 220.0).abs() < 1.0)
    );
    let silence = clip(1.5, |_| 0.0);
    let empty = compare_partials(&a, &silence, options(1.5)).unwrap();
    assert_counts(&empty);
    assert!(empty.matches.is_empty());
    assert!(empty.candidate_gain_to_match_reference.is_none());
}

#[test]
fn multiple_possible_counterparts_are_not_arbitrarily_paired() {
    let a = clip(2.5, |t| tone(700.0, t, 8.0) + tone(710.0, t, 8.0));
    let b = clip(2.5, |t| tone(705.0, t, 8.0));
    let report = compare_partials(
        &a,
        &b,
        PartialComparisonOptions {
            partial_window_ms: 1024,
            ..options(2.5)
        },
    )
    .unwrap();
    assert_counts(&report);
    assert!(report.reference_observations.ambiguous_match > 0);
    assert!(report.candidate_observations.ambiguous_match > 0);
    assert!(report.matches.is_empty());
}

#[test]
fn explicit_region_offsets_align_a_delayed_take_without_changing_audio() {
    let a = clip(3.0, |t| tone(731.23, t, 4.0));
    let mut delayed = vec![0.0; 137];
    delayed.extend_from_slice(a.samples());
    let b = AudioClip::from_samples(16000, delayed).unwrap();
    let report = compare_partials(
        &a,
        &b,
        PartialComparisonOptions {
            reference_start_seconds: 0.125,
            candidate_start_seconds: 0.125 + 137.0 / 16000.0,
            ..options(2.0)
        },
    )
    .unwrap();
    assert_counts(&report);
    assert_eq!(
        report.candidate_region.start_frame - report.reference_region.start_frame,
        137
    );
    assert!(
        report
            .matches
            .iter()
            .all(|p| p.rms_frequency_error_cents == 0.0 && p.rms_level_error_db == 0.0)
    );
    assert_eq!(a.samples().len(), 48000);
    assert_eq!(b.samples().len(), 48137);
}

#[test]
fn different_qualified_fit_intervals_do_not_get_a_decay_difference() {
    let a = clip(4.0, |t| tone(731.23, t, 4.0));
    let b = clip(4.0, |t| tone(731.23, t, 1.2));
    let report = compare_partials(&a, &b, options(4.0)).unwrap();
    let pair = report
        .matches
        .iter()
        .max_by_key(|p| p.paired_center_seconds.len())
        .unwrap();
    assert_eq!(pair.decay.status, PairedDecayStatus::DifferentFitIntervals);
    assert!(pair.decay.candidate_minus_reference_t60_seconds.is_none());
}

#[test]
fn invalid_regions_rates_and_tolerances_fail_before_tracking() {
    let a = clip(1.5, |t| tone(731.23, t, 4.0));
    for bad in [
        PartialComparisonOptions {
            seconds: f64::NAN,
            ..Default::default()
        },
        PartialComparisonOptions {
            seconds: 0.01,
            ..Default::default()
        },
        PartialComparisonOptions {
            seconds: 61.0,
            ..Default::default()
        },
        PartialComparisonOptions {
            reference_start_seconds: -0.1,
            ..Default::default()
        },
        PartialComparisonOptions {
            candidate_start_seconds: f64::INFINITY,
            ..Default::default()
        },
        PartialComparisonOptions {
            candidate_start_seconds: 0.6,
            ..Default::default()
        },
        PartialComparisonOptions {
            partial_window_ms: 64,
            ..Default::default()
        },
        PartialComparisonOptions {
            match_cents: f64::NAN,
            ..Default::default()
        },
        PartialComparisonOptions {
            match_cents: 101.0,
            ..Default::default()
        },
    ] {
        assert!(compare_partials(&a, &a, bad).is_err());
    }
    let b = AudioClip::from_samples(8000, vec![0.0; 8000]).unwrap();
    assert!(compare_partials(&a, &b, options(1.0)).is_err());
}

#[test]
fn missing_pairings_within_qualified_fits_prevent_a_decay_comparison() {
    let a = clip(2.4, |t| tone(731.23, t, 4.0));
    let b = clip(2.4, |t| {
        0.4 * (TAU * (731.23 * t + 0.15 * (TAU * 0.5 * t).sin())).sin()
            * (-1000.0_f64.ln() * t / 4.0).exp()
    });
    let report = compare_partials(
        &a,
        &b,
        PartialComparisonOptions {
            match_cents: 0.2,
            ..options(2.4)
        },
    )
    .unwrap();
    assert_counts(&report);
    let pair = report
        .matches
        .iter()
        .max_by_key(|p| p.paired_center_seconds.len())
        .unwrap();
    assert_eq!(pair.decay.status, PairedDecayStatus::IncompletePairing);
    assert!(pair.decay.candidate_minus_reference_t60_seconds.is_none());
}

#[test]
fn opposing_level_errors_do_not_cancel_in_rms() {
    let a = clip(2.0, |t| tone(731.23, t, 4.0));
    let b = clip(2.0, |t| {
        tone(731.23, t, 4.0) * 10.0_f64.powf(3.0 * (TAU * t).sin() / 20.0)
    });
    let report = compare_partials(&a, &b, options(2.0)).unwrap();
    let pair = report
        .matches
        .iter()
        .max_by_key(|p| p.paired_center_seconds.len())
        .unwrap();
    assert!(pair.mean_candidate_minus_reference_db.abs() < 0.2);
    assert!(pair.rms_level_error_db > 1.8);
    assert!(pair.level_matched_rms_level_error_db.unwrap() > 1.8);
}

#[test]
fn truncated_detection_disables_matching_instead_of_claiming_complete_correspondence() {
    let a = clip(0.5, |t| {
        (0..40)
            .map(|n| 0.005 * (TAU * (200.0 + 150.0 * n as f64) * t).sin())
            .sum()
    });
    let report = compare_partials(
        &a,
        &a,
        PartialComparisonOptions {
            partial_window_ms: 128,
            ..options(0.5)
        },
    )
    .unwrap();
    assert_counts(&report);
    assert!(!report.detection_complete);
    assert!(report.matches.is_empty());
    assert!(report.reference_observations.excluded > 0);
    assert_eq!(
        report.reference_observations.total,
        report.reference_observations.excluded
    );
}
