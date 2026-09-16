//! Bounded upper-register geometry using the existing Calibrated pickup.
use super::robust::{Merit, assess};
use super::velocity_mapping::assign;
use super::*;
const MAPPING: [f64; 4] = [0.29, 0.44, 0.61, 0.85];
pub(super) fn control(args: &[String]) -> Result<()> {
    if args.len() != 3 {
        return Err("usage: continuous_pickup_fit --upper-control SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY FROZEN_SEARCH".into());
    }
    let frozen: Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if frozen["held_out_loaded"] != true || frozen["mapping"] != json!(MAPPING) {
        return Err(
            "Control requires an already frozen, validated search with matching mapping".into(),
        );
    }
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    for (split, notes) in [
        ("training", vec![43, 47, 50, 55, 59, 64, 72]),
        ("held_out", vec![45, 53, 60, 67, 76, 86]),
    ] {
        let mut cases = load(Path::new(&args[0]), &notes)?;
        assign(&mut cases, MAPPING);
        let (mse, rows) = score(&cases, baseline(), true)?;
        write(
            &out.join(format!("{split}-baseline.json")),
            &json!({"harmonic_mse_db2":mse,"cases":rows}),
        )?;
    }
    write(
        &out.join("protocol.json"),
        &json!({"frozen_search":args[2],"mapping":MAPPING,"scope":"Post-freeze mapping-matched Calibrated control; no fitting or candidate reselection"}),
    )?;
    Ok(())
}
fn shaped(note: u8, x: [f64; 3]) -> Profile {
    let t = ((f64::from(note) - 55.0) / 17.0).clamp(0.0, 1.0);
    let w = t * t * (3.0 - 2.0 * t);
    let p = baseline();
    Profile {
        pickup_gap_m: p.pickup_gap_m * (w * x[0]).exp(),
        pickup_offset_m: p.pickup_offset_m * (w * x[1]).exp(),
        pickup_pole_radius_m: p.pickup_pole_radius_m * (w * x[2]).exp(),
        ..p
    }
}
fn evaluate(cases: &[Case], x: [f64; 3], detailed: bool) -> Result<(f64, Vec<Value>)> {
    score_with(cases, baseline(), detailed, |c, _, s| {
        previous::render(c, shaped(c.note, x), s)
    })
}
fn h2_body(rows: &[Value]) -> Vec<Value> {
    rows.iter().filter(|r|r["note"]==72).map(|r|json!({"layer":r["layer"],
        "balance":r["balances"].as_array().unwrap().iter().find(|b|b["window"]==2&&b["harmonic"]==2)})).collect()
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: continuous_pickup_fit --upper-register SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY [PRIOR_SEARCH]"
                .into(),
        );
    }
    let source = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":[43,47,50,55,59,64,72],
        "held_out_notes":[45,53,60,67,76,86],"mapping":MAPPING,
        "model":"Existing Calibrated discrete Aperture; mechanics and velocity mapping frozen",
        "geometry":"Three log multipliers; smoothstep MIDI55..72, zero below55, constant above72",
        "search":if args.len()==3 {"One frozen prior seed; coordinate steps 0.02/0.01/0.005; 19 evaluations"} else {"15 seeds including zero; coordinate steps 0.10/0.05; 27 evaluations including bounded duplicates"},
        "prior_search":args.get(2),
        "constraints":"Unchanged robust full-score gates versus original Calibrated; C5 body H2 tracked diagnostically",
        "held_out_policy":"Only after development passes; freeze parameters before loading",
        "scope":"Offline parameter screen, not physical identification or realtime implementation"}),
    )?;
    let mut cases = load(source, &[43, 47, 50, 55, 59, 64, 72])?;
    let (_, base) = score(&cases, baseline(), false)?;
    write(
        &out.join("development-baseline.json"),
        &json!({"cases":base}),
    )?;
    assign(&mut cases, MAPPING);
    let mut seeds = vec![[0.0; 3]];
    for (d, values) in [
        (0, vec![0.15, 0.3, 0.6]),
        (1, vec![-0.6, -0.3, -0.15, 0.15, 0.3, 0.6]),
        (2, vec![-0.6, -0.3, -0.15, 0.15, 0.3]),
    ] {
        for v in values {
            let mut x = [0.0; 3];
            x[d] = v;
            seeds.push(x);
        }
    }
    if let Some(path) = args.get(2) {
        let prior: Value = serde_json::from_slice(&fs::read(path)?)?;
        if prior["held_out_loaded"] != false || prior["mapping"] != json!(MAPPING) {
            return Err("Refinement requires matching mapping and untouched held-out notes".into());
        }
        let mut seed = [0.0; 3];
        for i in 0..3 {
            seed[i] = prior["winner_coordinates"][i]
                .as_f64()
                .ok_or("invalid prior coordinate")?;
        }
        seeds = vec![seed];
    }
    let mut best = [0.0; 3];
    let mut merit = Merit {
        violation: f64::INFINITY,
        mean_ratio: f64::INFINITY,
    };
    let mut best_rows = Vec::new();
    let mut history = Vec::new();
    let mut attempt = |x: [f64; 3]| -> Result<(Merit, Vec<Value>)> {
        let (_, rows) = evaluate(&cases, x, false)?;
        let (m, a) = assess(&base, &rows)?;
        let index = history.len();
        write(
            &out.join(format!("trial-{index:02}.json")),
            &json!({"coordinates":x,"assessment":a,"cases":rows,"c5_body_h2":h2_body(&rows)}),
        )?;
        history.push(json!({"trial":index,"coordinates":x,"assessment":a}));
        println!(
            "Upper trial {index}: violation {:.5}, mean ratio {:.5}",
            m.violation, m.mean_ratio
        );
        Ok((m, rows))
    };
    for x in seeds {
        let (m, rows) = attempt(x)?;
        if m.better_than(merit) {
            best = x;
            merit = m;
            best_rows = rows;
        }
    }
    let steps = if args.len() == 3 {
        vec![0.02, 0.01, 0.005]
    } else {
        vec![0.1, 0.05]
    };
    for step in steps {
        for d in 0..3 {
            let center = best;
            for sign in [-1.0, 1.0] {
                let mut x = center;
                x[d] += sign * step;
                let limits = [
                    (0.0, 2.0_f64.ln()),
                    (-2.0_f64.ln(), 2.0_f64.ln()),
                    (-2.0_f64.ln(), 1.5_f64.ln()),
                ];
                x[d] = x[d].clamp(limits[d].0, limits[d].1);
                let (m, rows) = attempt(x)?;
                if m.better_than(merit) {
                    best = x;
                    merit = m;
                    best_rows = rows;
                }
            }
        }
    }
    let (assessment, details) = assess(&base, &best_rows)?;
    let validate = assessment.feasible();
    write(
        &out.join("search.json"),
        &json!({"winner_coordinates":best,"mapping":MAPPING,"assessment":details,"history":history,"held_out_loaded":validate,"plugin_promoted":false}),
    )?;
    write(
        &out.join("development-candidate.json"),
        &json!({"cases":best_rows}),
    )?;
    if validate {
        for (split, mut rows) in [
            ("training", load(source, &[43, 47, 50, 55, 59, 64, 72])?),
            ("held_out", load(source, &[45, 53, 60, 67, 76, 86])?),
        ] {
            let (mse, base) = score(&rows, baseline(), true)?;
            write(
                &out.join(format!("{split}-baseline.json")),
                &json!({"harmonic_mse_db2":mse,"cases":base}),
            )?;
            assign(&mut rows, MAPPING);
            let (mse, candidate) = evaluate(&rows, best, true)?;
            write(
                &out.join(format!("{split}-candidate.json")),
                &json!({"harmonic_mse_db2":mse,"cases":candidate}),
            )?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upper_shape_preserves_lower_keys_and_stays_within_profile_limits() {
        let x = [2.0_f64.ln(), -2.0_f64.ln(), 1.5_f64.ln()];
        for n in 28..=100 {
            let p = shaped(n, x);
            rf_73_dsp::Engine::new(48000.0, p).unwrap();
            if n <= 55 {
                assert_eq!(p.pickup_gap_m, baseline().pickup_gap_m);
                assert_eq!(p.pickup_offset_m, baseline().pickup_offset_m);
                assert_eq!(p.pickup_pole_radius_m, baseline().pickup_pole_radius_m);
            }
            if n >= 72 {
                assert_eq!(p.pickup_gap_m, shaped(72, x).pickup_gap_m);
            }
        }
    }
}
