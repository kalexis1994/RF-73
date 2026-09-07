//! Reciprocal spatial pickup, loaded circuit and a common offline output filter.
use rf_73_dsp::{ElectromechanicalAssembly, ElectromechanicalProfile, ProductionDecimator};
use serde_json::{Value, json};
use std::{error::Error, io::BufWriter, path::Path};

pub const HELP: &str = "Electromechanical audit:
  electromechanical --output REPORT.json [--refined]
  electromechanical-render --output AUDIO.wav [--gain FS_PER_VOLT]
Eight loaded two-plane cases; render writes one 1.2-second WAV plus its JSON receipt.
";
const DURATION: f64 = 0.18;
const NAMES: [&str; 4] = ["zero_flux", "load_1k", "load_10k", "open_capacitive"];
fn profile(case: usize, length: f64) -> ElectromechanicalProfile {
    let mut p = ElectromechanicalProfile::default();
    p.geometry.length_m = length;
    match case {
        0 => p.pickup.flux_scale_wb = 0.0,
        1 => p.circuit.load_resistance_ohm = Some(1000.0),
        3 => {
            p.circuit.load_resistance_ohm = None;
            p.circuit.shunt_capacitance_f *= 10.0;
        }
        _ => {}
    }
    p
}
struct Take {
    samples: Vec<[f64; 4]>,
    summary: Value,
}
fn take(length: f64, rate: u32, steps: usize, case: usize) -> Result<Take, Box<dyn Error>> {
    let p = profile(case, length);
    let h = 1.0 / (f64::from(rate) * steps as f64);
    let mut v = ElectromechanicalAssembly::new(h, p)?;
    let mut previous = v.probe();
    let mut x = p.action.hammer_rest_m;
    let mut filter = ProductionDecimator::new();
    let mut samples = Vec::new();
    let mut balance = 0.0_f64;
    let mut exchange = 0.0_f64;
    let mut electrical = 0.0_f64;
    let mut passive = 0.0_f64;
    let mut absolute_electrical_work = 0.0;
    let mut peak_voltage = 0.0_f64;
    let mut peak_force = 0.0_f64;
    let mut heat_monotone = true;
    for frame in 0..(DURATION * f64::from(rate)).round() as usize {
        let mut average = 0.0;
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = if (0.01..0.045).contains(&t) || (0.095..0.13).contains(&t) {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            v.advance(x, p.action.damper_closed_m)?;
            let b = v.probe();
            let a = previous;
            previous = b;
            let scale =
                (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
            balance = balance.max(b.total_balance_residual_j.abs() / scale);
            exchange = exchange.max(b.exchange_residual_j.abs() / scale);
            absolute_electrical_work += (b.circuit_input_work_j - a.circuit_input_work_j).abs();
            electrical = electrical
                .max(b.circuit_balance_residual_j.abs() / absolute_electrical_work.max(1e-24));
            if x == a.mechanical.pedestal_position_m {
                passive = passive.max(
                    (b.mechanical.mechanical_energy_j + b.electrical_energy_j
                        - a.mechanical.mechanical_energy_j
                        - a.electrical_energy_j)
                        / scale,
                );
            }
            heat_monotone &= b.coil_heat_j >= a.coil_heat_j && b.load_heat_j >= a.load_heat_j;
            peak_force = peak_force.max(b.reaction_force_xy_n[0].hypot(b.reaction_force_xy_n[1]));
            average += 0.5 * (a.output_voltage_v + b.output_voltage_v) / (steps / 4) as f64;
            if (sub + 1) % (steps / 4) == 0 {
                filter.push(average);
                average = 0.0;
            }
        }
        let b = previous;
        let voltage = filter.output();
        peak_voltage = peak_voltage.max(voltage.abs());
        samples.push([
            voltage,
            b.current_a,
            b.mechanical.pickup_velocity_xy_m_s[0],
            b.mechanical.pickup_velocity_xy_m_s[1],
        ]);
    }
    let b = previous;
    let behavior = if case == 0 {
        peak_voltage == 0.0 && peak_force == 0.0 && b.coil_heat_j == 0.0 && b.load_heat_j == 0.0
    } else {
        peak_voltage > 1e-4
            && peak_force > 1e-8
            && b.coil_heat_j > 0.0
            && if case == 3 {
                b.load_heat_j == 0.0
            } else {
                b.load_heat_j > 0.0
            }
    };
    let passed = balance < 1e-8
        && exchange < 1e-10
        && electrical < 1e-8
        && passive < 1e-10
        && heat_monotone
        && behavior
        && b.mechanical.contact_entries[0] >= 2;
    Ok(Take {
        samples,
        summary: json!({"steps_per_frame":steps,"passed":passed,"behavior_passed":behavior,
        "max_relative_total_balance_defect":balance,"max_relative_exchange_defect":exchange,"max_relative_circuit_balance_defect":electrical,
        "max_stationary_drive_relative_energy_growth":passive,"heat_monotone":heat_monotone,
        "peak_filtered_voltage_v":peak_voltage,"peak_reaction_force_n":peak_force,"maximum_coupling_iterations":b.maximum_iterations,
        "coil_heat_j":b.coil_heat_j,"load_heat_j":b.load_heat_j,"electrical_energy_j":b.electrical_energy_j,
        "circuit_input_work_j":b.circuit_input_work_j,"absolute_circuit_input_work_j":absolute_electrical_work,
        "mechanical_pickup_work_j":b.mechanical.pickup_force_work_j,"pedestal_work_j":b.mechanical.pedestal_work_j,
        "mechanical_contact_entries":b.mechanical.contact_entries,"final_position":b.mechanical.position,
        "final_velocity":b.mechanical.velocity,"final_current_a":b.current_a,"final_voltage_v":b.output_voltage_v}),
    })
}
fn compare(a: &Take, b: &Take, rate: u32) -> Value {
    let windows:Vec<_>=[(0.0,0.045),(0.045,0.095),(0.095,0.14),(0.14,DURATION)].into_iter().map(|(lo,hi)| {
        let start=(lo*f64::from(rate)).round() as usize;let end=(hi*f64::from(rate)).round() as usize;
        let mut error=[0.0;4];let mut signal=[0.0;4];
        for (x,y) in a.samples[start..end].iter().zip(&b.samples[start..end]) {for j in 0..4 {error[j]+=(x[j]-y[j]).powi(2);signal[j]+=y[j]*y[j];}}
        let relative:[f64;4]=core::array::from_fn(|i|(error[i]/signal[i].max(1e-30)).sqrt());
        json!({"start_seconds":lo,"end_seconds":hi,"relative_rmse_voltage_current_vertical_horizontal":relative,"passed":relative.iter().all(|e|*e<0.01)})
    }).collect();
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
fn settings(p: ElectromechanicalProfile) -> Value {
    json!({"gap_m":p.pickup.gap_m,"offset_xy_m":p.pickup.offset_xy_m,
    "pole_radius_m":p.pickup.pole_radius_m,"flux_scale_wb":p.pickup.flux_scale_wb,
    "inductance_h":p.circuit.inductance_h,"coil_resistance_ohm":p.circuit.coil_resistance_ohm,
    "shunt_capacitance_f":p.circuit.shunt_capacitance_f,"load_resistance_ohm":p.circuit.load_resistance_ohm})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3 | 4)
        || args[1] != "--output"
        || (args.len() == 4 && args[3] != "--refined")
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let steps = if args.len() == 4 {
        [256, 512, 1024]
    } else {
        [64, 128, 256]
    };
    let mut cases = Vec::new();
    for (length, rate) in [(0.075, 48000), (0.12, 96000)] {
        let mut zero: Option<Vec<[f64; 4]>> = None;
        for (case, name) in NAMES.iter().enumerate() {
            let a = take(length, rate, steps[0], case)?;
            let b = take(length, rate, steps[1], case)?;
            let c = take(length, rate, steps[2], case)?;
            let first = compare(&a, &c, rate);
            let second = compare(&b, &c, rate);
            let feedback = zero.as_ref().map_or(0.0, |z| {
                (z.iter()
                    .zip(&c.samples)
                    .map(|(a, b)| (a[2] - b[2]).powi(2) + (a[3] - b[3]).powi(2))
                    .sum::<f64>()
                    / z.len() as f64)
                    .sqrt()
            });
            let passed = [&a, &b, &c].iter().all(|t| t.summary["passed"] == true)
                && first["passed"] == true
                && second["passed"] == true
                && (case == 0 || feedback > 1e-12);
            println!("Electromechanical: {length} m, {rate} Hz, {name}, passed={passed}");
            cases.push(json!({"length_m":length,"sample_rate":rate,"case":name,"passed":passed,"pickup_circuit":settings(profile(case,length)),
                "feedback_velocity_rms_difference_m_s":feedback,"takes":[a.summary,b.summary,c.summary],"coarse_vs_fine":first,"medium_vs_fine":second}));
            if case == 0 {
                zero = Some(c.samples);
            }
        }
    }
    let report = json!({"schema_version":1,"experiment":"reciprocal-electromechanical-v1","passed":cases.iter().all(|c|c["passed"]==true),"steps_per_frame":steps,"cases":cases,
        "protocol":"Frozen before first run. Eight selected cases, 24 takes: 75 mm at 48 kHz and 120 mm at 96 kHz, each with zero flux, 1k load, 10k load and open resistive load with tenfold shunt capacitance. Reuse default two-plane action; two strikes over 180 ms, key down 10-45 and 95-130 ms, pedestal slew 1.5 m/s, pedal closed. Finite-aperture 16-node spatial flux proxy; reciprocal coil-current force; constant-L coil plus series resistance and parallel capacitance/load. Require total energy defect <1e-8, exchange defect <1e-10, circuit energy defect <1e-8, stationary-drive total energy growth <1e-10, monotone electrical heat, two strikes, zero-flux silence and nonzero loaded voltage/reaction/coil heat. Resistive load heat is positive except in zero/open cases. Require nonzero mechanical response change versus zero-flux control. Average circuit midpoint voltage into 4x then use common 127-tap FIR. Both lower resolutions versus highest require <1% RMS error in voltage, current and each mechanical pickup velocity independently in all four time windows.",
        "scope":"Offline uncalibrated reciprocal transducer and single-coil load reduction. No static magnet attraction, nonlinear inductance, hysteresis, eddy-current fit, measured field map, full 73-pickup wiring, amplifier, loudspeaker or realtime plugin integration. Filtered temporal convergence is not a complete aliasing bound."});
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("electromechanical study retained failed qualification".into());
    }
    Ok(())
}
pub fn render(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3 | 5)
        || args[1] != "--output"
        || (args.len() == 5 && args[3] != "--gain")
    {
        return Err(HELP.into());
    }
    let gain: f64 = if args.len() == 5 {
        args[4].parse()?
    } else {
        1.0
    };
    if !gain.is_finite() || !(1e-6..=1.0).contains(&gain) {
        return Err("gain must be finite and within 0.000001..=1 FS/V".into());
    }
    let output = Path::new(&args[2]);
    let receipt = output.with_extension("json");
    if output.extension().is_none_or(|x| x != "wav") || output.exists() || receipt.exists() {
        return Err("render requires new .wav and .json paths".into());
    }
    let rate = 48000;
    let steps = 256;
    let frames = 57600;
    let h = 1.0 / (f64::from(rate) * steps as f64);
    let p = ElectromechanicalProfile::default();
    let mut v = ElectromechanicalAssembly::new(h, p)?;
    let mut filter = ProductionDecimator::new();
    let mut x = p.action.hammer_rest_m;
    let mut previous = 0.0;
    let mut writer =
        crate::wav::FloatWav::new(BufWriter::new(crate::new_file(output)?), rate, frames)?;
    let mut peak = 0.0_f64;
    let mut square = 0.0;
    let mut max_balance = 0.0_f64;
    for frame in 0..frames {
        let mut average = 0.0;
        for sub in 0..steps {
            let t = (frame as usize * steps + sub) as f64 * h;
            let target = if (0.03..0.55).contains(&t) || (0.7..1.0).contains(&t) {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            v.advance(x, p.action.damper_closed_m)?;
            let b = v.probe();
            max_balance = max_balance.max(
                b.total_balance_residual_j.abs()
                    / (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j)
                        .max(1e-20),
            );
            average += 0.5 * (previous + b.output_voltage_v) / (steps / 4) as f64;
            previous = b.output_voltage_v;
            if (sub + 1) % (steps / 4) == 0 {
                filter.push(average);
                average = 0.0;
            }
        }
        let sample = gain * filter.output();
        peak = peak.max(sample.abs());
        square += sample * sample;
        writer.sample(sample as f32)?;
    }
    writer.finish()?;
    let passed = max_balance < 1e-8 && peak > 0.0 && peak < 1.0;
    let b = v.probe();
    crate::analysis::write_report(
        &receipt,
        &json!({"schema_version":1,"experiment":"electromechanical-render-v1","passed":passed,
        "sample_rate":rate,"frames":frames,"steps_per_frame":steps,"seconds":1.2,"peak":peak,"rms":(square/f64::from(frames)).sqrt(),
        "gain_fs_per_volt":gain,"peak_filtered_voltage_v":peak/gain,"max_relative_total_balance_defect":max_balance,"pickup_circuit":settings(p),
        "contact_entries":b.mechanical.contact_entries,"maximum_coupling_iterations":b.maximum_iterations,
        "protocol":"Single 75 mm tine, two physical key gestures, 10k load, 256 midpoint ticks, box average to 4x then common 127-tap FIR. Mono float WAV with explicit fixed gain_fs_per_volt applied only after filtering, no normalization or limiter. Preload relaxation retained. Not a tuned 73-key instrument or host test."}),
    )?;
    if !passed {
        return Err("render retained failed qualification".into());
    }
    println!("Rendered {}", output.display());
    Ok(())
}
