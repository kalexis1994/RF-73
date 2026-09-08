//! Render a Standard MIDI File through the playable 0.1.2 engine. Format 0/1
//! files, tempo map, note on/off, sustain (CC 64) and all-notes-off are
//! honoured; every non-drum channel is merged into one instrument.
use rf_73_dsp::{Engine, FIRST_NOTE, LAST_NOTE, PICKUP_NAMES, Profile};
use serde_json::json;
use std::{error::Error, fs, io::BufWriter, path::Path, time::Instant};

pub const HELP: &str = "MIDI render:
  render-midi INPUT.mid --output AUDIO.wav [--gain G] [--normalize] [--sample-rate HZ] [--tail S]
    [--pickup I] [--sustain original|calibrated] [--bar-ratio R] [--contact-stiffness K]
    [--bar-strike W]
Uncalibrated playable engine (0.1.2 mechanics, default pickup). --pickup 0..3 renders the
laboratory engine's level-matched path instead (3 is Close Aperture). --sustain calibrated
uses the recording-derived first/bar partial T60 (20 s / 2.3 s at A3); --bar-ratio sets the
second partial over the fundamental (default 6.267, recordings 6.0) and --contact-stiffness
the quadratic contact coefficient (default 4e10); --bar-strike the second partial's strike
weight (default -0.3). Gain 0.05..2 scales the
engine output before the WAV (default 1). --normalize also writes AUDIO-norm.wav peaking
at -1 dBFS. Tail 0..30 s after the last event (default 4). A JSON receipt accompanies
the WAV. Existing files are never overwritten.
";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    On(u8, u8),
    Off(u8),
    Sustain(u8),
    AllNotesOff,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub seconds: f64,
    pub order: usize,
    pub kind: Kind,
}
fn variable(data: &[u8], i: &mut usize) -> Result<u32, Box<dyn Error>> {
    let mut value: u32 = 0;
    for _ in 0..4 {
        let byte = *data.get(*i).ok_or("truncated variable-length quantity")?;
        *i += 1;
        value = (value << 7) | u32::from(byte & 0x7F);
        if byte < 0x80 {
            return Ok(value);
        }
    }
    Err("variable-length quantity too long".into())
}
/// Events in seconds sorted by time then file order, the tick division and
/// the tempo map (tick, microseconds per quarter) that placed them.
pub type Parsed = (Vec<Event>, u16, Vec<(u64, u32)>);
pub fn parse(data: &[u8]) -> Result<Parsed, Box<dyn Error>> {
    if data.len() < 14 || &data[..4] != b"MThd" {
        return Err("not a Standard MIDI File".into());
    }
    let header = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let format = u16::from_be_bytes([data[8], data[9]]);
    let tracks = u16::from_be_bytes([data[10], data[11]]);
    let division = u16::from_be_bytes([data[12], data[13]]);
    if format > 1 {
        return Err("only MIDI formats 0 and 1 are handled".into());
    }
    if division & 0x8000 != 0 || division == 0 {
        return Err("SMPTE time division is not handled".into());
    }
    let mut i = 8 + header;
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    let mut ticked: Vec<(u64, usize, Kind)> = Vec::new();
    let mut order = 0usize;
    for _ in 0..tracks {
        if data.len() < i + 8 || &data[i..i + 4] != b"MTrk" {
            return Err("missing track chunk".into());
        }
        let length =
            u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]) as usize;
        let end = i + 8 + length;
        if end > data.len() {
            return Err("truncated track chunk".into());
        }
        let mut j = i + 8;
        let mut tick: u64 = 0;
        let mut status: u8 = 0;
        while j < end {
            tick += u64::from(variable(data, &mut j)?);
            let byte = data[j];
            if byte == 0xFF {
                let meta = *data.get(j + 1).ok_or("truncated meta event")?;
                j += 2;
                let size = variable(data, &mut j)? as usize;
                if meta == 0x51 && size == 3 {
                    let us = u32::from_be_bytes([0, data[j], data[j + 1], data[j + 2]]);
                    tempos.push((tick, us));
                }
                j += size;
                continue;
            }
            if byte == 0xF0 || byte == 0xF7 {
                j += 1;
                let size = variable(data, &mut j)? as usize;
                j += size;
                continue;
            }
            if byte >= 0x80 {
                status = byte;
                j += 1;
            }
            if status < 0x80 {
                return Err("data byte without running status".into());
            }
            let kind = status & 0xF0;
            let channel = status & 0x0F;
            if kind == 0xC0 || kind == 0xD0 {
                j += 1;
                continue;
            }
            let a = *data.get(j).ok_or("truncated channel event")?;
            let b = *data.get(j + 1).ok_or("truncated channel event")?;
            j += 2;
            if channel == 9 {
                continue;
            }
            let event = match kind {
                0x90 if b > 0 => Some(Kind::On(a, b)),
                0x80 | 0x90 => Some(Kind::Off(a)),
                0xB0 if a == 64 => Some(Kind::Sustain(b)),
                0xB0 if a == 123 => Some(Kind::AllNotesOff),
                _ => None,
            };
            if let Some(event) = event {
                ticked.push((tick, order, event));
                order += 1;
            }
        }
        i = end;
    }
    tempos.sort();
    tempos.dedup();
    if tempos.first().is_none_or(|t| t.0 > 0) {
        tempos.insert(0, (0, 500_000));
    }
    ticked.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    let events = ticked
        .into_iter()
        .map(|(tick, order, kind)| Event {
            seconds: seconds_at(tick, division, &tempos),
            order,
            kind,
        })
        .collect();
    Ok((events, division, tempos))
}
pub fn seconds_at(tick: u64, division: u16, tempos: &[(u64, u32)]) -> f64 {
    let mut seconds = 0.0;
    let mut last_tick = 0u64;
    let mut us = 500_000u32;
    for &(at, tempo) in tempos {
        if at >= tick {
            break;
        }
        seconds += (at - last_tick) as f64 * f64::from(us) / (f64::from(division) * 1e6);
        last_tick = at;
        us = tempo;
    }
    seconds + (tick - last_tick) as f64 * f64::from(us) / (f64::from(division) * 1e6)
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let input = Path::new(&args[1]);
    let output = Path::new(&args[3]);
    let mut gain = 1.0_f64;
    let mut normalize = false;
    let mut rate = 48000u32;
    let mut tail = 4.0_f64;
    let mut pickup: Option<usize> = None;
    let mut calibrated = false;
    let mut bar_ratio: Option<f64> = None;
    let mut contact_stiffness: Option<f64> = None;
    let mut bar_strike: Option<f64> = None;
    let mut k = 4;
    while k < args.len() {
        match args[k].as_str() {
            "--normalize" => normalize = true,
            "--gain"
            | "--sample-rate"
            | "--tail"
            | "--pickup"
            | "--sustain"
            | "--bar-ratio"
            | "--contact-stiffness"
            | "--bar-strike"
                if k + 1 < args.len() =>
            {
                let value = &args[k + 1];
                match args[k].as_str() {
                    "--gain" => gain = value.parse()?,
                    "--sample-rate" => rate = value.parse()?,
                    "--pickup" => pickup = Some(value.parse()?),
                    "--bar-ratio" => bar_ratio = Some(value.parse()?),
                    "--contact-stiffness" => contact_stiffness = Some(value.parse()?),
                    "--bar-strike" => bar_strike = Some(value.parse()?),
                    "--sustain" => {
                        calibrated = match value.as_str() {
                            "original" => false,
                            "calibrated" => true,
                            _ => return Err("sustain must be original or calibrated".into()),
                        }
                    }
                    _ => tail = value.parse()?,
                }
                k += 1;
            }
            _ => return Err(HELP.into()),
        }
        k += 1;
    }
    if !gain.is_finite() || !(0.05..=2.0).contains(&gain) {
        return Err("gain must be within 0.05..=2".into());
    }
    if !matches!(rate, 44100 | 48000 | 96000 | 192000) {
        return Err("sample rate must be 44100, 48000, 96000 or 192000".into());
    }
    if !tail.is_finite() || !(0.0..=30.0).contains(&tail) {
        return Err("tail must be within 0..=30 seconds".into());
    }
    if pickup.is_some_and(|i| i >= PICKUP_NAMES.len()) {
        return Err("pickup path must be 0..=3".into());
    }
    if output.extension().is_none_or(|x| x != "wav") {
        return Err("output must be a new .wav path".into());
    }
    let receipt = output.with_extension("json");
    let stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid output name")?;
    let normalized = output.with_file_name(format!("{stem}-norm.wav"));
    for path in [
        Some(output),
        Some(receipt.as_path()),
        normalize.then_some(normalized.as_path()),
    ]
    .into_iter()
    .flatten()
    {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }
    let data = fs::read(input)?;
    let (events, division, tempos) = parse(&data)?;
    if events.is_empty() {
        return Err("the MIDI file has no note or pedal events".into());
    }
    let last = events.iter().map(|e| e.seconds).fold(0.0, f64::max);
    let frames = ((last + tail) * f64::from(rate)).ceil() as u32;
    if frames == 0 || f64::from(frames) / f64::from(rate) > 1800.0 {
        return Err("render length must be positive and at most 30 minutes".into());
    }
    let base = if calibrated {
        Profile::calibrated_sustain()
    } else {
        Profile::default()
    };
    let profile = Profile {
        bar_partial_ratio: bar_ratio.unwrap_or(base.bar_partial_ratio),
        contact_stiffness: contact_stiffness.unwrap_or(base.contact_stiffness),
        bar_partial_strike_weight: bar_strike.unwrap_or(base.bar_partial_strike_weight),
        ..base
    };
    let mut engine = match pickup {
        Some(index) => {
            let mut engine = Engine::new_laboratory_with(f64::from(rate), profile)?;
            if !engine.set_pickup(index) {
                return Err("invalid laboratory pickup path".into());
            }
            engine.reset();
            engine
        }
        None => Engine::new(f64::from(rate), profile)?,
    };
    engine.set_gain(1.0);
    let mut samples = Vec::with_capacity(frames as usize);
    let mut next = 0usize;
    let mut notes = 0u64;
    let mut dropped = 0u64;
    let mut pedal = 0u64;
    let started = Instant::now();
    let mut peak = 0.0_f64;
    let mut square = 0.0_f64;
    for frame in 0..frames {
        let now = f64::from(frame) / f64::from(rate);
        while next < events.len() && events[next].seconds <= now {
            match events[next].kind {
                Kind::On(note, velocity) => {
                    if (FIRST_NOTE..=LAST_NOTE).contains(&note) {
                        notes += 1;
                        engine.note_on(0, note, f64::from(velocity) / 127.0);
                    } else {
                        dropped += 1;
                    }
                }
                Kind::Off(note) => {
                    engine.note_off(0, note);
                }
                Kind::Sustain(value) => {
                    pedal += 1;
                    engine.control_change(0, 64, f64::from(value) / 127.0);
                }
                Kind::AllNotesOff => {
                    engine.control_change(0, 123, 0.0);
                }
            }
            next += 1;
        }
        let sample = f64::from(engine.next_sample()) * gain;
        if !sample.is_finite() || engine.faults() != 0 {
            return Err("render encountered a numerical fault".into());
        }
        peak = peak.max(sample.abs());
        square += sample * sample;
        samples.push(sample as f32);
    }
    let elapsed = started.elapsed().as_secs_f64();
    let mut writer =
        crate::wav::FloatWav::new(BufWriter::new(crate::new_file(output)?), rate, frames)?;
    for s in &samples {
        writer.sample(*s)?;
    }
    writer.finish()?;
    let target = 10f64.powf(-1.0 / 20.0);
    let normalize_gain = if normalize && peak > 0.0 {
        let g = target / peak;
        let mut writer =
            crate::wav::FloatWav::new(BufWriter::new(crate::new_file(&normalized)?), rate, frames)?;
        for s in &samples {
            writer.sample((f64::from(*s) * g) as f32)?;
        }
        writer.finish()?;
        Some(g)
    } else {
        None
    };
    let report = json!({"schema_version":1,"experiment":"midi-render-v1","input":input.display().to_string(),
        "model":"research-0.1.2-uncalibrated","sample_rate":rate,"frames":frames,"seconds":f64::from(frames)/f64::from(rate),
        "division_ticks_per_quarter":division,"tempo_changes":tempos.len(),"events":events.len(),"notes_played":notes,
        "notes_outside_range_dropped":dropped,"sustain_events":pedal,"last_event_seconds":last,"tail_seconds":tail,
        "pickup_path":pickup,"pickup_name":pickup.map(|i| PICKUP_NAMES[i]),
        "sustain":if calibrated {"calibrated"} else {"original"},
        "bar_partial_ratio":profile.bar_partial_ratio,"contact_stiffness":profile.contact_stiffness,
        "bar_partial_strike_weight":profile.bar_partial_strike_weight,
        "gain":gain,"peak":peak,"rms":(square/f64::from(frames)).sqrt(),"peak_dbfs":20.0*peak.max(1e-12).log10(),
        "normalized_output":normalize_gain.map(|_|normalized.display().to_string()),"normalize_gain":normalize_gain,
        "normalized_peak_dbfs":normalize_gain.map(|_|-1.0),"faults":engine.faults(),
        "render_wall_seconds":elapsed,"realtime_ratio":elapsed/(f64::from(frames)/f64::from(rate)),
        "scope":"Playable 0.1.2 research engine with the default pickup, or one level-matched laboratory path when --pickup is given, and no limiter, reverb or amplifier; uncalibrated. Channels merged, drums excluded, CC 66/67 ignored. Normalization is a separate explicitly requested file."});
    crate::analysis::write_report(&receipt, &report)?;
    println!(
        "Rendered {} ({} notes, peak {:.3} dBFS, {:.2}x realtime)",
        output.display(),
        notes,
        20.0 * peak.max(1e-12).log10(),
        elapsed / (f64::from(frames) / f64::from(rate))
    );
    if let Some(g) = normalize_gain {
        println!("Normalized copy {} (gain {:.4})", normalized.display(), g);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn track(body: &[u8]) -> Vec<u8> {
        let mut t = b"MTrk".to_vec();
        t.extend_from_slice(&(body.len() as u32).to_be_bytes());
        t.extend_from_slice(body);
        t
    }
    fn file(format: u16, tracks: &[Vec<u8>], division: u16) -> Vec<u8> {
        let mut f = b"MThd".to_vec();
        f.extend_from_slice(&6u32.to_be_bytes());
        f.extend_from_slice(&format.to_be_bytes());
        f.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        f.extend_from_slice(&division.to_be_bytes());
        for t in tracks {
            f.extend_from_slice(t);
        }
        f
    }
    #[test]
    fn parses_tempo_map_running_status_sustain_and_orders_events() {
        // Tempo 120 bpm at tick 0, 60 bpm at tick 480 (one quarter later).
        let tempo = track(&[
            0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20, // 500000 us
            0x83, 0x60, 0xFF, 0x51, 0x03, 0x0F, 0x42, 0x40, // +480: 1000000 us
            0x00, 0xFF, 0x2F, 0x00,
        ]);
        let notes = track(&[
            0x00, 0x90, 60, 100, // on C4 at tick 0
            0x83, 0x60, 62, 90, // running status: on D4 at tick 480
            0x83, 0x60, 60, 0, // off C4 at tick 960 (velocity 0)
            0x00, 0xB0, 64, 127, // sustain down at tick 960
            0x00, 0x80, 62, 0, // off D4 at 960
            0x00, 0x99, 36, 100, // drum channel ignored
            0x00, 0xC0, 5, // program change skipped
            0x00, 0xF0, 0x01, 0x00, // sysex skipped
            0x00, 0xB0, 123, 0, // all notes off
            0x00, 0xFF, 0x2F, 0x00,
        ]);
        let data = file(1, &[tempo, notes], 480);
        let (events, division, tempos) = parse(&data).unwrap();
        assert_eq!(division, 480);
        assert_eq!(tempos, vec![(0, 500_000), (480, 1_000_000)]);
        assert_eq!(events.len(), 6);
        assert_eq!(events[0].kind, Kind::On(60, 100));
        assert_eq!(events[0].seconds, 0.0);
        assert_eq!(events[1].kind, Kind::On(62, 90));
        assert!((events[1].seconds - 0.5).abs() < 1e-12);
        // Tick 960 is one quarter at 120 bpm plus one quarter at 60 bpm.
        assert_eq!(events[2].kind, Kind::Off(60));
        assert!((events[2].seconds - 1.5).abs() < 1e-12);
        assert_eq!(events[3].kind, Kind::Sustain(127));
        assert_eq!(events[4].kind, Kind::Off(62));
        assert_eq!(events[5].kind, Kind::AllNotesOff);
        assert!(events.windows(2).all(|w| w[0].seconds <= w[1].seconds));
        assert!((seconds_at(1440, 480, &tempos) - 2.5).abs() < 1e-12);
    }
    #[test]
    fn rejects_non_midi_smpte_and_truncated_files() {
        assert!(parse(b"RIFF....").is_err());
        let smpte = file(0, &[track(&[0x00, 0xFF, 0x2F, 0x00])], 0xE728);
        assert!(parse(&smpte).is_err());
        let mut truncated = file(0, &[track(&[0x00, 0x90, 60])], 96);
        truncated.truncate(truncated.len() - 1);
        assert!(parse(&truncated).is_err());
        let empty = file(0, &[track(&[0x00, 0xFF, 0x2F, 0x00])], 96);
        let (events, _, tempos) = parse(&empty).unwrap();
        assert!(events.is_empty());
        assert_eq!(tempos, vec![(0, 500_000)]);
    }
}
