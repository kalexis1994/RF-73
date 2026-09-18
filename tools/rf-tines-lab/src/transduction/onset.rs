//! Onset acceleration and complete key-drive shapes with segment-wise pedestal work.
use super::{bridle, launch, repetition, threshold, tuning};
use rf_tines_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const FRACTION: f64 = 0.2;
const REPEAT: usize = 10080;
const HOLD: f64 = 0.12;
const NAMES: [&str; 4] = [
    "constant_slew",
    "onset_ease_20pct",
    "onset_linear_20pct",
    "full_ease_20pct",
];
const SEGMENTS: [&str; 4] = ["onset", "cruise", "stop", "hold"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Onset {
    Abrupt,
    Smooth,
    Linear,
}
#[derive(Clone, Copy)]
pub(super) struct Shape {
    onset: Onset,
    smooth_stop: bool,
}
fn shape(index: usize) -> Shape {
    match index {
        0 => Shape {
            onset: Onset::Abrupt,
            smooth_stop: false,
        },
        1 => Shape {
            onset: Onset::Smooth,
            smooth_stop: false,
        },
        2 => Shape {
            onset: Onset::Linear,
            smooth_stop: false,
        },
        _ => Shape {
            onset: Onset::Smooth,
            smooth_stop: true,
        },
    }
}
fn ramp(length: f64, speed: f64) -> f64 {
    2.0 * FRACTION * length / speed
}
// End of onset ramp, end of cruise and arrival, all measured after key-down.
fn boundaries(length: f64, speed: f64, shape: Shape) -> [f64; 3] {
    let onset = if shape.onset == Onset::Abrupt {
        0.0
    } else {
        ramp(length, speed)
    };
    let mut cruise = length;
    if shape.onset != Onset::Abrupt {
        cruise -= FRACTION * length;
    }
    if shape.smooth_stop {
        cruise -= FRACTION * length;
    }
    let cruise_end = onset + cruise / speed;
    let stop = if shape.smooth_stop {
        ramp(length, speed)
    } else {
        0.0
    };
    [onset, cruise_end, cruise_end + stop]
}
fn peak_onset_acceleration(length: f64, speed: f64, shape: Shape) -> Option<f64> {
    match shape.onset {
        Onset::Abrupt => None,
        Onset::Smooth => Some(0.75 * speed * speed / (FRACTION * length)),
        Onset::Linear => Some(0.5 * speed * speed / (FRACTION * length)),
    }
}
// Position, right-hand velocity and acceleration after a key-down command.
pub(super) fn stroke(t: f64, length: f64, speed: f64, shape: Shape) -> [f64; 3] {
    let [onset, cruise_end, duration] = boundaries(length, speed, shape);
    if t < 0.0 {
        return [0.0; 3];
    }
    if t >= duration {
        return [length, 0.0, 0.0];
    }
    if t < onset {
        let s = t / onset;
        return match shape.onset {
            Onset::Linear => [0.5 * speed / onset * t * t, speed * s, speed / onset],
            _ => [
                speed * onset * (s.powi(3) - 0.5 * s.powi(4)),
                speed * (3.0 * s * s - 2.0 * s.powi(3)),
                speed / onset * (6.0 * s - 6.0 * s * s),
            ],
        };
    }
    let covered = if shape.onset == Onset::Abrupt {
        0.0
    } else {
        FRACTION * length
    };
    if t <= cruise_end {
        return [covered + speed * (t - onset), speed, 0.0];
    }
    let r = ramp(length, speed);
    let s = (t - cruise_end) / r;
    [
        (1.0 - FRACTION) * length + speed * r * (s - s.powi(3) + 0.5 * s.powi(4)),
        speed * (1.0 - 3.0 * s * s + 2.0 * s.powi(3)),
        speed / r * (-6.0 * s + 6.0 * s * s),
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
fn target(p: ElectromechanicalProfile, t: f64, h: f64, speed: f64, index: usize) -> f64 {
    let Some((_, start)) = onset(p, t) else {
        return p.action.hammer_rest_m;
    };
    if index == 0 {
        // Preserve the original accumulated-slew trajectory exactly.
        return -p.action.escapement_m;
    }
    let length = -p.action.escapement_m - p.action.hammer_rest_m;
    (p.action.hammer_rest_m + stroke(t - start + h, length, speed, shape(index))[0])
        .clamp(p.action.hammer_rest_m, -p.action.escapement_m)
}
// Fraction of the tick [e0, e1] lying inside [lo, hi].
fn overlap(e0: f64, e1: f64, lo: f64, hi: f64) -> f64 {
    ((e1.min(hi) - e0.max(lo)) / (e1 - e0)).clamp(0.0, 1.0)
}
#[derive(Default, Clone)]
struct Segment {
    seconds: f64,
    actuator_work: f64,
    displacement_work: f64,
    impulse: f64,
    peak_force: f64,
    contact_seconds: f64,
    exits: u32,
    peak_acceleration: Option<f64>,
    end: Value,
}
impl Segment {
    fn report(&self, name: &str, lo: f64, hi: f64) -> Value {
        json!({"name":name,"start_seconds_after_key_down":lo,"end_seconds_after_key_down":hi,
            "observed_seconds":self.seconds,"actuator_pedestal_work_j":self.actuator_work,
            "pedestal_force_hammer_displacement_work_j":self.displacement_work,"pedestal_impulse_n_s":self.impulse,
            "peak_pedestal_force_n":self.peak_force,"pedestal_contact_seconds":self.contact_seconds,
            "pedestal_exits":self.exits,"peak_measured_pedestal_acceleration_m_s2":self.peak_acceleration,"end":self.end})
    }
}
struct KeyDown {
    start: f64,
    bounds: [f64; 4],
    segments: [Segment; 4],
    first: Option<ElectromechanicalProbe>,
    last: Option<ElectromechanicalProbe>,
    previous_velocity: Option<f64>,
}
impl KeyDown {
    fn new(start: f64, travel: [f64; 3]) -> Self {
        Self {
            start,
            bounds: [travel[0], travel[1], travel[2], HOLD],
            segments: std::array::from_fn(|_| Segment::default()),
            first: None,
            last: None,
            previous_velocity: None,
        }
    }
    fn observe(&mut self, a: ElectromechanicalProbe, b: ElectromechanicalProbe, t: f64, h: f64) {
        if self.first.is_none() {
            self.first = Some(a);
        }
        self.last = Some(b);
        let e1 = t - self.start + h;
        let e0 = e1 - h;
        let work = b.mechanical.pedestal_work_j - a.mechanical.pedestal_work_j;
        let f = b.mechanical.contact_force_n[2];
        let dh = b.mechanical.position[18] - a.mechanical.position[18];
        let velocity = (b.mechanical.pedestal_position_m - a.mechanical.pedestal_position_m) / h;
        for j in 0..4 {
            let lo = if j == 0 { 0.0 } else { self.bounds[j - 1] };
            let hi = self.bounds[j];
            let share = overlap(e0, e1, lo, hi);
            if share <= 0.0 {
                continue;
            }
            let s = &mut self.segments[j];
            s.seconds += share * h;
            s.actuator_work += share * work;
            s.displacement_work += share * f * dh;
            s.impulse += share * h * f;
            s.contact_seconds += share * h * f64::from(u8::from(f > 0.0));
            if share > 0.5 || (share == 0.5 && e1 <= hi) {
                s.peak_force = s.peak_force.max(f);
                s.exits += u32::from(a.mechanical.contact_force_n[2] > 0.0 && f == 0.0);
            }
            if e0 - h >= lo
                && e1 <= hi
                && hi > lo
                && let Some(previous) = self.previous_velocity
            {
                let acceleration = ((velocity - previous) / h).abs();
                s.peak_acceleration = Some(
                    s.peak_acceleration
                        .map_or(acceleration, |x| x.max(acceleration)),
                );
            }
            if s.end.is_null() && hi > lo && e1 + 1e-9 >= hi {
                s.end = json!({"seconds_after_key_down":e1,"hammer_position_m":b.mechanical.position[18],
                    "hammer_velocity_m_s":b.mechanical.velocity[18],"pedestal_position_m":b.mechanical.pedestal_position_m,
                    "pedestal_force_n":f,"pedestal_compression_m":b.mechanical.compression_m[2],
                    "arm_velocity_m_s":b.mechanical.velocity[19]});
            }
        }
        self.previous_velocity = Some(velocity);
    }
    fn report(&self, p: ElectromechanicalProfile, peaks: [Option<f64>; 2]) -> Value {
        let (Some(first), Some(last)) = (self.first, self.last) else {
            return json!({"passed":false});
        };
        let actuator = last.mechanical.pedestal_work_j - first.mechanical.pedestal_work_j;
        let displacement: f64 = self.segments.iter().map(|s| s.displacement_work).sum();
        let stored = threshold::potential(
            p.action.pedestal_stiffness_n_m2,
            last.mechanical.compression_m[2],
        ) - threshold::potential(
            p.action.pedestal_stiffness_n_m2,
            first.mechanical.compression_m[2],
        );
        let heat = last.mechanical.contact_heat_j[2] - first.mechanical.contact_heat_j[2];
        let scale =
            (last.mechanical.initial_energy_j + last.mechanical.absolute_drive_work_j).max(1e-20);
        let defect = (actuator - displacement - stored - heat).abs() / scale;
        let seconds: f64 = self.segments.iter().map(|s| s.seconds).sum();
        let segment_sum: f64 = self.segments.iter().map(|s| s.actuator_work).sum();
        let acceleration: Vec<Value> = [0, 2]
            .iter()
            .zip(peaks)
            .map(|(j, expected)| {
                let measured = self.segments[*j].peak_acceleration;
                let error = expected.zip(measured).map(|(e, m)| (m - e).abs() / e);
                json!({"segment":SEGMENTS[*j],"analytical_peak_m_s2":expected,"measured_peak_m_s2":measured,
                    "relative_error":error,"passed":expected.is_none() || error.is_some_and(|e|e<1e-3)})
            })
            .collect();
        let passed = defect < 1e-8
            && (seconds - HOLD).abs() < 1e-6
            && (segment_sum - actuator).abs() <= 1e-12 * actuator.abs().max(1e-12)
            && acceleration.iter().all(|a| a["passed"] == true);
        json!({"passed":passed,"start_seconds":self.start,"boundaries_seconds_after_key_down":self.bounds,
            "segments":self.segments.iter().enumerate().map(|(j,s)|s.report(SEGMENTS[j],if j==0{0.0}else{self.bounds[j-1]},self.bounds[j])).collect::<Vec<_>>(),
            "actuator_pedestal_work_j":actuator,"pedestal_force_hammer_displacement_work_j":displacement,
            "pedestal_stored_change_j":stored,"pedestal_heat_change_j":heat,"relative_pedestal_port_defect":defect,
            "observed_seconds":seconds,"ramp_acceleration":acceleration})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    index: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let h = 1.0 / (48000.0 * steps as f64);
    let length = -p.action.escapement_m - p.action.hammer_rest_m;
    let form = shape(index);
    let travel = boundaries(length, speed, form);
    let mut tracking = 0.0_f64;
    let mut maximum = 0.0_f64;
    let mut minimum = 0.0_f64;
    let mut arrivals = [None; 2];
    let mut keys = [
        KeyDown::new(0.03, travel),
        KeyDown::new(REPEAT as f64 / 48000.0, travel),
    ];
    let mut result = launch::take_driven(
        p,
        steps,
        speed,
        REPEAT,
        960,
        |t| target(p, t, h, speed, index),
        |_, a, b, t, h| {
            if let Some((i, start)) = onset(p, t) {
                let expected = (p.action.hammer_rest_m
                    + stroke(t - start + h, length, speed, form)[0])
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
                keys[i].observe(a, b, t, h);
            } else {
                keys[0].previous_velocity = None;
                keys[1].previous_velocity = None;
            }
        },
    )?;
    let expected = travel[2];
    let driver_passed = tracking < 1e-8
        && maximum <= speed * (1.0 + 1e-8)
        && minimum >= -speed * 1e-8
        && arrivals
            .iter()
            .all(|a| a.is_some_and(|t| (t - expected).abs() < 1e-5));
    let onset_peak = peak_onset_acceleration(length, speed, form);
    let stop_peak = form
        .smooth_stop
        .then(|| 0.75 * speed * speed / (FRACTION * length));
    let key_reports: Vec<Value> = keys
        .iter()
        .map(|k| k.report(p, [onset_peak, stop_peak]))
        .collect();
    let segments_passed = key_reports.iter().all(|k| k["passed"] == true);
    result.report["driver"] = json!({"passed":driver_passed,"shape":NAMES[index],"travel_m":length,"nominal_speed_m_s":speed,
        "onset_kind":match form.onset{Onset::Abrupt=>"abrupt",Onset::Smooth=>"smooth",Onset::Linear=>"constant_acceleration"},
        "smooth_stop":form.smooth_stop,"boundaries_seconds_after_key_down":travel,
        "nominal_arrival_seconds_after_key_down":expected,"measured_arrivals_seconds_after_key_down":arrivals,
        "max_tracking_error_m":tracking,"max_key_down_speed_m_s":maximum,"min_key_down_speed_m_s":minimum,
        "onset_peak_acceleration_m_s2":onset_peak,"terminal_peak_deceleration_m_s2":stop_peak});
    result.report["drive_segments"] =
        json!({"passed":segments_passed,"segment_order":SEGMENTS,"key_downs":key_reports});
    result.report["passed"] =
        json!(result.report["passed"] == true && driver_passed && segments_passed);
    Ok(result)
}
fn scalar_error(a: &Value, b: &Value, floor: f64) -> f64 {
    let a = a.as_f64().unwrap();
    let b = b.as_f64().unwrap();
    (a - b).abs() / b.abs().max(floor)
}
// Segment differences are normalized by the segment magnitude or 0.1% of the
// key-down's summed absolute magnitude, whichever is larger. Boundary ticks
// are split in time, so a segment whose pedestal stops mid-tick keeps a small
// resolution-dependent remainder that must not dominate a relative error.
fn segment_error(a: &Value, b: &Value, key: &str, floor: f64) -> f64 {
    let total: f64 = b
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s[key].as_f64().unwrap().abs())
        .sum();
    a.as_array()
        .unwrap()
        .iter()
        .zip(b.as_array().unwrap())
        .map(|(sa, sb)| {
            let (x, y) = (sa[key].as_f64().unwrap(), sb[key].as_f64().unwrap());
            (x - y).abs() / y.abs().max(1e-3 * total).max(floor)
        })
        .fold(0.0_f64, f64::max)
}
pub(super) fn segment_convergence(a: &Value, b: &Value) -> Value {
    let mut rows = Vec::new();
    for (i, (ka, kb)) in a["drive_segments"]["key_downs"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["drive_segments"]["key_downs"].as_array().unwrap())
        .enumerate()
    {
        let (sa, sb) = (&ka["segments"], &kb["segments"]);
        let work = [
            "actuator_pedestal_work_j",
            "pedestal_force_hammer_displacement_work_j",
        ]
        .iter()
        .map(|key| segment_error(sa, sb, key, 1e-10))
        .fold(0.0_f64, f64::max);
        let impulse = segment_error(sa, sb, "pedestal_impulse_n_s", 1e-9);
        let exits = |s: &Value| -> u64 {
            s.as_array()
                .unwrap()
                .iter()
                .map(|x| x["pedestal_exits"].as_u64().unwrap())
                .sum()
        };
        // Exits at a segment boundary may fall on either side at different
        // resolutions, so only the key-down total is compared.
        let mut same = exits(sa) == exits(sb);
        let mut velocity = 0.0_f64;
        for (x, y) in sa.as_array().unwrap().iter().zip(sb.as_array().unwrap()) {
            if x["end"].is_null() || y["end"].is_null() {
                same &= x["end"].is_null() && y["end"].is_null();
            } else {
                velocity = velocity.max(scalar_error(
                    &x["end"]["hammer_velocity_m_s"],
                    &y["end"]["hammer_velocity_m_s"],
                    0.01,
                ));
            }
        }
        rows.push(
            json!({"key_down":i+1,"passed":same && work<0.01 && impulse<0.01 && velocity<0.01,
            "total_exit_counts_agree":same,"total_pedestal_exits":[exits(sa),exits(sb)],
            "max_relative_work_error":work,"max_relative_impulse_error":impulse,
            "max_relative_end_velocity_error":velocity}),
        );
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"key_downs":rows})
}
fn compare_first(candidate: &Value, reference: &Value) -> Value {
    let mut first = candidate["repetition"]["phases"][0].clone();
    first["start_seconds"] = json!(0.03);
    repetition::repeated_attack(&reference["repetition"]["phases"][0], &first)
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-drive-onset --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("drive onset study requires a new JSON path".into());
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
                    println!("Loaded drive onset: {name}, {driver}, {speed} m/s, 128/256 ticks");
                    let a = take(p, 128, *speed, index)?;
                    let b = take(p, 256, *speed, index)?;
                    let c = repetition::convergence(&a, &b, REPEAT);
                    let l = launch::convergence(&a.report, &b.report);
                    let s = segment_convergence(&a.report, &b.report);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true
                        && l["passed"] == true
                        && s["passed"] == true;
                    let first = compare_first(
                        &b.report,
                        if index == 0 {
                            &b.report
                        } else {
                            &drivers[0]["rows"][row]["takes"][1]
                        },
                    );
                    let same_arrival = (index == 2)
                        .then(|| compare_first(&b.report, &drivers[1]["rows"][row]["takes"][1]));
                    println!(
                        "Loaded drive onset: qualified={qualified}, repeatable={}",
                        b.report["repetition"]["two_clean_repeatable_strikes"]
                    );
                    rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_original":first,
                        "first_vs_onset_ease_same_arrival":same_arrival,"takes":[a.report,b.report],"convergence":c,
                        "launch_convergence":l,"segment_convergence":s}));
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
                d.len() == 4
                    && d.iter().all(|d| {
                        d["rows"].as_array().is_some_and(|r| {
                            r.len() == 2 && r.iter().all(|r| r["measurement_qualified"] == true)
                        })
                    })
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-drive-onset-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"profiles":profiles,"segment_order":SEGMENTS,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 32-take matrix: original and pedestal-rate-loss-10 profiles; nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. Four key-down drivers over the same 10.5 mm travel: original constant slew; smooth onset over the first 20% of travel (velocity v*(3s^2-2s^3), ramp 0.4*travel/v, continuous velocity and acceleration, abrupt stop); constant-acceleration onset covering the same first 20% in the same ramp time (acceleration v^2/(0.4*travel), discontinuous at both ramp ends, abrupt stop); and full ease with the smooth onset plus the retained smooth terminal deceleration over the last 20%. Arrival times are 1.0, 1.2, 1.2 and 1.4 times travel/v. Key-up retains abrupt constant slew. Analytical tracking error <1e-8 m, key-down speed within [0, v] to 1e-8 relative, arrival within 10 us. Each key-down is divided into onset, cruise, stop and hold segments; boundary ticks are split proportionally in time. Segments retain actuator pedestal work, pedestal force times hammer displacement, pedestal impulse, peak force, contact time, pedestal exits, interior finite-difference peak acceleration and the end state. Per key-down the actuator work minus hammer-displacement work must equal pedestal storage plus heat change within 1e-8 relative; segment times sum to the 120 ms hold within 1 us; measured ramp peak acceleration agrees with the analytical peak within 0.1%. Segment work and impulse must refine within 1%, normalized by the segment magnitude or 0.1% of the key-down summed absolute magnitude, whichever is larger, with 1e-10 J and 1e-9 Ns floors; end hammer velocity within 1% (0.01 m/s floor); key-down total pedestal exit counts equal between resolutions, since a boundary exit may fall on either side of a segment boundary. All repetition and launch gates are retained. Constant-slew takes replay the retained drive-release receipt. First-strike comparison against the same-profile original and between the two same-arrival onset shapes uses the prior 5% impact/speed and 1 ms latency limits. Repeatability retains within-take criteria. No candidate selection.",
        "scope":"Prescribed-motion intervention on the pedestal; not a measured key, finger force or finite-inertia mechanism. Onset ramps lengthen arrival; the constant-acceleration control shares the arrival time and ramp displacement of the smooth onset but not its jerk. No new contact law, geometry, gain, source fit, audio render or production default. Only the 60 ms repetition wait is covered."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("drive onset study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn onset_curves_cover_travel_with_declared_durations_and_peak_accelerations() {
        let length = 0.0105;
        let speed = 1.125;
        for index in 0..4 {
            let form = shape(index);
            let [onset, cruise_end, end] = boundaries(length, speed, form);
            let factor = [1.0, 1.2, 1.2, 1.4][index];
            assert!((end - factor * length / speed).abs() < 1e-15);
            assert_eq!(stroke(end, length, speed, form), [length, 0.0, 0.0]);
            assert_eq!(stroke(-1e-9, length, speed, form), [0.0; 3]);
            if onset > 0.0 {
                let reached = stroke(onset, length, speed, form);
                assert!((reached[0] - FRACTION * length).abs() < 1e-15);
                assert!((reached[1] - speed).abs() < 1e-12);
            }
            let cruise = stroke(0.5 * (onset + cruise_end), length, speed, form);
            assert_eq!(cruise[1], speed);
            assert_eq!(cruise[2], 0.0);
            let mut integral = 0.0;
            let mut peak = 0.0_f64;
            let n = 20000;
            let h = end / n as f64;
            for i in 0..n {
                let t = (i as f64 + 0.5) * h;
                let s = stroke(t, length, speed, form);
                assert!((-1e-12..=speed + 1e-12).contains(&s[1]));
                integral += s[1] * h;
                let interior = |lo: f64, hi: f64| t > lo + 1e-6 && t < hi - 1e-6;
                if interior(0.0, onset) || interior(onset, cruise_end) || interior(cruise_end, end)
                {
                    let eps = 1e-7;
                    let lo = stroke(t - eps, length, speed, form);
                    let hi = stroke(t + eps, length, speed, form);
                    assert!(((hi[0] - lo[0]) / (2.0 * eps) - s[1]).abs() < 1e-7);
                    assert!(((hi[1] - lo[1]) / (2.0 * eps) - s[2]).abs() < 1e-4);
                }
                if t < onset {
                    peak = peak.max(s[2]);
                }
            }
            assert!((integral - length).abs() < 1e-9);
            match peak_onset_acceleration(length, speed, form) {
                None => assert_eq!(peak, 0.0),
                Some(expected) => assert!((peak - expected).abs() < 1e-3 * expected),
            }
        }
        // The smooth onset has continuous velocity at its start; the linear onset does not.
        assert!(stroke(1e-6, length, speed, shape(1))[1] < 1e-6);
        assert!(stroke(1e-6, length, speed, shape(2))[1] > 1e-4);
    }
    #[test]
    fn full_ease_terminal_segment_matches_retained_terminal_ease_shifted_by_onset() {
        let length = 0.0105;
        let speed = 1.5;
        let form = shape(3);
        let [onset, _, end] = boundaries(length, speed, form);
        let shift = onset - FRACTION * length / speed;
        for i in 0..200 {
            let t = onset + (end - onset) * (i as f64 + 0.5) / 200.0;
            let a = stroke(t, length, speed, form);
            let b = super::super::drive::stroke(t - shift, length, speed, 1);
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 1e-9 * (1.0 + b[k].abs()));
            }
        }
    }
    #[test]
    fn drivers_keep_original_control_release_targets_and_start_below_travel_end() {
        let p = repetition::profile(0.8, 0);
        let h = 1.0 / (48000.0 * 128.0);
        for t in [0.0, 0.029, 0.15, 0.16, 0.209, 0.34] {
            for index in 0..4 {
                assert_eq!(target(p, t, h, 1.125, index), p.action.hammer_rest_m);
            }
        }
        assert_eq!(target(p, 0.03, h, 1.125, 0), -p.action.escapement_m);
        for index in 1..4 {
            let first = target(p, 0.03, h, 1.125, index);
            assert!(first > p.action.hammer_rest_m && first < p.action.hammer_rest_m + 1.125 * h);
            assert_eq!(target(p, 0.06, h, 1.125, index), -p.action.escapement_m);
        }
    }
    #[test]
    fn segment_refinement_tolerates_boundary_remainders_but_not_large_segment_changes() {
        let b = json!([{"w":1e-2},{"w":1e-9},{"w":0.0}]);
        let remainder = json!([{"w":1e-2},{"w":2.5e-8},{"w":0.0}]);
        assert!(segment_error(&remainder, &b, "w", 1e-10) < 0.01);
        let changed = json!([{"w":1.02e-2},{"w":1e-9},{"w":0.0}]);
        assert!(segment_error(&changed, &b, "w", 1e-10) > 0.01);
        let zero = json!([{"w":0.0},{"w":0.0}]);
        assert_eq!(segment_error(&zero, &zero, "w", 1e-10), 0.0);
    }
    #[test]
    fn boundary_ticks_split_proportionally_and_segment_shares_sum_to_one() {
        assert_eq!(overlap(0.0, 1.0, 0.0, 0.25), 0.25);
        assert_eq!(overlap(0.0, 1.0, 0.25, 2.0), 0.75);
        assert_eq!(overlap(2.0, 3.0, 0.0, 1.0), 0.0);
        assert_eq!(overlap(0.0, 1.0, 0.0, 0.0), 0.0);
        let bounds = [0.3, 1.1, 1.1, 2.0];
        for i in 0..40 {
            let e0 = i as f64 * 0.05;
            let e1 = e0 + 0.05;
            let total: f64 = (0..4)
                .map(|j| overlap(e0, e1, if j == 0 { 0.0 } else { bounds[j - 1] }, bounds[j]))
                .sum();
            assert!((total - 1.0).abs() < 1e-12);
        }
    }
}
