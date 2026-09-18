use rf_tines_dsp::{
    APERTURE_PICKUP, AxialAperture, Engine, MagneticPickup, OVERSAMPLE, PICKUP_LEVEL_MATCH,
    PICKUP_NAMES, PROFILE_NAMES, ProductionDecimator, Profile, Voice, aperture_voltage,
};

#[test]
fn each_matched_path_agrees_with_independent_voice_and_filter() {
    for rate in [44100.0, 48000.0, 96000.0, 192000.0] {
        for note in [28, 55, 100] {
            for (index, compensation) in PICKUP_LEVEL_MATCH.into_iter().enumerate() {
                let mut engine = Engine::new_laboratory(rate).unwrap();
                assert!(engine.set_pickup(index));
                engine.reset();
                let mut voice = Voice::new(rate, note, Profile::default()).unwrap();
                let pickup = MagneticPickup::new(0.0005, 0.00025).unwrap();
                let aperture = AxialAperture::new(APERTURE_PICKUP).unwrap();
                let mut filter = ProductionDecimator::new();
                engine.note_on(0, note, 0.9);
                voice.strike(0.9);
                for frame in 0..2048 {
                    if frame == 1000 {
                        engine.note_off(0, note);
                        voice.set_damped(true);
                    }
                    if frame == 1200 {
                        engine.note_on(0, note, 0.2);
                        voice.strike(0.2);
                    }
                    for _ in 0..OVERSAMPLE {
                        let current = voice.tick();
                        let p = voice.probe();
                        filter.push(match index {
                            0 => current,
                            1 => pickup.voltage(p.displacement_m, p.velocity_m_s),
                            2 => {
                                pickup.research_point_pole_voltage(p.displacement_m, p.velocity_m_s)
                            }
                            _ => aperture_voltage(&aperture, p.displacement_m, p.velocity_m_s),
                        });
                    }
                    let expected = (filter.output() * compensation * 0.7 * 0.12) as f32;
                    assert_eq!(
                        engine.next_sample(),
                        expected,
                        "{rate}/{note}/{index}/{frame}"
                    );
                }
                assert_eq!(engine.faults(), 0);
            }
        }
    }
}

#[test]
fn switching_and_interrupted_fades_preserve_mechanics_and_pedal() {
    let mut baseline = Engine::new(48000.0, Profile::default()).unwrap();
    let mut lab = Engine::new_laboratory(48000.0).unwrap();
    assert!(!baseline.set_pickup(1));
    assert_eq!(PICKUP_NAMES.len(), 4);
    assert!(!lab.set_pickup(PICKUP_NAMES.len()));
    for frame in 0..6000 {
        for engine in [&mut baseline, &mut lab] {
            match frame {
                0 => {
                    engine.note_on(2, 55, 0.8);
                }
                700 => {
                    engine.note_off(2, 55);
                }
                740 => {
                    engine.control_change(2, 64, 1.0);
                }
                1600 => {
                    engine.note_on(2, 55, 0.4);
                }
                2000 => {
                    engine.note_off(2, 55);
                }
                3500 => {
                    engine.control_change(2, 64, 0.0);
                }
                _ => {}
            }
        }
        if [500, 800, 1000, 1700, 2100, 3100].contains(&frame) {
            lab.set_pickup((frame / 100) % PICKUP_NAMES.len());
        }
        if frame == 4000 {
            lab.set_pickup(0);
        }
        let current = baseline.next_sample();
        let candidate = lab.next_sample();
        assert!(candidate.is_finite());
        if !(500..4960).contains(&frame) {
            assert_eq!(current, candidate);
        }
        let a = baseline.probe(55).unwrap();
        let b = lab.probe(55).unwrap();
        assert_eq!(a.displacement_m, b.displacement_m);
        assert_eq!(a.velocity_m_s, b.velocity_m_s);
        assert_eq!(a.mechanical_energy_j, b.mechanical_energy_j);
    }
    lab.reset();
    for _ in 0..4096 {
        assert_eq!(lab.next_sample(), 0.0);
    }
}

#[test]
fn profile_switching_keeps_ringing_notes_and_matches_a_fresh_engine_when_set_before_playing() {
    let rate = 48_000.0;
    for index in 0..PROFILE_NAMES.len() {
        let profile = Profile::named(index).unwrap();
        // Setting the profile before any note is the same as building with it.
        let mut switched = Engine::new_laboratory(rate).unwrap();
        assert!(switched.set_profile(profile));
        let mut built = Engine::new_laboratory_with(rate, profile).unwrap();
        for engine in [&mut switched, &mut built] {
            assert!(engine.set_pickup(3));
            engine.reset();
            engine.note_on(0, 55, 0.8);
        }
        for frame in 0..3000 {
            if frame == 1500 {
                switched.note_off(0, 55);
                built.note_off(0, 55);
            }
            assert_eq!(
                switched.next_sample(),
                built.next_sample(),
                "{index} {frame}"
            );
        }
    }
    assert!(Profile::named(PROFILE_NAMES.len()).is_none());
    // Switching while a note rings keeps the state: no gap, no fault, and the
    // sample after the switch stays within the recent peak.
    let mut engine = Engine::new_laboratory(rate).unwrap();
    engine.note_on(0, 55, 0.9);
    let mut peak: f32 = 0.0;
    let mut last = 0.0f32;
    for frame in 0..24_000 {
        if frame == 6000 {
            assert!(engine.set_profile(Profile::calibrated()));
        }
        if frame == 12_000 {
            assert!(engine.set_profile(Profile::calibrated_sustain()));
        }
        if frame == 18_000 {
            assert!(engine.set_profile(Profile::default()));
        }
        let sample = engine.next_sample();
        assert!(sample.is_finite());
        if [6000, 12_000, 18_000].contains(&frame) {
            assert!(
                (sample - last).abs() < 0.5 * peak,
                "{frame}: {last} -> {sample}"
            );
        }
        peak = peak.max(sample.abs());
        last = sample;
    }
    assert_eq!(engine.faults(), 0);
    assert!(engine.probe(55).unwrap().mechanical_energy_j > 0.0);
    // Invalid profiles and a moved pickup geometry are rejected without change.
    assert!(!engine.set_profile(Profile {
        decay_seconds: 0.0,
        ..Profile::default()
    }));
    assert!(!engine.set_profile(Profile {
        pickup_gap_m: 0.001,
        ..Profile::default()
    }));
    let mut raw = Engine::new(rate, Profile::default()).unwrap();
    assert!(raw.set_profile(Profile {
        pickup_gap_m: 0.001,
        ..Profile::default()
    }));
}
