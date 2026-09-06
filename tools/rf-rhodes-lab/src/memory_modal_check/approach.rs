//! Shared impulse checkpoints separate free approach from later collisions.
use super::*;
use checkpoint::{PAIRS, PATHS};
use serde_json::Value;

pub const HELP: &str = "Late-impulse approach study:
  memory-modal-approach-check --output REPORT.json
Four profiles, common checkpoints before 32/80 ms impulses, six paths each.
48 ms continuations; original events and complete physical history retained.
Compare the common force-free prefix, whole record and separate 2 ms sections.
Missing contacts/prefixes remain unqualified; failures retain reports.
No audio, physical calibration, new tolerances or realtime timing.
";
const FRAMES: usize = 2304;

fn common_prefix(takes: &[Take], start_frame: usize) -> Result<Option<usize>, Box<dyn Error>> {
    let mut earliest = None;
    for take in takes {
        let Some(frame) = take.report["first_contact_frame"].as_u64() else {
            return Ok(None);
        };
        let frame = usize::try_from(frame)?;
        if frame < start_frame || frame >= start_frame + take.states.len() {
            return Err("contact frame outside its continuation".into());
        }
        earliest = Some(earliest.map_or(frame, |old: usize| old.min(frame)));
    }
    Ok(earliest.and_then(|frame| (frame > start_frame).then_some(frame - start_frame)))
}
fn prefix(take: &Take, frames: usize) -> Result<Take, Box<dyn Error>> {
    if frames == 0
        || frames > take.states.len()
        || take.states.len() != take.forces.len()
        || take.forces[..frames].iter().any(|x| *x != 0.0)
        || take.states[..frames]
            .iter()
            .any(|q| q.hammer.surface_energy_j != 0.0 || q.hammer.contact_force_n != 0.0)
    {
        return Err("approach prefix must contain only complete force-free observations".into());
    }
    // Prefix trajectory gates are independent of any later full-take failure.
    Ok(Take {
        states: take.states[..frames].to_vec(),
        forces: take.forces[..frames].to_vec(),
        mass: take.mass,
        pass: true,
        report: Value::Null,
    })
}
fn endpoint_difference(a: MemoryModalProbe, b: MemoryModalProbe) -> Value {
    json!({"core_position_m":a.hammer.core_position_m-b.hammer.core_position_m,
        "tip_position_m":a.hammer.tip_position_m-b.hammer.tip_position_m,
        "core_velocity_m_s":a.hammer.core_velocity_m_s-b.hammer.core_velocity_m_s,
        "tip_velocity_m_s":a.hammer.tip_velocity_m_s-b.hammer.tip_velocity_m_s,
        "material_displacement_m":a.hammer.material.displacement_m-b.hammer.material.displacement_m,
        "material_branch_extension_m":a.hammer.material.branch_extension_m-b.hammer.material.branch_extension_m,
        "pickup_velocity_m_s":a.pickup_velocity_m_s-b.pickup_velocity_m_s,
        "structural_position":core::array::from_fn::<_,9,_>(|i| a.position[i]-b.position[i]),
        "structural_velocity":core::array::from_fn::<_,9,_>(|i| a.velocity[i]-b.velocity[i])})
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
            for (index, saved) in checkpoint::donor(length, tau, true)?
                .into_iter()
                .enumerate()
            {
                let mut takes = Vec::new();
                for (_, substeps, adaptive, limits) in PATHS {
                    takes.push(checkpoint::continuation(
                        &saved, substeps, adaptive, limits, FRAMES,
                    )?);
                }
                let prefix_frames = common_prefix(&takes, saved.frame)?;
                let mut comparisons = Vec::new();
                for (a, b) in PAIRS {
                    let mut row =
                        checkpoint::comparison_frames(&takes[a], &takes[b], saved.frame, FRAMES)?;
                    row["candidate"] = json!(PATHS[a].0);
                    row["reference"] = json!(PATHS[b].0);
                    row["common_free_prefix"] = if let Some(frames) = prefix_frames {
                        let mut report = refinement::compare(
                            &prefix(&takes[a], frames)?,
                            &prefix(&takes[b], frames)?,
                            0.8,
                        )?;
                        report["start_seconds"] = json!(saved.frame as f64 / 48000.0);
                        report["end_seconds"] = json!((saved.frame + frames) as f64 / 48000.0);
                        report["endpoint_difference"] = endpoint_difference(
                            takes[a].states[frames - 1],
                            takes[b].states[frames - 1],
                        );
                        // These relative windows may end in a partial 2 ms segment.
                        report
                    } else {
                        Value::Null
                    };
                    comparisons.push(row);
                }
                let prefix_passed = comparisons
                    .iter()
                    .all(|r| r["common_free_prefix"]["passed"] == true);
                let passed = prefix_passed && comparisons.iter().all(|r| r["passed"] == true);
                let reports: Vec<_> = takes
                    .into_iter()
                    .zip(PATHS)
                    .map(|(take, path)| json!({"path":path.0,"report":take.report}))
                    .collect();
                cases.push(json!({"tine_length_m":length,"relaxation_seconds":tau,"late_impact_index":index+1,
                "checkpoint_frame":saved.frame,"checkpoint_seconds":saved.frame as f64/48000.0,
                "donor_first_contact_seconds":saved.first_contact_seconds,"checkpoint_state":state(saved.probe),
                "common_free_prefix_frames":prefix_frames,"common_free_prefix_passed":prefix_passed,
                "passed":passed,"takes":reports,"comparisons":comparisons}));
                println!(
                    "Completed impulse checkpoint: length {length} m, relaxation {tau} s, late impact {}.",
                    index + 1
                );
            }
        }
    }
    let passed = cases.iter().all(|c| c["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-approach-v1",
        "status":if passed {"pass"} else {"fail"},"cases":cases,"continuation_seconds":0.048,
        "gates":{"each_take_energy_and_port_work":1e-8,"positive_energy_step":1e-10,"whole_section_and_prefix_kinetic_velocity_rmse":0.01,"whole_section_and_prefix_pickup_velocity_rmse":0.01,"whole_section_and_prefix_mean_force_rmse":0.02},
        "protocol":"Default-RK4 repeated-excitation donor. Save complete physical state immediately before each 32/80 ms impulse. Restart six fresh integrators identically, apply the scheduled impulse exactly once and continue 48 ms with original absolute events. Contact/free caps and uniform grids match the local checkpoint study. Require contact then separation and all independent work/energy/heat/force gates. Compare the full trajectory and every 2 ms section. Additionally compare an identical prefix ending at the START of the earliest positive-force observation frame across ALL paths, excluding that contact-containing frame. Every prefix observation must have zero force and surface energy. Prefix trajectory gates exclude subsequent take failures; full-take audits remain required independently.",
        "scope":"Numerical localization from an approximate shared donor with fresh controllers. Each epoch has its own donor checkpoint; the second does not inherit a candidate's first-epoch endpoint. Prefix RMSE uses the original launch speed or reference pickup power; force-relative error is null for exact zero power. Prefix/whole two_ms_windows are relative to their starts; absolute_two_ms_sections use donor time. Structural difference coordinate 1 is radians/radians per second; other coordinates are meters/meters per second. No exact-reference, physical realism, action/backcheck, audio or realtime claim. Failed qualifications retain reports and return nonzero; existing files are preserved."}),
    )?;
    writeln!(file)?;
    if !passed {
        return Err("impulse-checkpoint approach qualification failed; see retained report".into());
    }
    println!("Impulse-checkpoint approach qualification passed: 8 checkpoints, 48 continuations.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_prefix_excludes_the_earliest_contact_frame_and_does_not_invent_missing_data() {
        let voice = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let make = |frame: Value| Take {
            states: vec![voice.probe(); 8],
            forces: vec![0.0; 8],
            mass: voice.mass_matrix(),
            pass: false,
            report: json!({"first_contact_frame":frame}),
        };
        let mut takes = [make(json!(103)), make(json!(106))];
        takes[0].forces[3] = 1.0;
        assert_eq!(common_prefix(&takes, 100).unwrap(), Some(3));
        assert!(
            prefix(&takes[0], 3).unwrap().pass,
            "later failure does not contaminate prefix trajectory gates"
        );
        assert!(prefix(&takes[0], 4).is_err());
        takes[1].report["first_contact_frame"] = Value::Null;
        assert_eq!(common_prefix(&takes, 100).unwrap(), None);
        takes[1].report["first_contact_frame"] = json!(100);
        assert_eq!(common_prefix(&takes, 100).unwrap(), None);
        takes[1].report["first_contact_frame"] = json!(99);
        assert!(common_prefix(&takes, 100).is_err());
        assert!(prefix(&takes[0], 0).is_err());
        assert!(prefix(&takes[0], 9).is_err());
    }
}
