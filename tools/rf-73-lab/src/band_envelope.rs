//! Prescribed distant-interference challenge; no recording selection or tuning.
use rf_73_analysis::{
    AudioClip, ShortEnvelopeOptions, measure_band_envelope, measure_short_envelope,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    f64::consts::{PI, TAU},
    path::Path,
};

pub const HELP: &str = "Band-isolated short envelope validation:
  validate-band-envelope --output REPORT.json
Fixed 44.1/48/96 kHz synthetic mixtures, 32/64 ms regression and bounded centered FIR.
Retains unfiltered controls and failed expectations. No source processing or physical calibration.
";

fn validation() -> Result<Value, Box<dyn Error>> {
    let mut cases = Vec::new();
    for rate in [44100, 48000, 96000] {
        for probe in [
            "clean",
            "distant_fundamental",
            "harmonics_and_neighbor",
            "detuned_target",
            "noise",
            "changing_decay",
            "stationary",
            "missing_target",
            "unresolved_carriers",
            "onset_inside_interval",
            "low_margin",
        ] {
            let accepted = matches!(
                probe,
                "clean"
                    | "distant_fundamental"
                    | "harmonics_and_neighbor"
                    | "detuned_target"
                    | "noise"
            );
            let expected_rejection = match probe {
                "changing_decay" => Some("inconsistent_slopes"),
                "stationary" => Some("insufficient_decay"),
                "unresolved_carriers" => Some("ill_conditioned_carriers"),
                "low_margin" => Some("low_regression_margin"),
                "missing_target" | "onset_inside_interval" => Some("any_gate"),
                _ => None,
            };
            let offset = if probe == "detuned_target" { 2.0 } else { 0.0 };
            let neighbor = if probe == "unresolved_carriers" {
                1620.5
            } else {
                1568.0
            };
            let mut seed = 0x73_ba11_u64;
            let samples = (0..rate / 4)
                .map(|i| {
                    let t = i as f64 / rate as f64;
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let noise = 2.0 * ((seed >> 11) as f64 / (1_u64 << 53) as f64) - 1.0;
                    let decay = match probe {
                        "changing_decay" => 4.0 * t.min(0.09) + 18.0 * (t - 0.09).max(0.0),
                        "stationary" => 0.0,
                        _ => 8.0 * t,
                    };
                    let mut x = if probe == "missing_target"
                        || (probe == "onset_inside_interval" && t < 0.04)
                    {
                        0.0
                    } else {
                        0.002 * (-decay).exp() * (TAU * (1620.0 + offset) * t + 0.73).cos()
                    };
                    if probe != "clean" {
                        x += 0.6 * (-0.33 * t).exp() * (TAU * 196.35 * t + 0.1).cos();
                    }
                    if !matches!(probe, "clean" | "distant_fundamental") {
                        x += 0.02 * (-t).exp() * (TAU * neighbor * t - 0.4).cos()
                            + 0.05 * (-0.66 * t).exp() * (TAU * 392.7 * t + 0.9).cos()
                            + 0.015 * (-0.99 * t).exp() * (TAU * 589.05 * t - 0.7).cos();
                    }
                    if probe == "noise" {
                        x += 0.00001 * noise;
                    }
                    if probe == "low_margin" {
                        x += 0.03 * noise;
                    }
                    x
                })
                .collect();
            let clip = AudioClip::from_samples(rate, samples)?;
            for window in [0.032, 0.064] {
                let options = ShortEnvelopeOptions {
                    frequencies_hz: vec![1620.0, neighbor],
                    start_seconds: 0.02,
                    end_seconds: 0.18,
                    window_seconds: window,
                    hop_seconds: 0.008,
                };
                let raw = measure_short_envelope(&clip, options.clone())?;
                let filtered = measure_band_envelope(&clip, options)?;
                let m = &filtered.measurement;
                let decay_error = m
                    .provisional_fit
                    .as_ref()
                    .map(|f| (f.amplitude_decay_per_second - 8.0).abs());
                let carrier_error = m
                    .provisional_fit
                    .as_ref()
                    .map(|f| (f.carrier_offset_hz - offset).abs());
                let amplitude_bias = m
                    .points
                    .iter()
                    .map(|p| {
                        let expected = 0.002 * (-8.0 * p.center_seconds).exp();
                        (p.amplitude_dbfs - 20.0 * expected.log10()).abs()
                    })
                    .reduce(f64::max);
                let phase_bias = m
                    .points
                    .iter()
                    .map(|p| {
                        (p.unwrapped_phase_radians - 0.73 - TAU * offset * p.center_seconds + PI)
                            .rem_euclid(TAU)
                            - PI
                    })
                    .map(f64::abs)
                    .reduce(f64::max);
                let passed = if let Some(reason) = expected_rejection {
                    !m.qualified && (reason == "any_gate" || m.rejection_reasons.contains(&reason))
                } else {
                    m.qualified
                        && decay_error.is_some_and(|x| x < 0.1)
                        && carrier_error.is_some_and(|x| x < 0.02)
                        && amplitude_bias.is_some_and(|x| x < 0.05)
                        && phase_bias.is_some_and(|x| x < 0.01)
                };
                cases.push(json!({"probe":probe,"sample_rate_hz":rate,"expected_rejection":expected_rejection,
                    "expectation_passed":passed,
                    "accepted_case_errors":if accepted {json!({"decay_per_second":decay_error,"carrier_hz":carrier_error,
                        "maximum_amplitude_bias_db":amplitude_bias,"maximum_phase_bias_radians":phase_bias})} else {Value::Null},
                    "unfiltered_control":{"qualified":raw.qualified,"rejection_reasons":raw.rejection_reasons,
                        "provisional_fit":raw.provisional_fit,"minimum_regression_margin_db":raw.points.iter().map(|p|p.regression_margin_db).reduce(f64::min)},
                    "filtered":filtered}));
            }
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"band-isolated-short-envelope-validation-v1",
        "all_expectations_passed":cases.iter().all(|c|c["expectation_passed"]==true),"cases":cases,
        "protocol":"Declared before first run: eleven 0.25 s probes at 44.1/48/96 kHz, fixed 0.02..0.18 s, 32/64 ms windows and 8 ms hop. Target 1620 Hz, amplitude 0.002, phase 0.73, decay 8 /s. Distant fundamental 196.35 Hz, amplitude 0.6, phase 0.1, decay 0.33 /s. Neighbor 1568 Hz, amplitude 0.02, phase -0.4, decay 1 /s. Harmonics 392.7/589.05 Hz, amplitudes 0.05/0.015, phases 0.9/-0.7, rates 0.66/0.99 /s. Clean omits all interference; distant-only omits neighbor/harmonics. Detuned target +2 Hz; uniform LCG64 seed 0x73ba11 noise amplitude 0.00001 or 0.03. Changing decay 4 /s until 0.09 s then 18 /s, continuous amplitude; stationary rate zero; missing target amplitude zero; unresolved neighbor at 1620.5 Hz; onset target zero before 0.04 s then prescribed absolute-time exponential. Each declares target and neighbor, even where neighbor is absent. Fixed centered Blackman sinc FIR, 300 Hz half-bandwidth, 16 ms half-support, unit stationary target gain. Accepted cases require rate error <0.1 /s, carrier error <0.02 Hz, maximum amplitude bias <0.05 dB and phase bias <0.01 rad versus true unfiltered target; negative cases require their declared rejection. Unfiltered controls retained descriptively, without selecting or adjusting thresholds from their outcomes.",
        "scope":"Known synthetic mixtures only. FIR colors noise and can smooth or anticipate transients inside its real-data halo. Fixed gates are not statistical confidence bounds. No source receipt altered, no file interval/carrier search, mechanical identification, parameter calibration, listening or realtime integration."}),
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
    let report = validation()?;
    crate::analysis::write_report(output, &report)?;
    if report["all_expectations_passed"] != true {
        return Err("band-envelope study retained failed expectations".into());
    }
    println!("Band envelope validation: {}", output.display());
    Ok(())
}
