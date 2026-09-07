//! Loaded bridle/damper work, lift and key-release diagnostics.
use super::{threshold, tuning, voicing};
use rf_73_dsp::{ElectromechanicalAssembly, ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const FRAMES: usize = 19200;
const SPEEDS: [f64; 3] = [1.0, 1.125, 1.5];
const NAMES: [&str; 5] = [
    "baseline",
    "slack_4mm",
    "ratio_0_6",
    "half_arm_stiffness",
    "half_arm_damping",
];
fn profile(position: f64, case: usize) -> ElectromechanicalProfile {
    let mut p = voicing::profile(position, 0);
    match case {
        1 => p.action.bridle_slack_m = 0.004,
        2 => p.action.bridle_ratio = 0.6,
        3 => p.felt.arm_stiffness_n_m *= 0.5,
        4 => p.felt.arm_damping_n_s_m *= 0.5,
        _ => {}
    }
    p
}
fn arm_energy(p: ElectromechanicalProfile, b: ElectromechanicalProbe) -> f64 {
    0.5 * p.felt.arm_mass_kg * b.mechanical.velocity[19].powi(2)
        + 0.5
            * p.felt.arm_stiffness_n_m
            * (b.mechanical.position[19] - b.mechanical.pedal_position_m).powi(2)
}
#[derive(Default)]
struct Coupling {
    hammer_to_bridle: f64,
    bridle_to_arm: f64,
    arm_to_felt: f64,
    felt_to_structure: f64,
    arm_heat: f64,
    pedal_work: f64,
    defects: [f64; 5],
}
impl Coupling {
    fn observe(
        &mut self,
        p: ElectromechanicalProfile,
        initial: ElectromechanicalProbe,
        a: ElectromechanicalProbe,
        b: ElectromechanicalProbe,
        h: f64,
    ) {
        let dh = b.mechanical.position[18] - a.mechanical.position[18];
        let dz = b.mechanical.position[19] - a.mechanical.position[19];
        let dr = b.mechanical.pedal_position_m - a.mechanical.pedal_position_m;
        let vm = 0.5 * (a.mechanical.velocity[19] + b.mechanical.velocity[19]);
        let f = b.mechanical.contact_force_n;
        self.hammer_to_bridle += p.action.bridle_ratio * f[3] * dh;
        self.bridle_to_arm -= f[3] * dz;
        self.arm_to_felt += f[1] * dz;
        self.felt_to_structure +=
            f[1] * (dz - b.mechanical.compression_m[1] + a.mechanical.compression_m[1]);
        self.arm_heat += p.felt.arm_damping_n_s_m * h * (vm - dr / h).powi(2);
        self.pedal_work -= (p.felt.arm_stiffness_n_m
            * 0.5
            * (a.mechanical.position[19] + b.mechanical.position[19]
                - a.mechanical.pedal_position_m
                - b.mechanical.pedal_position_m)
            + p.felt.arm_damping_n_s_m * (vm - dr / h))
            * dr;
        let du = |k: f64, i: usize| {
            threshold::potential(k, b.mechanical.compression_m[i])
                - threshold::potential(k, initial.mechanical.compression_m[i])
        };
        let heat = |i: usize| b.mechanical.contact_heat_j[i] - initial.mechanical.contact_heat_j[i];
        let defects = [
            self.hammer_to_bridle
                - self.bridle_to_arm
                - du(p.action.bridle_stiffness_n_m2, 3)
                - heat(3),
            arm_energy(p, b) - arm_energy(p, initial) - self.bridle_to_arm
                + self.arm_to_felt
                + self.arm_heat
                - self.pedal_work,
            self.arm_to_felt - self.felt_to_structure - du(p.felt.felt_stiffness_n_m2, 1) - heat(1),
            self.arm_heat - (b.mechanical.arm_heat_j - initial.mechanical.arm_heat_j),
            self.pedal_work - (b.mechanical.pedal_work_j - initial.mechanical.pedal_work_j),
        ];
        let scale = (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
        for (m, d) in self.defects.iter_mut().zip(defects) {
            *m = m.max(d.abs() / scale);
        }
    }
    fn snapshot(&self, p: ElectromechanicalProfile, b: ElectromechanicalProbe) -> Value {
        json!({"hammer_to_bridle_work_j":self.hammer_to_bridle,"bridle_to_arm_work_j":self.bridle_to_arm,
            "arm_to_felt_work_j":self.arm_to_felt,"felt_to_structure_work_j":self.felt_to_structure,
            "arm_heat_j":self.arm_heat,"pedal_work_j":self.pedal_work,"arm_energy_j":arm_energy(p,b),
            "arm_position_m":b.mechanical.position[19],"arm_velocity_m_s":b.mechanical.velocity[19],
            "felt_clearance_m":-b.mechanical.compression_m[1],"felt_force_n":b.mechanical.contact_force_n[1],
            "bridle_potential_j":threshold::potential(p.action.bridle_stiffness_n_m2,b.mechanical.compression_m[3]),
            "felt_potential_j":threshold::potential(p.felt.felt_stiffness_n_m2,b.mechanical.compression_m[1]),
            "bridle_heat_j":b.mechanical.contact_heat_j[3],"felt_heat_j":b.mechanical.contact_heat_j[1]})
    }
}
fn target(p: ElectromechanicalProfile, time: f64) -> f64 {
    if (0.03..0.15).contains(&time) {
        -p.action.escapement_m
    } else {
        p.action.hammer_rest_m
    }
}
pub(super) struct Take {
    pub(super) report: Value,
    pub(super) velocities: Vec<[f64; 4]>,
}
fn take(p: ElectromechanicalProfile, steps: usize, speed: f64) -> Result<Take, Box<dyn Error>> {
    take_observed(p, steps, speed, FRAMES, |_, _, _, _, _| {})
}
pub(super) fn take_observed(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    frames: usize,
    observe: impl FnMut(
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        f64,
        f64,
    ),
) -> Result<Take, Box<dyn Error>> {
    take_driven(p, steps, speed, frames, |t| target(p, t), observe)
}
pub(super) fn take_driven(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    frames: usize,
    drive: impl Fn(f64) -> f64,
    mut observe: impl FnMut(
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        ElectromechanicalProbe,
        f64,
        f64,
    ),
) -> Result<Take, Box<dyn Error>> {
    if ![128, 256].contains(&steps)
        || !speed.is_finite()
        || !(0.1..=2.0).contains(&speed)
        || !(FRAMES..=48000).contains(&frames)
    {
        return Err("invalid observed action protocol".into());
    }
    let h = 1.0 / (48000.0 * steps as f64);
    let mut model = ElectromechanicalAssembly::new_at_rest(h, p)?;
    let initial = model.probe();
    let mut old = initial;
    let mut x = p.action.hammer_rest_m;
    let mut hammer = threshold::Work::default();
    let mut coupling = Coupling::default();
    let initial_coupling = coupling.snapshot(p, initial);
    let mut snapshots = Vec::new();
    let mut first_contact = Value::Null;
    let mut impact = [0.0_f64; 3];
    let mut contact_ticks = 0_u64;
    let mut balance = 0.0_f64;
    let mut exchange = 0.0_f64;
    let mut quiet = 0.0_f64;
    let mut monotone = true;
    let mut lift = f64::INFINITY;
    let mut held_felt_ticks = 0_u64;
    let mut return_contacts = 0_u64;
    let mut return_ticks = 0_u64;
    let mut return_position = [0.0_f64; 2];
    let mut return_speed = [0.0_f64; 2];
    let mut velocities = Vec::with_capacity(frames);
    for frame in 0..frames {
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = drive(t);
            if !target.is_finite()
                || !(p.action.hammer_rest_m..=-p.action.escapement_m).contains(&target)
            {
                return Err("action drive target outside regulated travel".into());
            }
            x += (target - x).clamp(-speed * h, speed * h);
            model.advance(x, p.action.damper_closed_m)?;
            let b = model.probe();
            if first_contact.is_null() && b.mechanical.contact_entries[0] > 0 {
                first_contact = json!({"seconds":t+h,"before_hammer":hammer.snapshot(p,old,t),"before_coupling":coupling.snapshot(p,old)});
            }
            hammer.observe(p, initial, old, b, h);
            coupling.observe(p, initial, old, b, h);
            observe(initial, old, b, t, h);
            let f = b.mechanical.contact_force_n[0];
            impact[0] += h * f;
            impact[1] = impact[1].max(f);
            contact_ticks += u64::from(f > 0.0);
            let scale =
                (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
            balance = balance.max(b.total_balance_residual_j.abs() / scale);
            exchange = exchange.max(b.exchange_residual_j.abs() / scale);
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
            if (3840..6720).contains(&frame) {
                lift = lift.min(-b.mechanical.compression_m[1]);
                held_felt_ticks += u64::from(b.mechanical.contact_force_n[1] > 0.0);
            }
            if frame >= frames - 2400 {
                return_ticks += 1;
                return_contacts += u64::from(b.mechanical.contact_force_n[1] > 0.0);
                for i in 0..2 {
                    return_position[i] = return_position[i].max(
                        (b.mechanical.position[18 + i] - initial.mechanical.position[18 + i]).abs(),
                    );
                    return_speed[i] = return_speed[i].max(b.mechanical.velocity[18 + i].abs());
                }
            }
            old = b;
        }
        velocities.push([
            old.mechanical.velocity[18],
            old.mechanical.velocity[19],
            old.mechanical.pickup_velocity_xy_m_s[0],
            old.mechanical.pickup_velocity_xy_m_s[1],
        ]);
        if [5760, 7200, 10560, 19200, frames].contains(&(frame + 1)) {
            snapshots.push(json!({"seconds":(frame+1) as f64/48000.0,"hammer":hammer.snapshot(p,old,(frame+1) as f64/48000.0),
                "coupling":coupling.snapshot(p,old),"contact_entries":old.mechanical.contact_entries}));
        }
    }
    impact[2] = contact_ticks as f64 * h;
    let fraction = return_contacts as f64 / return_ticks as f64;
    let lifted = lift >= 0.0001 && held_felt_ticks == 0;
    let returned = return_position.iter().all(|x| *x < 0.0001)
        && return_speed.iter().all(|x| *x < 0.01)
        && fraction >= 0.9;
    let passed = balance < 1e-8
        && exchange < 1e-10
        && quiet < 1e-9
        && monotone
        && hammer.defects().iter().all(|x| *x < 1e-8)
        && coupling.defects[..3].iter().all(|x| *x < 1e-8)
        && coupling.defects[3..].iter().all(|x| *x < 1e-10);
    Ok(Take {
        velocities,
        report: json!({"passed":passed,"steps_per_frame":steps,"duration_seconds":frames as f64/48000.0,
        "initial_coupling":initial_coupling,"first_contact":first_contact,"snapshots":snapshots,"contact_entries":old.mechanical.contact_entries,
        "impact":{"impulse_n_s":impact[0],"peak_force_n":impact[1],"active_contact_seconds":impact[2]},
        "max_relative_energy_defect":balance,"max_relative_exchange_defect":exchange,"hammer_work_defects":hammer.defects(),
        "coupling_work_defects":coupling.defects,"heat_monotone":monotone,"pre_key_raw_voltage_peak_v":quiet,
        "function":{"struck":old.mechanical.contact_entries[0]>0,"single_strike":old.mechanical.contact_entries[0]==1,
            "held_lift_passed":lifted,"minimum_held_felt_clearance_m":lift,"held_felt_contact_ticks":held_felt_ticks,
            "return_passed":returned,"return_max_position_error_m":return_position,"return_max_velocity_m_s":return_speed,"return_felt_contact_fraction":fraction,
            "single_strike_lift_return_passed":old.mechanical.contact_entries[0]==1 && lifted && returned}}),
    })
}
pub(super) fn convergence(a: &Take, b: &Take) -> Value {
    let mut rows = Vec::new();
    let mut windows = vec![(1440, 7200), (7200, 10560), (10560, 19200)];
    if b.velocities.len() > FRAMES {
        windows.push((FRAMES, b.velocities.len()));
    }
    for (lo, hi) in windows {
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
            rows.push(json!({"start_seconds":lo as f64/48000.0,"end_seconds":hi as f64/48000.0,"channel":name,"relative_velocity_rmse":e,"passed":e<0.01}));
        }
    }
    let same = a.report["contact_entries"][0] == b.report["contact_entries"][0];
    let impact = if b.report["contact_entries"][0] == 0 {
        json!({"passed":same && a.report["impact"]==b.report["impact"]})
    } else {
        voicing::impact_convergence(&a.report["impact"], &b.report["impact"])
    };
    json!({"passed":same && rows.iter().all(|r|r["passed"]==true) && impact["passed"]==true,"hammer_count_agrees":same,"velocity":rows,"impact":impact})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-bridle --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("bridle study requires a new JSON path".into());
    }
    let target_hz = 196.38614697959488;
    let (position, fit) = tuning::fitted_position(target_hz)?;
    let mut cases = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let p = profile(position, case);
            let mut rows = Vec::new();
            for speed in SPEEDS {
                println!("Loaded bridle: {name}, {speed} m/s, 128/256 ticks");
                let a = take(p, 128, speed)?;
                let b = take(p, 256, speed)?;
                let c = convergence(&a, &b);
                let passed =
                    a.report["passed"] == true && b.report["passed"] == true && c["passed"] == true;
                println!(
                    "Loaded bridle: {name}, {speed} m/s, qualified={passed}, function={}",
                    b.report["function"]["single_strike_lift_return_passed"]
                );
                rows.push(json!({"speed_m_s":speed,"measurement_qualified":passed,"takes":[a.report,b.report],"convergence":c}));
            }
            cases.push(json!({"name":name,"rows":rows,"settings":{"bridle_slack_m":p.action.bridle_slack_m,"bridle_ratio":p.action.bridle_ratio,
                "arm_stiffness_n_m":p.felt.arm_stiffness_n_m,"arm_damping_n_s_m":p.felt.arm_damping_n_s_m}}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && cases.len() == 5
        && cases.iter().all(|c| {
            c["rows"].as_array().is_some_and(|r| {
                r.len() == 3 && r.iter().all(|r| r["measurement_qualified"] == true)
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-bridle-v1","measurement_qualified":passed,"failure_reason":failure,
        "target_hz":target_hz,"structural_fit":fit,"cases":cases,"reference_match_claimed":false,"physical_calibration_claimed":false,
        "coupling_defect_order":["bridle_port","arm_energy","felt_port","arm_heat","pedal_work"],
        "protocol":"Frozen five-case/three-speed study: conditional 70 mm G3 baseline; bridle slack 4 mm versus 2; ratio 0.6 versus 0.8; halve arm stiffness; halve arm damping. All other parameters fixed. 1/1.125/1.5 m/s pedestal slew from stationary rest, key down at 30 ms, return at 150 ms at the same bounded slew, held at rest through 400 ms. Pedal fixed closed. Thirty takes at 128/256 ticks per 48 kHz observation frame. Reuse independent hammer work ledger and add signed bridle, arm and felt transfer/storage/heat identities including initial felt preload. Gate relative total/hammer/port energy defects <1e-8, exchange/independent arm heat/pedal work <1e-10, monotone heat and pre-key raw voltage <1e-9 V. Require matching hammer counts, <1% positive-impact impulse/peak/duration refinement (exact zero for paired no-contact), and independent hammer/arm/two pickup velocity RMSE <1% in drive/hold, release and return windows, with 1e-8 m/s RMS floor. Preserve 120 ms prefix for historical replay. Functional diagnostics remain separate: exactly one impact, at least 0.1 mm felt clearance and zero felt contact throughout 80-140 ms, then positions within 0.1 mm of each profile's prepared rest, speeds below 0.01 m/s and felt-contact fraction at least 90% throughout 350-400 ms. No-contact and recontact remain recorded. No candidate selection, source fit, WAVs or production changes.",
        "scope":"Conditional work routing and key-release diagnosis, not measured instrument linkage or half-pedal behavior. Signed port transfer is not heat; reduced arm stiffness also changes static preload. Functional limits are engineering diagnostics, not calibrated regulation tolerances. A 400 ms gesture does not establish long sustain, repetition, full keyboard, listening or realtime performance."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("bridle study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coupled_work_preserves_preload_and_closes_through_drive_and_return() {
        let p = profile(0.8, 0);
        let h = 1e-6;
        let mut model = ElectromechanicalAssembly::new_at_rest(h, p).unwrap();
        let initial = model.probe();
        assert!(arm_energy(p, initial) > 0.0);
        let mut old = initial;
        let mut observer = Coupling::default();
        let mut x = p.action.hammer_rest_m;
        let mut max_transfer = 0.0_f64;
        for tick in 0..60000 {
            let target = if (5000..25000).contains(&tick) {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            model.advance(x, p.action.damper_closed_m).unwrap();
            let b = model.probe();
            observer.observe(p, initial, old, b, h);
            max_transfer = max_transfer.max(observer.hammer_to_bridle);
            old = b;
        }
        assert!(observer.defects.iter().all(|d| *d < 1e-8));
        assert!(observer.arm_heat > 0.0 && max_transfer > 0.001);
        assert!(observer.hammer_to_bridle < max_transfer);
        assert_eq!(observer.pedal_work, 0.0);
    }
}
