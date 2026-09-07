//! Controlled excitation and transducer sensitivity on the conditional G3 preset.
use super::{losses, tuning};
use rf_73_analysis::{detect_timbre_onset, measure_timbre_profile, pitch_anchor};
use rf_73_dsp::ElectromechanicalProfile;
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const NAMES: [&str; 7] = [
    "baseline",
    "strike_10_percent",
    "strike_30_percent",
    "half_contact_stiffness",
    "double_contact_stiffness",
    "pickup_gap_1mm",
    "pickup_vertical_offset_1mm",
];
fn profile(position: f64, case: usize) -> ElectromechanicalProfile {
    let mut p = ElectromechanicalProfile::default();
    p.geometry.length_m = 0.07;
    p.geometry.tuning_position = position;
    p.assembly.tine_decay_seconds[0] = 30.0;
    match case {
        1 => p.geometry.hammer_position = 0.1,
        2 => p.geometry.hammer_position = 0.3,
        3 => p.assembly.contact_stiffness_n_m2 *= 0.5,
        4 => p.assembly.contact_stiffness_n_m2 *= 2.0,
        5 => p.pickup.gap_m = 0.001,
        6 => p.pickup.offset_xy_m[0] = 0.001,
        _ => {}
    }
    p
}
// Missing observations never reduce a score by silently dropping dimensions.
fn attack_error(source: &Value, candidate: &Value) -> Option<f64> {
    let mut square = 0.0;
    for i in 0..3 {
        let a = source["windows"][0]["band_db_relative_to_fundamental_band"][i].as_f64()?;
        let b = candidate["windows"][0]["band_db_relative_to_fundamental_band"][i].as_f64()?;
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        square += (a - b).powi(2);
    }
    Some((square / 3.0).sqrt())
}
fn comparison(source: &Value, candidate: &Value) -> Value {
    if candidate.is_null() {
        return json!({"available":false});
    }
    let mut worst = 0.0_f64;
    let mut count = 0;
    let windows: Vec<_> = (0..5)
        .map(|w| {
            let bands: [Option<f64>; 3] = core::array::from_fn(|i| {
                source["windows"][w]["band_db_relative_to_fundamental_band"][i]
                    .as_f64()
                    .zip(
                        candidate["windows"][w]["band_db_relative_to_fundamental_band"][i].as_f64(),
                    )
                    .map(|(a, b)| b - a)
            });
            for d in bands.into_iter().flatten() {
                worst = worst.max(d.abs());
                count += 1;
            }
            let level = source["windows"][w]["level_relative_to_body_db"]
                .as_f64()
                .zip(candidate["windows"][w]["level_relative_to_body_db"].as_f64())
                .map(|(a, b)| b - a);
            json!({"band_balance_difference_db":bands,"relative_level_difference_db":level})
        })
        .collect();
    json!({"available":true,"attack_band_rms_db":attack_error(source,candidate),"windows":windows,"available_band_comparisons":count,"maximum_band_difference_db":worst,
        "spectral_agreement":if worst>6.0 {Some(false)} else if count==15 {Some(true)} else {None},
        "sustain_within_3db":windows[3..].iter().all(|w|w["relative_level_difference_db"].as_f64().is_some_and(|d|d.abs()<=3.0))})
}
fn impact_convergence(a: &Value, b: &Value) -> Value {
    let rows: Vec<_> = ["impulse_n_s", "peak_force_n", "active_contact_seconds"]
        .into_iter()
        .map(|key| {
            let error = a[key]
                .as_f64()
                .zip(b[key].as_f64())
                .filter(|(a, b)| a.is_finite() && b.is_finite() && *b > 0.0)
                .map(|(a, b)| (a - b).abs() / b);
            json!({"quantity":key,"relative_error":error,"passed":error.is_some_and(|e|e<0.01)})
        })
        .collect();
    json!({"passed":rows.iter().all(|r|r["passed"]==true),"quantities":rows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err("loaded-voicing MANIFEST.json --output REPORT.json".into());
    }
    let output = Path::new(&args[3]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("voicing study requires a new JSON path".into());
    }
    let bank = crate::pitch_reference::verified_bank(Path::new(&args[1]))?;
    let mut sources = Vec::new();
    let mut training = Vec::new();
    for (id, role, clip) in &bank.takes {
        let anchor = pitch_anchor(clip, 55)?;
        let f = anchor.frequency_hz.ok_or("unqualified source pitch")?;
        let p = measure_timbre_profile(clip, detect_timbre_onset(clip)?, f)?;
        if !p.qualified {
            return Err("unqualified source timbre".into());
        }
        if role == "training" {
            training.push(f);
        }
        sources.push(json!({"id":id,"role":role,"pitch_anchor":anchor,"profile":p}));
    }
    let target = (training.iter().map(|f| f.ln()).sum::<f64>() / training.len() as f64).exp();
    let (position, fit) = tuning::fitted_position(target)?;
    let mut cases = Vec::new();
    let mut comparisons = Vec::new();
    let mut baseline = None;
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (case, name) in NAMES.iter().enumerate() {
            let p = profile(position, case);
            println!("Loaded voicing: {name}, 128 ticks");
            let a = losses::take_observed(p, 128, 1.5, true)?;
            println!("Loaded voicing: {name}, 256 ticks");
            let b = losses::take_observed(p, 256, 1.5, true)?;
            let convergence = losses::convergence(&a, &b);
            let impact = impact_convergence(&a.report["impact"], &b.report["impact"]);
            let passed = a.report["passed"] == true
                && b.report["passed"] == true
                && convergence["passed"] == true
                && impact["passed"] == true;
            for source in &sources {
                comparisons.push(json!({"case":name,"source_id":source["id"],"source_role":source["role"],"comparison":comparison(&source["profile"],&b.report["timbre"])}));
            }
            let relative = baseline
                .as_ref()
                .map(|base| comparison(base, &b.report["timbre"]));
            let attack: Vec<_> = sources
                .iter()
                .filter(|s| s["role"] == "training")
                .map(|s| attack_error(&s["profile"], &b.report["timbre"]))
                .collect();
            let training_attack = attack
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .map(|v| (v.iter().map(|e| e * e).sum::<f64>() / v.len() as f64).sqrt());
            println!(
                "Loaded voicing: {name}, qualified={passed}, training attack RMS {} dB",
                json!(training_attack)
            );
            if case == 0 {
                baseline = Some(b.report["timbre"].clone());
            }
            cases.push(json!({"name":name,"measurement_qualified":passed,"training_attack_rms_db":training_attack,"baseline_difference":relative,
                "takes":[a.report,b.report],"voltage_convergence":convergence,"impact_convergence":impact,
                "settings":{"hammer_position_from_root_mm":70.0*p.geometry.hammer_position,"contact_stiffness_n_m2":p.assembly.contact_stiffness_n_m2,
                    "pickup_gap_m":p.pickup.gap_m,"pickup_offset_xy_m":p.pickup.offset_xy_m,"first_tine_t60_seconds":30.0}}));
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let report = json!({"schema_version":1,"experiment":"loaded-voicing-v1","measurement_qualified":failure.is_none() && cases.len()==7 && cases.iter().all(|c|c["measurement_qualified"]==true),
        "failure_reason":failure,"reference_match_claimed":false,"physical_calibration_claimed":false,"manifest":bank.manifest,"sources":sources,"structural_fit":fit,"target_hz":target,
        "cases":cases,"comparisons":comparisons,
        "protocol":"Frozen seven-case, fourteen-take sensitivity study. Use conditional G3 loss preset: first tine basis T60 30 s, original support losses, fixed mode-tracked spring on 70 mm beam. At 1.5 m/s from stationary rest, compare baseline, hammer point 10/30% from root versus 20%, half/double quadratic hammer contact stiffness, pickup gap 1 mm versus 1.5 mm, and vertical pickup offset 1 mm versus 0.5 mm. Change one physical control family per case; no gain matching, EQ, loss retuning or source-layer velocity assignment. 1.8 seconds held, 128/256 ticks per 48 kHz frame, original midpoint averaging/FIR and fixed 0.1 FS/V. Reuse loss-budget energy/quiet/pitch/timbre gates and <1% windowed voltage RMSE. Record hammer force peak, integrated impulse, active contact duration and pre-contact speed; require first three quantities to converge within 1%. Preserve every source/case comparison, including missing bands. Fixed 0-64 ms post-onset three-band RMS versus training sources is descriptive only; any missing attack band withholds the score. Subsequent four windows retain spectral/level differences; 3 dB late-level and 6 dB spectral gates are unchanged. No candidate is selected or promoted and no WAV matrix is saved.",
        "scope":"Sensitivity of a provisional point-contact, spatial-flux reduction at one note and drive speed. Force duration is tick-resolved. Contact and pickup changes can alter trajectories through reciprocal forces; source EQ/noise reduction and unknown velocity labels limit physical inference. First-mode loss remains boundary-selected and ambiguous. No new material law, full keyboard, release/repetition or realtime integration claim."});
    crate::analysis::write_report(output, &report)?;
    if report["measurement_qualified"] != true {
        return Err("voicing study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_attack_bands_do_not_improve_scores() {
        let a = json!({"windows":[{"band_db_relative_to_fundamental_band":[1.0,2.0,3.0]}]});
        let mut b = a.clone();
        assert_eq!(attack_error(&a, &b), Some(0.0));
        b["windows"][0]["band_db_relative_to_fundamental_band"][2] = Value::Null;
        assert_eq!(attack_error(&a, &b), None);
    }
    #[test]
    fn physical_controls_preserve_free_modes_and_losses() {
        let base = profile(0.8, 0);
        let reference = rf_73_dsp::ModalSpectrum::<18>::prepare_polarized(
            base.geometry,
            base.assembly,
            base.polarization,
        )
        .unwrap();
        for case in 1..7 {
            let p = profile(0.8, case);
            assert_eq!(
                p.assembly.tine_decay_seconds,
                base.assembly.tine_decay_seconds
            );
            let spectrum = rf_73_dsp::ModalSpectrum::<18>::prepare_polarized(
                p.geometry,
                p.assembly,
                p.polarization,
            )
            .unwrap();
            for (a, b) in reference.modes.iter().zip(spectrum.modes) {
                assert_eq!(a.frequency_hz, b.frequency_hz);
            }
        }
    }
}
