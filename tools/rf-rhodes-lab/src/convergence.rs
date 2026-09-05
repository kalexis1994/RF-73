//! Offline convergence experiment, separate from the production decimator.
use rf_rhodes_analysis::{AudioClip, Comparison, compare};
use rf_rhodes_dsp::{FIRST_NOTE, LAST_NOTE, Profile, Voice};
use serde::Serialize;
use std::{collections::BTreeSet, error::Error, f64::consts::PI, path::PathBuf};

pub const HELP: &str = "Convergence:
  converge --output REPORT.json [--note 57] [--velocity 0.7]
           [--sample-rate 48000] [--seconds 0.25]
Compares 4/8/16/32 internal steps against 64; default physical profile, held note.
Also compares production contact refinement against the same reference.
Output rate: 44100, 48000, 96000 or 192000 Hz. Duration: 0.05..1 second.
Velocity: 0.01..1, excluding strikes below the numerical measurement floor.
Common offline FIR, equal physical observation times, no alignment or gain hiding.
This measures finite-resolution differences, not an absolute aliasing bound.
";

struct Options {
    output: PathBuf,
    rate: u32,
    note: u8,
    velocity: f64,
    seconds: f64,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut result = Self {
            output: PathBuf::new(),
            rate: 48_000,
            note: 57,
            velocity: 0.7,
            seconds: 0.25,
        };
        let (pairs, remainder) = args[1..].as_chunks::<2>();
        if !remainder.is_empty() {
            return Err("convergence options require a flag and value".into());
        }
        let mut seen = BTreeSet::new();
        for [flag, value] in pairs {
            if !seen.insert(flag) {
                return Err(format!("duplicate option: {flag}").into());
            }
            match flag.as_str() {
                "--output" => result.output = value.into(),
                "--note" => result.note = value.parse()?,
                "--velocity" => result.velocity = value.parse()?,
                "--sample-rate" => result.rate = value.parse()?,
                "--seconds" => result.seconds = value.parse()?,
                _ => return Err(format!("unknown option: {flag}").into()),
            }
        }
        if result.output.extension().is_none_or(|s| s != "json") {
            return Err("--output REPORT.json is required".into());
        }
        if result.output.exists() {
            return Err(format!("refusing to overwrite {}", result.output.display()).into());
        }
        if ![44_100, 48_000, 96_000, 192_000].contains(&result.rate)
            || !(FIRST_NOTE..=LAST_NOTE).contains(&result.note)
            || !result.velocity.is_finite()
            || !(0.01..=1.0).contains(&result.velocity)
            || !result.seconds.is_finite()
            || !(0.05..=1.0).contains(&result.seconds)
        {
            return Err("invalid convergence rate, note, velocity or duration; see --help".into());
        }
        Ok(result)
    }
}

#[derive(Serialize)]
struct Mechanics {
    substeps: usize,
    contact_substeps: usize,
    internal_rate_hz: u32,
    initial_energy_j: f64,
    final_energy_j: f64,
    /// End of the first step that detects separation, quantized to internal dt.
    separation_seconds: Option<f64>,
    peak_step_average_force_n: f64,
    contact_impulse_n_s: f64,
    maximum_positive_energy_step_j: f64,
}

struct Take {
    mechanics: Mechanics,
    displacement: Vec<f64>,
    velocity: Vec<f64>,
    audio: AudioClip,
}

#[derive(Serialize)]
struct Row {
    mechanics: Mechanics,
    displacement_normalized_rmse: f64,
    velocity_normalized_rmse: f64,
    final_energy_relative_error: f64,
    contact_impulse_relative_error: f64,
    separation_error_seconds: Option<f64>,
    full_audio: Comparison,
    attack_audio: Comparison,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    method: &'static str,
    model: &'static str,
    output_rate_hz: u32,
    frames: usize,
    duration_seconds: f64,
    note: u8,
    velocity: f64,
    reference: Mechanics,
    filter_cutoff_over_output_rate: f64,
    filter_delay_output_samples: usize,
    attack_frames: usize,
    comparisons: Vec<Row>,
    production: Row,
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let frames = (options.seconds * options.rate as f64).round() as usize;
    let reference = render(options.rate, options.note, options.velocity, frames, 64)?;
    let attack_frames = (options.rate as usize * 32 / 1000).min(frames);
    let reference_attack = AudioClip::from_samples(
        options.rate,
        reference.audio.samples()[..attack_frames].to_vec(),
    )?;
    let make_row = |take: Take| -> Result<Row, Box<dyn Error>> {
        let attack =
            AudioClip::from_samples(options.rate, take.audio.samples()[..attack_frames].to_vec())?;
        Ok(Row {
            displacement_normalized_rmse: normalized_error(
                &reference.displacement,
                &take.displacement,
            ),
            velocity_normalized_rmse: normalized_error(&reference.velocity, &take.velocity),
            final_energy_relative_error: take.mechanics.final_energy_j
                / reference.mechanics.final_energy_j
                - 1.0,
            contact_impulse_relative_error: take.mechanics.contact_impulse_n_s
                / reference.mechanics.contact_impulse_n_s
                - 1.0,
            separation_error_seconds: take
                .mechanics
                .separation_seconds
                .zip(reference.mechanics.separation_seconds)
                .map(|(a, b)| a - b),
            full_audio: compare(&reference.audio, &take.audio, 0.0)?,
            attack_audio: compare(&reference_attack, &attack, 0.0)?,
            mechanics: take.mechanics,
        })
    };
    let mut comparisons = Vec::new();
    for substeps in [4, 8, 16, 32] {
        comparisons.push(make_row(render(
            options.rate,
            options.note,
            options.velocity,
            frames,
            substeps,
        )?)?);
    }
    let production = make_row(render_prepared(
        options.rate,
        options.velocity,
        frames,
        4,
        Voice::new(options.rate as f64, options.note, Profile::default())?,
    )?)?;
    let report = Report {
        schema_version: 1,
        method: "midpoint-discrete-gradient; common-blackman-fir-v1; fixed-time-no-alignment",
        model: "research-0.1.1-uncalibrated-default-profile",
        output_rate_hz: options.rate,
        frames,
        duration_seconds: frames as f64 / options.rate as f64,
        note: options.note,
        velocity: options.velocity,
        reference: reference.mechanics,
        filter_cutoff_over_output_rate: 0.42,
        filter_delay_output_samples: DELAY,
        attack_frames,
        comparisons,
        production,
    };
    super::analysis::write_report(&options.output, &report)?;
    println!(
        "Convergence: {} (64x finite reference)",
        options.output.display()
    );
    for row in report.comparisons {
        println!(
            "{}x: displacement_nrmse={:.6e} attack_audio_nrmse={:.6e} level_delta_db={:.6} energy_relative_error={:.6e}",
            row.mechanics.substeps,
            row.displacement_normalized_rmse,
            row.attack_audio.raw_normalized_rmse.unwrap_or(f64::NAN),
            row.full_audio
                .candidate_level_minus_reference_db
                .unwrap_or(f64::NAN),
            row.final_energy_relative_error
        );
    }
    println!(
        "Production 4x with {} contact substeps: attack_audio_nrmse={:.6e}",
        report.production.mechanics.contact_substeps,
        report
            .production
            .attack_audio
            .raw_normalized_rmse
            .unwrap_or(f64::NAN)
    );
    Ok(())
}

fn normalized_error(reference: &[f64], candidate: &[f64]) -> f64 {
    let error: f64 = reference
        .iter()
        .zip(candidate)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    let power: f64 = reference.iter().map(|x| x * x).sum();
    (error / power).sqrt()
}

fn render(
    rate: u32,
    note: u8,
    velocity: f64,
    frames: usize,
    substeps: usize,
) -> Result<Take, Box<dyn Error>> {
    render_prepared(
        rate,
        velocity,
        frames,
        substeps,
        Voice::new_for_convergence(rate as f64, note, Profile::default(), substeps)?,
    )
}

fn render_prepared(
    rate: u32,
    velocity: f64,
    frames: usize,
    substeps: usize,
    mut voice: Voice,
) -> Result<Take, Box<dyn Error>> {
    voice.strike(velocity);
    let initial_energy_j = voice.probe().mechanical_energy_j;
    let mut previous_energy = initial_energy_j;
    let mut mechanics = Mechanics {
        substeps,
        contact_substeps: voice.contact_substeps(),
        internal_rate_hz: rate * substeps as u32,
        initial_energy_j,
        final_energy_j: initial_energy_j,
        separation_seconds: None,
        peak_step_average_force_n: 0.0,
        contact_impulse_n_s: 0.0,
        maximum_positive_energy_step_j: 0.0,
    };
    let mut displacement = Vec::with_capacity(frames);
    let mut velocities = Vec::with_capacity(frames);
    let mut audio = Vec::with_capacity(frames);
    let mut filter = OfflineFir::new(substeps);
    let dt = 1.0 / mechanics.internal_rate_hz as f64;
    // Extra physical samples compensate the FIR delay without dropping attack
    // or padding the end with artificial silence. Mechanics use the original span.
    for frame in 0..frames + DELAY {
        for step in 0..substeps {
            let signal = voice.tick();
            if !signal.is_finite() {
                return Err("nonfinite convergence signal".into());
            }
            filter.push(signal);
            if frame < frames {
                let probe = voice.probe();
                if !probe.mechanical_energy_j.is_finite() || !probe.contact_force_n.is_finite() {
                    return Err("nonfinite convergence mechanics".into());
                }
                mechanics.maximum_positive_energy_step_j = mechanics
                    .maximum_positive_energy_step_j
                    .max(probe.mechanical_energy_j - previous_energy);
                previous_energy = probe.mechanical_energy_j;
                mechanics.peak_step_average_force_n = mechanics
                    .peak_step_average_force_n
                    .max(probe.contact_force_n);
                mechanics.contact_impulse_n_s += probe.contact_force_n * dt;
                if !probe.contact_active && mechanics.separation_seconds.is_none() {
                    mechanics.separation_seconds = Some((frame * substeps + step + 1) as f64 * dt);
                }
            }
        }
        if frame < frames {
            let probe = voice.probe();
            displacement.push(probe.displacement_m);
            velocities.push(probe.velocity_m_s);
            mechanics.final_energy_j = probe.mechanical_energy_j;
        }
        if frame >= DELAY {
            audio.push(filter.output());
        }
    }
    Ok(Take {
        mechanics,
        displacement,
        velocity: velocities,
        audio: AudioClip::from_samples(rate, audio)?,
    })
}

const DELAY: usize = 64;

/// A common physical kernel: 128 output-sample span, cutoff 0.42 * output Fs.
/// This intentionally does not reuse the shorter production decimator.
pub(super) struct OfflineFir {
    taps: Vec<f64>,
    history: Vec<f64>,
    cursor: usize,
}
impl OfflineFir {
    fn new(substeps: usize) -> Self {
        Self::with_half_length(substeps, DELAY * substeps)
    }
    /// Sample the production filter's physical kernel at a denser internal rate.
    /// Same 31.5-output-sample support and 15.75-sample group delay at every rate.
    pub(super) fn production_kernel(substeps: usize) -> Self {
        Self::with_half_length(substeps, 63 * substeps / 4)
    }
    fn with_half_length(substeps: usize, half_length: usize) -> Self {
        let length = 2 * half_length + 1;
        let cutoff = 0.42 / substeps as f64;
        let mut taps: Vec<_> = (0..length)
            .map(|i| {
                let x = i as f64 - (length - 1) as f64 / 2.0;
                let window = 0.42 - 0.5 * (2.0 * PI * i as f64 / (length - 1) as f64).cos()
                    + 0.08 * (4.0 * PI * i as f64 / (length - 1) as f64).cos();
                window
                    * if x == 0.0 {
                        2.0 * cutoff
                    } else {
                        (2.0 * PI * cutoff * x).sin() / (PI * x)
                    }
            })
            .collect();
        let sum: f64 = taps.iter().sum();
        taps.iter_mut().for_each(|tap| *tap /= sum);
        Self {
            taps,
            history: vec![0.0; length],
            cursor: 0,
        }
    }
    pub(super) fn push(&mut self, sample: f64) {
        self.history[self.cursor] = sample;
        self.cursor = (self.cursor + 1) % self.history.len();
    }
    pub(super) fn output(&self) -> f64 {
        self.taps[..self.cursor]
            .iter()
            .zip(self.history[..self.cursor].iter().rev())
            .chain(
                self.taps[self.cursor..]
                    .iter()
                    .zip(self.history[self.cursor..].iter().rev()),
            )
            .map(|(tap, sample)| tap * sample)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampled_production_kernel_matches_real_filter_and_preserves_physical_delay() {
        let mut real = rf_rhodes_dsp::ProductionDecimator::new();
        let mut sampled = OfflineFir::production_kernel(4);
        for i in 0..400 {
            let input = if i == 0 || i == 180 { 1.0 } else { 0.0 };
            real.push(input);
            sampled.push(input);
            assert!((real.output() - sampled.output()).abs() < 1e-14);
        }
        for steps in [4, 8, 16, 32, 64, 128, 256] {
            let filter = OfflineFir::production_kernel(steps);
            assert_eq!(filter.taps.len(), 126 * steps / 4 + 1);
            assert!((filter.taps.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            let center = filter.taps.len() / 2;
            assert_eq!(center as f64 / steps as f64, 15.75);
            for i in 0..center {
                assert!((filter.taps[i] - filter.taps[filter.taps.len() - 1 - i]).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn reference_filter_has_consistent_gain_delay_and_rejection() {
        for substeps in [4, 8, 16, 32, 64] {
            let mut filter = OfflineFir::new(substeps);
            let n = filter.taps.len();
            assert!((filter.taps.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            for i in 0..n {
                assert!((filter.taps[i] - filter.taps[n - 1 - i]).abs() < 1e-14);
            }
            for (frequency, passband) in [(0.05, true), (0.3, true), (0.5, false), (0.75, false)] {
                let omega = 2.0 * PI * frequency / substeps as f64;
                let re: f64 = filter
                    .taps
                    .iter()
                    .enumerate()
                    .map(|(i, t)| t * (omega * i as f64).cos())
                    .sum();
                let im: f64 = filter
                    .taps
                    .iter()
                    .enumerate()
                    .map(|(i, t)| t * (omega * i as f64).sin())
                    .sum();
                if passband {
                    assert!((re.hypot(im) - 1.0).abs() < 0.0001);
                } else {
                    assert!(
                        re.hypot(im) < 0.0001,
                        "{substeps}x at {frequency}: {}",
                        re.hypot(im)
                    );
                }
            }
            // An impulse tests ring ordering independently of the frequency response.
            for i in 0..2 * n {
                filter.push(if i == 0 { 1.0 } else { 0.0 });
                assert!(
                    (filter.output() - filter.taps.get(i).copied().unwrap_or(0.0)).abs() < 1e-14
                );
            }
        }
    }

    #[test]
    fn finer_steps_reduce_mechanical_and_attack_error_for_a3() {
        let reference = render(48_000, 57, 1.0, 2400, 64).unwrap();
        let coarse = render(48_000, 57, 1.0, 2400, 4).unwrap();
        let finer = render(48_000, 57, 1.0, 2400, 16).unwrap();
        assert!(
            normalized_error(&reference.displacement, &finer.displacement)
                < normalized_error(&reference.displacement, &coarse.displacement) * 0.2
        );
        assert!(
            normalized_error(reference.audio.samples(), finer.audio.samples())
                < normalized_error(reference.audio.samples(), coarse.audio.samples()) * 0.2
        );
        assert_eq!(reference.audio.samples().len(), 2400);
    }

    #[test]
    fn production_refinement_corrects_soft_treble_contact_error() {
        for rate in [44_100, 48_000] {
            let frames = rate as usize / 20;
            let reference = render(rate, 100, 0.2, frames, 64).unwrap();
            let coarse = render(rate, 100, 0.2, frames, 4).unwrap();
            let voice = Voice::new(rate as f64, 100, Profile::default()).unwrap();
            assert!(voice.contact_substeps() > 1);
            let production = render_prepared(rate, 0.2, frames, 4, voice).unwrap();
            let error = normalized_error(reference.audio.samples(), production.audio.samples());
            assert!(error < 0.002, "production error at {rate}: {error}");
            assert!(
                error < normalized_error(reference.audio.samples(), coarse.audio.samples()) * 0.02
            );
            assert!(production.mechanics.separation_seconds.is_some());
            assert!(production.mechanics.maximum_positive_energy_step_j < 1e-15);
        }
    }
}
