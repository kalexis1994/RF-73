use rf_rhodes_analysis::{AudioClip, ToneComparisonOptions, compare_tone};
use rf_rhodes_dsp::{MagneticPickup, OVERSAMPLE, ProductionDecimator, Voice};
use serde::Serialize;
use std::{error::Error, io::BufWriter, path::Path};

pub const HELP: &str = "Mechanical pickup pair:
  render-pickup-pair --output BASE.wav [render options, except --trace]
Writes BASE.wav (production), BASE-point-pole.wav and BASE-pickup-pair.json.
One production mechanical trajectory feeds both laws and identical production FIRs.
Duration 0.65..10 s; hold at least 0.6 s and shorter than duration.
Same raw output gain; no normalization, inferred reference velocity or plugin change.
";

#[derive(Default, Serialize)]
struct Mechanics {
    internal_samples: usize,
    contact_substeps: usize,
    peak_displacement_m: f64,
    peak_velocity_m_s: f64,
    peak_step_average_force_n: f64,
    contact_impulse_n_s: f64,
    first_separation_seconds: Option<f64>,
    final_energy_j: f64,
}

#[derive(Serialize)]
struct Levels {
    peak: f64,
    rms: f64,
    exceeds_full_scale: bool,
}

struct Pair {
    production: Vec<f32>,
    point_pole: Vec<f32>,
    mechanics: Mechanics,
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut render_args = args.to_vec();
    render_args[0] = "render".into();
    let options = super::Options::parse(&render_args)?;
    if options.trace || !(0.65..=10.0).contains(&options.seconds) || options.hold < 0.6 {
        return Err(HELP.into());
    }
    let production = options.output.as_ref().expect("validated output");
    let stem = production
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("output needs a UTF-8 file stem")?;
    let point_pole = production.with_file_name(format!("{stem}-point-pole.wav"));
    let report_path = production.with_file_name(format!("{stem}-pickup-pair.json"));
    for path in [production, &point_pole, &report_path] {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }

    // Finish numerical checks and analysis before creating any output files.
    let pair = render_pair(&options)?;
    let a = AudioClip::from_samples(
        options.rate,
        pair.production.iter().map(|&v| v as f64).collect(),
    )?;
    let b = AudioClip::from_samples(
        options.rate,
        pair.point_pole.iter().map(|&v| v as f64).collect(),
    )?;
    let tone = compare_tone(
        &a,
        &b,
        ToneComparisonOptions {
            note: options.note,
            ..ToneComparisonOptions::default()
        },
    )?;
    let production_levels = levels(&pair.production);
    let point_pole_levels = levels(&pair.point_pole);
    let matching_gain =
        (point_pole_levels.rms > 0.0).then(|| production_levels.rms / point_pole_levels.rms);
    let report = serde_json::json!({
        "schema_version": 1,
        "experiment": "same_mechanical_trajectory_pickup_pair",
        "model": "research-0.1.1-uncalibrated",
        "production_wav": production,
        "point_pole_wav": point_pole,
        "sample_rate": options.rate,
        "frames": pair.production.len(),
        "note": options.note,
        "velocity": options.velocity,
        "hold_seconds": options.hold,
        "release_frame": (options.hold * options.rate as f64).round() as usize,
        "pickup_gap_mm": options.gap_mm,
        "pickup_offset_mm": options.offset_mm,
        "oversampling": OVERSAMPLE,
        "filter": "Production 127-tap Blackman FIR, cutoff 0.42 output Fs, 63 internal samples delay; no delay compensation",
        "gain": "Both paths: filtered signal * 0.7 * 0.12, then f32; no clipping or normalization",
        "production_levels": production_levels,
        "point_pole_levels": point_pole_levels,
        "point_pole_gain_to_match_full_clip_production_rms_not_applied": matching_gain,
        "mechanics": pair.mechanics,
        "faults": 0,
        "tone_comparison": tone,
        "limitations": [
            "One production voice supplies identical internal-rate position and velocity to both laws.",
            "Point-pole is a single-coordinate field-as-flux proxy, not the full published magnetic geometry.",
            "Actual production filtering is included; no high-rate reference or absolute aliasing bound.",
            "No inferred real-instrument strike velocity, fitting, held-out calibration or realism score."
        ]
    });
    write_wav(production, options.rate, &pair.production)?;
    write_wav(&point_pole, options.rate, &pair.point_pole)?;
    super::analysis::write_report(&report_path, &report)?;
    println!("Pickup pair: {}", report_path.display());
    Ok(())
}

fn render_pair(options: &super::Options) -> Result<Pair, Box<dyn Error>> {
    let profile = options.profile();
    let mut voice = Voice::new(options.rate as f64, options.note, profile)?;
    let pickup = MagneticPickup::new(profile.pickup_gap_m, profile.pickup_offset_m)?;
    let mut filters = [ProductionDecimator::new(), ProductionDecimator::new()];
    if !voice.strike(options.velocity) {
        return Err("invalid pair strike velocity".into());
    }
    let frames = (options.seconds * options.rate as f64).round() as usize;
    let release = (options.hold * options.rate as f64).round() as usize;
    let dt = 1.0 / (options.rate as f64 * OVERSAMPLE as f64);
    let mut result = Pair {
        production: Vec::with_capacity(frames),
        point_pole: Vec::with_capacity(frames),
        mechanics: Mechanics {
            contact_substeps: voice.contact_substeps(),
            ..Mechanics::default()
        },
    };
    for frame in 0..frames {
        if frame == release {
            voice.set_damped(true);
        }
        for step in 0..OVERSAMPLE {
            let production_signal = voice.tick();
            let probe = voice.probe();
            let alternative =
                pickup.research_point_pole_voltage(probe.displacement_m, probe.velocity_m_s);
            if [
                production_signal,
                alternative,
                probe.displacement_m,
                probe.velocity_m_s,
                probe.mechanical_energy_j,
                probe.contact_force_n,
            ]
            .iter()
            .any(|v| !v.is_finite())
            {
                return Err("non-finite pickup pair mechanics or voltage".into());
            }
            filters[0].push(production_signal);
            filters[1].push(alternative);
            let m = &mut result.mechanics;
            m.internal_samples += 1;
            m.peak_displacement_m = m.peak_displacement_m.max(probe.displacement_m.abs());
            m.peak_velocity_m_s = m.peak_velocity_m_s.max(probe.velocity_m_s.abs());
            m.peak_step_average_force_n = m.peak_step_average_force_n.max(probe.contact_force_n);
            m.contact_impulse_n_s += probe.contact_force_n * dt;
            m.final_energy_j = probe.mechanical_energy_j;
            if !probe.contact_active && m.first_separation_seconds.is_none() {
                m.first_separation_seconds = Some((frame * OVERSAMPLE + step + 1) as f64 * dt);
            }
        }
        // Match Engine's default gain and exact operation order. No dynamic gain
        // or post-mix nonlinearity belongs in this single-strike experiment.
        let output = filters
            .each_ref()
            .map(|filter| (filter.output() * 0.7 * 0.12) as f32);
        if output.iter().any(|v| !v.is_finite()) {
            return Err("non-finite pickup pair output".into());
        }
        result.production.push(output[0]);
        result.point_pole.push(output[1]);
    }
    Ok(result)
}

fn levels(samples: &[f32]) -> Levels {
    let peak = samples
        .iter()
        .map(|&v| (v as f64).abs())
        .fold(0.0, f64::max);
    let rms =
        (samples.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
    Levels {
        peak,
        rms,
        exceeds_full_scale: peak > 1.0,
    }
}

fn write_wav(path: &Path, rate: u32, samples: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut wav = super::wav::FloatWav::new(
        BufWriter::new(super::new_file(path)?),
        rate,
        samples.len() as u32,
    )?;
    for &sample in samples {
        wav.sample(sample)?;
    }
    wav.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rf_rhodes_dsp::Engine;

    #[test]
    fn paired_production_is_sample_identical_to_engine_including_release() {
        for rate in [44100, 48000, 96000, 192000] {
            for note in [28, 55, 100] {
                for (gap, offset, velocity) in [(1.5, 0.5, 0.2), (0.5, 0.25, 0.9)] {
                    let options = super::super::Options {
                        laboratory: false,
                        command: "render".into(),
                        output: None,
                        note,
                        velocity,
                        rate,
                        seconds: 0.05,
                        hold: 0.025,
                        gap_mm: gap,
                        offset_mm: offset,
                        trace: false,
                    };
                    let pair = render_pair(&options).unwrap();
                    let mut engine = Engine::new(rate as f64, options.profile()).unwrap();
                    engine.note_on(0, note, velocity);
                    for (frame, &actual) in pair.production.iter().enumerate() {
                        if frame == (0.025 * rate as f64).round() as usize {
                            engine.note_off(0, note);
                        }
                        assert_eq!(
                            actual.to_bits(),
                            engine.next_sample().to_bits(),
                            "rate={rate}, note={note}, frame={frame}"
                        );
                    }
                    assert_eq!(engine.faults(), 0);
                    assert!(pair.mechanics.first_separation_seconds.is_some());
                    assert!(pair.point_pole.iter().any(|&v| v != 0.0));
                    assert!(
                        pair.point_pole
                            .iter()
                            .zip(&pair.production)
                            .any(|(a, b)| a != b)
                    );
                }
            }
        }
    }
}
