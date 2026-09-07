//! Conditional loss attribution of a fixed, tuned, loaded G3 mechanism.
use rf_73_analysis::{AudioClip, measure_timbre_profile, pitch_anchor};
use rf_73_dsp::{
    ElectromechanicalAssembly, ElectromechanicalProbe, ElectromechanicalProfile,
    ProductionDecimator,
};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const RATE: u32 = 48000;
const FRAMES: usize = 86400;
const NAMES: [&str; 6] = [
    "baseline",
    "half_support_loss",
    "half_tine_loss",
    "half_tonebar_loss",
    "half_damper_loss",
    "load_100k",
];
const CHANNELS: [&str; 12] = [
    "support_translation",
    "support_rotation",
    "tine",
    "tonebar",
    "hammer_return",
    "damper_arm",
    "hammer_contact",
    "felt_contact",
    "pedestal_contact",
    "bridle_contact",
    "coil",
    "load",
];

// Principal boundary coordinates couple only corresponding transverse pairs.
// Refuse a future operator outside that structure instead of silently dropping terms.
struct LossObserver([[f64; 3]; 9]);
impl LossObserver {
    fn new(c: [[f64; 18]; 18]) -> Result<Self, Box<dyn Error>> {
        for (i, row) in c.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                if !value.is_finite() || value != c[j][i] || (i % 9 != j % 9 && value != 0.0) {
                    return Err("unsupported structural loss operator".into());
                }
            }
        }
        let pairs = core::array::from_fn(|i| [c[i][i], c[i + 9][i + 9], c[i][i + 9]]);
        if pairs
            .iter()
            .any(|r| r[0] < 0.0 || r[1] < 0.0 || r[2] * r[2] > r[0] * r[1] * (1.0 + 1e-12))
        {
            return Err("nonpassive structural loss operator".into());
        }
        Ok(Self(pairs))
    }
    fn power(&self, v: [f64; 20]) -> [f64; 4] {
        let mut power = [0.0; 4];
        for (i, [a, b, c]) in self.0.iter().copied().enumerate() {
            let group = match i {
                0 => 0,
                1 => 1,
                8 => 3,
                _ => 2,
            };
            power[group] += a * v[i] * v[i] + b * v[i + 9] * v[i + 9] + 2.0 * c * v[i] * v[i + 9];
        }
        power
    }
}
fn profile(position: f64, case: usize) -> ElectromechanicalProfile {
    let mut p = ElectromechanicalProfile::default();
    p.geometry.length_m = 0.07;
    p.geometry.tuning_position = position;
    match case {
        1 => {
            p.assembly.translation_damping_n_s_m *= 0.5;
            p.assembly.rotation_damping_n_m_s_rad *= 0.5;
        }
        2 => {
            p.assembly.tine_decay_seconds = p.assembly.tine_decay_seconds.map(|t| 2.0 * t);
        }
        3 => {
            p.assembly.tonebar_decay_seconds *= 2.0;
        }
        4 => {
            p.felt.arm_damping_n_s_m *= 0.5;
            p.felt.felt_rate_loss_s_m *= 0.5;
        }
        5 => {
            p.circuit.load_resistance_ohm = Some(100000.0);
        }
        _ => {}
    }
    p
}
fn heat(b: ElectromechanicalProbe, structural: [f64; 4]) -> [f64; 12] {
    let m = b.mechanical;
    [
        structural[0],
        structural[1],
        structural[2],
        structural[3],
        m.hammer_return_heat_j,
        m.arm_heat_j,
        m.contact_heat_j[0],
        m.contact_heat_j[1],
        m.contact_heat_j[2],
        m.contact_heat_j[3],
        b.coil_heat_j,
        b.load_heat_j,
    ]
}
struct Take {
    samples: Vec<f64>,
    report: Value,
}
fn take(position: f64, case: usize, steps: usize) -> Result<Take, Box<dyn Error>> {
    let p = profile(position, case);
    let h = 1.0 / (f64::from(RATE) * steps as f64);
    let mut model = ElectromechanicalAssembly::new_at_rest(h, p)?;
    let observer = LossObserver::new(model.structural_damping_matrix())?;
    let initial = model.probe();
    let mut old = initial;
    let mut x = p.action.hammer_rest_m;
    let mut structural = [0.0; 4];
    let mut filter = ProductionDecimator::new();
    let mut samples = Vec::with_capacity(FRAMES);
    let mut balance = 0.0_f64;
    let mut split_defect = 0.0_f64;
    let mut exchange = 0.0_f64;
    let mut monotone = true;
    let mut onset = None;
    let mut snapshots = Vec::new();
    let mut prior_heat = [0.0; 12];
    let mut prior_probe = initial;
    let mut prior_frame = 0;
    let mut felt_steps = 0_u64;
    let mut stationary_felt_steps = 0_u64;
    for frame in 0..FRAMES {
        let mut average = 0.0;
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = if t >= 0.03 {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            model.advance(x, p.action.damper_closed_m)?;
            let b = model.probe();
            if onset.is_none() && b.mechanical.contact_entries[0] > 0 {
                onset = Some(t + h);
            }
            let vm = core::array::from_fn(|i| {
                0.5 * (old.mechanical.velocity[i] + b.mechanical.velocity[i])
            });
            let power = observer.power(vm);
            if power.iter().any(|v| !v.is_finite() || *v < -1e-20) {
                return Err("invalid observed loss power".into());
            }
            for i in 0..4 {
                structural[i] += h * power[i];
            }
            let scale =
                (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
            split_defect = split_defect.max(
                (structural.iter().sum::<f64>() - b.mechanical.structural_heat_j).abs() / scale,
            );
            balance = balance.max(b.total_balance_residual_j.abs() / scale);
            exchange = exchange.max(b.exchange_residual_j.abs() / scale);
            monotone &= heat(b, structural)
                .iter()
                .zip(heat(
                    old,
                    core::array::from_fn(|i| structural[i] - h * power[i]),
                ))
                .all(|(a, b)| *a >= b);
            if b.mechanical.contact_force_n[1] > 0.0 {
                felt_steps += 1;
                if t >= 0.3 {
                    stationary_felt_steps += 1;
                }
            }
            average += 0.5 * (old.output_voltage_v + b.output_voltage_v) / (steps / 4) as f64;
            old = b;
            if (sub + 1) % (steps / 4) == 0 {
                filter.push(average);
                average = 0.0;
            }
        }
        samples.push(0.1 * filter.output());
        if [1440, 14400, 28800, 57600, 86400].contains(&(frame + 1)) {
            let current_heat = heat(old, structural);
            let delta: [f64; 12] = core::array::from_fn(|i| current_heat[i] - prior_heat[i]);
            let q = delta.iter().sum::<f64>();
            let work = old.mechanical.pedestal_work_j + old.mechanical.pedal_work_j
                - prior_probe.mechanical.pedestal_work_j
                - prior_probe.mechanical.pedal_work_j;
            let energy = |v: ElectromechanicalProbe| {
                v.mechanical.mechanical_energy_j + v.electrical_energy_j
            };
            let residual = energy(old) - energy(prior_probe) + q - work;
            let relative_residual = residual.abs()
                / (old.mechanical.initial_energy_j + old.mechanical.absolute_drive_work_j)
                    .max(1e-20);
            snapshots.push(json!({"start_seconds":prior_frame as f64/f64::from(RATE),"end_seconds":(frame+1) as f64/f64::from(RATE),
                "heat_j":delta,"heat_fraction":if q>1e-18 {Some(delta.map(|x|x/q))} else {None},"total_heat_j":q,
                "start_energy_j":energy(prior_probe),"end_energy_j":energy(old),"drive_work_j":work,"energy_residual_j":residual,
                "relative_energy_defect":relative_residual,"passed":relative_residual<1e-8,"felt_contact_ticks":felt_steps}));
            prior_heat = current_heat;
            prior_probe = old;
            prior_frame = frame + 1;
            felt_steps = 0;
        }
    }
    let peak = samples.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    let quiet = samples[..1440]
        .iter()
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    let clip = AudioClip::from_samples(RATE, samples.clone())?;
    let anchor = pitch_anchor(&clip, 55)?;
    let frequency = anchor.frequency_hz.ok_or("loss study pitch unavailable")?;
    let timbre = measure_timbre_profile(
        &clip,
        onset.ok_or("loss study produced no impact")? + 63.0 / (48000.0 * 4.0),
        frequency,
    )?;
    let passed = balance < 1e-8
        && exchange < 1e-10
        && split_defect < 1e-10
        && monotone
        && quiet < 1e-10
        && peak > 0.0
        && peak < 1.0
        && timbre.qualified
        && snapshots.iter().all(|w| w["passed"] == true)
        && old.mechanical.contact_entries[0] == 1;
    Ok(Take {
        samples,
        report: json!({"passed":passed,"steps_per_frame":steps,"max_relative_energy_defect":balance,"max_relative_exchange_defect":exchange,
        "max_relative_structural_split_defect":split_defect,"heat_monotone":monotone,"peak_fs":peak,"pre_key_peak_fs":quiet,"initial_position":initial.mechanical.position,
        "first_contact_seconds":onset,"contact_entries":old.mechanical.contact_entries,"felt_contact_ticks_after_300ms":stationary_felt_steps,
        "pitch_anchor":anchor,"timbre":timbre,"windows":snapshots}),
    })
}
fn convergence(a: &Take, b: &Take) -> Value {
    let windows:Vec<_>=[(0,14400),(14400,28800),(28800,57600),(57600,86400)].into_iter().map(|(lo,hi)| {
        let e=a.samples[lo..hi].iter().zip(&b.samples[lo..hi]).map(|(a,b)|(a-b).powi(2)).sum::<f64>();
        let s=b.samples[lo..hi].iter().map(|x|x*x).sum::<f64>();
        let error=(e/s.max(1e-30)).sqrt();
        json!({"start_seconds":lo as f64/48000.0,"end_seconds":hi as f64/48000.0,"relative_voltage_rmse":error,"passed":error<0.01})
    }).collect();
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-loss-budget --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|s| s != "json") {
        return Err("loss budget requires a new .json path".into());
    }
    // A fixed target from the qualified training-only bank pilot. No re-fit to these interventions.
    let target = 196.38614697959488;
    let (position, fit) = super::tuning::fitted_position(target)?;
    let mut cases = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            println!("Loss budget: {name}, 128 ticks");
            let a = take(position, case, 128)?;
            println!("Loss budget: {name}, 256 ticks");
            let b = take(position, case, 256)?;
            let comparison = convergence(&a, &b);
            let p = profile(position, case);
            let passed = a.report["passed"] == true
                && b.report["passed"] == true
                && comparison["passed"] == true;
            println!(
                "Loss budget: {name}, passed={passed}, late level {} dB",
                b.report["timbre"]["windows"][4]["level_relative_to_body_db"]
            );
            cases.push(json!({"name":name,"passed":passed,"takes":[a.report,b.report],"convergence":comparison,
            "settings":{"support_translation_damping":p.assembly.translation_damping_n_s_m,"support_rotation_damping":p.assembly.rotation_damping_n_m_s_rad,
            "tine_t60_seconds":p.assembly.tine_decay_seconds,"tonebar_t60_seconds":p.assembly.tonebar_decay_seconds,
            "felt_rate_loss_s_m":p.felt.felt_rate_loss_s_m,"arm_damping_n_s_m":p.felt.arm_damping_n_s_m,"load_resistance_ohm":p.circuit.load_resistance_ohm}}));
        }
        Ok(())
    })();
    let failure_reason = outcome.err().map(|e| e.to_string());
    let report = json!({"schema_version":1,"experiment":"loaded-loss-budget-v1","passed":failure_reason.is_none() && cases.len()==6 && cases.iter().all(|c|c["passed"]==true),"failure_reason":failure_reason,"channels":CHANNELS,"cases":cases,"target_hz":target,"structural_fit":fit,
        "protocol":"Frozen six-case conditional sensitivity study, 12 takes. Tuned 70 mm G3, default stationary preload, same 1.5 m/s pedestal gesture from 30 ms, held through 1.8 s. Baseline; halve support viscous coefficients; double tine T60 values; double tonebar T60; halve felt rate loss and arm viscous loss together; change 10k load to 100k. All other parameters remain fixed; no retuning after interventions. 128/256 ticks at 48 kHz, midpoint voltage averaged to 4x and common FIR at fixed 0.1 FS/V. Independent sparse damping-quadratic observer separates support translation/rotation, tine and tonebar heat and checks their sum against the solver ledger. Retain other mechanical/electrical heat, actuator work and stored energy over 0-30,30-300,300-600,600-1200,1200-1800 ms. Require relative energy <1e-8, exchange/split defects <1e-10, monotone heat, first-30-ms output <1e-10 FS, finite unclipped output, one strike, qualified pitch-aware timbre windows and <1% windowed voltage refinement error. No WAV matrix is written.",
        "scope":"Conditional attribution within an uncalibrated reduction, not measured material identification. Changed dissipation also changes attack and modal participation; fractions are trajectory-specific, not additive causal sensitivities. Load resistance also changes electrical transfer. Absolute bank gain, EQ/noise reduction, gravity, static magnetic bias, full keyboard and realtime integration remain outside this study."});
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("loss budget retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_loss_observer_matches_dense_quadratic_and_rejects_unmodeled_coupling() {
        let p = profile(0.8, 0);
        let model = ElectromechanicalAssembly::new_at_rest(1e-6, p).unwrap();
        let c = model.structural_damping_matrix();
        let observer = LossObserver::new(c).unwrap();
        for k in 0..12 {
            let v = core::array::from_fn(|i| ((i * 7 + k * 11) as f64).sin());
            let dense = (0..18)
                .map(|i| (0..18).map(|j| v[i] * c[i][j] * v[j]).sum::<f64>())
                .sum::<f64>();
            let parts = observer.power(v);
            assert!(parts.iter().all(|x| *x >= 0.0));
            assert!((dense - parts.iter().sum::<f64>()).abs() < 1e-12 * dense);
        }
        let mut bad = c;
        bad[0][2] = 0.01;
        bad[2][0] = 0.01;
        assert!(LossObserver::new(bad).is_err());
        bad = c;
        bad[0][0] = -1.0;
        assert!(LossObserver::new(bad).is_err());
        bad = c;
        bad[0][0] = f64::NAN;
        assert!(LossObserver::new(bad).is_err());
    }
    #[test]
    fn loss_interventions_preserve_static_geometry_and_equilibrium() {
        let reference = ElectromechanicalAssembly::new_at_rest(1e-6, profile(0.8, 0))
            .unwrap()
            .probe();
        for case in 1..6 {
            let b = ElectromechanicalAssembly::new_at_rest(1e-6, profile(0.8, case))
                .unwrap()
                .probe();
            assert_eq!(reference, b);
        }
    }
}
