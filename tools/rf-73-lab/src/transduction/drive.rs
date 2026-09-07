//! Terminal pedestal deceleration and a matched-duration linear control.
use super::{bridle, launch, repetition, tuning};
use rf_73_dsp::ElectromechanicalProfile;
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const FRACTION: f64 = 0.2;
const REPEAT: usize = 10080;
const NAMES: [&str; 3] = [
    "constant_slew",
    "terminal_ease_20pct",
    "matched_duration_linear",
];

fn duration(length: f64, speed: f64, shape: usize) -> f64 {
    length / speed * if shape == 0 { 1.0 } else { 1.0 + FRACTION }
}
// Position, right-hand velocity and acceleration after a key-down command.
// Only the terminal stop is smoothed; initial onset and key-up remain abrupt.
fn stroke(t: f64, length: f64, speed: f64, shape: usize) -> [f64; 3] {
    if t < 0.0 {
        return [0.0; 3];
    }
    if t >= duration(length, speed, shape) {
        return [length, 0.0, 0.0];
    }
    if shape == 0 {
        return [speed * t, speed, 0.0];
    }
    if shape == 2 {
        let v = speed / (1.0 + FRACTION);
        return [v * t, v, 0.0];
    }
    let cruise = (1.0 - FRACTION) * length / speed;
    if t <= cruise {
        return [speed * t, speed, 0.0];
    }
    let ramp = 2.0 * FRACTION * length / speed;
    let s = (t - cruise) / ramp;
    [
        (1.0 - FRACTION) * length + speed * ramp * (s - s.powi(3) + 0.5 * s.powi(4)),
        speed * (1.0 - 3.0 * s * s + 2.0 * s.powi(3)),
        speed / ramp * (-6.0 * s + 6.0 * s * s),
    ]
}
fn onset(p: ElectromechanicalProfile, t: f64) -> Option<(usize, f64)> {
    if repetition::target(p, t, REPEAT) == p.action.hammer_rest_m {
        None
    } else if t < 0.15 {
        Some((0, 0.03))
    } else {
        Some((1, REPEAT as f64 / 48000.0))
    }
}
fn target(p: ElectromechanicalProfile, t: f64, h: f64, speed: f64, shape: usize) -> f64 {
    let Some((_, start)) = onset(p, t) else {
        return p.action.hammer_rest_m;
    };
    let length = -p.action.escapement_m - p.action.hammer_rest_m;
    let elapsed = t - start + h;
    if shape == 0 || (shape == 1 && elapsed <= (1.0 - FRACTION) * length / speed) {
        // Preserve the original accumulated-slew prefix exactly.
        return -p.action.escapement_m;
    }
    (p.action.hammer_rest_m + stroke(elapsed, length, speed, shape)[0])
        .clamp(p.action.hammer_rest_m, -p.action.escapement_m)
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    shape: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let h = 1.0 / (48000.0 * steps as f64);
    let length = -p.action.escapement_m - p.action.hammer_rest_m;
    let mut tracking = 0.0_f64;
    let mut maximum = 0.0_f64;
    let mut minimum = 0.0_f64;
    let mut arrivals = [None; 2];
    let mut result = launch::take_driven(
        p,
        steps,
        speed,
        REPEAT,
        |t| target(p, t, h, speed, shape),
        |_, a, b, t, h| {
            if let Some((i, start)) = onset(p, t) {
                let expected = (p.action.hammer_rest_m
                    + stroke(t - start + h, length, speed, shape)[0])
                    .clamp(p.action.hammer_rest_m, -p.action.escapement_m);
                tracking = tracking.max((b.mechanical.pedestal_position_m - expected).abs());
                let velocity =
                    (b.mechanical.pedestal_position_m - a.mechanical.pedestal_position_m) / h;
                maximum = maximum.max(velocity);
                minimum = minimum.min(velocity);
                if arrivals[i].is_none()
                    && b.mechanical.pedestal_position_m == -p.action.escapement_m
                {
                    arrivals[i] = Some(t + h - start);
                }
            }
        },
    )?;
    let expected = duration(length, speed, shape);
    let passed = tracking < 1e-8
        && maximum <= speed * (1.0 + 1e-8)
        && minimum >= -speed * 1e-8
        && arrivals
            .iter()
            .all(|a| a.is_some_and(|t| (t - expected).abs() < 1e-5));
    result.report["driver"] = json!({"passed":passed,"shape":NAMES[shape],"travel_m":length,"nominal_speed_m_s":speed,
        "nominal_arrival_seconds_after_key_down":expected,"measured_arrivals_seconds_after_key_down":arrivals,
        "max_tracking_error_m":tracking,"max_key_down_speed_m_s":maximum,"min_key_down_speed_m_s":minimum,
        "terminal_deceleration_duration_seconds":if shape==1 {2.0*FRACTION*length/speed}else{0.0},
        "terminal_peak_deceleration_m_s2":if shape==1 {Some(0.75*speed*speed/(FRACTION*length))}else{None}});
    result.report["passed"] = json!(result.report["passed"] == true && passed);
    Ok(result)
}
fn compare_first(a: &Value, b: &Value) -> Value {
    let mut candidate = a["repetition"]["phases"][0].clone();
    candidate["start_seconds"] = json!(0.03);
    repetition::repeated_attack(&b["repetition"]["phases"][0], &candidate)
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-drive-release --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("drive release study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut profiles = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (profile_index, name) in [(0, "baseline"), (2, "pedestal_rate_loss_10")] {
            let p = repetition::profile(position, profile_index);
            let mut drivers: Vec<Value> = Vec::new();
            for (shape, driver) in NAMES.iter().enumerate() {
                let mut rows = Vec::new();
                for (index, speed) in [1.125, 1.5].iter().enumerate() {
                    println!("Loaded drive release: {name}, {driver}, {speed} m/s, 128/256 ticks");
                    let a = take(p, 128, *speed, shape)?;
                    let b = take(p, 256, *speed, shape)?;
                    let c = repetition::convergence(&a, &b, REPEAT);
                    let l = launch::convergence(&a.report, &b.report);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true
                        && l["passed"] == true;
                    let first = compare_first(
                        &b.report,
                        if shape == 0 {
                            &b.report
                        } else {
                            &drivers[0]["rows"][index]["takes"][1]
                        },
                    );
                    println!(
                        "Loaded drive release: qualified={qualified}, repeatable={}",
                        b.report["repetition"]["two_clean_repeatable_strikes"]
                    );
                    rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_original":first,
                        "takes":[a.report,b.report],"convergence":c,"launch_convergence":l}));
                }
                drivers.push(json!({"name":driver,"rows":rows}));
            }
            profiles.push(
                json!({"name":name,"settings":{"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m},"drivers":drivers}),
            );
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && profiles.len() == 2
        && profiles.iter().all(|p| {
            p["drivers"].as_array().is_some_and(|d| {
                d.len() == 3
                    && d.iter().all(|d| {
                        d["rows"].as_array().is_some_and(|r| {
                            r.len() == 2 && r.iter().all(|r| r["measurement_qualified"] == true)
                        })
                    })
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-drive-release-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"profiles":profiles,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix: original and pedestal-rate-loss-10 profiles; nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. Compare original constant slew, terminal ease over final 20% of upward pedestal travel, and constant-speed control with the same extended arrival time. Eased cruise retains nominal speed through 80% of travel; ramp duration 0.4*travel/speed, velocity v*(1-3*s^2+2*s^3), continuous velocity and acceleration at ramp boundaries. Total eased duration 1.2*travel/speed. Matched-duration linear uses speed/1.2. Initial onset and all key-up trajectories retain abrupt constant slew. Analytical driver tracking error <1e-8 m, nonnegative key-down speed bounded by nominal speed within 1e-8 relative tolerance, arrival within 10 us of prediction. Reuse all repetition and launch work/momentum/impact/event refinement gates. Original takes replay the retained launch study; easing preserves its accumulated-slew prefix. First-strike comparison uses previous 5% impact/speed and 1 ms latency limits against the same-profile original. Repeatability and lift retain prior within-take criteria. No candidate selection.",
        "scope":"Prescribed-motion intervention, not a measured key action or finite-inertia actuator. End-travel smoothing changes work and arrival time. Matched-duration linear control also changes initial velocity; it does not isolate every kinematic factor. Initial acceleration and key-up are not smoothed. No new contact law, geometry, gain, source fit, audio render or production default. Only the 60 ms repetition wait is covered."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("drive release study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_curve_matches_travel_speed_area_and_smooth_stop() {
        let length = 0.0105;
        let speed = 1.125;
        let start = 0.8 * length / speed;
        let end = duration(length, speed, 1);
        assert_eq!(stroke(start, length, speed, 1), [0.8 * length, speed, 0.0]);
        assert_eq!(stroke(end, length, speed, 1), [length, 0.0, 0.0]);
        let near = stroke(end - 1e-9, length, speed, 1);
        assert!((near[0] - length).abs() < 1e-14 && near[1].abs() < 1e-10 && near[2].abs() < 0.001);
        let mut integral = 0.0;
        for i in 0..10000 {
            let h = end / 10000.0;
            let t = (i as f64 + 0.5) * h;
            let s = stroke(t, length, speed, 1);
            assert!((0.0..=speed).contains(&s[1]));
            integral += s[1] * h;
            if t > start + 1e-6 && t < end - 1e-6 {
                let eps = 1e-7;
                let lo = stroke(t - eps, length, speed, 1);
                let hi = stroke(t + eps, length, speed, 1);
                assert!(((hi[0] - lo[0]) / (2.0 * eps) - s[1]).abs() < 1e-7);
                assert!(((hi[1] - lo[1]) / (2.0 * eps) - s[2]).abs() < 1e-4);
            }
        }
        assert!((integral - length).abs() < 1e-10);
        assert_eq!(duration(length, speed, 1), duration(length, speed, 2));
        assert!(stroke(0.001, length, speed, 2)[1] < speed);
    }
    #[test]
    fn eased_driver_keeps_cruise_and_release_targets() {
        let p = repetition::profile(0.8, 0);
        let h = 1.0 / (48000.0 * 128.0);
        for t in [0.0, 0.03, 0.035, 0.15, 0.16, 0.21, 0.215, 0.34] {
            assert_eq!(target(p, t, h, 1.125, 1), target(p, t, h, 1.125, 0));
        }
        assert!(target(p, 0.039, h, 1.125, 1) < -p.action.escapement_m);
    }
}
