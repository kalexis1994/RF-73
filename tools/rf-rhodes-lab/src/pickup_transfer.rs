use rf_rhodes_dsp::MagneticPickup;
use serde::Serialize;
use std::{collections::BTreeMap, error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Pickup transfer experiment:
  pickup-transfer --output REPORT.json [--sample-rate HZ] [--period-frames N]
    [--gap-mm X] [--offset-mm X] [--amplitudes-mm CSV]
Default: 44100 Hz, 225 frames/period (196 Hz), gap 1.5, offset 0.5 mm.
Period 16..512; 1..5 unique amplitudes 0.005..3 mm (default 0.05,0.25,0.75).
Identical analytic motion for two flux proxies, 4..128x internal sampling.
Ideal common output band only; no production FIR, mechanics or realism score.
";

const DENSITIES: [usize; 6] = [4, 8, 16, 32, 64, 128];
const ERROR_FLOOR: f64 = 1e-8;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Law {
    Production,
    ResearchPointPole,
}

impl Law {
    fn voltage(self, pickup: MagneticPickup, x: f64, v: f64) -> f64 {
        match self {
            Self::Production => pickup.voltage(x, v),
            Self::ResearchPointPole => pickup.research_point_pole_voltage(x, v),
        }
    }
}

#[derive(Serialize)]
struct Harmonic {
    order: usize,
    amplitude: f64,
    relative_to_fundamental_db: Option<f64>,
}

#[derive(Serialize)]
struct SamplingError {
    internal_oversampling: usize,
    ideal_band_nrmse_against_128x: f64,
    error_db_above_numerical_floor: Option<f64>,
}

#[derive(Serialize)]
struct Observation {
    law: Law,
    amplitude_mm: f64,
    reference_dc: f64,
    reference_ideal_band_rms: f64,
    reference_harmonics: Vec<Harmonic>,
    sampling_errors: Vec<SamplingError>,
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !(args.len() - 1).is_multiple_of(2) {
        return Err(HELP.into());
    }
    let mut flags = BTreeMap::new();
    for pair in args[1..].as_chunks::<2>().0 {
        let flag = pair[0].as_str();
        if !matches!(
            flag,
            "--output"
                | "--sample-rate"
                | "--period-frames"
                | "--gap-mm"
                | "--offset-mm"
                | "--amplitudes-mm"
        ) || flags.insert(flag, pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or duplicate pickup-transfer option: {flag}").into());
        }
    }
    let output = Path::new(flags.get("--output").ok_or("--output is required")?);
    if output.extension().is_none_or(|s| s != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let rate: u32 = flags.get("--sample-rate").unwrap_or(&"44100").parse()?;
    let period: usize = flags.get("--period-frames").unwrap_or(&"225").parse()?;
    let gap: f64 = flags.get("--gap-mm").unwrap_or(&"1.5").parse()?;
    let offset: f64 = flags.get("--offset-mm").unwrap_or(&"0.5").parse()?;
    if ![44100, 48000, 96000, 192000].contains(&rate) || !(16..=512).contains(&period) {
        return Err("unsupported sample rate or period outside 16..512 frames".into());
    }
    let pickup = MagneticPickup::new(gap * 0.001, offset * 0.001)?;
    let amplitudes = super::pickup_sweep::grid(
        flags.get("--amplitudes-mm").unwrap_or(&"0.05,0.25,0.75"),
        0.005,
        3.0,
    )?;
    let frequency = rate as f64 / period as f64;
    let last_harmonic = (0.42 * period as f64).floor() as usize;
    let mut observations = Vec::new();
    for amplitude in amplitudes {
        for law in [Law::Production, Law::ResearchPointPole] {
            observations.push(observe(
                pickup,
                law,
                amplitude,
                frequency,
                period,
                last_harmonic,
            )?);
        }
    }
    super::analysis::write_report(
        output,
        &serde_json::json!({
            "schema_version": 1,
            "experiment": "periodic_pickup_transfer",
            "sample_rate_hz": rate,
            "period_frames": period,
            "frequency_hz": frequency,
            "gap_mm": gap,
            "offset_mm": offset,
            "trajectory": "x=A*cos(2*pi*f*t); v=-A*2*pi*f*sin(2*pi*f*t)",
            "ideal_band_limit_hz": 0.42 * rate as f64,
            "last_retained_harmonic": last_harmonic,
            "reference_oversampling": 128,
            "numerical_reporting_floor_db": 20.0 * ERROR_FLOOR.log10(),
            "limitations": [
                "Finite 128x reference; inspect 64x residual before interpreting 4x error.",
                "Ideal Fourier band projection; no production FIR, transients or mechanical solver.",
                "Single-coordinate flux proxies, uncalibrated common scale; not full published pickup geometry.",
                "More harmonics do not establish realism or improvement against recorded Rhodes audio."
            ],
            "observations": observations,
        }),
    )?;
    println!("Pickup transfer: {}", output.display());
    Ok(())
}

fn observe(
    pickup: MagneticPickup,
    law: Law,
    amplitude_mm: f64,
    frequency: f64,
    period: usize,
    last: usize,
) -> Result<Observation, Box<dyn Error>> {
    let mut spectra = Vec::new();
    for density in DENSITIES {
        let count = period * density;
        let amplitude = amplitude_mm * 0.001;
        let samples: Vec<_> = (0..count)
            .map(|i| {
                let (sin, cos) = (TAU * i as f64 / count as f64).sin_cos();
                law.voltage(pickup, amplitude * cos, -amplitude * TAU * frequency * sin)
            })
            .collect();
        if samples.iter().any(|v| !v.is_finite()) {
            return Err("non-finite pickup trajectory output".into());
        }
        spectra.push(coefficients(&samples, last));
    }
    let reference = spectra.last().expect("fixed nonempty densities");
    let power = band_power(reference);
    if !power.is_finite() || power <= 0.0 {
        return Err("invalid reference band energy".into());
    }
    let fundamental = reference[1].0.hypot(reference[1].1);
    let detection_floor = power.sqrt() * 1e-10;
    let harmonics = reference
        .iter()
        .enumerate()
        .skip(1)
        .take(12)
        .map(|(order, &(re, im))| {
            let amplitude = re.hypot(im);
            Harmonic {
                order,
                amplitude,
                relative_to_fundamental_db: (fundamental > detection_floor
                    && amplitude > detection_floor)
                    .then(|| 20.0 * (amplitude / fundamental).log10()),
            }
        })
        .collect();
    let errors = spectra
        .iter()
        .zip(DENSITIES)
        .take(DENSITIES.len() - 1)
        .map(|(spectrum, density)| {
            let delta: Vec<_> = spectrum
                .iter()
                .zip(reference)
                .map(|(a, b)| (a.0 - b.0, a.1 - b.1))
                .collect();
            let error = (band_power(&delta) / power).sqrt();
            SamplingError {
                internal_oversampling: density,
                ideal_band_nrmse_against_128x: error,
                error_db_above_numerical_floor: (error >= ERROR_FLOOR)
                    .then(|| 20.0 * error.log10()),
            }
        })
        .collect();
    Ok(Observation {
        law,
        amplitude_mm,
        reference_dc: reference[0].0,
        reference_ideal_band_rms: power.sqrt(),
        reference_harmonics: harmonics,
        sampling_errors: errors,
    })
}

// One coherent period, no window. Index zero is DC; subsequent entries are
// complex peak coefficients (2/N DFT). Re-anchor rotations to bound roundoff.
fn coefficients(samples: &[f64], last: usize) -> Vec<(f64, f64)> {
    let n = samples.len() as f64;
    let mut result = vec![(samples.iter().sum::<f64>() / n, 0.0)];
    for k in 1..=last {
        let step = TAU * k as f64 / n;
        let (ds, dc) = step.sin_cos();
        let (mut sin, mut cos) = (0.0, 1.0);
        let (mut re, mut im) = (0.0, 0.0);
        for (i, &sample) in samples.iter().enumerate() {
            if i.is_multiple_of(1024) {
                (sin, cos) = (step * i as f64).sin_cos();
            }
            re += sample * cos;
            im -= sample * sin;
            (sin, cos) = (sin * dc + cos * ds, cos * dc - sin * ds);
        }
        result.push((2.0 * re / n, 2.0 * im / n));
    }
    result
}

fn band_power(spectrum: &[(f64, f64)]) -> f64 {
    spectrum[0].0.powi(2)
        + 0.5
            * spectrum[1..]
                .iter()
                .map(|&(re, im)| re * re + im * im)
                .sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coherent_projection_preserves_phase_dc_and_parseval_energy() {
        let samples: Vec<_> = (0..4097)
            .map(|i| {
                let t = TAU * i as f64 / 4097.0;
                0.2 + 0.4 * t.cos() + 0.3 * (3.0 * t).sin()
            })
            .collect();
        let spectrum = coefficients(&samples, 9);
        assert!((spectrum[0].0 - 0.2).abs() < 1e-12);
        assert!((spectrum[1].0 - 0.4).abs() < 1e-12);
        assert!((spectrum[3].1 + 0.3).abs() < 1e-12);
        assert!(spectrum[2].0.hypot(spectrum[2].1) < 1e-12);
        let rms_squared = samples.iter().map(|v| v * v).sum::<f64>() / samples.len() as f64;
        assert!((band_power(&spectrum) - rms_squared).abs() < 1e-12);
    }

    #[test]
    fn centered_pickup_has_even_harmonics_and_no_fundamental_ratios() {
        let pickup = MagneticPickup::new(0.0015, 0.0).unwrap();
        for law in [Law::Production, Law::ResearchPointPole] {
            let result = observe(pickup, law, 0.75, 196.0, 32, 13).unwrap();
            assert!(result.reference_harmonics[1].amplitude > 0.1);
            for h in &result.reference_harmonics {
                assert!(h.relative_to_fundamental_db.is_none());
                if h.order % 2 == 1 {
                    assert!(h.amplitude < 1e-10);
                }
            }
        }
    }

    #[test]
    fn demanding_motion_exposes_internal_aliasing_and_refinement() {
        let pickup = MagneticPickup::new(0.0005, 0.00025).unwrap();
        for law in [Law::Production, Law::ResearchPointPole] {
            let result = observe(pickup, law, 3.0, 2756.25, 16, 6).unwrap();
            let error = &result.sampling_errors;
            assert!(error[0].ideal_band_nrmse_against_128x > 1e-6);
            assert!(
                error[1].ideal_band_nrmse_against_128x < error[0].ideal_band_nrmse_against_128x
            );
            assert!(error[4].ideal_band_nrmse_against_128x < ERROR_FLOOR);
        }
    }
}
