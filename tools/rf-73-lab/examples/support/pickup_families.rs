//! Controlled comparison of retained transfer laws with calibrated mechanics.
use super::robust::Merit;
use super::velocity_mapping::{assign, select};
use super::*;
use rf_73_dsp::{MagneticPickup, PickupLaw};
#[derive(Clone, Copy, Debug)]
enum Law {
    Production,
    PointPole,
}
impl Law {
    fn name(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::PointPole => "point-pole",
        }
    }
    fn voltage(self, pickup: &MagneticPickup, q: f64, v: f64) -> f64 {
        match self {
            Self::Production => pickup.voltage(q, v),
            Self::PointPole => pickup.research_point_pole_voltage(q, v),
        }
    }
}
fn evaluate(cases: &[Case], law: Law, p: Profile, detailed: bool) -> Result<(f64, Vec<Value>)> {
    let pickup = MagneticPickup::new(p.pickup_gap_m, p.pickup_offset_m)?;
    score_with(cases, p, detailed, |c, p, s| {
        render_using(c, p, s, |q| Ok(-law.voltage(&pickup, q, 1.0) / 15.0))
    })
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if args.len() != 2 {
        return Err(
            "usage: continuous_pickup_fit --families SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY".into(),
        );
    }
    let source = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    let training = [43, 47, 50, 55, 59, 64, 72];
    let reserved = [45, 53, 60, 67, 76, 86];
    let choices: [Vec<f64>; 4] =
        std::array::from_fn(|_| (2..=20).map(|i| f64::from(i) / 20.0).collect());
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training,"held_out_notes":reserved,
        "laws":["production","point-pole"],"gaps_m":[0.0005,0.001],"offset_m":0.00075,
        "maximum_hammer_speeds_m_s":[0.8,1.6],"velocity_exponent":1.4,
        "mechanics":"Calibrated decay and bar excitation for every candidate; only maximum strike speed differs",
        "mapping_grid":choices[0],"mappings_per_configuration":3876,"configurations":8,
        "selection":"Existing robust constraints versus original fixed Calibrated; same mapping freedom for both laws",
        "short_render_seconds":0.9,
        "onset_failure_policy":"Reject the entire configuration if any velocity lacks a sustained onset; do not select a partial grid",
        "held_out_policy":"Load only if a selected family passes development; freeze both selections first",
        "scope":"Bounded screen of two existing formulas and previously motivated geometries, not global optimization or physical strike identification"}),
    )?;
    let mut cases = load(source, &training)?;
    let (mse, base_rows) = score(&cases, baseline(), false)?;
    write(
        &out.join("development-fixed-baseline.json"),
        &json!({"harmonic_mse_db2":mse,"cases":base_rows}),
    )?;
    let mut selected = Vec::new();
    let mut summary = Vec::new();
    for law in [Law::Production, Law::PointPole] {
        let mut best = Merit {
            violation: f64::INFINITY,
            mean_ratio: f64::INFINITY,
        };
        let mut best_profile = baseline();
        let mut best_mapping = [0.25, 0.45, 0.65, 0.85];
        let mut best_id = String::new();
        for gap in [0.0005, 0.001] {
            for speed in [0.8, 1.6] {
                let p = Profile {
                    pickup_law: PickupLaw::Production,
                    pickup_gap_m: gap,
                    pickup_offset_m: 0.00075,
                    maximum_hammer_speed_m_s: speed,
                    ..baseline()
                };
                let id = format!(
                    "{}-gap-{}-speed-{}",
                    law.name(),
                    (gap * 1e6) as u32,
                    (speed * 100.0) as u32
                );
                let mut grid = Vec::new();
                let mut failure = None;
                for &v in &choices[0] {
                    assign(&mut cases, [v; 4]);
                    match evaluate(&cases, law, p, false) {
                        Ok((_, rows)) => grid.push(rows),
                        Err(error) if error.to_string() == "no sustained onset" => {
                            failure = Some(json!({"velocity":v,"reason":error.to_string()}));
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
                if let Some(failure) = failure {
                    let receipt = json!({"id":id,"law":law.name(),"profile":format!("{p:?}"),
                        "status":"not_evaluable","failure":failure,"completed_velocity_rows":grid.len(),
                        "evaluated_mappings":0,"reason":"The complete common velocity grid is required; no partial-grid selection"});
                    write(&out.join(format!("{id}.json")), &receipt)?;
                    summary.push(receipt);
                    println!("{id}: not evaluable ({failure})");
                    continue;
                }
                let (mapping, merit, assessment, rows, count) =
                    select(&grid, &choices, &base_rows)?;
                let compact:Vec<_>=grid.iter().enumerate().map(|(i,rows)|json!({"velocity":choices[0][i],
                "cases":rows.iter().map(|r|json!({"note":r["note"],"layer":r["layer"],"harmonic_mse_db2":r["harmonic_mse_db2"],"candidate_onset_seconds":r["candidate_onset_seconds"]})).collect::<Vec<_>>()})).collect();
                write(
                    &out.join(format!("{id}.json")),
                    &json!({"law":law.name(),"profile":format!("{p:?}"),
                "mapping":mapping,"assessment":assessment,"selected_cases":rows,"grid":compact,"evaluated_mappings":count}),
                )?;
                summary.push(json!({"id":id,"mapping":mapping,"assessment":assessment}));
                println!(
                    "{id}: violation {:.4}, mean ratio {:.4}",
                    merit.violation, merit.mean_ratio
                );
                if merit.better_than(best) {
                    best = merit;
                    best_profile = p;
                    best_mapping = mapping;
                    best_id = id;
                }
            }
        }
        if !best_id.is_empty() {
            selected.push((law, best_profile, best_mapping, best, best_id));
        }
    }
    let validate = selected.iter().any(|(_, _, _, m, _)| m.feasible());
    write(
        &out.join("decision.json"),
        &json!({"plugin_promoted":false,"held_out_loaded":validate,
        "selected":selected.iter().map(|(l,_,m,s,id)|json!({"law":l.name(),"id":id,"mapping":m,"development_feasible":s.feasible()})).collect::<Vec<_>>(),
        "configurations":summary,"reason":if validate {"Frozen family winners proceed to held-out sensitivity evaluation"} else {"No selected family passes development constraints"}}),
    )?;
    if !validate {
        return Ok(());
    }
    for (split, mut cases) in [("training", cases), ("held_out", load(source, &reserved)?)] {
        assign(&mut cases, [0.25, 0.45, 0.65, 0.85]);
        let (mse, rows) = score(&cases, baseline(), true)?;
        write(
            &out.join(format!("{split}-baseline.json")),
            &json!({"harmonic_mse_db2":mse,"cases":rows}),
        )?;
        for (law, p, mapping, _, _) in &selected {
            assign(&mut cases, *mapping);
            let (mse, rows) = evaluate(&cases, *law, *p, true)?;
            write(
                &out.join(format!("{split}-{}.json", law.name())),
                &json!({"harmonic_mse_db2":mse,"cases":rows}),
            )?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapter_matches_production_engine_without_level_compensation() {
        let p = Profile {
            pickup_law: PickupLaw::Production,
            pickup_gap_m: 0.0005,
            pickup_offset_m: 0.00075,
            ..baseline()
        };
        let c = Case {
            note: 55,
            layer: "f",
            velocity: 0.85,
            clip: AudioClip::from_samples(48000, vec![0.0]).unwrap(),
            onset: 0.0,
        };
        let pickup = MagneticPickup::new(p.pickup_gap_m, p.pickup_offset_m).unwrap();
        let clip = render_using(&c, p, 0.15, |q| {
            Ok(-Law::Production.voltage(&pickup, q, 1.0) / 15.0)
        })
        .unwrap();
        let mut engine = rf_73_dsp::Engine::new(48000.0, p).unwrap();
        engine.set_gain(0.1);
        engine.set_level_compensation(false);
        engine.reset();
        engine.note_on(0, 55, 0.85);
        let mut error = 0.0;
        let mut energy = 0.0;
        for &a in clip.samples() {
            let b = f64::from(engine.next_sample());
            error += (a - b).powi(2);
            energy += b * b;
        }
        assert!((error / energy).sqrt() < 1e-7);
        assert_eq!(engine.faults(), 0);
    }
    #[test]
    fn point_pole_matches_independent_flux_difference() {
        let pickup = MagneticPickup::new(0.001, 0.00075).unwrap();
        for q in [-0.001, 0.0, 0.001] {
            let flux = |x: f64| 0.015 * (1.0 + ((x + 0.00075) / 0.001).powi(2)).powf(-1.5);
            let dx = 1e-9;
            let expected = -(flux(q + dx) - flux(q - dx)) / (2.0 * dx);
            assert!((Law::PointPole.voltage(&pickup, q, 1.0) - expected).abs() < 1e-7);
        }
    }
    #[test]
    fn late_detected_onset_keeps_the_complete_body_window() {
        let delayed = |seconds: f64| {
            AudioClip::from_samples(
                48000,
                (0..(48000.0 * seconds) as usize)
                    .map(|i| {
                        let t = i as f64 / 48000.0 - 0.2;
                        if t < 0.0 {
                            0.0
                        } else {
                            let phase = TAU * 195.99771799087463 * t;
                            0.1 * (phase.sin()
                                + 0.2 * (2.0 * phase).sin()
                                + 0.1 * (3.0 * phase).sin()
                                + 0.05 * (4.0 * phase).sin())
                        }
                    })
                    .collect(),
            )
            .unwrap()
        };
        let case = Case {
            note: 55,
            layer: "f",
            velocity: 0.85,
            clip: delayed(1.0),
            onset: 0.2,
        };
        let (loss, rows) = score_with(&[case], baseline(), false, |_, _, seconds| {
            Ok(delayed(seconds))
        })
        .unwrap();
        assert!(loss < 1e-12);
        assert_eq!(rows[0]["candidate_onset_seconds"], 0.2);
        assert_eq!(rows[0]["harmonic_terms"], 6);
    }
}
