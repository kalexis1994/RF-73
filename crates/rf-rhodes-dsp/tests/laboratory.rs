use rf_rhodes_dsp::{
    Engine, MagneticPickup, OVERSAMPLE, PICKUP_LEVEL_MATCH, ProductionDecimator, Profile, Voice,
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
                            _ => {
                                pickup.research_point_pole_voltage(p.displacement_m, p.velocity_m_s)
                            }
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
    assert!(!lab.set_pickup(3));
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
            lab.set_pickup((frame / 100) % 3);
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
