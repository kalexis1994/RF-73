//! Bounded geometry sensitivity at fixed coupled pitch; no timbral selection.
use super::tuning::{self, Selected, follow_local, spectrum_with_mass};
use rf_73_dsp::{ModalAssemblyProfile, TineGeometry};
use serde_json::{Value, json};
use std::{error::Error, fs::File, io::Read, path::Path};

pub const HELP: &str = "Fixed-pitch geometry sensitivity:
  sweep-tuned-geometry REFERENCE.json --output REPORT.json
Nine fixed cases: 68/70/72 mm tine, 0.08/0.10/0.12 g point tuning mass.
Restore the frozen qualified G3 frequency using spring position within 0.50..0.95 L.
Start at 0.85 L; track physical fields in 0.025 L steps, then bounded bisection.
Retain unreachable and rejected cases, all coupled modes and linear port residues.
This is undamped structural sensitivity, not audio, mode identification or a fit
to the exploratory 1425 Hz family. No parameter selection or plugin change.
";
const LOW: f64 = 0.5;
const HIGH: f64 = 0.95;
const START: f64 = 0.85;
const TOLERANCE_CENTS: f64 = 0.0001;

fn trace(s: &Selected, score: f64, target: f64) -> Value {
    json!({"spring_position_fraction":s.position,"frequency_hz":s.mode().frequency_hz,
        "error_cents":1200.0*(s.mode().frequency_hz/target).log2(),"spatial_mac":score})
}

fn cell(length: f64, mass: f64, target: f64) -> Value {
    let mut trials = Vec::new();
    let mut initial = Value::Null;
    let mut minimum_mac = 1.0_f64;
    let attempt = (|| -> Result<Option<Selected>, Box<dyn Error>> {
        if !target.is_finite() || !(150.0..=250.0).contains(&target) {
            return Err("invalid finite G3 target".into());
        }
        let mut previous = spectrum_with_mass(length, mass, START)?;
        initial = previous.row(1.0);
        trials.push(trace(&previous, 1.0, target));
        if previous.mode().first_tine_projection < 0.5 {
            return Err("initial first-tine coordinate is not dominant".into());
        }
        let initial_error = 1200.0 * (previous.mode().frequency_hz / target).log2();
        if initial_error.abs() < TOLERANCE_CENTS {
            return Ok(Some(previous));
        }
        // Outward movement lowers pitch; the bracket names below refer to Hz.
        let inward = initial_error < 0.0;
        let steps = if inward { 14 } else { 4 };
        let mut bracket = None;
        for step in 1..=steps {
            let position =
                (START + if inward { -1.0 } else { 1.0 } * step as f64 * 0.025).clamp(LOW, HIGH);
            let (next, score) = follow_local(&previous, position)?;
            minimum_mac = minimum_mac.min(score);
            trials.push(trace(&next, score, target));
            let change = next.mode().frequency_hz - previous.mode().frequency_hz;
            if (inward && change <= 0.0) || (!inward && change >= 0.0) {
                return Err("tracked pitch is nonmonotonic in the spring search".into());
            }
            if (inward && next.mode().frequency_hz >= target)
                || (!inward && next.mode().frequency_hz <= target)
            {
                bracket = Some(if inward {
                    (previous, next)
                } else {
                    (next, previous)
                });
                break;
            }
            previous = next;
        }
        let Some((mut low, mut high)) = bracket else {
            return Ok(None);
        };
        for _ in 0..32 {
            let position = 0.5 * (low.position + high.position);
            let (next, score) = follow_local(&low, position)?;
            let (other, reverse_score) = follow_local(&high, position)?;
            minimum_mac = minimum_mac.min(score).min(reverse_score);
            trials.push(trace(&next, score.min(reverse_score), target));
            // Comparing final frequencies alone cannot establish branch identity.
            if next.mode().shape != other.mode().shape {
                return Err("bracket endpoints disagree on mode identity".into());
            }
            let error = 1200.0 * (next.mode().frequency_hz / target).log2();
            if error.abs() < TOLERANCE_CENTS {
                return Ok(Some(next));
            }
            if error < 0.0 {
                low = next;
            } else {
                high = next;
            }
        }
        Err("bounded fixed-pitch bisection did not converge".into())
    })();
    let (status, reason, solution) = match attempt {
        Ok(Some(s)) => (
            "retuned",
            Value::Null,
            s.row(
                trials
                    .last()
                    .and_then(|r| r["spatial_mac"].as_f64())
                    .unwrap_or(1.0),
            ),
        ),
        Ok(None) => (
            "unreachable_within_spring_bounds",
            json!("monotonic tracked branch did not bracket target"),
            Value::Null,
        ),
        Err(e) => ("rejected", json!(e.to_string()), Value::Null),
    };
    json!({"tine_length_mm":length*1000.0,"tuning_mass_g":mass*1000.0,"status":status,"reason":reason,
        "target_hz":target,"initial_structure":initial,"solution":solution,
        "minimum_spatial_mac":minimum_mac,"trials":trials})
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let mut bytes = Vec::new();
    File::open(&args[1])?.take(262145).read_to_end(&mut bytes)?;
    if bytes.len() > 262144 {
        return Err("frequency receipt exceeds 256 KiB".into());
    }
    let reference: Value = serde_json::from_slice(&bytes)?;
    let target = tuning::reference(&reference)?;
    let mut cases = Vec::new();
    for length in [0.068, 0.070, 0.072] {
        for mass in [0.00008, 0.00010, 0.00012] {
            let result = cell(length, mass, target);
            println!(
                "Geometry {:.1} mm / {:.2} g: {}",
                length * 1000.0,
                mass * 1000.0,
                result["status"]
            );
            cases.push(result);
        }
    }
    let rejected = cases.iter().filter(|c| c["status"] == "rejected").count();
    let retuned = cases.iter().filter(|c| c["status"] == "retuned").count();
    let g = TineGeometry::default();
    let p = ModalAssemblyProfile::default();
    let report = json!({"schema_version":1,"experiment":"fixed-pitch-geometry-sensitivity-v1",
        "completed_without_numerical_rejections":rejected==0,"retuned_cases":retuned,
        "unreachable_cases":cases.len()-retuned-rejected,"rejected_cases":rejected,
        "frozen_reference_file":args[1],"frozen_reference":reference,
        "shared_structural_parameters":{"tine_elements":64,"diameter_m":g.diameter_m,
            "young_modulus_pa":g.young_modulus_pa,"density_kg_m3":g.density_kg_m3,
            "hammer_position_fraction":g.hammer_position,"pickup_position_fraction":g.pickup_position,
            "support_mass_kg":p.support_mass_kg,"support_inertia_kg_m2":p.support_inertia_kg_m2,
            "root_translation_stiffness_n_m":p.translation_stiffness_n_m,
            "root_rotation_stiffness_n_m_rad":p.rotation_stiffness_n_m_rad,
            "tonebar_mass_kg":p.tonebar_mass_kg,"tonebar_arm_m":p.tonebar_arm_m,
            "tonebar_frequency_hz":p.tonebar_frequency_hz},
        "protocol":{"tine_lengths_mm":[68,70,72],"tuning_masses_g":[0.08,0.10,0.12],
            "spring_position_fraction_bounds":[LOW,HIGH],"initial_spring_fraction":START,
            "coarse_step_fraction":0.025,"maximum_bisection_iterations":32,
            "pitch_tolerance_cents":TOLERANCE_CENTS,"minimum_mac":0.98,"maximum_runner_up_mac":0.05},
        "cases":cases,
        "scope":"Undamped fixed-pitch sensitivity only. Per-cell physical-field tracking uses that cell's fixed length/mass and pairwise average actual spring inertia; no cross-cell MAC or identity assignment for higher modes. Higher coupled modes are frequency ranks. Diameter/material, root/tonebar and normalized hammer/pickup positions remain at defaults; their absolute tine locations scale with length. The sign-invariant hammer/pickup product describes the linear velocity response to a force impulse, not nonlinear pickup amplitude or an audible timbre fit. No 1425 Hz target, best-cell selection, audio rendering, geometry identification or plugin mutation."});
    crate::analysis::write_report(output, &report)?;
    if rejected > 0 {
        return Err("geometry study retained numerical/branch rejections; inspect report".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retuning_preserves_pitch_while_mass_changes_higher_modes_and_ports() {
        let source: Value = serde_json::from_str(include_str!(
            "../../../../references/g3-pitch-reference-validation.json"
        ))
        .unwrap();
        let target = tuning::reference(&source).unwrap();
        let mut solved = Vec::new();
        for mass in [0.00008, 0.0001, 0.00012] {
            let c = cell(0.070, mass, target);
            assert_eq!(c["status"], "retuned", "{c}");
            let s = &c["solution"];
            assert_eq!(s["tuning_mass_kg"], mass);
            assert!(
                (1200.0 * (s["selected_frequency_hz"].as_f64().unwrap() / target).log2()).abs()
                    < TOLERANCE_CENTS
            );
            assert!(c["minimum_spatial_mac"].as_f64().unwrap() >= 0.98);
            let mode = &s["coupled_modes_by_frequency_rank"][4];
            solved.push((
                mode["frequency_hz"].as_f64().unwrap(),
                mode["linear_hammer_to_pickup_velocity_residue_per_kg"]
                    .as_f64()
                    .unwrap(),
            ));
            if mass == 0.0001 {
                assert!(
                    (s["spring_center_from_root_mm"].as_f64().unwrap() - 55.7408218383789).abs()
                        < 0.0001
                );
            }
        }
        assert!((solved[0].0 - solved[2].0).abs() > 1.0);
        assert!((solved[0].1 - solved[2].1).abs() > 0.01);
    }
    #[test]
    fn both_directions_boundaries_unreachable_and_invalid_cases_are_explicit() {
        let initial = spectrum_with_mass(0.070, 0.0001, START)
            .unwrap()
            .mode()
            .frequency_hz;
        for (target, above) in [(initial - 1.0, true), (initial + 1.0, false)] {
            let c = cell(0.070, 0.0001, target);
            assert_eq!(c["status"], "retuned");
            assert_eq!(
                c["solution"]["spring_position_fraction"].as_f64().unwrap() > START,
                above
            );
        }
        let exact = cell(0.070, 0.0001, initial);
        assert_eq!(exact["trials"].as_array().unwrap().len(), 1);
        let missing = cell(0.075, 0.0001, 196.386);
        assert_eq!(missing["status"], "unreachable_within_spring_bounds");
        assert!(missing["solution"].is_null());
        assert_eq!(
            missing["trials"].as_array().unwrap().last().unwrap()["spring_position_fraction"],
            LOW
        );
        assert_eq!(cell(0.070, 0.0001, f64::NAN)["status"], "rejected");
        assert_eq!(cell(0.0, 0.0001, 196.386)["status"], "rejected");
    }
}
