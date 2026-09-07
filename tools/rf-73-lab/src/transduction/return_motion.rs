//! Long-tail hammer return: free-flight oracle, support events and attack tradeoffs.
use super::{bridle, tuning, voicing};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const FRAMES: usize = 38400;
const SPEEDS: [f64; 2] = [1.125, 1.5];
const NAMES: [&str; 3] = ["baseline", "return_damping_0_1", "pedestal_rate_loss_10"];

// Exact continuous solution of m*x'' + c*x' + k*x = 0 about hammer rest.
// All frozen cases are underdamped; keep this oracle independent of the solver.
fn free_state(x: f64, v: f64, t: f64, m: f64, k: f64, c: f64) -> [f64; 2] {
    let alpha = c / (2.0 * m);
    let omega = (k / m - alpha * alpha).sqrt();
    let (sin, cos) = (omega * t).sin_cos();
    let e = (-alpha * t).exp();
    [
        e * (x * cos + (v + alpha * x) / omega * sin),
        e * (v * cos - (alpha * v + k / m * x) / omega * sin),
    ]
}
fn hammer_energy(p: ElectromechanicalProfile, b: ElectromechanicalProbe) -> f64 {
    0.5 * p.assembly.hammer_mass_kg * b.mechanical.velocity[18].powi(2)
        + 0.5
            * p.action.hammer_return_n_m
            * (b.mechanical.position[18] - p.action.hammer_rest_m).powi(2)
}
fn state(p: ElectromechanicalProfile, b: ElectromechanicalProbe) -> Value {
    json!({"position_m":b.mechanical.position[18],"velocity_m_s":b.mechanical.velocity[18],
        "energy_j":hammer_energy(p,b),"pedestal_position_m":b.mechanical.pedestal_position_m,
        "compression_m":b.mechanical.compression_m,"force_n":b.mechanical.contact_force_n,
        "pedestal_heat_j":b.mechanical.contact_heat_j[2],"return_heat_j":b.mechanical.hammer_return_heat_j,
        "pedestal_work_j":b.mechanical.pedestal_work_j})
}
#[derive(Default)]
struct ReturnTrace {
    events: Vec<Value>,
    flights: Vec<Value>,
    free_start: Option<(f64, ElectromechanicalProbe)>,
    overflow: bool,
    last_violation: f64,
    max_free_error: f64,
}
impl ReturnTrace {
    fn finish_flight(&mut self, p: ElectromechanicalProfile, b: ElectromechanicalProbe, t: f64) {
        let Some((start, a)) = self.free_start.take() else {
            return;
        };
        let dt = t - start;
        if dt < 0.001 {
            return;
        }
        let m = p.assembly.hammer_mass_kg;
        let k = p.action.hammer_return_n_m;
        let x = a.mechanical.position[18] - p.action.hammer_rest_m;
        let v = a.mechanical.velocity[18];
        let predicted = free_state(x, v, dt, m, k, p.action.hammer_return_n_s_m);
        let actual = [
            b.mechanical.position[18] - p.action.hammer_rest_m,
            b.mechanical.velocity[18],
        ];
        let error = ((k / m * (actual[0] - predicted[0]).powi(2)
            + (actual[1] - predicted[1]).powi(2))
            / (k / m * x * x + v * v).max(1e-16))
        .sqrt();
        self.max_free_error = self.max_free_error.max(error);
        if self.flights.len() == 256 {
            self.overflow = true;
            return;
        }
        self.flights.push(json!({"start_seconds":start,"end_seconds":t,"initial_offset_m":x,
            "initial_velocity_m_s":v,"predicted_offset_m":predicted[0],"predicted_velocity_m_s":predicted[1],
            "actual_offset_m":actual[0],"actual_velocity_m_s":actual[1],"relative_state_error":error,
            "initial_energy_j":hammer_energy(p,a),"final_energy_j":hammer_energy(p,b),
            "return_heat_j":b.mechanical.hammer_return_heat_j-a.mechanical.hammer_return_heat_j}));
    }
    fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        initial: ElectromechanicalProbe,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        if t < 0.15 {
            return;
        }
        let active = |s: ElectromechanicalProbe| s.mechanical.contact_force_n[2] > 0.0;
        if active(a) != active(b) {
            if self.events.len() < 256 {
                self.events.push(
                    json!({"seconds":t+h,"kind":if active(b){"entry"}else{"exit"},
                    "before":state(p,a),"after":state(p,b)}),
                );
            } else {
                self.overflow = true;
            }
        }
        // A free interval contains no force from any of the three hammer ports.
        let free = [0, 2, 3]
            .iter()
            .all(|i| b.mechanical.contact_force_n[*i] == 0.0);
        if free {
            if self.free_start.is_none() {
                self.free_start = Some((t, a));
            }
        } else {
            self.finish_flight(p, a, t);
        }
        if (b.mechanical.position[18] - initial.mechanical.position[18]).abs() >= 0.0001
            || b.mechanical.velocity[18].abs() >= 0.01
        {
            self.last_violation = t + h;
        }
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut trace = ReturnTrace::default();
    let mut final_probe = None;
    let mut result = bridle::take_observed(p, steps, speed, FRAMES, |initial, a, b, t, h| {
        trace.observe(p, initial, a, b, t, h);
        final_probe = Some(b);
    })?;
    let end = final_probe.ok_or("missing final return state")?;
    trace.finish_flight(p, end, 0.8);
    let qualified = !trace.overflow && !trace.flights.is_empty() && trace.max_free_error < 1e-6;
    result.report["return_trace"] = json!({"qualified":qualified,"overflow":trace.overflow,
        "pedestal_events":trace.events,"free_intervals":trace.flights,"max_free_relative_state_error":trace.max_free_error,
        "hammer_last_tolerance_violation_seconds":trace.last_violation,
        "observed_terminal_settled_duration_seconds":0.8-trace.last_violation,"final":state(p,end)});
    result.report["passed"] = json!(result.report["passed"] == true && qualified);
    Ok(result)
}
fn event_convergence(a: &Value, b: &Value) -> Value {
    let a = a["return_trace"]["pedestal_events"].as_array().unwrap();
    let b = b["return_trace"]["pedestal_events"].as_array().unwrap();
    let same = a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a["kind"] == b["kind"]);
    let error = a
        .iter()
        .zip(b)
        .map(|(a, b)| (a["seconds"].as_f64().unwrap() - b["seconds"].as_f64().unwrap()).abs())
        .fold(0.0_f64, f64::max);
    json!({"passed":same && error<0.0001,"sequence_agrees":same,"max_time_error_seconds":error})
}
fn attack_comparison(a: &Value, baseline: &Value) -> Value {
    let fields = ["impulse_n_s", "peak_force_n", "active_contact_seconds"];
    let errors: Vec<f64> = fields
        .iter()
        .map(|key| {
            let base = baseline["impact"][key].as_f64().unwrap();
            (a["impact"][key].as_f64().unwrap() - base).abs() / base.max(1e-20)
        })
        .collect();
    let va = a["first_contact"]["before_hammer"]["hammer_velocity_m_s"].as_f64();
    let vb = baseline["first_contact"]["before_hammer"]["hammer_velocity_m_s"]
        .as_f64()
        .unwrap();
    let speed_error = va.map(|v| (v - vb).abs() / vb.abs().max(1e-20));
    let time_error = a["first_contact"]["seconds"]
        .as_f64()
        .map(|t| (t - baseline["first_contact"]["seconds"].as_f64().unwrap()).abs());
    json!({"passed":a["contact_entries"][0]==1 && errors.iter().all(|e|*e<0.05)
        && speed_error.is_some_and(|e|e<0.05) && time_error.is_some_and(|e|e<0.001),
        "relative_impact_errors":errors,"relative_preimpact_speed_error":speed_error,"first_contact_time_error_seconds":time_error})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-hammer-return --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("hammer return study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let mut p = voicing::profile(position, 0);
            match case {
                1 => p.action.hammer_return_n_s_m = 0.1,
                2 => p.action.pedestal_rate_loss_s_m = 10.0,
                _ => {}
            }
            let mut rows = Vec::new();
            for (index, speed) in SPEEDS.iter().enumerate() {
                println!("Loaded hammer return: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let convergence = bridle::convergence(&a, &b);
                let events = event_convergence(&a.report, &b.report);
                let attack = attack_comparison(
                    &b.report,
                    if case == 0 {
                        &b.report
                    } else {
                        &cases[0]["rows"][index]["takes"][1]
                    },
                );
                let qualified = a.report["passed"] == true
                    && b.report["passed"] == true
                    && convergence["passed"] == true
                    && events["passed"] == true;
                let functional = attack["passed"] == true
                    && b.report["function"]["single_strike_lift_return_passed"] == true;
                println!(
                    "Loaded hammer return: {name}, {speed} m/s, qualified={qualified}, attack/lift/return={functional}"
                );
                rows.push(json!({"speed_m_s":speed,"measurement_qualified":qualified,"attack_lift_return_passed":functional,
                    "attack_vs_baseline":attack,"takes":[a.report,b.report],"convergence":convergence,"event_convergence":events}));
            }
            cases.push(json!({"name":name,"rows":rows,"settings":{"hammer_mass_kg":p.assembly.hammer_mass_kg,
                "return_stiffness_n_m":p.action.hammer_return_n_m,"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m}}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && cases.len() == 3
        && cases.iter().all(|c| {
            c["rows"].as_array().is_some_and(|rows| {
                rows.len() == 2 && rows.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-hammer-return-v1","measurement_qualified":passed,
        "failure_reason":failure,"structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen three-case/two-speed study, twelve 800 ms takes at 128/256 ticks per 48 kHz frame. Conditional 70 mm G3, stationary rest, key down 30 ms, return 150 ms, closed pedal. Slew 1.125/1.5 m/s. Baseline versus constant return damping 0.1 instead of 0.025 Ns/m, versus pedestal rate loss 10 instead of 2 s/m. Reuse independent hammer/bridle/arm/felt work and heat qualification. Keep 120/150/220/400 ms snapshots, append 800 ms. Return gate over 750-800 ms: both action positions <0.1 mm from prepared rest, speeds <0.01 m/s, felt contact >=90%. Lift gate unchanged at 80-140 ms. All return pedestal force entry/exit events retained up to 256; overflow fails. Match event sequence and timing <0.1 ms across resolutions; four-channel velocity RMSE <1% separately over 30-150/150-220/220-400/400-800 ms and impact refinement <1%. For every force-free hammer interval >=1 ms after release, compare terminal state against exact continuous damped oscillator in energy norm, relative to initial state with 1e-8 m/s floor; error <1e-6. Separate functional comparison: single strike and baseline-relative impulse/peak/duration/preimpact speed changes <5%, first contact shift <1 ms, plus held lift and final return. These are frozen engineering gates, not measured regulation tolerances.",
        "scope":"Offline diagnosis of existing constant spring/damper and pedestal contact laws. No time-switched damping, backcheck claim, source fit, WAV, default change or plugin integration. Settling is observed only until 800 ms; attack preservation is not spectral or listening equivalence. Two gestures cannot establish velocity monotonicity, repetition or physical calibration."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("hammer return study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_oracle_preserves_undamped_energy_and_composes_damped_time() {
        let x = 0.001;
        let v = 0.03;
        let m = 0.004;
        let k = 4.0;
        let energy = |s: [f64; 2]| 0.5 * k * s[0] * s[0] + 0.5 * m * s[1] * s[1];
        let conservative = free_state(x, v, 0.2, m, k, 0.0);
        assert!((energy(conservative) - energy([x, v])).abs() < 1e-18);
        for c in [0.025, 0.1] {
            let a = free_state(x, v, 0.12, m, k, c);
            let b = free_state(a[0], a[1], 0.23, m, k, c);
            let whole = free_state(x, v, 0.35, m, k, c);
            assert!((b[0] - whole[0]).abs() < 1e-16 && (b[1] - whole[1]).abs() < 1e-16);
            assert!(energy(whole) < energy([x, v]));
        }
    }
    #[test]
    fn free_interval_ledger_detects_unmodelled_force() {
        let p = voicing::profile(0.8, 0);
        let model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
        let mut a = model.probe();
        a.mechanical.position[18] = p.action.hammer_rest_m + 0.001;
        a.mechanical.velocity[18] = 0.03;
        let exact = free_state(
            0.001,
            0.03,
            0.1,
            p.assembly.hammer_mass_kg,
            p.action.hammer_return_n_m,
            p.action.hammer_return_n_s_m,
        );
        let mut b = a;
        b.mechanical.position[18] = p.action.hammer_rest_m + exact[0];
        b.mechanical.velocity[18] = exact[1];
        let mut trace = ReturnTrace {
            free_start: Some((0.0, a)),
            ..Default::default()
        };
        trace.finish_flight(p, b, 0.1);
        assert!(trace.max_free_error < 1e-12);
        trace.free_start = Some((0.0, a));
        b.mechanical.velocity[18] += 0.001;
        trace.finish_flight(p, b, 0.1);
        assert!(trace.max_free_error > 0.01);
    }
}
