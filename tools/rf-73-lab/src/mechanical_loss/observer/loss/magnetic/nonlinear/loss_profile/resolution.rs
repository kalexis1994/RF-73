//! Local profiled prediction distances around pinned noisy loss estimates.
use super::*;
use serde::Deserialize;
use std::{
    fs::File,
    io::{Read, Write},
    process::{Command, Stdio},
};

pub const HELP: &str = "Magnetic loss resolution study:
  magnetic-loss-resolution --input NOISE_RECEIPT.json --output REPORT.json
Pinned estimates, continuous-state refits and oracle noise-scaled prediction distances.
";
pub(super) const SOURCE_BLOB: &str = "5e32ac1255599fd8aae280eb6656d14921c0f174";
pub(super) const SOURCE_SHA256: &str =
    "5499232a9abd733a0e1a09169b14d8d17de6da855202923669e92cac8a54d1a1";

// No reference losses, reference motion, held-out scores or prior states are
// deserialized into the local diagnostic. Fixture truth only regenerates input.
#[derive(Deserialize)]
struct Source {
    schema_version: u32,
    experiment: String,
    cases: Vec<SourceCase>,
}
#[derive(Deserialize)]
struct SourceCase {
    sample_rate: u32,
    observations: Vec<SourceRow>,
}
#[derive(Deserialize)]
struct SourceRow {
    snr_db: Option<f64>,
    seed: u64,
    fit: SourceFit,
}
#[derive(Deserialize)]
struct SourceFit {
    estimated_scales: [f64; 2],
    training_relative_rmse: f64,
}

pub(super) fn pinned_bytes(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(8_000_001).read_to_end(&mut bytes)?;
    if bytes.len() > 8_000_000 {
        return Err("loss receipt exceeds byte limit".into());
    }
    let mut child = Command::new("git")
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing receipt hash pipe")?
        .write_all(&bytes)?;
    let result = child.wait_with_output()?;
    if !result.status.success() || String::from_utf8(result.stdout)?.trim() != SOURCE_BLOB {
        return Err("loss receipt does not match pinned evidence".into());
    }
    Ok(bytes)
}

fn source(path: &Path) -> Result<Source, Box<dyn Error>> {
    let parsed: Source = serde_json::from_slice(&pinned_bytes(path)?)?;
    if parsed.schema_version != 1
        || parsed.experiment != "nonlinear-magnetic-loss-noise-v1"
        || parsed.cases.len() != 6
    {
        return Err("unsupported loss receipt schema".into());
    }
    Ok(parsed)
}

struct Sensitivity {
    singular_values: [f64; 2],
    weak_direction: [f64; 2],
}
fn sensitivity(columns: &[Vec<f64>; 2]) -> Result<Sensitivity, Box<dyn Error>> {
    if columns[0].is_empty()
        || columns[0].len() != columns[1].len()
        || columns.iter().flatten().any(|x| !x.is_finite())
    {
        return Err("invalid sensitivity columns".into());
    }
    let a = dot(&columns[0], &columns[0]).sqrt();
    let c = dot(&columns[1], &columns[1]).sqrt();
    if a <= 0.0 || c <= 0.0 {
        return Err("unexcited loss sensitivity".into());
    }
    let q: Vec<_> = columns[0].iter().map(|x| x / a).collect();
    let mut orthogonal = columns[1].clone();
    for _ in 0..2 {
        let projection = dot(&q, &orthogonal);
        for (x, basis) in orthogonal.iter_mut().zip(&q) {
            *x -= projection * basis;
        }
    }
    let b = dot(&orthogonal, &orthogonal).sqrt();
    let cross = dot(&columns[0], &columns[1]);
    let largest = (0.5 * (a * a + c * c + (a * a - c * c).hypot(2.0 * cross))).sqrt();
    // |det R| / largest avoids subtracting almost equal Gram determinants.
    let smallest = a * b / largest;
    let theta = 0.5 * (2.0 * cross).atan2(a * a - c * c);
    let mut weak = [-theta.sin(), theta.cos()];
    if weak[0] < 0.0 {
        weak = weak.map(|x| -x);
    }
    Ok(Sensitivity {
        singular_values: [largest, smallest],
        weak_direction: weak,
    })
}
fn residual_in_voltage(profile: &Profile<'_>, c: &Candidate) -> Vec<f64> {
    let mut result = Vec::with_capacity(c.residual.len());
    let mut offset = 0;
    for y in &profile.training {
        let scale = 2.0_f64.sqrt() * dot(y, y).sqrt();
        result.extend(
            c.residual[offset..offset + y.len()]
                .iter()
                .map(|r| r * scale),
        );
        offset += y.len();
    }
    result
}
fn alternative(
    profile: &mut Profile<'_>,
    center: &Candidate,
    offset: [f64; 2],
    sigma: f64,
    label: &str,
) -> Result<(Candidate, Value), Box<dyn Error>> {
    let scales = core::array::from_fn(|j| center.scales[j] * offset[j].exp());
    let candidate = profile.candidate(scales)?;
    let a = residual_in_voltage(profile, &candidate);
    let b = residual_in_voltage(profile, center);
    let distance = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt();
    let noise_distance = if sigma > 0.0 {
        Some(distance / sigma)
    } else {
        None
    };
    let report = json!({"label":label,"log_scale_offset":offset,"scales":scales,"evaluation_index":candidate.evaluation_index,
        "training_objective":candidate.objective,"objective_change":candidate.objective-center.objective,
        "prediction_change_voltage_l2":distance,"oracle_noise_scaled_prediction_distance":noise_distance,
        "within_one_noise_unit":noise_distance.map(|d|d<1.0)});
    Ok((candidate, report))
}

fn diagnose(
    profile: &mut Profile<'_>,
    row: &SourceRow,
    sigma: f64,
) -> Result<Value, Box<dyn Error>> {
    let center = profile.candidate(row.fit.estimated_scales)?;
    let h = 1.01_f64.ln();
    let large = 1.05_f64.ln();
    let mut alternatives = Vec::new();
    let mut raw: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let mut normalized: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for j in 0..2 {
        let mut offset = [0.0; 2];
        offset[j] = -h;
        let (minus, m) = alternative(
            profile,
            &center,
            offset,
            sigma,
            &format!("axis_{j}_minus_1pct"),
        )?;
        offset[j] = h;
        let (plus, p) = alternative(
            profile,
            &center,
            offset,
            sigma,
            &format!("axis_{j}_plus_1pct"),
        )?;
        let v_minus = residual_in_voltage(profile, &minus);
        let v_plus = residual_in_voltage(profile, &plus);
        raw[j] = v_plus
            .iter()
            .zip(&v_minus)
            .map(|(a, b)| (a - b) / (2.0 * h))
            .collect();
        normalized[j] = plus
            .residual
            .iter()
            .zip(&minus.residual)
            .map(|(a, b)| (a - b) / (2.0 * h))
            .collect();
        alternatives.extend([m, p]);
        for sign in [-1.0, 1.0] {
            offset[j] = sign * large;
            alternatives.push(
                alternative(
                    profile,
                    &center,
                    offset,
                    sigma,
                    &format!(
                        "axis_{j}_{}_5pct",
                        if sign < 0.0 { "minus" } else { "plus" }
                    ),
                )?
                .1,
            );
        }
    }
    let weighted = sensitivity(&normalized)?;
    let voltage = sensitivity(&raw)?;
    for sign in [-1.0, 1.0] {
        let offset = voltage.weak_direction.map(|x| x * sign * large);
        alternatives.push(
            alternative(
                profile,
                &center,
                offset,
                sigma,
                if sign < 0.0 {
                    "weak_minus_5pct"
                } else {
                    "weak_plus_5pct"
                },
            )?
            .1,
        );
    }
    let replay = (center.objective.sqrt() - row.fit.training_relative_rmse).abs();
    let noise_singular = if sigma > 0.0 {
        Some(voltage.singular_values.map(|s| s / sigma))
    } else {
        None
    };
    let radius = if sigma > 0.0 && voltage.singular_values[1] > 0.0 {
        Some(sigma / voltage.singular_values[1])
    } else {
        None
    };
    Ok(
        json!({"center_scales":center.scales,"center_evaluation_index":center.evaluation_index,"center_training_relative_rmse":center.objective.sqrt(),
        "source_training_rmse_absolute_difference":replay,"center_replayed":replay<1e-8,
        "relative_window_profile_singular_values":weighted.singular_values,"raw_voltage_profile_singular_values":voltage.singular_values,
        "weak_log_scale_direction":voltage.weak_direction,"minimum_to_maximum_raw_singular_ratio":voltage.singular_values[1]/voltage.singular_values[0],
        "oracle_noise_scaled_singular_values":noise_singular,"linearized_log_radius_for_one_noise_unit":radius,
        "noise_resolution_status":if sigma>0.0{"descriptive_oracle_distance"}else{"withheld_no_noise_scale"},"alternatives":alternatives}),
    )
}

fn study(input: &Source) -> Result<Value, Box<dyn Error>> {
    let p = prepare(perturbations()[0])?;
    let sensor = sensors()?
        .into_iter()
        .find(|s| s.law == "production" && s.geometry == "baseline")
        .ok_or("missing production sensor")?;
    let expected_noise = [
        (None, 0),
        (Some(40.0), 17),
        (Some(40.0), 71),
        (Some(20.0), 17),
        (Some(20.0), 71),
    ];
    let mut cases = Vec::new();
    for (case_index, source_case) in input.cases.iter().enumerate() {
        let (alpha, beta) = [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)][case_index / 2];
        let rate = if case_index % 2 == 0 { 48000 } else { 96000 };
        if source_case.sample_rate != rate || source_case.observations.len() != 5 {
            return Err("unexpected source case layout".into());
        }
        let mut clean: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
        let mut contact_free = true;
        let take = simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
            for (i, (start, end)) in [(0.02, 0.10), (0.14, 0.22)].into_iter().enumerate() {
                if tick >= (start * tick_rate as f64).round() as usize
                    && tick < (end * tick_rate as f64).round() as usize
                {
                    contact_free &= !before.contact_active && !after.contact_active;
                    if tick % (tick_rate / rate as usize) == 0 {
                        clean[i].push(
                            sensor
                                .voltage(before.pickup_displacement_m, before.pickup_velocity_m_s),
                        );
                    }
                }
            }
        })?;
        if !contact_free
            || clean
                .iter()
                .any(|y| y.len() != (0.08 * f64::from(rate)).round() as usize)
            || !take.diagnostics["last_contact_seconds"]
                .as_f64()
                .is_some_and(|v| v < 0.02)
        {
            return Err("invalid replay history".into());
        }
        let mut rows = Vec::new();
        for (row_index, row) in source_case.observations.iter().enumerate() {
            if (row.snr_db, row.seed) != expected_noise[row_index] {
                return Err("unexpected source noise layout".into());
            }
            let (measured, sigma) =
                robustness::additive_voltage_noise(&clean, row.snr_db, row.seed);
            let mut profile = Profile {
                spectrum: &p.spectrum,
                structural: &p.structural,
                damper: &p.damper,
                sensor,
                rate,
                training: core::array::from_fn(|i| measured[i][..measured[i].len() / 2].to_vec()),
                weighting: Weighting::RelativeWindows,
                evaluations: Vec::new(),
            };
            let diagnosis = match diagnose(&mut profile, row, sigma) {
                Ok(v) => v,
                Err(e) => json!({"error":e.to_string()}),
            };
            rows.push(json!({"source_row_index":row_index,"snr_db":row.snr_db,"seed":row.seed,"noise_standard_deviation":sigma,
                "diagnosis":diagnosis,"profile_evaluations":profile.evaluations}));
        }
        let passed = take.diagnostics["mechanical_checks_passed"] == true
            && rows
                .iter()
                .all(|r| r["diagnosis"]["center_replayed"] == true);
        cases.push(json!({"source_case_index":case_index,"sample_rate":rate,"controls_passed":passed,"diagnostics":take.diagnostics,"observations":rows}));
        println!("Loss resolution: completed source case {case_index} at {rate} Hz");
    }
    Ok(
        json!({"schema_version":1,"experiment":"nonlinear-magnetic-loss-resolution-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),
        "source_git_blob_sha1":SOURCE_BLOB,"source_sha256":SOURCE_SHA256,"cases":cases,
        "protocol":"Frozen before first run. Verify pinned noisy-loss receipt bytes using git hash-object; narrow projection reads only rate, noise condition, estimated scales and training RMSE. Replay six original synthetic histories, no outer loss search. For each of 30 rows refit state at center and ten nearby loss pairs with the original inner optimizer and three starts. Require center RMSE replay within absolute 1e-8. Axial log perturbations +/-ln(1.01) estimate profiled residual derivatives; axial +/-ln(1.05) and weakest raw-voltage derivative direction +/-ln(1.05) explore alternatives. Multiplicative decreases use reciprocals, not exact -1%/-5%. Every candidate refits one continuous 18-coordinate state from measured training voltage only; no held-out voltage, reference state or reference loss errors enter diagnosis. Original normalized residuals are unweighted back into voltage with sqrt(2)*training-window norm. Reorthogonalized two-column QR gives a stable smallest singular value via det(R)/largest; weak direction comes from the two-column Gram orientation. For noisy rows divide voltage derivatives and alternative-minus-center voltage L2 distance by injected sigma. Report distances below one noise unit descriptively and radius sigma/minimum raw singular value in log-scale norm. Withhold noise-scaled quantities for sigma=0. All candidate outcomes and failures retained; no optimizer or fit retuning.",
        "scope":"Training-only local sensitivity, not confidence intervals, posterior probability, global identifiability or a new optimum. Oracle sigma comes from injection. One noise unit is a descriptive signal-distance threshold, not waveform RMS noise or a calibrated hypothesis test. Existing relative-window state fit is not a whitened-noise likelihood. Reference fixtures only regenerate signals; truth errors never choose alternatives. No source calibration, antialiasing, sensor uncertainty or production DSP/audio/host change."}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 5 || args[1] != "--input" || args[3] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[4]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let input = source(Path::new(&args[2]))?;
    let report = study(&input)?;
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("loss resolution study retained failed replay controls".into());
    }
    println!("Loss resolution: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiled_residual_unweighting_restores_each_training_window_voltage() {
        let p = prepare(perturbations()[0]).unwrap();
        let sensor = sensors().unwrap()[0];
        let t = templates(&p.spectrum, &p.structural, &p.damper, 1.0, 1.0, 48000).unwrap();
        let measured = [vec![2.0; 40], vec![5.0; 40]];
        let data = Training::new(&t, &measured).unwrap();
        let c = Candidate {
            scales: [1.0, 1.0],
            state: vec![0.0; 18],
            residual: data.residual(sensor, &[0.0; 18]),
            objective: 1.0,
            evaluation_index: 0,
        };
        let profile = Profile {
            spectrum: &p.spectrum,
            structural: &p.structural,
            damper: &p.damper,
            sensor,
            rate: 48000,
            training: measured.clone(),
            weighting: Weighting::RelativeWindows,
            evaluations: Vec::new(),
        };
        let voltage = residual_in_voltage(&profile, &c);
        for (actual, expected) in voltage.iter().zip(measured.iter().flatten()) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }
    #[test]
    fn profile_singular_values_preserve_small_independent_direction() {
        let s = sensitivity(&[vec![2.0, 0.0], vec![0.0, 1.0]]).unwrap();
        assert!((s.singular_values[0] - 2.0).abs() < 1e-14);
        assert!((s.singular_values[1] - 1.0).abs() < 1e-14);
        assert!(s.weak_direction[0].abs() < 1e-14);
        let small = sensitivity(&[vec![1.0, 0.0], vec![1.0, 1e-10]]).unwrap();
        assert!((small.singular_values[1] / (1e-10 / 2.0_f64.sqrt()) - 1.0).abs() < 1e-12);
        assert_eq!(
            sensitivity(&[vec![1.0, 0.0], vec![2.0, 0.0]])
                .unwrap()
                .singular_values[1],
            0.0
        );
    }
    #[test]
    fn weak_direction_is_unit_length_and_minimizes_linear_response() {
        let cols = [vec![3.0, 1.0, 2.0], vec![1.0, 2.0, -1.0]];
        let s = sensitivity(&cols).unwrap();
        let d = s.weak_direction;
        assert!((dot(&d, &d) - 1.0).abs() < 1e-14);
        let response: Vec<_> = cols[0]
            .iter()
            .zip(&cols[1])
            .map(|(a, b)| a * d[0] + b * d[1])
            .collect();
        assert!((dot(&response, &response).sqrt() - s.singular_values[1]).abs() < 1e-12);
        assert!(sensitivity(&[Vec::new(), Vec::new()]).is_err());
    }
}
