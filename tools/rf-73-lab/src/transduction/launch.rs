//! Launch work and signed force impulses, without modifying incoming state.
use super::{bridle, repetition, threshold, tuning};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 3] = ["baseline", "return_damping_0_1", "pedestal_rate_loss_10"];
const IMPULSES: [&str; 5] = [
    "pedestal",
    "bridle",
    "hammer_contact",
    "return_spring",
    "return_damper",
];
const WORK: [&str; 6] = [
    "incoming_kinetic",
    "pedestal_work",
    "negative_bridle_work",
    "negative_contact_work",
    "negative_return_heat",
    "released_return_potential",
];

struct Launch {
    initial: ElectromechanicalProbe,
    work: threshold::Work,
    impulses: [f64; 5],
    absolute_impulse: f64,
    max_momentum_defect: f64,
    events: Vec<Value>,
    overflow: bool,
    before_contact: Value,
    peak_velocity: f64,
    peak_time: f64,
}
impl Launch {
    fn new(initial: ElectromechanicalProbe, start: f64) -> Self {
        Self {
            initial,
            work: threshold::Work::default(),
            impulses: [0.0; 5],
            absolute_impulse: 0.0,
            max_momentum_defect: 0.0,
            events: Vec::new(),
            overflow: false,
            before_contact: Value::Null,
            peak_velocity: initial.mechanical.velocity[18],
            peak_time: start,
        }
    }
    fn snapshot(&self, p: ElectromechanicalProfile, b: ElectromechanicalProbe, t: f64) -> Value {
        let mut work = self.work.snapshot(p, b, t);
        for (key, initial) in [
            (
                "actuator_pedestal_work_j",
                self.initial.mechanical.pedestal_work_j,
            ),
            ("pedestal_heat_j", self.initial.mechanical.contact_heat_j[2]),
            (
                "hammer_contact_heat_j",
                self.initial.mechanical.contact_heat_j[0],
            ),
        ] {
            work[key] = json!(work[key].as_f64().unwrap() - initial);
        }
        let n = |key: &str| work[key].as_f64().unwrap();
        let initial_kinetic =
            0.5 * p.assembly.hammer_mass_kg * self.initial.mechanical.velocity[18].powi(2);
        let initial_potential = 0.5
            * p.action.hammer_return_n_m
            * (self.initial.mechanical.position[18] - p.action.hammer_rest_m).powi(2);
        let terms = [
            initial_kinetic,
            n("pedestal_to_hammer_work_j"),
            -n("hammer_to_bridle_work_j"),
            -n("hammer_to_contact_work_j"),
            -n("hammer_return_heat_j"),
            initial_potential - n("hammer_return_potential_j"),
        ];
        json!({"seconds":t,"hammer":work,"kinetic_work_terms_j":terms,"impulse_components_n_s":self.impulses,
            "initial_momentum_n_s":p.assembly.hammer_mass_kg*self.initial.mechanical.velocity[18],
            "momentum_n_s":p.assembly.hammer_mass_kg*b.mechanical.velocity[18],
            "peak_preimpact_velocity_m_s":self.peak_velocity,"peak_preimpact_seconds":self.peak_time,
            "compression_m":b.mechanical.compression_m,"arm_position_m":b.mechanical.position[19],"arm_velocity_m_s":b.mechanical.velocity[19]})
    }
    fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        t: f64,
        h: f64,
    ) {
        if self.before_contact.is_null()
            && b.mechanical.contact_entries[0] > a.mechanical.contact_entries[0]
        {
            self.before_contact = self.snapshot(p, a, t);
        }
        let f = b.mechanical.contact_force_n;
        let qmid = 0.5 * (a.mechanical.position[18] + b.mechanical.position[18]);
        let vmid = 0.5 * (a.mechanical.velocity[18] + b.mechanical.velocity[18]);
        let forces = [
            f[2],
            -p.action.bridle_ratio * f[3],
            -f[0],
            -p.action.hammer_return_n_m * (qmid - p.action.hammer_rest_m),
            -p.action.hammer_return_n_s_m * vmid,
        ];
        for (impulse, force) in self.impulses.iter_mut().zip(forces) {
            *impulse += h * force;
            self.absolute_impulse += h * force.abs();
        }
        self.work.observe(p, self.initial, a, b, h);
        let residual = p.assembly.hammer_mass_kg
            * (b.mechanical.velocity[18] - self.initial.mechanical.velocity[18])
            - self.impulses.iter().sum::<f64>();
        let scale = (p.assembly.hammer_mass_kg * self.initial.mechanical.velocity[18].abs()
            + self.absolute_impulse)
            .max(1e-12);
        self.max_momentum_defect = self.max_momentum_defect.max(residual.abs() / scale);
        if self.before_contact.is_null() && b.mechanical.velocity[18] > self.peak_velocity {
            self.peak_velocity = b.mechanical.velocity[18];
            self.peak_time = t + h;
        }
        for port in [0, 2, 3] {
            let was = a.mechanical.contact_force_n[port] > 0.0;
            let active = f[port] > 0.0;
            if was != active {
                if self.events.len() < 64 {
                    let snapshot = self.snapshot(p, b, t + h);
                    self.events.push(json!({"seconds":t+h,"port":port,"kind":if active{"entry"}else{"exit"},
                        "hammer_position_m":[a.mechanical.position[18],b.mechanical.position[18]],
                        "hammer_velocity_m_s":[a.mechanical.velocity[18],b.mechanical.velocity[18]],
                        "compression_m":[a.mechanical.compression_m[port],b.mechanical.compression_m[port]],"force_n":f[port],
                        "impulse_components_n_s":self.impulses,"kinetic_work_terms_j":snapshot["kinetic_work_terms_j"]}));
                } else {
                    self.overflow = true;
                }
            }
        }
    }
    fn report(
        &self,
        p: ElectromechanicalProfile,
        b: ElectromechanicalProbe,
        t: f64,
        start: f64,
    ) -> Value {
        json!({"passed":!self.overflow && self.max_momentum_defect<1e-8 && self.work.defects().iter().all(|d|*d<1e-8),
            "overflow":self.overflow,"start_seconds":start,"end_seconds":t,
            "initial":Launch::new(self.initial,start).snapshot(p,self.initial,start),"before_contact":self.before_contact,
            "end":self.snapshot(p,b,t),"events":self.events,"max_relative_momentum_defect":self.max_momentum_defect,
            "work_defects":self.work.defects()})
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    repeat: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let mut current: Option<Launch> = None;
    let mut reports = Vec::new();
    let mut tick = 0usize;
    let mut result = repetition::take_observed(p, steps, speed, repeat, |_, a, b, t, h| {
        let frame = tick / steps;
        for start in [1440, repeat] {
            if tick == start * steps {
                current = Some(Launch::new(a, t));
            }
            if (start..start + 960).contains(&frame) {
                let observer = current.as_mut().unwrap();
                observer.observe(p, a, b, t, h);
                if tick + 1 == (start + 960) * steps {
                    reports.push(observer.report(p, b, t + h, start as f64 / 48000.0));
                }
            }
        }
        tick += 1;
    })?;
    result.report["passed"] = json!(
        result.report["passed"] == true
            && reports.len() == 2
            && reports.iter().all(|r| r["passed"] == true)
    );
    result.report["launches"] = json!(reports);
    Ok(result)
}
fn scalar_error(a: &Value, b: &Value, floor: f64) -> f64 {
    let a = a.as_f64().unwrap();
    let b = b.as_f64().unwrap();
    (a - b).abs() / b.abs().max(floor)
}
fn convergence(a: &Value, b: &Value) -> Value {
    let mut rows = Vec::new();
    for (index, (a, b)) in a["launches"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["launches"].as_array().unwrap())
        .enumerate()
    {
        let mut max_impulse = 0.0_f64;
        let mut max_work = 0.0_f64;
        let mut max_time = 0.0_f64;
        let mut same = true;
        for snapshot in ["before_contact", "end"] {
            if a[snapshot].is_null() || b[snapshot].is_null() {
                same &= a[snapshot].is_null() && b[snapshot].is_null();
                continue;
            }
            for (aa, bb) in a[snapshot]["impulse_components_n_s"]
                .as_array()
                .unwrap()
                .iter()
                .zip(b[snapshot]["impulse_components_n_s"].as_array().unwrap())
            {
                max_impulse = max_impulse.max(scalar_error(aa, bb, 1e-9));
            }
            for (aa, bb) in a[snapshot]["kinetic_work_terms_j"]
                .as_array()
                .unwrap()
                .iter()
                .zip(b[snapshot]["kinetic_work_terms_j"].as_array().unwrap())
            {
                max_work = max_work.max(scalar_error(aa, bb, 1e-10));
            }
        }
        for port in [0, 2, 3] {
            let ea: Vec<&Value> = a["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["port"] == port)
                .collect();
            let eb: Vec<&Value> = b["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["port"] == port)
                .collect();
            same &= ea.len() == eb.len();
            for (a, b) in ea.iter().zip(eb) {
                same &= a["kind"] == b["kind"];
                max_time = max_time
                    .max((a["seconds"].as_f64().unwrap() - b["seconds"].as_f64().unwrap()).abs());
            }
        }
        let peak = scalar_error(
            &a["end"]["peak_preimpact_velocity_m_s"],
            &b["end"]["peak_preimpact_velocity_m_s"],
            1e-8,
        );
        rows.push(json!({"strike":index+1,"passed":same && max_impulse<0.01 && max_work<0.01 && max_time<0.0001 && peak<0.01,
            "event_sequences_agree":same,"max_relative_impulse_error":max_impulse,"max_relative_work_term_error":max_work,
            "max_event_time_error_seconds":max_time,"relative_peak_velocity_error":peak}));
    }
    json!({"passed":rows.len()==2 && rows.iter().all(|r|r["passed"]==true),"strikes":rows})
}
fn difference(take: &Value, mass: f64) -> Value {
    let a = &take["launches"][0]["before_contact"];
    let b = &take["launches"][1]["before_contact"];
    if a.is_null() || b.is_null() {
        return json!({"available":false});
    }
    let delta = |key: &str| -> Vec<f64> {
        a[key]
            .as_array()
            .unwrap()
            .iter()
            .zip(b[key].as_array().unwrap())
            .map(|(a, b)| b.as_f64().unwrap() - a.as_f64().unwrap())
            .collect()
    };
    let work = delta("kinetic_work_terms_j");
    let impulse = delta("impulse_components_n_s");
    json!({"available":true,"second_minus_first_kinetic_work_terms_j":work,"second_minus_first_impulse_components_n_s":impulse,
        "incoming_velocity_difference_m_s":(b["initial_momentum_n_s"].as_f64().unwrap()-a["initial_momentum_n_s"].as_f64().unwrap())/mass,
        "preimpact_velocity_difference_m_s":b["hammer"]["hammer_velocity_m_s"].as_f64().unwrap()-a["hammer"]["hammer_velocity_m_s"].as_f64().unwrap(),
        "preimpact_kinetic_difference_j":b["hammer"]["hammer_kinetic_j"].as_f64().unwrap()-a["hammer"]["hammer_kinetic_j"].as_f64().unwrap()})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-launch --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("launch study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut cases = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let p = repetition::profile(position, case);
            let mut rows = Vec::new();
            for speed in [1.125, 1.5] {
                for repeat in [10080, 21600] {
                    println!(
                        "Loaded launch: {name}, {speed} m/s, second onset {} ms",
                        repeat as f64 / 48.0
                    );
                    let a = take(p, 128, speed, repeat)?;
                    let b = take(p, 256, speed, repeat)?;
                    let c = repetition::convergence(&a, &b, repeat);
                    let launch = convergence(&a.report, &b.report);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true
                        && launch["passed"] == true;
                    let differences = difference(&b.report, p.assembly.hammer_mass_kg);
                    println!("Loaded launch: qualified={qualified}");
                    rows.push(json!({"speed_m_s":speed,"release_to_repeat_seconds":(repeat-7200) as f64/48000.0,"measurement_qualified":qualified,
                    "takes":[a.report,b.report],"convergence":c,"launch_convergence":launch,"preimpact_difference":differences}));
                }
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
            c["rows"].as_array().is_some_and(|r| {
                r.len() == 4 && r.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-launch-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"cases":cases,"impulse_component_order":IMPULSES,"kinetic_work_term_order":WORK,
        "physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Replay the frozen 24-take repetition matrix without changing any state, driver or coefficient. Observe the first 20 ms of each key-down. Start independent hammer/contact/pedestal work ledgers at the incoming state and integrate signed pedestal, bridle, tine-contact, return-spring and return-damper force impulses. Momentum balance includes incoming momentum and normalizes residual by its magnitude plus accumulated absolute impulse, floor 1e-12 Ns; defect <1e-8. Work ledger defects <1e-8. Capture initial, pre-first-contact and terminal states, peak preimpact velocity/time and contact force entry/exit events at hammer/pedestal/bridle ports, capped at 64 per launch with overflow rejection. Retain incoming kinetic energy, signed port work, return heat and released return-spring potential as additive terms reproducing preimpact kinetic energy. Refine each impulse component within 1% with 1e-9 Ns denominator floor, each kinetic-work term within 1% with 1e-10 J floor, peak velocity within 1%, and per-port event sequences/times within 0.1 ms. All original repetition energy, velocity, impact and function evidence is retained. Signed second-minus-first work and momentum differences expose the accounting of the changed strike.",
        "scope":"Trajectory accounting, not independent causal intervention on initial state. No state resets, damping switches, new force law, source fit, audio render or default adoption. A ledger difference cannot by itself separate the causal effects of coupled hammer, tine, arm and circuit states. Repeatability retains prior first-attack tradeoffs; geometry/regulation changes need separate qualification."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("launch study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_momentum_and_work_retain_incoming_motion_and_detect_wrong_mass() {
        let p = repetition::profile(0.8, 0);
        let h = 1e-6;
        let mut model = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(h, p).unwrap();
        let mut x = p.action.hammer_rest_m;
        for _ in 0..2000 {
            x += h;
            model.advance(x, p.action.damper_closed_m).unwrap();
        }
        let initial = model.probe();
        assert!(initial.mechanical.velocity[18].abs() > 0.01);
        let mut correct = Launch::new(initial, 0.002);
        let mut wrong = Launch::new(initial, 0.002);
        let mut bad = p;
        bad.assembly.hammer_mass_kg *= 1.1;
        let mut old = initial;
        for i in 0..4000 {
            x = (x + h).min(-p.action.escapement_m);
            model.advance(x, p.action.damper_closed_m).unwrap();
            let b = model.probe();
            correct.observe(p, old, b, 0.002 + i as f64 * h, h);
            wrong.observe(bad, old, b, 0.002 + i as f64 * h, h);
            old = b;
        }
        assert!(
            correct.max_momentum_defect < 1e-8 && correct.work.defects().iter().all(|x| *x < 1e-8)
        );
        assert!(wrong.max_momentum_defect > 1e-4);
        let s = correct.snapshot(p, old, 0.006);
        let sum = s["kinetic_work_terms_j"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .sum::<f64>();
        assert!((sum - s["hammer"]["hammer_kinetic_j"].as_f64().unwrap()).abs() < 1e-12);
        assert!(s["initial_momentum_n_s"].as_f64().unwrap().abs() > 0.0);
    }
}
