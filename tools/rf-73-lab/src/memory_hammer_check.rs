use rf_73_dsp::{HammerMemoryProfile, MemoryHammer, MemoryHammerProbe, MemoryHammerProfile};
use serde_json::json;
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Material-memory impact experiment:
  memory-hammer-check --output REPORT.json [--substeps N]
Audits two-mass impacts, separation and impulse-driven reimpact against a fixed
elastic penalty surface. No tine, plugin, audio device or material calibration.
Default candidate step is at most 5 ns, with four times finer reference steps.
Optional uniform substeps: 1..1302 per output frame; coarse runs can fail accuracy.
";

pub(super) struct Take {
    pub report: serde_json::Value,
    pub states: Vec<MemoryHammerProbe>,
    pub forces: Vec<f64>,
    pub pass: bool,
}

pub(super) fn state(q: MemoryHammerProbe) -> serde_json::Value {
    json!({"core_position_m":q.core_position_m,"tip_position_m":q.tip_position_m,
        "core_velocity_m_s":q.core_velocity_m_s,"tip_velocity_m_s":q.tip_velocity_m_s,
        "material_deformation_m":q.material.displacement_m,"viscous_deformation_m":q.material.viscous_deformation_m,
        "branch_extension_m":q.material.branch_extension_m,"material_energy_j":q.material.stored_energy_j,
        "surface_energy_j":q.surface_energy_j,"mechanical_energy_j":q.mechanical_energy_j,
        "heat_j":q.material.dissipated_energy_j,"external_work_j":q.external_work_j,
        "absolute_impulse_work_j":q.absolute_impulse_work_j,"surface_impulse_n_s":q.surface_impulse_n_s,
        "balance_residual_j":q.balance_residual_j})
}

pub(super) fn take(
    rate: u32,
    p: MemoryHammerProfile,
    speed: f64,
    substeps: usize,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * substeps as f64);
    let mut hammer = MemoryHammer::new(h, p, 0.0, speed)?;
    let frames = (f64::from(rate) * 0.016).ceil() as usize;
    let kick_frame = (f64::from(rate) * 0.004).ceil() as usize;
    let mass = p.core_mass_kg + p.tip_mass_kg;
    let kick = 2.5 * mass * speed;
    let mut states = Vec::with_capacity(frames);
    let mut forces = Vec::with_capacity(frames);
    let mut events = Vec::new();
    let mut balance = 0.0_f64;
    let mut momentum_error = 0.0_f64;
    let mut peak_force = 0.0_f64;
    let mut peak_deformation = 0.0_f64;
    let mut free_heat = 0.0;
    let mut contact_runs = 0;
    let mut reimpact = false;
    let mut pass = true;
    for frame in 0..frames {
        if frame == kick_frame {
            let before = hammer.probe();
            hammer.apply_core_impulse(kick)?;
            let after = hammer.probe();
            pass &= before.material == after.material;
            events.push(
                json!({"event":"core_impulse","time_seconds":frame as f64/f64::from(rate),
                "impulse_n_s":kick,"before":state(before),"after":state(after)}),
            );
        }
        let mut frame_force = 0.0;
        for sub in 0..substeps {
            let before = hammer.probe();
            let q = hammer.tick()?;
            let scale = q.initial_energy_j + q.absolute_impulse_work_j;
            let now = q.contact_force_n > 0.0;
            let was = before.contact_force_n > 0.0;
            if now != was {
                if now {
                    contact_runs += 1;
                    reimpact |= frame >= kick_frame;
                }
                events.push(
                    json!({"event":if now{"contact_step_started"}else{"first_force_free_step"},
                    "step_end_seconds":(frame * substeps + sub + 1) as f64*h,"state":state(q)}),
                );
            }
            if !now && !was {
                free_heat += q.material.last_step_heat_j;
            }
            let momentum = p.core_mass_kg * q.core_velocity_m_s
                + p.tip_mass_kg * q.tip_velocity_m_s
                + q.surface_impulse_n_s
                - q.applied_impulse_n_s
                - mass * speed;
            let error = momentum.abs() / (mass * speed + q.applied_impulse_n_s.abs());
            momentum_error = momentum_error.max(error);
            balance = balance.max(q.balance_residual_j.abs() / scale);
            peak_force = peak_force.max(q.contact_force_n);
            peak_deformation = peak_deformation.max(q.material.displacement_m.abs());
            pass &= q.contact_force_n.is_finite()
                && q.contact_force_n >= 0.0
                && q.material.last_step_heat_j.is_finite()
                && q.material.last_step_heat_j >= 0.0
                && error.is_finite()
                && q.balance_residual_j.is_finite()
                && q.mechanical_energy_j <= before.mechanical_energy_j + scale * 1e-10;
            frame_force += q.contact_force_n / substeps as f64;
        }
        states.push(hammer.probe());
        forces.push(frame_force);
    }
    pass &= balance < 1e-8 && momentum_error < 1e-10 && free_heat > 0.0 && reimpact;
    Ok(Take {
        report: json!({"substeps":substeps,"maximum_relative_energy_residual":balance,
            "maximum_relative_momentum_residual":momentum_error,"peak_mean_contact_force_n":peak_force,
            "peak_absolute_material_deformation_m":peak_deformation,"heat_in_force_free_steps_j":free_heat,
            "contact_runs":contact_runs,"reimpact_after_impulse":reimpact,"events":events,
            "duration_seconds":frames as f64/f64::from(rate),"final_state":state(hammer.probe()),"passed":pass}),
        states,
        forces,
        pass,
    })
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3 | 5)
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let requested_substeps = if args.len() == 5 {
        if args[3] != "--substeps" {
            return Err(HELP.into());
        }
        let n: usize = args[4].parse()?;
        if !(1..=1302).contains(&n) {
            return Err("substeps must be within 1..1302".into());
        }
        Some(n)
    } else {
        None
    };
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for rate in [44100, 192000] {
        for tau in [0.0001, 0.001, 0.01] {
            for speed in [0.2, 0.8] {
                for tip_mass in [0.0001, 0.0005] {
                    let p = MemoryHammerProfile {
                        core_mass_kg: 0.004 - tip_mass,
                        tip_mass_kg: tip_mass,
                        material: HammerMemoryProfile {
                            relaxation_seconds: tau,
                            ..HammerMemoryProfile::default()
                        },
                        ..MemoryHammerProfile::default()
                    };
                    let substeps = requested_substeps
                        .unwrap_or_else(|| (1.0 / (f64::from(rate) * 5e-9)).ceil() as usize);
                    let candidate = take(rate, p, speed, substeps)?;
                    let reference = take(rate, p, speed, substeps * 4)?;
                    let mut velocity_error = 0.0;
                    let mut impulse_error = 0.0_f64;
                    for (a, b) in candidate.states.iter().zip(&reference.states) {
                        velocity_error += (p.core_mass_kg
                            * (a.core_velocity_m_s - b.core_velocity_m_s).powi(2)
                            + p.tip_mass_kg * (a.tip_velocity_m_s - b.tip_velocity_m_s).powi(2))
                            / 0.004;
                        impulse_error = impulse_error.max(
                            (a.surface_impulse_n_s - b.surface_impulse_n_s).abs() / (0.004 * speed),
                        );
                    }
                    velocity_error =
                        (velocity_error / candidate.states.len() as f64).sqrt() / speed;
                    let force_error = (candidate
                        .forces
                        .iter()
                        .zip(&reference.forces)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        / reference.forces.iter().map(|f| f * f).sum::<f64>())
                    .sqrt();
                    let passed = candidate.pass
                        && reference.pass
                        && velocity_error.is_finite()
                        && velocity_error < 0.01
                        && impulse_error.is_finite()
                        && impulse_error < 0.01
                        && force_error.is_finite()
                        && force_error < 0.02;
                    pass &= passed;
                    cases.push(json!({"sample_rate_hz":rate,"relaxation_seconds":tau,"launch_speed_m_s":speed,
                        "tip_mass_kg":tip_mass,"core_mass_kg":p.core_mass_kg,"passed":passed,
                        "mass_weighted_velocity_rmse_over_launch_speed":velocity_error,
                        "maximum_surface_impulse_error_over_initial_momentum":impulse_error,
                        "output_frame_mean_contact_force_relative_rmse":force_error,
                        "candidate":candidate.report,"reference":reference.report}));
                }
            }
        }
    }
    let report = json!({"schema_version":1,"experiment":"two-mass-memory-hammer-fixed-surface-v1",
        "status":if pass{"pass"}else{"fail"},"calibrated":false,"tine_integrated":false,"plugin_integrated":false,
        "requested_uniform_substeps":requested_substeps,
        "protocol":"Both masses start at the wall with the launch speed. After 4 ms rounded up to an output frame, a core impulse of 2.5 times initial momentum drives reimpact without resetting or repositioning. Duration 16 ms rounded up. Default candidate step <=5 ns; explicit substeps override it. Reference has four times as many substeps. Output observations at matching frame ends.",
        "scope":"Provisional two-mass hammer, bilateral memory material, stationary cubic elastic penalty surface. Independent material heat and surface momentum ledger. No action, exact impenetrability, parameter fit, audio or realtime qualification. Contact force is a step mean; reported event times mark discrete step ends, not exact contact roots.",
        "profile":{"total_mass_kg":0.004,"surface_stiffness_n_m2":1e12,"equilibrium_stiffness_n_m":0.0,
            "equilibrium_cubic_n_m2":4e10,"memory_stiffness_n_m":2e5},
        "gates":{"relative_energy_residual":1e-8,"relative_momentum_residual":1e-10,
            "velocity_rmse_over_launch_speed":0.01,"surface_impulse_error_over_initial_momentum":0.01,
            "output_mean_force_relative_rmse":0.02,"requires_free_recovery_heat_and_reimpact":true},"cases":cases});
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    if !pass {
        return Err("memory hammer audit failed; see report".into());
    }
    println!(
        "Memory hammer audit passed: 24 cases, 48 takes. Report: {}",
        args[2]
    );
    Ok(())
}
