use rf_rhodes_analysis::{AudioClip, ToneComparisonOptions, compare_tone};
use std::f64::consts::TAU;

fn signal(rate: u32, h1: f64, h3: f64) -> AudioClip {
    AudioClip::from_samples(
        rate,
        (0..rate)
            .map(|i| {
                let phase = TAU * 220.0 * i as f64 / rate as f64;
                h1 * phase.sin() + h3 * (phase * 3.0).sin()
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn identity_and_constant_gain_preserve_balance_but_report_raw_gain() {
    let a = signal(48000, 0.4, 0.2);
    let b = signal(48000, 0.2, 0.1);
    let identity = compare_tone(&a, &a, Default::default()).unwrap();
    let scaled = compare_tone(&a, &b, Default::default()).unwrap();
    for (same, window) in identity.windows.iter().zip(&scaled.windows) {
        assert_eq!(
            same.harmonics[2].candidate_minus_reference_balance_db,
            Some(0.0)
        );
        assert!(
            window.harmonics[2]
                .candidate_minus_reference_balance_db
                .unwrap()
                .abs()
                < 1e-9
        );
        assert!(
            (window.harmonics[2]
                .candidate_minus_reference_raw_db
                .unwrap()
                + 6.0206)
                .abs()
                < 0.001
        );
        assert!((window.candidate_minus_reference_rms_db.unwrap() + 6.0206).abs() < 0.001);
    }
}

#[test]
fn changed_harmonic_balance_is_measured_without_filling_missing_peaks() {
    let a = signal(48000, 0.4, 0.2);
    let b = signal(48000, 0.4, 0.1);
    let report = compare_tone(&a, &b, Default::default()).unwrap();
    let body = &report.windows[2];
    assert!(
        (body.harmonics[2]
            .candidate_minus_reference_balance_db
            .unwrap()
            + 6.0206)
            .abs()
            < 0.05
    );
    let missing = compare_tone(&a, &signal(48000, 0.4, 0.0), Default::default()).unwrap();
    assert!(
        missing.windows[2].harmonics[2]
            .candidate_minus_reference_balance_db
            .is_none()
    );
    let no_h1 = compare_tone(&signal(48000, 0.0, 0.2), &b, Default::default()).unwrap();
    assert!(
        no_h1.windows[2].harmonics[2]
            .reference_relative_to_fundamental_db
            .is_none()
    );
}

#[test]
fn explicit_offsets_recover_identity_and_high_rates_keep_full_windows() {
    let a = signal(192000, 0.4, 0.2);
    let mut delayed = vec![0.0; 19200];
    delayed.extend_from_slice(a.samples());
    let b = AudioClip::from_samples(192000, delayed).unwrap();
    let report = compare_tone(
        &a,
        &b,
        ToneComparisonOptions {
            candidate_start_seconds: 0.1,
            ..Default::default()
        },
    )
    .unwrap();
    let body = &report.windows[2];
    assert_eq!(body.reference.observed_samples, 67200);
    assert_eq!(body.candidate.observed_samples, 67200);
    assert_eq!(body.reference_start_frame, 48000);
    assert_eq!(body.candidate_start_frame, 67200);
    assert_eq!(body.reference.fft_size, 262144);
    assert_eq!(
        body.harmonics[2].candidate_minus_reference_balance_db,
        Some(0.0)
    );
}

#[test]
fn insufficient_cycles_and_silence_do_not_get_normalized_harmonic_balance() {
    let a = signal(44100, 0.4, 0.1);
    let bass = compare_tone(
        &a,
        &a,
        ToneComparisonOptions {
            note: 28,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!bass.windows[0].enough_fundamental_cycles);
    assert!(
        bass.windows[0]
            .harmonics
            .iter()
            .all(|h| h.reference_relative_to_fundamental_db.is_none())
    );
    let silence = AudioClip::from_samples(44100, vec![0.0; 44100]).unwrap();
    let report = compare_tone(&silence, &silence, Default::default()).unwrap();
    assert!(report.windows.iter().all(|w| {
        w.harmonics
            .iter()
            .all(|h| h.candidate_minus_reference_balance_db.is_none())
    }));
}

#[test]
fn invalid_regions_and_rates_fail_instead_of_shortening_observations() {
    let a = signal(48000, 0.4, 0.2);
    let short = AudioClip::from_samples(48000, a.samples()[..28000].to_vec()).unwrap();
    assert!(compare_tone(&a, &short, Default::default()).is_err());
    assert!(compare_tone(&a, &signal(44100, 0.4, 0.2), Default::default()).is_err());
    for start in [-1.0, 0.5, f64::NAN, f64::INFINITY] {
        assert!(
            compare_tone(
                &a,
                &a,
                ToneComparisonOptions {
                    reference_start_seconds: start,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
