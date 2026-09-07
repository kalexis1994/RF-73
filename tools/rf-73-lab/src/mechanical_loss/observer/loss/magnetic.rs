//! Existing magnetic laws observed by a deliberately linear continuous-state inverse.
use super::continuity::continuous_outcome;
use super::geometry::{classify, perturbations, prepare};
use super::*;
use rf_73_dsp::MagneticPickup;

pub const HELP: &str = "Magnetic observation loss study:
  magnetic-pickup-loss --output REPORT.json
Existing magnetic laws on shared mechanical histories; rest-slope linear observer.
Linear controls and centered zero-slope withholding; point samples, not antialiased audio.
";

#[derive(Clone, Copy, Serialize)]
struct Sensor {
    law: &'static str,
    geometry: &'static str,
    gap_m: f64,
    offset_m: f64,
    #[serde(skip)]
    pickup: MagneticPickup,
}
impl Sensor {
    fn voltage(self, x: f64, v: f64) -> f64 {
        if self.law == "production" {
            self.pickup.voltage(x, v)
        } else {
            self.pickup.research_point_pole_voltage(x, v)
        }
    }
    fn rest_gain(self) -> f64 {
        self.voltage(0.0, 1.0)
    }
}
fn sensors() -> Result<Vec<Sensor>, Box<dyn Error>> {
    let mut sensors = Vec::new();
    for law in ["production", "point_pole_proxy"] {
        for (geometry, gap_m, offset_m) in [
            ("baseline", 0.0015, 0.0005),
            ("close", 0.0005, 0.00025),
            ("centered", 0.0015, 0.0),
        ] {
            sensors.push(Sensor {
                law,
                geometry,
                gap_m,
                offset_m,
                pickup: MagneticPickup::new(gap_m, offset_m)?,
            });
        }
    }
    Ok(sensors)
}

#[derive(Default)]
struct Forward {
    count: usize,
    squared_voltage: f64,
    squared_linear_error: f64,
    peak_displacement_over_gap: f64,
    reversed_slope_samples: usize,
}
impl Forward {
    fn add(&mut self, s: Sensor, x: f64, v: f64) {
        let voltage = s.voltage(x, v);
        let linear = s.rest_gain() * v;
        self.count += 1;
        self.squared_voltage += voltage * voltage;
        self.squared_linear_error += (voltage - linear).powi(2);
        self.peak_displacement_over_gap = self.peak_displacement_over_gap.max((x / s.gap_m).abs());
        self.reversed_slope_samples += usize::from(s.voltage(x, 1.0) * s.rest_gain() < 0.0);
    }
    fn report(&self) -> Value {
        json!({"mechanical_tick_samples":self.count,"voltage_proxy_rms":(self.squared_voltage/self.count as f64).sqrt(),
            "rest_linearization_relative_rmse":(self.squared_linear_error/self.squared_voltage).sqrt(),
            "peak_displacement_over_gap":self.peak_displacement_over_gap,"slope_reversed_fraction":self.reversed_slope_samples as f64/self.count as f64,
            "scope":"All mechanical ticks in this 80 ms interval, including fit and held-out portions; descriptive forward diagnostics only, never supplied to inverse selection."})
    }
}

fn observation_traces(
    traces: &[Trace; 2],
    positions: &[Vec<f64>; 2],
    s: Sensor,
    linear: bool,
) -> [Trace; 2] {
    core::array::from_fn(|i| Trace {
        pickup: positions[i]
            .iter()
            .zip(&traces[i].pickup)
            .map(|(x, v)| {
                if linear {
                    s.rest_gain() * v
                } else {
                    s.voltage(*x, *v)
                }
            })
            .collect(),
        truth: traces[i].truth.clone(),
        contact_free: traces[i].contact_free,
    })
}

fn observation_names(mut fit: Value) -> Value {
    if let Some(windows) = fit["windows"].as_array_mut() {
        for window in windows {
            if let Some(map) = window.as_object_mut() {
                for (old, new) in [
                    (
                        "held_out_clean_pickup_relative_rmse",
                        "held_out_clean_observation_relative_rmse",
                    ),
                    (
                        "training_clean_pickup_relative_rmse",
                        "training_clean_observation_relative_rmse",
                    ),
                    (
                        "training_noisy_pickup_rmse_over_clean_rms",
                        "training_observation_relative_rmse",
                    ),
                ] {
                    if let Some(value) = map.remove(old) {
                        map.insert(new.into(), value);
                    }
                }
            }
        }
    }
    fit
}

fn infer_observation(
    base: &Model<'_>,
    traces: &[Trace; 2],
    gain: f64,
) -> Result<Value, Box<dyn Error>> {
    if !gain.is_finite() || gain == 0.0 {
        return Err(
            "zero or invalid rest sensitivity: linear observation has no state information".into(),
        );
    }
    let mut spectrum = base.spectrum.clone();
    for mode in &mut spectrum.modes {
        mode.pickup_weight *= gain;
    }
    let model = Model {
        spectrum: &spectrum,
        structural: base.structural,
        damper: base.damper,
        rate: base.rate,
    };
    Ok(observation_names(continuous_outcome(&model, traces)?))
}

fn result_row(
    base: &Model<'_>,
    traces: &[Trace; 2],
    gain: f64,
    alpha: f64,
    beta: f64,
    control: bool,
) -> Value {
    let fit = match infer_observation(base, traces, gain) {
        Ok(fit) => fit,
        Err(e) => json!({"error":e.to_string()}),
    };
    let classification = classify(&fit, alpha, beta);
    let passed = classification["known_scale_relative_errors"]
        .as_array()
        .is_some_and(|e| e.len() == 2 && e.iter().all(|e| e.as_f64().is_some_and(|e| e < 0.001)))
        && fit["prediction_consistent"] == true
        && fit["windows"].as_array().is_some_and(|w| {
            w.len() == 2
                && w.iter().all(|w| {
                    w["held_out_clean_observation_relative_rmse"]
                        .as_f64()
                        .is_some_and(|e| e < 1e-5)
                })
        });
    json!({"status":if fit["error"].is_string(){"fit_error"}else{"fitted"},"fit":fit,"classification":classification,
        "required_control":control,"required_control_passed":if control{Some(passed)}else{None}})
}

fn study() -> Result<Value, Box<dyn Error>> {
    let prepared = prepare(perturbations()[0])?;
    let sensors = sensors()?;
    let mut cases = Vec::new();
    for (alpha, beta) in [(0.63, 1.37), (1.13, 0.57), (1.47, 0.91)] {
        for rate in [48000, 96000] {
            let mut traces: [Trace; 2] = core::array::from_fn(|_| Trace {
                pickup: Vec::new(),
                truth: Vec::new(),
                contact_free: true,
            });
            let mut positions: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
            let mut forward: Vec<[Forward; 2]> = sensors
                .iter()
                .map(|_| core::array::from_fn(|_| Forward::default()))
                .collect();
            let take =
                simulate_observed(rate, alpha, beta, |tick, tick_rate, before, after, _| {
                    for (i, (start, end)) in [(0.02, 0.10), (0.14, 0.22)].into_iter().enumerate() {
                        if tick >= (start * tick_rate as f64).round() as usize
                            && tick < (end * tick_rate as f64).round() as usize
                        {
                            traces[i].contact_free &=
                                !before.contact_active && !after.contact_active;
                            for (s, stats) in sensors.iter().zip(&mut forward) {
                                stats[i].add(
                                    *s,
                                    before.pickup_displacement_m,
                                    before.pickup_velocity_m_s,
                                );
                            }
                            if tick % (tick_rate / rate as usize) == 0 {
                                traces[i].pickup.push(before.pickup_velocity_m_s);
                                traces[i]
                                    .truth
                                    .push(reference_state(&prepared.spectrum, before));
                                positions[i].push(before.pickup_displacement_m);
                            }
                        }
                    }
                })?;
            let contact_free = take.diagnostics["last_contact_seconds"]
                .as_f64()
                .is_some_and(|t| t < 0.02)
                && traces.iter().all(|t| {
                    t.contact_free && t.pickup.len() == (0.08 * f64::from(rate)).round() as usize
                });
            if !contact_free || !prepared.invariants_passed {
                return Err("invalid magnetic observation mechanics".into());
            }
            let model = Model {
                spectrum: &prepared.spectrum,
                structural: prepared.structural,
                damper: prepared.damper,
                rate,
            };
            let mut observations = Vec::new();
            let mut mechanical = result_row(&model, &traces, 1.0, alpha, beta, true);
            mechanical["observation"] = json!("mechanical_velocity");
            mechanical["observation_units"] = json!("m/s");
            observations.push(mechanical);
            for (s, stats) in sensors.iter().zip(&forward) {
                let gain = s.rest_gain();
                for linear in if gain == 0.0 {
                    vec![false]
                } else {
                    vec![true, false]
                } {
                    let mut row = if gain == 0.0 {
                        let transformed = observation_traces(&traces, &positions, *s, false);
                        let withheld = infer_observation(&model, &transformed, gain)
                            .err()
                            .map(|e| e.to_string());
                        json!({"status":"withheld_zero_rest_sensitivity","reason":withheld,"required_control":true,"required_control_passed":withheld.is_some(),
                            "scope":"Rest-slope H is zero, so this linear observer cannot infer state. The nonlinear voltage can still be nonzero; no loss estimate is attempted."})
                    } else {
                        result_row(
                            &model,
                            &observation_traces(&traces, &positions, *s, linear),
                            gain,
                            alpha,
                            beta,
                            linear,
                        )
                    };
                    row["observation"] = json!(if linear {
                        "linearized_voltage"
                    } else {
                        "nonlinear_voltage"
                    });
                    row["observation_units"] = json!("uncalibrated_voltage_proxy");
                    row["sensor"] = json!(s);
                    row["rest_sensitivity_per_velocity"] = json!(gain);
                    row["nonlinear_forward_diagnostics"] =
                        json!(stats.iter().map(Forward::report).collect::<Vec<_>>());
                    observations.push(row);
                }
            }
            cases.push(json!({"known_structural_scale":alpha,"known_damper_scale":beta,"diagnostics":take.diagnostics,
                "controls_passed":take.diagnostics["mechanical_checks_passed"]==true && observations.iter().filter(|o|o["required_control"]==true).all(|o|o["required_control_passed"]==true),"observations":observations}));
            println!("Magnetic pickup loss: completed {rate} Hz, scales ({alpha}, {beta})");
        }
    }
    let rows: Vec<_> = cases
        .iter()
        .flat_map(|c| c["observations"].as_array().unwrap())
        .collect();
    let count = |f: fn(&Value) -> bool| rows.iter().filter(|o| f(o)).count();
    Ok(
        json!({"schema_version":1,"experiment":"magnetic-observation-loss-v1","controls_passed":cases.iter().all(|c|c["controls_passed"]==true),
        "summary":{"observations":rows.len(),"positive_controls":count(|o|o["required_control"]==true && o["status"]=="fitted"),"centered_withheld":count(|o|o["status"]=="withheld_zero_rest_sensitivity"),
            "nonlinear_fitted":count(|o|o["observation"]=="nonlinear_voltage" && o["status"]=="fitted"),"nonlinear_prediction_consistent":count(|o|o["observation"]=="nonlinear_voltage" && o["fit"]["prediction_consistent"]==true),
            "nonlinear_scale_recovery":count(|o|o["observation"]=="nonlinear_voltage" && o["classification"]["known_scale_recovery_within_one_percent"]==true),"nonlinear_consistent_but_biased":count(|o|o["observation"]=="nonlinear_voltage" && o["classification"]["prediction_consistent_but_biased"]==true),"fit_errors":count(|o|o["status"]=="fit_error")},"cases":cases,
        "protocol":"Frozen before first run. Same six matched mechanical trajectories as continuous-pickup-loss, with unknown structural/damper scales (0.63,1.37),(1.13,0.57),(1.47,0.91) at 48/96 kHz. Existing production and research point-pole magnetic laws, each with (gap,offset) in mm (1.5,0.5),(0.5,0.25),(1.5,0). Fixed longitudinal pickup position 0.98 and damper position 0.8. Mechanical velocity reference, rest-linearized gain*v controls, and full voltage V(x,v) from the exact same motion. Linear observer H is scaled by known rest sensitivity k0=V(0,1), never by instantaneous displacement-dependent gain. Centered zero-slope cases must be withheld before inference. Continuous initial-state and sequential loss searches unchanged; fit [0.02,0.06),[0.14,0.18), hold out [0.06,0.10),[0.18,0.22). Known q,v used only for forward observation generation and state scoring. Thirty positive controls require both scale errors <0.001, both held-out observation errors <1e-5 and existing prediction/mechanical checks; twelve centered controls require explicit withholding. Nonlinear outcomes descriptive with unchanged 0.005 prediction and 1% scored loss gates. Forward linearization error and displacement/gap ratio sampled at every mechanical tick (192/384 kHz); inverse sees direct point samples at 48/96 kHz. No threshold tuning.",
        "scope":"Linear continuous-state inverse deliberately confronted with nonlinear observation, not a nonlinear magnetic inverse. Sensor gain and geometry are supplied assumptions with arbitrary uncalibrated flux scale. Mechanical state and losses are not supplied to fitting. Point-sampled voltage is not antialiased audio; harmonics can alias, and paired rates are not an antialiasing proof. No production FIR/circuit, source-bank fit, finite pole surface, magnetic loading, second motion coordinate or new magnetic law. Production DSP, presets and audio assets unchanged; no host launch."}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = study()?;
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("magnetic pickup loss study retained failed controls".into());
    }
    println!("Magnetic pickup loss: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rest_slope_matches_small_motion_and_centered_voltage_is_not_silence() {
        for s in sensors().unwrap() {
            if s.offset_m == 0.0 {
                assert_eq!(s.rest_gain(), 0.0);
                assert_ne!(s.voltage(0.0001, 0.2), 0.0);
            } else {
                let slope = s.rest_gain();
                assert!(slope.is_finite() && slope > 0.0);
                for v in [-0.2, 0.3] {
                    assert!((s.voltage(1e-12, v) / (slope * v) - 1.0).abs() < 1e-7);
                }
                let mut stats = Forward::default();
                stats.add(s, 0.0, 0.2);
                // V(0,v) and V(0,1)*v associate products differently in the
                // preserved production expression; allow roundoff, not model error.
                assert!(
                    stats.report()["rest_linearization_relative_rmse"]
                        .as_f64()
                        .unwrap()
                        < 8.0 * f64::EPSILON
                );
            }
        }
    }
    #[test]
    fn zero_slope_withholds_inference_before_any_data_fit() {
        let p = prepare(perturbations()[0]).unwrap();
        let model = Model {
            spectrum: &p.spectrum,
            structural: p.structural,
            damper: p.damper,
            rate: 48000,
        };
        let empty = core::array::from_fn(|_| Trace {
            pickup: Vec::new(),
            truth: Vec::new(),
            contact_free: true,
        });
        assert!(
            infer_observation(&model, &empty, 0.0)
                .unwrap_err()
                .to_string()
                .contains("rest sensitivity")
        );
        assert!(infer_observation(&model, &empty, f64::NAN).is_err());
    }
}
