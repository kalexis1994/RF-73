//! Two prescribed modes through the existing pickup; no mechanical parameter fitting.
use rf_73_analysis::coherent_coefficients;
use rf_73_dsp::MagneticPickup;
use serde::Serialize;
use serde_json::{Value, json};
use std::{error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Controlled pickup mixing:
  pickup-mixing --output REPORT.json
Fixed coherent two-tone motion near G3/1425 Hz; production and point-pole laws.
Combined nonlinear, sum of independent nonlinear, and linearized controls.
Full common ideal-band complex FFT refinement at 1/2/4/8/16/32/64x.
Eight fixed probes including centered geometry and numerical-only stress.
No WAV, device launch, production FIR, mechanical calibration or geometry fit.
";
const RATE: f64 = 48000.0;
const PERIOD: usize = 16384;
const FIRST: usize = 67;
const SECOND: usize = 487;
const LAST: usize = (0.42 * PERIOD as f64) as usize;
const DENSITIES: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];
const FLOOR: f64 = 1e-8;
const LINES: [(&str, usize); 10] = [
    ("f1", FIRST),
    ("f2", SECOND),
    ("2*f1", 2 * FIRST),
    ("2*f2", 2 * SECOND),
    ("f2-f1", SECOND - FIRST),
    ("f2+f1", SECOND + FIRST),
    ("f2-2*f1", SECOND - 2 * FIRST),
    ("f2+2*f1", SECOND + 2 * FIRST),
    ("f2-3*f1", SECOND - 3 * FIRST),
    ("f2+3*f1", SECOND + 3 * FIRST),
];
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Law {
    Production,
    PointPole,
}
impl Law {
    pub(super) fn voltage(self, p: MagneticPickup, x: f64, v: f64) -> f64 {
        match self {
            Self::Production => p.voltage(x, v),
            Self::PointPole => p.research_point_pole_voltage(x, v),
        }
    }
    // Derivative at equilibrium of the displacement-dependent velocity sensitivity.
    pub(super) fn quadratic(self, gap: f64, offset: f64) -> f64 {
        let z = offset / gap;
        match self {
            Self::Production => 0.015 / gap.powi(2) * (1.0 - 2.0 * z * z) / (1.0 + z * z).powf(2.5),
            Self::PointPole => 0.045 / gap.powi(2) * (1.0 - 4.0 * z * z) / (1.0 + z * z).powf(3.5),
        }
    }
}
struct Spectra {
    combined: Vec<[f64; 2]>,
    independent: Vec<[f64; 2]>,
    linear: Vec<[f64; 2]>,
}
fn sample(
    law: Law,
    gap: f64,
    offset: f64,
    amplitude: f64,
    secondary: f64,
    density: usize,
) -> Result<Spectra, Box<dyn Error>> {
    let p = MagneticPickup::new(gap, offset)?;
    let n = PERIOD * density;
    let (mut combined, mut independent, mut linear) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let slope = law.voltage(p, 0.0, 1.0);
    let (w1, w2) = (
        TAU * FIRST as f64 * RATE / PERIOD as f64,
        TAU * SECOND as f64 * RATE / PERIOD as f64,
    );
    for i in 0..n {
        let phase = TAU * i as f64 / n as f64;
        let (s1, c1) = (phase * FIRST as f64).sin_cos();
        let (s2, c2) = (phase * SECOND as f64).sin_cos();
        let (x1, v1) = (amplitude * c1, -amplitude * w1 * s1);
        let (x2, v2) = (secondary * c2, -secondary * w2 * s2);
        combined.push(law.voltage(p, x1 + x2, v1 + v2));
        independent.push(law.voltage(p, x1, v1) + law.voltage(p, x2, v2));
        linear.push(slope * (v1 + v2));
    }
    Ok(Spectra {
        combined: coherent_coefficients(combined, LAST)?,
        independent: coherent_coefficients(independent, LAST)?,
        linear: coherent_coefficients(linear, LAST)?,
    })
}
fn amplitude(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn power(c: &[[f64; 2]]) -> f64 {
    c[0][0].powi(2)
        + 0.5
            * c[1..]
                .iter()
                .map(|v| v[0] * v[0] + v[1] * v[1])
                .sum::<f64>()
}
fn difference(a: &[[f64; 2]], b: &[[f64; 2]]) -> Vec<[f64; 2]> {
    a.iter()
        .zip(b)
        .map(|(a, b)| [a[0] - b[0], a[1] - b[1]])
        .collect()
}
fn observe(law: Law, gap: f64, offset: f64, a: f64) -> Result<Value, Box<dyn Error>> {
    let b = 0.2 * a;
    let spectra = DENSITIES
        .iter()
        .map(|&d| sample(law, gap, offset, a, b, d))
        .collect::<Result<Vec<_>, _>>()?;
    let reference = spectra.last().unwrap();
    let energy = power(&reference.combined);
    if !energy.is_finite() || energy <= 0.0 {
        return Err("invalid nonzero pickup reference power".into());
    }
    let interaction = difference(&reference.combined, &reference.independent);
    let residuals:Vec<_>=spectra.iter().zip(DENSITIES).map(|(s,d)| {
        let full=(power(&difference(&s.combined,&reference.combined))/energy).sqrt();
        let independent=(power(&difference(&s.independent,&reference.independent))/energy).sqrt();
        let linear=(power(&difference(&s.linear,&reference.linear))/energy).sqrt();
        json!({"oversampling":d,"combined_band_nrmse":full,"independent_band_nrmse":independent,"linear_band_nrmse":linear,
            "combined_error_db_above_floor":(full>=FLOOR).then(||20.0*full.log10())})
    }).collect();
    let refined = residuals[5]["combined_band_nrmse"].as_f64().unwrap() < FLOOR
        && residuals[5]["independent_band_nrmse"].as_f64().unwrap() < FLOOR
        && residuals[5]["linear_band_nrmse"].as_f64().unwrap() < FLOOR;
    let parent = amplitude(reference.combined[FIRST]);
    let lines:Vec<_>=LINES.iter().map(|&(label,k)| {
        let f=k as f64*RATE/PERIOD as f64;let mag=amplitude(reference.combined[k]);
        let leading=if k==SECOND-FIRST || k==SECOND+FIRST {Some(law.quadratic(gap,offset)*a*b*TAU*f/2.0)} else {None};
        json!({"label":label,"frequency_hz":f,"combined_coefficient":reference.combined[k],"combined_amplitude":mag,
            "independent_amplitude":amplitude(reference.independent[k]),"linear_amplitude":amplitude(reference.linear[k]),
            "interaction_coefficient":interaction[k],"interaction_amplitude":amplitude(interaction[k]),
            "relative_to_combined_f1_db":(parent>1e-10*energy.sqrt() && mag>1e-10*energy.sqrt()).then(||20.0*(mag/parent).log10()),
            "leading_quadratic_imaginary_coefficient":leading})
    }).collect();
    Ok(
        json!({"law":law,"gap_mm":1000.0*gap,"offset_mm":1000.0*offset,"primary_amplitude_mm":1000.0*a,"secondary_amplitude_mm":1000.0*b,
        "combined_ideal_band_rms":energy.sqrt(),"independent_ideal_band_rms":power(&reference.independent).sqrt(),
        "linear_ideal_band_rms":power(&reference.linear).sqrt(),"interaction_ideal_band_rms":power(&interaction).sqrt(),
        "combined_dc":reference.combined[0][0],"reference_refinement_passed":refined,"lines":lines,"sampling_residuals":residuals}),
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
        for (kind, gap, offset, a) in [
            ("small_motion", 0.0015, 0.0005, 0.000005),
            ("larger_motion", 0.0015, 0.0005, 0.00025),
            ("centered_control", 0.0015, 0.0, 0.00025),
            ("numerical_stress_only", 0.0005, 0.00025, 0.003),
        ] {
            let mut row = observe(law, gap, offset, a)?;
            row["probe"] = json!(kind);
            cases.push(row);
            println!("Pickup mixing: {kind}");
        }
    }
    let qualified = cases
        .iter()
        .all(|c| c["reference_refinement_passed"] == true);
    super::analysis::write_report(
        output,
        &json!({"schema_version":1,"experiment":"coherent-two-mode-pickup-mixing-v1","sample_rate_hz":RATE,
        "period_frames":PERIOD,"duration_seconds":PERIOD as f64/RATE,"frequency_grid_hz":RATE/PERIOD as f64,
        "parent_frequencies_hz":[FIRST as f64*RATE/PERIOD as f64,SECOND as f64*RATE/PERIOD as f64],"ideal_band_limit_hz":LAST as f64*RATE/PERIOD as f64,
        "all_reference_refinements_passed":qualified,"numerical_reporting_floor_db":-160,"cases":cases,
        "protocol":"x=a*cos(w1*t)+b*cos(w2*t), exact derivative, zero phases. Combined nonlinear versus sum of two independent nonlinear voltages and equilibrium-linearized control; identical trajectory and raw gain. Coherent unwindowed complex FFT over the complete period. DC plus every bin through floor(0.42*period_frames) is retained. Parseval residuals of all paths normalized by combined nonlinear reference band power. Finite 64x reference, 32x agreement required below 1e-8. No output normalization or fitting.",
        "scope":"Mechanism experiment, not measured modal identity. Parent frequencies are coherent probes near G3/1425 Hz, not fitted source resonances. Single displacement coordinate and existing flux proxies; no mechanics, production FIR, transients, circuit, second polarization or real-time timing. Ideal-band refinement diagnoses internally sampled nonlinear aliasing, not complete instrument anti-alias performance. A generated sideband does not prove the recorded family's origin; numerical stress is not an inferred instrument trajectory."}),
    )?;
    if !qualified {
        return Err("pickup mixing retained a failed reference refinement".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weak_mixing_matches_independent_quadratic_flux_expansion() {
        for law in [Law::Production, Law::PointPole] {
            let (a, b) = (1e-7, 2e-8);
            let s = sample(law, 0.0015, 0.0005, a, b, 1).unwrap();
            for k in [SECOND - FIRST, SECOND + FIRST] {
                let expected =
                    law.quadratic(0.0015, 0.0005) * a * b * TAU * (k as f64 * RATE / PERIOD as f64)
                        / 2.0;
                assert!((s.combined[k][1] / expected - 1.0).abs() < 1e-6);
                assert!(s.combined[k][0].abs() < expected.abs() * 1e-6);
                assert!(amplitude(s.independent[k]) < expected.abs() * 1e-6);
                assert!(amplitude(s.linear[k]) < expected.abs() * 1e-6);
            }
        }
    }
    #[test]
    fn removing_one_parent_removes_cross_interaction_and_centered_pickup_has_zero_linear_gain() {
        let s = sample(Law::Production, 0.0015, 0.0005, 0.00025, 0.0, 1).unwrap();
        assert_eq!(s.combined, s.independent);
        let centered = sample(Law::Production, 0.0015, 0.0, 0.00025, 0.00005, 1).unwrap();
        assert_eq!(power(&centered.linear), 0.0);
        assert!(amplitude(centered.combined[SECOND + FIRST]) > 0.001);
    }
    #[test]
    fn stress_sampling_exposes_aliasing_and_converges_in_the_full_common_band() {
        let r = observe(Law::PointPole, 0.0005, 0.00025, 0.003).unwrap();
        let errors = r["sampling_residuals"].as_array().unwrap();
        assert!(errors[0]["combined_band_nrmse"].as_f64().unwrap() > 1e-5);
        assert!(
            errors[3]["combined_band_nrmse"].as_f64().unwrap()
                < errors[0]["combined_band_nrmse"].as_f64().unwrap()
        );
        assert_eq!(r["reference_refinement_passed"], true);
    }
}
