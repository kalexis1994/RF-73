use rackforge_plugin_sdk::Processor;
use rf_73_dsp::{Engine, PickupLaw};
use rf_73_plugin::{Rf73Processor, STATE_BYTES, presets};

#[test]
fn register_preset_survives_state_and_matches_frozen_geometry_at_normal_velocities() {
    let mut plugin = Rf73Processor::default();
    assert!(plugin.load_preset("calibrated-register"));
    assert!(plugin.prepare(48000.0, 256, 0, 2));
    let mut state = [0; STATE_BYTES];
    plugin.save_state(&mut state).unwrap();
    let mut restored = Rf73Processor::default();
    assert!(restored.load_state(&state));
    assert_eq!(restored.get_parameter(1), Some(2.0));
    assert!(restored.prepare(48000.0, 256, 0, 2));
    let selected = presets()[4].3.profile();
    let base = presets()[3].3.profile();
    for note in [28, 55, 64, 72, 100] {
        let t = ((f64::from(note) - 55.0) / 17.0).clamp(0.0, 1.0);
        let w = t * t * (3.0 - 2.0 * t);
        let expected = rf_73_dsp::Profile {
            pickup_law: PickupLaw::Aperture,
            pickup_gap_m: base.pickup_gap_m * (w * 0.135).exp(),
            pickup_offset_m: base.pickup_offset_m * (-w * 0.020).exp(),
            ..base
        };
        for velocity in [0.3, 0.85] {
            let mut a = Engine::new(48000.0, selected).unwrap();
            let mut b = Engine::new(48000.0, expected).unwrap();
            // Exercise profile replacement as well as construction.
            assert!(a.set_profile(base));
            assert!(a.set_profile(selected));
            for engine in [&mut a, &mut b] {
                engine.set_gain(0.1);
                engine.reset();
                engine.note_on(0, note, velocity);
            }
            for _ in 0..4800 {
                assert_eq!(a.next_sample(), b.next_sample());
            }
            assert_eq!(a.faults(), 0);
            assert_eq!(b.faults(), 0);
        }
    }
}
