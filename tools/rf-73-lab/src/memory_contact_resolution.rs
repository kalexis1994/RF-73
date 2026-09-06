//! Read-only contact step-size sweeps along the existing event protocol.
use crate::memory_modal_check::free_controller::Controller;
use rf_73_dsp::{
    HammerMemoryProfile, MemoryContactStatus, MemoryContactStep, MemoryHammerProfile,
    MemoryModalAssembly, ModalAssemblyProfile, TineGeometry,
};
use serde_json::{Value, json};
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Memory contact resolution diagnostic:
  memory-contact-resolution --output REPORT.json
Twelve provisional profiles; read-only dyadic contact trials during an 8 ms
impact, impulse and damper protocol. Reports state-error terms and local scaling.
No tolerance changes, calibration, audio or realtime qualification.
";
const TERMS: [&str; 7] = [
    "structural_kinetic",
    "structural_elastic",
    "core_kinetic",
    "tip_kinetic",
    "memory_coordinates",
    "equilibrium_material",
    "surface_contact",
];
const SUBSTEPS: usize = 16672;
const H: f64 = 1.0 / (48000.0 * SUBSTEPS as f64);

fn snapshot(v: &MemoryModalAssembly, ticks: usize) -> Result<Value, Box<dyn Error>> {
    let before = v.probe();
    let mut rows = Vec::new();
    let mut previous_error: Option<f64> = None;
    let mut first_limited = None;
    for level in 1..=12 {
        let trial = v.inspect_contact_step(level)?;
        if v.probe() != before {
            return Err("contact inspection changed physical state".into());
        }
        let error = trial.result.normalized_state_error;
        let mut dominant = None;
        let mut fraction = None;
        if let Some(terms) = trial.error_metric_terms_j {
            if !terms.iter().all(|x| x.is_finite() && *x >= 0.0) {
                return Err("invalid contact error decomposition".into());
            }
            let sum = terms.iter().sum::<f64>();
            let scale = trial.energy_scale_j.ok_or("missing metric energy scale")?;
            let reconstructed = (sum / (2.0 * scale)).sqrt();
            if Some(reconstructed) != error {
                return Err("contact error terms do not reconstruct the estimator".into());
            }
            if sum > 0.0 {
                let index = (0..7)
                    .max_by(|a, b| terms[*a].total_cmp(&terms[*b]))
                    .unwrap();
                dominant = Some(TERMS[index]);
                fraction = Some(terms[index] / sum);
            }
        }
        // Raw adjacent-level slope, not an asserted asymptotic method order.
        // Missing/zero/tiny estimates do not support a useful ratio.
        let scaling = error
            .zip(previous_error)
            .filter(|(a, b)| *a > 1e-14 && *b > 1e-14)
            .map(|(a, b)| (a / b).log2());
        previous_error = error;
        if first_limited.is_none()
            && trial.result.status == MemoryContactStatus::AccuracyRequired
            && error.is_some_and(|e| e > MemoryContactStep::STATE_ERROR_LIMIT)
        {
            first_limited = Some(
                json!({"level":level,"step_seconds":H * (1u32 << level) as f64,
                "dominant_term":dominant,"dominant_squared_metric_fraction":fraction}),
            );
        }
        rows.push(
            json!({"level":level,"step_seconds":H * (1u32 << level) as f64,
            "status":match trial.result.status {
                MemoryContactStatus::Advanced => "accepted",
                MemoryContactStatus::BoundaryRequired => "boundary_required",
                MemoryContactStatus::AccuracyRequired => "accuracy_required",
            },
            "normalized_state_error":error,"error_metric_terms_j":trial.error_metric_terms_j,
            "energy_scale_j":trial.energy_scale_j,"dominant_term":dominant,
            "dominant_squared_metric_fraction":fraction,"adjacent_level_log2_error_ratio":scaling}),
        );
    }
    Ok(json!({"base_ticks":ticks,"time_seconds":ticks as f64 * H,
        "surface_energy_j":before.hammer.surface_energy_j,
        "core_velocity_m_s":before.hammer.core_velocity_m_s,
        "tip_velocity_m_s":before.hammer.tip_velocity_m_s,
        "material_deformation_m":before.hammer.material.displacement_m,
        "branch_extension_m":before.hammer.material.branch_extension_m,
        "first_state_limited_trial":first_limited,"trials":rows}))
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|x| x != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    for length in [0.05, 0.075, 0.12] {
        for speed in [0.2, 0.8] {
            for tau in [0.001, 0.01] {
                let mut voice = MemoryModalAssembly::new(
                    H,
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
                voice.prepare_free_steps(12)?;
                voice.prepare_contact_steps(12)?;
                let mut controller = Controller::with_economical_contact();
                let mut snapshots = Vec::new();
                let mut maximum_balance = 0.0_f64;
                for frame in 0..384 {
                    if frame == 96 {
                        voice.apply_core_impulse(2.5 * 0.004 * speed)?;
                    }
                    if frame == 192 {
                        voice.set_damped(true);
                    }
                    if frame == 288 {
                        voice.set_damped(false);
                    }
                    if matches!(frame, 0..=15 | 96..=111 | 192..=195 | 288..=291) {
                        snapshots.push(snapshot(&voice, frame * SUBSTEPS)?);
                    }
                    let early = matches!(frame, 0 | 96 | 192 | 288);
                    let mut offset = 0;
                    while offset < SUBSTEPS {
                        let end = if early && offset < 1600 {
                            1600
                        } else {
                            SUBSTEPS
                        };
                        offset += controller.advance(&mut voice, end - offset)?;
                        let q = voice.probe();
                        let scale = (q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j)
                            .max(1e-30);
                        let balance = q.balance_residual_j.abs() / scale;
                        if !balance.is_finite() || balance > 1e-8 {
                            return Err(
                                "contact diagnostic trajectory failed energy balance".into()
                            );
                        }
                        maximum_balance = maximum_balance.max(balance);
                        if early && offset == 1600 {
                            snapshots.push(snapshot(&voice, frame * SUBSTEPS + offset)?);
                        }
                    }
                }
                cases.push(json!({"tine_length_m":length,"launch_speed_m_s":speed,
                    "relaxation_seconds":tau,"maximum_relative_energy_residual":maximum_balance,
                    "snapshots":snapshots}));
            }
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,
        "experiment":"memory-contact-resolution-v1","status":"pass",
        "base_step_seconds":H,"state_error_limit":MemoryContactStep::STATE_ERROR_LIMIT,
        "error_metric_term_order":TERMS,"cases":cases,
        "protocol":"8 ms at 48 kHz observations; core impulse at 2 ms, damper on at 4 ms and off at 6 ms. Economical controller at 16672 base ticks per frame. Inspect frames 0..15, 96..111, 192..195, 288..291 after events, plus tick 1600 of each event frame. Every inspection sweeps levels 1..12 from the same unchanged state. Extra observation boundaries can change controller subdivision; no trial is committed.",
        "scope":"Squared coarse/fine error metric contributions, not physical energy shares or measured acoustic error. Full reciprocal structural mass terms are retained. Memory-coordinate terms include the estimator's core/tip position weights. Raw adjacent-level ratios are not an order guarantee. Missing metrics mean boundary or physical/finite checks prevented a usable state estimate. Provisional profiles, no calibration, independent time-reference comparison, pickup voltage, audio or realtime qualification."}),
    )?;
    file.write_all(b"\n")?;
    println!(
        "Contact resolution diagnostic passed: 12 cases. Report: {}",
        args[2]
    );
    Ok(())
}
