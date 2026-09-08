//! Escapement release: the pedestal lets the hammer off at a regulated point
//! before key bottom, with the key continuing into aftertouch.
use super::key::{self, LetOff, REPEAT, SPEED_BOUND, Sim, WINDOW};
use super::{bridle, launch, repetition, tuning};
use rf_73_dsp::ElectromechanicalProfile;
use serde_json::{Value, json};
use std::{cell::RefCell, error::Error, path::Path};

const AFTERTOUCH: f64 = 0.001;
const EARLY: f64 = 0.001;
const BAND: f64 = 0.0006;
const NAMES: [&str; 4] = [
    "constant_slew",
    "key_letoff_sharp",
    "key_letoff_rolled",
    "key_letoff_early",
];

fn letoff(p: ElectromechanicalProfile, index: usize) -> Option<LetOff> {
    let top = -p.action.escapement_m;
    match index {
        1 => Some(LetOff { top, band: 0.0 }),
        2 => Some(LetOff { top, band: BAND }),
        3 => Some(LetOff {
            top: top - EARLY,
            band: 0.0,
        }),
        _ => None,
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    index: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let h = 1.0 / (48000.0 * steps as f64);
    let Some(l) = letoff(p, index) else {
        let mut result = launch::take_driven(
            p,
            steps,
            speed,
            REPEAT,
            WINDOW,
            |t| repetition::target(p, t, REPEAT),
            |_, _, _, _, _| {},
        )?;
        result.report["driver"] = json!({"shape":NAMES[0],"nominal_speed_m_s":speed});
        result.report["key"] = Value::Null;
        return Ok(result);
    };
    // The finger force lets a free key reach the nominal speed at let-off.
    let travel = l.start() - p.action.hammer_rest_m;
    let k = key::key(1, speed, travel).unwrap();
    let bed = -p.action.escapement_m + AFTERTOUCH;
    let sim = RefCell::new(Sim::with_letoff(p, k, bed, Some(l)));
    let mut result = launch::take_driven(
        p,
        steps,
        SPEED_BOUND,
        REPEAT,
        WINDOW,
        |t| sim.borrow_mut().step(t, h),
        |_, _, b, _, _| sim.borrow_mut().observe(b),
    )?;
    let frames = REPEAT + 9120;
    let key_report = sim
        .borrow()
        .report(NAMES[index], speed, frames as f64 / 48000.0);
    result.report["driver"] = json!({"shape":NAMES[index],"nominal_speed_m_s":speed,"letoff_travel_m":travel,
        "letoff_start_m":l.start(),"letoff_top_m":l.top,"letoff_band_m":l.band,"key_bed_m":bed});
    result.report["passed"] =
        json!(result.report["passed"] == true && key_report["passed"] == true);
    result.report["key"] = key_report;
    Ok(result)
}
fn optional_error(a: &Value, b: &Value, relative: bool, floor: f64) -> Option<f64> {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => Some(if relative {
            (a - b).abs() / b.abs().max(floor)
        } else {
            (a - b).abs()
        }),
        (None, None) => Some(0.0),
        _ => None,
    }
}
pub(super) fn letoff_convergence(a: &Value, b: &Value) -> Value {
    if a["key"].is_null() || b["key"].is_null() {
        return json!({"passed":a["key"].is_null() && b["key"].is_null(),"prescribed":true});
    }
    let mut rows = Vec::new();
    for (i, (ga, gb)) in a["key"]["gestures"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["key"]["gestures"].as_array().unwrap())
        .enumerate()
    {
        let (la, lb) = (&ga["letoff"], &gb["letoff"]);
        let begin = optional_error(
            &la["begin_seconds_after_key_down"],
            &lb["begin_seconds_after_key_down"],
            false,
            0.0,
        );
        let complete = optional_error(
            &la["complete_seconds_after_key_down"],
            &lb["complete_seconds_after_key_down"],
            false,
            0.0,
        );
        let key_speed = optional_error(
            &la["key_speed_at_begin_m_s"],
            &lb["key_speed_at_begin_m_s"],
            true,
            0.01,
        );
        let hammer = optional_error(
            &la["hammer_at_begin"]["hammer_velocity_m_s"],
            &lb["hammer_at_begin"]["hammer_velocity_m_s"],
            true,
            0.01,
        );
        let total = gb["window"]["terms_j"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap().abs())
            .sum::<f64>();
        let after = optional_error(
            &la["finger_work_after_begin_j"],
            &lb["finger_work_after_begin_j"],
            true,
            1e-3 * total,
        );
        let passed = begin.is_some_and(|e| e < 0.0001)
            && complete.is_some_and(|e| e < 0.0001)
            && key_speed.is_some_and(|e| e < 0.01)
            && hammer.is_some_and(|e| e < 0.01)
            && after.is_some_and(|e| e < 0.01);
        rows.push(json!({"gesture":i+1,"passed":passed,"begin_time_error_seconds":begin,"complete_time_error_seconds":complete,
            "relative_key_speed_error":key_speed,"relative_hammer_velocity_error":hammer,"relative_aftertouch_work_error":after}));
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"prescribed":false,"gestures":rows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-letoff --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("let-off study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut profiles = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (profile_index, name) in [(0, "baseline"), (2, "pedestal_rate_loss_10")] {
            let p = repetition::profile(position, profile_index);
            let mut drivers: Vec<Value> = Vec::new();
            for (index, driver) in NAMES.iter().enumerate() {
                let mut rows = Vec::new();
                for (row, speed) in [1.125, 1.5].iter().enumerate() {
                    println!("Loaded let-off: {name}, {driver}, {speed} m/s, 128/256 ticks");
                    let a = take(p, 128, *speed, index)?;
                    let b = take(p, 256, *speed, index)?;
                    let c = repetition::convergence(&a, &b, REPEAT);
                    let l = launch::convergence(&a.report, &b.report);
                    let k = key::key_convergence(&a.report, &b.report);
                    let e = letoff_convergence(&a.report, &b.report);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true
                        && l["passed"] == true
                        && k["passed"] == true
                        && e["passed"] == true;
                    let first = key::compare_first(
                        &b.report,
                        if index == 0 {
                            &b.report
                        } else {
                            &drivers[0]["rows"][row]["takes"][1]
                        },
                    );
                    let sharp = (index >= 2).then(|| {
                        key::compare_first(&b.report, &drivers[1]["rows"][row]["takes"][1])
                    });
                    println!(
                        "Loaded let-off: qualified={qualified}, repeatable={}",
                        b.report["repetition"]["two_clean_repeatable_strikes"]
                    );
                    rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_original":first,
                        "first_vs_sharp_letoff":sharp,"takes":[a.report,b.report],"convergence":c,
                        "launch_convergence":l,"key_convergence":k,"letoff_convergence":e}));
                }
                drivers.push(json!({"name":driver,"rows":rows}));
            }
            profiles.push(
                json!({"name":name,"settings":{"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m,"hammer_mass_kg":p.assembly.hammer_mass_kg},"drivers":drivers}),
            );
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && profiles.len() == 2
        && profiles.iter().all(|p| {
            p["drivers"].as_array().is_some_and(|d| {
                d.len() == 4
                    && d.iter().all(|d| {
                        d["rows"].as_array().is_some_and(|r| {
                            r.len() == 2 && r.iter().all(|r| r["measurement_qualified"] == true)
                        })
                    })
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-letoff-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"profiles":profiles,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 32-take matrix: original and pedestal-rate-loss-10 profiles; nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. Drivers: retained constant slew; a 0.05 kg key with a sharp let-off at the retained escapement top; the same key with a rolled let-off whose effective pedestal height follows start + band*(u - u^3/3) over a 0.6 mm band of key travel ending at the same top with zero slope; and the same key with a sharp let-off 1 mm earlier. Every key continues 1 mm past the escapement top into aftertouch and stops on an inelastic bed; the rest stop is inelastic. The effective pedestal height is a monotone map of key position with slope in [0, 1]; the reaction on the key is the pedestal force times the average slope over the step, so the key-side pedestal work equals pedestal force times pedestal displacement exactly. A constant 1 N return force acts toward rest; the step finger force is 1 N plus m*v^2/(2*travel to let-off), so a free key reaches the nominal speed at let-off. The key integrates one tick ahead of the assembly with the previous tick's pedestal force. Key gates: assembly pedestal position matches the mapped key position within 1e-9 m; key energy identity within 1e-8 relative every tick; key speed below 2 m/s; key-side pedestal work within 0.1% of the assembly's in every window; both let-offs, arrivals and the first landing exist. Launch observation is 30 ms after each key-down. Refinement requires let-off begin/complete times and arrival/landing times within 0.1 ms, key and hammer speeds at let-off, arrival, peak and landing speeds within 1% (0.01 m/s floor), window terms and aftertouch work within 1% normalized by the term or 0.1% of the window magnitude, and held positions within 1 um. All repetition and launch gates are retained. First-strike comparison against the same-profile constant slew and against the sharp let-off uses the prior 5% impact/speed and 1 ms latency limits. No candidate selection.",
        "scope":"Lumped key and kinematic let-off map, not a measured cam, pedestal pad or hammer geometry. The map is a reduction of the cam's velocity ratio; the hammer support after let-off stays at the top height while the key is depressed. Nominal speeds are free-key labels at let-off. The early let-off lengthens the hammer flight by 1 mm. Only the 60 ms repetition wait is covered. No new contact law inside the assembly, geometry, gain, source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("let-off study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn letoff_maps_are_monotone_continuous_and_reach_the_top_with_bounded_slope() {
        for band in [0.0, BAND] {
            let l = LetOff { top: -0.0015, band };
            assert!((l.map(l.complete()) - l.top).abs() < 1e-18);
            assert_eq!(l.map(l.top + 0.002), l.top);
            assert_eq!(l.map(l.start() - 0.001), l.start() - 0.001);
            let mut previous = l.map(-0.012);
            for i in 1..=4000 {
                let x = -0.012 + 0.0115 * i as f64 / 4000.0;
                let y = l.map(x);
                assert!(y >= previous - 1e-18 && y <= l.top + 1e-18);
                let slope = (y - previous) / (0.0115 / 4000.0);
                assert!((-1e-9..=1.0 + 1e-9).contains(&slope));
                previous = y;
            }
            let x0 = l.start() - 1e-4;
            let x1 = l.complete() + 1e-4;
            let r = l.ratio(x0, x1);
            assert!((r * (x1 - x0) - (l.map(x1) - l.map(x0))).abs() < 1e-18);
            assert_eq!(l.ratio(x0, x0), 1.0);
            assert_eq!(l.ratio(x1, x1), 0.0);
        }
        let rolled = LetOff {
            top: -0.0015,
            band: BAND,
        };
        let start = rolled.start();
        let eps = 1e-7;
        // Unit slope entering the band, zero slope leaving it, continuous position.
        assert!(
            ((rolled.map(start + eps) - rolled.map(start - eps)) / (2.0 * eps) - 1.0).abs() < 1e-6
        );
        let end = rolled.complete();
        // The finite difference straddling the band end is second order in eps.
        assert!(((rolled.map(end + eps) - rolled.map(end - eps)) / (2.0 * eps)).abs() < 1e-3);
        assert_eq!(rolled.slope(end), 0.0);
        assert!((rolled.slope(0.5 * (start + end)) - 0.75).abs() < 1e-12);
    }
    #[test]
    fn free_key_lets_off_at_nominal_speed_and_pedestal_work_follows_the_map() {
        let p = repetition::profile(0.8, 0);
        for (index, name) in NAMES.iter().enumerate().skip(1) {
            let l = letoff(p, index).unwrap();
            let travel = l.start() - p.action.hammer_rest_m;
            let k = key::key(1, 1.125, travel).unwrap();
            let bed = -p.action.escapement_m + AFTERTOUCH;
            let mut sim = Sim::with_letoff(p, k, bed, Some(l));
            let h = 1e-6;
            let mut top = f64::NEG_INFINITY;
            for i in 0..200_000 {
                let x = sim.step(i as f64 * h, h);
                top = top.max(x);
            }
            assert!((top - l.top).abs() < 1e-15);
            let report = sim.report(name, 1.125, 0.2);
            assert_eq!(report["passed"], false); // No assembly probe: no hammer state.
            let g = &report["gestures"][0];
            let begin = g["letoff"]["begin_seconds_after_key_down"]
                .as_f64()
                .unwrap();
            let speed = g["letoff"]["key_speed_at_begin_m_s"].as_f64().unwrap();
            assert!((speed - 1.125).abs() < 2e-3);
            assert!((begin - 2.0 * travel / 1.125).abs() < 1e-4);
            let arrival = g["arrival_seconds_after_key_down"].as_f64().unwrap();
            assert!(arrival > begin && g["arrival_speed_m_s"].as_f64().unwrap() > speed);
            assert_eq!(g["window"]["end_position_m"], bed);
            assert!(g["letoff"]["finger_work_after_begin_j"].as_f64().unwrap() > 0.0);
            assert!(report["max_relative_energy_defect"].as_f64().unwrap() < 1e-8);
        }
        // A constant pedestal force does work only through the mapped displacement.
        let l = letoff(p, 2).unwrap();
        let travel = l.start() - p.action.hammer_rest_m;
        let k = key::key(1, 1.5, travel).unwrap();
        let mut sim = Sim::with_letoff(p, k, -p.action.escapement_m + AFTERTOUCH, Some(l));
        let h = 1e-6;
        for i in 0..150_000 {
            sim.pedestal_force = 0.7;
            sim.step(i as f64 * h, h);
        }
        let report = sim.report(NAMES[2], 1.5, 0.15);
        let pedestal = -report["gestures"][0]["window"]["terms_j"][2]
            .as_f64()
            .unwrap();
        assert!((pedestal - 0.7 * (l.top - p.action.hammer_rest_m)).abs() < 1e-12);
        assert!(report["max_relative_energy_defect"].as_f64().unwrap() < 1e-8);
    }
}
