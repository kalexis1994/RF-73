use super::convergence::OfflineFir;
use rf_rhodes_dsp::{MagneticPickup, ProductionDecimator, Profile, Voice};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, path::PathBuf};

pub const HELP: &str = "Pickup convergence:
  converge-pickup --output REPORT.json [--note 55] [--velocity 0.9]
    [--sample-rate 44100] [--seconds 0.1] [--gap-mm 1.5] [--offset-mm 0.5]
    [--reference-steps 128|256]
Production refined 4x and independent denser voices versus finite 128x or 256x.
Both laws; matched production-kernel support/delay; separate frozen-trajectory 4x.
Held note, duration 0.05..0.25 s, velocity 0.01..1; no alignment or level fitting.
This is numerical convergence, not an absolute aliasing or realism bound.
";

struct Options {
    output: PathBuf,
    rate: u32,
    note: u8,
    velocity: f64,
    seconds: f64,
    reference_steps: usize,
    profile: Profile,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let (pairs, remainder) = args[1..].as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(HELP.into());
        }
        let mut flags = BTreeMap::new();
        for [flag, value] in pairs {
            if !matches!(
                flag.as_str(),
                "--output"
                    | "--note"
                    | "--velocity"
                    | "--sample-rate"
                    | "--seconds"
                    | "--gap-mm"
                    | "--offset-mm"
                    | "--reference-steps"
            ) || flags.insert(flag.as_str(), value.as_str()).is_some()
            {
                return Err(
                    format!("unknown or duplicate pickup convergence option: {flag}").into(),
                );
            }
        }
        let gap: f64 = flags.get("--gap-mm").unwrap_or(&"1.5").parse()?;
        let offset: f64 = flags.get("--offset-mm").unwrap_or(&"0.5").parse()?;
        let options = Self {
            output: flags
                .get("--output")
                .ok_or("--output REPORT.json is required")?
                .into(),
            rate: flags.get("--sample-rate").unwrap_or(&"44100").parse()?,
            note: flags.get("--note").unwrap_or(&"55").parse()?,
            velocity: flags.get("--velocity").unwrap_or(&"0.9").parse()?,
            seconds: flags.get("--seconds").unwrap_or(&"0.1").parse()?,
            reference_steps: flags.get("--reference-steps").unwrap_or(&"128").parse()?,
            profile: Profile {
                pickup_gap_m: gap * 0.001,
                pickup_offset_m: offset * 0.001,
                ..Profile::default()
            },
        };
        if options.output.extension().is_none_or(|e| e != "json") || options.output.exists() {
            return Err("output must be a new .json file".into());
        }
        if ![44100, 48000, 96000, 192000].contains(&options.rate)
            || ![128, 256].contains(&options.reference_steps)
            || !(28..=100).contains(&options.note)
            || !options.seconds.is_finite()
            || !(0.05..=0.25).contains(&options.seconds)
            || !options.velocity.is_finite()
            || !(0.01..=1.0).contains(&options.velocity)
        {
            return Err("invalid rate, note, velocity or duration; see --help".into());
        }
        options.profile.validate(options.rate as f64)?;
        Ok(options)
    }
}

#[derive(Default, Serialize)]
struct Mechanics {
    internal_steps: usize,
    contact_substeps: usize,
    peak_displacement_m: f64,
    peak_velocity_m_s: f64,
    initial_energy_j: f64,
    final_energy_j: f64,
    maximum_positive_energy_step_j: f64,
    first_separation_seconds: Option<f64>,
}

type Signals = [Vec<f64>; 2];
struct Take {
    mechanics: Mechanics,
    displacement: Vec<f64>,
    velocity: Vec<f64>,
    signals: Signals,
}

enum Filter {
    Production(Box<ProductionDecimator>),
    Dense(OfflineFir),
}
impl Filter {
    fn new(steps: usize) -> Self {
        if steps == 4 {
            Self::Production(Box::default())
        } else {
            Self::Dense(OfflineFir::production_kernel(steps))
        }
    }
    fn push(&mut self, v: f64) {
        match self {
            Self::Production(f) => f.push(v),
            Self::Dense(f) => f.push(v),
        }
    }
    fn output(&self) -> f64 {
        (match self {
            Self::Production(f) => f.output(),
            Self::Dense(f) => f.output(),
        }) * 0.7
            * 0.12
    }
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let (reference, frozen) = render(&options, options.reference_steps, true)?;
    let mut rows = Vec::new();
    for steps in [4, 8, 16, 32, 64, 128]
        .into_iter()
        .filter(|&s| s < options.reference_steps)
    {
        let (take, _) = render(&options, steps, false)?;
        rows.push(json!({
            "path": if steps == 4 { "production_refined" } else { "independent_research" },
            "internal_steps": steps,
            "mechanics": take.mechanics,
            "displacement_nrmse": nrmse(&reference.displacement, &take.displacement),
            "velocity_nrmse": nrmse(&reference.velocity, &take.velocity),
            "laws": compare_signals(&reference.signals, &take.signals, options.rate),
        }));
    }
    rows.push(json!({
        "path": format!("frozen_{}x_trajectory_sampled_at_4x", options.reference_steps),
        "internal_steps": 4,
        "mechanics": null,
        "displacement_nrmse": null,
        "velocity_nrmse": null,
        "laws": compare_signals(&reference.signals, &frozen, options.rate),
    }));
    let report = json!({
        "schema_version": 1,
        "experiment": "pickup_and_mechanical_convergence_with_production_kernel",
        "model": "research-0.1.1-uncalibrated",
        "note": options.note, "velocity": options.velocity, "sample_rate": options.rate,
        "frames": reference.signals[0].len(),
        "duration_seconds": reference.signals[0].len() as f64 / options.rate as f64,
        "pickup_gap_mm": options.profile.pickup_gap_m * 1000.0,
        "pickup_offset_mm": options.profile.pickup_offset_m * 1000.0,
        "reference": reference.mechanics,
        "filter": "Actual production FIR at 4x; same normalized Blackman-sinc physical kernel sampled at each denser rate",
        "frozen_reference_stride": options.reference_steps / 4,
        "filter_support_output_samples": 31.5,
        "filter_delay_output_samples": 15.75,
        "filter_cutoff_over_output_rate": 0.42,
        "sampling_phase": "Output after every complete internal-step group; zero initial history; no delay compensation",
        "gain": "Same filtered * 0.7 * 0.12 in f64; output quantization excluded",
        "attack_frames": options.rate as usize * 32 / 1000,
        "faults": 0,
        "comparisons": rows,
        "limitations": [
            "Highest density is finite; inspect the next-lower independent residual and mechanical passivity before interpreting production error.",
            "Frozen path retains exact reference states at 4x endpoints: no interpolation or changed contact integration.",
            "Frozen residual includes transfer sampling AND discrete FIR-kernel sampling; it is not a pure aliasing bound.",
            "Independent paths combine contact, trajectory, transducer and filter discretization differences.",
            "Held single strike only; no reference recording, fitting, release, retrigger or hardware timing qualification."
        ]
    });
    super::analysis::write_report(&options.output, &report)?;
    println!("Pickup convergence: {}", options.output.display());
    Ok(())
}

fn render(
    options: &Options,
    steps: usize,
    freeze: bool,
) -> Result<(Take, Signals), Box<dyn Error>> {
    let mut voice = if steps == 4 {
        Voice::new(options.rate as f64, options.note, options.profile)?
    } else {
        Voice::new_for_convergence(options.rate as f64, options.note, options.profile, steps)?
    };
    if !voice.strike(options.velocity) {
        return Err("invalid research strike".into());
    }
    let pickup = MagneticPickup::new(
        options.profile.pickup_gap_m,
        options.profile.pickup_offset_m,
    )?;
    let frames = (options.seconds * options.rate as f64).round() as usize;
    let mut filters = [Filter::new(steps), Filter::new(steps)];
    let mut frozen_filters = [Filter::new(4), Filter::new(4)];
    let mut frozen: Signals =
        std::array::from_fn(|_| Vec::with_capacity(if freeze { frames } else { 0 }));
    let initial_energy = voice.probe().mechanical_energy_j;
    let mut previous_energy = initial_energy;
    let mut take = Take {
        mechanics: Mechanics {
            internal_steps: steps,
            contact_substeps: voice.contact_substeps(),
            initial_energy_j: initial_energy,
            ..Mechanics::default()
        },
        displacement: Vec::with_capacity(frames),
        velocity: Vec::with_capacity(frames),
        signals: std::array::from_fn(|_| Vec::with_capacity(frames)),
    };
    for frame in 0..frames {
        for step in 0..steps {
            let production = voice.tick();
            let p = voice.probe();
            let alternative = pickup.research_point_pole_voltage(p.displacement_m, p.velocity_m_s);
            if [
                production,
                alternative,
                p.displacement_m,
                p.velocity_m_s,
                p.mechanical_energy_j,
            ]
            .iter()
            .any(|v| !v.is_finite())
            {
                return Err("non-finite convergence trajectory or voltage".into());
            }
            for (i, v) in [production, alternative].into_iter().enumerate() {
                filters[i].push(v);
                if freeze && (step + 1).is_multiple_of(steps / 4) {
                    frozen_filters[i].push(v);
                }
            }
            let m = &mut take.mechanics;
            m.peak_displacement_m = m.peak_displacement_m.max(p.displacement_m.abs());
            m.peak_velocity_m_s = m.peak_velocity_m_s.max(p.velocity_m_s.abs());
            m.maximum_positive_energy_step_j = m
                .maximum_positive_energy_step_j
                .max(p.mechanical_energy_j - previous_energy);
            previous_energy = p.mechanical_energy_j;
            m.final_energy_j = p.mechanical_energy_j;
            if !p.contact_active && m.first_separation_seconds.is_none() {
                m.first_separation_seconds =
                    Some((frame * steps + step + 1) as f64 / (options.rate as f64 * steps as f64));
            }
        }
        take.displacement.push(voice.probe().displacement_m);
        take.velocity.push(voice.probe().velocity_m_s);
        for i in 0..2 {
            let sample = filters[i].output();
            let frozen_sample = frozen_filters[i].output();
            if !sample.is_finite() || !frozen_sample.is_finite() {
                return Err("non-finite filtered convergence output".into());
            }
            take.signals[i].push(sample);
            if freeze {
                frozen[i].push(frozen_sample);
            }
        }
    }
    Ok((take, frozen))
}

fn nrmse(reference: &[f64], candidate: &[f64]) -> Option<f64> {
    let power = reference.iter().map(|v| v * v).sum::<f64>();
    (power > 1e-30).then(|| {
        (reference
            .iter()
            .zip(candidate)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / power)
            .sqrt()
    })
}

fn residual(reference: &[f64], candidate: &[f64]) -> Value {
    let rms = |s: &[f64]| (s.iter().map(|v| v * v).sum::<f64>() / s.len() as f64).sqrt();
    let peak = |s: &[f64]| s.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let a = rms(reference);
    let b = rms(candidate);
    json!({ "raw_nrmse": nrmse(reference, candidate), "reference_rms": a, "candidate_rms": b,
        "candidate_minus_reference_rms_db": (a > 0.0 && b > 0.0).then(|| 20.0 * (b/a).log10()),
        "reference_peak": peak(reference), "candidate_peak": peak(candidate) })
}

fn compare_signals(reference: &Signals, candidate: &Signals, rate: u32) -> Vec<Value> {
    let attack = rate as usize * 32 / 1000;
    ["production", "research_point_pole"]
        .into_iter()
        .enumerate()
        .map(|(i, law)| {
            json!({
                "law": law,
                "full": residual(&reference[i], &candidate[i]),
                "attack_32_ms": residual(&reference[i][..attack], &candidate[i][..attack]),
                "after_attack": residual(&reference[i][attack..], &candidate[i][attack..]),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_preserves_gain_and_silence_is_unqualified() {
        assert_eq!(nrmse(&[1.0, -1.0], &[2.0, -2.0]), Some(1.0));
        assert_eq!(nrmse(&[0.0, 0.0], &[1.0, 1.0]), None);
        assert_eq!(nrmse(&[0.5, -0.5], &[0.5, -0.5]), Some(0.0));
    }

    #[test]
    fn production_path_matches_engine_and_frozen_4x_path_is_identity() {
        let options = Options {
            output: PathBuf::new(),
            rate: 44100,
            note: 100,
            velocity: 0.9,
            seconds: 0.05,
            reference_steps: 128,
            profile: Profile {
                pickup_gap_m: 0.0005,
                pickup_offset_m: 0.00025,
                ..Profile::default()
            },
        };
        let (take, frozen) = render(&options, 4, true).unwrap();
        assert_eq!(take.signals, frozen);
        let mut engine = rf_rhodes_dsp::Engine::new(options.rate as f64, options.profile).unwrap();
        engine.note_on(0, options.note, options.velocity);
        for &v in &take.signals[0] {
            assert_eq!((v as f32).to_bits(), engine.next_sample().to_bits());
        }
        assert_eq!(engine.faults(), 0);
    }
}
