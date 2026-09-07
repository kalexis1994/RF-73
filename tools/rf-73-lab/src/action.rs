//! Reproducible whole-action cycles; no hammer impulses or per-strike state reset.
use rf_73_dsp::{
    ActionAssembly, ActionProfile, FeltDamperProfile, ModalAssemblyProfile, TineGeometry,
};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

pub const HELP: &str = "Persistent action audit:
  action-cycle --output REPORT.json [--refined | --reference] [--fast-drive]
16 strike/release/restrike cases; joint contacts, reciprocal bridle and drive work.
";
const DURATION: f64 = 0.18;

const NAMES: [&str; 4] = ["repeat", "pedal_repeat", "partial_release", "slack_bridle"];
struct Take {
    samples: Vec<[f64; 3]>,
    summary: Value,
}

fn control(gesture: usize, t: f64) -> (f64, f64) {
    let key = if (0.01..0.045).contains(&t) || (0.095..0.13).contains(&t) {
        1.0
    } else if gesture == 2 && (0.045..0.095).contains(&t) {
        0.7
    } else {
        0.0
    };
    let pedal = f64::from(gesture == 1 && (0.005..0.15).contains(&t));
    (key, pedal)
}
fn take(
    length: f64,
    rate: u32,
    steps: usize,
    gesture: usize,
    speed: f64,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * steps as f64);
    let profile = ActionProfile {
        bridle_slack_m: if gesture == 3 { 0.02 } else { 0.002 },
        ..ActionProfile::default()
    };
    let assembly = ModalAssemblyProfile::default();
    let felt = FeltDamperProfile::default();
    let mut v = ActionAssembly::new(
        h,
        TineGeometry {
            length_m: length,
            ..TineGeometry::default()
        },
        assembly,
        felt,
        profile,
    )?;
    let stiffness = [
        assembly.contact_stiffness_n_m2,
        felt.felt_stiffness_n_m2,
        profile.pedestal_stiffness_n_m2,
        profile.bridle_stiffness_n_m2,
    ];
    let frames = (DURATION * f64::from(rate)).round() as usize;
    let mut samples = Vec::with_capacity(frames);
    let mut x = profile.hammer_rest_m;
    let mut r = profile.damper_closed_m;
    let (mut balance, mut work_defect, mut passive_growth, mut continuity) =
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let mut heat_monotone = true;
    let mut force_nonnegative = true;
    let mut peak = [0.0_f64; 4];
    let mut strike_peak = [0.0_f64; 2];
    let mut previous = v.probe();
    let key_positions = [
        profile.pedestal_position_m(0.0)?,
        profile.pedestal_position_m(0.7)?,
        profile.pedestal_position_m(1.0)?,
    ];
    let pedal_positions = [
        profile.pedal_position_m(0.0)?,
        profile.pedal_position_m(1.0)?,
    ];
    for frame in 0..frames {
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let (key, pedal) = control(gesture, t);
            x += (key_positions[if key == 1.0 {
                2
            } else if key == 0.7 {
                1
            } else {
                0
            }] - x)
                .clamp(-speed * h, speed * h);
            r += (pedal_positions[usize::from(pedal == 1.0)] - r).clamp(-speed * h, speed * h);
            let a = previous;
            v.advance(x, r)?;
            let b = v.probe();
            previous = b;
            let scale = (b.initial_energy_j + b.absolute_drive_work_j).max(1e-20);
            balance = balance.max(b.balance_residual_j.abs() / scale);
            for j in 0..4 {
                let du = stiffness[j]
                    * (b.compression_m[j].max(0.0).powi(3) - a.compression_m[j].max(0.0).powi(3))
                    / 3.0;
                let heat = b.contact_heat_j[j] - a.contact_heat_j[j];
                let work = b.contact_force_n[j] * (b.compression_m[j] - a.compression_m[j]);
                work_defect = work_defect.max((work - du - heat).abs() / scale);
                heat_monotone &= heat >= 0.0;
                force_nonnegative &= b.contact_force_n[j] >= 0.0;
                peak[j] = peak[j].max(b.contact_force_n[j]);
            }
            heat_monotone &= b.structural_heat_j >= a.structural_heat_j
                && b.hammer_return_heat_j >= a.hammer_return_heat_j
                && b.arm_heat_j >= a.arm_heat_j;
            if x == a.pedestal_position_m && r == a.pedal_position_m {
                passive_growth =
                    passive_growth.max((b.mechanical_energy_j - a.mechanical_energy_j) / scale);
            }
            for j in 0..11 {
                continuity = continuity.max(
                    (b.position[j] - a.position[j] - 0.5 * h * (a.velocity[j] + b.velocity[j]))
                        .abs(),
                );
            }
            let interval = usize::from(t >= 0.09);
            strike_peak[interval] = strike_peak[interval].max(b.contact_force_n[0]);
        }
        let b = v.probe();
        samples.push([b.pickup_velocity_m_s, b.position[9], b.position[10]]);
    }
    let b = v.probe();
    let behavior = strike_peak[0] > 0.0
        && (gesture == 2 || strike_peak[1] > 0.0)
        && (gesture != 3 || b.simultaneous_hammer_felt_steps > 0);
    let passed = balance < 1e-8
        && work_defect < 1e-9
        && passive_growth < 1e-10
        && continuity < 1e-14
        && heat_monotone
        && force_nonnegative
        && behavior;
    Ok(Take {
        samples,
        summary: json!({"steps_per_frame": steps, "passed": passed,
        "max_relative_balance_defect": balance, "max_relative_contact_work_defect": work_defect,
        "max_stationary_drive_relative_energy_growth": passive_growth,"max_kinematic_defect": continuity,
        "heat_monotone": heat_monotone,"force_nonnegative": force_nonnegative,"behavior_passed": behavior,
        "peak_contact_force_n": peak,"strike_window_peak_n": strike_peak,"contact_entries": b.contact_entries,
        "simultaneous_hammer_felt_steps": b.simultaneous_hammer_felt_steps,"maximum_solver_sweeps": b.maximum_solver_sweeps,
        "limited_unloading_steps": b.limited_unloading_steps,"contact_heat_j": b.contact_heat_j,
        "structural_heat_j":b.structural_heat_j,"hammer_return_heat_j":b.hammer_return_heat_j,"arm_heat_j":b.arm_heat_j,
        "pedestal_work_j":b.pedestal_work_j,"pedal_work_j":b.pedal_work_j,"absolute_drive_work_j":b.absolute_drive_work_j,
        "initial_energy_j":b.initial_energy_j,"final_mechanical_energy_j":b.mechanical_energy_j,
        "final_position":b.position,"final_velocity":b.velocity }),
    })
}
fn compare(a: &Take, b: &Take, rate: u32) -> Value {
    let windows: Vec<_> = [(0.0, 0.045), (0.045, 0.095), (0.095, 0.14), (0.14, DURATION)].into_iter().map(|(lo, hi)| {
        let start = (lo * f64::from(rate)).round() as usize;
        let end = (hi * f64::from(rate)).round() as usize;
        let mut error = [0.0; 3];
        let mut signal = 0.0_f64;
        for (x,y) in a.samples[start..end].iter().zip(&b.samples[start..end]) {
            for j in 0..3 { error[j] += (x[j] - y[j]).powi(2); }
            signal += y[0] * y[0];
        }
        let velocity = (error[0] / signal.max(1e-30)).sqrt();
        let hammer = (error[1] / (end-start) as f64).sqrt();
        let arm = (error[2] / (end-start) as f64).sqrt();
        json!({"start_seconds":lo,"end_seconds":hi,"pickup_velocity_relative_rmse":velocity,
            "hammer_position_rmse_m":hammer,"arm_position_rmse_m":arm,"passed":velocity<0.01 && hammer<1e-5 && arm<1e-5})
    }).collect();
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3..=5)
        || args[1] != "--output"
        || args[3..]
            .iter()
            .any(|s| s != "--refined" && s != "--reference" && s != "--fast-drive")
        || (args.len() == 5 && args[3] == args[4])
        || (args.iter().any(|s| s == "--reference") && args.iter().any(|s| s == "--refined"))
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let speed = if args.iter().any(|s| s == "--fast-drive") {
        1.5
    } else {
        1.0
    };
    let steps = if args.iter().any(|s| s == "--reference") {
        [512, 1024, 2048]
    } else if args.iter().any(|s| s == "--refined") {
        [128, 256, 512]
    } else {
        [32, 64, 128]
    };
    let mut cases = Vec::new();
    for length in [0.075, 0.12] {
        for rate in [48000, 96000] {
            for (gesture, name) in NAMES.iter().enumerate() {
                let a = take(length, rate, steps[0], gesture, speed)?;
                let b = take(length, rate, steps[1], gesture, speed)?;
                let c = take(length, rate, steps[2], gesture, speed)?;
                let first = compare(&a, &c, rate);
                let second = compare(&b, &c, rate);
                let passed = [&a, &b, &c].iter().all(|t| t.summary["passed"] == true)
                    && first["passed"] == true
                    && second["passed"] == true;
                println!("Action cycle: {length} m, {rate} Hz, {name}, passed={passed}");
                cases.push(json!({"length_m":length,"sample_rate":rate,"gesture":name,"passed":passed,
            "takes":[a.summary,b.summary,c.summary],"coarse_vs_fine":first,"medium_vs_fine":second}));
            }
        }
    }
    let p = ActionProfile::default();
    let mut report = json!({"schema_version":1,"experiment":"persistent-action-cycle-v1","passed":cases.iter().all(|c|c["passed"]==true),
        "steps_per_frame":steps,"cases":cases,"contact_order":["hammer_tine","felt_tine","pedestal_hammer","bridle"],
        "profile":{"hammer_rest_m":p.hammer_rest_m,"escapement_m":p.escapement_m,
            "hammer_return_n_m":p.hammer_return_n_m,"hammer_return_n_s_m":p.hammer_return_n_s_m,
            "pedestal_stiffness_n_m2":p.pedestal_stiffness_n_m2,"pedestal_rate_loss_s_m":p.pedestal_rate_loss_s_m,
            "bridle_ratio":p.bridle_ratio,"bridle_slack_m":p.bridle_slack_m,"bridle_stiffness_n_m2":p.bridle_stiffness_n_m2,
            "bridle_rate_loss_s_m":p.bridle_rate_loss_s_m,"damper_closed_m":p.damper_closed_m,"pedal_travel_m":p.pedal_travel_m},
        "protocol":"Frozen before first matrix run. Two tine lengths, 48/96 kHz, four gestures, three uniform midpoint resolutions. 180 ms from rest including initial felt preload relaxation. Key down at 10 and 95 ms, release at 45 and 130 ms, both drives speed limited to 1 m/s. Pedal held 5-150 ms in pedal_repeat; partial_release retains 0.7 key lift between strikes; slack_bridle increases slack to 20 mm to stress simultaneous tine contacts. All other assembly/felt parameters use library defaults. No impulse, strike event, state reset or force normalization. Require relative energy defect <1e-8, per-contact work defect <1e-9, stationary-drive energy growth <1e-10, midpoint position defect <1e-14, monotone heat and nonnegative force. Require initial strike in all cases, repeat strike except shallow partial release, and simultaneous hammer/felt contact in slack-brindle stress cases. For both coarse/fine and medium/fine, pickup velocity relative RMSE <1% and each hammer/arm displacement RMSE <10 micrometers in four separate windows. Failures retain their report; no gate tuning.",
        "scope":"Offline whole-action reduction, not measured pivot geometry, calibrated hammer/strap/felt materials, production MIDI action or realtime plugin replacement. Bridle is reciprocal; pedestal and pedal spring base are prescribed mechanical work ports. No listening or host test claimed."});
    report["drive_speed_m_s"] = json!(speed);
    if speed != 1.0 || steps[0] != 32 {
        report["protocol"] = json!(report["protocol"].as_str().unwrap()
            .replace("Frozen before first matrix run.", "Follow-up frozen before execution. The original 1 m/s 32/64/128 matrix failed strike coverage and short-tine convergence. Keep material parameters and gates; this run uses the explicit drive_speed_m_s and steps_per_frame fields.")
            .replace("both drives speed limited to 1 m/s", &format!("both drives speed limited to {speed} m/s")));
    }
    if steps[0] == 512 && speed == 1.5 {
        report["follow_up"] = json!(
            "Resolution-only follow-up to the 1.5 m/s 128/256/512 matrix: all behaviors and energy gates passed but five case comparisons failed. Same physics, drives and gates; quadruple each subdivision. The overlapping 512-step summaries must match."
        );
    }
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("action cycle retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod regression {
    #[test]
    fn generic_action_replays_the_committed_planar_reference_exactly() {
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../references/persistent-action-cycle-reference-validation.json"
        ))
        .unwrap();
        let observed = super::take(0.075, 48000, 512, 0, 1.5).unwrap();
        // Apply the same JSON readback to both sides. Without float_roundtrip,
        // serde_json's reader can round a stored decimal by one ulp relative
        // to Value::from(f64); that is not a mechanical-state difference.
        let observed: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&observed.summary).unwrap()).unwrap();
        assert_eq!(observed, expected["cases"][0]["takes"][0]);
    }
}
