use rf_73_analysis::{AudioClip, pitch_anchor};
use std::f64::consts::TAU;

fn clip(rate: u32, tones: &[(f64, f64)]) -> AudioClip {
    AudioClip::from_samples(
        rate,
        (0..rate as usize * 2)
            .map(|i| {
                tones
                    .iter()
                    .map(|&(f, a)| a * (TAU * f * i as f64 / rate as f64).sin())
                    .sum()
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn broad_anchor_tracks_dominant_component_instead_of_weak_note_hint() {
    for rate in [44100, 48000, 96000] {
        let a = pitch_anchor(&clip(rate, &[(169.57, 0.1), (198.0, 0.0001)]), 55).unwrap();
        assert!(a.qualified, "{a:?}");
        assert!((a.frequency_hz.unwrap() - 169.57).abs() < 0.03);
        let scaled = pitch_anchor(&clip(rate, &[(169.57, 0.01), (198.0, 0.00001)]), 55).unwrap();
        assert!((a.frequency_hz.unwrap() - scaled.frequency_hz.unwrap()).abs() < 1e-8);
    }
}

#[test]
fn ambiguity_silence_wrong_band_and_short_audio_do_not_become_targets() {
    for tones in [
        vec![],
        vec![(196.3, 0.1), (206.0, 0.04)],
        vec![(392.6, 0.1)],
    ] {
        assert!(!pitch_anchor(&clip(48000, &tones), 55).unwrap().qualified);
    }
    let short = AudioClip::from_samples(48000, vec![0.0; 48000]).unwrap();
    assert!(pitch_anchor(&short, 55).is_err());
    assert!(pitch_anchor(&clip(48000, &[(196.3, 0.1)]), 127).is_err());
}

#[test]
fn harmonic_rich_target_survives_while_temporal_drift_is_rejected() {
    let a = pitch_anchor(
        &clip(48000, &[(196.34, 0.1), (392.68, 0.2), (589.02, 0.3)]),
        55,
    )
    .unwrap();
    assert!(a.qualified, "{a:?}");
    assert!((a.frequency_hz.unwrap() - 196.34).abs() < 0.03);
    let drifting = AudioClip::from_samples(
        48000,
        (0..96000)
            .map(|i| {
                let t = i as f64 / 48000.0;
                0.1 * (TAU * (196.0 * t + 2.0 * t * t)).sin()
            })
            .collect(),
    )
    .unwrap();
    let a = pitch_anchor(&drifting, 55).unwrap();
    assert!(!a.qualified);
    assert!(
        a.rejection_reasons
            .contains(&"temporal_frequency_span_exceeds_5_cents")
    );
}
