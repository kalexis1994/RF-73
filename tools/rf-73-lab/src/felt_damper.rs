//! Full moving-damper block: coupled release gestures and fixed refinement audit.
use rf_73_dsp::{
    DamperDrive, FeltDamperAssembly, FeltDamperProbe, FeltDamperProfile, ModalAssembly,
    ModalAssemblyProfile, ModalIntegration, TineGeometry,
};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

pub const HELP: &str = "Moving felt damper audit:
  felt-damper --output REPORT.json [--refined]
24 coupled gestures, three fixed resolutions, independent work and state checks.
";
const DURATION: f64 = 0.16;
const GESTURES: [&str; 6] = [
    "held",
    "fast_release",
    "slow_release",
    "partial_lift",
    "pedal_catch",
    "reopen_reclose",
];

fn control(gesture: usize, t: f64) -> (f64, f64, f64) {
    let key = if t < 0.015 || gesture == 0 || (gesture == 5 && (0.055..0.085).contains(&t)) {
        1.0
    } else {
        0.0
    };
    let pedal = if gesture == 3 && t >= 0.015 {
        0.05
    } else if gesture == 4 && (0.035..0.065).contains(&t) {
        1.0
    } else {
        0.0
    };
    (key, pedal, if gesture == 2 { 0.05 } else { 0.5 })
}
struct Take {
    samples: Vec<FeltDamperProbe>,
    summary: Value,
}
fn take(
    initial: &ModalAssembly,
    rate: u32,
    substeps: usize,
    gesture: usize,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * substeps as f64);
    let mut drive = DamperDrive::new(0.0002, 0.0022, 1.0)?;
    let mut voice = FeltDamperAssembly::from_released(
        h,
        initial,
        FeltDamperProfile::default(),
        drive.position_m(),
    )?;
    let frames = (DURATION * f64::from(rate)).round() as usize;
    let mut samples = Vec::with_capacity(frames);
    let mut max_balance = 0.0_f64;
    let mut min_force = f64::INFINITY;
    let mut max_force = 0.0_f64;
    let mut contact_work_defect = 0.0_f64;
    let mut passive_growth = 0.0_f64;
    let mut first_contact = None;
    let mut heat_monotone = true;
    let initial_energy = voice.probe().initial_energy_j;
    for frame in 0..frames {
        if gesture == 5 && frame == (0.065 * f64::from(rate)).round() as usize {
            voice.apply_hammer_port_impulse(1e-5)?;
        }
        for step in 0..substeps {
            let tick = frame * substeps + step;
            let t = tick as f64 * h;
            let (key, pedal, speed) = control(gesture, t);
            let before = voice.probe();
            let r = drive.advance(key, pedal, speed, h)?;
            voice.advance(r)?;
            let p = voice.probe();
            let scale =
                (initial_energy + p.actuator_absolute_work_j + p.injected_impulse_energy_j.abs())
                    .max(1e-20);
            max_balance = max_balance.max(p.balance_residual_j.abs() / scale);
            min_force = min_force.min(p.contact_force_n);
            max_force = max_force.max(p.contact_force_n);
            if p.contact_force_n > 0.0 && first_contact.is_none() {
                first_contact = Some(t + h);
            }
            let delta = p.compression_m - before.compression_m;
            let du = FeltDamperProfile::default().felt_stiffness_n_m2
                * (p.compression_m.max(0.0).powi(3) - before.compression_m.max(0.0).powi(3))
                / 3.0;
            let defect = p.contact_force_n * delta - du - (p.felt_heat_j - before.felt_heat_j);
            contact_work_defect = contact_work_defect.max(defect.abs() / scale);
            heat_monotone &= p.felt_heat_j >= before.felt_heat_j
                && p.arm_heat_j >= before.arm_heat_j
                && p.structural_heat_j >= before.structural_heat_j;
            if r == before.drive_position_m {
                passive_growth = passive_growth
                    .max((p.mechanical_energy_j - before.mechanical_energy_j) / scale);
            }
        }
        samples.push(voice.probe());
    }
    let p = voice.probe();
    let passed = max_balance < 1e-8
        && contact_work_defect < 1e-9
        && passive_growth < 1e-10
        && heat_monotone
        && min_force >= 0.0
        && if gesture == 0 {
            p.contact_entries == 0 && p.felt_heat_j == 0.0
        } else {
            p.contact_entries > 0 && p.felt_heat_j > 0.0
        };
    let summary = json!({"substeps":substeps,"passed":passed,"max_relative_balance_defect":max_balance,
        "max_relative_felt_work_defect":contact_work_defect,"max_stationary_drive_relative_energy_growth":passive_growth,
        "heat_monotone":heat_monotone,"minimum_force_n":min_force,"maximum_force_n":max_force,"first_contact_seconds":first_contact,
        "contact_entries":p.contact_entries,"limited_unloading_steps":p.limited_unloading_steps,
        "initial_energy_j":initial_energy,"final_structural_energy_j":p.structural_energy_j,"final_mechanical_energy_j":p.mechanical_energy_j,
        "felt_heat_j":p.felt_heat_j,"arm_heat_j":p.arm_heat_j,"structural_heat_j":p.structural_heat_j,
        "actuator_work_j":p.actuator_work_j,"actuator_absolute_work_j":p.actuator_absolute_work_j,"injected_impulse_energy_j":p.injected_impulse_energy_j,
        "final_drive_position_m":p.drive_position_m,"final_arm_position_m":p.arm_position_m});
    Ok(Take { samples, summary })
}
fn compare(a: &Take, b: &Take) -> Value {
    let mut windows = Vec::new();
    for (lo, hi) in [(0.0, 0.03), (0.03, 0.09), (0.09, DURATION)] {
        let start = (lo / DURATION * a.samples.len() as f64).round() as usize;
        let end = (hi / DURATION * a.samples.len() as f64).round() as usize;
        let (mut error, mut signal, mut arm_error, mut arm_signal) = (0.0, 0.0, 0.0, 0.0);
        for (x, y) in a.samples[start..end].iter().zip(&b.samples[start..end]) {
            error += (x.pickup_velocity_m_s - y.pickup_velocity_m_s).powi(2);
            signal += y.pickup_velocity_m_s.powi(2);
            arm_error += (x.arm_position_m - y.arm_position_m).powi(2);
            arm_signal += y.arm_position_m.powi(2);
        }
        let velocity = (error / signal.max(1e-30)).sqrt();
        let arm = (arm_error / arm_signal.max(1e-30)).sqrt();
        windows.push(json!({"start_seconds":lo,"end_seconds":hi,"pickup_velocity_relative_rmse":velocity,"arm_position_relative_rmse":arm,
            "passed":velocity<0.01 && arm<0.01}));
    }
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3 | 4)
        || args[1] != "--output"
        || (args.len() == 4 && args[3] != "--refined")
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let refined = args.len() == 4;
    let steps = if refined { [32, 64, 128] } else { [16, 32, 64] };
    let mut cases = Vec::new();
    for length in [0.075, 0.12] {
        for rate in [48000, 96000] {
            let g = TineGeometry {
                length_m: length,
                ..TineGeometry::default()
            };
            let mut initial = ModalAssembly::new(
                f64::from(rate),
                g,
                ModalAssemblyProfile::default(),
                ModalIntegration::Refined {
                    contact_substeps: 64,
                },
            )?;
            initial.strike(0.6);
            for _ in 0..(0.02 * f64::from(rate) * 4.0).round() as usize {
                initial.tick();
            }
            let mut reference_held = None;
            for (gesture, name) in GESTURES.iter().enumerate() {
                let coarse = take(&initial, rate, steps[0], gesture)?;
                let medium = take(&initial, rate, steps[1], gesture)?;
                let fine = take(&initial, rate, steps[2], gesture)?;
                let first = compare(&coarse, &fine);
                let second = compare(&medium, &fine);
                let end_energy = fine.summary["final_structural_energy_j"]
                    .as_f64()
                    .ok_or("missing energy")?;
                if gesture == 0 {
                    reference_held = Some(end_energy);
                }
                let ratio = end_energy / reference_held.ok_or("missing held reference")?;
                let passed = [&coarse, &medium, &fine]
                    .iter()
                    .all(|t| t.summary["passed"] == true)
                    && first["passed"] == true
                    && second["passed"] == true;
                cases.push(json!({"length_m":length,"sample_rate":rate,"gesture":name,"passed":passed,
                    "takes":[coarse.summary,medium.summary,fine.summary],"coarse_vs_fine":first,"medium_vs_fine":second,
                    "final_structural_energy_relative_to_held":ratio}));
                println!("Felt damper: {length} m, {rate} Hz, {name}, passed={passed}");
            }
        }
    }
    let p = FeltDamperProfile::default();
    let mut report = json!({"schema_version":1,"experiment":"moving-felt-damper-v1","passed":cases.iter().all(|c|c["passed"]==true),"cases":cases,
        "profile":{"arm_mass_kg":p.arm_mass_kg,"arm_stiffness_n_m":p.arm_stiffness_n_m,"arm_damping_n_s_m":p.arm_damping_n_s_m,
            "felt_stiffness_n_m2":p.felt_stiffness_n_m2,"felt_rate_loss_s_m":p.felt_rate_loss_s_m},
        "protocol":"Frozen before first run. Two tine lengths, 48/96 kHz, six gestures, 16/32/64 midpoint ticks per frame: 24 cases, 72 takes. Identical ringing-state handoff 20 ms after a 0.6 normalized elastic strike, no legacy damper. Simulate 160 ms of held, fast/slow release, partial lift, pedal catch and reopen/reclose with a diagnostic hammer-port impulse at 65 ms. Closed drive +0.2 mm, travel 2.2 mm; max(key,pedal) lift; 0.5 m/s drive, 0.05 m/s slow release. Continuous drive work, arm/structural/felt heat and unilateral contact tracked independently. Require balance defect <1e-8, felt work defect <1e-9, stationary-drive energy growth <1e-10, monotone heat, nonnegative force, zero held contact and nonzero released contact/heat. Pickup velocity and arm displacement relative RMSE <1% in each of three separate windows for both 16/64 and 32/64 comparisons. Final energy ratios to held are descriptive because the moving actuator supplies work and the reclose case has an extra impulse. No tuned gates or measured material claim.",
        "scope":"A new coupled physical damper reduction, not a calibrated felt/leaf-spring model, full bridle action, half-pedal MIDI integration or production plugin replacement. Selected post-strike gestures only; simultaneous live hammer/felt contact and 73-key realtime cost remain open. No audio/host test claimed."});
    report["steps_per_frame"] = json!(steps);
    if refined {
        report["experiment"] = json!("moving-felt-damper-refined-v1");
        report["protocol"] = json!(report["protocol"].as_str().unwrap().replace("16/32/64","32/64/128").replace("16/64","32/128").replace("32/64 comparisons","64/128 comparisons").replace("Frozen before first run.","Follow-up frozen before refined run. The original 16/32/64 matrix failed the 1% pickup criterion in one recontact case. Keep all physics, gestures and gates; double each subdivision count."));
    }
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("moving felt damper retained failed qualification".into());
    }
    Ok(())
}
