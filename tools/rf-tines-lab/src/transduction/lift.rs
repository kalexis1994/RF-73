//! Damper lift geometry: bridle ratio and slack set how far the felt lifts,
//! and a softer arm spring with matched seating force sets how fast it
//! returns, on the settled hammer.
use super::key::REPEAT;
use super::{bridle, damper, key, launch, letoff, repetition, tuning};
use rf_tines_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 6] = [
    "control",
    "settled_hammer",
    "bridle_ratio_0_5",
    "bridle_slack_4mm",
    "soft_arm_matched_seating",
    "ratio_0_5_soft_arm",
];

// The soft arm halves the spring and moves its base so that the spring force
// at the settled rest arm position is unchanged; that position therefore stays
// an equilibrium and the rest felt force is matched exactly.
fn soften_arm(p: &mut ElectromechanicalProfile) {
    let rest = rf_tines_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, *p)
        .expect("settled rest")
        .probe();
    let z = rest.mechanical.position[19];
    let base = p.action.damper_closed_m;
    p.felt.arm_stiffness_n_m *= 0.5;
    p.action.damper_closed_m = z + 2.0 * (base - z);
}
fn profile(position: f64, index: usize) -> ElectromechanicalProfile {
    let mut p = repetition::profile(position, 0);
    if index == 0 {
        return p;
    }
    p.action.gravity_m_s2 = 9.81;
    p.action.pedestal_rate_loss_s_m = 30.0;
    match index {
        2 => p.action.bridle_ratio = 0.5,
        3 => p.action.bridle_slack_m = 0.004,
        4 => soften_arm(&mut p),
        5 => {
            p.action.bridle_ratio = 0.5;
            soften_arm(&mut p);
        }
        _ => {}
    }
    p
}
struct Observer {
    rest: Option<Value>,
    hold_max_lift: [f64; 2],
    hold_max_arm_speed: [f64; 2],
    hold_min_felt_force: [f64; 2],
}
impl Observer {
    fn new() -> Self {
        Self {
            rest: None,
            hold_max_lift: [0.0; 2],
            hold_max_arm_speed: [0.0; 2],
            hold_min_felt_force: [f64::INFINITY; 2],
        }
    }
    fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        initial: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        if self.rest.is_none() {
            self.rest = Some(json!({"felt_force_n":initial.mechanical.contact_force_n[1],
                "arm_offset_m":initial.mechanical.position[19]-p.action.damper_closed_m,
                "arm_spring_force_n":p.felt.arm_stiffness_n_m*(p.action.damper_closed_m-initial.mechanical.position[19]),
                "felt_compression_m":initial.mechanical.compression_m[1],
                "bridle_compression_m":initial.mechanical.compression_m[3]}));
        }
        let now = t + h;
        for (i, (start, end)) in [(0.03, 0.15), (0.21, 0.33)].iter().enumerate() {
            if now > *start && now <= *end {
                self.hold_max_lift[i] = self.hold_max_lift[i].max(-b.mechanical.compression_m[1]);
                self.hold_max_arm_speed[i] =
                    self.hold_max_arm_speed[i].max(b.mechanical.velocity[19].abs());
                if now > start + 0.05 && now <= start + 0.11 {
                    self.hold_min_felt_force[i] =
                        self.hold_min_felt_force[i].min(b.mechanical.contact_force_n[1]);
                }
            }
        }
    }
    fn report(&self) -> Value {
        json!({"rest":self.rest,"held_max_felt_lift_m":self.hold_max_lift,"held_max_arm_speed_m_s":self.hold_max_arm_speed,
            "held_min_felt_force_n":self.hold_min_felt_force})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut observer = Observer::new();
    let mut result = damper::take_observed(p, steps, speed, |initial, _, b, t, h| {
        observer.observe(p, initial, b, t, h);
    })?;
    result.report["lift"] = observer.report();
    Ok(result)
}
pub(super) fn lift_convergence(a: &Value, b: &Value) -> Value {
    let (la, lb) = (&a["lift"], &b["lift"]);
    let rest = ["felt_force_n", "arm_offset_m", "felt_compression_m"]
        .iter()
        .map(|k| (la["rest"][k].as_f64().unwrap() - lb["rest"][k].as_f64().unwrap()).abs())
        .fold(0.0_f64, f64::max);
    let mut lift = 0.0_f64;
    let mut speed = 0.0_f64;
    for i in 0..2 {
        let x = la["held_max_felt_lift_m"][i].as_f64().unwrap();
        let y = lb["held_max_felt_lift_m"][i].as_f64().unwrap();
        lift = lift.max((x - y).abs() / y.abs().max(1e-5));
        let x = la["held_max_arm_speed_m_s"][i].as_f64().unwrap();
        let y = lb["held_max_arm_speed_m_s"][i].as_f64().unwrap();
        speed = speed.max((x - y).abs() / y.abs().max(0.01));
    }
    json!({"passed":rest<1e-12 && lift<0.01 && speed<0.01,"max_rest_difference":rest,
        "max_relative_lift_error":lift,"max_relative_arm_speed_error":speed})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-damper-lift --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("damper lift study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (index, name) in NAMES.iter().enumerate() {
            let p = profile(position, index);
            let mut rows = Vec::new();
            for (row, speed) in [1.125, 1.5].iter().enumerate() {
                println!("Loaded damper lift: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let c = repetition::convergence(&a, &b, REPEAT);
                let l = launch::convergence(&a.report, &b.report);
                let k = key::key_convergence(&a.report, &b.report);
                let e = letoff::letoff_convergence(&a.report, &b.report);
                let f = super::flight::flight_convergence(&a.report, &b.report);
                let fq = damper::flight_qualification(&f);
                let d = damper::damper_convergence(&a.report, &b.report);
                let g = lift_convergence(&a.report, &b.report);
                let qualified = a.report["passed"] == true
                    && b.report["passed"] == true
                    && [&c, &l, &k, &e, &fq, &d, &g]
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
                    "Loaded damper lift: qualified={qualified}, repeatable={}, ready={}",
                    b.report["repetition"]["two_clean_repeatable_strikes"],
                    b.report["repetition"]["readiness"]["passed"]
                );
                rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_control":first,
                    "first_vs_settled_hammer":base,"takes":[a.report,b.report],"convergence":c,"launch_convergence":l,
                    "key_convergence":k,"letoff_convergence":e,"flight_convergence":f,"flight_qualification":fq,
                    "damper_convergence":d,"lift_convergence":g}));
            }
            cases.push(json!({"name":name,"settings":{"gravity_m_s2":p.action.gravity_m_s2,"pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m,
                "bridle_ratio":p.action.bridle_ratio,"bridle_slack_m":p.action.bridle_slack_m,"arm_stiffness_n_m":p.felt.arm_stiffness_n_m,
                "damper_closed_m":p.action.damper_closed_m,"arm_damping_n_s_m":p.felt.arm_damping_n_s_m,"arm_mass_kg":p.felt.arm_mass_kg,
                "felt_rate_loss_s_m":p.felt.felt_rate_loss_s_m,"hammer_mass_kg":p.assembly.hammer_mass_kg},"rows":rows}));
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
    let report = json!({"schema_version":1,"experiment":"loaded-damper-lift-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix on the original profile with the 4 g hammer, 0.05 kg key and sharp let-off: the retained gravity-free control; the settled hammer (9.81 m/s^2 weight with pedestal rate loss 30 s/m); and, on the settled hammer, bridle ratio 0.5 instead of 0.8, bridle slack 4 mm instead of 2, a damper-arm spring of 100 instead of 200 N/m with its base moved so the spring force at the settled rest arm position is unchanged, which keeps that rest and its felt force exactly, and ratio 0.5 with the soft arm together. Nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. The retained damper observer is kept in full (felt contact events, reseat, bounces, settling, contact fraction, lift after key-up and raw-output damping onset in 1 ms bins). A lift observer retains the rest felt force, arm offset, arm spring force and felt and bridle compressions, the maximum felt lift and maximum arm speed during each held gesture, and the minimum felt force during 50-110 ms after each key-down. Refinement requires rest values within 1e-12, held lifts within 1% (10 um floor) and held arm speeds within 1% (0.01 m/s floor), together with the retained damper, repetition, launch, key and let-off gates; flight identities, release and strike agreement, release-to-impact refinement and window-end hammer terms are gated, window-end coupling terms are retained but not gated. First strikes are compared with the control and with the settled hammer using the prior 5% impact/speed and 1 ms latency limits; readiness keeps its retained limits. No candidate selection.",
        "scope":"Bridle ratio, slack and arm spring are provisional reductions, not a measured damper linkage. The matched seating keeps the settled rest arm position and felt force; the retained rest values are the evidence. Damping onset is a raw output level measure in 1 ms bins, not a perceptual damping time. Only the original profile, one key, one let-off and the 60 ms repetition wait are covered. No source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("damper lift study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lift_profiles_change_only_the_declared_geometry() {
        let base = profile(0.8, 1);
        assert_eq!(base.action.bridle_ratio, 0.8);
        assert_eq!(base.action.bridle_slack_m, 0.002);
        assert_eq!(base.felt.arm_stiffness_n_m, 200.0);
        assert_eq!(profile(0.8, 2).action.bridle_ratio, 0.5);
        assert_eq!(profile(0.8, 3).action.bridle_slack_m, 0.004);
        let soft = profile(0.8, 4);
        assert_eq!(soft.felt.arm_stiffness_n_m, 100.0);
        assert!(soft.action.damper_closed_m > 0.0002 && soft.action.damper_closed_m < 0.0005);
        let both = profile(0.8, 5);
        assert_eq!(both.action.bridle_ratio, 0.5);
        assert_eq!(both.felt.arm_stiffness_n_m, 100.0);
        for index in 2..6 {
            let p = profile(0.8, index);
            assert_eq!(p.action.gravity_m_s2, 9.81);
            assert_eq!(p.action.pedestal_rate_loss_s_m, 30.0);
            assert_eq!(p.felt.felt_rate_loss_s_m, 5.0);
        }
    }
    #[test]
    fn soft_arm_matches_the_settled_rest_felt_force_to_first_order() {
        let (position, _) = tuning::fitted_position(196.38614697959488).unwrap();
        let settled =
            rf_tines_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, profile(position, 1))
                .unwrap()
                .probe();
        let soft = rf_tines_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, profile(position, 4))
            .unwrap()
            .probe();
        let a = settled.mechanical.contact_force_n[1];
        let b = soft.mechanical.contact_force_n[1];
        assert!(a > 0.0 && b > 0.0);
        assert!((b / a - 1.0).abs() < 1e-6, "felt force {a} vs {b}");
    }
}
