//! Short loaded action threshold study with independent hammer and port work.
use super::{tuning, voicing};
use rf_73_dsp::{ElectromechanicalAssembly, ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const FRAMES: usize = 5760;
const SPEEDS: [f64; 6] = [1.0, 1.0625, 1.125, 1.1875, 1.3125, 1.5];
const NAMES: [&str; 5] = [
    "baseline",
    "half_return_stiffness",
    "half_return_damping",
    "escapement_1mm",
    "half_contact_stiffness",
];

fn profile(position: f64, case: usize) -> ElectromechanicalProfile {
    let mut p = voicing::profile(position, 0);
    match case {
        1 => p.action.hammer_return_n_m *= 0.5,
        2 => p.action.hammer_return_n_s_m *= 0.5,
        3 => p.action.escapement_m = 0.001,
        4 => p.assembly.contact_stiffness_n_m2 *= 0.5,
        _ => {}
    }
    p
}
pub(super) fn potential(k: f64, d: f64) -> f64 {
    k * d.max(0.0).powi(3) / 3.0
}
fn hammer_energy(p: ElectromechanicalProfile, b: ElectromechanicalProbe) -> f64 {
    0.5 * p.assembly.hammer_mass_kg * b.mechanical.velocity[18].powi(2)
        + 0.5
            * p.action.hammer_return_n_m
            * (b.mechanical.position[18] - p.action.hammer_rest_m).powi(2)
}

#[derive(Default)]
pub(super) struct Work {
    pedestal_to_hammer: f64,
    hammer_to_contact: f64,
    contact_to_structure: f64,
    hammer_to_bridle: f64,
    return_heat: f64,
    max_hammer_defect: f64,
    max_pedestal_defect: f64,
    max_contact_defect: f64,
}
impl Work {
    pub(super) fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        initial: ElectromechanicalProbe,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        h: f64,
    ) {
        let dh = b.mechanical.position[18] - a.mechanical.position[18];
        let vm = 0.5 * (a.mechanical.velocity[18] + b.mechanical.velocity[18]);
        let f = b.mechanical.contact_force_n;
        self.pedestal_to_hammer += f[2] * dh;
        self.hammer_to_contact += f[0] * dh;
        // compression_0 = q_hammer - spatial tine position at the hammer port.
        self.contact_to_structure +=
            f[0] * (dh - b.mechanical.compression_m[0] + a.mechanical.compression_m[0]);
        self.hammer_to_bridle += p.action.bridle_ratio * f[3] * dh;
        self.return_heat += p.action.hammer_return_n_s_m * h * vm * vm;
        let scale = (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
        let hammer = hammer_energy(p, b) - hammer_energy(p, initial) - self.pedestal_to_hammer
            + self.hammer_to_contact
            + self.hammer_to_bridle
            + self.return_heat;
        let contact = self.hammer_to_contact
            - self.contact_to_structure
            - potential(
                p.assembly.contact_stiffness_n_m2,
                b.mechanical.compression_m[0],
            )
            + potential(
                p.assembly.contact_stiffness_n_m2,
                initial.mechanical.compression_m[0],
            )
            - (b.mechanical.contact_heat_j[0] - initial.mechanical.contact_heat_j[0]);
        let pedestal = b.mechanical.pedestal_work_j
            - initial.mechanical.pedestal_work_j
            - self.pedestal_to_hammer
            - potential(
                p.action.pedestal_stiffness_n_m2,
                b.mechanical.compression_m[2],
            )
            + potential(
                p.action.pedestal_stiffness_n_m2,
                initial.mechanical.compression_m[2],
            )
            - (b.mechanical.contact_heat_j[2] - initial.mechanical.contact_heat_j[2]);
        self.max_hammer_defect = self.max_hammer_defect.max(hammer.abs() / scale);
        self.max_contact_defect = self.max_contact_defect.max(contact.abs() / scale);
        self.max_pedestal_defect = self.max_pedestal_defect.max(pedestal.abs() / scale);
    }
    pub(super) fn defects(&self) -> [f64; 3] {
        [
            self.max_hammer_defect,
            self.max_pedestal_defect,
            self.max_contact_defect,
        ]
    }
    pub(super) fn snapshot(
        &self,
        p: ElectromechanicalProfile,
        b: ElectromechanicalProbe,
        time: f64,
    ) -> Value {
        json!({"seconds":time,"hammer_position_m":b.mechanical.position[18],"hammer_velocity_m_s":b.mechanical.velocity[18],
            "hammer_energy_j":hammer_energy(p,b),"hammer_kinetic_j":0.5*p.assembly.hammer_mass_kg*b.mechanical.velocity[18].powi(2),
            "hammer_return_potential_j":0.5*p.action.hammer_return_n_m*(b.mechanical.position[18]-p.action.hammer_rest_m).powi(2),
            "actuator_pedestal_work_j":b.mechanical.pedestal_work_j,"pedestal_to_hammer_work_j":self.pedestal_to_hammer,
            "hammer_to_contact_work_j":self.hammer_to_contact,"contact_to_structure_work_j":self.contact_to_structure,
            "hammer_to_bridle_work_j":self.hammer_to_bridle,"hammer_return_heat_j":self.return_heat,
            "pedestal_heat_j":b.mechanical.contact_heat_j[2],"hammer_contact_heat_j":b.mechanical.contact_heat_j[0],
            "hammer_contact_potential_j":potential(p.assembly.contact_stiffness_n_m2,b.mechanical.compression_m[0]),
            "pedestal_potential_j":potential(p.action.pedestal_stiffness_n_m2,b.mechanical.compression_m[2]),
            "contact_forces_n":b.mechanical.contact_force_n})
    }
}
struct Take {
    report: Value,
    velocities: Vec<[f64; 3]>,
}
fn take(p: ElectromechanicalProfile, steps: usize, speed: f64) -> Result<Take, Box<dyn Error>> {
    if ![128, 256].contains(&steps) || !speed.is_finite() || !(0.1..=2.0).contains(&speed) {
        return Err("invalid threshold resolution or speed".into());
    }
    let h = 1.0 / (48000.0 * steps as f64);
    let mut model = ElectromechanicalAssembly::new_at_rest(h, p)?;
    let initial = model.probe();
    let mut old = initial;
    let mut x = p.action.hammer_rest_m;
    let mut work = Work::default();
    let mut events = Vec::new();
    let mut overflow = false;
    let mut before_contact = None;
    let mut first_contact = None;
    let mut impulse = 0.0;
    let mut peak_force = 0.0_f64;
    let mut contact_ticks = 0_u64;
    let mut peak_approach = initial.mechanical.compression_m[0];
    let mut balance = 0.0_f64;
    let mut exchange = 0.0_f64;
    let mut return_defect = 0.0_f64;
    let mut quiet = 0.0_f64;
    let mut monotone = true;
    let mut velocities = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = if t >= 0.03 {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-speed * h, speed * h);
            model.advance(x, p.action.damper_closed_m)?;
            let b = model.probe();
            if old.mechanical.contact_entries[0] == 0 {
                peak_approach = peak_approach.max(old.mechanical.compression_m[0]);
            }
            if first_contact.is_none() && b.mechanical.contact_entries[0] > 0 {
                first_contact = Some(t + h);
                before_contact = Some(work.snapshot(p, old, t));
            }
            work.observe(p, initial, old, b, h);
            let f = b.mechanical.contact_force_n[0];
            impulse += h * f;
            peak_force = peak_force.max(f);
            contact_ticks += u64::from(f > 0.0);
            for (label, event) in [
                (
                    "pedestal_departure",
                    old.mechanical.contact_force_n[2] > 0.0
                        && b.mechanical.contact_force_n[2] == 0.0,
                ),
                (
                    "hammer_entry",
                    b.mechanical.contact_entries[0] > old.mechanical.contact_entries[0],
                ),
                (
                    "hammer_exit",
                    old.mechanical.contact_force_n[0] > 0.0 && f == 0.0,
                ),
            ] {
                if event {
                    if events.len() < 64 {
                        events.push(json!({"event":label,"state":work.snapshot(p,b,t+h)}));
                    } else {
                        overflow = true;
                    }
                }
            }
            let scale =
                (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
            balance = balance.max(b.total_balance_residual_j.abs() / scale);
            exchange = exchange.max(b.exchange_residual_j.abs() / scale);
            return_defect = return_defect.max(
                (work.return_heat - b.mechanical.hammer_return_heat_j
                    + initial.mechanical.hammer_return_heat_j)
                    .abs()
                    / scale,
            );
            monotone &= b
                .mechanical
                .contact_heat_j
                .iter()
                .zip(old.mechanical.contact_heat_j)
                .all(|(a, b)| *a >= b)
                && b.mechanical.structural_heat_j >= old.mechanical.structural_heat_j
                && b.mechanical.hammer_return_heat_j >= old.mechanical.hammer_return_heat_j
                && b.mechanical.arm_heat_j >= old.mechanical.arm_heat_j
                && b.coil_heat_j >= old.coil_heat_j
                && b.load_heat_j >= old.load_heat_j;
            if frame < 1440 {
                quiet = quiet.max(b.output_voltage_v.abs());
            }
            old = b;
        }
        velocities.push([
            old.mechanical.velocity[18],
            old.mechanical.pickup_velocity_xy_m_s[0],
            old.mechanical.pickup_velocity_xy_m_s[1],
        ]);
    }
    let passed = balance < 1e-8
        && exchange < 1e-10
        && return_defect < 1e-10
        && quiet < 1e-9
        && monotone
        && !overflow
        && work.max_hammer_defect < 1e-8
        && work.max_pedestal_defect < 1e-8
        && work.max_contact_defect < 1e-8;
    Ok(Take {
        velocities,
        report: json!({"passed":passed,"steps_per_frame":steps,"duration_seconds":0.12,
        "classification":if old.mechanical.contact_entries[0]==0 {"no_contact_in_window"} else if old.mechanical.contact_entries[0]==1 {"single_contact"} else {"multiple_contacts"},
        "contact_entries":old.mechanical.contact_entries,"first_contact_seconds":first_contact,"before_first_contact":before_contact,
        "closest_pre_contact_compression_m":peak_approach,"impact":{"impulse_n_s":impulse,"peak_force_n":peak_force,"active_contact_seconds":contact_ticks as f64*h},
        "max_relative_energy_defect":balance,"max_relative_exchange_defect":exchange,"max_relative_hammer_defect":work.max_hammer_defect,
        "max_relative_pedestal_defect":work.max_pedestal_defect,"max_relative_contact_defect":work.max_contact_defect,"max_relative_return_heat_defect":return_defect,
        "pre_key_raw_voltage_peak_v":quiet,"heat_monotone":monotone,"event_overflow":overflow,"events":events,"end":work.snapshot(p,old,0.12)}),
    })
}
fn convergence(a: &Take, b: &Take) -> Value {
    let channels: Vec<_> = ["hammer", "pickup_vertical", "pickup_horizontal"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let error = a
                .velocities
                .iter()
                .zip(&b.velocities)
                .map(|(a, b)| (a[i] - b[i]).powi(2))
                .sum::<f64>();
            let signal = b.velocities.iter().map(|v| v[i] * v[i]).sum::<f64>();
            let e = (error / signal.max(FRAMES as f64 * 1e-16)).sqrt();
            json!({"channel":name,"relative_velocity_rmse":e,"passed":e<0.01})
        })
        .collect();
    let same = a.report["contact_entries"][0] == b.report["contact_entries"][0];
    let impact = if b.report["contact_entries"][0] == 0 {
        json!({"passed":same && a.report["impact"]==b.report["impact"],"scope":"Exact zero impact for paired no-contact controls"})
    } else {
        voicing::impact_convergence(&a.report["impact"], &b.report["impact"])
    };
    json!({"passed":same && channels.iter().all(|c|c["passed"]==true) && impact["passed"]==true,
        "contact_count_agrees":same,"velocity":channels,"impact":impact})
}
fn transitions(rows: &[Value]) -> Vec<Value> {
    rows.windows(2)
        .filter_map(|w| {
            if w.iter().any(|c| c["measurement_qualified"] != true) {
                return None;
            }
            let struck = |v: &Value| {
                v["takes"][1]["contact_entries"][0]
                    .as_u64()
                    .is_some_and(|n| n > 0)
            };
            (struck(&w[0]) != struck(&w[1])).then(|| {
                json!({"lower_speed_m_s":w[0]["speed_m_s"],"upper_speed_m_s":w[1]["speed_m_s"],
            "lower_struck":struck(&w[0]),"upper_struck":struck(&w[1])})
            })
        })
        .collect()
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-strike-threshold --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("threshold study requires a new JSON path".into());
    }
    let target = 196.38614697959488;
    let (position, fit) = tuning::fitted_position(target)?;
    let mut cases = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let p = profile(position, case);
            let mut rows = Vec::new();
            for speed in SPEEDS {
                println!("Strike threshold: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, speed)?;
                let b = take(p, 256, speed)?;
                let convergence = convergence(&a, &b);
                let passed = a.report["passed"] == true
                    && b.report["passed"] == true
                    && convergence["passed"] == true;
                println!(
                    "Strike threshold: {name}, {speed} m/s, {}, qualified={passed}",
                    b.report["classification"]
                );
                rows.push(json!({"speed_m_s":speed,"measurement_qualified":passed,"takes":[a.report,b.report],"convergence":convergence}));
            }
            cases.push(json!({"name":name,"transition_intervals":transitions(&rows),"rows":rows,
                "settings":{"hammer_return_n_m":p.action.hammer_return_n_m,"hammer_return_n_s_m":p.action.hammer_return_n_s_m,
                    "escapement_m":p.action.escapement_m,"contact_stiffness_n_m2":p.assembly.contact_stiffness_n_m2}}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && cases.len() == 5
        && cases.iter().all(|c| {
            c["rows"].as_array().is_some_and(|r| {
                r.len() == 6 && r.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-strike-threshold-v1","measurement_qualified":passed,"failure_reason":failure,
        "target_hz":target,"structural_fit":fit,"cases":cases,"reference_match_claimed":false,"physical_calibration_claimed":false,
        "protocol":"Frozen 5 by 6 matrix, 60 short takes: conditional 70 mm G3 baseline; halve hammer return stiffness; halve return viscous damping; reduce escapement to 1 mm; halve quadratic hammer contact stiffness. Fixed 1/1.0625/1.125/1.1875/1.3125/1.5 m/s pedestal slew beginning at 30 ms, hold through 120 ms. Stationary rest, reciprocal loaded circuit, 128/256 ticks per 48 kHz observation frame. Independent signed midpoint work ledgers close hammer kinetic/return energy, pedestal-to-hammer transfer, and hammer-contact-to-structure transfer including contact potentials and heat. Require <1e-8 total/hammer/port defects, <1e-10 exchange/return-heat defect, monotone heat, pre-key raw voltage <1e-9 V, no event truncation. Classify no-contact/single/multiple within this finite window; no-contact is a valid control. Require paired hammer entry counts and <1% per-channel velocity RMSE with 1e-8 m/s RMS floor; positive impacts also require <1% impulse/peak/duration refinement, paired no-contact impacts must be exactly zero. Record pre-contact state and bounded pedestal-departure/hammer-entry/exit events with signed energy/work. Report only observed adjacent speed transitions with qualified endpoints, no interpolated threshold or monotonicity assumption. No source fit, gain mapping, WAVs or production changes.",
        "scope":"Short-window action diagnosis in an uncalibrated reduction. Work transfers are signed and trajectory-dependent; they are not additive audible contributions. Escapement changes prescribed travel as well as flight geometry. No-contact means none within 120 ms, not a proof of never striking. No sustain, spectral fit, release/repetition, actual instrument threshold, material identification, listening or realtime claim."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("threshold study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_hammer_work_closes_and_detects_wrong_port_sign() {
        let h = 1e-5;
        let mut p = profile(0.8, 0);
        let initial = ElectromechanicalAssembly::new_at_rest(h, p)
            .unwrap()
            .probe();
        // Isolate the observer's inertial work identity using a prepared zero-velocity probe.
        p.action.hammer_return_n_m = 0.0;
        p.action.hammer_return_n_s_m = 0.0;
        let mut b = initial;
        let force = 2.0;
        b.mechanical.velocity[18] = h * force / p.assembly.hammer_mass_kg;
        b.mechanical.position[18] += 0.5 * h * b.mechanical.velocity[18];
        b.mechanical.contact_force_n[2] = force;
        b.mechanical.absolute_drive_work_j = 1.0;
        let mut work = Work::default();
        work.observe(p, initial, initial, b, h);
        assert!(work.max_hammer_defect < 1e-15);
        b.mechanical.contact_force_n[2] = 0.0;
        b.mechanical.contact_force_n[0] = force;
        let mut wrong = Work::default();
        wrong.observe(p, initial, initial, b, h);
        assert!(wrong.max_hammer_defect > 1e-8);
    }
    #[test]
    fn threshold_intervals_preserve_reversals_and_withhold_unqualified_endpoints() {
        let row = |speed: f64, count: u64, passed: bool| json!({"speed_m_s":speed,"measurement_qualified":passed,"takes":[null,{"contact_entries":[count]}]});
        let rows = vec![
            row(1.0, 0, true),
            row(1.1, 1, true),
            row(1.2, 0, true),
            row(1.3, 1, false),
        ];
        let t = transitions(&rows);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0]["upper_struck"], true);
        assert_eq!(t[1]["upper_struck"], false);
    }
}
