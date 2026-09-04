use core::f64::consts::TAU;
use rf_rhodes_analysis::{AnalysisOptions, AudioClip, analyze, compare};

fn tone(rate: u32, seconds: f64, frequency: f64, amplitude: f64, t60: Option<f64>) -> AudioClip {
    AudioClip::from_samples(
        rate,
        (0..(rate as f64 * seconds) as usize)
            .map(|i| {
                let time = i as f64 / rate as f64;
                amplitude
                    * (TAU * frequency * time).sin()
                    * t60.map_or(1.0, |t| (-1000.0_f64.ln() * time / t).exp())
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn recovers_known_pitch_level_and_decay_without_inventing_t60() {
    let clip = tone(48_000, 2.0, 220.0, 0.5, Some(5.0));
    let report = analyze(
        &clip,
        AnalysisOptions {
            sustain_end_seconds: Some(1.8),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(report.tuning_error_cents.unwrap().abs() < 0.5);
    assert!((report.peak_dbfs.unwrap() + 6.0206).abs() < 0.05);
    let decay = report.decay.unwrap();
    assert!((decay.slope_db_per_second + 12.0).abs() < 0.02);
    assert!(decay.r_squared > 0.999);
    assert!((decay.extrapolated_t60_seconds.unwrap() - 5.0).abs() < 0.01);
    assert!(
        analyze(&clip, AnalysisOptions::default())
            .unwrap()
            .decay
            .is_none()
    );
}

#[test]
fn silence_and_missing_fundamental_remain_explicitly_unknown() {
    let silence = AudioClip::from_samples(48_000, vec![0.0; 4800]).unwrap();
    let report = analyze(&silence, AnalysisOptions::default()).unwrap();
    assert!(report.peak_dbfs.is_none());
    assert!(report.fundamental.is_none());
    assert!(report.onset_seconds.is_none());
    assert!(report.decay.is_none());
    let overtone = tone(48_000, 0.8, 440.0, 0.5, None);
    assert!(
        analyze(&overtone, AnalysisOptions::default())
            .unwrap()
            .fundamental
            .is_none()
    );
}

#[test]
fn short_constant_or_rising_audio_does_not_get_a_false_decay_time() {
    let constant = tone(48_000, 0.5, 220.0, 0.5, None);
    let report = analyze(
        &constant,
        AnalysisOptions {
            sustain_end_seconds: Some(0.45),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        report
            .decay
            .and_then(|d| d.extrapolated_t60_seconds)
            .is_none()
    );
    let ramp = AudioClip::from_samples(
        48_000,
        (0..24_000)
            .map(|i| (TAU * 220.0 * i as f64 / 48_000.0).sin() * (i as f64 / 24_000.0 + 0.1))
            .collect(),
    )
    .unwrap();
    assert!(
        analyze(
            &ramp,
            AnalysisOptions {
                sustain_end_seconds: Some(0.49),
                ..Default::default()
            }
        )
        .unwrap()
        .decay
        .and_then(|d| d.extrapolated_t60_seconds)
        .is_none()
    );
    assert!(
        analyze(
            &constant,
            AnalysisOptions {
                sustain_end_seconds: Some(0.6),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn identity_and_gain_comparisons_preserve_the_original_level_difference() {
    let reference = tone(48_000, 0.5, 220.0, 0.5, None);
    let same = compare(&reference, &reference, 20.0).unwrap();
    assert_eq!(same.candidate_delay_samples, 0);
    assert_eq!(same.raw_normalized_rmse, Some(0.0));
    assert!(same.log_spectral_distance_db.unwrap() < 1e-12);
    let quieter = tone(48_000, 0.5, 220.0, 0.25, None);
    let report = compare(&reference, &quieter, 0.0).unwrap();
    assert!((report.candidate_level_minus_reference_db.unwrap() + 6.0206).abs() < 1e-5);
    assert_eq!(report.candidate_gain_to_match_reference, Some(2.0));
    assert_eq!(report.raw_normalized_rmse, Some(0.5));
    assert_eq!(report.level_matched_normalized_rmse, Some(0.0));
    assert!(report.level_matched_log_spectral_distance_db.unwrap() < 1e-10);
}

#[test]
fn alignment_finds_known_broadband_delay_and_does_not_change_polarity() {
    let mut seed = 123456_u32;
    let samples: Vec<_> = (0..24_000)
        .map(|i| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f64 / u32::MAX as f64 - 0.5) * (-i as f64 / 8000.0).exp()
        })
        .collect();
    let reference = AudioClip::from_samples(48_000, samples.clone()).unwrap();
    for delay in [137, 511, 959] {
        let mut shifted = vec![0.0; delay];
        shifted.extend(samples.iter().map(|x| x * 0.5));
        let candidate = AudioClip::from_samples(48_000, shifted).unwrap();
        let report = compare(&reference, &candidate, 20.0).unwrap();
        assert_eq!(report.candidate_delay_samples, delay as i32);
        assert_eq!(report.level_matched_normalized_rmse, Some(0.0));
        let reversed = compare(&candidate, &reference, 20.0).unwrap();
        assert_eq!(reversed.candidate_delay_samples, -(delay as i32));
    }
    let inverted = AudioClip::from_samples(48_000, samples.iter().map(|x| -x).collect()).unwrap();
    assert_eq!(
        compare(&reference, &inverted, 0.0)
            .unwrap()
            .raw_normalized_rmse,
        Some(2.0)
    );
}

#[test]
fn comparison_rejects_rate_mismatch_and_undefined_values() {
    let a = tone(48_000, 0.1, 220.0, 0.5, None);
    let b = tone(44_100, 0.1, 220.0, 0.5, None);
    assert!(compare(&a, &b, 20.0).is_err());
    assert!(compare(&a, &a, f64::NAN).is_err());
    let silent = AudioClip::from_samples(48_000, vec![0.0; 4800]).unwrap();
    let report = compare(&silent, &a, 20.0).unwrap();
    assert!(report.raw_normalized_rmse.is_none());
    assert!(report.candidate_gain_to_match_reference.is_none());
}

#[test]
fn invalid_clips_fail_before_analysis() {
    for value in [f64::NAN, f64::INFINITY, 1e7] {
        assert!(AudioClip::from_samples(48_000, vec![value; 128]).is_err());
    }
    assert!(AudioClip::from_samples(0, vec![0.0; 128]).is_err());
    assert!(AudioClip::from_samples(48_000, vec![]).is_err());
}

#[test]
fn leading_silence_does_not_hide_spectral_differences() {
    let a = tone(48_000, 0.5, 220.0, 0.5, None);
    let b = tone(48_000, 0.5, 330.0, 0.5, None);
    let padded = |clip: AudioClip| {
        let mut samples = vec![0.0; 48_000];
        samples.extend_from_slice(clip.samples());
        AudioClip::from_samples(48_000, samples).unwrap()
    };
    let report = compare(&padded(a), &padded(b), 0.0).unwrap();
    assert!(report.spectral_start_seconds >= 1.0);
    assert!(report.log_spectral_distance_db.unwrap() > 1.0);
}

#[test]
fn decay_windows_do_not_cross_the_declared_key_release() {
    let clip = tone(48_000, 2.0, 220.0, 0.5, Some(5.0));
    let mut released = clip.samples().to_vec();
    released[48_000..].fill(0.0);
    let released = AudioClip::from_samples(48_000, released).unwrap();
    let report = analyze(
        &released,
        AnalysisOptions {
            sustain_end_seconds: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    let decay = report.decay.unwrap();
    assert!(decay.end_seconds <= 0.99);
    assert!((decay.extrapolated_t60_seconds.unwrap() - 5.0).abs() < 0.01);
}
