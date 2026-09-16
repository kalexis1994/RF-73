//! Two smooth register anchors; no independent per-note correction.
use super::robust::{Merit, assess};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const ANCHORS: [u8; 2] = [43, 72];
fn coordinates(x: [f64; 8], note: u8) -> [f64; 5] {
    let t = ((f64::from(note) - f64::from(ANCHORS[0])) / f64::from(ANCHORS[1] - ANCHORS[0]))
        .clamp(0.0, 1.0);
    let w = t * t * (3.0 - 2.0 * t);
    std::array::from_fn(|i| {
        if i >= 3 {
            return x[i].clamp(BOUNDS[i].0.ln(), BOUNDS[i].1.ln());
        }
        let lo = (x[i] - x[i + 5]).clamp(BOUNDS[i].0.ln(), BOUNDS[i].1.ln());
        let hi = (x[i] + x[i + 5]).clamp(BOUNDS[i].0.ln(), BOUNDS[i].1.ln());
        lo * (1.0 - w) + hi * w
    })
}
fn evaluate(cases: &[Case], x: [f64; 8], detailed: bool) -> Result<(f64, Vec<Value>)> {
    let mut tables = BTreeMap::new();
    for note in cases.iter().map(|c| c.note).collect::<BTreeSet<_>>() {
        let p = profile(coordinates(x, note));
        tables.insert(note, (Table::new(p), p));
    }
    score_with(cases, baseline(), detailed, |c, _, seconds| {
        let (table, p) = tables.get(&c.note).ok_or("missing register table")?;
        table.render(c, *p, seconds)
    })
}
fn qualification(x: [f64; 8]) -> Result<Value> {
    let mut rows = Vec::new();
    // Numerical-only check over the whole 73-key model range; no source audio.
    for note in 28..=100 {
        let p = profile(coordinates(x, note));
        rows.push(json!({"note":note,"parameters":params(coordinates(x,note)),
            "qualification":Table::new(p).qualification(p)?}));
    }
    Ok(json!({"qualified":rows.iter().all(|r|r["qualification"]["qualified"]==true),"notes":rows}))
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if args.len() != 3 {
        return Err("usage: continuous_pickup_fit --register SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY PRIOR_ROBUST_OR_REGISTER_SEARCH".into());
    }
    let source = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    let prior: Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if prior["held_out_loaded"] != false {
        return Err("register study needs untouched reserved notes".into());
    }
    let mut center = [0.0; 8];
    let refinement = prior.get("winner_coordinates").is_some();
    if refinement {
        for i in 0..8 {
            center[i] = prior["winner_coordinates"][i]
                .as_f64()
                .ok_or("invalid register coordinate")?;
        }
    } else {
        for i in 0..5 {
            center[i] = prior["winner_parameters"][i]
                .as_f64()
                .ok_or("invalid prior parameter")?
                .ln();
        }
    }
    let training = [43, 47, 50, 55, 59, 64, 72];
    let reserved = [45, 53, 60, 67, 76, 86];
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training,"held_out_notes":reserved,
        "prior_search":args[2],"anchors":ANCHORS,"geometry":"Log endpoint interpolation with smoothstep; constant outside anchors; shared velocity law",
        "free_coordinates":"Five log center parameters plus three signed geometry half-differences bounded to +/-0.5",
        "search":if refinement {"One prior seed, coordinate steps 0.02/0.01/0.005/0.0025; 65 evaluations"} else {"Seven seeds, coordinate steps 0.16/0.08/0.04; 55 evaluations"},
        "constraints":"Unchanged robust note/critical-case/global constraints; unchanged held-out and envelope gates",
        "held_out_policy":"Only load after development constraints and all-key numerical qualification pass",
        "scope":"Offline research; ordinal source velocity; no physical parameter identification or realtime certification"}),
    )?;
    let cases = load(source, &training)?;
    let (_, base_rows) = score(&cases, baseline(), false)?;
    write(
        &out.join("development-baseline.json"),
        &json!({"cases":base_rows}),
    )?;
    let mut seeds = vec![center];
    if !refinement {
        for i in 5..8 {
            for sign in [-1.0, 1.0] {
                let mut x = center;
                x[i] = sign * 0.15;
                seeds.push(x);
            }
        }
    }
    let mut best = center;
    let mut merit = Merit {
        violation: f64::INFINITY,
        mean_ratio: f64::INFINITY,
    };
    let mut best_rows = Vec::new();
    let mut assessment = Value::Null;
    let mut history = Vec::new();
    for x in seeds {
        let (_, rows) = evaluate(&cases, x, false)?;
        let (m, a) = assess(&base_rows, &rows)?;
        history.push(json!({"coordinates":x,"assessment":a}));
        if m.better_than(merit) {
            best = x;
            merit = m;
            assessment = a;
            best_rows = rows;
        }
        println!(
            "Register seed {}: violation {:.4}; best {:.4}",
            history.len(),
            m.violation,
            merit.violation
        );
    }
    let steps = if refinement {
        vec![0.02, 0.01, 0.005, 0.0025]
    } else {
        vec![0.16, 0.08, 0.04]
    };
    for step in steps {
        for d in 0..8 {
            let center = best;
            for sign in [-1.0, 1.0] {
                let mut x = center;
                x[d] += sign * step;
                x[d] = if d < 5 {
                    x[d].clamp(BOUNDS[d].0.ln(), BOUNDS[d].1.ln())
                } else {
                    x[d].clamp(-0.5, 0.5)
                };
                let (_, rows) = evaluate(&cases, x, false)?;
                let (m, a) = assess(&base_rows, &rows)?;
                history.push(json!({"coordinates":x,"assessment":a}));
                if m.better_than(merit) {
                    best = x;
                    merit = m;
                    assessment = a;
                    best_rows = rows;
                }
                println!(
                    "Register trial {}: violation {:.4}; best {:.4}",
                    history.len(),
                    m.violation,
                    merit.violation
                );
            }
        }
    }
    let numeric = qualification(best)?;
    let validate = merit.feasible() && numeric["qualified"] == true;
    write(
        &out.join("search.json"),
        &json!({"winner_coordinates":best,"assessment":assessment,"history":history,
        "numerical_qualification":numeric,"held_out_loaded":validate}),
    )?;
    write(
        &out.join("development-candidate.json"),
        &json!({"cases":best_rows}),
    )?;
    if !validate {
        write(
            &out.join("decision.json"),
            &json!({"promote":false,"held_out_loaded":false,
            "reason":"Selected smooth-register candidate fails development or numerical constraints"}),
        )?;
        println!("Register candidate not feasible; reserved audio remains unused");
        return Ok(());
    }
    for (split, cases) in [("training", cases), ("held_out", load(source, &reserved)?)] {
        for name in ["baseline", "candidate"] {
            let (mse, rows) = if name == "baseline" {
                score(&cases, baseline(), true)?
            } else {
                evaluate(&cases, best, true)?
            };
            write(
                &out.join(format!("{split}-{name}.json")),
                &json!({"harmonic_mse_db2":mse,"cases":rows}),
            )?;
            println!("Validated {split}/{name}: {mse:.4}");
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_slopes_recover_shared_profile_and_outer_keys_are_constant() {
        let x = [
            0.001_f64.ln(),
            0.001_f64.ln(),
            0.0005_f64.ln(),
            0.8_f64.ln(),
            1.4_f64.ln(),
            0.0,
            0.0,
            0.0,
        ];
        for note in 28..=100 {
            for (a, b) in coordinates(x, note).iter().zip(&x[..5]) {
                assert!((a - b).abs() < 1e-14);
            }
        }
        let mut shaped = x;
        shaped[5] = 0.2;
        shaped[6] = -0.1;
        assert_eq!(coordinates(shaped, 28), coordinates(shaped, 43));
        assert_eq!(coordinates(shaped, 100), coordinates(shaped, 72));
    }
    #[test]
    fn interpolation_is_bounded_and_monotone_without_per_note_jumps() {
        let x = [
            0.0005_f64.ln(),
            0.003_f64.ln(),
            0.001_f64.ln(),
            0.8_f64.ln(),
            1.4_f64.ln(),
            0.5,
            -0.5,
            0.3,
        ];
        let mut previous = coordinates(x, 28);
        for note in 29..=100 {
            let current = coordinates(x, note);
            for i in 0..5 {
                assert!(
                    current[i] >= BOUNDS[i].0.ln() - 1e-14
                        && current[i] <= BOUNDS[i].1.ln() + 1e-14
                );
            }
            assert!(current[0] >= previous[0]);
            assert!(current[1] <= previous[1]);
            assert!((current[0] - previous[0]).abs() < 0.03);
            previous = current;
        }
    }
}
