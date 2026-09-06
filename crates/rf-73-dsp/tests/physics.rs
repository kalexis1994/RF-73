use rf_73_dsp::{Engine, FIRST_NOTE, LAST_NOTE, OVERSAMPLE, Profile, Voice};

#[test]
fn contact_is_passive_and_separates_across_registers_and_rates() {
    for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
        for note in [FIRST_NOTE, 45, 57, 69, LAST_NOTE] {
            for velocity in [0.01, 0.2, 0.7, 1.0] {
                let mut voice = Voice::new(rate, note, Profile::default()).unwrap();
                voice.strike(velocity);
                let mut previous = voice.probe().mechanical_energy_j;
                let mut peak = 0.0_f64;
                for _ in 0..(rate * OVERSAMPLE as f64 * 0.04) as usize {
                    let value = voice.tick();
                    let probe = voice.probe();
                    assert!(value.is_finite());
                    assert!(probe.contact_force_n >= 0.0);
                    assert!(
                        probe.mechanical_energy_j <= previous * (1.0 + 1e-8) + 1e-15,
                        "energy increased at {rate}/{note}/{velocity}: {} > {previous}",
                        probe.mechanical_energy_j
                    );
                    previous = probe.mechanical_energy_j;
                    peak = peak.max(value.abs());
                }
                assert!(!voice.probe().contact_active, "contact did not separate");
                assert!(peak > 1e-9, "silent strike");
            }
        }
    }
}

#[test]
fn dampers_remove_energy_and_retrigger_preserves_motion() {
    let mut voice = Voice::new(48_000.0, 57, Profile::default()).unwrap();
    voice.strike(0.8);
    for _ in 0..10_000 {
        voice.tick();
    }
    let before = voice.probe();
    voice.strike(0.6);
    assert_eq!(voice.probe().displacement_m, before.displacement_m);
    assert_eq!(voice.probe().velocity_m_s, before.velocity_m_s);
    for _ in 0..10_000 {
        voice.tick();
    }
    voice.set_damped(true);
    for _ in 0..100_000 {
        voice.tick();
    }
    assert!(voice.probe().mechanical_energy_j < 1e-18);
}

#[test]
fn invalid_inputs_are_rejected_without_poisoning_audio() {
    for rate in [f64::NAN, f64::INFINITY, 0.0, 8000.0] {
        assert!(Engine::new(rate, Profile::default()).is_err());
    }
    assert!(
        Engine::new(
            48_000.0,
            Profile {
                pickup_gap_m: 0.0,
                ..Profile::default()
            }
        )
        .is_err()
    );
    let mut engine = Engine::new(48_000.0, Profile::default()).unwrap();
    assert!(!engine.note_on(16, 57, 1.0));
    assert!(!engine.note_on(0, 0, 1.0));
    assert!(!engine.note_on(0, 57, f64::NAN));
    assert!(!engine.set_gain(f64::INFINITY));
    assert!(!engine.control_change(0, 64, f64::NAN));
    for _ in 0..1000 {
        assert_eq!(engine.next_sample(), 0.0);
    }
}

#[test]
fn pedal_and_all_notes_off_are_channel_aware() {
    let mut engine = Engine::new(48_000.0, Profile::default()).unwrap();
    engine.note_on(0, 57, 0.8);
    engine.control_change(0, 64, 1.0);
    engine.control_change(0, 123, 0.0);
    for _ in 0..24_000 {
        engine.next_sample();
    }
    assert!(engine.probe(57).unwrap().mechanical_energy_j > 1e-7);
    engine.control_change(1, 64, 0.0);
    for _ in 0..100 {
        engine.next_sample();
    }
    assert!(engine.probe(57).unwrap().mechanical_energy_j > 1e-7);
    engine.control_change(0, 64, 0.0);
    for _ in 0..24_000 {
        engine.next_sample();
    }
    assert!(engine.probe(57).unwrap().mechanical_energy_j < 1e-18);
}

#[test]
fn silence_reset_and_render_are_deterministic() {
    let mut a = Engine::new(48_000.0, Profile::default()).unwrap();
    let mut b = Engine::new(48_000.0, Profile::default()).unwrap();
    a.note_on(2, 57, 0.7);
    b.note_on(2, 57, 0.7);
    for _ in 0..4096 {
        assert_eq!(a.next_sample(), b.next_sample());
    }
    a.reset();
    for _ in 0..100 {
        assert_eq!(a.next_sample(), 0.0);
    }
    assert_eq!(a.faults(), 0);
}

#[test]
fn all_sound_off_stops_released_tails() {
    let mut engine = Engine::new(48_000.0, Profile::default()).unwrap();
    engine.note_on(3, 57, 1.0);
    for _ in 0..1000 {
        engine.next_sample();
    }
    engine.note_off(3, 57);
    engine.control_change(3, 120, 0.0);
    for _ in 0..64 {
        engine.next_sample();
    }
    assert_eq!(engine.next_sample(), 0.0);
}

#[test]
fn full_keyboard_extreme_profile_stays_finite() {
    let profile = Profile {
        contact_stiffness: 1e12,
        maximum_hammer_speed_m_s: 3.0,
        pickup_gap_m: 0.0005,
        modal_mass_kg: 0.0001,
        ..Profile::default()
    };
    let mut engine = Engine::new(44_100.0, profile).unwrap();
    for note in FIRST_NOTE..=LAST_NOTE {
        engine.note_on(0, note, 1.0);
    }
    for _ in 0..4096 {
        assert!(engine.next_sample().is_finite());
    }
    assert_eq!(engine.faults(), 0);
}

#[test]
fn late_pedal_recaptures_a_released_tail() {
    let mut engine = Engine::new(48_000.0, Profile::default()).unwrap();
    engine.note_on(0, 57, 0.8);
    for _ in 0..5000 {
        engine.next_sample();
    }
    engine.note_off(0, 57);
    for _ in 0..200 {
        engine.next_sample();
    }
    engine.control_change(0, 64, 1.0);
    for _ in 0..24_000 {
        engine.next_sample();
    }
    assert!(engine.probe(57).unwrap().mechanical_energy_j > 1e-7);
}

#[test]
fn one_channels_panic_does_not_kill_another_channels_held_key() {
    let mut engine = Engine::new(48_000.0, Profile::default()).unwrap();
    engine.note_on(0, 57, 0.7);
    engine.note_on(1, 57, 0.7);
    for _ in 0..5000 {
        engine.next_sample();
    }
    engine.control_change(0, 120, 0.0);
    assert!(engine.probe(57).unwrap().mechanical_energy_j > 1e-7);
    engine.control_change(1, 120, 0.0);
    assert_eq!(engine.probe(57).unwrap().mechanical_energy_j, 0.0);
}

#[test]
fn sustained_fundamental_tracks_target_pitch_at_all_output_rates() {
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        for note in [40, 57, 81, 100] {
            let mut voice = Voice::new(rate, note, Profile::default()).unwrap();
            voice.strike(0.5);
            let internal_rate = rate * OVERSAMPLE as f64;
            for _ in 0..(internal_rate * 0.12) as usize {
                voice.tick();
            }
            let mut previous = voice.probe().displacement_m;
            let mut crossings = Vec::new();
            for frame in 0..(internal_rate * 0.12) as usize {
                voice.tick();
                let now = voice.probe().displacement_m;
                if previous < 0.0 && now >= 0.0 {
                    crossings.push(frame as f64 + (-previous) / (now - previous));
                }
                previous = now;
            }
            assert!(crossings.len() > 2);
            let frequency = (crossings.len() - 1) as f64 * internal_rate
                / (crossings.last().unwrap() - crossings[0]);
            let expected = 440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0);
            let cents = 1200.0 * (frequency / expected).log2();
            assert!(cents.abs() < 0.1, "{rate}/{note}: {cents} cents");
        }
    }
}
