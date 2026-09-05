//! Deterministic mechanics audit: no audio device, reference samples or plugin changes.
use rf_rhodes_dsp::{AssemblyParameters, AssemblyVoice};
use serde::Serialize;
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Coupled assembly experiment:
  assembly-check --output REPORT.json
Audits 24 provisional note/rate/velocity cases at 4/16/64/128/256 substeps.
Each take lasts 50 ms, with the binary damper enabled at 25 ms.
Reports mechanical convergence against 256 steps and the complete energy budget.
No audio device is opened; this is not an instrument calibration or listening test.
";

#[derive(Serialize)]
struct Row {
    note: u8,
    sample_rate_hz: u32,
    velocity: f64,
    substeps: usize,
    masses_kg: [f64; 3],
    stiffnesses_n_m: [f64; 3],
    damping_n_s_m: [f64; 3],
    damper_n_s_m: f64,
    hammer_mass_kg: f64,
    contact_stiffness_n_m2: f64,
    maximum_hammer_speed_m_s: f64,
    maximum_balance_relative_residual: f64,
    maximum_positive_energy_step_relative: f64,
    initial_energy_j: f64,
    final_energy_j: f64,
    dissipated_energy_j: f64,
    escaped_hammer_energy_j: f64,
    separation_seconds: Option<f64>,
    contact_impulse_n_s: f64,
    peak_support_displacement_m: f64,
    peak_tonebar_displacement_m: f64,
    /// Combined absolute coordinates, mass-weighted; no time/gain alignment.
    displacement_relative_rmse_vs_256: f64,
    velocity_relative_rmse_vs_256: f64,
}

struct Take {
    row: Row,
    q: Vec<[f64; 3]>,
    v: Vec<[f64; 3]>,
}

fn take(note: u8, rate: u32, velocity: f64, substeps: usize) -> Result<Take, Box<dyn Error>> {
    let p = AssemblyParameters::provisional(note)?;
    let mut voice = AssemblyVoice::new(f64::from(rate), substeps, p)?;
    if !voice.strike(velocity) {
        return Err("assembly strike rejected".into());
    }
    let initial = voice.probe().injected_energy_j;
    let mut row = Row {
        note,
        sample_rate_hz: rate,
        velocity,
        substeps,
        masses_kg: p.masses_kg,
        stiffnesses_n_m: p.stiffnesses_n_m,
        damping_n_s_m: p.damping_n_s_m,
        damper_n_s_m: p.damper_n_s_m,
        hammer_mass_kg: p.hammer_mass_kg,
        contact_stiffness_n_m2: p.contact_stiffness_n_m2,
        maximum_hammer_speed_m_s: p.maximum_hammer_speed_m_s,
        maximum_balance_relative_residual: 0.0,
        maximum_positive_energy_step_relative: 0.0,
        initial_energy_j: initial,
        final_energy_j: 0.0,
        dissipated_energy_j: 0.0,
        escaped_hammer_energy_j: 0.0,
        separation_seconds: None,
        contact_impulse_n_s: 0.0,
        peak_support_displacement_m: 0.0,
        peak_tonebar_displacement_m: 0.0,
        displacement_relative_rmse_vs_256: 0.0,
        velocity_relative_rmse_vs_256: 0.0,
    };
    let frames = rate as usize / 20;
    let dt = 1.0 / (f64::from(rate) * substeps as f64);
    let mut q = Vec::with_capacity(frames);
    let mut v = Vec::with_capacity(frames);
    let mut previous = initial;
    for frame in 0..frames {
        if frame == frames / 2 {
            voice.set_damped(true);
        }
        for step in 0..substeps {
            voice.tick();
            let probe = voice.probe();
            if !probe.mechanical_energy_j.is_finite() || !probe.balance_residual_j.is_finite() {
                return Err("non-finite assembly energy".into());
            }
            row.maximum_balance_relative_residual = row
                .maximum_balance_relative_residual
                .max(probe.balance_residual_j.abs() / initial);
            row.maximum_positive_energy_step_relative = row
                .maximum_positive_energy_step_relative
                .max((probe.mechanical_energy_j - previous) / initial);
            row.contact_impulse_n_s += probe.contact_force_n * dt;
            row.peak_support_displacement_m = row
                .peak_support_displacement_m
                .max(probe.displacement_m[2].abs());
            row.peak_tonebar_displacement_m = row
                .peak_tonebar_displacement_m
                .max(probe.displacement_m[1].abs());
            if !probe.contact_active && row.separation_seconds.is_none() {
                row.separation_seconds = Some((frame * substeps + step + 1) as f64 * dt);
            }
            previous = probe.mechanical_energy_j;
        }
        let probe = voice.probe();
        q.push(probe.displacement_m);
        v.push(probe.velocity_m_s);
    }
    let probe = voice.probe();
    row.final_energy_j = probe.mechanical_energy_j;
    row.dissipated_energy_j = probe.dissipated_energy_j;
    row.escaped_hammer_energy_j = probe.escaped_hammer_energy_j;
    Ok(Take { row, q, v })
}

fn error(a: &[[f64; 3]], b: &[[f64; 3]], masses: [f64; 3]) -> f64 {
    let mut difference = 0.0;
    let mut reference = 0.0;
    for (a, b) in a.iter().zip(b) {
        for i in 0..3 {
            difference += masses[i] * (a[i] - b[i]).powi(2);
            reference += masses[i] * b[i] * b[i];
        }
    }
    (difference / reference).sqrt()
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    // Reserve the destination before expensive work; never overwrite an earlier audit.
    let mut output = crate::new_file(Path::new(&args[2]))?;
    let mut rows = Vec::new();
    for note in [28, 40, 55, 69, 88, 100] {
        for rate in [44100, 192000] {
            for velocity in [0.01, 1.0] {
                let reference = take(note, rate, velocity, 256)?;
                for substeps in [4, 16, 64, 128] {
                    let mut candidate = take(note, rate, velocity, substeps)?;
                    candidate.row.displacement_relative_rmse_vs_256 =
                        error(&candidate.q, &reference.q, candidate.row.masses_kg);
                    candidate.row.velocity_relative_rmse_vs_256 =
                        error(&candidate.v, &reference.v, candidate.row.masses_kg);
                    rows.push(candidate.row);
                }
                rows.push(reference.row);
            }
        }
    }
    let pass = rows.iter().all(|row| {
        row.maximum_balance_relative_residual < 1e-8
            && row.maximum_positive_energy_step_relative < 1e-10
            && row.separation_seconds.is_some()
            && row.displacement_relative_rmse_vs_256.is_finite()
            && row.velocity_relative_rmse_vs_256.is_finite()
            && (row.substeps != 128
                || (row.displacement_relative_rmse_vs_256 < 0.005
                    && row.velocity_relative_rmse_vs_256 < 0.005))
    });
    let report = serde_json::json!({
        "schema_version": 1, "model": "provisional-common-support-assembly-v1",
        "status": if pass { "pass" } else { "fail" },
        "calibrated": false, "plugin_integrated": false,
        "duration_seconds": 0.05, "damper_frame": "floor(output_frames / 2)",
        "reference_substeps": 256,
        "gates": {"energy_balance_relative": 1e-8, "positive_energy_step_relative": 1e-10, "rmse_128_vs_256": 0.005},
        "scope": "Mechanical probes only. Finite-reference convergence is not an absolute error bound. No pickup, audio filtering, tuning fit, or real-time qualification.",
        "cases": rows
    });
    serde_json::to_writer_pretty(&mut output, &report)?;
    writeln!(output)?;
    if !pass {
        return Err("assembly audit failed; see report".into());
    }
    println!(
        "Assembly audit passed: 24 cases, 120 takes. Report: {}",
        args[2]
    );
    Ok(())
}
