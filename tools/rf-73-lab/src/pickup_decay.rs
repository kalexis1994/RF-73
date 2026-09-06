//! Phase-resolved envelopes of prescribed damped modes, not a recording estimator.
use super::pickup_mixing::Law;
use rf_73_dsp::MagneticPickup;
use serde_json::{Value, json};
use std::{error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Controlled pickup decay:
  pickup-decay --output REPORT.json
Two prescribed damped modes; phase-torus projection of sum/difference envelopes.
Both pickup laws, exact envelope derivative, weak/large/equal/zero-loss controls.
Not a recording estimator, temporal FFT or instrument antialias qualification.
";
const F1: f64 = 196.2890625;
const F2: f64 = 1426.7578125;
const GAP: f64 = 0.0015;
const OFFSET: f64 = 0.0005;
const TIMES: [f64; 6] = [0.0, 0.05, 0.1, 0.2, 0.35, 0.5];
const PHASES: [f64; 2] = [0.37, -0.61];
const SIGNS: [f64; 2] = [-1.0, 1.0];
type Complex = [f64; 2];

#[derive(Clone, Copy)]
struct Motion {
    amplitude: f64,
    decay: f64,
    omega: f64,
    phase: f64,
}
impl Motion {
    fn state(self, age: f64, angle: f64) -> (f64, f64, f64) {
        let a = self.amplitude * (-self.decay * age).exp();
        let (s, c) = (angle + self.phase).sin_cos();
        let x = a * c;
        let carrier_velocity = -a * self.omega * s;
        (x, carrier_velocity - self.decay * x, carrier_velocity)
    }
}
fn magnitude(c: Complex) -> f64 {
    c[0].hypot(c[1])
}
fn relative_error(a: Complex, b: Complex) -> f64 {
    magnitude([a[0] - b[0], a[1] - b[1]]) / magnitude(b)
}
fn phase_error(a: Complex, b: Complex) -> f64 {
    // Argument of a * conjugate(b), wrapped to [-pi, pi].
    (a[1] * b[0] - a[0] * b[1]).atan2(a[0] * b[0] + a[1] * b[1])
}
fn predicted(law: Law, modes: [Motion; 2], age: f64, sign: f64) -> Complex {
    let [a, b] = modes;
    let decay = a.decay + b.decay;
    let omega = b.omega + sign * a.omega;
    let scale = law.quadratic(GAP, OFFSET) * a.amplitude * b.amplitude * (-decay * age).exp() / 2.0;
    let (s, c) = (b.phase + sign * a.phase).sin_cos();
    [
        scale * (-decay * c - omega * s),
        scale * (omega * c - decay * s),
    ]
}

// [sideband][combined, independent, linear, interaction, omitted envelope derivative].
// The torus varies the two carrier phases independently at a fixed envelope age.
// Coefficients are demodulated by the carrier evolution, retaining initial phases.
fn project(law: Law, modes: [Motion; 2], age: f64, n: usize) -> [[Complex; 5]; 2] {
    let pickup = MagneticPickup::new(GAP, OFFSET).unwrap();
    let slope = law.voltage(pickup, 0.0, 1.0);
    let states = modes.map(|mode| {
        (0..n)
            .map(|i| mode.state(age, TAU * i as f64 / n as f64))
            .collect::<Vec<_>>()
    });
    let rotations: Vec<_> = (0..n)
        .map(|i| (TAU * i as f64 / n as f64).sin_cos())
        .collect();
    let mut result = [[[0.0; 2]; 5]; 2];
    for (i, &(x1, v1, carrier1)) in states[0].iter().enumerate() {
        for (j, &(x2, v2, carrier2)) in states[1].iter().enumerate() {
            let combined = law.voltage(pickup, x1 + x2, v1 + v2);
            let independent = law.voltage(pickup, x1, v1) + law.voltage(pickup, x2, v2);
            let values = [
                combined,
                independent,
                slope * (v1 + v2),
                combined - independent,
                law.voltage(pickup, x1 + x2, carrier1 + carrier2),
            ];
            let (si, ci) = rotations[i];
            let (sj, cj) = rotations[j];
            for (k, sign) in SIGNS.iter().enumerate() {
                let (s, c) = (sj * ci + sign * cj * si, cj * ci - sign * sj * si);
                for (out, value) in result[k].iter_mut().zip(values) {
                    out[0] += value * c;
                    out[1] -= value * s;
                }
            }
        }
    }
    for sideband in &mut result {
        for value in sideband {
            for part in value {
                *part *= 2.0 / (n * n) as f64;
            }
        }
    }
    result
}

// Unweighted least squares in log amplitude; no prescribed rate in this fit.
fn fit_decay(times: &[f64], coefficients: &[Complex]) -> Result<(f64, f64), Box<dyn Error>> {
    if times.len() != coefficients.len() || times.len() < 3 {
        return Err("decay fit needs at least three paired observations".into());
    }
    let logs: Vec<_> = coefficients.iter().map(|&c| magnitude(c).ln()).collect();
    if times.iter().chain(&logs).any(|v| !v.is_finite()) {
        return Err("decay fit needs finite times and positive amplitudes".into());
    }
    let count = times.len() as f64;
    let mean_t = times.iter().sum::<f64>() / count;
    let mean_y = logs.iter().sum::<f64>() / count;
    let variance = times.iter().map(|t| (t - mean_t).powi(2)).sum::<f64>();
    if variance <= 0.0 {
        return Err("decay fit needs distinct times".into());
    }
    let slope = times
        .iter()
        .zip(&logs)
        .map(|(t, y)| (t - mean_t) * (y - mean_y))
        .sum::<f64>()
        / variance;
    let residual = (times
        .iter()
        .zip(&logs)
        .map(|(t, y)| (y - mean_y - slope * (t - mean_t)).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    Ok((-slope, residual))
}

fn observe(
    law: Law,
    probe: &str,
    amplitude: f64,
    rates: [f64; 2],
) -> Result<Value, Box<dyn Error>> {
    let modes = [
        Motion {
            amplitude,
            decay: rates[0],
            omega: TAU * F1,
            phase: PHASES[0],
        },
        Motion {
            amplitude: amplitude * 0.2,
            decay: rates[1],
            omega: TAU * F2,
            phase: PHASES[1],
        },
    ];
    let mut rows = Vec::new();
    let mut coefficients: [Vec<Complex>; 2] = [Vec::new(), Vec::new()];
    let mut max_refinement: f64 = 0.0;
    let mut max_control: f64 = 0.0;
    let mut max_weak_error: f64 = 0.0;
    for age in TIMES {
        let coarse = project(law, modes, age, 32);
        let fine = project(law, modes, age, 64);
        let reference = project(law, modes, age, 128);
        let mut lines = Vec::new();
        for (k, sign) in SIGNS.iter().enumerate() {
            let expected = predicted(law, modes, age, *sign);
            let measured = reference[k][3];
            coefficients[k].push(measured);
            let refinement = relative_error(fine[k][3], measured);
            let control =
                magnitude(reference[k][1]).max(magnitude(reference[k][2])) / magnitude(measured);
            let weak_error = relative_error(measured, expected);
            if ![refinement, control, weak_error]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err("unresolved or nonfinite sideband coefficient".into());
            }
            max_refinement = max_refinement.max(refinement);
            max_control = max_control.max(control);
            max_weak_error = max_weak_error.max(weak_error);
            lines.push(json!({
                "label": if k == 0 {"f2-f1"} else {"f2+f1"},
                "frequency_hz": F2 + sign * F1,
                "combined_coefficient": reference[k][0],
                "independent_coefficient": reference[k][1],
                "linear_coefficient": reference[k][2],
                "interaction_coefficient": measured,
                "omitted_envelope_derivative_coefficient": reference[k][4],
                "quadratic_prediction": expected,
                "quadratic_relative_error": weak_error,
                "phase_error_vs_quadratic_radians": phase_error(measured, expected),
                "omitted_derivative_phase_error_radians": phase_error(reference[k][4], measured),
                "phase_grid_32_relative_error": relative_error(coarse[k][3], measured),
                "phase_grid_64_relative_error": refinement,
                "control_relative_amplitude": control
            }));
        }
        rows.push(json!({"age_seconds": age, "lines": lines}));
    }
    let expected_rate = rates.iter().sum::<f64>();
    let mut fits = Vec::new();
    let mut max_rate_error: f64 = 0.0;
    for (k, c) in coefficients.iter().enumerate() {
        let (rate, residual) = fit_decay(&TIMES, c)?;
        max_rate_error = max_rate_error.max((rate - expected_rate).abs());
        fits.push(json!({"label": if k == 0 {"f2-f1"} else {"f2+f1"},
            "fitted_amplitude_decay_per_second": rate,
            "quadratic_expected_decay_per_second": expected_rate,
            "log_amplitude_fit_rms": residual}));
    }
    let weak = amplitude <= 0.000005;
    let qualified = max_refinement < 1e-8
        && max_control < 1e-8
        && (!weak || (max_weak_error < 1e-3 && max_rate_error < 1e-3));
    Ok(
        json!({"law": law, "probe": probe, "primary_amplitude_mm": amplitude * 1000.0,
        "secondary_amplitude_mm": amplitude * 200.0, "modal_amplitude_decay_per_second": rates,
        "qualified": qualified, "weak_prediction_gate_applied": weak,
        "max_refinement_relative_error": max_refinement, "max_control_relative_amplitude": max_control,
        "max_quadratic_relative_error": max_weak_error, "observations": rows, "decay_fits": fits}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let mut cases = Vec::new();
    for law in [Law::Production, Law::PointPole] {
        for (probe, amplitude, rates) in [
            ("weak_unequal_decay", 0.000005, [0.8, 5.0]),
            ("larger_unequal_decay", 0.00025, [0.8, 5.0]),
            ("weak_equal_decay", 0.000005, [2.0, 2.0]),
            ("weak_zero_decay", 0.000005, [0.0, 0.0]),
        ] {
            cases.push(observe(law, probe, amplitude, rates)?);
        }
    }
    let qualified = cases.iter().all(|c| c["qualified"] == true);
    super::analysis::write_report(
        output,
        &json!({
            "schema_version": 1, "experiment": "phase-resolved-pickup-decay-v1",
            "parent_frequencies_hz": [F1, F2], "initial_phases_radians": PHASES,
            "gap_mm": GAP * 1000.0, "offset_mm": OFFSET * 1000.0,
            "phase_grid_sizes": [32, 64, 128], "all_cases_qualified": qualified,
            "cases": cases,
            "protocol": "Independent phase-torus quadrature at each fixed envelope age; exact damped-motion velocity. Peak complex coefficients use exp(-i*(theta2 +/- theta1)) and 2/N^2 scaling, retaining initial phases but demodulating carrier evolution. Free log-amplitude regression estimates decay from six observations. 64/128 grid relative agreement and null controls below 1e-8; weak quadratic complex and fitted rate absolute errors below 1e-3. Larger-motion approximation errors are observations, not gates.",
            "scope": "Prescribed motion and existing one-coordinate flux proxies. Not a temporal FFT, measured decay estimator, global aliasing bound, finite-pole model or instrument antialias qualification. Independent modal amplitude decay rates are probes, not calibrated damping or measured energy decay. No geometry fitting, audio or plugin change."
        }),
    )?;
    if !qualified {
        return Err("pickup decay retained a failed qualification".into());
    }
    println!("Pickup decay: eight cases qualified; {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn modes() -> [Motion; 2] {
        [
            Motion {
                amplitude: 1e-7,
                decay: 35.0,
                omega: TAU * F1,
                phase: PHASES[0],
            },
            Motion {
                amplitude: 2e-8,
                decay: 85.0,
                omega: TAU * F2,
                phase: PHASES[1],
            },
        ]
    }
    #[test]
    fn damped_velocity_matches_displacement_derivative() {
        for mode in modes() {
            for age in [0.0, 0.002, 0.01] {
                let h = 1e-8;
                let at = |t| mode.state(t, mode.omega * t).0;
                let numerical = (at(age + h) - at(age - h)) / (2.0 * h);
                let (_, actual, _) = mode.state(age, mode.omega * age);
                assert!((numerical - actual).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn damped_mixing_matches_complex_prediction_and_detects_missing_derivative() {
        for law in [Law::Production, Law::PointPole] {
            for age in [0.0, 0.003, 0.01] {
                let s = project(law, modes(), age, 64);
                for (k, sign) in SIGNS.iter().enumerate() {
                    let expected = predicted(law, modes(), age, *sign);
                    assert!(relative_error(s[k][3], expected) < 1e-6);
                    assert!(magnitude(s[k][1]) / magnitude(expected) < 1e-8);
                    assert!(magnitude(s[k][2]) / magnitude(expected) < 1e-8);
                    assert!(phase_error(s[k][4], s[k][3]).abs() > 0.01);
                }
            }
        }
    }
    #[test]
    fn free_decay_fit_recovers_rates_and_rejects_undefined_observations() {
        for rate in [0.0, 4.0, 5.8] {
            let c: Vec<_> = TIMES
                .iter()
                .map(|t| [0.4 * (-rate * t).exp(), -0.3 * (-rate * t).exp()])
                .collect();
            let (fit, residual) = fit_decay(&TIMES, &c).unwrap();
            assert!((fit - rate).abs() < 1e-12);
            assert!(residual < 1e-12);
        }
        assert!(fit_decay(&TIMES, &[[0.0; 2]; 6]).is_err());
        assert!(fit_decay(&[0.0; 3], &[[1.0, 0.0]; 3]).is_err());
        assert!(fit_decay(&TIMES, &[[1.0, 0.0]; 3]).is_err());
        assert!(fit_decay(&[0.0, f64::NAN, 1.0], &[[1.0, 0.0]; 3]).is_err());
    }
    #[test]
    fn phase_grid_refines_and_removing_a_parent_removes_interaction() {
        let mut m = modes();
        m[0].amplitude = 0.00025;
        m[1].amplitude = 0.00005;
        let coarse = project(Law::PointPole, m, 0.0, 32);
        let fine = project(Law::PointPole, m, 0.0, 64);
        for k in 0..2 {
            assert!(relative_error(coarse[k][3], fine[k][3]) < 1e-8);
        }
        m[1].amplitude = 0.0;
        let single = project(Law::Production, m, 0.0, 32);
        for line in single {
            assert_eq!(line[3], [0.0, 0.0]);
        }
    }
}
