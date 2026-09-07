//! Two strikes on one persistent electromechanical state, with phase-local evidence.
use super::{bridle, tuning, voicing};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const SPEEDS: [f64; 2] = [1.125, 1.5];
const REPEATS: [usize; 2] = [10080, 21600]; // 60/300 ms after first release.
const NAMES: [&str; 3] = ["baseline", "return_damping_0_1", "pedestal_rate_loss_10"];
const PHASES: [&str; 4] = ["first", "recovery", "second", "second_release"];

fn profile(position: f64, case: usize) -> ElectromechanicalProfile {
    let mut p = voicing::profile(position, 0);
    match case {
        1 => p.action.hammer_return_n_s_m = 0.1,
        2 => p.action.pedestal_rate_loss_s_m = 10.0,
        _ => {}
    }
    p
}
fn phase(frame: usize, repeat: usize) -> usize {
    if frame < 7200 {
        0
    } else if frame < repeat {
        1
    } else if frame < repeat + 5760 {
        2
    } else {
        3
    }
}
fn target(p: ElectromechanicalProfile, t: f64, repeat: usize) -> f64 {
    let start = repeat as f64 / 48000.0;
    if (0.03..0.15).contains(&t) || (start..start + 0.12).contains(&t) {
        -p.action.escapement_m
    } else {
        p.action.hammer_rest_m
    }
}
fn state(b: ElectromechanicalProbe) -> Value {
    json!({"position_m":b.mechanical.position,"velocity_m_s":b.mechanical.velocity,
        "compression_m":b.mechanical.compression_m,"contact_force_n":b.mechanical.contact_force_n,
        "pedestal_position_m":b.mechanical.pedestal_position_m,"pedal_position_m":b.mechanical.pedal_position_m,
        "mechanical_energy_j":b.mechanical.mechanical_energy_j,"electrical_energy_j":b.electrical_energy_j,
        "current_a":b.current_a,"output_voltage_v":b.output_voltage_v,"contact_entries":b.mechanical.contact_entries})
}
#[derive(Default)]
struct Segment {
    impulse: f64,
    peak: f64,
    ticks: u64,
    entries: Vec<Value>,
    overflow: bool,
    voltage_energy: f64,
    carry_in: bool,
    carry_out: bool,
}
impl Segment {
    fn observe(&mut self, a: ElectromechanicalProbe, b: ElectromechanicalProbe, t: f64, h: f64) {
        let f = b.mechanical.contact_force_n[0];
        self.impulse += h * f;
        self.peak = self.peak.max(f);
        self.ticks += u64::from(f > 0.0);
        self.voltage_energy += h * b.output_voltage_v.powi(2);
        self.carry_out = f > 0.0;
        if b.mechanical.contact_entries[0] > a.mechanical.contact_entries[0] {
            if self.entries.len() < 16 {
                self.entries.push(json!({"seconds":t+h,"before":state(a)}));
            } else {
                self.overflow = true;
            }
        }
    }
    fn report(&self, h: f64, start: f64, end: f64) -> Value {
        json!({"start_seconds":start,"end_seconds":end,"entries":self.entries,"overflow":self.overflow,
            "carry_in":self.carry_in,"carry_out":self.carry_out,
            "impact":{"impulse_n_s":self.impulse,"peak_force_n":self.peak,"active_contact_seconds":self.ticks as f64*h},
            "raw_voltage_rms_v":(self.voltage_energy/(end-start)).sqrt()})
    }
}
fn repeated_attack(first: &Value, second: &Value) -> Value {
    let first_entries = first["entries"].as_array().unwrap();
    let second_entries = second["entries"].as_array().unwrap();
    let errors: Vec<Option<f64>> = ["impulse_n_s", "peak_force_n", "active_contact_seconds"]
        .iter()
        .map(|key| {
            let base = first["impact"][key].as_f64().unwrap();
            (base > 0.0).then(|| (second["impact"][key].as_f64().unwrap() - base).abs() / base)
        })
        .collect();
    let velocity_error = first_entries
        .first()
        .zip(second_entries.first())
        .and_then(|(a, b)| {
            let va = a["before"]["velocity_m_s"][18].as_f64().unwrap();
            let vb = b["before"]["velocity_m_s"][18].as_f64().unwrap();
            (va > 0.0).then_some((vb - va).abs() / va)
        });
    let latency_error = first_entries
        .first()
        .zip(second_entries.first())
        .map(|(a, b)| {
            let first_latency = a["seconds"].as_f64().unwrap() - 0.03;
            let second_latency =
                b["seconds"].as_f64().unwrap() - second["start_seconds"].as_f64().unwrap();
            (second_latency - first_latency).abs()
        });
    json!({"passed":first_entries.len()==1 && second_entries.len()==1
        && errors.iter().all(|e|e.is_some_and(|e|e<0.05)) && velocity_error.is_some_and(|e|e<0.05)
        && latency_error.is_some_and(|e|e<0.001),
        "relative_impact_errors":errors,"relative_preimpact_speed_error":velocity_error,"latency_difference_seconds":latency_error})
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    repeat: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let frames = repeat + 9120; // 120 ms second hold and 70 ms release observation.
    let h = 1.0 / (48000.0 * steps as f64);
    let mut segments: [Segment; 4] = std::array::from_fn(|_| Segment::default());
    let mut tick = 0usize;
    let mut before_repeat = Value::Null;
    let mut ready_position = [0.0_f64; 2];
    let mut ready_velocity = [0.0_f64; 2];
    let mut ready_felt_ticks = 0u64;
    let mut lift = [f64::INFINITY; 2];
    let mut felt_ticks = [0u64; 2];
    let boundaries = [0, 7200, repeat, repeat + 5760, frames];
    let mut result = bridle::take_driven(
        p,
        steps,
        speed,
        frames,
        |t| target(p, t, repeat),
        |initial, a, b, t, h| {
            let frame = tick / steps;
            let index = phase(frame, repeat);
            if tick == boundaries[index] * steps {
                segments[index].carry_in = a.mechanical.contact_force_n[0] > 0.0;
            }
            if tick == repeat * steps {
                before_repeat = state(a);
            }
            segments[index].observe(a, b, t, h);
            if (repeat - 960..repeat).contains(&frame) {
                for i in 0..2 {
                    ready_position[i] = ready_position[i].max(
                        (b.mechanical.position[18 + i] - initial.mechanical.position[18 + i]).abs(),
                    );
                    ready_velocity[i] = ready_velocity[i].max(b.mechanical.velocity[18 + i].abs());
                }
                ready_felt_ticks += u64::from(b.mechanical.contact_force_n[1] > 0.0);
            }
            for (i, start) in [1440, repeat].iter().enumerate() {
                if (start + 2400..start + 5280).contains(&frame) {
                    lift[i] = lift[i].min(-b.mechanical.compression_m[1]);
                    felt_ticks[i] += u64::from(b.mechanical.contact_force_n[1] > 0.0);
                }
            }
            tick += 1;
        },
    )?;
    let reports: Vec<Value> = segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.report(
                h,
                boundaries[i] as f64 / 48000.0,
                boundaries[i + 1] as f64 / 48000.0,
            )
        })
        .collect();
    let repeatability = repeated_attack(&reports[0], &reports[2]);
    let felt_fraction = ready_felt_ticks as f64 / (960 * steps) as f64;
    let ready = ready_position.iter().all(|x| *x < 0.0001)
        && ready_velocity.iter().all(|x| *x < 0.01)
        && felt_fraction >= 0.9;
    let lifted = lift.iter().all(|x| *x >= 0.0001) && felt_ticks == [0, 0];
    let separated = segments.iter().all(|s| !s.carry_in && !s.carry_out)
        && [1, 3]
            .iter()
            .all(|i| segments[*i].ticks == 0 && segments[*i].entries.is_empty());
    let passed = result.report["passed"] == true && segments.iter().all(|s| !s.overflow);
    let whole_impact = result
        .report
        .as_object_mut()
        .unwrap()
        .remove("impact")
        .unwrap();
    let old_function = result
        .report
        .as_object_mut()
        .unwrap()
        .remove("function")
        .unwrap();
    result.report["whole_gesture_impact"] = whole_impact;
    result.report["passed"] = json!(passed);
    result.report["repetition"] = json!({"before_second_drive":before_repeat,"phases":reports,
        "readiness":{"passed":ready,"window_start_seconds":(repeat-960) as f64/48000.0,"window_end_seconds":repeat as f64/48000.0,
            "max_position_error_m":ready_position,"max_velocity_m_s":ready_velocity,"felt_contact_fraction":felt_fraction},
        "attack_repeatability":repeatability,"minimum_held_felt_clearance_m":lift,"held_felt_contact_ticks":felt_ticks,
        "both_lifts_passed":lifted,"separated_strikes":separated,
        "two_clean_repeatable_strikes":repeatability["passed"]==true && lifted && separated,
        "terminal_return":{"passed":old_function["return_passed"],"max_position_error_m":old_function["return_max_position_error_m"],
            "max_velocity_m_s":old_function["return_max_velocity_m_s"],"felt_contact_fraction":old_function["return_felt_contact_fraction"]}});
    Ok(result)
}
fn convergence(a: &bridle::Take, b: &bridle::Take, repeat: usize) -> Value {
    let boundaries = [1440, 7200, repeat, repeat + 5760, b.velocities.len()];
    let mut velocities = Vec::new();
    for window in boundaries.windows(2) {
        let (lo, hi) = (window[0], window[1]);
        for (i, name) in ["hammer", "arm", "pickup_vertical", "pickup_horizontal"]
            .iter()
            .enumerate()
        {
            let error = a.velocities[lo..hi]
                .iter()
                .zip(&b.velocities[lo..hi])
                .map(|(a, b)| (a[i] - b[i]).powi(2))
                .sum::<f64>();
            let signal = b.velocities[lo..hi]
                .iter()
                .map(|v| v[i] * v[i])
                .sum::<f64>();
            let e = (error / signal.max((hi - lo) as f64 * 1e-16)).sqrt();
            velocities.push(json!({"channel":name,"start_seconds":lo as f64/48000.0,"end_seconds":hi as f64/48000.0,
                "relative_velocity_rmse":e,"passed":e<0.01}));
        }
    }
    let mut phases = Vec::new();
    for (i, name) in PHASES.iter().enumerate() {
        let aa = &a.report["repetition"]["phases"][i];
        let bb = &b.report["repetition"]["phases"][i];
        let ea = aa["entries"].as_array().unwrap();
        let eb = bb["entries"].as_array().unwrap();
        let timing = ea
            .iter()
            .zip(eb)
            .map(|(a, b)| (a["seconds"].as_f64().unwrap() - b["seconds"].as_f64().unwrap()).abs())
            .fold(0.0_f64, f64::max);
        let impact = if bb["impact"]["impulse_n_s"] == 0.0 {
            json!({"passed":aa["impact"]==bb["impact"]})
        } else {
            voicing::impact_convergence(&aa["impact"], &bb["impact"])
        };
        phases.push(json!({"name":name,"passed":ea.len()==eb.len() && timing<0.0001 && impact["passed"]==true
            && aa["carry_in"]==bb["carry_in"] && aa["carry_out"]==bb["carry_out"],"entry_count_agrees":ea.len()==eb.len(),
            "max_entry_time_error_seconds":timing,"impact":impact}));
    }
    json!({"passed":velocities.iter().chain(phases.iter()).all(|r|r["passed"]==true),"velocity":velocities,"phases":phases})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-repetition --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("repetition study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let p = profile(position, case);
            let mut rows = Vec::new();
            for speed in SPEEDS {
                for repeat in REPEATS {
                    println!(
                        "Loaded repetition: {name}, {speed} m/s, repeat at {} ms, 128/256 ticks",
                        repeat as f64 / 48.0
                    );
                    let a = take(p, 128, speed, repeat)?;
                    let b = take(p, 256, speed, repeat)?;
                    let c = convergence(&a, &b, repeat);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true;
                    println!(
                        "Loaded repetition: qualified={qualified}, repeatable={}",
                        b.report["repetition"]["two_clean_repeatable_strikes"]
                    );
                    rows.push(json!({"speed_m_s":speed,"release_to_repeat_seconds":(repeat-7200) as f64/48000.0,
                    "measurement_qualified":qualified,"takes":[a.report,b.report],"convergence":c}));
                }
            }
            cases.push(json!({"name":name,"rows":rows,"settings":{"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m}}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && cases.len() == 3
        && cases.iter().all(|c| {
            c["rows"].as_array().is_some_and(|r| {
                r.len() == 4 && r.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-repetition-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "phase_order":PHASES,
        "protocol":"Frozen 24-take matrix: conditional 70 mm G3, baseline/return damping 0.1 Ns/m/pedestal rate loss 10 s/m, drive speeds 1.125/1.5 m/s, second key-down 60/300 ms after first key-up. First key-down 30 ms, first key-up 150 ms, second held 120 ms then released for 70 ms. Closed pedal; same bounded down/up slew, one prepared model per take and no state reset between strokes. 128/256 ticks per 48 kHz frame. Existing independent energy/heat/exchange/quiet-rest gates retained. Phase-local contact entry states, impulse/peak/duration, boundary contact carry, raw voltage RMS and full pre-repeat mechanical/electrical state. Match phase contact counts/carry, timing <0.1 ms, impact refinement <1%, four-channel velocity RMSE <1% separately over first drive/recovery/second hold/second release with 1e-8 m/s floor. Readiness independently requires both action positions <0.1 mm from prepared rest, speeds <0.01 m/s and felt contact >=90% during 20 ms before repeat. Repeatability compares second against first within each profile: exactly one strike each, <5% impulse/peak/duration/preimpact speed changes, latency shift <1 ms. Two clean repeatable strikes additionally require no recovery/release contact or boundary carry, and >=0.1 mm felt clearance with zero contact during 50-110 ms after each key-down. Readiness and terminal return remain separate from repeatability. All limits are engineering diagnostics, not calibrated regulation tolerances.",
        "scope":"Finite two-strike offline study with inherited first-strike tradeoffs. Retains residual tine, action and circuit state; raw voltage RMS includes overlap and is not isolated-note loudness or timbre equivalence. No automatic loss selection, source fitting, audio render or production change. Does not establish a maximum repetition rate, arbitrary velocities, half-pedal behavior or full-keyboard realism."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("repetition study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_gesture_keeps_release_interval_and_bounded_endpoints() {
        let p = profile(0.8, 0);
        for repeat in REPEATS {
            let start = repeat as f64 / 48000.0;
            assert_eq!(target(p, 0.149, repeat), -p.action.escapement_m);
            assert_eq!(target(p, 0.15, repeat), p.action.hammer_rest_m);
            assert_eq!(target(p, start - 1e-6, repeat), p.action.hammer_rest_m);
            assert_eq!(target(p, start, repeat), -p.action.escapement_m);
            assert_eq!(target(p, start + 0.12, repeat), p.action.hammer_rest_m);
            assert_eq!(
                [
                    phase(7199, repeat),
                    phase(7200, repeat),
                    phase(repeat, repeat),
                    phase(repeat + 5760, repeat)
                ],
                [0, 1, 2, 3]
            );
        }
    }
    #[test]
    fn repeatability_rejects_missing_or_multiple_strikes_and_timing_changes() {
        let p = profile(0.8, 0);
        let model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
        let mut b = model.probe();
        b.mechanical.velocity[18] = 0.5;
        let first = json!({"start_seconds":0.0,"entries":[{"seconds":0.04,"before":state(b)}],
            "impact":{"impulse_n_s":0.001,"peak_force_n":1.0,"active_contact_seconds":0.001}});
        let mut second = first.clone();
        second["start_seconds"] = json!(0.21);
        second["entries"][0]["seconds"] = json!(0.22);
        assert_eq!(repeated_attack(&first, &second)["passed"], true);
        second["entries"][0]["seconds"] = json!(0.23);
        assert_eq!(repeated_attack(&first, &second)["passed"], false);
        second["entries"] = json!([]);
        assert_eq!(repeated_attack(&first, &second)["passed"], false);
        second["entries"] = json!([first["entries"][0], first["entries"][0]]);
        assert_eq!(repeated_attack(&first, &second)["passed"], false);
    }
}
