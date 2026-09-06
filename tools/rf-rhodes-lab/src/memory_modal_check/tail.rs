//! Longer trajectories with separately gated attack and tail sections.
use super::*;
use serde_json::Value;

pub const HELP: &str = "Stateful modal tail qualification:
  memory-modal-tail-check --output REPORT.json
Four strong 75/120 mm strikes, 1/10 ms material relaxation, 128 ms duration.
Default RK4, capped RK4 and a 1.000064 ns uniform implicit reference.
Whole-record and separate 0-8, 8-32, 32-64, 64-128 ms accuracy gates.
No audio device, calibration, new physical coefficients or timing claims.
";
const FRAMES: usize = 6144;
const SECTIONS: [(usize, usize); 4] = [(0, 384), (384, 1536), (1536, 3072), (3072, FRAMES)];
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
    if a.states.len() != FRAMES || b.states.len() != FRAMES {
        return Err("modal tail comparison requires 128 ms trajectories".into());
    }
    let whole = refinement::compare(a, b, speed)?;
    let mut sections = Vec::new();
    for (start, end) in SECTIONS {
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

fn study_case(length: f64, tau: f64) -> Result<Value, Box<dyn Error>> {
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
            },
        )?);
    }
    let mut comparisons = Vec::new();
    for (candidate, reference) in [(0, 1), (0, 2), (1, 2)] {
        let mut row = compare(&takes[candidate], &takes[reference], 0.8)?;
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
            cases.push(study_case(length, tau)?);
            println!("Completed 128 ms tail: length {length} m, relaxation {tau} s.");
        }
    }
    let passed = cases.iter().all(|x| x["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({
            "schema_version":1,"experiment":"memory-modal-tail-v1","status":if passed {"pass"} else {"fail"},
            "duration_seconds":0.128,"observation_rate_hz":48000,"calibrated":false,"plugin_integrated":false,
            "cases":cases,
            "gates":{"each_take_energy_and_port_work":1e-8,"each_take_positive_energy_step":1e-10,
                "whole_and_section_kinetic_velocity_rmse":0.01,"whole_and_section_pickup_velocity_rmse":0.01,
                "whole_and_section_mean_force_rmse":0.02},
            "protocol":"Four strong-strike cases extend the 75/120 mm, 1/10 ms relaxation profiles to 128 ms. Same zero-gap impact, core impulse at 2 ms, damper on at 4 ms and off at 6 ms, with no later events. Compare default RK4, RK4 capped at contact level 2/free level 8, and uniform implicit 20832 ticks per 48 kHz frame. Every take retains independent global work/energy, nonnegative heat/force, free recovery and reimpact checks. Each pair must pass whole-record and separate 0-8, 8-32, 32-64, 64-128 ms accuracy gates. All 2 ms windows use absolute times.",
            "scope":"Selected longer numerical trajectories, not whole-keyboard or physical calibration. Kinetic velocity error remains normalized by launch speed; pickup and mean-force errors use each section's reference power. Zero reference power yields null and passes only for exact zero difference. Peak and 2 ms window metrics remain diagnostic. Fine reference uses incremental midpoint but is not exact. No new impulses or action/repetition physics, no audio, realtime qualification or cost claim. Existing reports are never overwritten; completed gate failures retain the report and exit unsuccessfully."
        }),
    )?;
    writeln!(file)?;
    if !passed {
        return Err("modal tail qualification failed; see retained report".into());
    }
    println!(
        "Modal tail qualification passed: 4 cases, 12 takes. Report: {}",
        args[2]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
