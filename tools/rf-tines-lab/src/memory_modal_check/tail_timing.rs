//! Native timing in contiguous sections; diagnostic serialization is outside each timer.
use super::*;
use std::{hint::black_box, time::Instant};

pub const HELP: &str = "Stateful modal tail timing:
  memory-modal-tail-timing --output REPORT.json
  memory-modal-stiffness-timing --output REPORT.json
  memory-modal-trial-reuse-timing --output REPORT.json
  memory-modal-damping-timing --output REPORT.json
Four 128 ms profiles, default/capped RK4, three alternating-order repetitions.
Stiffness timing instead pairs diagonal/dense arithmetic in the default controller.
Trial-reuse timing pairs reused/recomputed contact trials in the default controller.
Damping timing pairs diagonal/dense contact damping in the default controller.
Measures preparation and 0-8, 8-32, 32-64, 64-128 ms execution separately.
Final-state checks do not replace memory-modal-tail-check. No realtime claim.
";
const SECTIONS: [(usize, usize); 4] = [(0, 384), (384, 1536), (1536, 3072), (3072, 6144)];

#[derive(Clone, Copy, PartialEq)]
enum Comparison {
    Controller,
    Stiffness,
    TrialReuse,
    Damping,
}
impl Comparison {
    fn experiment(self) -> &'static str {
        match self {
            Self::Controller => "memory-modal-tail-native-timing-v1",
            Self::Stiffness => "memory-modal-stiffness-native-timing-v1",
            Self::TrialReuse => "memory-modal-trial-reuse-native-timing-v1",
            Self::Damping => "memory-modal-damping-native-timing-v1",
        }
    }
    fn paths(self) -> [&'static str; 2] {
        match self {
            Self::Controller => ["rk4_default", "rk4_contact_2_free_8"],
            Self::Stiffness => ["diagonal_stiffness", "dense_stiffness"],
            Self::TrialReuse => ["reused_contact_trial", "recomputed_contact_trial"],
            Self::Damping => ["diagonal_contact_damping", "dense_contact_damping"],
        }
    }
    fn description(self) -> &'static str {
        match self {
            Self::Controller => "Default versus capped RK4 controller.",
            Self::Stiffness => {
                "Default RK4 controller in both paths; alternate diagonal and forced dense stiffness within this executable. All section states/controller reports must match exactly across both paths and every repetition."
            }
            Self::TrialReuse => {
                "Default RK4 controller and diagonal stiffness in both paths; alternate shared initial RHS/energy reuse and recomputed contact trials within this executable. All section states/controller reports must match exactly across both paths and every repetition."
            }
            Self::Damping => {
                "Default RK4 controller, diagonal stiffness and trial reuse in both paths; alternate exact-diagonal and forced dense contact damping within this executable. Coupled damper matrices stay dense. All section states/controller reports must match exactly across both paths and every repetition."
            }
        }
    }
}

fn measured(
    length: f64,
    tau: f64,
    comparison: Comparison,
    variant: bool,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let h = 1.0 / (48000.0 * 16672.0);
    let prep = Instant::now();
    let mut voice = MemoryModalAssembly::new(
        h,
        TineGeometry {
            length_m: length,
            ..TineGeometry::default()
        },
        ModalAssemblyProfile::default(),
        MemoryHammerProfile {
            material: HammerMemoryProfile {
                relaxation_seconds: tau,
                ..HammerMemoryProfile::default()
            },
            ..MemoryHammerProfile::default()
        },
        0.0,
        0.8,
    )?;
    if comparison == Comparison::Stiffness && variant {
        voice.use_dense_stiffness_reference();
    }
    if comparison == Comparison::Damping && variant {
        voice.use_dense_contact_damping_reference();
    }
    voice.prepare_free_steps(12)?;
    voice.prepare_rk4_contact()?;
    if comparison == Comparison::TrialReuse && variant {
        voice.use_recomputed_contact_trial_reference()?;
    }
    let mut controller = if comparison == Comparison::Controller && variant {
        Controller::with_rk4_limits(2, 8)?
    } else {
        Controller::with_rk4_contact()
    };
    let preparation_seconds = prep.elapsed().as_secs_f64();
    let mut sections = Vec::new();
    let mut total = 0.0;
    for (start, end) in SECTIONS {
        let timer = Instant::now();
        for frame in start..end {
            if frame == 96 {
                voice.apply_core_impulse(2.5 * 0.004 * 0.8)?;
            }
            if frame == 192 {
                voice.set_damped(true);
            }
            if frame == 288 {
                voice.set_damped(false);
            }
            let mut remaining = 16672;
            while remaining > 0 {
                remaining -= controller.advance(&mut voice, remaining)?;
                black_box(voice.probe());
            }
        }
        let elapsed = timer.elapsed().as_secs_f64();
        let probe = voice.probe();
        let scale = probe.hammer.initial_energy_j + probe.hammer.absolute_impulse_work_j;
        let residuals = [
            probe.balance_residual_j,
            probe.structural_work_residual_j,
            probe.hammer.balance_residual_j,
        ];
        if !elapsed.is_finite()
            || elapsed <= 0.0
            || !scale.is_finite()
            || scale <= 0.0
            || residuals
                .iter()
                .any(|x| !x.is_finite() || x.abs() / scale >= 1e-8)
        {
            return Err("invalid tail timing clock or section-end energy/work".into());
        }
        total += elapsed;
        sections.push(json!({"start_seconds":start as f64/48000.0,"end_seconds":end as f64/48000.0,
            "elapsed_seconds":elapsed,"seconds_per_simulated_second":elapsed/((end-start) as f64/48000.0),
            "final_state":state(probe),"cumulative_controller":controller.report(h)}));
    }
    Ok(json!({"path":comparison.paths()[usize::from(variant)],
        "elapsed_seconds":total,"preparation_seconds":preparation_seconds,"sections":sections,
        "free_operator_reserved_bytes":voice.free_operator_bytes(),"rk4_contact_operator_reserved_bytes":voice.rk4_contact_operator_bytes()}))
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let comparison = match args.first().map(String::as_str) {
        Some("memory-modal-tail-timing") => Comparison::Controller,
        Some("memory-modal-stiffness-timing") => Comparison::Stiffness,
        Some("memory-modal-trial-reuse-timing") => Comparison::TrialReuse,
        Some("memory-modal-damping-timing") => Comparison::Damping,
        _ => return Err(HELP.into()),
    };
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|x| x != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    for length in [0.075, 0.12] {
        for tau in [0.001, 0.01] {
            let mut runs = Vec::new();
            for repetition in 0..3 {
                for variant in if repetition % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let mut row = measured(length, tau, comparison, variant)?;
                    row["repetition"] = json!(repetition);
                    runs.push(row);
                }
            }
            if comparison != Comparison::Controller {
                let expected = &runs[0]["sections"];
                for run in &runs[1..] {
                    for i in 0..SECTIONS.len() {
                        for key in ["final_state", "cumulative_controller"] {
                            if run["sections"][i][key] != expected[i][key] {
                                return Err("paired timing changed state or controller".into());
                            }
                        }
                    }
                }
            }
            cases.push(json!({"tine_length_m":length,"relaxation_seconds":tau,"launch_speed_m_s":0.8,"runs":runs}));
            println!("Completed native tail timing: length {length} m, relaxation {tau} s.");
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":comparison.experiment(),
        "status":"pass","duration_seconds":0.128,"step_seconds":1.0/(48000.0*16672.0),
        "voice_inline_bytes":std::mem::size_of::<MemoryModalAssembly>(),"cases":cases,
        "comparison":comparison.description(),
        "scope":"Four strong-strike 128 ms profiles; three repetitions per compared path, alternating order. Same impulse at 2 ms and damper on/off at 4/6 ms. Timed regions include controller/rejections and consumed probes. Preparation, JSON and section-end energy/work checks are outside execution timers. Sections remain one continuous trajectory; reported controller counts are cumulative. No per-step audit inside timing: compare final states/counts with the independent tail audit. Section timers and intervening diagnostics can affect cache/load. Observations are not confidence intervals or universal speedups. No pickup voltage, mixing, polyphony, host, WASM deadlines or realtime qualification. No machine-dependent timing pass threshold."}),
    )?;
    writeln!(file)?;
    println!("Native tail timing completed: 24 runs. Report: {}", args[2]);
    Ok(())
}
