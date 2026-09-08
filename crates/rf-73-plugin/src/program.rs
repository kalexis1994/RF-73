//! Control-thread JSON only. Audio rendering and parameter automation never serialize.
use crate::{Rf73Processor, STATE_VERSION, Settings};
use rackforge_program_api::{
    PreparedProgram, ProgramDocument, ProgramEditRequest, ProgramEditorValue, ProgramEditorView,
    ProgramFieldEditRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::collections::BTreeMap;

const PLUGIN_ID: &str = "org.rackforge.rhodes";
const MAX_PROGRAMS: usize = 8;
const MAX_TRANSFER: usize = 4096;

fn read<T: DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    if bytes.len() > MAX_TRANSFER {
        return None;
    }
    serde_json::from_slice(bytes).ok()
}

fn write<T: Serialize>(value: &T, destination: &mut [u8]) -> Option<usize> {
    let bytes = serde_json::to_vec(value).ok()?;
    if bytes.len() > MAX_TRANSFER {
        return None;
    }
    destination.get_mut(..bytes.len())?.copy_from_slice(&bytes);
    Some(bytes.len())
}

pub fn settings(document: &ProgramDocument) -> Option<Settings> {
    document.validate().ok()?;
    if document.plugin_id != PLUGIN_ID
        || document.plugin_state_version != STATE_VERSION
        || document.payload_version != 1
        || document.id.len() > 64
        || document.name.len() > 64
    {
        return None;
    }
    let value: Settings = serde_json::from_value(document.payload.clone()).ok()?;
    value.valid().then_some(value)
}

fn envelope(document: ProgramDocument) -> Option<PreparedProgram> {
    settings(&document)?;
    let prepared = PreparedProgram {
        schema_version: 1,
        storage_path: format!("programs/{}.json", document.id),
        preview_sound_id: format!("custom.{}", document.id),
        document,
        artifacts: Vec::new(),
    };
    prepared.validate().ok()?;
    Some(prepared)
}

pub fn validated_prepared(bytes: &[u8]) -> Option<ProgramDocument> {
    let prepared: PreparedProgram = read(bytes)?;
    prepared.validate().ok()?;
    let expected = envelope(prepared.document.clone())?;
    (prepared == expected).then_some(prepared.document)
}

pub fn begin(plugin: &Rf73Processor, request: &[u8], destination: &mut [u8]) -> Option<usize> {
    let request: ProgramEditRequest = read(request)?;
    request.validate().ok()?;
    if let Some(id) = request
        .program_id
        .as_deref()
        .and_then(|id| id.strip_prefix("custom."))
    {
        return write(&envelope(plugin.programs.get(id)?.clone())?, destination);
    }
    let factory = request
        .program_id
        .as_deref()
        .map(|id| crate::settings::presets().into_iter().find(|p| p.0 == id));
    let base = match factory {
        None => plugin.settings,
        Some(Some(preset)) => preset.3,
        Some(None) => return None,
    };
    if plugin.programs.len() >= MAX_PROGRAMS {
        return None;
    }
    let id = (1..=MAX_PROGRAMS)
        .map(|i| format!("lab-{i}"))
        .find(|id| !plugin.programs.contains_key(id))?;
    let document = ProgramDocument {
        schema_version: 1,
        id,
        name: "Voicing".into(),
        plugin_id: PLUGIN_ID.into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        plugin_state_version: STATE_VERSION,
        payload_version: 1,
        category: Some("Electric Piano".into()),
        tags: vec!["uncalibrated".into()],
        payload: serde_json::to_value(base).ok()?,
    };
    write(&envelope(document)?, destination)
}

pub fn prepare(bytes: &[u8], destination: &mut [u8]) -> Option<usize> {
    write(&envelope(read(bytes)?)?, destination)
}

pub fn install(plugin: &mut Rf73Processor, bytes: &[u8]) -> bool {
    let Some(document) = validated_prepared(bytes) else {
        return false;
    };
    if !plugin.programs.contains_key(&document.id) && plugin.programs.len() >= MAX_PROGRAMS {
        return false;
    }
    let mut programs = plugin.programs.clone();
    programs.insert(document.id.clone(), document);
    if catalog(&programs, &mut [0; MAX_TRANSFER]).is_none() {
        return false;
    }
    plugin.programs = programs;
    true
}

pub fn catalog(
    programs: &BTreeMap<String, ProgramDocument>,
    destination: &mut [u8],
) -> Option<usize> {
    let mut catalog: serde_json::Value =
        serde_json::from_str(include_str!("../../../package/metadata/presets.json")).ok()?;
    let entries = catalog["presets"].as_array_mut()?;
    let factory = entries.len();
    for (i, document) in programs.values().enumerate() {
        entries.push(
            json!({"id": format!("custom.{}", document.id), "name": document.name,
            "bank": "research", "category": "Electric Piano", "order": factory + i,
            "tags": ["custom", "uncalibrated"], "editable": true,
            "description": "Saved voicing. Level compensated pickup; no limiter."}),
        );
    }
    write(&catalog, destination)
}

/// Editor fields carry integers: hundredths of a millimetre for the distances,
/// thousandths for the unit controls and millionths for the gain.
type Field = (
    &'static str,
    &'static str,
    &'static str,
    u32,
    f64,
    i64,
    i64,
    u32,
    &'static str,
);
const FIELDS: [Field; 8] = [
    (
        "distance",
        "Pickup Distance",
        "Gap 0.50..3.00 mm; compensated level.",
        2,
        100.0,
        50,
        300,
        2,
        "mm",
    ),
    (
        "alignment",
        "Tine Alignment",
        "Offset -1.00..1.50 mm; centre is hollow.",
        3,
        100.0,
        -100,
        150,
        2,
        "mm",
    ),
    (
        "hardness",
        "Hammer Hardness",
        "Contact stiffness, 0.500 is the original.",
        4,
        1000.0,
        0,
        1000,
        3,
        "",
    ),
    (
        "sustain",
        "Sustain",
        "0 original 5 s, 0.5 calibrated 20 s, 1 is 80 s.",
        5,
        1000.0,
        0,
        1000,
        3,
        "",
    ),
    (
        "bell",
        "Bell",
        "Second partial strike, 1 original, 0.258 calibrated.",
        6,
        1000.0,
        0,
        1000,
        3,
        "",
    ),
    (
        "dynamics",
        "Dynamics",
        "Velocity curve, 0.5 is the original 1.4 power.",
        7,
        1000.0,
        0,
        1000,
        3,
        "",
    ),
    (
        "gain",
        "Output Gain",
        "Start at 0.100x. Watch host meters; no limiter.",
        0,
        1_000_000.0,
        0,
        2_000_000,
        6,
        "x",
    ),
    (
        "law",
        "Pickup Law",
        "Production surrogate or finite aperture, 2 mm pole.",
        1,
        1.0,
        0,
        1,
        0,
        "",
    ),
];

pub fn view(bytes: &[u8], destination: &mut [u8]) -> Option<usize> {
    let document: ProgramDocument = read(bytes)?;
    let settings = settings(&document)?;
    let laws: Vec<_> = crate::settings::LAW_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| json!({"value": i.to_string(), "label": name}))
        .collect();
    let mut fields = Vec::new();
    for (id, label, detail, index, scale, minimum, maximum, decimals, unit) in FIELDS {
        let value = settings.parameter(index)?;
        fields.push(if id == "law" {
            json!({"id": id, "label": label, "detail": detail,
                "value": {"type": "choice", "value": (value as u8).to_string()},
                "kind": {"type": "choice", "options": laws}, "live_preview": true})
        } else {
            let mut kind = json!({"type": "number", "minimum": minimum, "maximum": maximum,
                "step": if decimals == 6 { 10_000 } else { 1 }, "decimals": decimals});
            if !unit.is_empty() {
                kind["unit"] = json!(unit);
            }
            json!({"id": id, "label": label, "detail": detail,
                "value": {"type": "integer", "value": (value * scale).round() as i64},
                "kind": kind, "live_preview": true})
        });
    }
    let view: ProgramEditorView = serde_json::from_value(json!({
        "schema_version": 1, "title": "RF-73 Voicing",
        "pages": [{"id": "sound", "label": "Sound",
            "detail": "Physical voicing; the pickup keeps its level.",
            "fields": fields}]
    }))
    .ok()?;
    view.validate().ok()?;
    write(&view, destination)
}

pub fn edit(bytes: &[u8], destination: &mut [u8]) -> Option<usize> {
    let request: ProgramFieldEditRequest = read(bytes)?;
    request.validate().ok()?;
    let current = settings(&request.document)?;
    let field = FIELDS.iter().find(|f| f.0 == request.field_id)?;
    let (index, value) = match (&request.value, field.0) {
        (ProgramEditorValue::Choice(value), "law") => (field.3, value.parse::<u8>().ok()? as f64),
        (ProgramEditorValue::Integer(value), id) if id != "law" => {
            if !(field.5..=field.6).contains(value) {
                return None;
            }
            (field.3, *value as f64 / field.4)
        }
        _ => return None,
    };
    let updated = current.with_parameter(index, value)?;
    let mut document = request.document;
    document.payload = serde_json::to_value(updated).ok()?;
    write(&envelope(document)?, destination)
}
