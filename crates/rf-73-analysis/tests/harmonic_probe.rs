use rf_73_analysis::{AudioClip, probe_second_harmonic};
use std::f64::consts::TAU;
fn signal(h2_db: Option<f64>) -> AudioClip {
    let mut state = 17_u64;
    AudioClip::from_samples(
        48000,
        (0..48000)
            .map(|i| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let noise = ((state >> 32) as f64 / u32::MAX as f64 - 0.5) * 1e-7;
                let phase = TAU * 523.5 * i as f64 / 48000.0;
                0.1 * (phase.sin()
                    + h2_db.map_or(0.0, |db| 10.0_f64.powf(db / 20.0)) * (2.0 * phase).sin())
                    + noise
            })
            .collect(),
    )
    .unwrap()
}
#[test]
fn weak_harmonic_is_supported_below_legacy_cutoff() {
    let p = probe_second_harmonic(&signal(Some(-64.0)), 0.25, 0.35, 523.5).unwrap();
    assert!(p.supported);
    assert!(!p.legacy_detected);
    assert!((p.relative_to_h1_db.unwrap() + 64.0).abs() < 0.1);
}
#[test]
fn fundamental_leakage_is_not_supported() {
    for duration in [0.08, 0.096, 0.112, 0.30, 0.35, 0.40] {
        assert!(
            !probe_second_harmonic(&signal(None), 0.005, duration, 523.5)
                .unwrap()
                .supported
        );
    }
}
#[test]
fn incomplete_and_invalid_windows_are_rejected() {
    let clip = signal(Some(-50.0));
    assert!(probe_second_harmonic(&clip, 0.8, 0.35, 523.5).is_err());
    assert!(probe_second_harmonic(&clip, f64::NAN, 0.35, 523.5).is_err());
    assert!(probe_second_harmonic(&clip, 0.0, 0.01, 523.5).is_err());
}
