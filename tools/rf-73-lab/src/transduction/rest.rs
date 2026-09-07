//! Compare cold preload relaxation with independently qualified static rest.
use rf_73_dsp::{ElectromechanicalAssembly, ElectromechanicalProfile, ProductionDecimator};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("stationary-rest --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("stationary-rest requires a new .json path".into());
    }
    let mut cases = Vec::new();
    for (length, rate) in [(0.07, 48000), (0.12, 96000)] {
        for angle in [0.0, 0.4] {
            let mut p = ElectromechanicalProfile::default();
            p.geometry.length_m = length;
            p.polarization.felt_angle_rad = angle;
            let cold = idle(p, rate, false)?;
            let rest = idle(p, rate, true)?;
            let passed =
                cold["peak_filtered_voltage_v"].as_f64().unwrap() > 1e-4 && rest["passed"] == true;
            println!("Stationary rest: {length} m, {rate} Hz, felt angle {angle}, passed={passed}");
            cases.push(json!({"length_m":length,"sample_rate":rate,"felt_angle_rad":angle,"passed":passed,"cold":cold,"rest":rest}));
        }
    }
    let report = json!({"schema_version":1,"experiment":"stationary-rest-v1","passed":cases.iter().all(|c|c["passed"]==true),"cases":cases,
        "protocol":"Frozen before first run. Four profiles: 70 mm at 48 kHz and 120 mm at 96 kHz, each with felt angles 0 and 0.4 radians. Compare cold and static initializations with identical parameters, 128 ticks/frame, 100 ms of stationary pedestal/pedal and 10k loaded circuit. Average midpoint voltage to 4x and common 127-tap FIR. No warmup, mute, gain or state thresholding. Require cold filtered peak >1e-4 V as a positive control; at-rest raw and filtered peaks <1e-9 V, pickup displacement drift <1e-12 m, pickup speed <1e-10 m/s, relative total balance defect <1e-8, positive stored preload/felt force, zero drive work and zero new contact entries. Static qualification requires per-coordinate force defect <=1e-10 and constitutive defect <=1e-12. Reprepare at half the time step and require identical initial probe/diagnostics.",
        "scope":"Equilibrium of the existing convex contact/linear stiffness model, with a zero-current constant-L circuit. No static magnetic attraction, gravity, measured preload calibration or realtime integration. Historical cold constructors remain available."});
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("stationary rest retained failed qualification".into());
    }
    Ok(())
}
fn idle(p: ElectromechanicalProfile, rate: u32, at_rest: bool) -> Result<Value, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * 128.0);
    let mut model = if at_rest {
        ElectromechanicalAssembly::new_at_rest(h, p)?
    } else {
        ElectromechanicalAssembly::new(h, p)?
    };
    let start = model.probe();
    let preparation = model.rest_preparation();
    let independent = if at_rest {
        let half = ElectromechanicalAssembly::new_at_rest(h / 2.0, p)?;
        half.probe() == start && half.rest_preparation() == preparation
    } else {
        true
    };
    let mut old = start;
    let mut filter = ProductionDecimator::new();
    let mut peak = 0.0_f64;
    let mut raw = 0.0_f64;
    let mut drift = 0.0_f64;
    let mut speed = 0.0_f64;
    let mut balance = 0.0_f64;
    for _ in 0..rate / 10 {
        let mut average = 0.0;
        for sub in 0..128 {
            model.advance(p.action.hammer_rest_m, p.action.damper_closed_m)?;
            let b = model.probe();
            raw = raw.max(b.output_voltage_v.abs());
            for i in 0..2 {
                drift = drift.max(
                    (b.mechanical.pickup_displacement_xy_m[i]
                        - start.mechanical.pickup_displacement_xy_m[i])
                        .abs(),
                );
                speed = speed.max(b.mechanical.pickup_velocity_xy_m_s[i].abs());
            }
            balance = balance.max(
                b.total_balance_residual_j.abs() / start.mechanical.initial_energy_j.max(1e-20),
            );
            average += 0.5 * (old.output_voltage_v + b.output_voltage_v) / 32.0;
            old = b;
            if (sub + 1) % 32 == 0 {
                filter.push(average);
                average = 0.0;
            }
        }
        peak = peak.max(filter.output().abs());
    }
    let passed = at_rest
        && independent
        && raw < 1e-9
        && peak < 1e-9
        && drift < 1e-12
        && speed < 1e-10
        && balance < 1e-8
        && start.mechanical.initial_energy_j > 0.0
        && start.mechanical.contact_force_n[1] > 0.0
        && old.mechanical.absolute_drive_work_j == 0.0
        && old.mechanical.contact_entries == [0; 4];
    Ok(
        json!({"passed":passed,"time_step_independent":independent,"peak_raw_voltage_v":raw,"peak_filtered_voltage_v":peak,
        "maximum_pickup_drift_m":drift,"maximum_pickup_speed_m_s":speed,"max_relative_total_balance_defect":balance,
        "initial_energy_j":start.mechanical.initial_energy_j,"initial_contact_force_n":start.mechanical.contact_force_n,
        "absolute_drive_work_j":old.mechanical.absolute_drive_work_j,"contact_entries":old.mechanical.contact_entries,
        "preparation":preparation.map(|r|json!({"sweeps":r.sweeps,"max_relative_force_defect":r.max_relative_force_defect,"max_relative_contact_defect":r.max_relative_contact_defect}))}),
    )
}
