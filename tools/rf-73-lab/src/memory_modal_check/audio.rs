//! Selected-strike offline audio; shared-trajectory sampling and a finer solve.
use super::*;
use crate::convergence::OfflineFir;
use rf_73_dsp::MagneticPickup;
use serde_json::Value;
use std::{collections::BTreeSet, io::BufWriter, path::PathBuf};

pub const HELP: &str = "New physical-assembly audio preview:
  render-memory-modal --output PATH.wav [--seconds 1.5] [--hold 0.9]
    [--speed 0.4] [--length-mm 75] [--gain 0.084]
48 kHz mono; one strike, damper release, default uncalibrated geometry/material.
Duration 0.25..3 s; hold >=0.1 s and >=0.1 s release; speed 0.2..0.8 m/s;
length 75 or 120 mm; fixed output gain 0.001..10. No inferred MIDI pitch.
16/32/64x frozen-trajectory pickup sampling and a finer, capped solve are checked.
Writes PATH.json and a 64x-filtered WAV only when all preview gates pass.
No normalization/clipping, repeated-strike qualification or device/host launch.
";
pub(super) const RATE: usize = 48000;
pub(super) const OBS: usize = 64;
pub(super) const FACTORS: [usize; 3] = [16, 32, OBS];

pub(super) struct Options {
    pub(super) output: PathBuf,
    pub(super) frames: usize,
    pub(super) release: usize,
    pub(super) speed: f64,
    pub(super) length: f64,
    pub(super) gain: f64,
}
impl Options {
    pub(super) fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut output = PathBuf::new();
        let (mut seconds, mut hold, mut speed, mut length, mut gain) =
            (1.5_f64, 0.9_f64, 0.4_f64, 75.0_f64, 0.084_f64);
        let mut seen = BTreeSet::new();
        let (pairs, remainder) = args[1..].as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(HELP.into());
        }
        for [key, value] in pairs {
            if !seen.insert(key) {
                return Err("duplicate modal audio option".into());
            }
            match key.as_str() {
                "--output" => output = value.into(),
                "--seconds" => seconds = value.parse()?,
                "--hold" => hold = value.parse()?,
                "--speed" => speed = value.parse()?,
                "--length-mm" => length = value.parse()?,
                "--gain" => gain = value.parse()?,
                _ => return Err(HELP.into()),
            }
        }
        if output.extension().is_none_or(|s| s != "wav")
            || ![seconds, hold, speed, length, gain]
                .iter()
                .all(|x| x.is_finite())
            || !(0.25..=3.0).contains(&seconds)
            || hold < 0.1
            || seconds - hold < 0.1 - 1e-12
            || !(0.2..=0.8).contains(&speed)
            || ![75.0, 120.0].contains(&length)
            || !(0.001..=10.0).contains(&gain)
        {
            return Err(HELP.into());
        }
        Ok(Self {
            output,
            frames: (seconds * RATE as f64).round() as usize,
            release: (hold * RATE as f64).round() as usize,
            speed,
            length: length * 0.001,
            gain,
        })
    }
}

pub(super) struct Render {
    pub(super) take: Take,
    pub(super) signals: Vec<Vec<f64>>,
}

pub(super) fn render(o: &Options, refined: bool) -> Result<Render, Box<dyn Error>> {
    let ticks = if refined { 16384 } else { 8192 };
    let h = 1.0 / (RATE * ticks) as f64;
    let mut v = MemoryModalAssembly::new(
        h,
        TineGeometry {
            length_m: o.length,
            ..TineGeometry::default()
        },
        ModalAssemblyProfile::default(),
        MemoryHammerProfile::default(),
        0.0,
        o.speed,
    )?;
    let initial = v.probe();
    v.prepare_free_steps(12)?;
    v.prepare_rk4_contact()?;
    let mut c = if refined {
        Controller::with_rk4_limits(2, 5)?
    } else {
        Controller::with_rk4_contact()
    };
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
    let mut residuals = [0.0_f64; 3];
    let mut positive = 0.0_f64;
    let mut displacement_peak = 0.0_f64;
    let mut velocity_peak = 0.0_f64;
    let mut saw_contact = false;
    let mut separation = None;
    let mut first_contact = Value::Null;
    let mut last_contact = Value::Null;
    let mut pending_separation = false;
    let mut force_was_positive = false;
    let mut force_episodes = 0_u64;
    let mut force_positive_seconds = 0.0;
    let mut impulse = 0.0;
    let mut peak_mean_force = 0.0_f64;
    let mut pass = true;
    for frame in 0..o.frames {
        if frame == o.release {
            let before = v.probe();
            v.set_damped(true);
            if v.probe() != before {
                return Err("damper changed physical state at event".into());
            }
        }
        let mut frame_force = 0.0;
        for sample in 1..=OBS {
            let mut remaining = ticks / OBS;
            while remaining > 0 {
                let before = v.probe();
                let n = c.advance(&mut v, remaining)?;
                remaining -= n;
                let q = v.probe();
                let scale = initial.hammer.initial_energy_j;
                for (max, r) in residuals.iter_mut().zip([
                    q.balance_residual_j,
                    q.structural_work_residual_j,
                    q.hammer.balance_residual_j,
                ]) {
                    pass &= r.is_finite();
                    *max = max.max(r.abs() / scale);
                }
                let growth = (q.mechanical_energy_j - before.mechanical_energy_j) / scale;
                pass &= growth.is_finite()
                    && q.hammer.material.last_step_heat_j.is_finite()
                    && q.hammer.material.last_step_heat_j >= 0.0
                    && q.structural_heat_j.is_finite()
                    && q.structural_heat_j >= before.structural_heat_j
                    && q.hammer.surface_energy_j.is_finite()
                    && q.hammer.surface_energy_j >= 0.0;
                positive = positive.max(growth);
                let force = c.mean_force().unwrap_or(q.hammer.contact_force_n);
                pass &= force.is_finite() && force >= 0.0;
                frame_force += force * n as f64 / ticks as f64;
                impulse += force * n as f64 * h;
                peak_mean_force = peak_mean_force.max(force);
                if force > 0.0 {
                    force_episodes += u64::from(!force_was_positive);
                    force_positive_seconds += n as f64 * h;
                    pending_separation = true;
                }
                force_was_positive = force > 0.0;
                saw_contact |= force > 0.0;
                if pending_separation && force == 0.0 && q.hammer.surface_energy_j == 0.0 {
                    pending_separation = false;
                    let time = Some(
                        frame as f64 / RATE as f64
                            + ((sample * ticks / OBS) - remaining) as f64 * h,
                    );
                    let hammer = MemoryHammerProfile::default();
                    let com_velocity = (hammer.core_mass_kg * q.hammer.core_velocity_m_s
                        + hammer.tip_mass_kg * q.hammer.tip_velocity_m_s)
                        / (hammer.core_mass_kg + hammer.tip_mass_kg);
                    last_contact = json!({"separation_seconds":time,
                        "impulse_n_s":impulse,"peak_interval_mean_force_n":peak_mean_force,
                        "outgoing_hammer_com_velocity_m_s":com_velocity,
                        "structural_energy_j":q.structural_energy_j,
                        "structural_heat_j":q.structural_heat_j,
                        "contact_material_heat_j":q.hammer.material.dissipated_energy_j,
                        "hammer_energy_j":q.hammer.mechanical_energy_j});
                    if separation.is_none() {
                        separation = time;
                        first_contact = last_contact.clone();
                    }
                }
            }
            let q = v.probe();
            displacement_peak = displacement_peak.max(q.pickup_displacement_m.abs());
            velocity_peak = velocity_peak.max(q.pickup_velocity_m_s.abs());
            let voltage = pickup.voltage(q.pickup_displacement_m, q.pickup_velocity_m_s);
            if !voltage.is_finite() {
                return Err("non-finite modal pickup voltage".into());
            }
            for (i, &factor) in factors.iter().enumerate() {
                if sample % (OBS / factor) == 0 {
                    filters[i].push(voltage);
                }
            }
        }
        for (signal, filter) in signals.iter_mut().zip(&filters) {
            let y = filter.output() * o.gain;
            if !y.is_finite() {
                return Err("non-finite filtered modal output".into());
            }
            signal.push(y);
        }
        states.push(v.probe());
        forces.push(frame_force);
        if (frame + 1) % (RATE / 2) == 0 {
            println!(
                "{} trajectory: {:.1} s",
                if refined { "Refined" } else { "Primary" },
                (frame + 1) as f64 / RATE as f64
            );
        }
    }
    pass &= saw_contact
        && separation.is_some()
        && residuals.iter().all(|r| *r < 1e-8)
        && positive < 1e-10;
    Ok(Render {
        signals,
        take: Take {
            states,
            forces,
            mass: v.mass_matrix(),
            pass,
            report: json!({"passed":pass,"base_ticks_per_output_frame":ticks,"base_step_seconds":h,
            "initial_state":state(initial),"final_state":state(v.probe()),"first_separation_seconds":separation,
            "first_contact":first_contact,"total_contact_impulse_n_s":impulse,
            "last_observed_separation":last_contact,"force_positive_episode_count":force_episodes,
            "force_positive_intervals_seconds":force_positive_seconds,
            "peak_interval_mean_force_n":peak_mean_force,
            "maximum_relative_energy_and_port_residuals":residuals,"maximum_positive_relative_energy_step":positive,
            "pickup_peak_displacement_m":displacement_peak,"pickup_peak_velocity_m_s":velocity_peak,
            "controller":c.report(h)}),
        },
    })
}

fn audio_error(a: &[f64], b: &[f64]) -> Value {
    if a.is_empty() || a.len() != b.len() || !a.iter().chain(b).all(|v| v.is_finite()) {
        return json!({"passed":false,"relative_rmse":null,"reason":"invalid or empty signals"});
    }
    let power: f64 = b.iter().map(|v| v * v).sum();
    let error: f64 = a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum();
    let rmse = (power > 1e-24).then(|| (error / power).sqrt());
    json!({"passed":rmse.is_some_and(|r| r.is_finite() && r<0.01),"relative_rmse":rmse,
        "reference_rms":(power/b.len() as f64).sqrt(),"difference_rms":(error/a.len() as f64).sqrt()})
}
pub(super) fn audio_sections(a: &[f64], b: &[f64], release: usize) -> Value {
    let ranges = [
        ("whole", 0, a.len()),
        ("attack", 0, 1536),
        ("body", 1536, release),
        ("release", release, a.len()),
    ];
    let rows: Vec<_> = ranges
        .into_iter()
        .map(|(name, start, end)| {
            let mut row = audio_error(&a[start..end], &b[start..end]);
            row["section"] = json!(name);
            row["start_seconds"] = json!(start as f64 / RATE as f64);
            row["end_seconds"] = json!(end as f64 / RATE as f64);
            row
        })
        .collect();
    json!({"passed":rows.iter().all(|r| r["passed"]==true),"sections":rows})
}
pub(super) fn mechanics_sections(a: &Take, b: &Take, speed: f64) -> Result<Value, Box<dyn Error>> {
    let mut whole = refinement::compare(a, b, speed)?;
    whole.as_object_mut().unwrap().remove("two_ms_windows");
    let mut sections = Vec::new();
    for start in (0..a.states.len()).step_by(96) {
        let end = (start + 96).min(a.states.len());
        let slice = |t: &Take| Take {
            states: t.states[start..end].to_vec(),
            forces: t.forces[start..end].to_vec(),
            mass: t.mass,
            pass: t.pass,
            report: Value::Null,
        };
        let mut row = refinement::compare(&slice(a), &slice(b), speed)?;
        row.as_object_mut().unwrap().remove("two_ms_windows");
        row["start_seconds"] = json!(start as f64 / RATE as f64);
        row["end_seconds"] = json!(end as f64 / RATE as f64);
        sections.push(row);
    }
    Ok(
        json!({"passed":whole["passed"]==true && sections.iter().all(|s| s["passed"]==true),
        "whole":whole,"two_ms_sections":sections}),
    )
}

pub(super) fn configuration(o: &Options) -> Value {
    let g = TineGeometry {
        length_m: o.length,
        ..TineGeometry::default()
    };
    let p = ModalAssemblyProfile::default();
    let h = MemoryHammerProfile::default();
    json!({"geometry":{"length_m":g.length_m,"diameter_m":g.diameter_m,"young_modulus_pa":g.young_modulus_pa,
        "density_kg_m3":g.density_kg_m3,"tuning_mass_kg":g.tuning_mass_kg,"tuning_position":g.tuning_position,
        "hammer_position":g.hammer_position,"pickup_position":g.pickup_position},
        "structure":{"support_mass_kg":p.support_mass_kg,"support_inertia_kg_m2":p.support_inertia_kg_m2,
        "translation_stiffness_n_m":p.translation_stiffness_n_m,"rotation_stiffness_n_m_rad":p.rotation_stiffness_n_m_rad,
        "translation_damping_n_s_m":p.translation_damping_n_s_m,"rotation_damping_n_m_s_rad":p.rotation_damping_n_m_s_rad,
        "tonebar_mass_kg":p.tonebar_mass_kg,"tonebar_arm_m":p.tonebar_arm_m,"tonebar_frequency_hz":p.tonebar_frequency_hz,
        "tonebar_decay_seconds":p.tonebar_decay_seconds,"tine_decay_seconds":p.tine_decay_seconds,
        "damper_position":p.damper_position,"damper_n_s_m":p.damper_n_s_m},
        "hammer":{"core_mass_kg":h.core_mass_kg,"tip_mass_kg":h.tip_mass_kg,"surface_stiffness_n_m2":h.surface_stiffness_n_m2,
        "equilibrium_stiffness_n_m":h.material.equilibrium_stiffness_n_m,"equilibrium_cubic_n_m2":h.material.equilibrium_cubic_n_m2,
        "memory_stiffness_n_m":h.material.memory_stiffness_n_m,"relaxation_seconds":h.material.relaxation_seconds,
        "initial_gap_m":0.0,"launch_speed_m_s":o.speed},"pickup_gap_m":0.0015,"pickup_offset_m":0.0005,
        "fixed_output_gain":o.gain})
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let o = Options::parse(args)?;
    let report_path = o.output.with_extension("json");
    for path in [&o.output, &report_path] {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }
    let primary = render(&o, false)?;
    let refined = render(&o, true)?;
    let audio = &primary.signals[2];
    let sampling: Vec<_> = [0,1].into_iter().map(|i| json!({"candidate_oversampling":FACTORS[i],
        "reference_oversampling":OBS,"comparison":audio_sections(&primary.signals[i],audio,o.release)})).collect();
    let integration = audio_sections(audio, &refined.signals[0], o.release);
    let mechanics = mechanics_sections(&primary.take, &refined.take, o.speed)?;
    let peak = audio.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    let rms = (audio.iter().map(|v| v * v).sum::<f64>() / audio.len() as f64).sqrt();
    let ceiling = 10.0_f64.powf(-1.0 / 20.0);
    let pass = mechanics["passed"] == true
        && integration["passed"] == true
        && sampling.iter().all(|r| r["comparison"]["passed"] == true)
        && peak <= ceiling
        && rms > 1e-8;
    let report = json!({"schema_version":1,"experiment":"memory-modal-audio-v1",
        "status":if pass {"pass"} else {"fail"},"preview_gates_passed":pass,"wav":o.output,
        "sample_rate_hz":RATE,"frames":o.frames,"damper_frame":o.release,"configuration":configuration(&o),
        "filter":"Blackman FIR sampled from the production physical kernel; cutoff 0.42 output Fs, 31.5 output-sample support, 15.75-sample delay; zero initial history, no delay compensation or extra flush",
        "pickup":"Existing scalar production flux derivative; no magnetic loading or calibrated electrical output stage",
        "written_oversampling":OBS,"primary":primary.take.report,"refined":refined.take.report,
        "mechanical_refinement":mechanics,"frozen_trajectory_sampling":sampling,"audio_integration_refinement":integration,
        "levels":{"peak":peak,"rms":rms,"peak_dbfs":20.0*peak.log10(),"ceiling_dbfs":-1.0,"gain_normalization_applied":false},
        "gates":{"audio_relative_rmse_each_section":0.01,"energy_work_relative":1e-8,"positive_energy_step_relative":1e-10,
            "kinetic_and_pickup_velocity_rmse":0.01,"mean_force_relative_rmse":0.02},
        "scope":"Single uncalibrated strike, nine-coordinate structure and two-mass memory hammer. Original DSP tolerances preserved. Same-trajectory sampling comparisons and a twice-finer base grid with tighter contact/free caps are finite-resolution checks, not an exact solution or an absolute alias bound. No MIDI tuning, repeated-strike qualification, physical realism, polyphony, realtime performance or listening claim. WAV is f32, checks use f64; full failed reports retained, no WAV on failed preview gates."});
    crate::analysis::write_report(&report_path, &report)?;
    if !pass {
        return Err("modal audio preview gates failed; report retained, WAV not written".into());
    }
    let mut wav = crate::wav::FloatWav::new(
        BufWriter::new(crate::new_file(&o.output)?),
        RATE as u32,
        o.frames as u32,
    )?;
    for &sample in audio {
        wav.sample(sample as f32)?;
    }
    wav.finish()?;
    println!(
        "Physical assembly WAV: {} (peak {:.6}, RMS {:.6})",
        o.output.display(),
        peak,
        rms
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_gates_preserve_gain_and_reject_silence_nonfinite_and_hidden_attack_error() {
        let b = vec![0.1; 12000];
        let mut a = b.clone();
        assert_eq!(audio_sections(&a, &b, 7200)["passed"], true);
        a[..1536].fill(0.102);
        assert_eq!(audio_error(&a, &b)["passed"], true);
        assert_eq!(audio_sections(&a, &b, 7200)["passed"], false);
        assert_eq!(audio_error(&[0.0; 10], &[0.0; 10])["passed"], false);
        assert_eq!(audio_error(&[f64::NAN], &[1.0])["passed"], false);
        assert_eq!(audio_error(&[0.2], &[0.1])["passed"], false);
    }
    #[test]
    fn options_bound_work_and_keep_release_and_provenance_explicit() {
        let parse =
            |s: &str| Options::parse(&s.split_whitespace().map(str::to_string).collect::<Vec<_>>());
        let o = parse("render-memory-modal --output a.wav").unwrap();
        assert_eq!((o.frames, o.release), (72000, 43200));
        for tail in [
            "--seconds NaN",
            "--seconds 4",
            "--hold 1.5",
            "--speed inf",
            "--length-mm 80",
            "--note 57",
            "--gain 0",
            "--seconds 1 --seconds 2",
            "--hold",
        ] {
            assert!(
                parse(&format!("render-memory-modal --output a.wav {tail}")).is_err(),
                "{tail}"
            );
        }
    }
}
