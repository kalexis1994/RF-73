use rf_73_analysis::{AudioClip, ShortEnvelopeOptions, measure_short_envelope};
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Short component envelopes:
  short-envelope INPUT.wav --output REPORT.json --frequencies-hz TARGET,NEIGHBOR --start SECONDS --end SECONDS
                 [--window-ms 32] [--hop-ms 8] [--channel 0]
  validate-short-envelope --output REPORT.json
Joint quadratic complex envelopes for 1..3 declared carriers, reorthogonalized QR.
16..64 ms windows, at most 0.3 s interval. Rejected fits retained; no physical loss calibration.
";
#[derive(Clone, Copy)]
enum Probe {
    Clean,
    Neighbor,
    Noise,
    Detuned,
    OmittedNeighbor,
    Unresolved,
    LowMargin,
    Changing,
    Stationary,
}
impl Probe {
    fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Neighbor => "strong_neighbor",
            Self::Noise => "strong_neighbor_with_noise",
            Self::Detuned => "detuned_target",
            Self::OmittedNeighbor => "omitted_strong_neighbor",
            Self::Unresolved => "unresolved_carriers",
            Self::LowMargin => "low_margin",
            Self::Changing => "changing_decay",
            Self::Stationary => "stationary",
        }
    }
    fn rejection(self) -> Option<&'static str> {
        match self {
            Self::OmittedNeighbor => Some("low_regression_margin"),
            Self::Unresolved => Some("ill_conditioned_carriers"),
            Self::LowMargin => Some("low_regression_margin"),
            Self::Changing => Some("inconsistent_slopes"),
            Self::Stationary => Some("insufficient_decay"),
            _ => None,
        }
    }
    fn carriers(self) -> Vec<f64> {
        match self {
            Self::Clean
            | Self::OmittedNeighbor
            | Self::Changing
            | Self::Stationary
            | Self::LowMargin => vec![1620.0],
            Self::Unresolved => vec![1620.0, 1620.5],
            _ => vec![1620.0, 1568.0],
        }
    }
}
fn synthesize(probe: Probe) -> Result<AudioClip, Box<dyn Error>> {
    let mut seed = 0x73_5eed_u64;
    let samples = (0..12000)
        .map(|i| {
            let t = i as f64 / 48000.0;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = 2.0 * ((seed >> 11) as f64 / (1_u64 << 53) as f64) - 1.0;
            let decay = match probe {
                Probe::Stationary => 0.0,
                Probe::Changing => 4.0 * t.min(0.09) + 18.0 * (t - 0.09).max(0.0),
                _ => 8.0 * t,
            };
            let offset = if matches!(probe, Probe::Detuned) {
                2.0
            } else {
                0.0
            };
            let amplitude = if matches!(probe, Probe::LowMargin) {
                0.002
            } else {
                0.03
            };
            let mut x =
                0.01 + amplitude * (-decay).exp() * (TAU * (1620.0 + offset) * t + 0.73).cos();
            if matches!(
                probe,
                Probe::Neighbor
                    | Probe::Noise
                    | Probe::Detuned
                    | Probe::OmittedNeighbor
                    | Probe::Unresolved
            ) {
                let neighbor = if matches!(probe, Probe::Unresolved) {
                    1620.5
                } else {
                    1568.0
                };
                x += 0.3 * (-t).exp() * (TAU * neighbor * t - 0.4).cos();
            }
            if matches!(probe, Probe::Noise) {
                x += 0.0003 * noise;
            }
            if matches!(probe, Probe::LowMargin) {
                x += 0.03 * noise;
            }
            x
        })
        .collect();
    Ok(AudioClip::from_samples(48000, samples)?)
}
fn validation() -> Result<Value, Box<dyn Error>> {
    let mut cases = Vec::new();
    for probe in [
        Probe::Clean,
        Probe::Neighbor,
        Probe::Noise,
        Probe::Detuned,
        Probe::OmittedNeighbor,
        Probe::Unresolved,
        Probe::LowMargin,
        Probe::Changing,
        Probe::Stationary,
    ] {
        let clip = synthesize(probe)?;
        for window in [0.032, 0.064] {
            let measurement = measure_short_envelope(
                &clip,
                ShortEnvelopeOptions {
                    frequencies_hz: probe.carriers(),
                    start_seconds: 0.02,
                    end_seconds: 0.18,
                    window_seconds: window,
                    hop_seconds: 0.008,
                },
            )?;
            let decay_error = measurement
                .provisional_fit
                .as_ref()
                .map(|f| (f.amplitude_decay_per_second - 8.0).abs());
            let offset = if matches!(probe, Probe::Detuned) {
                2.0
            } else {
                0.0
            };
            let offset_error = measurement
                .provisional_fit
                .as_ref()
                .map(|f| (f.carrier_offset_hz - offset).abs());
            let passed = if let Some(reason) = probe.rejection() {
                !measurement.qualified && measurement.rejection_reasons.contains(&reason)
            } else {
                measurement.qualified
                    && decay_error.is_some_and(|e| e < 0.1)
                    && offset_error.is_some_and(|e| e < 0.02)
            };
            cases.push(json!({"probe":probe.name(),"expected_rejection":probe.rejection(),"expectation_passed":passed,
                "qualified_decay_error_per_second":probe.rejection().is_none().then_some(decay_error).flatten(),
                "qualified_carrier_error_hz":probe.rejection().is_none().then_some(offset_error).flatten(),"measurement":measurement}));
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"joint-short-envelope-validation-v1","sample_rate_hz":48000,"duration_seconds":0.25,
        "all_expectations_passed":cases.iter().all(|c|c["expectation_passed"]==true),"cases":cases,
        "protocol":"Nine prescribed temporal mixtures; 32/64 ms windows, 8 ms hop, fixed 0.02..0.18 s interval. Target 1620 Hz, amplitude 0.03, phase 0.73, decay 8 /s; neighbor 1568 Hz, amplitude 0.3, phase -0.4, decay 1 /s. Common DC 0.01. Detuned target +2 Hz; near-coincident neighbor +0.5 Hz. Uniform LCG64 noise seed 0x735eed, amplitude 0.0003 in noise probe; target 0.002 and noise 0.03 in low-margin probe. Changing decay 4 /s before 0.09 s, then 18 /s; stationary decay zero. Successful cases require rate error below 0.1 /s and carrier error below 0.02 Hz. Rejections must include the declared reason.",
        "scope":"Independent synthetic qualification of an approximate quadratic-envelope estimator. No source recording, nonlinear pickup render, damping/geometry fit, T60, listening or statistical noise coverage. Near-coincident declarations must withhold coefficients; omitted mixtures remain a limitation even when some are rejected."}),
    )
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let validate = args[0] == "validate-short-envelope";
    let position = if validate { 1 } else { 2 };
    if args.len() < position || !(args.len() - position).is_multiple_of(2) {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    for [key, value] in args[position..].as_chunks::<2>().0 {
        if !(key == "--output"
            || (!validate
                && matches!(
                    key.as_str(),
                    "--frequencies-hz"
                        | "--start"
                        | "--end"
                        | "--window-ms"
                        | "--hop-ms"
                        | "--channel"
                )))
            || flags.insert(key.as_str(), value.as_str()).is_some()
        {
            return Err(format!("unknown or duplicate short-envelope option: {key}").into());
        }
    }
    let output = Path::new(flags.get("--output").ok_or("--output is required")?);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = if validate {
        validation()?
    } else {
        let required = |key| -> Result<f64, Box<dyn Error>> {
            Ok(flags
                .get(key)
                .ok_or_else(|| format!("{key} is required"))?
                .parse()?)
        };
        let options = ShortEnvelopeOptions {
            frequencies_hz: flags
                .get("--frequencies-hz")
                .ok_or("--frequencies-hz is required")?
                .split(',')
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>()?,
            start_seconds: required("--start")?,
            end_seconds: required("--end")?,
            window_seconds: flags
                .get("--window-ms")
                .map_or(Ok(32.0), |v| v.parse::<f64>())?
                / 1000.0,
            hop_seconds: flags
                .get("--hop-ms")
                .map_or(Ok(8.0), |v| v.parse::<f64>())?
                / 1000.0,
        };
        let clip = AudioClip::open(
            &args[1],
            flags
                .get("--channel")
                .map(|v| v.parse::<u16>())
                .transpose()?,
        )?;
        json!({"source":args[1],"audio":clip.metadata(),"measurement":measure_short_envelope(&clip,options)?})
    };
    crate::analysis::write_report(output, &report)?;
    if validate && report["all_expectations_passed"] != true {
        return Err("short-envelope study retained failed expectations".into());
    }
    println!("Short envelope report: {}", output.display());
    Ok(())
}
