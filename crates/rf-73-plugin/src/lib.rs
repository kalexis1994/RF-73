//! RackForge adapter. Device access and persistence remain host responsibilities.
mod program;
mod settings;
use rackforge_plugin_sdk::{
    MIDI_FAMILY_CONTROL, MIDI_FAMILY_NOTE, MIDI2_FLAG_ORIGIN_7BIT, MIDI2_KIND_CONTROL_CHANGE,
    MIDI2_KIND_NOTE_OFF, MIDI2_KIND_NOTE_ON, MidiEvent, MidiEvent2, ParameterEvent, Processor,
    export_processor,
};
use rf_73_dsp::Engine;
pub use settings::{DEFAULT_GAIN, Settings};
use std::collections::BTreeMap;

pub const MAX_FRAMES: u32 = 4096;
pub const MAX_EVENTS: usize = 256;
pub const STATE_VERSION: u32 = 2;
pub const STATE_BYTES: usize = 20;
pub const PARAMETER_GAIN: u32 = 0;
pub const PARAMETER_A: u32 = 1;
pub const PARAMETER_B: u32 = 2;
pub const PARAMETER_LISTEN_B: u32 = 3;

#[derive(Default)]
pub struct Rf73Processor {
    engine: Option<Box<Engine>>,
    settings: Settings,
    programs: BTreeMap<String, rackforge_program_api::ProgramDocument>,
    maximum_frames: u32,
    channels: u32,
}

impl Rf73Processor {
    fn apply_settings(&mut self, settings: Settings) -> bool {
        if !settings.valid() {
            return false;
        }
        self.settings = settings;
        if let Some(engine) = &mut self.engine {
            engine.set_gain(settings.gain);
            engine.set_pickup(settings.selected());
        }
        true
    }

    fn midi1(&mut self, event: &MidiEvent) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let [status, index, value] = event.data;
        match status & 0xf0 {
            0x90 => {
                engine.note_on(status & 15, index, value as f64 / 127.0);
            }
            0x80 => {
                engine.note_off(status & 15, index);
            }
            0xb0 => {
                engine.control_change(status & 15, index, value as f64 / 127.0);
            }
            _ => {}
        }
    }

    fn midi2(&mut self, event: &MidiEvent2) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        match event.kind {
            MIDI2_KIND_NOTE_ON => {
                let velocity = if event.flags & MIDI2_FLAG_ORIGIN_7BIT != 0 {
                    (event.value >> 9) as f64 / 127.0
                } else {
                    // A genuine MIDI 2.0 Note On with zero velocity is not Note Off.
                    event.value.max(1) as f64 / 65535.0
                };
                engine.note_on(event.channel, event.index, velocity);
            }
            MIDI2_KIND_NOTE_OFF => {
                engine.note_off(event.channel, event.index);
            }
            MIDI2_KIND_CONTROL_CHANGE => {
                let value = if event.flags & MIDI2_FLAG_ORIGIN_7BIT != 0 {
                    (event.value >> 25) as f64 / 127.0
                } else {
                    event.value as f64 / u32::MAX as f64
                };
                engine.control_change(event.channel, event.index, value);
            }
            _ => {}
        }
    }
}

impl Processor for Rf73Processor {
    fn prepare(&mut self, rate: f64, frames: u32, inputs: u32, outputs: u32) -> bool {
        if frames == 0 || frames > MAX_FRAMES || inputs != 0 || !(1..=2).contains(&outputs) {
            return false;
        }
        let Ok(mut engine) = Engine::new_laboratory(rate) else {
            return false;
        };
        engine.set_gain(self.settings.gain);
        engine.set_pickup(self.settings.selected());
        engine.reset();
        self.engine = Some(Box::new(engine));
        self.maximum_frames = frames;
        self.channels = outputs;
        true
    }

    fn set_parameter(&mut self, index: u32, value: f64) -> bool {
        let Some(settings) = self.settings.with_parameter(index, value) else {
            return false;
        };
        self.apply_settings(settings)
    }

    fn get_parameter(&self, index: u32) -> Option<f64> {
        self.settings.parameter(index)
    }

    fn reset(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.reset();
        }
    }

    fn load_preset(&mut self, id: &str) -> bool {
        if id == "research-direct" {
            return self.apply_settings(Settings::default());
        }
        let Some(settings) = id
            .strip_prefix("custom.")
            .and_then(|id| self.programs.get(id))
            .and_then(program::settings)
        else {
            return false;
        };
        self.apply_settings(settings)
    }

    fn save_state(&self, destination: &mut [u8]) -> Option<usize> {
        let bytes = destination.get_mut(..STATE_BYTES)?;
        bytes[..4].copy_from_slice(b"RFRH");
        bytes[4..8].copy_from_slice(&STATE_VERSION.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.settings.gain.to_le_bytes());
        bytes[16..20].copy_from_slice(&[
            self.settings.a,
            self.settings.b,
            u8::from(self.settings.listen_b),
            0,
        ]);
        Some(STATE_BYTES)
    }

    fn load_state(&mut self, state: &[u8]) -> bool {
        if ![16, STATE_BYTES].contains(&state.len()) || &state[..4] != b"RFRH" {
            return false;
        }
        let version = u32::from_le_bytes(state[4..8].try_into().expect("validated state length"));
        let gain = f64::from_le_bytes(state[8..16].try_into().expect("validated state length"));
        let settings = match (version, state.len()) {
            (1, 16) => Settings {
                gain,
                ..Settings::default()
            },
            (STATE_VERSION, STATE_BYTES) if state[18] <= 1 && state[19] == 0 => Settings {
                gain,
                a: state[16],
                b: state[17],
                listen_b: state[18] == 1,
            },
            _ => return false,
        };
        self.apply_settings(settings)
    }

    fn program_editing_capabilities(&self) -> u32 {
        rackforge_plugin_sdk::PROGRAM_EDIT_BASIC
            | rackforge_plugin_sdk::PROGRAM_EDIT_PREVIEW
            | rackforge_plugin_sdk::PROGRAM_EDIT_DECLARATIVE
    }

    fn write_program_catalog(&mut self, destination: &mut [u8]) -> Option<usize> {
        program::catalog(&self.programs, destination)
    }

    fn begin_program_edit(&mut self, request: &[u8], destination: &mut [u8]) -> Option<usize> {
        program::begin(self, request, destination)
    }

    fn prepare_program_save(&mut self, document: &[u8], destination: &mut [u8]) -> Option<usize> {
        program::prepare(document, destination)
    }

    fn install_program(&mut self, prepared: &[u8]) -> bool {
        program::install(self, prepared)
    }

    fn preview_program(&mut self, prepared: &[u8]) -> bool {
        let Some(document) = program::validated_prepared(prepared) else {
            return false;
        };
        self.apply_settings(program::settings(&document).expect("validated program"))
    }

    fn program_editor_view(&mut self, document: &[u8], destination: &mut [u8]) -> Option<usize> {
        program::view(document, destination)
    }

    fn apply_program_edit(&mut self, request: &[u8], destination: &mut [u8]) -> Option<usize> {
        program::edit(request, destination)
    }

    fn process(
        &mut self,
        input: &[f32],
        output: &mut [f32],
        midi: &[MidiEvent],
        parameters: &[ParameterEvent],
        frames: u32,
        inputs: u32,
        outputs: u32,
    ) {
        self.process_wide(
            input,
            output,
            midi,
            &[],
            parameters,
            frames,
            inputs,
            outputs,
        );
    }

    fn process_wide(
        &mut self,
        _input: &[f32],
        output: &mut [f32],
        midi: &[MidiEvent],
        midi2: &[MidiEvent2],
        parameters: &[ParameterEvent],
        frames: u32,
        inputs: u32,
        outputs: u32,
    ) {
        output.fill(0.0);
        let samples = (frames as usize).checked_mul(outputs as usize);
        if self.engine.is_none()
            || frames > self.maximum_frames
            || inputs != 0
            || outputs != self.channels
            || samples.is_none_or(|n| n > output.len())
            || !ordered(midi.iter().map(|e| e.frame), frames)
            || !ordered(midi2.iter().map(|e| e.frame), frames)
            || !ordered(parameters.iter().map(|e| e.frame), frames)
            || midi.iter().any(|e| !valid_midi1(e))
            || midi2.iter().any(|e| {
                e.channel >= 16
                    || e.index >= 128
                    || (matches!(e.kind, MIDI2_KIND_NOTE_ON | MIDI2_KIND_NOTE_OFF)
                        && e.value > 65535)
            })
            || parameters
                .iter()
                .any(|e| self.settings.with_parameter(e.index, e.value).is_none())
        {
            return;
        }
        let (mut p, mut m, mut w) = (0, 0, 0);
        for frame in 0..frames {
            // Explicit stable tie order: parameters, MIDI 1.0, then MIDI 2.0.
            while p < parameters.len() && parameters[p].frame == frame {
                self.set_parameter(parameters[p].index, parameters[p].value);
                p += 1;
            }
            while m < midi.len() && midi[m].frame == frame {
                self.midi1(&midi[m]);
                m += 1;
            }
            while w < midi2.len() && midi2[w].frame == frame {
                self.midi2(&midi2[w]);
                w += 1;
            }
            let sample = self.engine.as_mut().expect("prepared engine").next_sample();
            let offset = frame as usize * outputs as usize;
            for channel in 0..outputs as usize {
                output[offset + channel] = sample;
            }
        }
    }
}

fn ordered(frames: impl Iterator<Item = u32>, block_frames: u32) -> bool {
    let mut previous = 0;
    let mut count = 0;
    for frame in frames {
        count += 1;
        if count > MAX_EVENTS || frame >= block_frames || frame < previous {
            return false;
        }
        previous = frame;
    }
    true
}

fn valid_midi1(event: &MidiEvent) -> bool {
    if !(1..=3).contains(&event.length) || event.data[0] < 128 {
        return false;
    }
    if event.data[1..event.length as usize]
        .iter()
        .any(|b| *b >= 128)
    {
        return false;
    }
    !matches!(event.data[0] & 0xf0, 0x80 | 0x90 | 0xb0) || event.length == 3
}

export_processor!(Rf73Processor,
    max_frames = 4096, max_input_channels = 0, max_output_channels = 2,
    max_midi_events = 256, max_parameter_events = 256, max_transfer_bytes = 4096,
    midi2 = { max_events = 256, families = MIDI_FAMILY_NOTE | MIDI_FAMILY_CONTROL }
);
