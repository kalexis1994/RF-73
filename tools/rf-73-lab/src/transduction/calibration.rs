//! Training-only conditional envelope fit; spectral disagreement remains explicit.
use super::{losses, tuning};
use rf_73_analysis::{detect_timbre_onset, measure_timbre_profile, pitch_anchor};
use rf_73_dsp::ElectromechanicalProfile;
use serde_json::{Value, json};
use std::{error::Error, io::BufWriter, path::Path};

const TINE: [f64; 3] = [1.0 / 6.0, 1.0 / 3.0, 0.5];
const SUPPORT: [f64; 3] = [0.1, 0.5, 1.0];
fn profile(position: f64, tine: f64, support: f64) -> ElectromechanicalProfile {
    let mut p = ElectromechanicalProfile::default();
    p.geometry.length_m = 0.07;
    p.geometry.tuning_position = position;
    p.assembly.tine_decay_seconds[0] /= tine;
    p.assembly.translation_damping_n_s_m *= support;
    p.assembly.rotation_damping_n_m_s_rad *= support;
    p
}
fn levels(p: &Value) -> Result<[f64; 2], Box<dyn Error>> {
    let level = |i| {
        p["windows"][i]["level_relative_to_body_db"]
            .as_f64()
            .filter(|x| x.is_finite())
            .ok_or("missing finite sustain level")
    };
    Ok([level(3)?, level(4)?])
}
fn score(sources: &[Value], candidate: &Value, role: &str) -> Result<f64, Box<dyn Error>> {
    let c = levels(candidate)?;
    let mut sum = 0.0;
    let mut count = 0;
    for s in sources.iter().filter(|s| s["role"] == role) {
        for (a, b) in c.into_iter().zip(levels(&s["profile"])?) {
            sum += (a - b).powi(2);
            count += 1;
        }
    }
    if count == 0 {
        return Err("empty source role in sustain score".into());
    }
    Ok((sum / count as f64).sqrt())
}
fn pair(source: &Value, candidate: &Value) -> Result<Value, Box<dyn Error>> {
    let a = levels(source)?;
    let b = levels(candidate)?;
    let delta = [b[0] - a[0], b[1] - a[1]];
    let mut spectral = Vec::new();
    let mut worst = 0.0_f64;
    let mut count = 0;
    for (a, b) in source["windows"]
        .as_array()
        .ok_or("missing source windows")?
        .iter()
        .zip(
            candidate["windows"]
                .as_array()
                .ok_or("missing candidate windows")?,
        )
    {
        let row: [Option<f64>; 3] = core::array::from_fn(|i| {
            b["band_db_relative_to_fundamental_band"][i]
                .as_f64()
                .zip(a["band_db_relative_to_fundamental_band"][i].as_f64())
                .map(|(b, a)| b - a)
        });
        for d in row.into_iter().flatten() {
            count += 1;
            worst = worst.max(d.abs());
        }
        spectral.push(row);
    }
    Ok(
        json!({"sustain_level_difference_db":delta,"sustain_within_3db":delta.iter().all(|d|d.abs()<=3.0),
        "band_balance_difference_db":spectral,"available_band_comparisons":count,"max_absolute_band_difference_db":worst,
        "spectral_agreement":if worst>6.0 {Some(false)} else if count==15 {Some(true)} else {None}}),
    )
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 4 || args[2] != "--output" {
        return Err(
            "calibrate-loaded-loss MANIFEST.json --output REPORT.json [--preview AUDIO.wav] [--qualified-grid]".into(),
        );
    }
    let output = Path::new(&args[3]);
    let mut preview = None;
    let mut qualified_grid = false;
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--qualified-grid" if !qualified_grid => {
                qualified_grid = true;
                i += 1;
            }
            "--preview" if preview.is_none() && i + 1 < args.len() => {
                preview = Some(Path::new(&args[i + 1]));
                i += 2;
            }
            _ => return Err("invalid or repeated calibration option".into()),
        }
    }
    if output.exists()
        || output.extension().is_none_or(|x| x != "json")
        || preview.is_some_and(|p| p.exists() || p.extension().is_none_or(|x| x != "wav"))
    {
        return Err("loss calibration requires new JSON and optional WAV paths".into());
    }
    let bank = crate::pitch_reference::verified_bank(Path::new(&args[1]))?;
    let mut sources = Vec::new();
    let mut training_pitch = Vec::new();
    for (id, role, clip) in &bank.takes {
        let anchor = pitch_anchor(clip, 55)?;
        let f = anchor.frequency_hz.ok_or("source pitch unavailable")?;
        let timbre = measure_timbre_profile(clip, detect_timbre_onset(clip)?, f)?;
        if !timbre.qualified {
            return Err("unqualified source profile".into());
        }
        if role == "training" {
            training_pitch.push(f);
        }
        sources.push(json!({"id":id,"role":role,"pitch_anchor":anchor,"profile":timbre}));
    }
    let target =
        (training_pitch.iter().map(|x| x.ln()).sum::<f64>() / training_pitch.len() as f64).exp();
    let (position, fit) = tuning::fitted_position(target)?;
    let mut rows = Vec::new();
    let result = (|| -> Result<Value, Box<dyn Error>> {
        println!("Loss calibration: baseline, 128 ticks");
        let baseline = losses::take_profile(profile(position, 1.0, 1.0), 128, 1.5)?;
        if baseline.report["passed"] != true {
            return Err("baseline failed numerical qualification".into());
        }
        let baseline_training = score(&sources, &baseline.report["timbre"], "training")?;
        let mut selected: Option<(f64, f64, f64, losses::Take)> = None;
        for tine in TINE {
            for support in SUPPORT {
                println!("Loss calibration: tine {tine}, support {support}, 128 ticks");
                let take = losses::take_profile(profile(position, tine, support), 128, 1.5)?;
                let rms = if take.report["passed"] == true {
                    Some(score(&sources, &take.report["timbre"], "training")?)
                } else {
                    None
                };
                rows.push(json!({"tine_loss_scale":tine,"support_loss_scale":support,"training_rms_db":rms,"take":take.report}));
                if take.report["passed"] != true {
                    if qualified_grid {
                        println!(
                            "Loss calibration: candidate withheld, pitch diagnostics {}",
                            take.report["pitch_anchor"]["rejection_reasons"]
                        );
                        continue;
                    }
                    return Err(
                        "grid candidate failed numerical qualification; selection withheld".into(),
                    );
                }
                let rms = rms.unwrap();
                println!("Loss calibration: training RMS {rms:.6} dB");
                // Strict improvement and fixed iteration order deterministically break ties.
                if selected.as_ref().is_none_or(|s| rms < s.0) {
                    selected = Some((rms, tine, support, take));
                }
            }
        }
        let (best, tine, support, coarse) = selected.ok_or("empty calibration grid")?;
        let near_best:Vec<_>=rows.iter().filter(|r|r["training_rms_db"].as_f64().is_some_and(|x|x<=best+0.25)).map(|r|json!({"tine_loss_scale":r["tine_loss_scale"],"support_loss_scale":r["support_loss_scale"],"training_rms_db":r["training_rms_db"]})).collect();
        println!(
            "Loss calibration: selected tine {tine}, support {support}; validating at 256 ticks"
        );
        let p = profile(position, tine, support);
        let fine = losses::take_profile(p, 256, 1.5)?;
        let convergence = losses::convergence(&coarse, &fine);
        let training = score(&sources, &fine.report["timbre"], "training")?;
        // Reserved-layer scores are evaluated only after the training-only selection.
        let validation = score(&sources, &fine.report["timbre"], "validation")?;
        let baseline_validation = score(&sources, &baseline.report["timbre"], "validation")?;
        let mut gestures = Vec::new();
        let mut pairs = Vec::new();
        for source in &sources {
            pairs.push(json!({"source_id":source["id"],"source_role":source["role"],"speed_m_s":1.5,"comparison":pair(&source["profile"],&fine.report["timbre"])?}));
        }
        let qualified = fine.report["passed"] == true && convergence["passed"] == true;
        if let Some(path) = preview {
            let mut wav = crate::wav::FloatWav::new(
                BufWriter::new(crate::new_file(path)?),
                48000,
                fine.samples.len() as u32,
            )?;
            for s in &fine.samples {
                wav.sample(*s as f32)?;
            }
            wav.finish()?;
        }
        gestures.push(json!({"speed_m_s":1.5,"qualified":qualified,"takes":[coarse.report,fine.report],"convergence":convergence}));
        for speed in [1.125, 1.75] {
            println!("Loss calibration: cross-speed {speed}, 128/256 ticks");
            let a = losses::take_profile(p, 128, speed)?;
            let b = losses::take_profile(p, 256, speed)?;
            let convergence = losses::convergence(&a, &b);
            for source in &sources {
                pairs.push(json!({"source_id":source["id"],"source_role":source["role"],"speed_m_s":speed,"comparison":if b.report["timbre"].is_null() {Value::Null} else {pair(&source["profile"],&b.report["timbre"])?}}));
            }
            gestures.push(json!({"speed_m_s":speed,"qualified":a.report["passed"]==true && b.report["passed"]==true && convergence["passed"]==true,"takes":[a.report,b.report],"convergence":convergence}));
        }
        let boundary =
            tine == TINE[0] || tine == TINE[2] || support == SUPPORT[0] || support == SUPPORT[2];
        let qualified = gestures.iter().all(|g| g["qualified"] == true);
        Ok(
            json!({"schema_version":1,"experiment":"loaded-loss-calibration-v1","measurement_qualified":qualified,"physical_calibration_claimed":false,"reference_match_claimed":false,
            "qualified_grid_followup":qualified_grid,"selection_policy":if qualified_grid {"Explicit follow-up after first grid corner withheld pitch: evaluate all nine candidates; retain every unqualified candidate and diagnostics with null score; select only qualified candidates. No numerical or observation threshold relaxed."} else {"Initial strict grid: withhold selection on any failed candidate"},
            "manifest":bank.manifest,"sources":sources,"target_hz":target,"structural_fit":fit,"grid":rows,
            "baseline":baseline.report,"baseline_training_rms_db":baseline_training,"baseline_validation_rms_db":baseline_validation,
            "selected":{"tine_loss_scale":tine,"support_loss_scale":support,"fundamental_t60_seconds":p.assembly.tine_decay_seconds[0],"training_coarse_rms_db":best,
                "training_fine_rms_db":training,"validation_fine_rms_db":validation,"on_grid_boundary":boundary,"within_025db_of_best":near_best},
            "envelope_improved":qualified && training<=0.5*baseline_training && validation<=0.5*baseline_validation,
            "gestures":gestures,"comparisons":pairs,
            "protocol":"Frozen before first run. Verify five pinned G3 source blobs. Training-only mean-log pitch sets spring; no retuning in the loss grid. Fit only the first tine-mode damping multiplier [1/6,1/3,1/2] and support translation/rotation multiplier [0.1,0.5,1]; upper-mode losses, tonebar, damper, pickup and circuit stay fixed. Rank all nine 128-tick candidates by pooled RMS dB level error over two sustain windows (640-1152 and 1152-1664 ms after onset), each relative to 256-512 ms body, using layers 1/3/5 only. Fixed 1.5 m/s key gesture, no layer-to-speed assignment. Select minimum with deterministic order; no reserved-layer score enters selection. Re-render selected at 256 ticks and evaluate reserved layers 2/4. Retain grid boundaries and alternatives within 0.25 dB training RMS, without a confidence claim. Cross-check selected parameters at 1.125/1.75 m/s with 128/256 ticks, preserving all 15 source/gesture comparisons and non-fitted spectral mismatches. Reuse loss-budget energy, quiet idle, contact, timbre and 1% waveform-refinement gates. Envelope improvement requires at least 50% RMS reduction on both training and reserved layers. Optional single fine medium WAV, fixed 0.1 FS/V, no normalization.",
            "scope":"Conditional output-envelope calibration on a processed bank with prior exposure. Reserved layers are not an independent instrument. No physical velocity labels or unique material identification; no fitted upper-mode damping, pickup field, EQ, amplifier or gain. Grid boundaries and similar candidates limit inference. Spectral mismatch is independent of envelope improvement. No production default or playable plugin change."}),
        )
    })();
    let report = match result {
        Ok(r) => r,
        Err(e) => {
            crate::analysis::write_report(
                output,
                &json!({"schema_version":1,"experiment":"loaded-loss-calibration-v1","measurement_qualified":false,"reason":e.to_string(),"completed_grid":rows}),
            )?;
            return Err(e);
        }
    };
    crate::analysis::write_report(output, &report)?;
    if report["measurement_qualified"] != true {
        return Err("loss calibration retained failed numerical qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_layers_cannot_change_training_score() {
        let p = |a, b| json!({"windows":[{},{},{},{"level_relative_to_body_db":a},{"level_relative_to_body_db":b}]});
        let mut sources = vec![
            json!({"role":"training","profile":p(-2.0,-4.0)}),
            json!({"role":"validation","profile":p(-1.0,-2.0)}),
        ];
        let c = p(-2.0, -3.0);
        let before = score(&sources, &c, "training").unwrap();
        sources[1]["profile"] = p(1000.0, -1000.0);
        assert_eq!(score(&sources, &c, "training").unwrap(), before);
        assert!(score(&sources, &c, "validation").unwrap() > 900.0);
        assert!(score(&sources, &c, "missing").is_err());
    }
    #[test]
    fn grid_preserves_upper_mode_losses_and_static_state() {
        let base = profile(0.8, 1.0, 1.0);
        let initial = rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, base)
            .unwrap()
            .probe();
        for t in TINE {
            for s in SUPPORT {
                let p = profile(0.8, t, s);
                assert_eq!(
                    p.assembly.tine_decay_seconds[1..],
                    base.assembly.tine_decay_seconds[1..]
                );
                assert_eq!(
                    rf_73_dsp::ElectromechanicalAssembly::new_at_rest(1e-6, p)
                        .unwrap()
                        .probe(),
                    initial
                );
            }
        }
    }
}
