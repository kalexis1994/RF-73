//! Hammer landing on the returned pedestal: reseating, restitution, bounces
//! and settling under pedestal contact loss and return damping controls.
use super::key::REPEAT;
use super::{bridle, flight, key, launch, letoff, repetition, tuning};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 6] = [
    "control",
    "gravity_4g",
    "gravity_pedestal_rate_loss_10",
    "gravity_pedestal_rate_loss_30",
    "gravity_return_damping_0_1",
    "gravity_pedestal_loss_30_return_damping_0_1",
];
const EVENT_CAP: usize = 32;

// Without weight nothing returns a bounced hammer to the pedestal within the
// repetition wait, so every landing intervention is applied on the weighted
// 4 g hammer; the gravity-free control anchors the replay chain.
fn profile(position: f64, index: usize) -> ElectromechanicalProfile {
    let mut p = repetition::profile(position, 0);
    if index == 0 {
        return p;
    }
    p.action.gravity_m_s2 = 9.81;
    match index {
        2 => p.action.pedestal_rate_loss_s_m = 10.0,
        3 => p.action.pedestal_rate_loss_s_m = 30.0,
        4 => p.action.hammer_return_n_s_m = 0.1,
        5 => {
            p.action.pedestal_rate_loss_s_m = 30.0;
            p.action.hammer_return_n_s_m = 0.1;
        }
        _ => {}
    }
    p
}
struct Window {
    key_up: f64,
    end: f64,
    events: Vec<Value>,
    overflow: bool,
    first_reseat: Option<(f64, f64)>,
    rebound: f64,
    rebound_window_end: f64,
    last_entry: Option<f64>,
    exits_after_reseat: u32,
    contact_at_end: bool,
    lowest: f64,
}
impl Window {
    fn new(key_up: f64, end: f64) -> Self {
        Self {
            key_up,
            end,
            events: Vec::new(),
            overflow: false,
            first_reseat: None,
            rebound: 0.0,
            rebound_window_end: 0.0,
            last_entry: None,
            exits_after_reseat: 0,
            contact_at_end: false,
            lowest: f64::INFINITY,
        }
    }
    fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        let now = t + h;
        if now <= self.key_up || now > self.end + 0.5 * h {
            return;
        }
        self.lowest = self.lowest.min(b.mechanical.position[18]);
        let was = a.mechanical.contact_force_n[2] > 0.0;
        let is = b.mechanical.contact_force_n[2] > 0.0;
        if was != is {
            if self.events.len() < EVENT_CAP {
                self.events.push(json!({"seconds":now,"kind":if is {"entry"} else {"exit"},
                    "hammer_velocity_before_m_s":a.mechanical.velocity[18],"hammer_velocity_after_m_s":b.mechanical.velocity[18],
                    "hammer_position_m":b.mechanical.position[18],"pedestal_position_m":b.mechanical.pedestal_position_m}));
            } else {
                self.overflow = true;
            }
            if is {
                self.last_entry = Some(now);
                if self.first_reseat.is_none()
                    && b.mechanical.pedestal_position_m == p.action.hammer_rest_m
                {
                    self.first_reseat = Some((now - self.key_up, -a.mechanical.velocity[18]));
                    self.rebound_window_end = now + 0.02;
                }
            } else if self.first_reseat.is_some() {
                self.exits_after_reseat += 1;
            }
        }
        if self.first_reseat.is_some() && now <= self.rebound_window_end {
            self.rebound = self.rebound.max(b.mechanical.velocity[18]);
        }
        self.contact_at_end = is;
    }
    fn report(&self) -> Value {
        let settled = self
            .contact_at_end
            .then_some(self.last_entry)
            .flatten()
            .map(|t| t - self.key_up);
        json!({"key_up_seconds":self.key_up,"end_seconds":self.end,"events":self.events,"overflow":self.overflow,
            "first_reseat_seconds_after_key_up":self.first_reseat.map(|r|r.0),"landing_speed_m_s":self.first_reseat.map(|r|r.1),
            "rebound_speed_m_s":self.first_reseat.map(|_|self.rebound),
            "restitution":self.first_reseat.map(|r| if r.1 > 0.0 { self.rebound / r.1 } else { 0.0 }),
            "pedestal_exits_after_reseat":self.exits_after_reseat,"settled_seconds_after_key_up":settled,
            "lowest_hammer_position_m":self.lowest})
    }
}
struct Observer {
    p: ElectromechanicalProfile,
    windows: [Window; 2],
}
impl Observer {
    fn new(p: ElectromechanicalProfile) -> Self {
        let repeat = REPEAT as f64 / 48000.0;
        Self {
            p,
            windows: [Window::new(0.15, repeat), Window::new(0.33, 0.4)],
        }
    }
    fn report(&self) -> Value {
        let passed = self.windows.iter().all(|w| !w.overflow);
        json!({"passed":passed,"windows":self.windows.iter().map(|w| w.report()).collect::<Vec<_>>()})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut observer = Observer::new(p);
    let mut result = flight::take_observed(p, steps, speed, |_, a, b, t, h| {
        for w in &mut observer.windows {
            w.observe(observer.p, a, b, t, h);
        }
    })?;
    let landing = observer.report();
    result.report["passed"] = json!(result.report["passed"] == true && landing["passed"] == true);
    result.report["landing"] = landing;
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
pub(super) fn landing_convergence(a: &Value, b: &Value) -> Value {
    let mut rows = Vec::new();
    for (i, (wa, wb)) in a["landing"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["landing"]["windows"].as_array().unwrap())
        .enumerate()
    {
        let reseat = optional_error(
            &wa["first_reseat_seconds_after_key_up"],
            &wb["first_reseat_seconds_after_key_up"],
            false,
            0.0,
        );
        let landing = optional_error(
            &wa["landing_speed_m_s"],
            &wb["landing_speed_m_s"],
            true,
            0.01,
        );
        let rebound = optional_error(
            &wa["rebound_speed_m_s"],
            &wb["rebound_speed_m_s"],
            true,
            0.01,
        );
        let settled = optional_error(
            &wa["settled_seconds_after_key_up"],
            &wb["settled_seconds_after_key_up"],
            false,
            0.0,
        );
        let lowest = (wa["lowest_hammer_position_m"].as_f64().unwrap()
            - wb["lowest_hammer_position_m"].as_f64().unwrap())
        .abs();
        let exits = wa["pedestal_exits_after_reseat"] == wb["pedestal_exits_after_reseat"];
        // The first landing is gated in full; the truncated second window only
        // needs agreeing reseat and landing speed.
        let passed = reseat.is_some_and(|e| e < 0.0001)
            && landing.is_some_and(|e| e < 0.01)
            && (i == 1
                || (rebound.is_some_and(|e| e < 0.01)
                    && settled.is_some_and(|e| e < 0.001)
                    && lowest < 1e-5
                    && exits));
        rows.push(json!({"window":i+1,"passed":passed,"reseat_time_error_seconds":reseat,"relative_landing_speed_error":landing,
            "relative_rebound_speed_error":rebound,"settled_time_error_seconds":settled,"lowest_position_error_m":lowest,
            "exit_counts_agree":exits}));
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"windows":rows})
}
fn flight_qualification(f: &Value) -> Value {
    let launches: Vec<Value> = f["launches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let end = &l["release_to_window_end"];
            let passed = l["struck_agrees"] == true
                && l["release_time_error_seconds"]
                    .as_f64()
                    .is_some_and(|e| e < 0.0001)
                && l["relative_release_speed_error"]
                    .as_f64()
                    .is_some_and(|e| e < 0.01)
                && l["release_to_impact"]["passed"] == true
                && end["present"] == true
                && end["time_error_seconds"]
                    .as_f64()
                    .is_some_and(|e| e < 0.0001)
                && end["relative_arrival_speed_error"]
                    .as_f64()
                    .is_some_and(|e| e < 0.01)
                && end["max_relative_term_error"]
                    .as_f64()
                    .is_some_and(|e| e < 0.01);
            json!({"launch":l["launch"],"passed":passed,
                "window_end_coupling_error_retained":end["max_relative_coupling_error"]})
        })
        .collect();
    json!({"passed":launches.len()==2 && launches.iter().all(|l|l["passed"]==true),"launches":launches})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-landing --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("landing study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (index, name) in NAMES.iter().enumerate() {
            let p = profile(position, index);
            let mut rows = Vec::new();
            for (row, speed) in [1.125, 1.5].iter().enumerate() {
                println!("Loaded landing: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let c = repetition::convergence(&a, &b, REPEAT);
                let l = launch::convergence(&a.report, &b.report);
                let k = key::key_convergence(&a.report, &b.report);
                let e = letoff::letoff_convergence(&a.report, &b.report);
                let f = flight::flight_convergence(&a.report, &b.report);
                let fq = flight_qualification(&f);
                let g = landing_convergence(&a.report, &b.report);
                let qualified = a.report["passed"] == true
                    && b.report["passed"] == true
                    && [&c, &l, &k, &e, &fq, &g]
                        .iter()
                        .all(|x| x["passed"] == true);
                let first = key::compare_first(
                    &b.report,
                    if index == 0 {
                        &b.report
                    } else {
                        &cases[0]["rows"][row]["takes"][1]
                    },
                );
                println!(
                    "Loaded landing: qualified={qualified}, repeatable={}, ready={}",
                    b.report["repetition"]["two_clean_repeatable_strikes"],
                    b.report["repetition"]["readiness"]["passed"]
                );
                rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_control":first,
                    "takes":[a.report,b.report],"convergence":c,"launch_convergence":l,"key_convergence":k,
                    "letoff_convergence":e,"flight_convergence":f,"flight_qualification":fq,"landing_convergence":g}));
            }
            cases.push(json!({"name":name,"settings":{"pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m,
                "pedestal_stiffness_n_m2":p.action.pedestal_stiffness_n_m2,"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "return_stiffness_n_m":p.action.hammer_return_n_m,"hammer_mass_kg":p.assembly.hammer_mass_kg,
                "gravity_m_s2":p.action.gravity_m_s2},"rows":rows}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && cases.len() == NAMES.len()
        && cases.iter().all(|c| {
            c["rows"].as_array().is_some_and(|r| {
                r.len() == 2 && r.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-landing-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix on the original profile with the 4 g hammer, 0.05 kg key and sharp let-off: the retained gravity-free control; the weighted (9.81 m/s^2) hammer and arm; and, on the weighted hammer, pedestal rate loss 10 and 30 s/m instead of 2, hammer return damping 0.1 instead of 0.025 Ns/m, and rate loss 30 with damping 0.1 together. Nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. For each key-up window (150-210 ms and 330-400 ms) a landing observer retains pedestal contact entry/exit events with hammer velocities before and after (capped at 32, overflow fails qualification), the first reseat on the pedestal returned to rest with the landing speed, the maximum upward hammer speed within 20 ms of reseating as rebound speed and their ratio as restitution, the number of pedestal exits after reseating, the settling time as the last pedestal entry whose contact persists to the window end, and the lowest hammer position. The first window must refine within 0.1 ms in reseat time, 1% in landing and rebound speeds (0.01 m/s floor), 1 ms in settling time, 10 um in lowest position and with equal exit counts; the truncated second window needs agreeing reseat time and landing speed. All repetition, launch, key and let-off gates are retained; flight identities, release and strike agreement, release-to-impact refinement and window-end hammer terms are gated, window-end coupling terms are retained but not gated. First-strike comparison against the control uses the prior 5% impact/speed and 1 ms latency limits; readiness and repeatability keep their retained limits. No candidate selection.",
        "scope":"One-at-a-time and one combined intervention on provisional contact-loss and damping coefficients, not a measured pedestal felt or hammer tip. Interventions ride on the weighted hammer because a gravity-free hammer cannot return to the pedestal within the repetition wait once it bounces; the gravity-free control anchors the replay chain. Only the original profile, one key, one let-off and the 60 ms repetition wait are covered; the second landing window is truncated at 70 ms. No source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("landing study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn landing_profiles_change_only_the_declared_coefficients() {
        let base = profile(0.8, 0);
        assert_eq!(base.action.pedestal_rate_loss_s_m, 2.0);
        assert_eq!(base.action.hammer_return_n_s_m, 0.025);
        assert_eq!(base.action.gravity_m_s2, 0.0);
        assert_eq!(profile(0.8, 1).action.pedestal_rate_loss_s_m, 2.0);
        assert_eq!(profile(0.8, 2).action.pedestal_rate_loss_s_m, 10.0);
        assert_eq!(profile(0.8, 3).action.pedestal_rate_loss_s_m, 30.0);
        assert_eq!(profile(0.8, 4).action.hammer_return_n_s_m, 0.1);
        let both = profile(0.8, 5);
        assert_eq!(both.action.pedestal_rate_loss_s_m, 30.0);
        assert_eq!(both.action.hammer_return_n_s_m, 0.1);
        for index in 1..6 {
            let p = profile(0.8, index);
            assert_eq!(p.assembly.hammer_mass_kg, base.assembly.hammer_mass_kg);
            assert_eq!(p.action.gravity_m_s2, 9.81);
            assert_eq!(p.action.hammer_return_n_m, base.action.hammer_return_n_m);
        }
    }
    #[test]
    fn landing_window_reports_reseat_rebound_and_settling_from_a_synthetic_bounce() {
        let p = profile(0.8, 0);
        let h = 1e-4;
        let mut w = Window::new(0.15, 0.21);
        let model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
        let mut a = model.probe();
        a.mechanical.contact_force_n[2] = 0.0;
        a.mechanical.pedestal_position_m = p.action.hammer_rest_m;
        // Falling at 0.6 m/s, contact at 0.18 s, one bounce to 0.3 m/s, exit,
        // re-entry at 0.19 s and continuous contact afterwards.
        let mut t: f64 = 0.15;
        let mut in_contact = false;
        while t < 0.21 {
            let mut b = a;
            let entry_1 = (t - 0.18).abs() < 0.5 * h;
            let exit_1 = (t - 0.1801).abs() < 0.5 * h;
            let entry_2 = (t - 0.19).abs() < 0.5 * h;
            if entry_1 || entry_2 {
                in_contact = true;
            }
            if exit_1 {
                in_contact = false;
            }
            b.mechanical.contact_force_n[2] = if in_contact { 1.0 } else { 0.0 };
            a.mechanical.velocity[18] = if t <= 0.18 + 0.5 * h { -0.6 } else { 0.0 };
            b.mechanical.velocity[18] = if entry_1 {
                0.3
            } else if t < 0.18 {
                -0.6
            } else {
                0.0
            };
            w.observe(p, a, b, t - h, h);
            a = b;
            t += h;
        }
        let r = w.report();
        assert!((r["first_reseat_seconds_after_key_up"].as_f64().unwrap() - 0.03).abs() < h);
        assert!((r["landing_speed_m_s"].as_f64().unwrap() - 0.6).abs() < 1e-12);
        assert!((r["rebound_speed_m_s"].as_f64().unwrap() - 0.3).abs() < 1e-12);
        assert!((r["restitution"].as_f64().unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(r["pedestal_exits_after_reseat"], 1);
        assert!((r["settled_seconds_after_key_up"].as_f64().unwrap() - 0.04).abs() < h);
        assert_eq!(r["events"].as_array().unwrap().len(), 3);
        assert_eq!(r["overflow"], false);
    }
}
