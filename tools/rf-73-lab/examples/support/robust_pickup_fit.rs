//! Constraint-first selection; held-out audio is not loaded for an infeasible winner.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
struct Merit {
    violation: f64,
    mean_ratio: f64,
}
impl Merit {
    fn feasible(self) -> bool {
        self.violation <= 1.0
    }
    fn better_than(self, other: Self) -> bool {
        match (self.feasible(), other.feasible()) {
            (true, false) => true,
            (false, true) => false,
            (true, true) => self.mean_ratio < other.mean_ratio,
            (false, false) => self.violation < other.violation,
        }
    }
}
fn assess(base: &[Value], candidate: &[Value]) -> Result<(Merit, Value)> {
    let mut notes: BTreeMap<u64, (f64, f64, usize)> = BTreeMap::new();
    let mut critical = Vec::new();
    let mut maximum = 0.0_f64;
    if base.len() != candidate.len() || base.is_empty() {
        return Err("mismatched or empty cases".into());
    }
    for (a, b) in base.iter().zip(candidate) {
        if a["note"] != b["note"] || a["layer"] != b["layer"] {
            return Err("mismatched case order".into());
        }
        let av = a["harmonic_mse_db2"]
            .as_f64()
            .ok_or("missing baseline objective")?;
        let bv = b["harmonic_mse_db2"]
            .as_f64()
            .ok_or("missing candidate objective")?;
        if !av.is_finite() || !bv.is_finite() || av <= 0.0 || bv < 0.0 {
            return Err("invalid objective".into());
        }
        let note = a["note"].as_u64().ok_or("invalid note")?;
        let sums = notes.entry(note).or_default();
        sums.0 += av;
        sums.1 += bv;
        sums.2 += 1;
        if (note == 64 && a["layer"] == "f") || (note == 47 && a["layer"] == "mp") {
            maximum = maximum.max(bv / av);
            critical.push(json!({"note":note,"layer":a["layer"],"ratio":bv/av,
                "baseline_mse_db2":av,"candidate_mse_db2":bv}));
        }
    }
    if critical.len() != 2 {
        return Err("missing critical cases".into());
    }
    let mut note_rows = Vec::new();
    let (mut a_total, mut b_total) = (0.0, 0.0);
    for (note, (a, b, count)) in notes {
        maximum = maximum.max(b / a);
        a_total += a;
        b_total += b;
        note_rows.push(json!({"note":note,"cases":count,"ratio":b/a,
            "baseline_mse_db2":a/count as f64,"candidate_mse_db2":b/count as f64}));
    }
    let mean_ratio = b_total / a_total;
    let merit = Merit {
        violation: (maximum / 1.1).max(mean_ratio / 0.9),
        mean_ratio,
    };
    Ok((
        merit,
        json!({"feasible":merit.feasible(),"constraint_violation":merit.violation,
        "mean_ratio":mean_ratio,"maximum_note_or_critical_ratio":maximum,
        "notes":note_rows,"critical_cases":critical}),
    ))
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: continuous_pickup_fit --robust SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY [PRIOR_SEARCH_FOR_REFINEMENT]".into(),
        );
    }
    let source = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    let training = [43, 47, 50, 55, 59, 64, 72];
    let reserved = [45, 53, 60, 67, 76, 86];
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training,"held_out_notes":reserved,
        "critical_cases":[[64,"f"],[47,"mp"]],"objective":"Same H2-H4 spectral MSE as previous continuous study",
        "selection":"Constraint-first: minimize maximum normalized violation until feasible, then mean MSE ratio",
        "constraints":{"each_training_note_max_ratio":1.1,"each_critical_case_max_ratio":1.1,"global_max_ratio":0.9},
        "search":if args.len()==2 {"Six seeds plus four coordinate rounds; 46 trials"} else {"Prior seed; coordinate steps 0.025/0.0125; all signed coordinate pairs at steps 0.05/0.025; 101 trials"},
        "refinement_source":args.get(2),
        "held_out_policy":"Only load reserved notes when training constraints and numerical qualification pass",
        "held_out_gates":"Unchanged matts_fit_gate aggregate, per-note, coverage and envelope policy",
        "scope":"Bounded local search, not global infeasibility proof; ordinal layer velocities remain unknown"}),
    )?;
    let cases = load(source, &training)?;
    let (_, base_rows) = score(&cases, baseline(), false)?;
    write(
        &out.join("development-baseline.json"),
        &json!({"cases":base_rows}),
    )?;
    let mut seeds: Vec<[f64; 5]> = vec![
        [
            0.0008187307530779819,
            0.00075,
            0.0006703200460356392,
            1.4,
            1.4,
        ],
        [0.0005, 0.0005, 0.002, 0.8, 1.4],
        [0.0005, 0.00075, 0.0002, 1.6, 1.4],
        [0.0005, 0.00075, 0.0005, 1.2, 1.4],
        [0.001, 0.001, 0.0005, 1.6, 1.0],
        [0.0007, 0.0015, 0.001, 1.0, 2.0],
    ];
    if let Some(path) = args.get(2) {
        let prior: Value = serde_json::from_slice(&fs::read(path)?)?;
        if prior["held_out_loaded"] != false {
            return Err("refinement requires a prior run with untouched holdouts".into());
        }
        let mut p = [0.0; 5];
        for i in 0..5 {
            p[i] = prior["winner_parameters"][i]
                .as_f64()
                .ok_or("invalid seed")?;
        }
        seeds = vec![p];
    }
    let mut best = seeds[0].map(f64::ln);
    let mut merit = Merit {
        violation: f64::INFINITY,
        mean_ratio: f64::INFINITY,
    };
    let mut best_rows = Vec::new();
    let mut best_assessment = Value::Null;
    let mut history = Vec::new();
    // Persist every trial's aggregate and constraints; only the best needs full spectra.
    for seed in seeds {
        let x = seed.map(f64::ln);
        let (_, rows) = trial(&cases, x, false)?;
        let (m, assessment) = assess(&base_rows, &rows)?;
        history.push(json!({"parameters":params(x),"assessment":assessment}));
        if m.better_than(merit) {
            best = x;
            merit = m;
            best_rows = rows;
            best_assessment = assessment;
        }
        println!(
            "Seed {}: violation {:.4}, best {:.4}",
            history.len(),
            m.violation,
            merit.violation
        );
    }
    let steps = if args.len() == 2 {
        vec![0.4, 0.2, 0.1, 0.05]
    } else {
        vec![0.025, 0.0125]
    };
    for step in steps {
        for d in 0..5 {
            let center = best;
            for sign in [-1.0, 1.0] {
                let mut x = center;
                x[d] = (x[d] + step * sign).clamp(BOUNDS[d].0.ln(), BOUNDS[d].1.ln());
                let (_, rows) = trial(&cases, x, false)?;
                let (m, assessment) = assess(&base_rows, &rows)?;
                history.push(json!({"parameters":params(x),"assessment":assessment}));
                if m.better_than(merit) {
                    best = x;
                    merit = m;
                    best_rows = rows;
                    best_assessment = assessment;
                }
                println!(
                    "Trial {}: violation {:.4}, best {:.4}",
                    history.len(),
                    m.violation,
                    merit.violation
                );
            }
        }
    }
    if args.len() == 3 {
        for step in [0.05, 0.025] {
            for i in 0..5 {
                for j in i + 1..5 {
                    let center = best;
                    for (si, sj) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                        let mut x = center;
                        x[i] = (x[i] + step * si).clamp(BOUNDS[i].0.ln(), BOUNDS[i].1.ln());
                        x[j] = (x[j] + step * sj).clamp(BOUNDS[j].0.ln(), BOUNDS[j].1.ln());
                        let (_, rows) = trial(&cases, x, false)?;
                        let (m, assessment) = assess(&base_rows, &rows)?;
                        history.push(json!({"parameters":params(x),"assessment":assessment}));
                        if m.better_than(merit) {
                            best = x;
                            merit = m;
                            best_rows = rows;
                            best_assessment = assessment;
                        }
                        println!(
                            "Pair trial {}: violation {:.4}, best {:.4}",
                            history.len(),
                            m.violation,
                            merit.violation
                        );
                    }
                }
            }
        }
    }
    let qualification = Table::new(profile(best)).qualification(profile(best))?;
    let validate = merit.feasible() && qualification["qualified"] == true;
    write(
        &out.join("search.json"),
        &json!({"winner_parameters":params(best),"assessment":best_assessment,
        "numerical_qualification":qualification,"history":history,"held_out_loaded":validate}),
    )?;
    write(
        &out.join("development-candidate.json"),
        &json!({"cases":best_rows}),
    )?;
    if !validate {
        write(
            &out.join("decision.json"),
            &json!({"promote":false,"held_out_loaded":false,
            "reason":"No selected candidate satisfies development and numerical gates; reserved notes remain unused"}),
        )?;
        println!("No feasible candidate; reserved validation audio was not loaded");
        return Ok(());
    }
    for (split, cases) in [("training", cases), ("held_out", load(source, &reserved)?)] {
        for name in ["baseline", "candidate"] {
            let (mse, rows) = if name == "baseline" {
                score(&cases, baseline(), true)?
            } else {
                trial(&cases, best, true)?
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
    fn rows(a: f64, b: f64) -> Vec<Value> {
        vec![
            json!({"note":64,"layer":"f","harmonic_mse_db2":a}),
            json!({"note":47,"layer":"mp","harmonic_mse_db2":b}),
        ]
    }
    #[test]
    fn average_gain_cannot_hide_critical_regression() {
        let (m, _) = assess(&rows(100.0, 100.0), &rows(1.0, 120.0)).unwrap();
        assert!(!m.feasible());
        assert!(m.mean_ratio < 0.9);
    }
    #[test]
    fn feasible_candidate_beats_lower_mean_with_regression() {
        let (good, _) = assess(&rows(100.0, 100.0), &rows(80.0, 85.0)).unwrap();
        let (bad, _) = assess(&rows(100.0, 100.0), &rows(1.0, 120.0)).unwrap();
        assert!(good.feasible());
        assert!(good.better_than(bad));
        assert!(!bad.better_than(good));
    }
    #[test]
    fn missing_critical_case_is_not_accepted() {
        let mut incomplete = rows(100.0, 100.0);
        incomplete.pop();
        assert!(assess(&incomplete, &incomplete).is_err());
    }
    #[test]
    fn noncritical_note_regression_is_also_rejected() {
        let mut base = rows(100.0, 100.0);
        let mut candidate = rows(1.0, 1.0);
        base.push(json!({"note":55,"layer":"p","harmonic_mse_db2":10.0}));
        candidate.push(json!({"note":55,"layer":"p","harmonic_mse_db2":20.0}));
        let (m, _) = assess(&base, &candidate).unwrap();
        assert!(m.mean_ratio < 0.9);
        assert!(!m.feasible());
    }
    #[test]
    fn ten_percent_global_improvement_is_required() {
        assert!(
            assess(&rows(100.0, 100.0), &rows(90.0, 90.0))
                .unwrap()
                .0
                .feasible()
        );
        assert!(
            !assess(&rows(100.0, 100.0), &rows(95.0, 95.0))
                .unwrap()
                .0
                .feasible()
        );
    }
}
