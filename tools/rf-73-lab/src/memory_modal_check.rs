use rf_73_dsp::{
    HammerMemoryProfile, MemoryHammerProfile, MemoryModalAssembly, MemoryModalProbe,
    ModalAssemblyProfile, TineGeometry,
};
use serde_json::json;
use std::{error::Error, io::Write, path::Path};
pub(crate) mod free_controller;
use free_controller::Controller;
mod refinement;
pub(crate) use refinement::{HELP as REFINEMENT_HELP, run as run_refinement};
mod tail;
pub(crate) use tail::{HELP as TAIL_HELP, run as run_tail};
mod tail_timing;
pub(crate) use tail_timing::{HELP as TAIL_TIMING_HELP, run as run_tail_timing};
mod checkpoint;
pub(crate) use checkpoint::{HELP as CHECKPOINT_HELP, run as run_checkpoint};
mod approach;
pub(crate) use approach::{HELP as APPROACH_HELP, run as run_approach};
mod recovery;
pub(crate) use recovery::{HELP as RECOVERY_HELP, run as run_recovery};
mod audio;
pub(crate) use audio::{HELP as AUDIO_HELP, run as render_audio};
mod tuning;
pub(crate) use tuning::{HELP as TUNING_HELP, run as tune_pitch};
mod geometry;
pub(crate) use geometry::{HELP as GEOMETRY_HELP, SPAN_HELP, run as sweep_geometry};
mod hammer_comparison;
pub(crate) use hammer_comparison::{HELP as HAMMER_COMPARISON_HELP, run as compare_hammers};

pub const HELP: &str = "Stateful multimode hammer:
  memory-modal-check --output REPORT.json [--coarse]
  memory-modal-free-check --output REPORT.json
  memory-modal-adaptive-check --output REPORT.json
  memory-modal-economical-check --output REPORT.json
  memory-modal-rk4-check --output REPORT.json
Audits reciprocal hammer/tine work, free recovery, reimpact and damper changes.
12 uncalibrated cases; uniform audit uses 2.5 ns steps and twofold finer reference.
Free audit uses certified longer intervals and 1.25 ns contact/reference steps.
Adaptive audit also tests error-controlled compressed contact intervals.
--coarse reproduces the preliminary 10/2.5 ns experiment, which can fail accuracy.
No audio device, pickup voltage or plugin integration.
";
struct Take {
    states: Vec<MemoryModalProbe>,
    forces: Vec<f64>,
    report: serde_json::Value,
    pass: bool,
    mass: [[f64; 9]; 9],
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContactMode {
    None,
    Strict,
    Economical,
    Rk4,
}
fn state(q: MemoryModalProbe) -> serde_json::Value {
    json!({"position":q.position,"velocity":q.velocity,"pickup_velocity_m_s":q.pickup_velocity_m_s,
        "core_position_m":q.hammer.core_position_m,"tip_position_m":q.hammer.tip_position_m,
        "core_velocity_m_s":q.hammer.core_velocity_m_s,"tip_velocity_m_s":q.hammer.tip_velocity_m_s,
        "material_deformation_m":q.hammer.material.displacement_m,"viscous_deformation_m":q.hammer.material.viscous_deformation_m,
        "branch_extension_m":q.hammer.material.branch_extension_m,"material_energy_j":q.hammer.material.stored_energy_j,
        "contact_energy_j":q.hammer.surface_energy_j,"structural_energy_j":q.structural_energy_j,
        "mechanical_energy_j":q.mechanical_energy_j,"structural_heat_j":q.structural_heat_j,
        "material_heat_j":q.hammer.material.dissipated_energy_j,"hammer_to_structure_work_j":q.hammer.surface_work_j,
        "impulse_work_j":q.hammer.external_work_j,"balance_residual_j":q.balance_residual_j})
}
fn take(length: f64, speed: f64, tau: f64, substeps: usize) -> Result<Take, Box<dyn Error>> {
    take_impl(length, speed, tau, substeps, false, ContactMode::None)
}
fn take_impl(
    length: f64,
    speed: f64,
    tau: f64,
    substeps: usize,
    adaptive: bool,
    mode: ContactMode,
) -> Result<Take, Box<dyn Error>> {
    take_configured(
        length,
        speed,
        tau,
        substeps,
        adaptive,
        mode,
        TakeConfig::default(),
    )
}
#[derive(Clone, Copy)]
struct TakeConfig {
    frames: usize,
    rk4_limits: Option<(u32, u32)>,
    late_reimpacts: bool,
}
impl Default for TakeConfig {
    fn default() -> Self {
        Self {
            frames: 384,
            rk4_limits: None,
            late_reimpacts: false,
        }
    }
}
fn take_configured(
    length: f64,
    speed: f64,
    tau: f64,
    substeps: usize,
    adaptive: bool,
    mode: ContactMode,
    config: TakeConfig,
) -> Result<Take, Box<dyn Error>> {
    if !(384..=6144).contains(&config.frames)
        || (config.rk4_limits.is_some() && (!adaptive || mode != ContactMode::Rk4))
        || (config.late_reimpacts && config.frames != 6144)
    {
        return Err("invalid modal diagnostic configuration".into());
    }
    let contact = mode != ContactMode::None;
    let h = 1.0 / (48000.0 * substeps as f64);
    let mut v = MemoryModalAssembly::new(
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
        speed,
    )?;
    let mut controller = if let Some((contact, free)) = config.rk4_limits {
        Controller::with_rk4_limits(contact, free)?
    } else if mode == ContactMode::Rk4 {
        Controller::with_rk4_contact()
    } else if mode == ContactMode::Economical {
        Controller::with_economical_contact()
    } else if contact {
        Controller::with_contact()
    } else {
        Controller::default()
    };
    if adaptive {
        v.prepare_free_steps(12)?;
    }
    if mode == ContactMode::Rk4 {
        v.prepare_rk4_contact()?;
    } else if contact {
        v.prepare_contact_steps(12)?;
    }
    let mut states = Vec::new();
    let mut forces = Vec::new();
    let mut events = Vec::new();
    let mut balance = 0.0_f64;
    let mut port_balance = 0.0_f64;
    let mut hammer_balance = 0.0_f64;
    let mut positive = 0.0_f64;
    let mut free_heat = 0.0;
    let mut reimpact = false;
    let mut late = LateReimpacts::default();
    let mut pass = true;
    let mut peaks = [0.0_f64; 9];
    for frame in 0..config.frames {
        let event = diagnostic_event(frame, speed, config.late_reimpacts);
        if let Some(event) = event {
            let before = v.probe();
            match event {
                DiagnosticEvent::Impulse(impulse) => v.apply_core_impulse(impulse)?,
                DiagnosticEvent::Damper(damped) => v.set_damped(damped),
            }
            let after = v.probe();
            pass &= before.hammer.material == after.hammer.material;
            if matches!(event, DiagnosticEvent::Damper(_)) {
                pass &= before == after;
            }
            events.push(
                json!({"event":match event {DiagnosticEvent::Impulse(_)=>"core_impulse",DiagnosticEvent::Damper(true)=>"damper_on",DiagnosticEvent::Damper(false)=>"damper_off"},
                "time_seconds":frame as f64/48000.0,"before":state(before),"after":state(after)}),
            );
        }
        let mut force = 0.0;
        let mut remaining = substeps;
        while remaining > 0 {
            let before = v.probe();
            let consumed = if adaptive {
                controller.advance(&mut v, remaining)?
            } else {
                v.tick()?;
                1
            };
            remaining -= consumed;
            let q = v.probe();
            let scale = q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j;
            balance = balance.max(q.balance_residual_j.abs() / scale);
            port_balance = port_balance.max(q.structural_work_residual_j.abs() / scale);
            hammer_balance = hammer_balance.max(q.hammer.balance_residual_j.abs() / scale);
            positive = positive.max((q.mechanical_energy_j - before.mechanical_energy_j) / scale);
            if q.hammer.contact_force_n == 0.0 && before.hammer.contact_force_n == 0.0 {
                free_heat += q.hammer.material.last_step_heat_j;
            }
            reimpact |= frame >= 96 && q.hammer.contact_force_n > 0.0;
            if config.late_reimpacts && frame >= 1536 {
                late.observe(
                    frame,
                    (frame as f64 + (substeps - remaining) as f64 / substeps as f64) / 48000.0,
                    q,
                );
            }
            pass &= q.balance_residual_j.is_finite()
                && q.structural_work_residual_j.is_finite()
                && q.hammer.balance_residual_j.is_finite()
                && q.hammer.contact_force_n.is_finite()
                && q.hammer.contact_force_n >= 0.0
                && q.hammer.material.last_step_heat_j >= 0.0
                && q.structural_heat_j >= before.structural_heat_j;
            for (peak, x) in peaks.iter_mut().zip(q.position) {
                *peak = peak.max(x.abs());
            }
            force += controller.mean_force().unwrap_or(q.hammer.contact_force_n) * consumed as f64
                / substeps as f64;
        }
        states.push(v.probe());
        forces.push(force);
    }
    pass &= balance < 1e-8
        && port_balance < 1e-8
        && hammer_balance < 1e-8
        && positive < 1e-10
        && free_heat > 0.0
        && reimpact
        && peaks.iter().all(|x| *x > 0.0);
    let mut result = Take {
        states,
        forces,
        mass: v.mass_matrix(),
        pass,
        report: json!({"substeps":substeps,"step_seconds":h,"maximum_relative_energy_residual":balance,
            "maximum_relative_structural_work_residual":port_balance,"maximum_relative_hammer_work_residual":hammer_balance,
            "maximum_positive_relative_energy_step":positive,"force_free_material_heat_j":free_heat,
            "reimpact_after_impulse":reimpact,"coordinate_absolute_peaks":peaks,"events":events,
            "final_state":state(v.probe()),"passed":pass}),
    };
    if adaptive {
        result.report["free_controller"] = controller.report(h);
        result.pass &= result.report["free_controller"]["accepted_free_intervals"]
            .as_u64()
            .unwrap_or(0)
            > 0;
        result.report["passed"] = json!(result.pass);
    }
    if config.frames != 384 || config.rk4_limits.is_some() {
        result.report["observation_frames"] = json!(config.frames);
        result.report["duration_seconds"] = json!(config.frames as f64 / 48000.0);
    }
    if contact {
        result.pass &= result.report["free_controller"]["contact"]["accepted_intervals"]
            .as_u64()
            .unwrap_or(0)
            > 0;
        result.report["passed"] = json!(result.pass);
    }
    if config.late_reimpacts {
        result.pass &= late.contacts.iter().all(Option::is_some);
        result.report["late_reimpact_first_contact_seconds"] = json!(late.contacts);
        result.report["late_reimpact_observed_separation"] = json!(late.separated);
        result.report["passed"] = json!(result.pass);
    }
    Ok(result)
}
#[derive(Default)]
struct LateReimpacts {
    separated: [bool; 2],
    contacts: [Option<f64>; 2],
}
impl LateReimpacts {
    fn observe(&mut self, frame: usize, end_seconds: f64, q: MemoryModalProbe) {
        if !(1536..6144).contains(&frame) {
            return;
        }
        let index = usize::from(frame >= 3840);
        self.separated[index] |=
            q.hammer.surface_energy_j == 0.0 && q.hammer.contact_force_n == 0.0;
        if self.separated[index] && q.hammer.contact_force_n > 0.0 {
            self.contacts[index].get_or_insert(end_seconds);
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum DiagnosticEvent {
    Impulse(f64),
    Damper(bool),
}
fn diagnostic_event(frame: usize, speed: f64, late: bool) -> Option<DiagnosticEvent> {
    match frame {
        96 => Some(DiagnosticEvent::Impulse(2.5 * 0.004 * speed)),
        192 => Some(DiagnosticEvent::Damper(true)),
        288 => Some(DiagnosticEvent::Damper(false)),
        1536 | 3840 if late => Some(DiagnosticEvent::Impulse(0.008)),
        1920 | 4608 if late => Some(DiagnosticEvent::Damper(true)),
        2688 | 5376 if late => Some(DiagnosticEvent::Damper(false)),
        _ => None,
    }
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let rk4_protocol = "Initial impact at zero gap; core impulse at 2 ms, damper on at 4 ms, off at 6 ms, finish at 8 ms. Candidate uses certified free recovery and coupled RK4 compressed contact, sharing 16672 base ticks per 48 kHz observation with the uniform implicit reference. Levels 0..12 cannot cross observations or events; uncertified boundaries and rejected minimum intervals use one original tick. Each contact trial checks one whole and two half steps, without extrapolation, using independent material heat/work, structural damping, moving-port work and surface-potential work quadratures. Mean force is integrated normal impulse divided by the whole interval. Interval counts are not measured speedups.";
    let rk4 = args[0] == "memory-modal-rk4-check";
    let economical = args[0] == "memory-modal-economical-check";
    let contact = rk4 || economical || args[0] == "memory-modal-adaptive-check";
    let adaptive = contact || args[0] == "memory-modal-free-check";
    if !matches!(args.len(), 3 | 4)
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let coarse = args.len() == 4;
    if coarse && (adaptive || args[3] != "--coarse") {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for length in [0.05, 0.075, 0.12] {
        for speed in [0.2, 0.8] {
            for tau in [0.001, 0.01] {
                let a = take_impl(
                    length,
                    speed,
                    tau,
                    if adaptive {
                        16672
                    } else if coarse {
                        2084
                    } else {
                        8336
                    },
                    adaptive,
                    if rk4 {
                        ContactMode::Rk4
                    } else if economical {
                        ContactMode::Economical
                    } else if contact {
                        ContactMode::Strict
                    } else {
                        ContactMode::None
                    },
                )?;
                let b = take(length, speed, tau, if coarse { 8336 } else { 16672 })?;
                let mut kinetic_error = 0.0;
                let mut pickup_error = 0.0;
                let mut pickup_power = 0.0;
                for (a, b) in a.states.iter().zip(&b.states) {
                    pickup_error += (a.pickup_velocity_m_s - b.pickup_velocity_m_s).powi(2);
                    pickup_power += b.pickup_velocity_m_s.powi(2);
                    kinetic_error += 0.0038
                        * (a.hammer.core_velocity_m_s - b.hammer.core_velocity_m_s).powi(2)
                        + 0.0002 * (a.hammer.tip_velocity_m_s - b.hammer.tip_velocity_m_s).powi(2);
                }
                for (qa, qb) in a.states.iter().zip(&b.states) {
                    let dv: [f64; 9] = core::array::from_fn(|i| qa.velocity[i] - qb.velocity[i]);
                    kinetic_error += (0..9)
                        .map(|i| dv[i] * (0..9).map(|j| a.mass[i][j] * dv[j]).sum::<f64>())
                        .sum::<f64>();
                }
                let velocity_error =
                    (kinetic_error / (a.states.len() as f64 * 0.004 * speed * speed)).sqrt();
                let pickup_error = (pickup_error / pickup_power).sqrt();
                let force_error = (a
                    .forces
                    .iter()
                    .zip(&b.forces)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    / b.forces.iter().map(|f| f * f).sum::<f64>())
                .sqrt();
                let passed = a.pass
                    && b.pass
                    && velocity_error.is_finite()
                    && velocity_error < 0.01
                    && pickup_error.is_finite()
                    && pickup_error < 0.01
                    && force_error.is_finite()
                    && force_error < 0.02;
                pass &= passed;
                cases.push(json!({"tine_length_m":length,"launch_speed_m_s":speed,"relaxation_seconds":tau,
                    "mass_matrix":a.mass,"kinetic_metric_velocity_rmse_over_launch_speed":velocity_error,
                    "pickup_velocity_relative_rmse":pickup_error,"output_mean_force_relative_rmse":force_error,
                    "candidate":a.report,"reference":b.report,"passed":passed}));
            }
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":if rk4 {"memory-modal-rk4-v1"} else if economical {"memory-modal-economical-v1"} else if contact {"memory-modal-adaptive-v1"} else if adaptive {"memory-modal-free-v1"} else {"memory-modal-coupling-v1"},
        "status":if pass{"pass"}else{"fail"},"calibrated":false,"plugin_integrated":false,
        "observation_rate_hz":48000,"duration_seconds":0.008,
        "coarse":coarse,
        "protocol":if rk4 {rk4_protocol} else if contact {"Initial impact at zero gap. Core impulse equal to 2.5 times initial momentum at 2 ms; damper on at 4 ms, off at 6 ms; finish at 8 ms. Candidate combines certified free recovery with step-doubled compressed contact, using the reported contact state-error limit. Reference uses 16672 uniform steps per 48 kHz frame. Candidate shares that base step and permits dyadic levels through 12. No interval crosses an observation or external event; uncertain contact boundaries and failed smallest attempts use one original fine tick. Accepted contact commits two half steps without extrapolation; mean force averages both reactions. Every accepted interval checks energy and both port work balances. Interval counts are not runtime speedups."} else if adaptive {"Initial impact at zero gap. Core impulse equal to 2.5 times initial momentum at 2 ms; damper on at 4 ms, off at 6 ms; finish at 8 ms. Candidate uses certified dyadic free intervals and fine implicit contact; reference uses 16672 uniform steps per 48 kHz frame. Candidate uses the same base step, with maximum level 12; intervals cannot cross observation or event boundaries. Every accepted interval checks energy and both port work balances. Counts include rejections separately; interval ratios are not runtime speedups."} else {"Initial impact at zero gap. Core impulse equal to 2.5 times initial momentum at 2 ms; damper on at 4 ms, off at 6 ms; finish at 8 ms. Memory and all coordinates persist. Default 8336 versus 16672 uniform microsteps per output frame; coarse 2084 versus 8336."},
        "scope":"Nine reciprocal structural coordinates plus two hammer masses and one material memory state. Provisional default profiles except case-specific length, launch speed and relaxation time. Surface coefficient 1e12 N/m2; core/tip masses 3.8/0.2 g. No action, pickup voltage, calibration or realtime qualification.",
        "gates":{"energy_and_each_port_work_residual":1e-8,"positive_energy_step":1e-10,
            "kinetic_velocity_rmse":0.01,"pickup_velocity_rmse":0.01,"mean_force_rmse":0.02},"cases":cases}),
    )?;
    writeln!(file)?;
    if !pass {
        return Err("stateful modal hammer audit failed; see report".into());
    }
    println!(
        "Stateful modal hammer audit passed: 12 cases, 24 takes. Report: {}",
        args[2]
    );
    Ok(())
}
