//! Shared ordinal layer mapping as a nuisance variable, not a new MIDI response.
use super::robust::{Merit, assess};
use super::*;
const FIXED: [f64; 4] = [0.25, 0.45, 0.65, 0.85];
fn layer_index(layer: &str) -> usize {
    match layer {
        "p" => 0,
        "mp" => 1,
        "mf" => 2,
        "f" => 3,
        _ => unreachable!(),
    }
}
fn combinations(n: usize) -> Vec<[usize; 4]> {
    let mut rows = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                for d in c + 1..n {
                    rows.push([a, b, c, d]);
                }
            }
        }
    }
    rows
}
type Selection = ([f64; 4], Merit, Value, Vec<Value>, usize);
fn select(grid: &[Vec<Value>], choices: &[Vec<f64>; 4], base: &[Value]) -> Result<Selection> {
    let mut best = Merit {
        violation: f64::INFINITY,
        mean_ratio: f64::INFINITY,
    };
    let mut mapping = FIXED;
    let mut assessment = Value::Null;
    let mut selected = Vec::new();
    let mut count = 0;
    for a in 0..choices[0].len() {
        for b in 0..choices[1].len() {
            for c in 0..choices[2].len() {
                for d in 0..choices[3].len() {
                    let indices = [a, b, c, d];
                    let values = std::array::from_fn(|i| choices[i][indices[i]]);
                    if !values.windows(2).all(|w| w[0] < w[1]) {
                        continue;
                    }
                    let rows: Vec<_> = base
                        .iter()
                        .enumerate()
                        .map(|(i, r)| {
                            grid[indices[layer_index(r["layer"].as_str().unwrap())]][i].clone()
                        })
                        .collect();
                    let (m, detail) = assess(base, &rows)?;
                    count += 1;
                    if m.better_than(best) {
                        best = m;
                        mapping = values;
                        assessment = detail;
                        selected = rows;
                    }
                }
            }
        }
    }
    if count == 0 {
        return Err("no strictly ordered mapping".into());
    }
    Ok((mapping, best, assessment, selected, count))
}
fn evaluate(
    cases: &[Case],
    coordinates: [f64; 8],
    family: &str,
    detailed: bool,
) -> Result<(f64, Vec<Value>)> {
    if family == "calibrated" {
        score(cases, baseline(), detailed)
    } else {
        register::evaluate(cases, coordinates, detailed)
    }
}
fn assign(cases: &mut [Case], mapping: [f64; 4]) {
    for c in cases {
        c.velocity = mapping[layer_index(c.layer)];
    }
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if !(3..=4).contains(&args.len()) {
        return Err("usage: continuous_pickup_fit --velocity-map SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY PRIOR_REGISTER_SEARCH [PRIOR_MAPPING_DIRECTORY]".into());
    }
    let source = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    fs::create_dir(out)?;
    let prior: Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if prior["held_out_loaded"] != false {
        return Err("mapping study requires untouched holdouts".into());
    }
    let mut coordinates = [0.0; 8];
    for i in 0..8 {
        coordinates[i] = prior["winner_coordinates"][i]
            .as_f64()
            .ok_or("invalid register coordinate")?;
    }
    if let Some(dir) = args.get(3) {
        let decision: Value =
            serde_json::from_slice(&fs::read(Path::new(dir).join("decision.json"))?)?;
        if decision["held_out_loaded"] != false {
            return Err("refinement requires untouched holdouts".into());
        }
    }
    let training = [43, 47, 50, 55, 59, 64, 72];
    let reserved = [45, 53, 60, 67, 76, 86];
    let velocities: Vec<f64> = (2..=20).map(|i| f64::from(i) / 20.0).collect();
    let mappings = combinations(velocities.len());
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training,"held_out_notes":reserved,
        "prior_register_search":args[2],"velocity_grid":velocities,"coarse_grid_mappings_per_family":mappings.len(),
        "families":["calibrated","register"],"fixed_pairing":FIXED,"refinement_source":args.get(3),"fine_grid":"If refining: each selected layer +/-0.05 at 0.01 spacing; shared and strictly ordered",
        "selection":"Same constraint-first assessment against original Calibrated fixed pairing; independent shared strictly increasing layer map for each frozen family",
        "held_out_policy":"Only load if at least one family passes development; freeze both mappings before loading",
        "scope":"Ordinal matching sensitivity, not measured strike identification or a changed MIDI/velocity law. No plugin promotion follows from fitting source labels alone."}),
    )?;
    let mut cases = load(source, &training)?;
    let (baseline_mse, base_rows) = score(&cases, baseline(), false)?;
    write(
        &out.join("development-fixed-baseline.json"),
        &json!({"harmonic_mse_db2":baseline_mse,"cases":base_rows}),
    )?;
    let mut selected = Vec::new();
    for family in ["calibrated", "register"] {
        let choices: [Vec<f64>; 4] = if let Some(dir) = args.get(3) {
            let seed: Value = serde_json::from_slice(&fs::read(
                Path::new(dir).join(format!("{family}-selection.json")),
            )?)?;
            let mut mapping = [0.0; 4];
            for i in 0..4 {
                mapping[i] = seed["mapping"][i].as_f64().ok_or("invalid mapping seed")?;
            }
            std::array::from_fn(|i| {
                (-5..=5)
                    .map(|j| (mapping[i] + f64::from(j) * 0.01).clamp(0.1, 1.0))
                    .collect()
            })
        } else {
            std::array::from_fn(|_| velocities.clone())
        };
        let mut grid = Vec::new();
        for (index, _) in choices[0].iter().enumerate() {
            let v = std::array::from_fn(|i| choices[i][index]);
            assign(&mut cases, v);
            let (_, rows) = evaluate(&cases, coordinates, family, false)?;
            grid.push(rows);
            println!("Measured {family} velocity row {v:?}");
        }
        let (best_mapping, best, best_assessment, best_rows, evaluated) =
            select(&grid, &choices, &base_rows)?;
        let compact:Vec<_>=grid.iter().enumerate().map(|(i,rows)|json!({"layer_velocities":choices.iter().map(|v|v[i]).collect::<Vec<_>>(),
            "cases":rows.iter().map(|r|json!({"note":r["note"],"layer":r["layer"],"harmonic_mse_db2":r["harmonic_mse_db2"]})).collect::<Vec<_>>()})).collect();
        write(
            &out.join(format!("{family}-grid.json")),
            &json!({"rows":compact}),
        )?;
        write(
            &out.join(format!("{family}-selection.json")),
            &json!({"mapping":best_mapping,"assessment":best_assessment,"cases":best_rows,"evaluated_mappings":evaluated}),
        )?;
        println!(
            "Selected {family}: {:?}, violation {:.4}",
            best_mapping, best.violation
        );
        selected.push((family, best_mapping, best));
    }
    let validate = selected.iter().any(|(_, _, m)| m.feasible());
    write(
        &out.join("decision.json"),
        &json!({"plugin_promoted":false,"held_out_loaded":validate,
        "reason":if validate {"Mappings frozen; evaluate held-out sensitivity, not a plugin change"} else {"Neither frozen family passes development constraints after shared ordinal remapping"}}),
    )?;
    if !validate {
        return Ok(());
    }
    for (split, mut cases) in [("training", cases), ("held_out", load(source, &reserved)?)] {
        assign(&mut cases, FIXED);
        let (mse, rows) = score(&cases, baseline(), true)?;
        write(
            &out.join(format!("{split}-fixed.json")),
            &json!({"harmonic_mse_db2":mse,"cases":rows}),
        )?;
        for (family, mapping, _) in &selected {
            assign(&mut cases, *mapping);
            let (mse, rows) = evaluate(&cases, coordinates, family, true)?;
            write(
                &out.join(format!("{split}-{family}.json")),
                &json!({"harmonic_mse_db2":mse,"mapping":mapping,"cases":rows}),
            )?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enumerates_every_strictly_ordered_mapping_once() {
        let rows = combinations(19);
        assert_eq!(rows.len(), 3876);
        let distinct: std::collections::BTreeSet<_> = rows.iter().copied().collect();
        assert_eq!(rows.len(), distinct.len());
        assert!(rows.iter().all(|r| r.windows(2).all(|p| p[0] < p[1])));
        assert!(rows.contains(&[3, 7, 11, 15]));
    }
    #[test]
    fn selector_recovers_known_shared_mapping_with_order_constraint() {
        let layers = ["p", "mp", "mf", "f"];
        let mut base = Vec::new();
        for note in [47, 64] {
            for layer in layers {
                base.push(json!({"note":note,"layer":layer,"harmonic_mse_db2":100.0}));
            }
        }
        let choices =
            std::array::from_fn(|_| (1..=9).map(|i| f64::from(i) / 10.0).collect::<Vec<_>>());
        let grid: Vec<_> = (0..9)
            .map(|v| {
                base.iter()
                    .map(|b| {
                        let layer = layer_index(b["layer"].as_str().unwrap());
                        let target = [1.0, 3.0, 5.0, 7.0][layer];
                        let mut row = b.clone();
                        row["harmonic_mse_db2"] = json!(10.0 + 20.0 * (v as f64 - target).powi(2));
                        row
                    })
                    .collect()
            })
            .collect();
        let (mapping, merit, _, _, count) = select(&grid, &choices, &base).unwrap();
        assert_eq!(mapping, [0.2, 0.4, 0.6, 0.8]);
        assert!(merit.feasible());
        assert_eq!(count, 126);
    }
}
