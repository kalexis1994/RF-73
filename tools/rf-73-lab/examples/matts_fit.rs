//! Bounded training-only search, followed by a frozen held-out evaluation.
use rf_73_analysis::{
    AudioClip, EnvelopeOptions, ToneComparisonOptions, compare_tone, detect_timbre_onset,
    measure_component_envelope, pitch_anchor,
};
use rf_73_dsp::{Engine, PickupLaw, Profile};
use serde_json::{Value, json};
use std::{error::Error, fs, io::Write, path::Path};
pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) struct Case {
    pub(crate) note: u8,
    pub(crate) layer: &'static str,
    pub(crate) velocity: f64,
    pub(crate) clip: AudioClip,
    pub(crate) onset: f64,
}
pub(crate) fn baseline() -> Profile {
    Profile {
        pickup_law: PickupLaw::Aperture,
        pickup_gap_m: 0.0005,
        pickup_offset_m: 0.0005,
        decay_seconds: 20.0,
        bar_partial_decay_seconds: 2.3,
        bar_partial_strike_weight: -0.3 * 0.2582 * 0.2582,
        ..Profile::default()
    }
}
// Coordinates are logarithms of positive physical/profile parameters.
fn profile(x: [f64; 4]) -> Profile {
    Profile {
        pickup_gap_m: x[0].exp().clamp(0.0005, 0.003),
        pickup_offset_m: x[1].exp(),
        maximum_hammer_speed_m_s: x[2].exp(),
        velocity_exponent: x[3].exp(),
        ..baseline()
    }
}
pub(crate) fn load(source: &Path, notes: &[u8]) -> Result<Vec<Case>> {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let mut cases = Vec::new();
    for &note in notes {
        for (layer, velocity) in [("p", 0.25), ("mp", 0.45), ("mf", 0.65), ("f", 0.85)] {
            let name = format!(
                "{}{}-{layer}.wav",
                names[usize::from(note % 12)],
                note / 12 - 2
            );
            let clip = AudioClip::open(source.join(name), None)?;
            let onset = detect_timbre_onset(&clip)?;
            cases.push(Case {
                note,
                layer,
                velocity,
                clip,
                onset,
            });
        }
    }
    Ok(cases)
}
fn render(c: &Case, p: Profile, seconds: f64) -> Result<AudioClip> {
    let mut engine = Engine::new(48000.0, p)?;
    engine.set_gain(0.1);
    engine.set_level_compensation(true);
    engine.reset();
    engine.note_on(0, c.note, c.velocity);
    let samples = (0..(48000.0 * seconds) as usize)
        .map(|_| f64::from(engine.next_sample()))
        .collect();
    if engine.faults() != 0 {
        return Err("engine fault during search".into());
    }
    Ok(AudioClip::from_samples(48000, samples)?)
}
fn envelope(clip: &AudioClip, frequency: f64, onset: f64) -> Result<Value> {
    let mut value = serde_json::to_value(measure_component_envelope(
        clip,
        EnvelopeOptions {
            frequency_hz: frequency,
            start_seconds: onset + 0.5,
            end_seconds: onset + 4.5,
            window_seconds: 0.512,
            hop_seconds: 0.128,
            known_neighbor_frequencies_hz: vec![],
        },
    )?)?;
    value.as_object_mut().unwrap().remove("points");
    Ok(value)
}
pub(crate) fn score(cases: &[Case], p: Profile, detailed: bool) -> Result<(f64, Vec<Value>)> {
    score_with(cases, p, detailed, render)
}
pub(crate) fn score_with(
    cases: &[Case],
    p: Profile,
    detailed: bool,
    renderer: impl Fn(&Case, Profile, f64) -> Result<AudioClip>,
) -> Result<(f64, Vec<Value>)> {
    let mut rows = Vec::new();
    let mut total = 0.0;
    let mut scored_cases = 0;
    for c in cases {
        let candidate = renderer(c, p, if detailed { 5.0 } else { 0.7 })?;
        let onset = detect_timbre_onset(&candidate)?;
        let comparison = compare_tone(
            &c.clip,
            &candidate,
            ToneComparisonOptions {
                note: c.note,
                reference_start_seconds: c.onset,
                candidate_start_seconds: onset,
            },
        )?;
        let value = serde_json::to_value(&comparison)?;
        let mut squared = 0.0;
        let mut count = 0;
        let mut balances = Vec::new();
        // Equal weight to attack96 and body, H2..H4. Missing candidate peaks
        // receive the floor; missing reference peaks are explicitly excluded.
        for wi in [1, 2] {
            for hi in 1..4 {
                let h = &value["windows"][wi]["harmonics"][hi];
                if let Some(reference) = h["reference_relative_to_fundamental_db"].as_f64() {
                    let predicted = h["candidate_relative_to_fundamental_db"]
                        .as_f64()
                        .unwrap_or(-60.0);
                    let error = predicted.max(-60.0) - reference.max(-60.0);
                    squared += error * error;
                    count += 1;
                    balances.push(json!({"window":wi,"harmonic":hi+1,"reference_db":reference,
                        "candidate_db":h["candidate_relative_to_fundamental_db"],"error_db":error}));
                }
            }
        }
        if count == 0 {
            rows.push(
                json!({"note":c.note,"layer":c.layer,"model_velocity":c.velocity,
                "harmonic_terms":0,"harmonic_mse_db2":null,
                "exclusion_reason":"no_reference_harmonics_for_objective"}),
            );
            continue;
        }
        scored_cases += 1;
        let mse = squared / f64::from(count);
        total += mse;
        let mut row = json!({"note":c.note,"layer":c.layer,"model_velocity":c.velocity,
            "harmonic_terms":count,"harmonic_mse_db2":mse,"balances":balances});
        if detailed {
            let f = 440.0 * 2.0_f64.powf((f64::from(c.note) - 69.0) / 12.0);
            if (40..=90).contains(&c.note) {
                let anchor = pitch_anchor(&c.clip, c.note)?;
                row["reference_pitch_anchor"] = serde_json::to_value(&anchor)?;
                if anchor.qualified {
                    row["reference_envelope"] = envelope(
                        &c.clip,
                        anchor.frequency_hz.ok_or("missing qualified pitch")?,
                        c.onset,
                    )?;
                }
            } else {
                row["reference_pitch_anchor"] = json!({"qualified":false,
                    "rejection_reasons":["outside_supported_anchor_range_40_90"]});
            }
            row["candidate_envelope"] = envelope(&candidate, f, onset)?;
        }
        rows.push(row);
    }
    if scored_cases == 0 {
        return Err("no scorable cases".into());
    }
    Ok((total / f64::from(scored_cases), rows))
}
pub(crate) fn write(path: &Path, value: &Value) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(&serde_json::to_vec_pretty(value)?)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: matts_fit SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY [FROZEN_SEARCH_JSON]".into(),
        );
    }
    let out = Path::new(&args[2]);
    fs::create_dir(out)?;
    let source = Path::new(&args[1]);
    // Fixed before search; held-out audio is loaded only after choosing the winner.
    let training_notes = [50, 55, 59];
    let held_out_notes = [36, 43, 64, 72, 84, 96];
    let training = load(source, &training_notes)?;
    let base = [0.0005_f64.ln(), 0.0005_f64.ln(), 0.8_f64.ln(), 1.4_f64.ln()];
    let bounds = [
        (0.0005_f64, 0.003_f64),
        (0.0001, 0.003),
        (0.2, 1.6),
        (0.5, 3.0),
    ];
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training_notes,"held_out_notes":held_out_notes,
        "objective":"Mean per-case squared dB error, H2-H4 attack96/body, -60 dB floor; missing reference excluded, missing candidate floored",
        "search":"Three deterministic coordinate rounds; log steps 0.5, 0.25, 0.125; no held-out feedback",
        "bounds":bounds,"coordinates":["gap_m","offset_m","max_hammer_speed_m_s","velocity_exponent"],
        "promotion_rule":"At least 10% training and held-out harmonic MSE reduction; no held-out note MSE regression >10%; no newly failed envelope gate among qualified reference/baseline cases; mean qualified envelope absolute slope error must not worsen by >0.5 dB/s",
        "scope":"Ordinal source layers only; fitted velocity law is not a measured hammer law. No release/T60 inference."}),
    )?;
    let (baseline_score, _) = score(&training, baseline(), false)?;
    let mut best = base;
    let mut best_score = baseline_score;
    let mut history = Vec::new();
    let steps = if let Some(path) = args.get(3) {
        let frozen: Value = serde_json::from_slice(&fs::read(path)?)?;
        for i in 0..4 {
            best[i] = frozen["winner_coordinates"][i]
                .as_f64()
                .ok_or("invalid frozen coordinate")?
                .ln();
        }
        best_score = frozen["winner_mse_db2"]
            .as_f64()
            .ok_or("invalid frozen score")?;
        history = frozen["history"]
            .as_array()
            .ok_or("missing frozen history")?
            .clone();
        vec![]
    } else {
        vec![0.5, 0.25, 0.125]
    };
    for step in steps {
        for dim in 0..4 {
            let center = best;
            for sign in [-1.0, 1.0] {
                let mut trial = center;
                trial[dim] =
                    (trial[dim] + sign * step).clamp(bounds[dim].0.ln(), bounds[dim].1.ln());
                let (loss, _) = score(&training, profile(trial), false)?;
                history.push(json!({"coordinates":trial.map(f64::exp),"mse_db2":loss}));
                if loss < best_score {
                    best = trial;
                    best_score = loss;
                }
                println!(
                    "Search {}: MSE {:.4}; best {:.4}",
                    history.len(),
                    loss,
                    best_score
                );
            }
        }
    }
    write(
        &out.join("search.json"),
        &json!({"baseline_mse_db2":baseline_score,"winner_mse_db2":best_score,
        "winner_coordinates":best.map(f64::exp),"winner_profile":format!("{:?}",profile(best)),"history":history}),
    )?;
    for (split, cases) in [
        ("training", training),
        ("held_out", load(source, &held_out_notes)?),
    ] {
        for (name, p) in [("baseline", baseline()), ("candidate", profile(best))] {
            let (loss, rows) = score(&cases, p, true)?;
            write(
                &out.join(format!("{split}-{name}.json")),
                &json!({"split":split,"profile":format!("{p:?}"),"harmonic_mse_db2":loss,"cases":rows}),
            )?;
            println!("Validated {split} {name}: MSE {loss:.4}");
        }
    }
    Ok(())
}
