use crate::memory_hammer_check::{Take, state, take};
use rf_rhodes_dsp::{
    HammerMemoryProfile, MemoryFreeStatus, MemoryHammer, MemoryHammerContactStatus,
    MemoryHammerContactStep, MemoryHammerProfile,
};
use serde_json::json;
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Certified free-recovery experiment:
  memory-free-check --output REPORT.json
  memory-contact-check --output REPORT.json
Compares bounded RK4 free intervals plus fine implicit contact against uniform
fine steps in 24 fixed-wall cases. No moving tine, audio or plugin integration.
Contact check also attempts certified RK4 compressed contact with unchanged accuracy gates.
";
fn adaptive(
    rate: u32,
    p: MemoryHammerProfile,
    speed: f64,
    substeps: usize,
    contact: bool,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * substeps as f64);
    let mut v = MemoryHammer::new(h, p, 0.0, speed)?;
    let frames = (f64::from(rate) * 0.016).ceil() as usize;
    let kick = (f64::from(rate) * 0.004).ceil() as usize;
    let mass = p.core_mass_kg + p.tip_mass_kg;
    let mut states = Vec::new();
    let mut forces = Vec::new();
    let mut fixed = 0usize;
    let mut free = 0usize;
    let mut rejected = 0usize;
    let mut contact_checks = 0usize;
    let mut free_ticks = 0usize;
    let mut level = 0u32;
    let mut balance = 0.0_f64;
    let mut material_balance = 0.0_f64;
    let mut momentum = 0.0_f64;
    let mut free_heat = 0.0;
    let mut reimpact = false;
    let mut pass = true;
    let mut max_interval = 0.0_f64;
    let mut contact_level = 0u32;
    let (mut contacts, mut contact_ticks, mut contact_accuracy, mut contact_boundary) =
        (0usize, 0usize, 0usize, 0usize);
    let mut max_contact_interval = 0.0_f64;
    for frame in 0..frames {
        if frame == kick {
            v.apply_core_impulse(2.5 * mass * speed)?;
        }
        let mut remaining = substeps;
        let mut force = 0.0;
        while remaining > 0 {
            let before = v.probe();
            let mut consumed = 1;
            let mut used_free = false;
            let mut used_contact = false;
            if contact && before.tip_position_m > 0.0 {
                loop {
                    let ticks = (1usize << contact_level)
                        .min(1usize << (usize::BITS - 1 - remaining.leading_zeros()));
                    let attempt = v.try_contact_step(h * ticks as f64)?;
                    match attempt.status {
                        MemoryHammerContactStatus::Advanced => {
                            consumed = ticks;
                            used_contact = true;
                            contacts += 1;
                            contact_ticks += ticks;
                            max_contact_interval = max_contact_interval.max(h * ticks as f64);
                            if attempt.normalized_state_error.unwrap_or(1.0)
                                < MemoryHammerContactStep::STATE_ERROR_LIMIT / 64.0
                                && attempt.relative_energy_defect.unwrap_or(1.0)
                                    < MemoryHammerContactStep::ENERGY_DEFECT_LIMIT / 64.0
                            {
                                contact_level = (contact_level + 1).min(12);
                            }
                            break;
                        }
                        MemoryHammerContactStatus::BoundaryRequired => {
                            contact_boundary += 1;
                        }
                        MemoryHammerContactStatus::AccuracyRequired => {
                            contact_accuracy += 1;
                        }
                    }
                    if contact_level == 0 {
                        break;
                    }
                    contact_level -= 1;
                }
            }
            if before.tip_position_m < 0.0 {
                loop {
                    let ticks = (1usize << level)
                        .min(1usize << (usize::BITS - 1 - remaining.leading_zeros()));
                    let attempt = v.try_free_step(h * ticks as f64)?;
                    match attempt.status {
                        MemoryFreeStatus::Advanced => {
                            consumed = ticks;
                            used_free = true;
                            free += 1;
                            free_ticks += ticks;
                            max_interval = max_interval.max(h * ticks as f64);
                            if attempt.normalized_state_error.unwrap_or(1.0) < 1e-10 / 64.0
                                && attempt.relative_energy_defect.unwrap_or(1.0) < 1e-13 / 64.0
                            {
                                level = (level + 1).min(12);
                            }
                            break;
                        }
                        MemoryFreeStatus::ContactRequired => {
                            contact_checks += 1;
                        }
                        MemoryFreeStatus::AccuracyRequired => {
                            rejected += 1;
                        }
                    }
                    if level == 0 {
                        break;
                    }
                    level -= 1;
                }
            }
            if !used_free && !used_contact {
                v.tick()?;
                fixed += 1;
                level = 0;
            }
            if used_free {
                contact_level = 0;
            }
            if used_contact {
                level = 0;
            }
            remaining -= consumed;
            let q = v.probe();
            let scale = q.initial_energy_j + q.absolute_impulse_work_j;
            balance = balance.max(q.balance_residual_j.abs() / scale);
            material_balance = material_balance.max(q.material.balance_residual_j.abs() / scale);
            let residual = (p.core_mass_kg * q.core_velocity_m_s
                + p.tip_mass_kg * q.tip_velocity_m_s
                + q.surface_impulse_n_s
                - q.applied_impulse_n_s
                - mass * speed)
                .abs()
                / (mass * speed + q.applied_impulse_n_s.abs());
            momentum = momentum.max(residual);
            if used_free {
                free_heat += q.material.last_step_heat_j;
            }
            reimpact |= frame >= kick && q.contact_force_n > 0.0;
            pass &= q.balance_residual_j.is_finite()
                && q.material.balance_residual_j.is_finite()
                && residual.is_finite()
                && q.material.last_step_heat_j >= 0.0
                && q.contact_force_n >= 0.0
                && q.mechanical_energy_j <= before.mechanical_energy_j + scale * 1e-10;
            force += q.contact_force_n * consumed as f64 / substeps as f64;
        }
        states.push(v.probe());
        forces.push(force);
    }
    pass &= balance < 1e-8
        && material_balance < 1e-8
        && momentum < 1e-10
        && free_heat > 0.0
        && reimpact
        && free > 0;
    pass &= !contact || contacts > 0;
    let mut report = json!({"passed":pass,"fixed_step_seconds":h,
        "fixed_ticks":fixed,"accepted_free_intervals":free,"accuracy_rejections":rejected,
        "uncertified_clearance_rejections":contact_checks,"uniform_ticks_replaced":free_ticks,
        "uniform_to_accepted_interval_ratio":(frames*substeps) as f64/(fixed+free+contacts) as f64,
        "maximum_free_interval_seconds":max_interval,"maximum_relative_energy_residual":balance,
        "maximum_relative_material_work_residual":material_balance,"maximum_relative_momentum_residual":momentum,
        "free_recovery_heat_j":free_heat,"reimpact_after_impulse":reimpact,"final_state":state(v.probe())});
    if contact {
        report["contact"] = json!({"accepted_intervals":contacts,"uniform_ticks_replaced":contact_ticks,
            "accuracy_rejections":contact_accuracy,"boundary_rejections":contact_boundary,
            "maximum_interval_seconds":max_contact_interval,
            "state_error_limit":MemoryHammerContactStep::STATE_ERROR_LIMIT,
            "local_energy_defect_limit":MemoryHammerContactStep::ENERGY_DEFECT_LIMIT});
    }
    Ok(Take {
        states,
        forces,
        pass,
        report,
    })
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let contact = args.first().is_some_and(|a| a == "memory-contact-check");
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for rate in [44100, 192000] {
        for tau in [0.0001, 0.001, 0.01] {
            for speed in [0.2, 0.8] {
                for tip in [0.0001, 0.0005] {
                    let p = MemoryHammerProfile {
                        core_mass_kg: 0.004 - tip,
                        tip_mass_kg: tip,
                        material: HammerMemoryProfile {
                            relaxation_seconds: tau,
                            ..HammerMemoryProfile::default()
                        },
                        ..MemoryHammerProfile::default()
                    };
                    let steps = (1.0 / (f64::from(rate) * 5e-9)).ceil() as usize * 4;
                    let a = adaptive(rate, p, speed, steps, contact)?;
                    let b = take(rate, p, speed, steps)?;
                    let velocity = (a
                        .states
                        .iter()
                        .zip(&b.states)
                        .map(|(a, b)| {
                            p.core_mass_kg * (a.core_velocity_m_s - b.core_velocity_m_s).powi(2)
                                + p.tip_mass_kg * (a.tip_velocity_m_s - b.tip_velocity_m_s).powi(2)
                        })
                        .sum::<f64>()
                        / (a.states.len() as f64 * 0.004 * speed * speed))
                        .sqrt();
                    let force = (a
                        .forces
                        .iter()
                        .zip(&b.forces)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        / b.forces.iter().map(|f| f * f).sum::<f64>())
                    .sqrt();
                    let impulse = a
                        .states
                        .iter()
                        .zip(&b.states)
                        .map(|(a, b)| {
                            (a.surface_impulse_n_s - b.surface_impulse_n_s).abs() / (0.004 * speed)
                        })
                        .fold(0.0_f64, f64::max);
                    let passed = a.pass
                        && b.pass
                        && velocity.is_finite()
                        && velocity < 0.01
                        && force.is_finite()
                        && force < 0.02
                        && impulse.is_finite()
                        && impulse < 0.01;
                    pass &= passed;
                    cases.push(json!({"sample_rate_hz":rate,"relaxation_seconds":tau,"launch_speed_m_s":speed,"tip_mass_kg":tip,
            "velocity_rmse_over_launch_speed":velocity,"output_mean_force_relative_rmse":force,"maximum_impulse_error_over_initial_momentum":impulse,
            "candidate":a.report,"reference":b.report,"passed":passed}));
                }
            }
        }
    }
    let mut report = json!({"schema_version":1,"experiment":"certified-hammer-free-motion-v1","status":if pass{"pass"}else{"fail"},
        "scope":"Fixed-wall experiment only. Continuous passive energy supplies a conservative tip-travel envelope before any free step. RK4 step doubling accepts two half steps without extrapolation. Local state and independent energy/work defects are checked; global errors are audited. Contact retains the original fine implicit solver.",
        "protocol":"Same 24 profiles and 16 ms/4 ms impulse protocol as memory-hammer-check. Candidate and reference share the original fine reference step. Dyadic free intervals never cross an output/event boundary. Each free attempt uses 12 RHS evaluations, so interval reduction is not a measured speedup.",
        "gates":{"relative_energy_and_material_work":1e-8,"relative_momentum":1e-10,"velocity_rmse":0.01,"mean_force_rmse":0.02,"impulse_error":0.01},
        "calibrated":false,"modal_integrated":false,"plugin_integrated":false,"cases":cases});
    if contact {
        report["experiment"] = json!("certified-hammer-contact-rk4-v1");
        report["scope"] = json!(
            "Fixed-wall RK4 contact with whole-interval compression certification, stage domain checks, step doubling without extrapolation, independent heat/material work/surface potential work quadratures, local passivity and momentum checks. Same free controller and original implicit ticks at unresolved boundaries. No modal coupling, calibration, pickup, audio or realtime qualification."
        );
        report["protocol"] = json!(
            "Same 24 profiles and 16 ms/4 ms impulse protocol as memory-hammer-check. Original uniform fine-step reference. Candidate attempts dyadic RK4 compressed contact and free intervals through level 12; no interval crosses an observation or impulse. Each accepted contact integrates normal impulse and reports its interval mean force. Twelve RHS evaluations per contact attempt; interval reduction is not a measured speedup."
        );
    }
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    if !pass {
        return Err("hammer motion audit failed; see report".into());
    }
    println!(
        "Hammer motion audit passed: 24 cases, 48 takes. Report: {}",
        args[2]
    );
    Ok(())
}
