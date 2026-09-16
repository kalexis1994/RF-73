//! Offline continuous-disk fit. Never changes the realtime pickup or presets.
#[allow(dead_code)]
#[path = "matts_fit.rs"]
mod previous;
#[path = "support/register_pickup_fit.rs"]
mod register;
#[path = "support/robust_pickup_fit.rs"]
mod robust;
#[path = "support/velocity_mapping.rs"]
mod velocity_mapping;
use previous::{Case, Result, baseline, load, score, score_with, write};
use rf_73_analysis::{AudioClip, ToneComparisonOptions, compare_tone, detect_timbre_onset};
use rf_73_dsp::{ProductionDecimator, Profile, Voice};
use serde_json::{Value, json};
use std::{f64::consts::TAU, fs, path::Path};

const LIMIT: f64 = 0.01;
const CELLS: usize = 8192;
const BOUNDS: [(f64, f64); 5] = [
    (0.0005, 0.003),
    (0.0001, 0.003),
    (0.0002, 0.003),
    (0.2, 1.6),
    (0.5, 3.0),
];
fn params(x: [f64; 5]) -> [f64; 5] {
    std::array::from_fn(|i| x[i].exp().clamp(BOUNDS[i].0, BOUNDS[i].1))
}
fn profile(x: [f64; 5]) -> Profile {
    let p = params(x);
    Profile {
        pickup_gap_m: p[0],
        pickup_offset_m: p[1],
        pickup_pole_radius_m: p[2],
        maximum_hammer_speed_m_s: p[3],
        velocity_exponent: p[4],
        ..baseline()
    }
}
struct Disk {
    terms: Vec<(f64, f64, f64)>,
    factor: f64,
    offset: f64,
}
impl Disk {
    fn new(p: Profile, n: usize) -> Self {
        let r = p.pickup_pole_radius_m;
        let g = p.pickup_gap_m;
        Self {
            terms: (0..n)
                .map(|i| {
                    let (s, c) = (TAU * (i as f64 + 0.5) / n as f64).sin_cos();
                    (r * c, g * g + (r * s).powi(2), c)
                })
                .collect(),
            factor: -2.0 * 0.001 * g.powi(3) / r / n as f64,
            offset: p.pickup_offset_m,
        }
    }
    fn slope(&self, q: f64) -> f64 {
        self.factor
            * self
                .terms
                .iter()
                .map(|&(x, y, c)| {
                    let r2 = y + (q + self.offset - x).powi(2);
                    c / (r2 * r2.sqrt())
                })
                .sum::<f64>()
    }
}
struct Table {
    slopes: Vec<f64>,
}
impl Table {
    fn new(p: Profile) -> Self {
        let disk = Disk::new(p, 128);
        Self {
            slopes: (0..=CELLS)
                .map(|i| disk.slope(-LIMIT + 2.0 * LIMIT * i as f64 / CELLS as f64))
                .collect(),
        }
    }
    fn slope(&self, q: f64) -> Result<f64> {
        if !q.is_finite() || !(-LIMIT..=LIMIT).contains(&q) {
            return Err("trajectory outside pickup table".into());
        }
        let u = (q + LIMIT) / (2.0 * LIMIT) * CELLS as f64;
        let i = (u.floor() as usize).min(CELLS - 1);
        let t = u - i as f64;
        Ok(self.slopes[i] * (1.0 - t) + self.slopes[i + 1] * t)
    }
    fn render(&self, c: &Case, p: Profile, seconds: f64) -> Result<AudioClip> {
        render_using(c, p, seconds, |q| self.slope(q))
    }
    fn qualification(&self, p: Profile) -> Result<Value> {
        let direct = Disk::new(p, 256);
        let coarse = Disk::new(p, 128);
        let mut maximum = 0.0_f64;
        let mut error = 0.0_f64;
        let mut refine = 0.0_f64;
        for i in 0..2001 {
            let q = -LIMIT + 2.0 * LIMIT * (i as f64 + 0.371) / 2001.0;
            let expected = direct.slope(q);
            maximum = maximum.max(expected.abs());
            error = error.max((self.slope(q)? - expected).abs());
            refine = refine.max((coarse.slope(q) - expected).abs());
        }
        Ok(json!({"off_grid_points":2001,"maximum_slope":maximum,
            "max_error_over_peak_slope":error/maximum,"refinement_error_over_peak_slope":refine/maximum,
            "qualified":error/maximum<1e-4 && refine/maximum<1e-6}))
    }
}
fn render_using(
    c: &Case,
    p: Profile,
    seconds: f64,
    slope: impl Fn(f64) -> Result<f64>,
) -> Result<AudioClip> {
    let mut voice = Voice::new(48000.0, c.note, p)?;
    voice.strike(c.velocity);
    let mut filter = ProductionDecimator::new();
    let mut samples = Vec::new();
    for _ in 0..(48000.0 * seconds) as usize {
        for _ in 0..4 {
            voice.tick();
            let probe = voice.probe();
            filter.push(-15.0 * slope(probe.displacement_m)? * probe.velocity_m_s);
        }
        samples.push(filter.output() * 0.012);
    }
    Ok(AudioClip::from_samples(48000, samples)?)
}

fn trial(cases: &[Case], x: [f64; 5], detailed: bool) -> Result<(f64, Vec<Value>)> {
    let p = profile(x);
    let table = Table::new(p);
    score_with(cases, p, detailed, |c, p, s| table.render(c, p, s))
}
fn verify_rendering(source: &Path, out: &Path, frozen: &Path) -> Result<()> {
    let search: Value = serde_json::from_slice(&fs::read(frozen)?)?;
    let mut x = [0.0; 5];
    for i in 0..5 {
        x[i] = search["winner_parameters"][i]
            .as_f64()
            .ok_or("invalid frozen parameter")?
            .ln();
    }
    let p = profile(x);
    let table = Table::new(p);
    let direct = Disk::new(p, 256);
    let cases = load(source, &[40, 55, 64, 88])?;
    let mut rows = Vec::new();
    for case in cases.iter().filter(|c| c.layer == "p" || c.layer == "f") {
        let a = render_using(case, p, 0.7, |q| Ok(direct.slope(q)))?;
        let b = table.render(case, p, 0.7)?;
        let relative_rms = (a
            .samples()
            .iter()
            .zip(b.samples())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            / a.samples().iter().map(|x| x * x).sum::<f64>())
        .sqrt();
        let comparison = compare_tone(
            &a,
            &b,
            ToneComparisonOptions {
                note: case.note,
                reference_start_seconds: detect_timbre_onset(&a)?,
                candidate_start_seconds: detect_timbre_onset(&b)?,
            },
        )?;
        let mut maximum = 0.0_f64;
        let mut terms = 0;
        for w in comparison.windows.iter().skip(1) {
            for h in w.harmonics.iter().skip(1).take(3) {
                if h.reference_relative_to_fundamental_db
                    .is_some_and(|x| x > -60.0)
                {
                    let diff = h
                        .candidate_minus_reference_balance_db
                        .ok_or("table lost a measurable harmonic")?;
                    maximum = maximum.max(diff.abs());
                    terms += 1;
                }
            }
        }
        let qualified = relative_rms < 1e-4 && maximum < 0.05 && terms > 0;
        rows.push(
            json!({"note":case.note,"layer":case.layer,"relative_waveform_rms_error":relative_rms,
            "max_harmonic_balance_error_db":maximum,"measured_terms":terms,"qualified":qualified}),
        );
    }
    write(
        &out.join("rendering-qualification.json"),
        &json!({"parameters":params(x),
        "method":"Same voice and filter; table versus 256-node direct integration; no waveform alignment or gain fitting",
        "scope":"Selected frozen winner cases, not a whole-domain realtime or aliasing qualification",
        "qualified":rows.iter().all(|r|r["qualified"]==true),"cases":rows}),
    )?;
    if rows.iter().any(|r| r["qualified"] != true) {
        return Err("rendering qualification failed".into());
    }
    println!(
        "Qualified {} direct-versus-table rendering cases",
        rows.len()
    );
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--velocity-map") {
        return velocity_mapping::run(&args[2..]);
    }
    if args.get(1).is_some_and(|a| a == "--register") {
        return register::run(&args[2..]);
    }
    if args.get(1).is_some_and(|a| a == "--robust") {
        return robust::run(&args[2..]);
    }
    if !(3..=4).contains(&args.len()) {
        return Err("usage: continuous_pickup_fit SOURCE_SAMPLES NEW_OUTPUT_DIRECTORY [FROZEN_SEARCH_FOR_RENDER_CHECK]".into());
    }
    let out = Path::new(&args[2]);
    fs::create_dir(out)?;
    if let Some(frozen) = args.get(3) {
        return verify_rendering(Path::new(&args[1]), out, Path::new(frozen));
    }
    let training_notes = [43, 50, 55, 59, 64, 72];
    let held_out_notes = [40, 47, 62, 69, 79, 88];
    write(
        &out.join("protocol.json"),
        &json!({"training_notes":training_notes,"held_out_notes":held_out_notes,
        "objective":"Same H2-H4 attack96/body mean squared balance error and missing-data policy as matts_fit",
        "selection":"Three fixed seeds, then three coordinate rounds, log steps 0.4/0.2/0.1; held-out not loaded until winner frozen",
        "bounds":BOUNDS,"coordinate_order":["gap_m","offset_m","radius_m","maximum_hammer_speed_m_s","velocity_exponent"],
        "promotion_rule":"Same 10% aggregate improvement, per-note regression <=10%, and envelope gates as matts_fit_gate; full harmonic case coverage required",
        "numerical_gate":"2001 off-grid points vs 256-node boundary; table error/peak <1e-4 and 128-vs-256 error/peak <1e-6",
        "table":{"domain_m":[-LIMIT,LIMIT],"cells":CELLS,"boundary_nodes":128,"interpolation":"linear; out-of-range is an error"},
        "scope":"Offline research only; no physical strike identification or realtime qualification"}),
    )?;
    let cases = load(Path::new(&args[1]), &training_notes)?;
    let seeds: [[f64; 5]; 3] = [
        [0.0005, 0.0005, 0.002, 0.8, 1.4],
        [0.0005, 0.00075, 0.0005, 1.2, 1.4],
        [0.001, 0.00075, 0.001, 1.4, 1.4],
    ];
    let mut best = seeds[0].map(f64::ln);
    let mut loss = f64::INFINITY;
    let mut history = Vec::new();
    for seed in seeds {
        let x = seed.map(f64::ln);
        let s = trial(&cases, x, false)?.0;
        history.push(json!({"parameters":params(x),"mse_db2":s}));
        if s < loss {
            loss = s;
            best = x;
        }
        println!("Seed MSE {s:.4}; best {loss:.4}");
    }
    for step in [0.4, 0.2, 0.1] {
        for d in 0..5 {
            let center = best;
            for sign in [-1.0, 1.0] {
                let mut x = center;
                x[d] = (x[d] + sign * step).clamp(BOUNDS[d].0.ln(), BOUNDS[d].1.ln());
                let s = trial(&cases, x, false)?.0;
                history.push(json!({"parameters":params(x),"mse_db2":s}));
                if s < loss {
                    loss = s;
                    best = x;
                }
                println!("Trial {}: MSE {s:.4}; best {loss:.4}", history.len());
            }
        }
    }
    let qualification = Table::new(profile(best)).qualification(profile(best))?;
    write(
        &out.join("search.json"),
        &json!({"winner_parameters":params(best),"winner_mse_db2":loss,
        "history":history,"numerical_qualification":qualification}),
    )?;
    if qualification["qualified"] != true {
        return Err("winner fails numerical qualification".into());
    }
    for (split, cases) in [
        ("training", cases),
        ("held_out", load(Path::new(&args[1]), &held_out_notes)?),
    ] {
        for name in ["baseline", "candidate"] {
            let (loss, rows) = if name == "baseline" {
                score(&cases, baseline(), true)?
            } else {
                trial(&cases, best, true)?
            };
            write(
                &out.join(format!("{split}-{name}.json")),
                &json!({"harmonic_mse_db2":loss,"cases":rows,
                "model":name,"candidate_parameters":params(best)}),
            )?;
            println!("Validated {split}/{name}: {loss:.4}");
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolation_is_qualified_at_extreme_and_nominal_geometries() {
        for values in [
            [0.0005, 0.0001, 0.003, 0.8, 1.4],
            [0.003, 0.003, 0.0002, 0.8, 1.4],
            [0.0005, 0.0005, 0.002, 0.8, 1.4],
        ] {
            let p = profile(values.map(f64::ln));
            let table = Table::new(p);
            assert_eq!(table.qualification(p).unwrap()["qualified"], true);
            assert!(table.slope(LIMIT + 1e-9).is_err());
            assert!(table.slope(f64::NAN).is_err());
        }
    }
    #[test]
    fn disk_matches_point_limit_and_reflection() {
        let p = Profile {
            pickup_gap_m: 0.003,
            pickup_offset_m: 0.0005,
            pickup_pole_radius_m: 1e-7,
            ..baseline()
        };
        let disk = Disk::new(p, 256);
        let q = 0.0002;
        let u = q + p.pickup_offset_m;
        let expected =
            -3.0 * 0.001 * p.pickup_gap_m.powi(3) * u / (p.pickup_gap_m.powi(2) + u * u).powf(2.5);
        assert!((disk.slope(q) / expected - 1.0).abs() < 1e-7);
        assert!((disk.slope(q) + disk.slope(-q - 2.0 * p.pickup_offset_m)).abs() < 1e-10);
    }
}
