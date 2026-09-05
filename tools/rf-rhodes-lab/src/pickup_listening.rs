use rf_rhodes_dsp::{
    FIRST_NOTE, KEY_COUNT, LAST_NOTE, MagneticPickup, OVERSAMPLE, ProductionDecimator, Profile,
    Voice,
};
use serde::Serialize;
use serde_json::json;
use std::{collections::BTreeMap, error::Error, io::BufWriter, path::PathBuf};

pub const HELP: &str = "Pickup listening and headroom:
  pickup-listening --output NEW_DIRECTORY [--sample-rate 44100]
    [--gap-mm 0.5] [--offset-mm 0.25] [--ceiling-dbfs -6] [--measure-only]
Three laws/geometries on one performance; one full-program RMS gain per track.
Common peak attenuation, no compression or per-note normalization. Sample peaks only.
Also measures all 73 isolated keys and repeated 10/73-key strikes at full velocity.
Writes report.json and three 24-second mono WAVs; --measure-only omits WAVs.
";

const NAMES: [&str; 3] = ["current", "close-original", "close-point-pole"];
const PROGRAM_SECONDS: u32 = 24;

struct Options {
    output: PathBuf,
    rate: u32,
    gap: f64,
    offset: f64,
    ceiling_dbfs: f64,
    measure_only: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut flags = BTreeMap::new();
        let mut measure_only = false;
        let mut i = 1;
        while i < args.len() {
            let flag = args[i].as_str();
            if flag == "--measure-only" {
                if measure_only {
                    return Err("duplicate --measure-only".into());
                }
                measure_only = true;
                i += 1;
                continue;
            }
            if !matches!(
                flag,
                "--output" | "--sample-rate" | "--gap-mm" | "--offset-mm" | "--ceiling-dbfs"
            ) {
                return Err(format!("unknown listening option: {flag}").into());
            }
            let value = args.get(i + 1).ok_or("missing listening option value")?;
            if flags.insert(flag, value.as_str()).is_some() {
                return Err(format!("duplicate option: {flag}").into());
            }
            i += 2;
        }
        let result = Self {
            output: flags
                .get("--output")
                .ok_or("--output NEW_DIRECTORY is required")?
                .into(),
            rate: flags.get("--sample-rate").unwrap_or(&"44100").parse()?,
            gap: flags.get("--gap-mm").unwrap_or(&"0.5").parse()?,
            offset: flags.get("--offset-mm").unwrap_or(&"0.25").parse()?,
            ceiling_dbfs: flags.get("--ceiling-dbfs").unwrap_or(&"-6").parse()?,
            measure_only,
        };
        if ![44100, 48000, 96000, 192000].contains(&result.rate)
            || !result.ceiling_dbfs.is_finite()
            || !(-24.0..=-1.0).contains(&result.ceiling_dbfs)
        {
            return Err("unsupported rate or sample ceiling outside -24..-1 dBFS".into());
        }
        MagneticPickup::new(result.gap * 0.001, result.offset * 0.001)?;
        if result.output.exists() || result.output.file_name().is_none() {
            return Err("output must be a new directory".into());
        }
        Ok(result)
    }
}

// Offline single-channel evaluator. Every pickup sees the same physical voices.
// State and preparation follow Engine; no new runtime path is added to the plugin.
struct Mix {
    voices: [Voice; KEY_COUNT],
    held: [bool; KEY_COUNT],
    activated: Vec<usize>,
    pedal: bool,
    pickup: MagneticPickup,
    filters: [ProductionDecimator; 3],
}
impl Mix {
    fn new(rate: u32, gap: f64, offset: f64) -> Result<Self, Box<dyn Error>> {
        let voices: Vec<_> = (FIRST_NOTE..=LAST_NOTE)
            .map(|note| Voice::new(rate as f64, note, Profile::default()))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            voices: voices.try_into().map_err(|_| "invalid voice count")?,
            held: [false; KEY_COUNT],
            activated: Vec::with_capacity(KEY_COUNT),
            pedal: false,
            pickup: MagneticPickup::new(gap * 0.001, offset * 0.001)?,
            filters: std::array::from_fn(|_| ProductionDecimator::new()),
        })
    }
    fn event(&mut self, event: EventKind) {
        match event {
            EventKind::On { note, velocity } => {
                let i = (note - FIRST_NOTE) as usize;
                self.held[i] = true;
                self.voices[i].strike(velocity);
                if !self.activated.contains(&i) {
                    self.activated.push(i);
                    self.activated.sort_unstable();
                }
            }
            EventKind::Off { note } => {
                let i = (note - FIRST_NOTE) as usize;
                self.held[i] = false;
                self.voices[i].set_damped(!self.pedal);
            }
            EventKind::Pedal { down } => {
                self.pedal = down;
                for &i in &self.activated {
                    self.voices[i].set_damped(!down && !self.held[i]);
                }
            }
        }
    }
    fn next(&mut self) -> Result<[f32; 3], Box<dyn Error>> {
        for _ in 0..OVERSAMPLE {
            let mut sum = [0.0; 3];
            for &i in &self.activated {
                sum[0] += self.voices[i].tick();
                let p = self.voices[i].probe();
                sum[1] += self.pickup.voltage(p.displacement_m, p.velocity_m_s);
                sum[2] += self
                    .pickup
                    .research_point_pole_voltage(p.displacement_m, p.velocity_m_s);
            }
            if sum.iter().any(|v| !v.is_finite()) {
                return Err("non-finite listening mix".into());
            }
            for (filter, value) in self.filters.iter_mut().zip(sum) {
                filter.push(value);
            }
        }
        let output = self
            .filters
            .each_ref()
            .map(|f| (f.output() * 0.7 * 0.12) as f32);
        if output.iter().any(|v| !v.is_finite()) {
            return Err("non-finite listening output".into());
        }
        Ok(output)
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EventKind {
    On { note: u8, velocity: f64 },
    Off { note: u8 },
    Pedal { down: bool },
}
#[derive(Clone, Copy, Serialize)]
struct Event {
    frame: usize,
    #[serde(flatten)]
    kind: EventKind,
}

fn program(rate: u32) -> Vec<Event> {
    let mut events = Vec::new();
    let mut add = |time: f64, kind| {
        events.push(Event {
            frame: (time * rate as f64).round() as usize,
            kind,
        })
    };
    for (register, note) in [40, 55, 88].into_iter().enumerate() {
        for (level, velocity) in [0.2, 0.5, 0.9].into_iter().enumerate() {
            let at = 0.25 + (register * 3 + level) as f64 * 1.75;
            add(at, EventKind::On { note, velocity });
            add(at + 1.1, EventKind::Off { note });
        }
    }
    let chord = [40, 47, 55, 59, 64, 66];
    add(16.5, EventKind::Pedal { down: true });
    for note in chord {
        add(
            16.5,
            EventKind::On {
                note,
                velocity: 0.65,
            },
        );
        add(17.3, EventKind::Off { note });
    }
    for note in [55, 59, 64] {
        add(
            18.0,
            EventKind::On {
                note,
                velocity: 0.85,
            },
        );
        add(18.5, EventKind::Off { note });
    }
    add(20.0, EventKind::Pedal { down: false });
    add(
        21.0,
        EventKind::On {
            note: 55,
            velocity: 0.5,
        },
    );
    add(21.3, EventKind::Off { note: 55 });
    add(21.32, EventKind::Pedal { down: true });
    add(22.5, EventKind::Pedal { down: false });
    events.sort_by_key(|event| event.frame);
    events
}

#[derive(Default, Clone, Copy, Serialize)]
struct Level {
    peak: f64,
    rms: f64,
}
fn levels(samples: &[f32]) -> Level {
    Level {
        peak: samples
            .iter()
            .map(|&v| (v as f64).abs())
            .fold(0.0, f64::max),
        rms: (samples.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / samples.len() as f64)
            .sqrt(),
    }
}

#[derive(Serialize)]
struct Matching {
    rms_gain: [f64; 3],
    common_attenuation: f64,
    applied_gain: [f64; 3],
    sample_ceiling: f64,
}
fn matching(input: &[Level; 3], ceiling: f64) -> Result<Matching, Box<dyn Error>> {
    if !ceiling.is_finite()
        || !(0.0..=1.0).contains(&ceiling)
        || ceiling == 0.0
        || input
            .iter()
            .any(|l| !l.rms.is_finite() || l.rms < 1e-12 || !l.peak.is_finite() || l.peak < l.rms)
    {
        return Err("invalid level-matching input or insufficient signal energy".into());
    }
    let rms_gain = input.map(|l| input[0].rms / l.rms);
    let peak = input
        .iter()
        .zip(rms_gain)
        .map(|(l, g)| l.peak * g)
        .fold(0.0, f64::max);
    // Leave a tiny rounding margin when converting the scaled signal to f32.
    let common_attenuation = (ceiling * (1.0 - 1e-6) / peak).min(1.0);
    Ok(Matching {
        rms_gain,
        common_attenuation,
        applied_gain: rms_gain.map(|g| g * common_attenuation),
        sample_ceiling: ceiling,
    })
}

fn diagnostic(
    options: &Options,
    notes: &[u8],
    repeated: bool,
) -> Result<[Level; 3], Box<dyn Error>> {
    let mut mix = Mix::new(options.rate, options.gap, options.offset)?;
    let frames = options.rate as usize;
    let mut peak = [0.0_f64; 3];
    let mut energy = [0.0; 3];
    for frame in 0..frames {
        if frame == 0 || (repeated && (frame == frames * 3 / 10 || frame == frames * 6 / 10)) {
            for &note in notes {
                mix.event(EventKind::On {
                    note,
                    velocity: 1.0,
                });
            }
        }
        for (i, value) in mix.next()?.into_iter().enumerate() {
            peak[i] = peak[i].max((value as f64).abs());
            energy[i] += (value as f64).powi(2);
        }
    }
    Ok(std::array::from_fn(|i| Level {
        peak: peak[i],
        rms: (energy[i] / frames as f64).sqrt(),
    }))
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let events = program(options.rate);
    let frames = PROGRAM_SECONDS as usize * options.rate as usize;
    let mut mix = Mix::new(options.rate, options.gap, options.offset)?;
    let mut signals: [Vec<f32>; 3] = std::array::from_fn(|_| Vec::with_capacity(frames));
    let mut next_event = 0;
    for frame in 0..frames {
        while events.get(next_event).is_some_and(|e| e.frame == frame) {
            mix.event(events[next_event].kind);
            next_event += 1;
        }
        for (track, value) in signals.iter_mut().zip(mix.next()?) {
            track.push(value);
        }
    }
    let raw_levels = signals.each_ref().map(|s| levels(s));
    let gains = matching(&raw_levels, 10.0_f64.powf(options.ceiling_dbfs / 20.0))?;
    let mut isolated = Vec::new();
    let mut maximum_isolated = [0.0_f64; 3];
    for note in FIRST_NOTE..=LAST_NOTE {
        let result = diagnostic(&options, &[note], false)?;
        for (maximum, level) in maximum_isolated.iter_mut().zip(result) {
            *maximum = maximum.max(level.peak);
        }
        isolated.push(json!({ "note": note, "levels": result }));
    }
    let chord_notes = [40, 47, 52, 55, 59, 62, 64, 66, 71, 74];
    let repeated_chord = diagnostic(&options, &chord_notes, true)?;
    let all_keys: Vec<_> = (FIRST_NOTE..=LAST_NOTE).collect();
    let repeated_all_keys = diagnostic(&options, &all_keys, true)?;
    for (signal, gain) in signals.iter_mut().zip(gains.applied_gain) {
        for value in signal {
            *value = (*value as f64 * gain) as f32;
        }
    }
    let export_levels = signals.each_ref().map(|s| levels(s));
    if export_levels
        .iter()
        .any(|l| !l.peak.is_finite() || l.peak > gains.sample_ceiling)
    {
        return Err("scaled listening output exceeds the sample ceiling".into());
    }
    let report = json!({
        "schema_version": 1, "experiment": "global_rms_matched_pickup_listening_and_observed_headroom",
        "model": "research-0.1.1-uncalibrated", "sample_rate": options.rate, "frames": frames,
        "track_order": NAMES, "current_geometry_mm": { "gap": 1.5, "offset": 0.5 },
        "candidate_geometry_mm": { "gap": options.gap, "offset": options.offset },
        "program_seconds": PROGRAM_SECONDS, "events": events, "faults": 0,
        "raw_program_levels": raw_levels, "matching": gains, "export_levels": export_levels,
        "wav_files_written": !options.measure_only,
        "diagnostics": {
            "duration_seconds": 1, "velocity": 1.0, "isolated_keys": isolated,
            "maximum_isolated_peaks": maximum_isolated,
            "repeated_strike_seconds": [0.0,0.3,0.6], "repeated_chord_notes": chord_notes,
            "repeated_chord_levels": repeated_chord, "repeated_all_73_keys_levels": repeated_all_keys,
            "observed_set_gain_to_sample_ceiling_not_applied": std::array::from_fn::<_,3,_>(|i| {
                gains.sample_ceiling / maximum_isolated[i].max(repeated_chord[i].peak).max(repeated_all_keys[i].peak)
            }),
        },
        "limitations": [
            "One fixed gain per entire 24-second program, then common attenuation. No per-note, per-velocity or per-window matching.",
            "RMS matching is not perceptual loudness matching. Sample peaks are not oversampled true peaks.",
            "The export ceiling applies to these listening WAVs, not all possible performances or host processing.",
            "Observed one-second stress peaks are diagnostics, not a mathematical headroom bound or a chosen plugin gain.",
            "No clipping, compression, limiter, reference-audio fitting, geometry promotion or real-time timing qualification."
        ]
    });
    if let Some(parent) = options
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&options.output)?;
    if !options.measure_only {
        for (name, signal) in NAMES.into_iter().zip(&signals) {
            let path = options.output.join(format!("{name}.wav"));
            let mut writer = super::wav::FloatWav::new(
                BufWriter::new(super::new_file(&path)?),
                options.rate,
                frames as u32,
            )?;
            for &sample in signal {
                writer.sample(sample)?;
            }
            writer.finish()?;
        }
    }
    super::analysis::write_report(&options.output.join("report.json"), &report)?;
    println!("Listening study: {}", options.output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_attenuation_and_fixed_gains_preserve_dynamics() {
        let input = [
            Level {
                peak: 1.0,
                rms: 0.1,
            },
            Level {
                peak: 4.0,
                rms: 0.2,
            },
            Level {
                peak: 3.0,
                rms: 0.3,
            },
        ];
        let result = matching(&input, 0.5).unwrap();
        for (actual, expected) in result.rms_gain.into_iter().zip([1.0, 0.5, 1.0 / 3.0]) {
            assert!((actual - expected).abs() < 1e-14);
        }
        for (level, gain) in input.into_iter().zip(result.applied_gain) {
            assert!((level.rms * gain - 0.1 * result.common_attenuation).abs() < 1e-14);
            assert!(level.peak * gain < 0.5);
            assert_eq!((0.2 * gain) / (0.1 * gain), 2.0);
        }
        assert!(matching(&[Level::default(); 3], 0.5).is_err());
    }
    #[test]
    fn performance_has_ordered_bounded_events_and_a_release_tail() {
        for rate in [44100, 48000, 96000, 192000] {
            let events = program(rate);
            assert!(events.windows(2).all(|p| p[0].frame <= p[1].frame));
            assert!(events.last().unwrap().frame < PROGRAM_SECONDS as usize * rate as usize);
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e.kind, EventKind::Pedal { down: true }))
            );
        }
    }
    #[test]
    fn current_mix_matches_engine_through_retriggers_and_late_pedal() {
        let rate = 44100;
        let mut mix = Mix::new(rate, 0.5, 0.25).unwrap();
        let mut engine = rf_rhodes_dsp::Engine::new(rate as f64, Profile::default()).unwrap();
        for frame in 0..6000 {
            let events = match frame {
                0 => vec![
                    EventKind::On {
                        note: 55,
                        velocity: 0.8,
                    },
                    EventKind::On {
                        note: 40,
                        velocity: 0.6,
                    },
                ],
                1000 => vec![EventKind::Off { note: 55 }, EventKind::Off { note: 40 }],
                1100 => vec![EventKind::Pedal { down: true }],
                2000 => vec![EventKind::On {
                    note: 55,
                    velocity: 0.9,
                }],
                3000 => vec![EventKind::Off { note: 55 }],
                4000 => vec![EventKind::Pedal { down: false }],
                _ => vec![],
            };
            for e in events {
                mix.event(e);
                match e {
                    EventKind::On { note, velocity } => {
                        engine.note_on(0, note, velocity);
                    }
                    EventKind::Off { note } => {
                        engine.note_off(0, note);
                    }
                    EventKind::Pedal { down } => {
                        engine.control_change(0, 64, if down { 1.0 } else { 0.0 });
                    }
                }
            }
            assert_eq!(
                mix.next().unwrap()[0].to_bits(),
                engine.next_sample().to_bits(),
                "frame {frame}"
            );
        }
        assert_eq!(engine.faults(), 0);
    }
}
