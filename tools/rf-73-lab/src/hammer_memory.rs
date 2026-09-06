use rf_73_dsp::{HammerMemory, HammerMemoryProbe, HammerMemoryProfile};
use serde_json::json;
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Material memory coupon:
  hammer-memory-check --output REPORT.json
Audits prescribed loading, relaxation, clamped rest and reloading at two rates.
Bilateral material only; no hammer contact, piano integration or audio device.
";
struct Take {
    report: serde_json::Value,
    forces: Vec<f64>,
    pass: bool,
}
fn probe(p: HammerMemoryProbe) -> serde_json::Value {
    json!({"displacement_m":p.displacement_m,"viscous_deformation_m":p.viscous_deformation_m,
        "branch_extension_m":p.branch_extension_m,"endpoint_force_n":p.force_n,"mean_force_n":p.mean_force_n,
        "stored_energy_j":p.stored_energy_j,"dissipated_energy_j":p.dissipated_energy_j,
        "external_work_j":p.external_work_j,"absolute_work_j":p.absolute_work_j,"balance_residual_j":p.balance_residual_j})
}
fn take(
    rate: u32,
    p: HammerMemoryProfile,
    amplitude: f64,
    rest: f64,
    substeps: usize,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * substeps as f64);
    let mut coupon = HammerMemory::new(h, p)?;
    let ramp_frames = (f64::from(rate) * 0.00025).ceil() as usize;
    let hold_frames = (f64::from(rate) * 0.002).ceil() as usize;
    let rest_frames = (f64::from(rate) * rest).ceil() as usize;
    let mut stages = Vec::new();
    let mut forces = Vec::new();
    let mut balance = 0.0_f64;
    let mut analytic_error = 0.0_f64;
    let mut frame_count = 0;
    let mut first_force = 0.0;
    let mut pass = true;
    for (index, (name, target, frames)) in [
        ("load", amplitude, ramp_frames),
        ("hold", amplitude, hold_frames),
        ("unload", 0.0, ramp_frames),
        ("clamped_rest", 0.0, rest_frames),
        ("reload", amplitude, ramp_frames),
    ]
    .into_iter()
    .enumerate()
    {
        let before = coupon.probe();
        for i in 1..=frames {
            for sub in 1..=substeps {
                let fraction = ((i - 1) * substeps + sub) as f64 / (frames * substeps) as f64;
                let x = before.displacement_m + (target - before.displacement_m) * fraction;
                let q = coupon.advance_to(x)?;
                let scale = q.absolute_work_j.max(1e-30);
                let residual = q.balance_residual_j.abs() / scale;
                pass &= residual.is_finite() && q.last_step_heat_j >= 0.0 && q.force_n.is_finite();
                balance = balance.max(residual);
            }
            forces.push(coupon.probe().force_n);
        }
        let after = coupon.probe();
        if index == 0 {
            first_force = after.force_n;
        }
        if index == 1 || index == 3 {
            let expected = before.branch_extension_m
                * (-(frames as f64) / (f64::from(rate) * p.relaxation_seconds)).exp();
            analytic_error = analytic_error.max(
                (after.branch_extension_m - expected).abs()
                    / before.branch_extension_m.abs().max(1e-30),
            );
            pass &= after.external_work_j == before.external_work_j
                && after.stored_energy_j <= before.stored_energy_j;
        }
        frame_count += frames;
        stages.push(json!({"stage":name,"output_frames":frames,"time_seconds":frame_count as f64/f64::from(rate),"state":probe(after)}));
    }
    pass &= balance < 1e-10 && analytic_error < 1e-10;
    Ok(Take {
        report: json!({"substeps":substeps,"maximum_relative_energy_residual":balance,
            "maximum_normalized_hold_extension_error":analytic_error,
            "reload_to_first_endpoint_force_ratio":coupon.probe().force_n/first_force,"stages":stages}),
        forces,
        pass,
    })
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
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
            for amplitude in [5e-6, 2e-5, 5e-5] {
                for rest in [0.0002, 0.02] {
                    let p = HammerMemoryProfile {
                        relaxation_seconds: tau,
                        ..HammerMemoryProfile::default()
                    };
                    let candidate = take(rate, p, amplitude, rest, 1)?;
                    let reference = take(rate, p, amplitude, rest, 2)?;
                    let error = (candidate
                        .forces
                        .iter()
                        .zip(&reference.forces)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        / reference.forces.iter().map(|f| f * f).sum::<f64>())
                    .sqrt();
                    pass &= candidate.pass && reference.pass && error.is_finite() && error < 1e-10;
                    cases.push(json!({"sample_rate_hz":rate,"relaxation_seconds":tau,"amplitude_m":amplitude,
                        "requested_rest_seconds":rest,"endpoint_force_relative_rmse_vs_two_substeps":error,
                        "candidate":candidate.report,"reference":reference.report}));
                }
            }
        }
    }
    let p = HammerMemoryProfile::default();
    let report = json!({"schema_version":1,"experiment":"hammer-memory-coupon-v1","status":if pass{"pass"}else{"fail"},
        "calibrated":false,"contact_integrated":false,"plugin_integrated":false,
        "profile":{"equilibrium_stiffness_n_m":p.equilibrium_stiffness_n_m,"equilibrium_cubic_n_m2":p.equilibrium_cubic_n_m2,
            "memory_stiffness_n_m":p.memory_stiffness_n_m,"relaxation_seconds":"per case","viscosity_n_s_m":"memory_stiffness_n_m * relaxation_seconds"},
        "protocol":"Linear 0.25 ms loading, 2 ms hold, 0.25 ms unloading, case-specific clamped rest at zero displacement, 0.25 ms reloading. Each duration rounds up to an output frame. Two substeps subdivide the identical path.",
        "scope":"Bilateral displacement-controlled material coupon, not nonadhesive hammer contact. Signed force/work are specimen reactions. Exact ramp moments and independent positive heat. No physical parameter fit, free recovery, audio or realtime qualification.",
        "gates":{"energy_relative_to_absolute_work":1e-10,"hold_extension_normalized":1e-10,"force_rmse_two_substeps":1e-10},"cases":cases});
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    if !pass {
        return Err("hammer memory coupon audit failed; see report".into());
    }
    println!(
        "Hammer memory coupon audit passed: 36 cases, 72 takes. Report: {}",
        args[2]
    );
    Ok(())
}
