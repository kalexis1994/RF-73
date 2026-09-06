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
    if request
        .program_id
        .as_deref()
        .is_some_and(|id| id != "research-direct")
        || plugin.programs.len() >= MAX_PROGRAMS
    {
        return None;
    }
    let id = (1..=MAX_PROGRAMS)
        .map(|i| format!("lab-{i}"))
        .find(|id| !plugin.programs.contains_key(id))?;
    let document = ProgramDocument {
        schema_version: 1,
        id,
        name: "Pickup Comparison".into(),
        plugin_id: PLUGIN_ID.into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        plugin_state_version: STATE_VERSION,
        payload_version: 1,
        category: Some("Electric Piano".into()),
        tags: vec!["uncalibrated".into()],
        payload: serde_json::to_value(plugin.settings).ok()?,
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
    for (i, document) in programs.values().enumerate() {
        entries.push(
            json!({"id": format!("custom.{}", document.id), "name": document.name,
            "bank": "research", "category": "Electric Piano", "order": i + 1,
            "tags": ["custom", "uncalibrated"], "editable": true,
            "description": "Saved pickup comparison. Fixed level matching; no limiter."}),
        );
    }
    write(&catalog, destination)
}

pub fn view(bytes: &[u8], destination: &mut [u8]) -> Option<usize> {
    let document: ProgramDocument = read(bytes)?;
    let settings = settings(&document)?;
    let choices: Vec<_> = rf_73_dsp::PICKUP_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| json!({"value": i.to_string(), "label": name}))
        .collect();
    let view: ProgramEditorView = serde_json::from_value(json!({
        "schema_version": 1, "title": "RF-73 Pickup Lab",
        "pages": [{"id": "comparison", "label": "Pickup Comparison",
            "detail": "Fixed level matching. Shared mechanics. 20 ms pickup crossfade.",
            "fields": [
                {"id": "a", "label": "Pickup A", "detail": "Current gap / offset: 1.5 / 0.5 mm.",
                    "value": {"type": "choice", "value": settings.a.to_string()},
                    "kind": {"type": "choice", "options": choices}, "live_preview": true},
                {"id": "b", "label": "Pickup B", "detail": "Close: 0.5 / 0.25 mm. Point Pole is experimental.",
                    "value": {"type": "choice", "value": settings.b.to_string()},
                    "kind": {"type": "choice", "options": choices}, "live_preview": true},
                {"id": "listen_b", "label": "Listen to B", "detail": "Off: A. On: B. Held notes and pedal continue.",
                    "value": {"type": "boolean", "value": settings.listen_b},
                    "kind": {"type": "toggle"}, "live_preview": true},
                {"id": "gain", "label": "Output Gain", "detail": "Start at 0.100x. Watch host meters; no limiter.",
                    "value": {"type": "integer", "value": (settings.gain * 1_000_000.0).round() as i64},
                    "kind": {"type": "number", "minimum": 0, "maximum": 2_000_000, "step": 10_000,
                        "decimals": 6, "unit": "x"}, "live_preview": true}
            ]}]
    })).ok()?;
    view.validate().ok()?;
    write(&view, destination)
}

pub fn edit(bytes: &[u8], destination: &mut [u8]) -> Option<usize> {
    let request: ProgramFieldEditRequest = read(bytes)?;
    request.validate().ok()?;
    let current = settings(&request.document)?;
    let (index, value) = match (request.field_id.as_str(), &request.value) {
        ("a" | "b", ProgramEditorValue::Choice(value))
            if ["0", "1", "2"].contains(&value.as_str()) =>
        {
            (
                if request.field_id == "a" { 1 } else { 2 },
                value.parse().ok()?,
            )
        }
        ("listen_b", ProgramEditorValue::Boolean(value)) => (3, f64::from(u8::from(*value))),
        ("gain", ProgramEditorValue::Integer(value)) if (0..=2_000_000).contains(value) => {
            (0, *value as f64 / 1_000_000.0)
        }
        _ => return None,
    };
    let updated = current.with_parameter(index, value)?;
    let mut document = request.document;
    document.payload = serde_json::to_value(updated).ok()?;
    write(&envelope(document)?, destination)
}
