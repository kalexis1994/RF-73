//! Shared physical voicing with training-only ordinal drive hypotheses.
use super::{losses, tuning, voicing};
use rf_73_analysis::{detect_timbre_onset, measure_timbre_profile, pitch_anchor};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

const SPEEDS: [f64; 5] = [1.125, 1.3125, 1.5, 1.625, 1.75];
const VOICINGS: [&str; 2] = ["baseline", "pickup_vertical_offset_1mm"];
const DIRECTIONS: [&str; 2] = ["increasing_with_layer", "decreasing_with_layer"];

fn index(layer: usize, reverse: bool) -> usize {
    if reverse { 4 - layer } else { layer }
}

fn ordered_sources(sources: &[Value]) -> Result<Vec<Value>, Box<dyn Error>> {
    if sources.len() != 5 {
        return Err("dynamics protocol requires exactly five declared G3 layers".into());
    }
    (1..=5)
        .map(|layer| {
            let id = format!("g3-layer-{layer}");
            let s = sources
                .iter()
                .find(|s| s["id"] == id)
                .ok_or("missing G3 layer")?;
            let role = if layer % 2 == 1 {
                "training"
            } else {
                "validation"
            };
            if s["role"] != role {
                return Err(
                    "dynamics protocol requires training layers 1/3/5 and reserved layers 2/4"
                        .into(),
                );
            }
            Ok(s.clone())
        })
        .collect()
}

// Called with a single shared voicing. Reserved spectra are never accessed.
fn training_score(sources: &[Value], takes: &[Value], reverse: bool) -> Option<f64> {
    let mut square = 0.0;
    for layer in [0, 2, 4] {
        let speed = index(layer, reverse);
        let take = takes.iter().find(|t| t["speed_index"] == speed)?;
        if take["measurement_qualified"] != true {
            return None;
        }
        square +=
            voicing::attack_error(&sources[layer]["profile"], &take["takes"][1]["timbre"])?.powi(2);
    }
    Some((square / 3.0).sqrt())
}

fn select(scores: &[Value]) -> Option<usize> {
    scores
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            s["training_attack_rms_db"]
                .as_f64()
                .filter(|v| v.is_finite())
                .map(|v| (i, v))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(i, _)| i)
}

fn render(position: f64, variant: usize, speed_index: usize) -> Result<Value, Box<dyn Error>> {
    let p = voicing::profile(position, if variant == 0 { 0 } else { 6 });
    let speed = SPEEDS[speed_index];
    println!(
        "Loaded dynamics: {}, {speed} m/s, 128 ticks",
        VOICINGS[variant]
    );
    let a = losses::take_observed(p, 128, speed, true)?;
    println!(
        "Loaded dynamics: {}, {speed} m/s, 256 ticks",
        VOICINGS[variant]
    );
    let b = losses::take_observed(p, 256, speed, true)?;
    let voltage = losses::convergence(&a, &b);
    let impact = voicing::impact_convergence(&a.report["impact"], &b.report["impact"]);
    let passed = a.report["passed"] == true
        && b.report["passed"] == true
        && voltage["passed"] == true
        && impact["passed"] == true;
    println!(
        "Loaded dynamics: {}, {speed} m/s, qualified={passed}",
        VOICINGS[variant]
    );
    Ok(
        json!({"voicing":VOICINGS[variant],"speed_index":speed_index,"speed_m_s":speed,
        "measurement_qualified":passed,"takes":[a.report,b.report],"voltage_convergence":voltage,"impact_convergence":impact}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err("loaded-dynamics MANIFEST.json --output REPORT.json".into());
    }
    let output = Path::new(&args[3]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("dynamics study requires a new JSON path".into());
    }
    let bank = crate::pitch_reference::verified_bank(Path::new(&args[1]))?;
    // Reject incompatible roles/IDs before any expensive source or model analysis.
    let identities: Vec<_> = bank
        .takes
        .iter()
        .map(|(id, role, _)| json!({"id":id,"role":role}))
        .collect();
    ordered_sources(&identities)?;
    let mut sources = Vec::new();
    let mut pitches = Vec::new();
    for (id, role, clip) in &bank.takes {
        let anchor = pitch_anchor(clip, 55)?;
        let f = anchor.frequency_hz.ok_or("unqualified source pitch")?;
        let profile = measure_timbre_profile(clip, detect_timbre_onset(clip)?, f)?;
        if !profile.qualified {
            return Err("unqualified source timbre".into());
        }
        if role == "training" {
            pitches.push(f);
        }
        sources.push(json!({"id":id,"role":role,"pitch_anchor":anchor,"profile":profile}));
    }
    let sources = ordered_sources(&sources)?;
    let target = (pitches.iter().map(|f| f.ln()).sum::<f64>() / pitches.len() as f64).exp();
    let (position, fit) = tuning::fitted_position(target)?;
    let mut training_cases = Vec::new();
    let mut validation_cases = Vec::new();
    let mut scores = Vec::new();
    let mut selected = Value::Null;
    let mut pairs = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (variant, voicing_name) in VOICINGS.iter().enumerate() {
            let mut group = Vec::new();
            for speed in [0, 2, 4] {
                let take = render(position, variant, speed)?;
                training_cases.push(take.clone());
                group.push(take);
            }
            for (direction, name) in DIRECTIONS.iter().enumerate() {
                scores.push(json!({"voicing":voicing_name,"variant_index":variant,"direction":name,
                    "reverse":direction==1,"training_attack_rms_db":training_score(&sources,&group,direction==1)}));
            }
        }
        let chosen = select(&scores).ok_or("no fully qualified shared training hypothesis")?;
        selected = scores[chosen].clone();
        selected["hypothesis_index"] = json!(chosen);
        let variant = selected["variant_index"]
            .as_u64()
            .ok_or("invalid selected voicing")? as usize;
        let reverse = selected["reverse"]
            .as_bool()
            .ok_or("invalid selected direction")?;
        println!(
            "Loaded dynamics: frozen selection {} / {}, training attack RMS {} dB",
            selected["voicing"], selected["direction"], selected["training_attack_rms_db"]
        );
        // Render reserved intermediate speeds only after freezing the training winner.
        for layer in [1, 3] {
            validation_cases.push(render(position, variant, index(layer, reverse))?);
        }
        for (layer, s) in sources.iter().enumerate() {
            let cases = if layer % 2 == 0 {
                &training_cases
            } else {
                &validation_cases
            };
            let take = cases
                .iter()
                .find(|t| {
                    t["voicing"] == selected["voicing"] && t["speed_index"] == index(layer, reverse)
                })
                .ok_or("missing mapped take")?;
            pairs.push(json!({"source_id":s["id"],"source_role":s["role"],"speed_m_s":take["speed_m_s"],
                "measurement_qualified":take["measurement_qualified"],"comparison":voicing::comparison(&s["profile"],&take["takes"][1]["timbre"])}));
        }
        let mut square = Some(0.0);
        for pair in pairs.iter().filter(|p| p["source_role"] == "validation") {
            square = square
                .zip(pair["comparison"]["attack_band_rms_db"].as_f64())
                .map(|(sum, e)| sum + e * e);
        }
        selected["validation_attack_rms_db"] = json!(square.map(|s| (s / 2.0).sqrt()));
        selected["validation_measurement_qualified"] = json!(
            validation_cases
                .iter()
                .all(|c| c["measurement_qualified"] == true)
        );
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let qualified = failure.is_none()
        && selected["validation_measurement_qualified"] == true
        && selected["validation_attack_rms_db"].as_f64().is_some();
    let all_comparisons: Vec<_> = training_cases.iter().chain(&validation_cases).flat_map(|t| {
        sources.iter().map(move |s|json!({"voicing":t["voicing"],"speed_m_s":t["speed_m_s"],
            "source_id":s["id"],"source_role":s["role"],"comparison":voicing::comparison(&s["profile"],&t["takes"][1]["timbre"])}))
    }).collect();
    let report = json!({"schema_version":1,"experiment":"loaded-dynamics-v1","measurement_qualified":qualified,
        "failure_reason":failure,"reference_match_claimed":false,"physical_calibration_claimed":false,
        "manifest":bank.manifest,"sources":sources,"target_hz":target,"structural_fit":fit,"speed_grid_m_s":SPEEDS,
        "training_cases":training_cases,"hypotheses":scores,"selected":selected,"validation_cases":validation_cases,
        "mapped_comparisons":pairs,"all_comparisons":all_comparisons,
        "protocol":"Frozen shared-voicing ordinal-drive experiment. Two fixed voicings: prior baseline and 1 mm vertical pickup offset; retain 70 mm tuned tine, first basis T60 30 s, original support/contact and other pickup settings. Training layers 1/3/5 use either 1.125/1.5/1.75 m/s or the reversed assignment. These are two explicit nuisance hypotheses, not measured layer velocities. Twelve 1.8 s stationary-rest training takes at 128/256 ticks per 48 kHz frame. Require unchanged energy/quiet/pitch/timbre/single-contact gates and <1% voltage/impact refinement for every take of an eligible shared voicing. Pool nine attack-band dB errors; any missing dimension withholds that hypothesis. Select minimum training RMS across four hypotheses with fixed-order tie break; no reserved spectra in score. Freeze voicing and direction, then render four reserved takes at interpolated speeds 1.3125/1.625 m/s for layers 2/4, reversed if selected. No refit or fallback after reserved failure. Preserve mapped and all cross-comparisons, original 6 dB spectral/3 dB sustain gates, source processing, numerical failures and null observations. No gain/EQ fit, velocity optimization or WAV matrix. Selection is an experimental candidate only; no plugin preset is changed.",
        "scope":"Conditional G3 dynamic-response test with known source processing and prior exposure. Ordinal speed endpoints and linear interpolation are imposed, not physically identified. Testing two directions does not explore arbitrary velocity curves. Reserved layers are same-bank checks, not blind instrument data. Sparse voicing/speed grid, boundary-selected losses, release/repetition, other registers, new material laws, listening and realtime integration remain unresolved."});
    crate::analysis::write_report(output, &report)?;
    if !qualified {
        return Err("dynamics study retained failed qualification".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Vec<Value>, Vec<Value>) {
        let sources = (0..5).map(|i|json!({"id":format!("g3-layer-{}",i+1),"role":if i%2==0 {"training"} else {"validation"},
            "profile":{"windows":[{"band_db_relative_to_fundamental_band":([i as f64;3])}]}})).collect();
        let takes = [0,2,4].into_iter().map(|i|json!({"speed_index":i,"measurement_qualified":true,
            "takes":[null,{"timbre":{"windows":[{"band_db_relative_to_fundamental_band":([i as f64;3])}]}}]})).collect();
        (sources, takes)
    }
    #[test]
    fn reserved_spectra_cannot_change_training_selection() {
        let (mut s, t) = fixture();
        assert_eq!(training_score(&s, &t, false), Some(0.0));
        assert!(training_score(&s, &t, true).unwrap() > 3.0);
        s[1]["profile"] = Value::Null;
        s[3]["profile"] = json!({"arbitrary":1e30});
        assert_eq!(training_score(&s, &t, false), Some(0.0));
        assert_eq!(
            select(&[
                json!({"training_attack_rms_db":null}),
                json!({"training_attack_rms_db":1.0}),
                json!({"training_attack_rms_db":1.0})
            ]),
            Some(1)
        );
    }
    #[test]
    fn missing_bands_or_failed_speeds_withhold_shared_hypothesis() {
        let (s, mut t) = fixture();
        t[0]["measurement_qualified"] = json!(false);
        assert_eq!(training_score(&s, &t, false), None);
        t[0]["measurement_qualified"] = json!(true);
        t[0]["takes"][1]["timbre"]["windows"][0]["band_db_relative_to_fundamental_band"][2] =
            Value::Null;
        assert_eq!(training_score(&s, &t, false), None);
        assert_eq!(training_score(&s, &t, true), None);
    }
    #[test]
    fn fixed_layer_roles_and_interpolation_are_unambiguous() {
        let (mut s, _) = fixture();
        s.reverse();
        assert_eq!(ordered_sources(&s).unwrap()[0]["id"], "g3-layer-1");
        s[0]["role"] = json!("validation");
        assert!(ordered_sources(&s).is_err());
        for reverse in [false, true] {
            for layer in [1, 3] {
                assert_eq!(
                    SPEEDS[index(layer, reverse)],
                    0.5 * (SPEEDS[index(layer - 1, reverse)] + SPEEDS[index(layer + 1, reverse)])
                );
            }
        }
    }
}
