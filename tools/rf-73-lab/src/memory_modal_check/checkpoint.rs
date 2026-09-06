//! Restart competing integrators from one complete preimpact physical state.
use super::*;
use rf_73_dsp::MemoryModalCheckpoint;
use serde_json::Value;

pub const HELP: &str = "Late-impact checkpoint study:
  memory-modal-checkpoint-check --output REPORT.json
Four profiles, first and second late impacts, six 8 ms continuations each.
Shared default-RK4 donor checkpoint; independent contact/free caps and two
uniform midpoint grids. Fresh controllers; all physical history preserved.
Whole-record and separate 2 ms gates. Failed qualifications retain the report.
No physical calibration, audio device or realtime claim.
";
const BASE: usize = 16672;
const FRAMES: usize = 384;
type IntegrationPath = (&'static str, usize, bool, Option<(u32, u32)>);
pub(super) const PATHS: [IntegrationPath; 6] = [
    ("rk4_default", BASE, true, None),
    ("rk4_contact_2", BASE, true, Some((2, 12))),
    ("rk4_free_8", BASE, true, Some((12, 8))),
    ("rk4_contact_2_free_8", BASE, true, Some((2, 8))),
    ("uniform_16672", BASE, false, None),
    ("uniform_20832", 20832, false, None),
];
pub(super) const PAIRS: [(usize, usize); 8] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 5),
    (1, 5),
    (2, 5),
    (3, 5),
    (4, 5),
];

pub(super) struct Saved {
    pub frame: usize,
    pub checkpoint: MemoryModalCheckpoint,
    pub probe: MemoryModalProbe,
    pub first_contact_seconds: f64,
}
pub(super) fn event(voice: &mut MemoryModalAssembly, frame: usize) -> Result<(), Box<dyn Error>> {
    if let Some(event) = diagnostic_event(frame, 0.8, true) {
        let before = voice.probe();
        match event {
            DiagnosticEvent::Impulse(impulse) => voice.apply_core_impulse(impulse)?,
            DiagnosticEvent::Damper(damped) => voice.set_damped(damped),
        }
        let after = voice.probe();
        if before.hammer.material != after.hammer.material
            || (matches!(event, DiagnosticEvent::Damper(_)) && before != after)
        {
            return Err("checkpoint event changed preserved physical history".into());
        }
    }
    Ok(())
}
pub(super) fn donor(length: f64, tau: f64, at_impulse: bool) -> Result<Vec<Saved>, Box<dyn Error>> {
    let mut voice = MemoryModalAssembly::new(
        1.0 / (48000.0 * BASE as f64),
        TineGeometry {
            length_m: length,
            ..TineGeometry::default()
        },
        ModalAssemblyProfile::default(),
        MemoryHammerProfile {
            material: HammerMemoryProfile {
                relaxation_seconds: tau,
                ..HammerMemoryProfile::default()
            },
            ..MemoryHammerProfile::default()
        },
        0.0,
        0.8,
    )?;
    voice.prepare_free_steps(12)?;
    voice.prepare_rk4_contact()?;
    let mut controller = Controller::with_rk4_contact();
    let mut observed = LateReimpacts::default();
    let mut saved = Vec::new();
    let mut pending = None;
    for frame in 0..6144 {
        let index = usize::from(frame >= 3840);
        if if at_impulse {
            frame == 1536 || frame == 3840
        } else {
            frame >= 1536 && observed.contacts[index].is_none()
        } {
            pending = Some((frame, voice.checkpoint(), voice.probe()));
        }
        event(&mut voice, frame)?;
        let mut remaining = BASE;
        while remaining > 0 {
            remaining -= controller.advance(&mut voice, remaining)?;
            let q = voice.probe();
            observed.observe(
                frame,
                (frame as f64 + (BASE - remaining) as f64 / BASE as f64) / 48000.0,
                q,
            );
        }
        if let Some(first_contact_seconds) = observed.contacts[index]
            && let Some((frame, checkpoint, probe)) = pending.take()
        {
            if probe.hammer.surface_energy_j != 0.0 || probe.hammer.contact_force_n != 0.0 {
                return Err("donor checkpoint is not before a separated late impact".into());
            }
            saved.push(Saved {
                frame,
                checkpoint,
                probe,
                first_contact_seconds,
            });
        }
        if saved.len() == 2 {
            return Ok(saved);
        }
    }
    Err("donor did not expose both late impacts".into())
}

pub(super) fn continuation(
    saved: &Saved,
    substeps: usize,
    adaptive: bool,
    limits: Option<(u32, u32)>,
    frames: usize,
) -> Result<Take, Box<dyn Error>> {
    if !matches!(frames, 384 | 2304) || saved.frame + frames > 6144 {
        return Err("checkpoint continuation outside its bounded observation domain".into());
    }
    let h = 1.0 / (48000.0 * substeps as f64);
    let mut v = saved.checkpoint.restart(h)?;
    if v.probe() != saved.probe {
        return Err("checkpoint restart changed physical state".into());
    }
    let mut controller = if let Some((contact, free)) = limits {
        Controller::with_rk4_limits(contact, free)?
    } else {
        Controller::with_rk4_contact()
    };
    if adaptive {
        v.prepare_free_steps(12)?;
        v.prepare_rk4_contact()?;
    }
    if v.probe() != saved.probe {
        return Err("checkpoint preparation changed physical state".into());
    }
    let mut states = Vec::new();
    let mut forces = Vec::new();
    let mut residuals = [0.0_f64; 3];
    let mut positive = 0.0_f64;
    let mut first_contact = None;
    let mut first_contact_frame = None;
    let mut separated_after_contact = false;
    let mut pass = true;
    for frame in saved.frame..saved.frame + frames {
        event(&mut v, frame)?;
        let mut remaining = substeps;
        let mut force = 0.0;
        while remaining > 0 {
            let before = v.probe();
            let consumed = if adaptive {
                controller.advance(&mut v, remaining)?
            } else {
                v.tick()?;
                1
            };
            remaining -= consumed;
            let q = v.probe();
            let scale = q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j;
            for (maximum, value) in residuals.iter_mut().zip([
                q.balance_residual_j,
                q.structural_work_residual_j,
                q.hammer.balance_residual_j,
            ]) {
                pass &= value.is_finite();
                *maximum = maximum.max(value.abs() / scale);
            }
            positive = positive.max((q.mechanical_energy_j - before.mechanical_energy_j) / scale);
            pass &= q.hammer.contact_force_n.is_finite()
                && q.hammer.contact_force_n >= 0.0
                && q.hammer.material.last_step_heat_j >= 0.0
                && q.structural_heat_j >= before.structural_heat_j;
            if q.hammer.contact_force_n > 0.0 {
                first_contact_frame.get_or_insert(frame);
                first_contact.get_or_insert(
                    (frame as f64 + (substeps - remaining) as f64 / substeps as f64) / 48000.0,
                );
            } else if first_contact.is_some() && q.hammer.surface_energy_j == 0.0 {
                separated_after_contact = true;
            }
            force += (if adaptive {
                controller.mean_force()
            } else {
                None
            })
            .unwrap_or(q.hammer.contact_force_n)
                * consumed as f64
                / substeps as f64;
        }
        states.push(v.probe());
        forces.push(force);
    }
    pass &= residuals.iter().all(|x| *x < 1e-8)
        && positive < 1e-10
        && first_contact.is_some()
        && separated_after_contact;
    let mut take = Take {
        states,
        forces,
        mass: v.mass_matrix(),
        pass,
        report: json!({"passed":pass,"substeps":substeps,"initial_state":state(saved.probe),
            "final_state":state(v.probe()),"maximum_relative_energy_and_port_residuals":residuals,
            "maximum_positive_relative_energy_step":positive,"first_contact_seconds":first_contact,
            "separated_after_contact":separated_after_contact,
            "controller":if adaptive {controller.report(h)} else {Value::Null}}),
    };
    if frames != FRAMES {
        take.report["first_contact_frame"] = json!(first_contact_frame);
        take.report["observation_frames"] = json!(frames);
    }
    Ok(take)
}
fn comparison(a: &Take, b: &Take, start_frame: usize) -> Result<Value, Box<dyn Error>> {
    comparison_frames(a, b, start_frame, FRAMES)
}
pub(super) fn comparison_frames(
    a: &Take,
    b: &Take,
    start_frame: usize,
    frames: usize,
) -> Result<Value, Box<dyn Error>> {
    if !matches!(frames, 384 | 1536 | 2304)
        || a.states.len() != frames
        || b.states.len() != frames
        || a.forces.len() != frames
        || b.forces.len() != frames
    {
        return Err("checkpoint comparison requires complete bounded trajectories".into());
    }
    let whole = refinement::compare(a, b, 0.8)?;
    let mut sections = Vec::new();
    for start in (0..frames).step_by(96) {
        let slice = |t: &Take| Take {
            states: t.states[start..start + 96].to_vec(),
            forces: t.forces[start..start + 96].to_vec(),
            mass: t.mass,
            pass: t.pass,
            report: Value::Null,
        };
        let mut row = refinement::compare(&slice(a), &slice(b), 0.8)?;
        row["start_seconds"] = json!((start_frame + start) as f64 / 48000.0);
        row["end_seconds"] = json!((start_frame + start + 96) as f64 / 48000.0);
        row.as_object_mut().unwrap().remove("two_ms_windows");
        sections.push(row);
    }
    let passed = whole["passed"] == true && sections.iter().all(|s| s["passed"] == true);
    Ok(json!({"passed":passed,"whole_record":whole,"absolute_two_ms_sections":sections}))
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|x| x != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    for length in [0.075, 0.12] {
        for tau in [0.001, 0.01] {
            for (index, saved) in donor(length, tau, false)?.into_iter().enumerate() {
                let mut takes = Vec::new();
                for (_, substeps, adaptive, limits) in PATHS {
                    takes.push(continuation(&saved, substeps, adaptive, limits, FRAMES)?);
                }
                let mut pairs = Vec::new();
                for (a, b) in PAIRS {
                    let mut row = comparison(&takes[a], &takes[b], saved.frame)?;
                    row["candidate"] = json!(PATHS[a].0);
                    row["reference"] = json!(PATHS[b].0);
                    pairs.push(row);
                }
                let passed = pairs.iter().all(|p| p["passed"] == true);
                let reports: Vec<_> = takes
                    .into_iter()
                    .zip(PATHS)
                    .map(|(take, path)| json!({"path":path.0,"report":take.report}))
                    .collect();
                cases.push(json!({"tine_length_m":length,"relaxation_seconds":tau,"late_impact_index":index+1,
                "checkpoint_frame":saved.frame,"checkpoint_seconds":saved.frame as f64/48000.0,
                "donor_first_contact_seconds":saved.first_contact_seconds,"checkpoint_state":state(saved.probe),
                "passed":passed,"takes":reports,"comparisons":pairs}));
                println!(
                    "Completed shared checkpoint: length {length} m, relaxation {tau} s, late impact {}.",
                    index + 1
                );
            }
        }
    }
    let passed = cases.iter().all(|c| c["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-checkpoint-v1",
        "status":if passed {"pass"} else {"fail"},"cases":cases,"continuation_seconds":0.008,
        "gates":{"each_take_energy_and_port_work":1e-8,"positive_energy_step":1e-10,"whole_and_section_kinetic_velocity_rmse":0.01,"whole_and_section_pickup_velocity_rmse":0.01,"whole_and_section_mean_force_rmse":0.02},
        "protocol":"Default-RK4 repeated-excitation donor. Capture the observation-frame start immediately before the first positive-force interval in each late epoch. Restart six fresh integrators with exactly equal complete physical state, including damper/material/work/heat history. No serialized state injection or reset. Independently cap contact/free steps, and use two uniform midpoint grids. Continue 8 ms with absolute original event scheduling. Each continuation must contact then separate and pass energy/work/heat/force checks. Eight pairs per checkpoint must pass whole-record and each 2 ms section. Whole-record two_ms_windows use time relative to the checkpoint; absolute_two_ms_sections use donor time.",
        "scope":"Numerical localization from a shared approximate donor, not continuous-time truth. Fresh controller/bank preparation is intentional. Two fine midpoint grids do not prove convergence. Four profiles and two impacts each; no action/backcheck, calibrated realism, audio, realtime timing or plugin integration. Failed qualifications retain reports and return nonzero; existing files are preserved."}),
    )?;
    writeln!(file)?;
    if !passed {
        return Err(
            "shared-checkpoint trajectory qualification failed; see retained report".into(),
        );
    }
    println!("Shared-checkpoint qualification passed: 8 checkpoints, 48 continuations.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_sections_detect_hidden_errors_and_preserve_absolute_event_time() {
        let voice = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let b = Take {
            states: vec![voice.probe(); FRAMES],
            forces: vec![0.0; FRAMES],
            mass: voice.mass_matrix(),
            pass: true,
            report: Value::Null,
        };
        let mut a = Take {
            states: b.states.clone(),
            forces: b.forces.clone(),
            mass: b.mass,
            pass: true,
            report: Value::Null,
        };
        for q in &mut a.states[288..] {
            q.hammer.core_velocity_m_s += 0.8 * 0.015;
        }
        let row = comparison(&a, &b, 3000).unwrap();
        assert_eq!(row["whole_record"]["passed"], true);
        assert_eq!(row["passed"], false);
        assert_eq!(row["absolute_two_ms_sections"][3]["passed"], false);
        assert_eq!(
            row["absolute_two_ms_sections"][0]["start_seconds"],
            3000.0 / 48000.0
        );
        assert_eq!(
            row["absolute_two_ms_sections"][3]["end_seconds"],
            3384.0 / 48000.0
        );
        a.forces.pop();
        assert!(comparison(&a, &b, 3000).is_err());
    }
}
