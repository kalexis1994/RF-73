//! Bounded, read-only source audit and explicitly provisional preset comparison.
use rf_tines_analysis::{
    AudioClip, EnvelopeOptions, ToneComparisonOptions, compare_tone, detect_timbre_onset,
    measure_component_envelope, pitch_anchor,
};
use rf_tines_dsp::{Engine, PickupLaw, Profile};
use serde_json::{Value, json};
use std::{error::Error, fs, io::Write, path::Path};

fn db(x: f64) -> Option<f64> {
    (x > 0.0).then(|| 20.0 * x.log10())
}
fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}
fn proportionality(a: &[f64], b: &[f64]) -> Value {
    let n = a.len().min(b.len());
    let aa = a[..n].iter().map(|x| x * x).sum::<f64>();
    let bb = b[..n].iter().map(|x| x * x).sum::<f64>();
    let ab = a[..n].iter().zip(&b[..n]).map(|(x, y)| x * y).sum::<f64>();
    if aa == 0.0 || bb == 0.0 {
        return json!({"qualified":false});
    }
    let gain = ab / aa;
    let residual = (a[..n]
        .iter()
        .zip(&b[..n])
        .map(|(x, y)| (gain * x - y).powi(2))
        .sum::<f64>()
        / bb)
        .sqrt();
    json!({"qualified":true,"same_length":a.len()==b.len(),"frames":n,
        "gain_b_over_a":gain,"correlation":ab/(aa*bb).sqrt(),"relative_rms_residual":residual})
}
fn write(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(&bytes)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn note(name: &str) -> Result<u8, Box<dyn Error>> {
    let (pitch, octave) = name.split_at(name.len() - 1);
    let pc = match pitch {
        "C" => 0,
        "C#" => 1,
        "D" => 2,
        "D#" => 3,
        "E" => 4,
        "F" => 5,
        "F#" => 6,
        "G" => 7,
        "G#" => 8,
        "A" => 9,
        "A#" => 10,
        "B" => 11,
        _ => return Err("invalid pitch".into()),
    };
    // Hypothesis checked against the retained spectra: author's C3 = MIDI 60.
    Ok((octave.parse::<u8>()? + 2) * 12 + pc)
}
fn levels(clip: &AudioClip) -> Value {
    let x = clip.samples();
    let rate = clip.metadata().sample_rate as usize;
    let peak = x.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let envelope: Vec<_> = x
        .chunks(rate / 50)
        .enumerate()
        .map(|(i, w)| {
            json!({
        "start_seconds":i as f64*0.02,"rms_dbfs":db(rms(w))})
        })
        .collect();
    json!({"peak":peak,"peak_dbfs":db(peak),"rms_dbfs":db(rms(x)),
        "dc":x.iter().sum::<f64>()/x.len() as f64,
        "full_scale_samples":x.iter().filter(|v|v.abs()>=1.0).count(),
        "first_sample":x[0],"last_sample":x[x.len()-1],
        "last_100ms_rms_dbfs":db(rms(&x[x.len()-rate/10..])),"envelope_20ms":envelope})
}
fn component(clip: &AudioClip, frequency: f64, onset: f64) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::to_value(measure_component_envelope(
        clip,
        EnvelopeOptions {
            frequency_hz: frequency,
            start_seconds: onset + 0.5,
            end_seconds: onset + 4.5,
            window_seconds: 0.512,
            hop_seconds: 0.128,
            known_neighbor_frequencies_hz: vec![],
        },
    )?)?)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || !["audit", "compare"].contains(&args[1].as_str()) {
        return Err(
            "usage: matts_reference audit|compare SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY".into(),
        );
    }
    let source = Path::new(&args[2]);
    let out = Path::new(&args[3]);
    fs::create_dir(out)?;
    if args[1] == "audit" {
        let mut paths = fs::read_dir(source)?
            .map(|r| r.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut rows = Vec::new();
        for path in paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "wav"))
        {
            let stem = path
                .file_stem()
                .unwrap()
                .to_str()
                .ok_or("non UTF8 filename")?;
            let (name, layer) = stem.rsplit_once('-').ok_or("missing layer")?;
            let midi = note(name)?;
            let clip = AudioClip::open(path, None)?;
            let onset = detect_timbre_onset(&clip)?;
            let tone = compare_tone(
                &clip,
                &clip,
                ToneComparisonOptions {
                    note: midi,
                    reference_start_seconds: onset,
                    candidate_start_seconds: onset,
                },
            )?;
            let body = &tone.windows[2].reference;
            let fundamental = body.harmonics[0].peak.as_ref();
            rows.push(json!({"file":path.file_name().unwrap().to_str(),"filename_note":name,
                "layer":layer,"hypothesized_midi_note":midi,"audio":clip.metadata(),
                "duration_seconds":clip.duration(),"onset_seconds":onset,"levels":levels(&clip),
                "body_spectrum":body,"fundamental_hz":fundamental.map(|p|p.frequency_hz),
                "tuning_cents":fundamental.map(|p|1200.0*(p.frequency_hz/(440.0*2.0_f64.powf((f64::from(midi)-69.0)/12.0))).log2())}));
        }
        let names: std::collections::BTreeSet<_> = rows
            .iter()
            .filter_map(|r| r["filename_note"].as_str())
            .collect();
        let mut pairs = Vec::new();
        for name in names {
            for (a, b) in [("p", "mp"), ("mp", "mf"), ("mf", "f")] {
                let left = AudioClip::open(source.join(format!("{name}-{a}.wav")), None)?;
                let right = AudioClip::open(source.join(format!("{name}-{b}.wav")), None)?;
                pairs.push(json!({"filename_note":name,"layers":[a,b],
                    "proportionality":proportionality(left.samples(),right.samples())}));
            }
        }
        write(
            &out.join("audit.json"),
            &json!({"schema_version":1,"method":"Rust AudioClip finite-sample validation, 20 ms RMS bins, onset-aligned self tone spectra; no source edits",
            "octave_hypothesis":"filename C3 = MIDI 60; verify spectra before interpreting",
            "adjacent_layer_proportionality_no_alignment":pairs,"files":rows}),
        )?;
        println!("Audited {} WAVs", rows.len());
    } else {
        let mut summary = Vec::new();
        for (filename, midi) in [("D2", 50u8), ("G2", 55u8), ("B2", 59u8)] {
            for (layer, velocity) in [("p", 0.25), ("mp", 0.45), ("mf", 0.65), ("f", 0.85)] {
                let src = source.join(format!("{filename}-{layer}.wav"));
                let reference = AudioClip::open(&src, None)?;
                let onset = detect_timbre_onset(&reference)?;
                let pitch = pitch_anchor(&reference, midi)?;
                // A failed qualified anchor stays visible; the short-window frequency is
                // an explicitly provisional carrier, never a qualified pitch measurement.
                let self_tone = compare_tone(
                    &reference,
                    &reference,
                    ToneComparisonOptions {
                        note: midi,
                        reference_start_seconds: onset,
                        candidate_start_seconds: onset,
                    },
                )?;
                let observed = self_tone.windows[2].reference.harmonics[0]
                    .peak
                    .as_ref()
                    .ok_or("missing reference fundamental")?
                    .frequency_hz;
                let frequency = pitch.frequency_hz.unwrap_or(observed);
                let source_component = component(&reference, frequency, onset)?;
                let source_receipt = json!({"file":src,"pitch_anchor":pitch,
                    "provisional_short_window_frequency_hz":observed,"onset_seconds":onset,
                    "component":source_component});
                write(
                    &out.join(format!("{filename}-{layer}-source.json")),
                    &source_receipt,
                )?;
                for preset in ["Original", "Calibrated"] {
                    let profile = if preset == "Original" {
                        Profile::default()
                    } else {
                        Profile {
                            pickup_law: PickupLaw::Aperture,
                            pickup_gap_m: 0.0005,
                            pickup_offset_m: 0.0005,
                            decay_seconds: 5.0 * 16.0_f64.powf(0.5),
                            bar_partial_decay_seconds: 0.16 * 14.375_f64.powf(1.0),
                            bar_partial_strike_weight: -0.3 * 0.2582 * 0.2582,
                            ..Profile::default()
                        }
                    };
                    let rate = reference.metadata().sample_rate;
                    let mut engine = Engine::new(f64::from(rate), profile)?;
                    engine.set_gain(0.1);
                    engine.set_level_compensation(true);
                    engine.reset();
                    engine.note_on(0, midi, velocity);
                    // Six seconds continuously held. No release boundary is inferred for the source.
                    let samples = (0..rate * 6)
                        .map(|_| f64::from(engine.next_sample()))
                        .collect();
                    if engine.faults() != 0 {
                        return Err("engine fault".into());
                    }
                    let candidate = AudioClip::from_samples(rate, samples)?;
                    let candidate_onset = detect_timbre_onset(&candidate)?;
                    let comparison = compare_tone(
                        &reference,
                        &candidate,
                        ToneComparisonOptions {
                            note: midi,
                            reference_start_seconds: onset,
                            candidate_start_seconds: candidate_onset,
                        },
                    )?;
                    let candidate_frequency = 440.0 * 2.0_f64.powf((f64::from(midi) - 69.0) / 12.0);
                    let receipt = json!({"preset":preset,"source":src,"note":midi,
                        "nominal_source_layer":layer,"illustrative_model_velocity":velocity,
                        "velocity_scope":"Ordinal pairing only; not measured source velocity, not fitted",
                        "profile":format!("{profile:?}"),"gain":0.1,"level_compensation":true,
                        "engine_faults":engine.faults(),"candidate_onset_seconds":candidate_onset,
                        "candidate_levels":levels(&candidate),"tone_comparison":comparison,
                        "candidate_component":component(&candidate,candidate_frequency,candidate_onset)?,
                        "scope":"Descriptive 0.5..4.5 s component envelope; source note-off unknown; no physical loss fit or T60 claim"});
                    let name = format!("{filename}-{layer}-{preset}.json");
                    write(&out.join(&name), &receipt)?;
                    summary.push(
                        json!({"file":name,"note":midi,"preset":preset,"layer":layer,
                        "body_harmonics":comparison.windows[2].harmonics}),
                    );
                }
                println!("Compared {filename} {layer}");
            }
        }
        write(
            &out.join("summary.json"),
            &json!({"comparisons":summary,"source_velocity_mapping":"unknown; fixed illustrative ordinal pairing","source_release_boundary":"unknown"}),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proportional_copy_is_separated_from_changed_waveform() {
        let a = [0.0, 1.0, -0.5, 0.2];
        let b = a.map(|x| x * 0.3);
        let exact = proportionality(&a, &b);
        assert!(exact["relative_rms_residual"].as_f64().unwrap() < 1e-14);
        assert!((exact["gain_b_over_a"].as_f64().unwrap() - 0.3).abs() < 1e-14);
        assert!(
            proportionality(&a, &[0.0, 0.3, -0.15, 0.2])["relative_rms_residual"]
                .as_f64()
                .unwrap()
                > 0.1
        );
        assert_eq!(proportionality(&[0.0; 4], &b)["qualified"], false);
    }
    #[test]
    fn known_exponential_has_correct_component_slope() {
        let rate = 48000;
        let clip = AudioClip::from_samples(
            rate,
            (0..rate * 6)
                .map(|i| {
                    let t = f64::from(i) / f64::from(rate);
                    0.2 * (-0.3 * t).exp() * (std::f64::consts::TAU * 196.0 * t).sin()
                })
                .collect(),
        )
        .unwrap();
        let report = component(&clip, 196.0, 0.0).unwrap();
        assert_eq!(report["qualified"], true);
        let decay = report["provisional_fit"]["amplitude_decay_per_second"]
            .as_f64()
            .unwrap();
        assert!((decay - 0.3).abs() < 1e-4);
    }
}
