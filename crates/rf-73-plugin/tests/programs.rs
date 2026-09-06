use rackforge_plugin_sdk::{MidiEvent, ParameterEvent, Processor};
use rackforge_program_api::{
    PreparedProgram, ProgramEditRequest, ProgramEditorValue, ProgramEditorView,
    ProgramFieldEditRequest,
};
use rf_73_plugin::{Rf73Processor, STATE_BYTES};

fn begin(plugin: &mut Rf73Processor, id: Option<&str>) -> PreparedProgram {
    let request = serde_json::to_vec(&ProgramEditRequest::new(id.map(str::to_owned))).unwrap();
    let mut bytes = [0; 4096];
    let len = plugin.begin_program_edit(&request, &mut bytes).unwrap();
    serde_json::from_slice(&bytes[..len]).unwrap()
}

fn state(plugin: &Rf73Processor) -> [u8; STATE_BYTES] {
    let mut bytes = [0; STATE_BYTES];
    assert_eq!(plugin.save_state(&mut bytes), Some(STATE_BYTES));
    bytes
}

#[test]
fn editor_preview_install_catalog_reload_and_snapshot_agree() {
    let mut plugin = Rf73Processor::default();
    let mut draft = begin(&mut plugin, Some("research-direct"));
    let mut destination = [0; 4096];
    for (field, value, parameter, expected) in [
        ("a", ProgramEditorValue::Choice("1".into()), 1, 1.0),
        ("b", ProgramEditorValue::Choice("0".into()), 2, 0.0),
        ("listen_b", ProgramEditorValue::Boolean(true), 3, 1.0),
        ("gain", ProgramEditorValue::Integer(123456), 0, 0.123456),
    ] {
        let before = state(&plugin);
        let request = ProgramFieldEditRequest {
            schema_version: 1,
            document: draft.document,
            field_id: field.into(),
            value,
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        assert!(plugin.apply_program_edit(&bytes, &mut [0; 1]).is_none());
        let len = plugin.apply_program_edit(&bytes, &mut destination).unwrap();
        assert_eq!(
            state(&plugin),
            before,
            "editing a draft must not change audio"
        );
        draft = serde_json::from_slice(&destination[..len]).unwrap();
        assert!(plugin.preview_program(&destination[..len]));
        assert_eq!(plugin.get_parameter(parameter), Some(expected));
    }
    let document = serde_json::to_vec(&draft.document).unwrap();
    let len = plugin
        .program_editor_view(&document, &mut destination)
        .unwrap();
    let view: ProgramEditorView = serde_json::from_slice(&destination[..len]).unwrap();
    view.validate().unwrap();
    assert_eq!(view.pages[0].fields.len(), 4);
    assert!(view.pages[0].fields.iter().all(|field| field.live_preview));
    let len = plugin
        .prepare_program_save(&document, &mut destination)
        .unwrap();
    let prepared = destination[..len].to_vec();
    assert!(plugin.install_program(&prepared));
    let saved = state(&plugin);
    assert!(plugin.load_preset("research-direct"));
    assert!(plugin.load_preset(&draft.preview_sound_id));
    assert_eq!(state(&plugin), saved);
    assert_eq!(begin(&mut plugin, Some(&draft.preview_sound_id)), draft);
    let len = plugin.write_program_catalog(&mut destination).unwrap();
    let catalog: serde_json::Value = serde_json::from_slice(&destination[..len]).unwrap();
    assert_eq!(catalog["presets"][1]["id"], draft.preview_sound_id);
    // The host replays stored program documents into fresh instances.
    let mut restored = Rf73Processor::default();
    assert!(restored.install_program(&prepared));
    assert!(restored.load_preset(&draft.preview_sound_id));
    assert_eq!(state(&restored), saved);
    let mut snapshot = Rf73Processor::default();
    assert!(snapshot.load_state(&saved)); // Full settings, independent of custom catalog.
    assert_eq!(state(&snapshot), saved);
}

#[test]
fn malformed_programs_and_parameter_domains_reject_atomically() {
    let mut plugin = Rf73Processor::default();
    let draft = begin(&mut plugin, None);
    let before = state(&plugin);
    for (index, value) in [
        (0, f64::NAN),
        (0, -0.1),
        (1, 0.5),
        (2, 3.0),
        (3, 0.5),
        (4, 0.0),
    ] {
        assert!(!plugin.set_parameter(index, value));
        assert_eq!(state(&plugin), before);
    }
    let mut malformed = draft.clone();
    malformed.storage_path = "../outside.json".into();
    assert!(!plugin.install_program(&serde_json::to_vec(&malformed).unwrap()));
    malformed = draft.clone();
    malformed.document.payload["a"] = serde_json::json!(3);
    assert!(!plugin.preview_program(&serde_json::to_vec(&malformed).unwrap()));
    malformed = draft.clone();
    malformed.document.plugin_id = "org.example.other".into();
    assert!(!plugin.install_program(&serde_json::to_vec(&malformed).unwrap()));
    assert_eq!(state(&plugin), before);
    assert!(plugin.prepare(48000.0, 128, 0, 2));
    let mut out = [1.0; 256];
    plugin.process(
        &[],
        &mut out,
        &[MidiEvent {
            frame: 0,
            length: 3,
            data: [0x90, 55, 100],
        }],
        &[
            ParameterEvent {
                frame: 0,
                index: 0,
                value: 0.2,
            },
            ParameterEvent {
                frame: 10,
                index: 2,
                value: 1.5,
            },
        ],
        128,
        0,
        2,
    );
    assert!(out.iter().all(|x| *x == 0.0));
    assert_eq!(state(&plugin), before);
}

#[test]
fn bounded_catalog_fits_transfer_and_rejects_overflow_without_losing_entries() {
    let mut plugin = Rf73Processor::default();
    let mut last = None;
    for _ in 0..8 {
        let mut draft = begin(&mut plugin, None);
        draft.document.name = "X".repeat(64);
        assert!(plugin.install_program(&serde_json::to_vec(&draft).unwrap()));
        last = Some(draft);
    }
    let mut out = [0; 4096];
    let len = plugin.write_program_catalog(&mut out).unwrap();
    let catalog: serde_json::Value = serde_json::from_slice(&out[..len]).unwrap();
    assert_eq!(catalog["presets"].as_array().unwrap().len(), 9);
    let mut draft = last.unwrap();
    assert!(plugin.install_program(&serde_json::to_vec(&draft).unwrap()));
    draft.document.id = "overflow".into();
    draft.storage_path = "programs/overflow.json".into();
    draft.preview_sound_id = "custom.overflow".into();
    assert!(!plugin.install_program(&serde_json::to_vec(&draft).unwrap()));
    assert_eq!(plugin.write_program_catalog(&mut [0; 4096]), Some(len));
}

#[test]
fn legacy_state_keeps_original_gain_and_pickup_and_new_state_rejects_every_invalid_field() {
    let mut plugin = Rf73Processor::default();
    let mut legacy = [0; 16];
    legacy[..4].copy_from_slice(b"RFRH");
    legacy[4..8].copy_from_slice(&1u32.to_le_bytes());
    legacy[8..].copy_from_slice(&0.7f64.to_le_bytes());
    assert!(plugin.set_parameter(3, 1.0));
    assert!(plugin.load_state(&legacy));
    assert_eq!(plugin.get_parameter(0), Some(0.7));
    assert_eq!(plugin.get_parameter(1), Some(0.0));
    assert_eq!(plugin.get_parameter(3), Some(0.0));
    let before = state(&plugin);
    for (index, value) in [(0, 0), (4, 3), (16, 3), (17, 3), (18, 2), (19, 1)] {
        let mut malformed = before;
        malformed[index] = value;
        assert!(!plugin.load_state(&malformed));
        assert_eq!(state(&plugin), before);
    }
    let mut destination = [42; STATE_BYTES - 1];
    assert_eq!(plugin.save_state(&mut destination), None);
    assert_eq!(destination, [42; STATE_BYTES - 1]);
}
