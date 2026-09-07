//! Known-state mechanical loss recovery from endpoint energy and velocity quadrature.
pub mod reduced;
use rf_73_dsp::{ModalAssembly, ModalAssemblyProfile, ModalIntegration, TineGeometry};
use serde::Serialize;
use serde_json::{Value, json};
use std::{error::Error, path::Path};
type Matrix = [[f64; 9]; 9];
const EDGES: [f64; 7] = [0.02, 0.06, 0.10, 0.14, 0.18, 0.22, 0.26];
pub const HELP: &str = "Controlled mechanical loss recovery:
  mechanical-loss --output REPORT.json
Nine-coordinate assembly, known strike and binary damper timing; 48/96 kHz refinement.
Two viscous scale factors from mechanical energy and independent velocity quadrature.
Held-out intervals and omitted-damper negative control; no source or pickup calibration.
";

fn quadratic(v: [f64; 9], m: &Matrix) -> f64 {
    (0..9)
        .map(|i| v[i] * (0..9).map(|j| m[i][j] * v[j]).sum::<f64>())
        .sum()
}
fn energy(q: [f64; 9], v: [f64; 9], m: &Matrix, k: &Matrix) -> f64 {
    0.5 * (quadratic(q, k) + quadratic(v, m))
}
#[derive(Clone, Serialize)]
struct Row {
    start_seconds: f64,
    end_seconds: f64,
    energy_drop_j: f64,
    structural_integral_j: f64,
    damper_integral_j: f64,
    contact_free: bool,
}
struct Take {
    rows: Vec<Row>,
    diagnostics: Value,
}

fn simulate(rate: u32, structural: f64, damper: f64) -> Result<Take, Box<dyn Error>> {
    simulate_observed(rate, structural, damper, |_, _, _, _, _| {})
}

fn simulate_observed(
    rate: u32,
    structural: f64,
    damper: f64,
    mut observe: impl FnMut(usize, usize, &rf_73_dsp::ModalProbe, &rf_73_dsp::ModalProbe, bool),
) -> Result<Take, Box<dyn Error>> {
    let geometry = TineGeometry::default();
    let baseline = ModalAssembly::new(
        rate as f64,
        geometry,
        ModalAssemblyProfile::default(),
        ModalIntegration::Refined {
            contact_substeps: 64,
        },
    )?;
    let (m, k, c) = (
        baseline.mass_matrix(),
        baseline.stiffness_matrix(),
        baseline.damping_matrix(false),
    );
    let on = baseline.damping_matrix(true);
    let d: Matrix = core::array::from_fn(|i| core::array::from_fn(|j| on[i][j] - c[i][j]));
    let mut profile = ModalAssemblyProfile::default();
    profile.translation_damping_n_s_m *= structural;
    profile.rotation_damping_n_m_s_rad *= structural;
    profile.tonebar_decay_seconds /= structural;
    for decay in &mut profile.tine_decay_seconds {
        *decay /= structural;
    }
    profile.damper_n_s_m *= damper;
    let mut voice = ModalAssembly::new(
        rate as f64,
        geometry,
        profile,
        ModalIntegration::Refined {
            contact_substeps: 64,
        },
    )?;
    if !voice.strike(0.5) {
        return Err("controlled loss strike rejected".into());
    }
    let steps = voice.steps_per_sample();
    let tick_rate = rate as usize * steps;
    let dt = 1.0 / tick_rate as f64;
    let edges = EDGES.map(|t| (t * tick_rate as f64).round() as usize);
    let release = (0.14 * tick_rate as f64).round() as usize;
    let launch = voice.probe().injected_energy_j;
    let mut rows = Vec::new();
    let mut start_energy = 0.0;
    let (mut a, mut b) = (0.0, 0.0);
    let mut contact_free = true;
    let mut separation = None;
    let mut last_contact = None;
    let mut max_balance = 0.0_f64;
    let mut max_positive = 0.0_f64;
    let mut index = 0;
    for tick in 0..*edges.last().unwrap() {
        if tick == release {
            let before = voice.probe();
            voice.set_damped(true);
            let after = voice.probe();
            if before.position != after.position || before.velocity != after.velocity {
                return Err("damper changed mechanical state instantaneously".into());
            }
        }
        let before = voice.probe();
        if tick == edges[index] {
            start_energy = energy(before.position, before.velocity, &m, &k);
            a = 0.0;
            b = 0.0;
            contact_free = true;
        }
        voice.tick();
        let after = voice.probe();
        if before.contact_active || after.contact_active {
            last_contact = Some((tick + 1) as f64 * dt);
        }
        if before.contact_active && !after.contact_active && separation.is_none() {
            separation = Some((tick + 1) as f64 * dt);
        }
        if !after.balance_residual_j.is_finite() || !after.mechanical_energy_j.is_finite() {
            return Err("non-finite controlled mechanical state".into());
        }
        max_balance = max_balance.max(after.balance_residual_j.abs() / launch);
        max_positive =
            max_positive.max((after.mechanical_energy_j - before.mechanical_energy_j) / launch);
        observe(tick, tick_rate, &before, &after, tick >= release);
        if tick >= edges[index] {
            // Endpoint trapezoidal quadrature, not the integrator's heat increment.
            a += 0.5 * dt * (quadratic(before.velocity, &c) + quadratic(after.velocity, &c));
            if tick >= release {
                b += 0.5 * dt * (quadratic(before.velocity, &d) + quadratic(after.velocity, &d));
            }
            contact_free &= !before.contact_active && !after.contact_active;
            if tick + 1 == edges[index + 1] {
                rows.push(Row {
                    start_seconds: EDGES[index],
                    end_seconds: EDGES[index + 1],
                    energy_drop_j: start_energy - energy(after.position, after.velocity, &m, &k),
                    structural_integral_j: a,
                    damper_integral_j: b,
                    contact_free,
                });
                index += 1;
            }
        }
    }
    Ok(Take {
        rows,
        diagnostics: json!({"sample_rate_hz":rate,"mechanical_tick_rate_hz":tick_rate,
        "contact_substeps":64,"strike_input":0.5,"initial_hammer_speed_m_s":profile.maximum_hammer_speed_m_s*0.5_f64.powf(1.4),
        "damper_on_seconds":0.14,"first_separation_seconds":separation,"last_contact_seconds":last_contact,
        "maximum_energy_ledger_relative_residual":max_balance,"maximum_positive_energy_step_relative":max_positive,
        "mechanical_checks_passed":separation.is_some() && max_balance<1e-8 && max_positive<1e-10}),
    })
}

fn fit(rows: &[Row]) -> Result<(f64, f64, f64), Box<dyn Error>> {
    if rows.len() < 2
        || rows.iter().any(|r| {
            !r.contact_free
                || !r.energy_drop_j.is_finite()
                || !r.structural_integral_j.is_finite()
                || !r.damper_integral_j.is_finite()
                || r.structural_integral_j < 0.0
                || r.damper_integral_j < 0.0
        })
    {
        return Err("loss recovery needs finite contact-free power integrals".into());
    }
    let norm_a = rows
        .iter()
        .map(|r| r.structural_integral_j.powi(2))
        .sum::<f64>()
        .sqrt();
    let norm_b = rows
        .iter()
        .map(|r| r.damper_integral_j.powi(2))
        .sum::<f64>()
        .sqrt();
    if norm_a <= 0.0 || norm_b <= 0.0 {
        return Err("unobservable loss column".into());
    }
    let q: Vec<_> = rows
        .iter()
        .map(|r| r.structural_integral_j / norm_a)
        .collect();
    let mut other: Vec<_> = rows.iter().map(|r| r.damper_integral_j / norm_b).collect();
    let mut projection = 0.0;
    for _ in 0..2 {
        let p = q.iter().zip(&other).map(|(a, b)| a * b).sum::<f64>();
        projection += p;
        for (v, q) in other.iter_mut().zip(&q) {
            *v -= p * q;
        }
    }
    let pivot = other.iter().map(|v| v * v).sum::<f64>().sqrt();
    if pivot < 1e-4 {
        return Err("indistinguishable structural and damper losses".into());
    }
    let z1 = q
        .iter()
        .zip(rows)
        .map(|(q, r)| q * r.energy_drop_j)
        .sum::<f64>();
    let z2 = other
        .iter()
        .zip(rows)
        .map(|(q, r)| q * r.energy_drop_j / pivot)
        .sum::<f64>();
    let scaled_b = z2 / pivot;
    Ok((
        (z1 - projection * scaled_b) / norm_a,
        scaled_b / norm_b,
        pivot,
    ))
}
fn residual(rows: &[&Row], a: f64, b: f64) -> f64 {
    let numerator = rows
        .iter()
        .map(|r| (r.energy_drop_j - a * r.structural_integral_j - b * r.damper_integral_j).powi(2))
        .sum::<f64>();
    let denominator = rows.iter().map(|r| r.energy_drop_j.powi(2)).sum::<f64>();
    (numerator / denominator).sqrt()
}

fn study() -> Result<Value, Box<dyn Error>> {
    let mut cases = Vec::new();
    for (structural, damper) in [(0.5, 1.5), (1.0, 0.5), (1.5, 1.0)] {
        for rate in [48000, 96000] {
            let take = simulate(rate, structural, damper)?;
            let fitting: Vec<_> = [0, 1, 3].map(|i| &take.rows[i]).into_iter().collect();
            let held_out: Vec<_> = [2, 4, 5].map(|i| &take.rows[i]).into_iter().collect();
            let fit_rows: Vec<_> = fitting
                .iter()
                .map(|r| Row {
                    start_seconds: r.start_seconds,
                    end_seconds: r.end_seconds,
                    energy_drop_j: r.energy_drop_j,
                    structural_integral_j: r.structural_integral_j,
                    damper_integral_j: r.damper_integral_j,
                    contact_free: r.contact_free,
                })
                .collect();
            let (a, b, pivot) = fit(&fit_rows)?;
            let omitted = take
                .rows
                .iter()
                .map(|r| r.structural_integral_j * r.energy_drop_j)
                .sum::<f64>()
                / take
                    .rows
                    .iter()
                    .map(|r| r.structural_integral_j.powi(2))
                    .sum::<f64>();
            let all: Vec<_> = take.rows.iter().collect();
            let validation = residual(&held_out, a, b);
            let omitted_error = residual(&all, omitted, 0.0);
            let passed = take.diagnostics["mechanical_checks_passed"] == true
                && take.rows.iter().all(|r| r.contact_free)
                && (a / structural - 1.0).abs() < 0.01
                && (b / damper - 1.0).abs() < 0.01
                && validation < 0.005
                && omitted_error > 0.01;
            cases.push(json!({"known_structural_scale":structural,"known_damper_scale":damper,"diagnostics":take.diagnostics,
                "rows":take.rows,"fit_row_indices":[0,1,3],"held_out_row_indices":[2,4,5],
                "estimated_structural_scale":a,"estimated_damper_scale":b,"normalized_qr_pivot":pivot,
                "fit_relative_energy_rmse":residual(&fitting,a,b),"held_out_relative_energy_rmse":validation,
                "omitted_damper_control":{"fitted_structural_scale":omitted,"relative_energy_rmse":omitted_error},
                "qualified":passed}));
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"controlled-mechanical-loss-recovery-v1",
        "all_cases_qualified":cases.iter().all(|c|c["qualified"]==true),"cases":cases,
        "protocol":"Fixed nine-coordinate default tine/tonebar geometry, elastic single-mass hammer, strike input 0.5; damper enabled at 0.14 s. Structural/damper scales (0.5,1.5), (1,0.5), (1.5,1), independently rendered at 48/96 kHz with four exact free-motion ticks per sample and 64 contact subdivisions. Energy endpoints reconstructed from q,v,M,K, without dissipated-energy ledger inputs. Endpoint trapezoidal integration of v^T C0 v and known on/off v^T D v. Six fixed intervals bounded by 0.02,0.06,0.10,0.14,0.18,0.22,0.26 s; require contact absent throughout. Two normalized columns, reorthogonalized QR with pivot >=1e-4. Fit rows 0,1,3, validate 2,4,5 without refit. Each scale error <1%, held-out relative energy RMSE <0.005, mechanical ledger <1e-8 and positive energy step <1e-10. Omitted-damper best scalar fit over all rows must retain relative residual >0.01. Thresholds and row choices declared before first run.",
        "scope":"Known full mechanical state and correct operator shapes, not audio-only identification. Recovers only two global multipliers, not nine independent losses or modal decay times. The hammer is the existing elastic assembly reference, not the memory-material hammer. Integration and quadrature refinement are separate from real-instrument validation. No pickup conversion, reference bank fitting, production preset changes, WAV or host launch."}),
    )
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = study()?;
    crate::analysis::write_report(output, &report)?;
    if report["all_cases_qualified"] != true {
        return Err("mechanical loss study retained failed cases".into());
    }
    println!("Mechanical loss recovery: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(a: f64, b: f64, y: f64) -> Row {
        Row {
            start_seconds: 0.0,
            end_seconds: 0.04,
            structural_integral_j: a,
            damper_integral_j: b,
            energy_drop_j: y,
            contact_free: true,
        }
    }
    #[test]
    fn independent_power_rows_recover_two_scales_and_predict_unused_energy() {
        let rows = [row(2.0, 0.0, 1.4), row(4.0, 0.0, 2.8), row(1.0, 3.0, 4.6)];
        let (a, b, pivot) = fit(&rows).unwrap();
        assert!((a - 0.7).abs() < 1e-12);
        assert!((b - 1.3).abs() < 1e-12);
        assert!(pivot > 0.9);
        let validation = row(2.0, 1.0, 2.7);
        assert!(residual(&[&validation], a, b) < 1e-12);
    }
    #[test]
    fn rank_deficiency_missing_excitation_and_contact_withhold_loss_recovery() {
        assert!(fit(&[row(0.0, 0.0, 0.0), row(0.0, 1.0, 1.0)]).is_err());
        assert!(fit(&[row(1.0, 0.0, 1.0), row(2.0, 0.0, 2.0)]).is_err());
        assert!(fit(&[row(1.0, 2.0, 3.0), row(2.0, 4.0, 6.0)]).is_err());
        let mut rows = [row(1.0, 0.0, 1.0), row(1.0, 1.0, 2.0)];
        rows[0].contact_free = false;
        assert!(fit(&rows).is_err());
        rows[0].contact_free = true;
        rows[0].energy_drop_j = f64::NAN;
        assert!(fit(&rows).is_err());
    }
}
