//! Gravitational weight on the hammer and damper arm under the sharp let-off,
//! with hammer mass, arm seating and return-law controls.
use super::key::REPEAT;
use super::{bridle, flight, key, launch, letoff, repetition, tuning};
use rf_tines_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const GRAVITY: f64 = 9.81;
const NAMES: [&str; 6] = [
    "control",
    "gravity_4g",
    "gravity_8g",
    "gravity_12g",
    "gravity_12g_arm_reseated",
    "gravity_12g_weight_return",
];

fn profile(position: f64, index: usize) -> ElectromechanicalProfile {
    let mut p = repetition::profile(position, 0);
    if index == 0 {
        return p;
    }
    p.action.gravity_m_s2 = GRAVITY;
    p.assembly.hammer_mass_kg = match index {
        1 => 0.004,
        2 => 0.008,
        _ => 0.012,
    };
    if index == 4 {
        // Raise the arm spring base by the arm's static sag so the felt seats
        // with the gravity-free force; the arm dynamics are then unchanged.
        p.action.damper_closed_m += p.felt.arm_mass_kg * GRAVITY / p.felt.arm_stiffness_n_m;
    }
    if index == 5 {
        // Weight-dominated return: half the retained spring, so the weight is
        // 85% of the return force at full lift. Smaller stiffness makes the
        // unloaded sag so large that the rest solve loses its 1e-12 contact
        // tolerance to cancellation.
        p.action.hammer_return_n_m = 2.0;
    }
    p
}
struct Rest {
    hammer_sag: f64,
    pedestal_force: f64,
    arm_offset: f64,
    felt_force: f64,
}
struct Return {
    key_up: f64,
    reseated: Option<f64>,
    lowest: f64,
    at_repeat: Option<(f64, f64, f64)>,
}
struct Observer {
    p: ElectromechanicalProfile,
    rest: Option<Rest>,
    returns: [Return; 2],
    arm_min_force: f64,
}
impl Observer {
    fn new(p: ElectromechanicalProfile) -> Self {
        Self {
            p,
            rest: None,
            returns: [
                Return {
                    key_up: 0.15,
                    reseated: None,
                    lowest: f64::INFINITY,
                    at_repeat: None,
                },
                Return {
                    key_up: 0.33,
                    reseated: None,
                    lowest: f64::INFINITY,
                    at_repeat: None,
                },
            ],
            arm_min_force: f64::INFINITY,
        }
    }
    fn observe(
        &mut self,
        initial: ElectromechanicalProbe,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        if self.rest.is_none() {
            self.rest = Some(Rest {
                hammer_sag: initial.mechanical.position[18] - self.p.action.hammer_rest_m,
                pedestal_force: initial.mechanical.contact_force_n[2],
                arm_offset: initial.mechanical.position[19] - self.p.action.damper_closed_m,
                felt_force: initial.mechanical.contact_force_n[1],
            });
        }
        // Felt seating force while the key is up and the arm should rest.
        if !(0.03..0.15).contains(&t) && !(0.21..0.33).contains(&t) && t > 0.001 {
            self.arm_min_force = self.arm_min_force.min(b.mechanical.contact_force_n[1]);
        }
        let repeat = REPEAT as f64 / 48000.0;
        for (i, r) in self.returns.iter_mut().enumerate() {
            if t + h <= r.key_up {
                continue;
            }
            r.lowest = r.lowest.min(b.mechanical.position[18]);
            // Reseated: pedestal contact regained with the pedestal back at rest.
            if r.reseated.is_none()
                && b.mechanical.pedestal_position_m == self.p.action.hammer_rest_m
                && a.mechanical.contact_force_n[2] == 0.0
                && b.mechanical.contact_force_n[2] > 0.0
            {
                r.reseated = Some(t + h - r.key_up);
            }
            if i == 0 && r.at_repeat.is_none() && t + h >= repeat - 0.5 * h {
                r.at_repeat = Some((
                    b.mechanical.position[18],
                    b.mechanical.velocity[18],
                    b.mechanical.contact_force_n[2],
                ));
            }
        }
    }
    fn report(&self) -> Value {
        let rest = self.rest.as_ref().unwrap();
        let weight = self.p.assembly.hammer_mass_kg * self.p.action.gravity_m_s2;
        json!({"rest":{"hammer_sag_m":rest.hammer_sag,"pedestal_force_n":rest.pedestal_force,"hammer_weight_n":weight,
                "arm_offset_m":rest.arm_offset,"felt_force_n":rest.felt_force,
                "arm_weight_n":self.p.felt.arm_mass_kg*self.p.action.gravity_m_s2},
            "minimum_felt_force_key_up_n":self.arm_min_force,
            "returns":self.returns.iter().map(|r| json!({"key_up_seconds":r.key_up,"reseated_seconds_after_key_up":r.reseated,
                "lowest_hammer_position_m":r.lowest,
                "at_repeat":r.at_repeat.map(|(q,v,f)| json!({"hammer_position_m":q,"hammer_velocity_m_s":v,"pedestal_force_n":f}))})).collect::<Vec<_>>()})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut observer = Observer::new(p);
    let mut result = flight::take_observed(p, steps, speed, |initial, a, b, t, h| {
        observer.observe(initial, a, b, t, h);
    })?;
    result.report["gravity"] = observer.report();
    Ok(result)
}
fn optional_time_error(a: &Value, b: &Value) -> Option<f64> {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        (None, None) => Some(0.0),
        _ => None,
    }
}
// Flight qualification for this study: release, strike and release-to-impact
// refinement are gated, together with the window-end hammer terms. Window-end
// coupling terms after a chattering re-landing are retained but not gated.
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
pub(super) fn gravity_convergence(a: &Value, b: &Value) -> Value {
    let (ga, gb) = (&a["gravity"], &b["gravity"]);
    let rest = [
        "hammer_sag_m",
        "pedestal_force_n",
        "arm_offset_m",
        "felt_force_n",
    ]
    .iter()
    .map(|k| (ga["rest"][k].as_f64().unwrap() - gb["rest"][k].as_f64().unwrap()).abs())
    .fold(0.0_f64, f64::max);
    let felt = (ga["minimum_felt_force_key_up_n"].as_f64().unwrap()
        - gb["minimum_felt_force_key_up_n"].as_f64().unwrap())
    .abs()
        / gb["minimum_felt_force_key_up_n"]
            .as_f64()
            .unwrap()
            .abs()
            .max(1e-3);
    let mut returns = Vec::new();
    for (ra, rb) in ga["returns"]
        .as_array()
        .unwrap()
        .iter()
        .zip(gb["returns"].as_array().unwrap())
    {
        let reseat = optional_time_error(
            &ra["reseated_seconds_after_key_up"],
            &rb["reseated_seconds_after_key_up"],
        );
        let lowest = (ra["lowest_hammer_position_m"].as_f64().unwrap()
            - rb["lowest_hammer_position_m"].as_f64().unwrap())
        .abs();
        returns.push(json!({"passed":reseat.is_some_and(|e|e<0.001) && lowest<1e-5,"reseat_time_error_seconds":reseat,
            "lowest_position_error_m":lowest}));
    }
    json!({"passed":rest<1e-12 && felt<0.01 && returns.iter().all(|r|r["passed"]==true),
        "max_rest_difference":rest,"relative_minimum_felt_force_error":felt,"returns":returns})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-gravity --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("gravity study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (index, name) in NAMES.iter().enumerate() {
            let p = profile(position, index);
            let mut rows = Vec::new();
            for (row, speed) in [1.125, 1.5].iter().enumerate() {
                println!("Loaded gravity: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let c = repetition::convergence(&a, &b, REPEAT);
                let l = launch::convergence(&a.report, &b.report);
                let k = key::key_convergence(&a.report, &b.report);
                let e = letoff::letoff_convergence(&a.report, &b.report);
                let f = flight::flight_convergence(&a.report, &b.report);
                let fq = flight_qualification(&f);
                let g = gravity_convergence(&a.report, &b.report);
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
                let same_mass = (index >= 4)
                    .then(|| key::compare_first(&b.report, &cases[3]["rows"][row]["takes"][1]));
                println!(
                    "Loaded gravity: qualified={qualified}, repeatable={}",
                    b.report["repetition"]["two_clean_repeatable_strikes"]
                );
                rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_control":first,
                    "first_vs_gravity_12g":same_mass,"takes":[a.report,b.report],"convergence":c,"launch_convergence":l,
                    "key_convergence":k,"letoff_convergence":e,"flight_convergence":f,"flight_qualification":fq,"gravity_convergence":g}));
            }
            cases.push(json!({"name":name,"settings":{"gravity_m_s2":p.action.gravity_m_s2,"hammer_mass_kg":p.assembly.hammer_mass_kg,
                "hammer_weight_n":p.assembly.hammer_mass_kg*p.action.gravity_m_s2,"return_stiffness_n_m":p.action.hammer_return_n_m,
                "return_damping_n_s_m":p.action.hammer_return_n_s_m,"arm_mass_kg":p.felt.arm_mass_kg,"arm_stiffness_n_m":p.felt.arm_stiffness_n_m,
                "damper_closed_m":p.action.damper_closed_m},"rows":rows}));
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
    let report = json!({"schema_version":1,"experiment":"loaded-gravity-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix on the original profile under the retained sharp let-off with the 0.05 kg key: gravity-free control; 9.81 m/s^2 on hammer and damper arm with hammer mass 0.004, 0.008 and 0.012 kg; 0.012 kg with the arm spring base raised by the arm's static sag (arm mass times gravity over arm stiffness) so the felt seats with the gravity-free force; and 0.012 kg with the return spring halved to 2 N/m so the return is weight-dominated. Weight is a constant force along each coordinate whose potential is part of the assembly's mechanical energy, of the laboratory hammer, launch, flight and arm ledgers, and of the rest preparation. Nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. The rest state retains hammer sag, pedestal force, arm offset and felt force; the minimum felt force while the key is up, the hammer reseating time on the returned pedestal, the lowest hammer position after each release and the hammer state at the repeat command are retained. Rest values must agree between resolutions within 1e-12, minimum felt force within 1%, reseating times within 1 ms and lowest positions within 10 um. All repetition, launch, key and let-off gates are retained. Flight identities with the gravitational potential, release and strike agreement, release-to-impact refinement and window-end hammer terms are gated; window-end coupling terms after a chattering re-landing are retained but not gated. First-strike comparison against the control and, for the two 12 g variants, against the plain 12 g case uses the prior 5% impact/speed and 1 ms latency limits. No candidate selection.",
        "scope":"Constant weight along the reduced coordinates, not a measured pivot geometry or lever ratio; the arm reseating shifts the spring base exactly and leaves the arm dynamics unchanged by construction. Hammer masses remain provisional. Only the original profile, one key, one let-off and the 60 ms repetition wait are covered. No source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("gravity study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gravity_profiles_set_weight_and_controls_as_declared() {
        let base = profile(0.8, 0);
        assert_eq!(base.action.gravity_m_s2, 0.0);
        for index in 1..6 {
            let p = profile(0.8, index);
            assert_eq!(p.action.gravity_m_s2, GRAVITY);
            assert_eq!(
                p.assembly.hammer_mass_kg,
                [0.004, 0.004, 0.008, 0.012, 0.012, 0.012][index]
            );
        }
        let reseated = profile(0.8, 4);
        assert!(
            (reseated.action.damper_closed_m
                - base.action.damper_closed_m
                - base.felt.arm_mass_kg * GRAVITY / base.felt.arm_stiffness_n_m)
                .abs()
                < 1e-18
        );
        assert_eq!(profile(0.8, 5).action.hammer_return_n_m, 2.0);
        assert_eq!(
            profile(0.8, 3).action.hammer_return_n_m,
            base.action.hammer_return_n_m
        );
    }
    #[test]
    fn weighted_rest_seats_the_hammer_on_the_pedestal_and_control_rest_is_unloaded() {
        let (position, _) = tuning::fitted_position(196.38614697959488).unwrap();
        for index in [0, 3] {
            let p = profile(position, index);
            let model = rf_tines_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
            let probe = model.probe();
            let weight = p.assembly.hammer_mass_kg * p.action.gravity_m_s2;
            let sag = probe.mechanical.position[18] - p.action.hammer_rest_m;
            if index == 0 {
                assert_eq!(probe.mechanical.contact_force_n[2], 0.0);
            } else {
                assert!(sag < 0.0 && sag > -1e-4);
                let expected = weight + p.action.hammer_return_n_m * sag;
                assert!((probe.mechanical.contact_force_n[2] - expected).abs() < 1e-9 * expected);
                assert!(probe.mechanical.contact_force_n[1] > 0.0);
            }
        }
    }
}
