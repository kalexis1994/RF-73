//! Hammer flight budget from escapement release to impact, with hammer mass
//! and bridle/damper-arm load interventions under the sharp let-off.
use super::key::REPEAT;
use super::{bridle, key, launch, letoff, repetition, tuning};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 6] = [
    "control",
    "hammer_mass_x2",
    "hammer_mass_x3",
    "half_arm_damping",
    "half_bridle_rate_loss",
    "half_arm_mass",
];
const TERMS: [&str; 5] = [
    "hammer_to_bridle_work_j",
    "return_heat_j",
    "return_potential_change_j",
    "negative_pedestal_work_j",
    "hammer_to_contact_work_j",
];
const COUPLING: [&str; 7] = [
    "bridle_potential_change_j",
    "bridle_heat_j",
    "bridle_to_arm_work_j",
    "arm_energy_change_j",
    "arm_heat_j",
    "arm_to_felt_work_j",
    "felt_change_j",
];

fn profile(position: f64, index: usize) -> ElectromechanicalProfile {
    let mut p = repetition::profile(position, 0);
    match index {
        1 => p.assembly.hammer_mass_kg = 0.008,
        2 => p.assembly.hammer_mass_kg = 0.012,
        3 => p.felt.arm_damping_n_s_m *= 0.5,
        4 => p.action.bridle_rate_loss_s_m *= 0.5,
        5 => p.felt.arm_mass_kg *= 0.5,
        _ => {}
    }
    p
}
#[derive(Clone)]
struct Snapshot {
    seconds: f64,
    velocity: f64,
    position: f64,
    kinetic: f64,
    potential: f64,
    hammer_to_bridle: f64,
    return_heat: f64,
    pedestal_work: f64,
    contact_work: f64,
    coupling: Value,
}
struct Observer {
    p: ElectromechanicalProfile,
    coupling: bridle::Coupling,
    hammer_to_bridle: f64,
    return_heat: f64,
    pedestal_work: f64,
    contact_work: f64,
    release: Option<Snapshot>,
    last_exit: Option<Snapshot>,
    first_top_exit: Option<Snapshot>,
    impact: Option<Snapshot>,
    end: Option<Snapshot>,
    initial: Option<ElectromechanicalProbe>,
    start: f64,
    window: f64,
}
impl Observer {
    fn new(p: ElectromechanicalProfile, start: f64, window: f64) -> Self {
        Self {
            p,
            coupling: bridle::Coupling::default(),
            hammer_to_bridle: 0.0,
            return_heat: 0.0,
            pedestal_work: 0.0,
            contact_work: 0.0,
            release: None,
            last_exit: None,
            first_top_exit: None,
            impact: None,
            end: None,
            initial: None,
            start,
            window,
        }
    }
    fn snapshot(&self, b: ElectromechanicalProbe, t: f64) -> Snapshot {
        let v = b.mechanical.velocity[18];
        let q = b.mechanical.position[18];
        Snapshot {
            seconds: t,
            velocity: v,
            position: q,
            kinetic: 0.5 * self.p.assembly.hammer_mass_kg * v * v,
            potential: 0.5
                * self.p.action.hammer_return_n_m
                * (q - self.p.action.hammer_rest_m).powi(2),
            hammer_to_bridle: self.hammer_to_bridle,
            return_heat: self.return_heat,
            pedestal_work: self.pedestal_work,
            contact_work: self.contact_work,
            coupling: self.coupling.snapshot(self.p, b),
        }
    }
    fn observe(
        &mut self,
        _take_initial: ElectromechanicalProbe,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        if t < self.start || self.end.is_some() {
            return;
        }
        // The coupling ledger references the state at this observer's own start.
        let initial = *self.initial.get_or_insert(a);
        // The impact snapshot is the state before the first contact tick, the
        // same state the launch ledger uses for its pre-contact record.
        if self.impact.is_none()
            && b.mechanical.contact_entries[0] > a.mechanical.contact_entries[0]
        {
            self.impact = Some(self.snapshot(a, t));
            // Whatever pedestal exit last preceded the strike launched the hammer.
            self.release = self.last_exit.clone();
        }
        let dh = b.mechanical.position[18] - a.mechanical.position[18];
        let vm = 0.5 * (a.mechanical.velocity[18] + b.mechanical.velocity[18]);
        let f = b.mechanical.contact_force_n;
        self.hammer_to_bridle += self.p.action.bridle_ratio * f[3] * dh;
        self.return_heat += self.p.action.hammer_return_n_s_m * h * vm * vm;
        self.pedestal_work += f[2] * dh;
        self.contact_work += f[0] * dh;
        self.coupling.observe(self.p, initial, a, b, h);
        if self.impact.is_none() && a.mechanical.contact_force_n[2] > 0.0 && f[2] == 0.0 {
            let exit = self.snapshot(b, t + h);
            if self.first_top_exit.is_none()
                && b.mechanical.pedestal_position_m == -self.p.action.escapement_m
            {
                self.first_top_exit = Some(exit.clone());
            }
            self.last_exit = Some(exit);
        }
        if t + h >= self.start + self.window - 0.5 * h {
            self.end = Some(self.snapshot(b, t + h));
            if self.impact.is_none() {
                // Without a strike, the exit at the let-off top is the release; a
                // launch that never reached the top keeps its last exit.
                self.release = self
                    .first_top_exit
                    .clone()
                    .or_else(|| self.last_exit.clone());
            }
        }
    }
    fn budget(&self, from: &Snapshot, to: &Snapshot) -> Value {
        let terms = [
            to.hammer_to_bridle - from.hammer_to_bridle,
            to.return_heat - from.return_heat,
            to.potential - from.potential,
            -(to.pedestal_work - from.pedestal_work),
            to.contact_work - from.contact_work,
        ];
        let residual = from.kinetic - to.kinetic - terms.iter().sum::<f64>();
        let scale = (from.kinetic + terms.iter().map(|x| x.abs()).sum::<f64>()).max(1e-20);
        let c =
            |key: &str| to.coupling[key].as_f64().unwrap() - from.coupling[key].as_f64().unwrap();
        let coupling = [
            c("bridle_potential_j"),
            c("bridle_heat_j"),
            c("bridle_to_arm_work_j"),
            c("arm_energy_j"),
            c("arm_heat_j"),
            c("arm_to_felt_work_j"),
            c("felt_potential_j") + c("felt_heat_j"),
        ];
        let bridle_residual = terms[0] - coupling[0] - coupling[1] - coupling[2];
        let arm_residual = coupling[2] - coupling[3] - coupling[4] - coupling[5];
        let coupling_scale =
            (terms[0].abs() + coupling.iter().map(|x| x.abs()).sum::<f64>()).max(1e-20);
        json!({"from_seconds":from.seconds,"to_seconds":to.seconds,"flight_seconds":to.seconds-from.seconds,
            "distance_m":to.position-from.position,"release_speed_m_s":from.velocity,"arrival_speed_m_s":to.velocity,
            "release_kinetic_j":from.kinetic,"arrival_kinetic_j":to.kinetic,"terms_j":terms,
            "relative_hammer_defect":residual.abs()/scale,"coupling_j":coupling,
            "relative_bridle_defect":bridle_residual.abs()/coupling_scale,"relative_arm_defect":arm_residual.abs()/coupling_scale,
            "bridle_share_of_release_kinetic":terms[0]/from.kinetic.max(1e-20)})
    }
    fn report(&self) -> Value {
        let Some(end) = &self.end else {
            return json!({"passed":false,"released":false,"observed":false});
        };
        let coupling_defects = self.coupling.defects();
        let Some(release) = &self.release else {
            // No pedestal exit: a functional outcome, retained without a budget.
            return json!({"passed":coupling_defects.iter().all(|d|*d<1e-8),"released":false,"observed":true,
                "struck":self.impact.is_some(),"start_seconds":self.start,"max_coupling_defects":coupling_defects});
        };
        let to_impact = self.impact.as_ref().map(|i| self.budget(release, i));
        let to_end = self.budget(release, end);
        let defects: Vec<f64> = to_impact
            .iter()
            .chain(std::iter::once(&to_end))
            .flat_map(|b| {
                [
                    "relative_hammer_defect",
                    "relative_bridle_defect",
                    "relative_arm_defect",
                ]
                .map(|k| b[k].as_f64().unwrap())
            })
            .collect();
        let passed =
            defects.iter().all(|d| *d < 1e-8) && coupling_defects.iter().all(|d| *d < 1e-8);
        let at_letoff = release.position >= -self.p.action.escapement_m - 0.0006;
        json!({"passed":passed,"released":true,"observed":true,"struck":self.impact.is_some(),"start_seconds":self.start,
            "release_seconds":release.seconds,"release_speed_m_s":release.velocity,"release_position_m":release.position,
            "released_at_letoff":at_letoff,
            "release_to_impact":to_impact,"release_to_window_end":to_end,"max_coupling_defects":coupling_defects})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
) -> Result<bridle::Take, Box<dyn Error>> {
    let window = key::WINDOW as f64 / 48000.0;
    let mut observers = [
        Observer::new(p, 0.03, window),
        Observer::new(p, REPEAT as f64 / 48000.0, window),
    ];
    let mut result = letoff::take_observed(p, steps, speed, 1, |initial, a, b, t, h| {
        for o in &mut observers {
            o.observe(initial, a, b, t, h);
        }
    })?;
    let flights: Vec<Value> = observers.iter().map(|o| o.report()).collect();
    let passed = flights.iter().all(|f| f["passed"] == true);
    result.report["passed"] = json!(result.report["passed"] == true && passed);
    result.report["flight"] =
        json!({"passed":passed,"term_order":TERMS,"coupling_order":COUPLING,"launches":flights});
    Ok(result)
}
fn scalar_error(a: &Value, b: &Value, floor: f64) -> f64 {
    let a = a.as_f64().unwrap();
    let b = b.as_f64().unwrap();
    (a - b).abs() / b.abs().max(floor)
}
fn array_error(a: &Value, b: &Value) -> f64 {
    let ta = a.as_array().unwrap();
    let tb = b.as_array().unwrap();
    let total: f64 = tb.iter().map(|x| x.as_f64().unwrap().abs()).sum();
    ta.iter()
        .zip(tb)
        .map(|(x, y)| {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() / y.abs().max(1e-3 * total).max(1e-10)
        })
        .fold(0.0_f64, f64::max)
}
fn budget_convergence(a: &Value, b: &Value) -> Option<Value> {
    if a.is_null() || b.is_null() {
        return (a.is_null() && b.is_null()).then(|| json!({"passed":true,"present":false}));
    }
    let time = (a["to_seconds"].as_f64().unwrap() - b["to_seconds"].as_f64().unwrap()).abs();
    let arrival = scalar_error(&a["arrival_speed_m_s"], &b["arrival_speed_m_s"], 0.01);
    let terms = array_error(&a["terms_j"], &b["terms_j"]);
    let coupling = array_error(&a["coupling_j"], &b["coupling_j"]);
    Some(
        json!({"passed":time<0.0001 && arrival<0.01 && terms<0.01 && coupling<0.01,"present":true,
        "time_error_seconds":time,"relative_arrival_speed_error":arrival,"max_relative_term_error":terms,
        "max_relative_coupling_error":coupling}),
    )
}
pub(super) fn flight_convergence(a: &Value, b: &Value) -> Value {
    let mut rows = Vec::new();
    for (i, (fa, fb)) in a["flight"]["launches"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["flight"]["launches"].as_array().unwrap())
        .enumerate()
    {
        let released = fa["released"] == true && fb["released"] == true;
        let release_time = if released {
            (fa["release_seconds"].as_f64().unwrap() - fb["release_seconds"].as_f64().unwrap())
                .abs()
        } else {
            f64::INFINITY
        };
        let release = if released {
            scalar_error(&fa["release_speed_m_s"], &fb["release_speed_m_s"], 0.01)
        } else {
            f64::INFINITY
        };
        let impact = budget_convergence(&fa["release_to_impact"], &fb["release_to_impact"]);
        let end = budget_convergence(&fa["release_to_window_end"], &fb["release_to_window_end"]);
        let passed = fa["released"] == fb["released"]
            && fa["released_at_letoff"] == fb["released_at_letoff"]
            && (released || fa["released"] == false)
            && fa["struck"] == fb["struck"]
            && release_time < 0.0001
            && release < 0.01
            && impact.as_ref().is_some_and(|c| c["passed"] == true)
            && end.as_ref().is_some_and(|c| c["passed"] == true);
        rows.push(
            json!({"launch":i+1,"passed":passed,"struck_agrees":fa["struck"]==fb["struck"],
            "release_time_error_seconds":release_time,"relative_release_speed_error":release,
            "release_to_impact":impact,"release_to_window_end":end}),
        );
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"launches":rows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-flight-budget --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("flight budget study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases: Vec<Value> = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (index, name) in NAMES.iter().enumerate() {
            let p = profile(position, index);
            let mut rows = Vec::new();
            for (row, speed) in [1.125, 1.5].iter().enumerate() {
                println!("Loaded flight budget: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, *speed)?;
                let b = take(p, 256, *speed)?;
                let c = repetition::convergence(&a, &b, REPEAT);
                let l = launch::convergence(&a.report, &b.report);
                let k = key::key_convergence(&a.report, &b.report);
                let e = letoff::letoff_convergence(&a.report, &b.report);
                let f = flight_convergence(&a.report, &b.report);
                let qualified = a.report["passed"] == true
                    && b.report["passed"] == true
                    && [&c, &l, &k, &e, &f].iter().all(|x| x["passed"] == true);
                let first = key::compare_first(
                    &b.report,
                    if index == 0 {
                        &b.report
                    } else {
                        &cases[0]["rows"][row]["takes"][1]
                    },
                );
                println!(
                    "Loaded flight budget: qualified={qualified}, repeatable={}",
                    b.report["repetition"]["two_clean_repeatable_strikes"]
                );
                rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_control":first,
                    "takes":[a.report,b.report],"convergence":c,"launch_convergence":l,"key_convergence":k,
                    "letoff_convergence":e,"flight_convergence":f}));
            }
            cases.push(json!({"name":name,"settings":{"hammer_mass_kg":p.assembly.hammer_mass_kg,
                "arm_mass_kg":p.felt.arm_mass_kg,"arm_damping_n_s_m":p.felt.arm_damping_n_s_m,
                "arm_stiffness_n_m":p.felt.arm_stiffness_n_m,"bridle_ratio":p.action.bridle_ratio,
                "bridle_slack_m":p.action.bridle_slack_m,"bridle_stiffness_n_m2":p.action.bridle_stiffness_n_m2,
                "bridle_rate_loss_s_m":p.action.bridle_rate_loss_s_m,"return_stiffness_n_m":p.action.hammer_return_n_m,
                "return_damping_n_s_m":p.action.hammer_return_n_s_m},"rows":rows}));
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
    let report = json!({"schema_version":1,"experiment":"loaded-flight-budget-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"flight_term_order":TERMS,"coupling_order":COUPLING,
        "physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 24-take matrix on the original profile under the retained sharp let-off with the 0.05 kg key: control; hammer mass 0.008 and 0.012 kg instead of 0.004; damper-arm damping 0.25 instead of 0.5 Ns/m; bridle rate loss 1 instead of 2 s/m; damper-arm mass 0.0005 instead of 0.001 kg. Nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. For each key-down an independent flight observer integrates hammer-to-bridle work, return-damper heat and pedestal work, tracks return-spring potential and hammer kinetic energy, and runs the retained bridle/arm/felt coupling ledger. The release snapshot is the state after the last pedestal exit before the first hammer/tine contact, or, without a strike, after the first pedestal exit taken with the pedestal at the let-off top; whether the release happened within 0.6 mm of the top is retained as functional evidence; the impact snapshot is the state before the first hammer/tine contact tick; a 30 ms window-end snapshot is always taken. Release-to-impact and release-to-window-end budgets must close the hammer identity (release kinetic minus arrival kinetic equals bridle work plus return heat plus potential change minus pedestal work plus tine-contact work), the bridle identity (bridle work equals bridle storage plus heat plus arm transfer) and the arm identity (arm transfer equals arm energy change plus arm heat plus felt transfer) within 1e-8 relative, and the coupling ledger defects must stay below 1e-8. Refinement requires release and arrival times within 0.1 ms, release and arrival speeds within 1% (0.01 m/s floor), budget and coupling terms within 1% normalized by the term or 0.1% of the summed magnitude, and agreeing strike presence. All repetition, launch, key and let-off gates are retained. First-strike comparison against the control uses the prior 5% impact/speed and 1 ms latency limits. No candidate selection.",
        "scope":"One-at-a-time interventions on provisional parameters, not identified material or geometric values. Only the original profile, one key, one let-off and the 60 ms repetition wait are covered. Bridle work is signed transfer; the coupling decomposition attributes it to storage, heat and arm energy, arm damping and felt transfer. No source fit, audio render or production default."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("flight budget study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_take_releases_at_the_top_and_closes_flight_identities() {
        let (position, _) = tuning::fitted_position(196.38614697959488).unwrap();
        let t = take(profile(position, 0), 128, 1.5).unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&t.report["flight"]).unwrap()
        );
        println!(
            "passed {} key {} launches {:?}",
            t.report["passed"],
            t.report["key"]["passed"],
            t.report["launches"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l["passed"].clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(t.report["flight"]["passed"], true);
        assert_eq!(t.report["passed"], true);
    }
    #[test]
    fn flight_observer_closes_hammer_and_coupling_identities_across_a_forced_release() {
        let p = profile(0.8, 1);
        assert_eq!(p.assembly.hammer_mass_kg, 0.008);
        assert_eq!(profile(0.8, 5).felt.arm_mass_kg, 0.0005);
        assert_eq!(profile(0.8, 4).action.bridle_rate_loss_s_m, 1.0);
        let h = 1e-6;
        let mut model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(h, p).unwrap();
        let initial = model.probe();
        let mut observer = Observer::new(p, 0.0, 0.03);
        let mut x = p.action.hammer_rest_m;
        let mut old = initial;
        for i in 0..30_000 {
            let t = i as f64 * h;
            // Prescribed pedestal: rise at 1.2 m/s to the top, then hold.
            x = (x + 1.2 * h).min(-p.action.escapement_m);
            model.advance(x, p.action.damper_closed_m).unwrap();
            let b = model.probe();
            observer.observe(initial, old, b, t, h);
            old = b;
        }
        let r = observer.report();
        assert_eq!(r["released"], true);
        assert_eq!(r["passed"], true);
        let end = &r["release_to_window_end"];
        assert!(end["relative_hammer_defect"].as_f64().unwrap() < 1e-8);
        assert!(end["relative_bridle_defect"].as_f64().unwrap() < 1e-8);
        assert!(end["relative_arm_defect"].as_f64().unwrap() < 1e-8);
        assert!(end["release_kinetic_j"].as_f64().unwrap() > 0.0);
        if r["struck"] == true {
            let flight = &r["release_to_impact"];
            assert!(flight["flight_seconds"].as_f64().unwrap() > 0.0);
            assert!(flight["relative_hammer_defect"].as_f64().unwrap() < 1e-8);
        }
        // A wrong hammer mass breaks the hammer identity but not the coupling one.
        let mut wrong = Observer::new(p, 0.0, 0.03);
        wrong.p.assembly.hammer_mass_kg *= 1.5;
        let mut model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(h, p).unwrap();
        let mut x = p.action.hammer_rest_m;
        let mut old = initial;
        for i in 0..30_000 {
            let t = i as f64 * h;
            x = (x + 1.2 * h).min(-p.action.escapement_m);
            model.advance(x, p.action.damper_closed_m).unwrap();
            let b = model.probe();
            wrong.observe(initial, old, b, t, h);
            old = b;
        }
        let w = wrong.report();
        assert_eq!(w["passed"], false);
        assert!(
            w["release_to_window_end"]["relative_hammer_defect"]
                .as_f64()
                .unwrap()
                > 1e-3
        );
        assert!(
            w["release_to_window_end"]["relative_bridle_defect"]
                .as_f64()
                .unwrap()
                < 1e-8
        );
    }
}
