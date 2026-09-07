//! Frozen real-source baseline: no fitted loss, pickup, EQ or layer-to-speed mapping.
use super::tuning;
use rf_73_analysis::{
    AudioClip, TimbreProfile, detect_timbre_onset, measure_timbre_profile, pitch_anchor,
};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

pub const HELP: &str = "Loaded real-source baseline:
  compare-loaded-bank MANIFEST.json --output REPORT.json [--preview AUDIO.wav] [--striking] [--at-rest]
Verify every G3 blob before rendering. Three prescribed pedestal speeds,
0.75/1.125/1.5 m/s, each at 128/256 ticks with the tuned 70 mm assembly.
Compare every source layer against every gesture; no assumed velocity mapping,
per-window gain fit, EQ fit or parameter optimization.
--striking selects the follow-up 1.125/1.5/1.75 m/s profile after a contact preflight.
--at-rest prepares static contact equilibrium before time integration.
The optional preview is always the 1.125 m/s gesture. Report measurement
qualification separately from descriptive mismatch.
";
fn comparison(source: &TimbreProfile, candidate: &TimbreProfile) -> Value {
    let mut worst_band = 0.0_f64;
    let mut worst_level = 0.0_f64;
    let mut count = 0;
    let windows:Vec<_>=source.windows.iter().zip(&candidate.windows).map(|(s,c)| {
        let bands:[Option<f64>;3]=core::array::from_fn(|i|c.band_db_relative_to_fundamental_band[i].zip(s.band_db_relative_to_fundamental_band[i]).map(|(c,s)|c-s));
        for x in bands.into_iter().flatten() {worst_band=worst_band.max(x.abs());count+=1;}
        let level=c.level_relative_to_body_db.zip(s.level_relative_to_body_db).map(|(c,s)|c-s);
        if let Some(x)=level {worst_level=worst_level.max(x.abs());}
        json!({"candidate_minus_source_band_balance_db":bands,"candidate_minus_source_relative_level_db":level})
    }).collect();
    // Missing spectral comparisons cannot establish agreement, but do not hide
    // disagreement in another measurable band. These are descriptive thresholds.
    let within = if worst_band > 6.0 || worst_level > 3.0 {
        Some(false)
    } else if count == 15
        && source.qualified
        && candidate.qualified
        && source
            .windows
            .iter()
            .chain(&candidate.windows)
            .all(|w| w.level_relative_to_body_db.is_some())
    {
        Some(true)
    } else {
        None
    };
    json!({"windows":windows,"available_band_comparisons":count,"max_absolute_band_balance_difference_db":worst_band,
        "max_absolute_relative_level_difference_db":worst_level,"within_descriptive_tolerances":within})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    let mut preview = None;
    let mut striking = false;
    let mut at_rest = false;
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--at-rest" if !at_rest => {
                at_rest = true;
                i += 1;
            }
            "--striking" if !striking => {
                striking = true;
                i += 1;
            }
            "--preview" if preview.is_none() && i + 1 < args.len() => {
                preview = Some(Path::new(&args[i + 1]));
                i += 2;
            }
            _ => return Err(HELP.into()),
        }
    }
    if output.extension().is_none_or(|x| x != "json")
        || output.exists()
        || preview.is_some_and(|p| p.exists() || p.extension().is_none_or(|e| e != "wav"))
    {
        return Err("bank comparison requires new JSON and optional WAV paths".into());
    }
    let bank = crate::pitch_reference::verified_bank(Path::new(&args[1]))?;
    let result = (|| -> Result<Value, Box<dyn Error>> {
        let mut source_rows = Vec::new();
        let mut source_profiles = Vec::new();
        let mut training = Vec::new();
        for (id, role, clip) in &bank.takes {
            let anchor = pitch_anchor(clip, 55)?;
            let f = anchor
                .frequency_hz
                .ok_or("source pitch observer withheld the fundamental")?;
            if role == "training" {
                training.push(f);
            }
            let onset = detect_timbre_onset(clip)?;
            let p = measure_timbre_profile(clip, onset, f)?;
            source_rows.push(json!({"id":id,"role":role,"pitch_anchor":anchor,"profile":p}));
            source_profiles.push(p);
        }
        let target = (training.iter().map(|f| f.ln()).sum::<f64>() / training.len() as f64).exp();
        let (position, fit) = tuning::fitted_position(target)?;
        let feasibility = if at_rest {
            tuning::strike_feasibility_initialized(position, true)?
        } else {
            tuning::strike_feasibility(position)?
        };
        let speeds = if striking {
            [1.125, 1.5, 1.75]
        } else {
            [0.75, 1.125, 1.5]
        };
        if speeds.iter().any(|speed| {
            feasibility["cases"].as_array().unwrap().iter().any(|r| {
                r["pedestal_speed_m_s"].as_f64() == Some(*speed)
                    && r["first_contact_seconds"].is_null()
            })
        }) {
            return Ok(
                json!({"schema_version":1,"experiment":"loaded-source-timbre-baseline-v1","measurement_qualified":false,
                "reason":"requested profile includes a non-striking gesture","at_rest":at_rest,"strike_feasibility":feasibility,"requested_speeds_m_s":speeds,
                "manifest":bank.manifest,"sources":source_rows}),
            );
        }
        println!(
            "Verified {} source layers; frozen target {target:.9} Hz",
            source_rows.len()
        );
        let mut candidates = Vec::new();
        let mut pairs = Vec::new();
        for speed in speeds {
            println!("Rendering loaded gesture {speed} m/s, 128 ticks");
            let coarse = tuning::take_initialized(position, 128, speed, at_rest)?;
            println!("Rendering loaded gesture {speed} m/s, 256 ticks");
            let fine = tuning::take_initialized(position, 256, speed, at_rest)?;
            let convergence = tuning::compare_resolution(&coarse, &fine);
            if speed == 1.125
                && let Some(p) = preview
            {
                tuning::write_wav(p, &fine)?;
            }
            // Known mechanical onset plus fixed FIR group delay. No detector
            // can mistake the model's retained preload relaxation for the strike.
            let onset = fine
                .first_contact_seconds
                .ok_or("gesture produced no hammer contact")?
                + 63.0 / (48000.0 * 4.0);
            let clip = AudioClip::from_samples(48000, fine.samples)?;
            let anchor = pitch_anchor(&clip, 55)?;
            let f = anchor
                .frequency_hz
                .ok_or("candidate pitch observer withheld the fundamental")?;
            let p = measure_timbre_profile(&clip, onset, f)?;
            let error_cents = 1200.0 * (f / target).log2();
            let qualified = coarse.summary["passed"] == true
                && fine.summary["passed"] == true
                && convergence["passed"] == true
                && p.qualified
                && error_cents.abs() < 5.0;
            for ((id, role, _), source) in bank.takes.iter().zip(&source_profiles) {
                pairs.push(json!({"source_id":id,"source_role":role,"pedestal_speed_m_s":speed,"comparison":comparison(source,&p)}));
            }
            candidates.push(json!({"pedestal_speed_m_s":speed,"measurement_qualified":qualified,"first_hammer_contact_seconds":fine.first_contact_seconds,
                "takes":[coarse.summary,fine.summary],"convergence":convergence,"pitch_anchor":anchor,"output_error_cents":error_cents,"profile":p}));
        }
        let qualified = source_profiles.iter().all(|p| p.qualified)
            && candidates
                .iter()
                .all(|c| c["measurement_qualified"] == true);
        Ok(
            json!({"schema_version":1,"experiment":"loaded-source-timbre-baseline-v1","measurement_qualified":qualified,
            "reference_match_claimed":false,"manifest":bank.manifest,"training_pitch_target_hz":target,"structural_fit":fit,
            "striking_followup":striking,"at_rest":at_rest,"initialization":if at_rest { "Qualified static mechanical equilibrium, zero circuit current/voltage; no warmup or muting" } else { "Historical cold preload relaxation" },"pedestal_speeds_m_s":speeds,"strike_feasibility":feasibility,
            "sources":source_rows,"candidates":candidates,"comparisons":pairs,
            "protocol":"Measurement gates fixed before first run. The initial 0.75/1.125/1.5 m/s study failed because 0.75 produced no hammer contact; its receipt is retained. The explicit --striking follow-up uses 1.125/1.5/1.75 m/s after the six-speed contact preflight, with no change to physics or qualification gates. Verify manifest and all Git blob identities; preserve source rates and gain. Training-only mean log frequency sets the spring target. No assignment to recorded layers, 128/256 ticks and prior long-gesture convergence/energy/headroom gates. Source onset: four 1 ms RMS bins above -40 dB of first-250-ms maximum. Model onset: first hammer contact plus FIR delay; retain pre-onset peak. Five windows after onset: 0-64,64-192,256-512,640-1152,1152-1664 ms. Hann spectral-power bands use edges 0.5/1.5/4/12 times each observed fundamental, capped at 8 kHz. Report band balance relative to first band and RMS level relative to 256-512 ms body. Fractions below 1e-8 withhold band dB. Compare all layers with all gestures, retaining roles without optimization. Descriptive differences >6 dB in band balance or >3 dB in relative level flag mismatch; missing bands cannot establish agreement. Technical qualification is distinct from timbre agreement.",
            "scope":"Processed sample-bank comparison, not raw physical parameter identification. Unknown capture gain and EQ/noise reduction confound absolute levels and spectral/decay interpretation. Layer labels are not hammer or key velocities. Prior exposure is declared in the source manifest. No fitted material/field parameters, time warping, per-window gain, plugin update or human-listening verdict."}),
        )
    })();
    let value = match result {
        Ok(v) => v,
        Err(e) => {
            crate::analysis::write_report(
                output,
                &json!({"schema_version":1,"experiment":"loaded-source-timbre-baseline-v1","at_rest":at_rest,"measurement_qualified":false,"reason":e.to_string()}),
            )?;
            return Err(e);
        }
    };
    crate::analysis::write_report(output, &value)?;
    if value["measurement_qualified"] != true {
        return Err("loaded source baseline retained failed measurement qualification".into());
    }
    println!("Loaded source baseline measured; inspect descriptive mismatches separately");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_bands_withhold_agreement_without_hiding_known_mismatch() {
        let clip = AudioClip::from_samples(
            48000,
            (0..96000)
                .map(|i| 0.1 * (core::f64::consts::TAU * 196.0 * f64::from(i) / 48000.0).sin())
                .collect(),
        )
        .unwrap();
        let a = measure_timbre_profile(&clip, 0.0, 196.0).unwrap();
        let mut b = measure_timbre_profile(&clip, 0.0, 196.0).unwrap();
        assert_eq!(
            comparison(&a, &b)["within_descriptive_tolerances"],
            Value::Null
        );
        b.windows[0].level_relative_to_body_db = Some(9.0);
        assert_eq!(comparison(&a, &b)["within_descriptive_tolerances"], false);
    }
}
