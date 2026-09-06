//! Equal-launch single-strike comparison; fidelity is deliberately not a gate.
use super::audio::{self, FACTORS, OBS, Options, RATE};
use crate::convergence::OfflineFir;
use rf_rhodes_dsp::{
    MagneticPickup, MemoryHammerProfile, ModalAssembly, ModalAssemblyProfile, ModalIntegration,
    ModalProbe, TineGeometry,
};
use serde_json::{Value, json};
use std::{error::Error, io::BufWriter};

pub const HELP: &str = "Controlled hammer comparison:
  compare-modal-hammers --output PATH.wav [--seconds 1.5] [--hold 0.9]
    [--speed 0.4] [--length-mm 75] [--gain 0.084]
Same bounds as render-memory-modal. Equal 4 g total launch mass and speed.
Writes PATH.wav (memory), PATH-elastic.wav, PATH-rate.wav and PATH.json.
Each candidate must pass its own integration, sampling and headroom checks.
Cross-model differences are descriptive, not realism or equivalence gates.
Single-mass hammers exit at separation; the memory hammer remains active.
No normalization, device launch, calibration or repeated-strike claim.
";

struct Single {
    states: Vec<ModalProbe>,
    forces: Vec<f64>,
    signals: Vec<Vec<f64>>,
    mass: [[f64; 9]; 9],
    report: Value,
    pass: bool,
}

fn single(o: &Options, beta: f64, refined: bool) -> Result<Single, Box<dyn Error>> {
    let steps = if refined { 512 } else { 256 };
    let h = 1.0 / (RATE * steps) as f64;
    let p = ModalAssemblyProfile {
        maximum_hammer_speed_m_s: o.speed,
        contact_damping_s_m: beta,
        ..ModalAssemblyProfile::default()
    };
    let mut v = ModalAssembly::new(
        RATE as f64,
        TineGeometry {
            length_m: o.length,
            ..TineGeometry::default()
        },
        p,
        ModalIntegration::Midpoint {
            steps_per_sample: steps,
        },
    )?;
    if !v.strike(1.0) {
        return Err("single-mass launch rejected".into());
    }
    let initial = v.probe().injected_energy_j;
    let factors: &[usize] = if refined { &[OBS] } else { &FACTORS };
    let mut filters: Vec<_> = factors
        .iter()
        .map(|&n| OfflineFir::production_kernel(n))
        .collect();
    let pickup = MagneticPickup::new(0.0015, 0.0005)?;
    let mut signals: Vec<_> = factors
        .iter()
        .map(|_| Vec::with_capacity(o.frames))
        .collect();
    let mut states = Vec::with_capacity(o.frames);
    let mut forces = Vec::with_capacity(o.frames);
    let mut first_contact = Value::Null;
    let mut impulse = 0.0;
    let mut peak_force = 0.0_f64;
    let mut residual = 0.0_f64;
    let mut positive = 0.0_f64;
    let mut pass = true;
    let mut previous = v.probe();
    for frame in 0..o.frames {
        if frame == o.release {
            v.set_damped(true);
        }
        let mut mean_force = 0.0;
        for observation in 1..=OBS {
            for substep in 1..=steps / OBS {
                // After escape the hammer cannot contact again. Audit every contact
                // step and every observation; no unobserved step maximum is claimed.
                let contact = previous.contact_active;
                v.tick();
                if contact || substep == steps / OBS {
                    let q = v.probe();
                    let r = q.balance_residual_j / initial;
                    let growth = (q.mechanical_energy_j + q.escaped_hammer_energy_j
                        - previous.mechanical_energy_j
                        - previous.escaped_hammer_energy_j)
                        / initial;
                    pass &= [
                        r,
                        growth,
                        q.dissipated_energy_j,
                        q.contact_dissipated_energy_j,
                        q.contact_force_n,
                        q.pickup_velocity_m_s,
                        q.pickup_displacement_m,
                    ]
                    .iter()
                    .all(|x| x.is_finite())
                        && q.position.iter().chain(&q.velocity).all(|x| x.is_finite())
                        && q.dissipated_energy_j >= previous.dissipated_energy_j
                        && q.contact_dissipated_energy_j >= previous.contact_dissipated_energy_j
                        && q.contact_force_n >= 0.0;
                    residual = residual.max(r.abs());
                    positive = positive.max(growth);
                    if contact {
                        impulse += q.contact_force_n * h;
                        mean_force += q.contact_force_n / steps as f64;
                        peak_force = peak_force.max(q.contact_force_n);
                        if !q.contact_active {
                            let outgoing = o.speed - impulse / p.hammer_mass_kg;
                            pass &= (0.5 * p.hammer_mass_kg * outgoing * outgoing
                                - q.escaped_hammer_energy_j)
                                .abs()
                                / initial
                                < 1e-8;
                            first_contact = json!({"separation_seconds":frame as f64 / RATE as f64
                                + ((observation - 1) * steps / OBS + substep) as f64 * h,
                                "impulse_n_s":impulse,"peak_interval_mean_force_n":peak_force,
                                "outgoing_hammer_com_velocity_m_s":outgoing,
                                "structural_energy_j":q.mechanical_energy_j,
                                "structural_heat_j":q.dissipated_energy_j-q.contact_dissipated_energy_j,
                                "contact_material_heat_j":q.contact_dissipated_energy_j,
                                "hammer_energy_j":q.escaped_hammer_energy_j});
                        }
                    }
                    previous = q;
                }
            }
            let voltage =
                pickup.voltage(previous.pickup_displacement_m, previous.pickup_velocity_m_s);
            if !voltage.is_finite() {
                return Err("non-finite single-mass pickup".into());
            }
            for (i, &factor) in factors.iter().enumerate() {
                if observation % (OBS / factor) == 0 {
                    filters[i].push(voltage);
                }
            }
        }
        for (signal, filter) in signals.iter_mut().zip(&filters) {
            let y = filter.output() * o.gain;
            if !y.is_finite() {
                return Err("non-finite single-mass output".into());
            }
            signal.push(y);
        }
        states.push(previous);
        forces.push(mean_force);
        if (frame + 1) % (RATE / 2) == 0 {
            println!(
                "Single-mass beta {beta}, {steps}x: {:.1} s",
                (frame + 1) as f64 / RATE as f64
            );
        }
    }
    pass &= !first_contact.is_null() && impulse > 0.0 && residual < 1e-8 && positive < 1e-10;
    let q = previous;
    Ok(Single {
        states,
        forces,
        signals,
        mass: v.mass_matrix(),
        pass,
        report: json!({"passed":pass,"uniform_steps_per_output_frame":steps,"step_seconds":h,
            "initial_energy_j":initial,"first_contact":first_contact,
            "total_contact_impulse_n_s":impulse,"peak_interval_mean_force_n":peak_force,
            "maximum_relative_balance_residual":residual,"maximum_observed_positive_energy_increment":positive,
            "final":{"structural_energy_j":q.mechanical_energy_j,"total_heat_j":q.dissipated_energy_j,
                "contact_material_heat_j":q.contact_dissipated_energy_j,"escaped_hammer_energy_j":q.escaped_hammer_energy_j,
                "contact_limited_heat_j":q.contact_limited_heat_j,"contact_limit_steps":q.contact_limit_steps},
            "audit_cadence":"Every contact integration step, then every 64x observation after hammer escape"}),
    })
}

fn mechanical_refinement(a: &Single, b: &Single, speed: f64) -> Result<Value, Box<dyn Error>> {
    if a.mass != b.mass
        || a.states.is_empty()
        || a.states.len() != b.states.len()
        || a.forces.len() != a.states.len()
        || b.forces.len() != b.states.len()
    {
        return Err("incompatible single-mass trajectories".into());
    }
    let relative = |difference: f64, power: f64, limit: f64| {
        let error = (power > 0.0).then(|| (difference / power).sqrt());
        json!({"relative_rmse":error,"passed":difference.is_finite() && power.is_finite()
            && error.map_or(difference == 0.0, |r| r.is_finite() && r < limit)})
    };
    let mut sections = Vec::new();
    for start in (0..a.states.len()).step_by(96) {
        let end = (start + 96).min(a.states.len());
        let (mut energy, mut pickup_error, mut pickup_power, mut force_error, mut force_power) =
            (0.0, 0.0, 0.0, 0.0, 0.0);
        for i in start..end {
            let dv: [f64; 9] =
                core::array::from_fn(|j| a.states[i].velocity[j] - b.states[i].velocity[j]);
            let e: f64 = (0..9)
                .map(|j| (0..9).map(|k| dv[j] * a.mass[j][k] * dv[k]).sum::<f64>())
                .sum();
            if !e.is_finite() || e < 0.0 {
                return Err("invalid structural kinetic metric".into());
            }
            energy += e;
            pickup_error +=
                (a.states[i].pickup_velocity_m_s - b.states[i].pickup_velocity_m_s).powi(2);
            pickup_power += b.states[i].pickup_velocity_m_s.powi(2);
            force_error += (a.forces[i] - b.forces[i]).powi(2);
            force_power += b.forces[i].powi(2);
        }
        let kinetic = (energy / ((end - start) as f64 * 0.004 * speed * speed)).sqrt();
        let pickup = relative(pickup_error, pickup_power, 0.01);
        let force = relative(force_error, force_power, 0.02);
        sections.push(json!({"start_seconds":start as f64/RATE as f64,"end_seconds":end as f64/RATE as f64,
            "structural_kinetic_velocity_error_over_launch":kinetic,"pickup_velocity":pickup,"mean_force":force,
            "passed":kinetic.is_finite() && kinetic<0.01 && pickup["passed"]==true && force["passed"]==true}));
    }
    let contact_fields = [
        "separation_seconds",
        "impulse_n_s",
        "peak_interval_mean_force_n",
        "outgoing_hammer_com_velocity_m_s",
        "structural_energy_j",
        "contact_material_heat_j",
    ];
    let contact: Vec<_> = contact_fields.into_iter().map(|field| {
        let x = a.report["first_contact"][field].as_f64().unwrap_or(f64::NAN);
        let y = b.report["first_contact"][field].as_f64().unwrap_or(f64::NAN);
        let error = (x-y).abs();
        json!({"observable":field,"absolute_difference":error,"reference":y,
            "passed":x.is_finite() && y.is_finite() && if y==0.0 {error==0.0} else {error/y.abs()<0.02}})
    }).collect();
    Ok(
        json!({"passed":a.pass && b.pass && sections.iter().chain(&contact).all(|x|x["passed"]==true),
        "two_ms_sections":sections,"first_contact_refinement":contact,
        "scope":"Structural velocities, pickup, frame mean force and first-contact observables; no hidden hammer-state trajectory metric"}),
    )
}

fn levels(signal: &[f64]) -> Value {
    let peak = signal.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    let rms = (signal.iter().map(|x| x * x).sum::<f64>() / signal.len() as f64).sqrt();
    json!({"peak":peak,"rms":rms,"passed":signal.iter().all(|x|x.is_finite())
        && peak<=10.0_f64.powf(-1.0/20.0) && rms>1e-8})
}

// Direct DFT of the fixed 32 ms Hann-windowed attack. No padded-bin resolution
// claim, pitch assumption, level matching, or spectral fidelity threshold.
fn attack_spectrum(signal: &[f64]) -> Value {
    let n = 1536;
    let windowed: Vec<_> = signal[..n]
        .iter()
        .enumerate()
        .map(|(i, x)| x * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()))
        .collect();
    let mut power = [0.0; 4];
    for k in 0..=n / 2 {
        let (si, co) = (std::f64::consts::TAU * k as f64 / n as f64).sin_cos();
        let (mut c, mut s, mut real, mut imag) = (1.0, 0.0, 0.0, 0.0);
        for &x in &windowed {
            real += x * c;
            imag -= x * s;
            (c, s) = (c * co - s * si, s * co + c * si);
        }
        let frequency = k as f64 * RATE as f64 / n as f64;
        let band = if frequency < 1000.0 {
            0
        } else if frequency < 5000.0 {
            1
        } else if frequency < 20000.0 {
            2
        } else {
            3
        };
        power[band] += (real * real + imag * imag) * if k == 0 || k == n / 2 { 1.0 } else { 2.0 }
            / (n * n) as f64;
    }
    json!({"window":"periodic Hann, first 32 ms including common FIR delay","bin_spacing_hz":31.25,
        "band_edges_hz":[0,1000,5000,20000,24000],"windowed_mean_square_by_band":power})
}

fn describe(signal: &[f64]) -> Value {
    let envelope: Vec<_> = signal.chunks(480).enumerate().map(|(i,x)|json!({
        "start_seconds":i as f64*0.01,"rms":(x.iter().map(|v|v*v).sum::<f64>()/x.len() as f64).sqrt()})).collect();
    json!({"levels":levels(signal),"attack":attack_spectrum(signal),"ten_ms_envelope":envelope})
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let o = Options::parse(args).map_err(|e| format!("{e}\n{HELP}"))?;
    let hammer = MemoryHammerProfile::default();
    if hammer.core_mass_kg + hammer.tip_mass_kg != ModalAssemblyProfile::default().hammer_mass_kg
        || ModalAssemblyProfile::default().hammer_mass_kg != 0.004
        || ModalAssemblyProfile::default().contact_stiffness_n_m2 != 4e10
    {
        return Err(
            "default hammer parameters changed; requalify the declared comparison configuration"
                .into(),
        );
    }
    let stem = o
        .output
        .file_stem()
        .ok_or("missing output stem")?
        .to_str()
        .ok_or("output stem must be UTF-8")?;
    let paths = [
        o.output.clone(),
        o.output.with_file_name(format!("{stem}-elastic.wav")),
        o.output.with_file_name(format!("{stem}-rate.wav")),
    ];
    let report_path = o.output.with_extension("json");
    for path in paths.iter().chain([&report_path]) {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }
    println!("Rendering memory candidate and its refined reference");
    let memory = audio::render(&o, false)?;
    let finer = audio::render(&o, true)?;
    let memory_mechanics = audio::mechanics_sections(&memory.take, &finer.take, o.speed)?;
    let mut signals = vec![memory.signals[2].clone()];
    let qualify = |primary: &[Vec<f64>], reference: &[f64], mechanics: Value| {
        let integration = audio::audio_sections(&primary[2], reference, o.release);
        let sampling: Vec<_> = [0,1].into_iter().map(|i|json!({"candidate_factor":FACTORS[i],
            "reference_factor":OBS,"comparison":audio::audio_sections(&primary[i],&primary[2],o.release)})).collect();
        let level = levels(&primary[2]);
        json!({"passed":mechanics["passed"]==true && integration["passed"]==true && level["passed"]==true
            && sampling.iter().all(|s|s["comparison"]["passed"]==true),"mechanical_refinement":mechanics,
            "audio_refinement":integration,"frozen_trajectory_sampling":sampling,"levels":level})
    };
    let mut candidates = vec![json!({"model":"two-mass-memory","wav":paths[0],
        "boundary":"Hammer remains active after first separation; no subsequent external impulses",
        "qualification":qualify(&memory.signals,&finer.signals[0],memory_mechanics),
        "primary":memory.take.report,"refined":finer.take.report,"audio_observables":describe(&signals[0])})];
    for (i, (label, beta)) in [("elastic", 0.0), ("rate", 2.0)].into_iter().enumerate() {
        println!("Rendering {label} candidate and its refined reference");
        let primary = single(&o, beta, false)?;
        let reference = single(&o, beta, true)?;
        if primary.mass != memory.take.mass {
            return Err("candidate structural inertia differs".into());
        }
        let mechanics = mechanical_refinement(&primary, &reference, o.speed)?;
        candidates.push(json!({"model":label,"wav":paths[i+1],
            "boundary":"Single hammer removed at first separation; outgoing kinetic energy retained in escaped ledger",
            "hammer":{"mass_kg":0.004,"contact_stiffness_n_m2":4e10,"contact_damping_s_m":beta,
                "initial_gap_m":0.0,"launch_speed_m_s":o.speed},
            "qualification":qualify(&primary.signals,&reference.signals[0],mechanics),
            "primary":primary.report,"refined":reference.report,"audio_observables":describe(&primary.signals[2])}));
        signals.push(primary.signals[2].clone());
    }
    let differences: Vec<_> = [(0,1),(0,2),(1,2)].into_iter().map(|(a,b)| {
        let mut comparison = audio::audio_sections(&signals[a],&signals[b],o.release);
        comparison.as_object_mut().unwrap().remove("passed");
        for section in comparison["sections"].as_array_mut().unwrap() {
            section.as_object_mut().unwrap().remove("passed");
        }
        json!({"candidate":candidates[a]["model"],"reference":candidates[b]["model"],"audio_difference":comparison})
    }).collect();
    let pass = candidates
        .iter()
        .all(|c| c["qualification"]["passed"] == true);
    let report = json!({"schema_version":1,"experiment":"controlled-modal-hammers-v1","passed":pass,
        "sample_rate_hz":RATE,"frames":o.frames,"damper_frame":o.release,
        "common_configuration_and_memory_hammer":audio::configuration(&o),"launch_energy_j":0.5*0.004*o.speed*o.speed,
        "written_observation_factor":OBS,
        "output_chain":"Identical scalar production magnetic pickup, 64x Blackman FIR with cutoff 0.42 output Fs, 31.5-sample support and 15.75-sample delay; zero histories, no alignment, gain matching, clipping or extra flush",
        "candidates":candidates,"descriptive_cross_model_differences":differences,
        "scope":"Equal launch mass/energy and shared nine-coordinate resonator, pickup and gain. Coefficients are uncalibrated hypotheses, not matched compliance. Every model qualifies independently; cross-model difference has no pass/fail threshold or preferred winner. First-contact observables use the first accepted separated endpoint; force peaks are interval means. Outgoing COM velocity is not relative restitution against the moving tine. Single-mass hammer exit and persistent memory hammer are different action boundaries. No repeated-strike, physical-realism, electrical-loading or realtime claim. All reports use f64 before f32 WAV conversion. Failed qualification retains report and writes no WAVs."});
    crate::analysis::write_report(&report_path, &report)?;
    if !pass {
        return Err(
            "hammer comparison qualification failed; report retained, no WAVs written".into(),
        );
    }
    for (path, signal) in paths.iter().zip(&signals) {
        let mut wav = crate::wav::FloatWav::new(
            BufWriter::new(crate::new_file(path)?),
            RATE as u32,
            o.frames as u32,
        )?;
        for &x in signal {
            wav.sample(x as f32)?;
        }
        wav.finish()?;
        println!("Qualified hammer WAV: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attack_bands_preserve_windowed_power_and_gain() {
        let signal: Vec<_> = (0..1536)
            .map(|i| (std::f64::consts::TAU * 100.0 * i as f64 / 1536.0).sin())
            .collect();
        let report = attack_spectrum(&signal);
        let bands: Vec<_> = report["windowed_mean_square_by_band"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert!((bands.iter().sum::<f64>() - 0.1875).abs() < 1e-12);
        assert!((bands[1] - 0.1875).abs() < 1e-12);
        let scaled = attack_spectrum(&signal.iter().map(|x| 2.0 * x).collect::<Vec<_>>());
        assert!(
            (scaled["windowed_mean_square_by_band"][1].as_f64().unwrap() - 4.0 * bands[1]).abs()
                < 1e-12
        );
        assert_eq!(levels(&[f64::NAN])["passed"], false);
        assert_eq!(levels(&[0.0; 10])["passed"], false);
    }
}
