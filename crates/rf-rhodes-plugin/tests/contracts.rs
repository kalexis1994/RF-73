use rackforge_plugin_sdk::{
    MIDI2_FLAG_ORIGIN_7BIT, MIDI2_KIND_NOTE_ON, MidiEvent, MidiEvent2, ParameterEvent, Processor,
};
use rf_rhodes_plugin::RhodesProcessor;

fn prepared() -> RhodesProcessor {
    let mut plugin = RhodesProcessor::default();
    assert!(plugin.prepare(48_000.0, 4096, 0, 2));
    plugin
}

fn render(block: usize, note: u8) -> Vec<f32> {
    let events = [
        (17, [0x90, note, 110]),
        (230, [0xb0, 64, 127]),
        (477, [0x80, note, 0]),
        (997, [0x90, note, 70]),
        (1900, [0x80, note, 0]),
        (2100, [0xb0, 64, 0]),
    ];
    let mut plugin = prepared();
    let mut result = Vec::new();
    for start in (0..4096).step_by(block) {
        let frames = block.min(4096 - start);
        let midi: Vec<_> = events
            .iter()
            .filter(|(t, _)| *t >= start && *t < start + frames)
            .map(|(t, data)| MidiEvent {
                frame: (t - start) as u32,
                data: *data,
                length: 3,
            })
            .collect();
        let parameters: Vec<_> = if (start..start + frames).contains(&1234) {
            vec![ParameterEvent {
                frame: (1234 - start) as u32,
                index: 0,
                value: 0.3,
            }]
        } else {
            vec![]
        };
        let mut out = vec![0.0; frames * 2];
        plugin.process(&[], &mut out, &midi, &parameters, frames as u32, 0, 2);
        result.extend(out);
    }
    result
}

#[test]
fn block_size_does_not_change_sound_or_event_timing() {
    for note in [57, 100] {
        let reference = render(4096, note);
        assert!(reference[..34].iter().all(|v| *v == 0.0));
        assert!(reference.iter().any(|v| v.abs() > 1e-5));
        for block in [1, 64, 127, 128, 256, 512] {
            assert_eq!(render(block, note), reference);
        }
    }
}

#[test]
fn state_is_versioned_and_rejected_atomically() {
    let mut plugin = prepared();
    plugin.set_parameter(0, 0.4);
    let mut state = [0; 16];
    assert_eq!(plugin.save_state(&mut state), Some(16));
    plugin.set_parameter(0, 0.8);
    assert!(plugin.load_state(&state));
    assert_eq!(plugin.get_parameter(0), Some(0.4));
    for len in 0..16 {
        assert!(!plugin.load_state(&state[..len]));
    }
    state[8..].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(!plugin.load_state(&state));
    assert_eq!(plugin.get_parameter(0), Some(0.4));
    state[4] = 2;
    assert!(!plugin.load_state(&state));
    assert!(!plugin.load_preset("unknown"));
}

#[test]
fn malformed_blocks_are_silent_and_do_not_apply_partial_edits() {
    let mut plugin = prepared();
    let mut out = [1.0; 128];
    let parameters = [
        ParameterEvent {
            frame: 0,
            index: 0,
            value: 0.1,
        },
        ParameterEvent {
            frame: 65,
            index: 0,
            value: 0.5,
        },
    ];
    plugin.process(&[], &mut out, &[], &parameters, 64, 0, 2);
    assert_eq!(out, [0.0; 128]);
    assert_eq!(plugin.get_parameter(0), Some(0.7));
    plugin.process(
        &[],
        &mut out,
        &[MidiEvent {
            frame: 0,
            data: [0x90, 57, 255],
            length: 3,
        }],
        &[],
        64,
        0,
        2,
    );
    assert_eq!(out, [0.0; 128]);
    assert!(!plugin.prepare(f64::NAN, 128, 0, 2));
}

#[test]
fn upscaled_midi1_matches_original_and_native_wide_velocity_is_preserved() {
    let mut a = prepared();
    let mut b = prepared();
    let mut out_a = [0.0; 8192];
    let mut out_b = [0.0; 8192];
    a.process(
        &[],
        &mut out_a,
        &[MidiEvent {
            frame: 0,
            data: [0x90, 57, 90],
            length: 3,
        }],
        &[],
        4096,
        0,
        2,
    );
    let wide = MidiEvent2 {
        frame: 0,
        kind: MIDI2_KIND_NOTE_ON,
        channel: 0,
        index: 57,
        flags: MIDI2_FLAG_ORIGIN_7BIT,
        value: 90 << 9,
        extra: 0,
    };
    b.process_wide(&[], &mut out_b, &[], &[wide], &[], 4096, 0, 2);
    assert_eq!(out_a, out_b);
    let mut c = prepared();
    let mut d = prepared();
    c.process_wide(
        &[],
        &mut out_a,
        &[],
        &[MidiEvent2 {
            flags: 0,
            value: 40_000,
            ..wide
        }],
        &[],
        4096,
        0,
        2,
    );
    d.process_wide(
        &[],
        &mut out_b,
        &[],
        &[MidiEvent2 {
            flags: 0,
            value: 40_001,
            ..wide
        }],
        &[],
        4096,
        0,
        2,
    );
    assert_ne!(out_a, out_b);
}
