//! Damper felt re-seating after key-up: felt contact events, settling,
//! contact fraction and damping onset under felt loss, arm damping and arm
//! mass controls, with the hammer already settled by pedestal loss.
use super::key::REPEAT;
use super::{bridle, flight, key, launch, letoff, repetition, tuning};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 6] = [
    "control",
    "settled_hammer",
    "felt_rate_loss_15",
    "felt_rate_loss_40",
    "arm_damping_2",
    "arm_mass_2g",
];
const EVENT_CAP: usize = 32;
const BIN: f64 = 0.001;

// The settled base is the weighted 4 g hammer with pedestal rate loss 30 s/m
// from the landing study; every felt intervention rides on it.
fn profile(position: f64, index: usize) -> ElectromechanicalProfile {
    let mut p = repetition::profile(position, 0);
    if index == 0 {
        return p;
    }
    p.action.gravity_m_s2 = 9.81;
    p.action.pedestal_rate_loss_s_m = 30.0;
    match index {
        2 => p.felt.felt_rate_loss_s_m = 15.0,
        3 => p.felt.felt_rate_loss_s_m = 40.0,
        4 => p.felt.arm_damping_n_s_m = 2.0,
        5 => p.felt.arm_mass_kg = 0.002,
        _ => {}
    }
    p
}
struct Window {
    key_up: f64,
    end: f64,
    events: Vec<Value>,
    overflow: bool,
    first_reseat: Option<f64>,
    last_entry: Option<f64>,
    exits_after_reseat: u32,
    contact_at_end: bool,
    contact_seconds: f64,
    seconds: f64,
    reference_square: f64,
    reference_ticks: u64,
    bin_square: f64,
    bin_ticks: u64,
    bin_index: u64,
    bins_db: Vec<f64>,
    onset_20: Option<f64>,
    onset_40: Option<f64>,
    max_felt_lift: f64,
}
impl Window {
    fn new(key_up: f64, end: f64) -> Self {
        Self {
            key_up,
            end,
            events: Vec::new(),
            overflow: false,
            first_reseat: None,
            last_entry: None,
            exits_after_reseat: 0,
            contact_at_end: false,
            contact_seconds: 0.0,
            seconds: 0.0,
            reference_square: 0.0,
            reference_ticks: 0,
            bin_square: 0.0,
            bin_ticks: 0,
            bin_index: 0,
            bins_db: Vec::new(),
            onset_20: None,
            onset_40: None,
            max_felt_lift: 0.0,
        }
    }
    fn close_bin(&mut self) {
        if self.bin_ticks == 0 {
            return;
        }
        let reference = (self.reference_square / self.reference_ticks.max(1) as f64).max(1e-30);
        let rms = self.bin_square / self.bin_ticks as f64;
        let db = 10.0 * (rms / reference).log10();
        self.bin_index += 1;
        let t = self.bin_index as f64 * BIN;
        if self.bins_db.len() < 128 {
            self.bins_db.push(db);
        }
        if self.onset_20.is_none() && db <= -20.0 {
            self.onset_20 = Some(t);
        }
        if self.onset_40.is_none() && db <= -40.0 {
            self.onset_40 = Some(t);
        }
        self.bin_square = 0.0;
        self.bin_ticks = 0;
    }
    fn observe(&mut self, a: ElectromechanicalProbe, b: ElectromechanicalProbe, t: f64, h: f64) {
        let now = t + h;
        // Reference level: the millisecond before key-up.
        if now > self.key_up - BIN && now <= self.key_up {
            self.reference_square += b.output_voltage_v * b.output_voltage_v;
            self.reference_ticks += 1;
            return;
        }
        if now <= self.key_up || now > self.end + 0.5 * h {
            return;
        }
        let elapsed = now - self.key_up;
        if elapsed > (self.bin_index + 1) as f64 * BIN + 0.5 * h {
            self.close_bin();
        }
        self.bin_square += b.output_voltage_v * b.output_voltage_v;
        self.bin_ticks += 1;
        let was = a.mechanical.contact_force_n[1] > 0.0;
        let is = b.mechanical.contact_force_n[1] > 0.0;
        self.seconds += h;
        if is {
            self.contact_seconds += h;
        }
        self.max_felt_lift = self.max_felt_lift.max(-b.mechanical.compression_m[1]);
        if was != is {
            if self.events.len() < EVENT_CAP {
                self.events.push(json!({"seconds":now,"kind":if is {"entry"} else {"exit"},
                    "arm_velocity_before_m_s":a.mechanical.velocity[19],"arm_velocity_after_m_s":b.mechanical.velocity[19],
                    "arm_position_m":b.mechanical.position[19]}));
            } else {
                self.overflow = true;
            }
            if is {
                self.last_entry = Some(now);
                if self.first_reseat.is_none() {
                    self.first_reseat = Some(elapsed);
                }
            } else if self.first_reseat.is_some() {
                self.exits_after_reseat += 1;
            }
        }
        self.contact_at_end = is;
    }
    fn report(&mut self) -> Value {
        self.close_bin();
        let settled = self
            .contact_at_end
            .then_some(self.last_entry)
            .flatten()
            .map(|t| t - self.key_up);
        json!({"key_up_seconds":self.key_up,"end_seconds":self.end,"events":self.events,"overflow":self.overflow,
            "first_reseat_seconds_after_key_up":self.first_reseat,"felt_exits_after_reseat":self.exits_after_reseat,
            "settled_seconds_after_key_up":settled,"felt_contact_fraction":self.contact_seconds/self.seconds.max(1e-12),
            "max_felt_lift_m":self.max_felt_lift,"reference_voltage_rms_v":(self.reference_square/self.reference_ticks.max(1) as f64).sqrt(),
            "output_level_db_per_ms":self.bins_db,"onset_20db_seconds_after_key_up":self.onset_20,"onset_40db_seconds_after_key_up":self.onset_40})
    }
}
struct Observer {
    windows: [Window; 2],
}
impl Observer {
    fn new() -> Self {
        let repeat = REPEAT as f64 / 48000.0;
        Self {
            windows: [Window::new(0.15, repeat), Window::new(0.33, 0.4)],
        }
    }
    fn report(&mut self) -> Value {
        let windows: Vec<Value> = self.windows.iter_mut().map(|w| w.report()).collect();
        let passed = windows.iter().all(|w| w["overflow"] == false);
        json!({"passed":passed,"windows":windows})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    take_observed(p, steps, speed, |_, _, _, _, _| {})
}
pub(super) fn take_observed(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    mut observe: impl FnMut(
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        f64,
        f64,
    ),
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut observer = Observer::new();
    let mut result = flight::take_observed(p, steps, speed, |initial, a, b, t, h| {
        for w in &mut observer.windows {
            w.observe(a, b, t, h);
        }
        observe(initial, a, b, t, h);
    })?;
    let damper = observer.report();
    result.report["passed"] = json!(result.report["passed"] == true && damper["passed"] == true);
    result.report["damper"] = damper;
    Ok(result)
}
fn optional_time_error(a: &Value, b: &Value) -> Option<f64> {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        (None, None) => Some(0.0),
        _ => None,
    }
}
pub(super) fn damper_convergence(a: &Value, b: &Value) -> Value {
    let mut rows = Vec::new();
    for (i, (wa, wb)) in a["damper"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["damper"]["windows"].as_array().unwrap())
        .enumerate()
    {
        let reseat = optional_time_error(
            &wa["first_reseat_seconds_after_key_up"],
            &wb["first_reseat_seconds_after_key_up"],
        );
        let settled = optional_time_error(
            &wa["settled_seconds_after_key_up"],
            &wb["settled_seconds_after_key_up"],
        );
        let onset_20 = optional_time_error(
            &wa["onset_20db_seconds_after_key_up"],
            &wb["onset_20db_seconds_after_key_up"],
        );
        let onset_40 = optional_time_error(
            &wa["onset_40db_seconds_after_key_up"],
            &wb["onset_40db_seconds_after_key_up"],
        );
        let fraction = (wa["felt_contact_fraction"].as_f64().unwrap()
            - wb["felt_contact_fraction"].as_f64().unwrap())
        .abs();
        let exits = wa["felt_exits_after_reseat"] == wb["felt_exits_after_reseat"];
        // Onset bins are 1 ms wide, so a one-bin difference is allowed.
        let passed = reseat.is_some_and(|e| e < 0.0001)
            && settled.is_some_and(|e| e < 0.001)
            && onset_20.is_some_and(|e| e <= BIN + 1e-9)
            && onset_40.is_some_and(|e| e <= BIN + 1e-9)
            && fraction < 0.01
            && exits;
        rows.push(json!({"window":i+1,"passed":passed,"reseat_time_error_seconds":reseat,"settled_time_error_seconds":settled,
            "onset_20db_error_seconds":onset_20,"onset_40db_error_seconds":onset_40,"contact_fraction_error":fraction,
            "exit_counts_agree":exits}));
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"windows":rows})
}
pub(super) fn flight_qualification(f: &Value) -> Value {
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
        return Err("loaded-damper-seating --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("damper seating study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (index, name) in NAMES.iter().enumerate() {
            let p = profile(position, index);
            let mut rows = Vec::new();
            for (row, speed) in [1.125, 1.5].iter().enumerate() {
                println!("Loaded damper seating: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let c = repetition::convergence(&a, &b, REPEAT);
                let l = launch::convergence(&a.report, &b.report);
                let k = key::key_convergence(&a.report, &b.report);
                let e = letoff::letoff_convergence(&a.report, &b.report);
                let f = flight::flight_convergence(&a.report, &b.report);
                let fq = flight_qualification(&f);
                let d = damper_convergence(&a.report, &b.report);
                let qualified = a.report["passed"] == true
                    && b.report["passed"] == true
                    && [&c, &l, &k, &e, &fq, &d]
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
                let base = (index >= 2)
                    .then(|| key::compare_first(&b.report, &cases[1]["rows"][row]["takes"][1]));
                println!(
                    "Loaded damper seating: qualified={qualified}, repeatable={}, ready={}",
                    b.report["repetition"]["two_clean_repeatable_strikes"],
                    b.report["repetition"]["readiness"]["passed"]
                );
                rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_control":first,
                    "first_vs_settled_hammer":base,"takes":[a.report,b.report],"convergence":c,"launch_convergence":l,
                    "key_convergence":k,"letoff_convergence":e,"flight_convergence":f,"flight_qualification":fq,
                    "damper_convergence":d}));
            }
            cases.push(json!({"name":name,"settings":{"gravity_m_s2":p.action.gravity_m_s2,"pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m,
                "felt_rate_loss_s_m":p.felt.felt_rate_loss_s_m,"felt_stiffness_n_m2":p.felt.felt_stiffness_n_m2,
                "arm_damping_n_s_m":p.felt.arm_damping_n_s_m,"arm_mass_kg":p.felt.arm_mass_kg,"arm_stiffness_n_m":p.felt.arm_stiffness_n_m,
                "hammer_mass_kg":p.assembly.hammer_mass_kg},"rows":rows}));
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
    let report = json!({"schema_version":1,"experiment":"loaded-damper-seating-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix on the original profile with the 4 g hammer, 0.05 kg key and sharp let-off: the retained gravity-free control; the settled hammer (9.81 m/s^2 weight with pedestal rate loss 30 s/m); and, on the settled hammer, felt rate loss 15 and 40 s/m instead of 5, damper-arm damping 2 instead of 0.5 Ns/m, and damper-arm mass 0.002 instead of 0.001 kg. Nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. For each key-up window (150-210 ms and 330-400 ms) a damper observer retains felt/tine contact entry and exit events with arm velocities (capped at 32, overflow fails qualification), the first felt reseat after key-up, the felt exits after reseating, the settling time as the last felt entry whose contact persists to the window end, the felt contact fraction over the window, the maximum felt lift, and the raw output level in 1 ms bins relative to the millisecond before key-up, with the first bins at or below -20 dB and -40 dB retained as damping onsets. Refinement requires felt reseat times within 0.1 ms, settling times within 1 ms, onset times within one 1 ms bin, contact fractions within 0.01 and equal exit counts. All repetition, launch, key and let-off gates are retained; flight identities, release and strike agreement, release-to-impact refinement and window-end hammer terms are gated, window-end coupling terms are retained but not gated. First strikes are compared with the control and with the settled hammer using the prior 5% impact/speed and 1 ms latency limits; readiness keeps its retained limits. No candidate selection.",
        "scope":"One-at-a-time interventions on provisional felt loss, arm damping and arm mass, not a measured damper felt or arm. Damping onset is a raw output level measure in 1 ms bins, not a perceptual damping time. Only the original profile, one key, one let-off and the 60 ms repetition wait are covered; the second window is truncated at 70 ms. No source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("damper seating study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damper_profiles_ride_on_the_settled_hammer_and_change_one_coefficient() {
        let base = profile(0.8, 0);
        assert_eq!(base.action.gravity_m_s2, 0.0);
        assert_eq!(base.action.pedestal_rate_loss_s_m, 2.0);
        let settled = profile(0.8, 1);
        assert_eq!(settled.action.gravity_m_s2, 9.81);
        assert_eq!(settled.action.pedestal_rate_loss_s_m, 30.0);
        assert_eq!(settled.felt.felt_rate_loss_s_m, 5.0);
        assert_eq!(profile(0.8, 2).felt.felt_rate_loss_s_m, 15.0);
        assert_eq!(profile(0.8, 3).felt.felt_rate_loss_s_m, 40.0);
        assert_eq!(profile(0.8, 4).felt.arm_damping_n_s_m, 2.0);
        assert_eq!(profile(0.8, 5).felt.arm_mass_kg, 0.002);
        for index in 2..6 {
            let p = profile(0.8, index);
            assert_eq!(p.action.gravity_m_s2, 9.81);
            assert_eq!(p.action.pedestal_rate_loss_s_m, 30.0);
            assert_eq!(p.assembly.hammer_mass_kg, 0.004);
            assert_eq!(p.felt.felt_stiffness_n_m2, base.felt.felt_stiffness_n_m2);
        }
    }
    #[test]
    fn damper_window_reports_reseat_settling_fraction_and_onset_from_a_synthetic_decay() {
        let p = profile(0.8, 0);
        let h = 1e-4;
        let mut w = Window::new(0.15, 0.21);
        let model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
        let mut a = model.probe();
        a.mechanical.contact_force_n[1] = 0.0;
        // Reference level 1 V before key-up; output decays by 20 dB per 10 ms
        // after key-up. Felt contact begins at 20 ms, bounces once at 25 ms for
        // 1 ms, then stays seated.
        let mut t: f64 = 0.148;
        while t < 0.21 {
            let mut b = a;
            let elapsed = t - 0.15;
            b.output_voltage_v = if elapsed <= 0.0 {
                1.0
            } else {
                10f64.powf(-elapsed / 0.01)
            };
            let seated = elapsed >= 0.02 && !((0.025..0.026).contains(&elapsed));
            b.mechanical.contact_force_n[1] = if seated { 0.5 } else { 0.0 };
            b.mechanical.compression_m[1] = if seated { 1e-5 } else { -2e-4 };
            w.observe(a, b, t - h, h);
            a = b;
            t += h;
        }
        let r = w.report();
        assert!((r["first_reseat_seconds_after_key_up"].as_f64().unwrap() - 0.02).abs() < h);
        assert_eq!(r["felt_exits_after_reseat"], 1);
        assert!((r["settled_seconds_after_key_up"].as_f64().unwrap() - 0.026).abs() < h);
        let fraction = r["felt_contact_fraction"].as_f64().unwrap();
        assert!((fraction - 39.0 / 60.0).abs() < 0.01);
        assert!((r["max_felt_lift_m"].as_f64().unwrap() - 2e-4).abs() < 1e-12);
        assert!((r["reference_voltage_rms_v"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        // 20 dB in amplitude is 20 dB in power ratio of squared RMS: reached at 10 ms.
        let t20 = r["onset_20db_seconds_after_key_up"].as_f64().unwrap();
        let t40 = r["onset_40db_seconds_after_key_up"].as_f64().unwrap();
        assert!((t20 - 0.011).abs() < 1.5 * BIN && (t40 - 0.021).abs() < 1.5 * BIN);
        assert_eq!(r["output_level_db_per_ms"].as_array().unwrap().len(), 60);
        assert_eq!(r["overflow"], false);
    }
}
