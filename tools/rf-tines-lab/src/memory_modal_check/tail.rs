//! Longer trajectories with separately gated attack and tail sections.
use super::*;
use serde_json::Value;

pub const HELP: &str = "Stateful modal tail qualification:
  memory-modal-tail-check --output REPORT.json
  memory-modal-reimpact-check --output REPORT.json
Four strong 75/120 mm strikes, 1/10 ms material relaxation, 128 ms duration.
Default RK4, capped RK4 and a 1.000064 ns uniform implicit reference.
Whole-record and separate 0-8, 8-32, 32-64, 64-128 ms accuracy gates.
Reimpact check adds 0.008 Ns core impulses at 32/80 ms and damper changes at
40/56/96/112 ms, gating 0-8, 8-32, 32-80 and 80-128 ms separately.
No audio device, calibration, new physical coefficients or timing claims.
";
const FRAMES: usize = 6144;
const SECTIONS: [(usize, usize); 4] = [(0, 384), (384, 1536), (1536, 3072), (3072, FRAMES)];
const REIMPACT_SECTIONS: [(usize, usize); 4] =
    [(0, 384), (384, 1536), (1536, 3840), (3840, FRAMES)];
const LABELS: [&str; 3] = ["rk4_default", "rk4_contact_2_free_8", "uniform_20832"];

fn section(take: &Take, start: usize, end: usize) -> Result<Take, Box<dyn Error>> {
    if start >= end || end > take.states.len() || take.states.len() != take.forces.len() {
        return Err("invalid modal tail section".into());
    }
    Ok(Take {
        states: take.states[start..end].to_vec(),
        forces: take.forces[start..end].to_vec(),
        mass: take.mass,
        pass: take.pass,
        report: Value::Null,
    })
}

fn compare(a: &Take, b: &Take, speed: f64) -> Result<Value, Box<dyn Error>> {
    compare_sections(a, b, speed, SECTIONS)
}
fn compare_sections(
    a: &Take,
    b: &Take,
    speed: f64,
    intervals: [(usize, usize); 4],
) -> Result<Value, Box<dyn Error>> {
    if a.states.len() != FRAMES || b.states.len() != FRAMES {
        return Err("modal tail comparison requires 128 ms trajectories".into());
    }
    let whole = refinement::compare(a, b, speed)?;
    let mut sections = Vec::new();
    for (start, end) in intervals {
        let mut row =
            refinement::compare(&section(a, start, end)?, &section(b, start, end)?, speed)?;
        // Reused comparison windows are relative to the slice; retain absolute times.
        for window in row["two_ms_windows"].as_array_mut().unwrap() {
            for key in ["start_seconds", "end_seconds"] {
                window[key] = json!(window[key].as_f64().unwrap() + start as f64 / 48000.0);
            }
        }
        row["start_seconds"] = json!(start as f64 / 48000.0);
        row["end_seconds"] = json!(end as f64 / 48000.0);
        sections.push(row);
    }
    let passed = whole["passed"] == true && sections.iter().all(|x| x["passed"] == true);
    Ok(json!({"passed":passed,"whole_record":whole,"sections":sections}))
}

fn study_case(length: f64, tau: f64, late: bool) -> Result<Value, Box<dyn Error>> {
    let mut takes = Vec::new();
    for (substeps, mode, limits) in [
        (16672, ContactMode::Rk4, None),
        (16672, ContactMode::Rk4, Some((2, 8))),
        (20832, ContactMode::None, None),
    ] {
        takes.push(take_configured(
            length,
            0.8,
            tau,
            substeps,
            mode == ContactMode::Rk4,
            mode,
            TakeConfig {
                frames: FRAMES,
                rk4_limits: limits,
                late_reimpacts: late,
            },
        )?);
    }
    let mut comparisons = Vec::new();
    for (candidate, reference) in [(0, 1), (0, 2), (1, 2)] {
        let mut row = if late {
            compare_sections(&takes[candidate], &takes[reference], 0.8, REIMPACT_SECTIONS)?
        } else {
            compare(&takes[candidate], &takes[reference], 0.8)?
        };
        row["candidate"] = json!(LABELS[candidate]);
        row["reference"] = json!(LABELS[reference]);
        comparisons.push(row);
    }
    let passed = comparisons.iter().all(|x| x["passed"] == true);
    let reports: Vec<_> = takes
        .into_iter()
        .zip(LABELS)
        .map(|(take, label)| json!({"path":label,"report":take.report}))
        .collect();
    Ok(
        json!({"tine_length_m":length,"launch_speed_m_s":0.8,"relaxation_seconds":tau,
        "passed":passed,"takes":reports,"comparisons":comparisons}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let late = args[0] == "memory-modal-reimpact-check";
    let label = if late { "reimpact" } else { "tail" };
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
            cases.push(study_case(length, tau, late)?);
            println!("Completed 128 ms {label}: length {length} m, relaxation {tau} s.");
        }
    }
    let passed = cases.iter().all(|x| x["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({
            "schema_version":1,"experiment":if late {"memory-modal-reimpact-v1"} else {"memory-modal-tail-v1"},"status":if passed {"pass"} else {"fail"},
            "duration_seconds":0.128,"observation_rate_hz":48000,"calibrated":false,"plugin_integrated":false,
            "cases":cases,
            "gates":{"each_take_energy_and_port_work":1e-8,"each_take_positive_energy_step":1e-10,
                "whole_and_section_kinetic_velocity_rmse":0.01,"whole_and_section_pickup_velocity_rmse":0.01,
                "whole_and_section_mean_force_rmse":0.02},
            "protocol":if late {"Four strong 75/120 mm, 1/10 ms relaxation profiles through 128 ms. Preserve the initial zero-gap impact, 2 ms core impulse and 4/6 ms damper events; add 0.008 Ns core impulses at 32/80 ms and damper on/off at 40/56 and 96/112 ms. No position, velocity or material reset. Every take must observe separation followed by positive contact force within both 32-80 and 80-128 ms. Compare default RK4, contact-2/free-8 capped RK4 and uniform implicit 20832 ticks per 48 kHz frame. Retain all energy/work/heat/force gates and whole-record plus separate 0-8, 8-32, 32-80, 80-128 ms accuracy gates. Events occur before their observation frame; no adaptive step crosses a frame or event. All 2 ms windows use absolute times."} else {"Four strong-strike cases extend the 75/120 mm, 1/10 ms relaxation profiles to 128 ms. Same zero-gap impact, core impulse at 2 ms, damper on at 4 ms and off at 6 ms, with no later events. Compare default RK4, RK4 capped at contact level 2/free level 8, and uniform implicit 20832 ticks per 48 kHz frame. Every take retains independent global work/energy, nonnegative heat/force, free recovery and reimpact checks. Each pair must pass whole-record and separate 0-8, 8-32, 32-64, 64-128 ms accuracy gates. All 2 ms windows use absolute times."},
            "scope":if late {"Selected repeated-excitation numerical qualification. Prescribed external core impulses are not a key/action/repetition model; the freely retreating hammer has no backcheck or return mechanism. No calibrated realism, pickup voltage, audio or realtime cost claim. Kinetic velocity error is normalized by initial launch speed; pickup and force errors use reference section power, with null for zero power and passage only for zero difference. Peak and 2 ms windows are diagnostic. The fine midpoint reference is approximate. Existing reports are never overwritten; completed gate failures retain the report and exit unsuccessfully."} else {"Selected longer numerical trajectories, not whole-keyboard or physical calibration. Kinetic velocity error remains normalized by launch speed; pickup and mean-force errors use each section's reference power. Zero reference power yields null and passes only for exact zero difference. Peak and 2 ms window metrics remain diagnostic. Fine reference uses incremental midpoint but is not exact. No new impulses or action/repetition physics, no audio, realtime qualification or cost claim. Existing reports are never overwritten; completed gate failures retain the report and exit unsuccessfully."}
        }),
    )?;
    writeln!(file)?;
    if !passed {
        return Err(format!("modal {label} qualification failed; see retained report").into());
    }
    println!(
        "Modal {label} qualification passed: 4 cases, 12 takes. Report: {}",
        args[2]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_reimpact_requires_separation_in_each_epoch_and_keeps_first_contact() {
        let v = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let separated = v.probe();
        let mut compressed = separated;
        compressed.hammer.surface_energy_j = 1e-8;
        compressed.hammer.contact_force_n = 1.0;
        let mut observed = LateReimpacts::default();
        observed.observe(0, 0.0, separated);
        observed.observe(1536, 0.033, compressed);
        assert_eq!(observed.contacts, [None; 2]);
        observed.observe(1537, 0.034, separated);
        observed.observe(1538, 0.035, compressed);
        observed.observe(1539, 0.036, compressed);
        assert_eq!(observed.contacts, [Some(0.035), None]);
        observed.observe(3840, 0.0801, compressed);
        assert_eq!(
            observed.contacts[1], None,
            "separation must belong to this epoch"
        );
        observed.observe(3841, 0.0802, separated);
        observed.observe(3842, 0.0803, compressed);
        assert_eq!(observed.contacts, [Some(0.035), Some(0.0803)]);
    }
    #[test]
    fn repeated_event_protocol_preserves_the_original_prefix_and_bounds() {
        let mut frames = Vec::new();
        for frame in 0..FRAMES {
            if frame < 1536 {
                assert_eq!(
                    diagnostic_event(frame, 0.8, true),
                    diagnostic_event(frame, 0.8, false)
                );
            } else {
                assert_eq!(diagnostic_event(frame, 0.8, false), None);
            }
            if diagnostic_event(frame, 0.8, true).is_some() {
                frames.push(frame);
            }
        }
        assert_eq!(frames, [96, 192, 288, 1536, 1920, 2688, 3840, 4608, 5376]);
        for pair in REIMPACT_SECTIONS.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
        assert_eq!(REIMPACT_SECTIONS[0].0, 0);
        assert_eq!(REIMPACT_SECTIONS[3].1, FRAMES);
        assert!(
            take_configured(
                0.075,
                0.8,
                0.001,
                16672,
                true,
                ContactMode::Rk4,
                TakeConfig {
                    late_reimpacts: true,
                    ..TakeConfig::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn tail_gate_detects_error_hidden_by_whole_record_and_preserves_absolute_windows() {
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
        let mut a = section(&b, 0, FRAMES).unwrap();
        for q in &mut a.states[3072..] {
            q.hammer.core_velocity_m_s += 0.8 * 0.014;
        }
        let result = compare(&a, &b, 0.8).unwrap();
        assert_eq!(result["whole_record"]["passed"], true);
        assert_eq!(result["passed"], false);
        assert_eq!(result["sections"][3]["passed"], false);
        assert_eq!(result["sections"][2]["passed"], true);
        assert_eq!(
            result["sections"][3]["two_ms_windows"][0]["start_seconds"],
            0.064
        );
        assert_eq!(SECTIONS[0].0, 0);
        for pair in SECTIONS.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
        assert_eq!(SECTIONS[3].1, FRAMES);
        assert!(section(&a, 1, 1).is_err());
        assert!(section(&a, 0, FRAMES + 1).is_err());
        assert!(compare(&section(&a, 0, 384).unwrap(), &b, 0.8).is_err());
    }
}
