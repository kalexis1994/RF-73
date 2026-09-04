//! Offline laboratory. All rendering, WAV writing and analysis runs in Rust.
mod analysis;
mod audition;
mod convergence;
mod package;
mod partial_comparison;
mod pickup_set;
mod pickup_sweep;
mod wav;
use rf_rhodes_dsp::{Engine, FIRST_NOTE, LAST_NOTE, Profile};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

const HELP: &str = "RF-Rhodes research laboratory 0.1.1
Usage:
  rf-rhodes-lab render --output PATH.wav [options]
  rf-rhodes-lab demo --output PATH.wav
  rf-rhodes-lab stress [--sample-rate HZ]
  rf-rhodes-lab inspect PATH.wav
  rf-rhodes-lab package
  rf-rhodes-lab audition [--prepare-only]
Render options:
  --note N          MIDI 28..100 (default 57 / A3)
  --velocity V      Greater than 0, up to 1 (default 0.7)
  --sample-rate HZ  44100, 48000, 96000 or 192000 (default 48000)
  --seconds S       Duration 0.05..60 (default 4)
  --hold S          Key hold, less than duration (default 2)
  --gap-mm X        Pickup gap 0.5..5 mm (default 1.5)
  --offset-mm X     Pickup offset -3..3 mm (default 0.5)
  --trace           Export output-rate physical probes as CSV
WAV is mono IEEE float, without normalization or clipping. Existing files are
never overwritten. Every render writes a JSON report. Parameters are uncalibrated.
Demo: three A3 intensities and a sustained E-minor chord, ten seconds.
Stress: 73 keys, 128-frame blocks, three seconds. No audio device is opened.
";

#[derive(Debug)]
struct Options {
    command: String,
    output: Option<PathBuf>,
    note: u8,
    velocity: f64,
    rate: u32,
    seconds: f64,
    hold: f64,
    gap_mm: f64,
    offset_mm: f64,
    trace: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(command) = args.first() else {
            return Err(HELP.into());
        };
        if !matches!(command.as_str(), "render" | "demo" | "stress") {
            return Err(format!("unknown command: {command}"));
        }
        let mut o = Self {
            command: command.clone(),
            output: None,
            note: 57,
            velocity: 0.7,
            rate: 48_000,
            seconds: 4.0,
            hold: 2.0,
            gap_mm: 1.5,
            offset_mm: 0.5,
            trace: false,
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut i = 1;
        while i < args.len() {
            let flag = args[i].as_str();
            if !seen.insert(flag) {
                return Err(format!("duplicate option: {flag}"));
            }
            if flag == "--trace" {
                if command != "render" {
                    return Err("--trace is a render option".into());
                }
                o.trace = true;
                i += 1;
                continue;
            }
            let allowed = match command.as_str() {
                "stress" => flag == "--sample-rate",
                "demo" => matches!(flag, "--sample-rate" | "--output"),
                _ => matches!(
                    flag,
                    "--output"
                        | "--note"
                        | "--velocity"
                        | "--sample-rate"
                        | "--seconds"
                        | "--hold"
                        | "--gap-mm"
                        | "--offset-mm"
                ),
            };
            if !allowed {
                return Err(format!("unknown or inapplicable option: {flag}"));
            }
            let value = args
                .get(i + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            let invalid = || format!("invalid value for {flag}: {value}");
            match flag {
                "--output" => o.output = Some(value.into()),
                "--note" => o.note = value.parse().map_err(|_| invalid())?,
                "--velocity" => o.velocity = value.parse().map_err(|_| invalid())?,
                "--sample-rate" => o.rate = value.parse().map_err(|_| invalid())?,
                "--seconds" => o.seconds = value.parse().map_err(|_| invalid())?,
                "--hold" => o.hold = value.parse().map_err(|_| invalid())?,
                "--gap-mm" => o.gap_mm = value.parse().map_err(|_| invalid())?,
                "--offset-mm" => o.offset_mm = value.parse().map_err(|_| invalid())?,
                _ => unreachable!(),
            }
            i += 2;
        }
        if ![44_100, 48_000, 96_000, 192_000].contains(&o.rate) {
            return Err("unsupported sample rate".into());
        }
        if command != "stress"
            && o.output
                .as_ref()
                .is_none_or(|p| p.extension().is_none_or(|e| e != "wav"))
        {
            return Err("--output must name a .wav file".into());
        }
        if !(FIRST_NOTE..=LAST_NOTE).contains(&o.note)
            || !o.velocity.is_finite()
            || o.velocity <= 0.0
            || o.velocity > 1.0
            || !o.seconds.is_finite()
            || !(0.05..=60.0).contains(&o.seconds)
            || !o.hold.is_finite()
            || o.hold < 0.0
            || o.hold >= o.seconds
        {
            return Err("note, velocity, duration or hold is outside its allowed range".into());
        }
        o.profile()
            .validate(o.rate as f64)
            .map_err(|e| e.to_string())?;
        Ok(o)
    }
    fn profile(&self) -> Profile {
        Profile {
            pickup_gap_m: self.gap_mm * 0.001,
            pickup_offset_m: self.offset_mm * 0.001,
            ..Profile::default()
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        print!("{HELP}");
        print!("{}", analysis::HELP);
        print!("{}", partial_comparison::HELP);
        print!("{}", pickup_sweep::HELP);
        print!("{}", pickup_set::HELP);
        print!("{}", convergence::HELP);
        return Ok(());
    }
    if args[0] == "inspect" {
        if args.len() != 2 {
            return Err("inspect needs exactly one WAV path".into());
        }
        println!("{}", wav::inspect(File::open(&args[1])?)?);
        return Ok(());
    }
    if args[0] == "compare-partials" {
        return partial_comparison::run(&args);
    }
    if args[0] == "sweep-pickup" {
        return pickup_sweep::run(&args);
    }
    if args[0] == "fit-pickup-set" {
        return pickup_set::run(&args);
    }
    if matches!(args[0].as_str(), "analyze" | "compare") {
        return analysis::run(&args);
    }
    if args[0] == "converge" {
        return convergence::run(&args);
    }
    if args[0] == "package" {
        if args.len() != 1 {
            return Err("package takes no arguments".into());
        }
        return package::build();
    }
    if args[0] == "audition" {
        return audition::run(&args[1..]);
    }
    let options = Options::parse(&args)?;
    if options.command == "stress" {
        stress(&options)
    } else {
        render(&options)
    }
}

fn new_file(path: &Path) -> std::io::Result<File> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn render(o: &Options) -> Result<(), Box<dyn Error>> {
    let output = o.output.as_ref().expect("validated output");
    let report_path = output.with_extension("json");
    let trace_path = output.with_extension("csv");
    for path in [
        Some(output),
        Some(&report_path),
        o.trace.then_some(&trace_path),
    ]
    .into_iter()
    .flatten()
    {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }
    let demo = o.command == "demo";
    let seconds = if demo { 10.0 } else { o.seconds };
    let frames = (seconds * o.rate as f64).round() as u32;
    let mut engine = Engine::new(o.rate as f64, o.profile())?;
    let mut audio = wav::FloatWav::new(BufWriter::new(new_file(output)?), o.rate, frames)?;
    let mut trace = if o.trace {
        Some(BufWriter::new(new_file(&trace_path)?))
    } else {
        None
    };
    if let Some(csv) = &mut trace {
        writeln!(
            csv,
            "time_s,displacement_m,velocity_m_s,contact_force_n,mechanical_energy_j,pickup_signal,contact_active,output"
        )?;
    }
    let mut peak = 0.0_f64;
    let mut square_sum = 0.0;
    let started = Instant::now();
    for frame in 0..frames {
        if demo {
            let second = o.rate;
            for (at, velocity) in [(0, 0.2), (2 * second, 0.55), (4 * second, 1.0)] {
                if frame == at {
                    engine.note_on(0, 57, velocity);
                }
                if frame == at + second {
                    engine.note_off(0, 57);
                }
            }
            if frame == 6 * second {
                engine.control_change(0, 64, 1.0);
                for note in [40, 47, 55, 59, 64, 66] {
                    engine.note_on(0, note, 0.65);
                }
            }
            if frame == 7 * second {
                engine.control_change(0, 123, 0.0);
            }
            if frame == 9 * second {
                engine.control_change(0, 64, 0.0);
            }
        } else {
            if frame == 0 {
                engine.note_on(0, o.note, o.velocity);
            }
            if frame == (o.hold * o.rate as f64).round() as u32 {
                engine.note_off(0, o.note);
            }
        }
        let sample = engine.next_sample();
        if !sample.is_finite() || engine.faults() != 0 {
            return Err("render encountered a numerical fault".into());
        }
        peak = peak.max(sample.abs() as f64);
        square_sum += (sample as f64).powi(2);
        audio.sample(sample)?;
        if let Some(csv) = &mut trace {
            let p = engine.probe(o.note).expect("validated note");
            writeln!(
                csv,
                "{:.9},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.9e}",
                (frame + 1) as f64 / o.rate as f64,
                p.displacement_m,
                p.velocity_m_s,
                p.contact_force_n,
                p.mechanical_energy_j,
                p.pickup_signal,
                u8::from(p.contact_active),
                sample
            )?;
        }
    }
    audio.finish()?;
    if let Some(csv) = &mut trace {
        csv.flush()?;
    }
    let elapsed = started.elapsed().as_secs_f64();
    let report = format!(
        "{{\n  \"schema_version\": 1,\n  \"model\": \"research-0.1.1-uncalibrated\",\n  \"mode\": \"{}\",\n  \"sample_rate\": {},\n  \"frames\": {},\n  \"note\": {},\n  \"velocity\": {},\n  \"hold_seconds\": {},\n  \"pickup_gap_mm\": {},\n  \"pickup_offset_mm\": {},\n  \"oversampling\": 4,\n  \"peak\": {:.9},\n  \"rms\": {:.9},\n  \"faults\": {},\n  \"render_wall_seconds_including_io\": {:.6}\n}}\n",
        o.command,
        o.rate,
        frames,
        o.note,
        o.velocity,
        o.hold,
        o.gap_mm,
        o.offset_mm,
        peak,
        (square_sum / frames as f64).sqrt(),
        engine.faults(),
        elapsed
    );
    new_file(&report_path)?.write_all(report.as_bytes())?;
    println!("Wrote {}\n{}", output.display(), report);
    if peak > 1.0 {
        eprintln!(
            "Float output exceeds full scale; lower monitor gain. WAV samples were not clipped."
        );
    }
    Ok(())
}

fn stress(o: &Options) -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new(o.rate as f64, o.profile())?;
    engine.note_on(0, 57, 0.5);
    for _ in 0..2048 {
        std::hint::black_box(engine.next_sample());
    }
    engine.reset();
    let mut times = Vec::new();
    let blocks = (3 * o.rate as usize).div_ceil(128);
    let mut peak = 0.0_f32;
    for block in 0..blocks {
        let start = Instant::now();
        if block % 188 == 0 {
            engine.control_change(0, 64, 1.0);
            for note in FIRST_NOTE..=LAST_NOTE {
                engine.note_on(0, note, 1.0);
            }
        }
        for _ in 0..128 {
            peak = peak.max(std::hint::black_box(engine.next_sample()).abs());
        }
        times.push(start.elapsed().as_secs_f64());
    }
    times.sort_by(f64::total_cmp);
    let deadline = 128.0 / o.rate as f64;
    let worst = times.last().copied().unwrap_or(0.0);
    let p99 = times[(times.len() * 99 / 100).min(times.len() - 1)];
    let misses = times.iter().filter(|t| **t > deadline).count();
    println!(
        "{{\"sample_rate\":{},\"keys\":73,\"block_frames\":128,\"blocks\":{},\"worst_ms\":{:.6},\"p99_ms\":{:.6},\"deadline_ms\":{:.6},\"deadline_misses\":{},\"peak\":{:.6},\"faults\":{}}}",
        o.rate,
        blocks,
        worst * 1000.0,
        p99 * 1000.0,
        deadline * 1000.0,
        misses,
        peak,
        engine.faults()
    );
    if engine.faults() != 0 {
        return Err("numerical fault during stress run".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn cli_rejects_invalid_and_ambiguous_input() {
        assert!(parse(&["render", "--output", "x.wav"]).is_ok());
        for args in [
            vec!["render", "--output", "x.wav", "--velocity", "NaN"],
            vec!["render", "--output", "x.wav", "--seconds", "1"],
            vec![
                "render", "--output", "x.wav", "--note", "57", "--note", "60",
            ],
            vec!["render", "--output", "x.csv"],
            vec!["demo", "--output", "x.wav", "--hold", "1"],
            vec!["render", "--output", "x.wav", "--unknown", "1"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
}
