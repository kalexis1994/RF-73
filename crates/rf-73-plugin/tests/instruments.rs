use rackforge_plugin_sdk::{MidiEvent, Processor};
use rf_73_plugin::{Rf73Processor, STATE_BYTES, Settings, presets};

#[test]
fn v4_state_migrates_to_neutral_electronics_and_identical_audio() {
    let mut old = Rf73Processor::default();
    assert!(old.load_preset("calibrated-register"));
    let mut bytes = [0; STATE_BYTES];
    old.save_state(&mut bytes).unwrap();
    bytes[4..8].copy_from_slice(&4u32.to_le_bytes());
    let mut migrated = Rf73Processor::default();
    assert!(migrated.load_state(&bytes[..68]));
    for index in 0..15 {
        assert_eq!(old.get_parameter(index), migrated.get_parameter(index));
    }
    for plugin in [&mut old, &mut migrated] {
        assert!(plugin.prepare(48000.0, 256, 0, 2));
    }
    let note = [MidiEvent {
        frame: 0,
        data: [0x90, 60, 110],
        length: 3,
    }];
    for block in 0..32 {
        let mut a = [0.0; 512];
        let mut b = a;
        let events = if block == 0 { &note[..] } else { &[] };
        old.process(&[], &mut a, events, &[], 256, 0, 2);
        migrated.process(&[], &mut b, events, &[], 256, 0, 2);
        assert_eq!(a, b);
    }
    let mut json = serde_json::to_value(Settings::default()).unwrap();
    for key in [
        "bass_db",
        "treble_db",
        "vibrato",
        "speed_hz",
        "intensity",
        "preamp",
        "bass_boost",
    ] {
        json.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(
        serde_json::from_value::<Settings>(json).unwrap(),
        Settings::default()
    );
}

#[test]
fn electronic_fields_roundtrip_and_invalid_state_is_atomic() {
    let mut plugin = Rf73Processor::default();
    for (index, value) in [
        (8, 6.0),
        (9, -3.0),
        (10, 1.0),
        (11, 7.5),
        (12, 0.8),
        (13, 0.0),
        (14, 0.4),
    ] {
        assert!(plugin.set_parameter(index, value));
    }
    let mut state = [0; STATE_BYTES];
    plugin.save_state(&mut state).unwrap();
    let mut restored = Rf73Processor::default();
    assert!(restored.load_state(&state));
    for i in 0..15 {
        assert_eq!(restored.get_parameter(i), plugin.get_parameter(i));
    }
    for i in 0..7 {
        let mut bad = state;
        bad[68 + i * 8..76 + i * 8].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(!restored.load_state(&bad));
        let mut after = [0; STATE_BYTES];
        restored.save_state(&mut after).unwrap();
        assert_eq!(after, state);
    }
}

#[test]
fn instrument_programs_have_distinct_physics_and_bounded_dense_chords() {
    let factory = presets();
    for a in 5..factory.len() {
        for b in a + 1..factory.len() {
            let x = factory[a].3;
            let y = factory[b].3;
            assert_ne!(
                [x.hardness, x.sustain, x.bell, x.distance_mm, x.alignment_mm],
                [y.hardness, y.sustain, y.bell, y.distance_mm, y.alignment_mm]
            );
        }
        let (id, _, _, settings) = factory[a];
        let mut plugin = Rf73Processor::default();
        assert!(plugin.load_preset(id));
        assert!(plugin.prepare(48000.0, 256, 0, 2));
        let mut peak = 0.0_f32;
        let mut stereo = false;
        for block in 0..375 {
            let mut notes = Vec::new();
            if [0, 94, 188].contains(&block) {
                for note in [40, 47, 52, 55, 59, 62, 64, 67, 71, 76] {
                    notes.push(MidiEvent {
                        frame: 0,
                        data: [0x90, note, 127],
                        length: 3,
                    });
                }
            }
            let mut audio = [0.0; 512];
            plugin.process(&[], &mut audio, &notes, &[], 256, 0, 2);
            for pair in audio.as_chunks::<2>().0 {
                assert!(pair[0].is_finite() && pair[1].is_finite());
                peak = peak.max(pair[0].abs()).max(pair[1].abs());
                stereo |= (pair[0] - pair[1]).abs() > 1e-5;
            }
        }
        println!("{id}: peak={peak:.6}, stereo={stereo}");
        assert!(peak > 0.001 && peak < 1.0, "{id}: {peak}");
        assert_eq!(stereo, settings.preamp == 1.0 && settings.vibrato == 1.0);
    }
}
