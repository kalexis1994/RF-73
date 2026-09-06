use rf_73_analysis::{AudioClip, EnvelopeOptions, EnvelopeRejection, measure_component_envelope};
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Component envelopes:
  component-envelope INPUT.wav --output REPORT.json --frequency-hz HZ --start SECONDS --end SECONDS
                     [--window-ms 128] [--hop-ms 32] [--neighbors-hz F1,F2] [--channel 0]
  validate-envelope --output REPORT.json
Fixed-frequency Hann complex projection, conditional decay/phase gates.
Explicit interval is an observation region, not an inferred sustain boundary.
No mode identification, measured physical damping, T60 extrapolation or audio playback.
";

#[derive(Clone, Copy)]
enum Probe {
    Clean,
    ModerateNoise,
    ResolvedNeighbor,
    UnresolvedNeighbor,
    LowMargin,
    Stationary,
    ChangingDecay,
    WrongCarrier,
    SidebandMixture,
}
impl Probe {
    fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::ModerateNoise => "moderate_noise",
            Self::ResolvedNeighbor => "resolved_neighbor",
            Self::UnresolvedNeighbor => "known_unresolved_neighbor",
            Self::LowMargin => "low_margin",
            Self::Stationary => "stationary",
            Self::ChangingDecay => "changing_decay",
            Self::WrongCarrier => "wrong_carrier",
            Self::SidebandMixture => "sideband_mixture",
        }
    }
    fn expected_rejection(self) -> Option<EnvelopeRejection> {
        match self {
            Self::UnresolvedNeighbor => Some(EnvelopeRejection::KnownUnresolvedNeighbor),
            Self::LowMargin => Some(EnvelopeRejection::LowMargin),
            Self::Stationary => Some(EnvelopeRejection::InsufficientDecay),
            Self::ChangingDecay => Some(EnvelopeRejection::InconsistentSlopes),
            Self::WrongCarrier => Some(EnvelopeRejection::FrequencyMismatch),
            _ => None,
        }
    }
    fn frequency(self) -> f64 {
        if matches!(self, Self::SidebandMixture) {
            1426.7578125 + 196.2890625
        } else {
            1426.7578125
        }
    }
    fn rate(self) -> f64 {
        if matches!(self, Self::SidebandMixture) {
            5.8
        } else {
            3.0
        }
    }
    fn neighbors(self) -> Vec<f64> {
        match self {
            Self::ResolvedNeighbor => vec![self.frequency() + 40.0],
            Self::UnresolvedNeighbor => vec![self.frequency() + 4.0],
            Self::SidebandMixture => vec![196.2890625, 1426.7578125, 1426.7578125 - 196.2890625],
            _ => vec![],
        }
    }
}
fn synthesize(probe: Probe, sample_rate: u32) -> Result<AudioClip, Box<dyn Error>> {
    let mut seed = 0x73_5eed_u64;
    let neighbor_frequency = probe.neighbors().first().copied().unwrap_or(0.0);
    let samples = (0..sample_rate * 2)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = ((seed >> 11) as f64 / ((1u64 << 53) as f64) - 0.5) * 2.0;
            let decay_exponent = match probe {
                Probe::Stationary => 0.0,
                Probe::ChangingDecay => t.min(0.7) + 6.0 * (t - 0.7).max(0.0),
                _ => probe.rate() * t,
            };
            let shift = if matches!(probe, Probe::WrongCarrier) {
                2.0
            } else {
                0.0
            };
            let amplitude = if matches!(probe, Probe::LowMargin) {
                0.01
            } else if matches!(probe, Probe::SidebandMixture) {
                0.04
            } else {
                0.2
            };
            let mut x = amplitude
                * (-decay_exponent).exp()
                * (TAU * (probe.frequency() + shift) * t + 0.73).cos();
            match probe {
                Probe::ModerateNoise => x += 0.001 * noise,
                Probe::LowMargin => x += 0.02 * noise,
                Probe::ResolvedNeighbor | Probe::UnresolvedNeighbor => {
                    let f = neighbor_frequency;
                    x += 0.18 * (-0.7 * t).exp() * (TAU * f * t - 0.4).cos();
                }
                Probe::SidebandMixture => {
                    x += 0.5 * (-0.8 * t).exp() * (TAU * 196.2890625 * t + 0.37).cos();
                    x += 0.2 * (-5.0 * t).exp() * (TAU * 1426.7578125 * t - 0.61).cos();
                    x += 0.03
                        * (-5.8 * t).exp()
                        * (TAU * (1426.7578125 - 196.2890625) * t - 0.98).cos();
                }
                _ => {}
            }
            x
        })
        .collect();
    Ok(AudioClip::from_samples(sample_rate, samples)?)
}

fn validate() -> Result<Value, Box<dyn Error>> {
    let mut cases = Vec::new();
    for probe in [
        Probe::Clean,
        Probe::ModerateNoise,
        Probe::ResolvedNeighbor,
        Probe::UnresolvedNeighbor,
        Probe::LowMargin,
        Probe::Stationary,
        Probe::ChangingDecay,
        Probe::WrongCarrier,
        Probe::SidebandMixture,
    ] {
        let clip = synthesize(probe, 48000)?;
        for window in [0.128, 0.256] {
            let report = measure_component_envelope(
                &clip,
                EnvelopeOptions {
                    frequency_hz: probe.frequency(),
                    start_seconds: 0.1,
                    end_seconds: 1.5,
                    window_seconds: window,
                    hop_seconds: window / 4.0,
                    known_neighbor_frequencies_hz: probe.neighbors(),
                },
            )?;
            let rejection = probe.expected_rejection();
            let expected_qualified = rejection.is_none();
            let rate_error = report
                .provisional_fit
                .as_ref()
                .map(|f| (f.amplitude_decay_per_second - probe.rate()).abs());
            let passed = if let Some(ref reason) = rejection {
                !report.qualified && report.rejection_reasons.contains(reason)
            } else {
                report.qualified && rate_error.is_some_and(|e| e < 0.05)
            };
            cases.push(json!({"probe": probe.name(), "expected_qualified": expected_qualified,
                "expected_rejection": rejection, "expected_amplitude_decay_per_second": expected_qualified.then(||probe.rate()),
                "qualified_rate_absolute_error": expected_qualified.then_some(rate_error).flatten(),
                "expectation_passed": passed, "measurement": report}));
        }
    }
    Ok(
        json!({"schema_version": 1, "experiment": "temporal-component-envelope-validation-v1",
        "sample_rate_hz": 48000, "duration_seconds": 2.0, "noise_generator": "LCG64 seed 0x735eed; uniform [-1,1); fixed per probe",
        "all_expectations_passed": cases.iter().all(|c| c["expectation_passed"] == true), "cases": cases,
        "scope": "Nine prescribed temporal waveforms at two window lengths. Known frequencies/rates are synthetic truth; fitted rate is unconstrained. Accepted-case absolute rate error below 0.05 /s; rejected cases require the declared reason. No recording calibration, stochastic coverage, mechanical identification, T60 or listening claim. Sideband mixture is a sum of prescribed damped sinusoids, not a full nonlinear pickup render."}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let validation = args[0] == "validate-envelope";
    let positional = if validation { 1 } else { 2 };
    if args.len() < positional || !(args.len() - positional).is_multiple_of(2) {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    for pair in args[positional..].as_chunks::<2>().0 {
        let key = pair[0].as_str();
        if !(key == "--output"
            || (!validation
                && matches!(
                    key,
                    "--frequency-hz"
                        | "--start"
                        | "--end"
                        | "--window-ms"
                        | "--hop-ms"
                        | "--neighbors-hz"
                        | "--channel"
                )))
            || flags.insert(key, pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or duplicate envelope option: {key}").into());
        }
    }
    let output = Path::new(flags.get("--output").ok_or("--output is required")?);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = if validation {
        validate()?
    } else {
        let required = |key| -> Result<f64, Box<dyn Error>> {
            Ok(flags
                .get(key)
                .ok_or_else(|| format!("{key} is required"))?
                .parse()?)
        };
        let options = EnvelopeOptions {
            frequency_hz: required("--frequency-hz")?,
            start_seconds: required("--start")?,
            end_seconds: required("--end")?,
            window_seconds: flags
                .get("--window-ms")
                .map_or(Ok(128.0), |v| v.parse::<f64>())?
                / 1000.0,
            hop_seconds: flags
                .get("--hop-ms")
                .map_or(Ok(32.0), |v| v.parse::<f64>())?
                / 1000.0,
            known_neighbor_frequencies_hz: flags
                .get("--neighbors-hz")
                .map(|v| {
                    v.split(',')
                        .map(str::parse::<f64>)
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default(),
        };
        let channel = flags
            .get("--channel")
            .map(|v| v.parse::<u16>())
            .transpose()?;
        let clip = AudioClip::open(&args[1], channel)?;
        json!({"source": args[1], "audio": clip.metadata(), "measurement": measure_component_envelope(&clip, options)?})
    };
    super::analysis::write_report(output, &report)?;
    if validation && report["all_expectations_passed"] != true {
        return Err("envelope validation retained failed expectations".into());
    }
    println!("Component envelope report: {}", output.display());
    Ok(())
}
