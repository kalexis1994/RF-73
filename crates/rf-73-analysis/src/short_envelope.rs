//! Joint local-polynomial carrier regression for explicitly declared short mixtures.
use crate::{AudioClip, AudioError};
use serde::Serialize;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Serialize)]
pub struct ShortEnvelopeOptions {
    /// First frequency is the target; remaining frequencies are nuisance carriers.
    pub frequencies_hz: Vec<f64>,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub window_seconds: f64,
    pub hop_seconds: f64,
}
#[derive(Serialize)]
pub struct ShortEnvelopePoint {
    pub center_seconds: f64,
    pub coefficient: [f64; 2],
    pub amplitude_dbfs: f64,
    pub unwrapped_phase_radians: f64,
    pub residual_rms: f64,
    /// Residual variance propagated through the target rows of R^-1.
    /// Not a confidence interval for unknown/colored interference.
    pub regression_margin_db: f64,
}
#[derive(Serialize)]
pub struct ShortEnvelopeFit {
    pub amplitude_decay_per_second: f64,
    pub carrier_offset_hz: f64,
    pub amplitude_residual_rms_db: f64,
    pub phase_residual_rms_radians: f64,
    pub early_decay_per_second: f64,
    pub late_decay_per_second: f64,
    pub fitted_drop_db: f64,
}
#[derive(Serialize)]
pub struct ShortEnvelope {
    pub schema_version: u32,
    pub method: &'static str,
    pub options: ShortEnvelopeOptions,
    pub actual_window_seconds: f64,
    pub actual_hop_seconds: f64,
    /// Smallest QR residual-column norm / original-column norm; not cond(A).
    pub minimum_relative_qr_pivot: f64,
    pub points: Vec<ShortEnvelopePoint>,
    pub provisional_fit: Option<ShortEnvelopeFit>,
    pub qualified: bool,
    pub rejection_reasons: Vec<&'static str>,
    pub scope: &'static str,
}
struct Qr {
    q: Vec<Vec<f64>>,
    r: Vec<Vec<f64>>,
    target_noise_scale: f64,
    minimum_pivot: f64,
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn backsolve(r: &[Vec<f64>], z: &[f64]) -> Vec<f64> {
    let mut x = z.to_vec();
    for i in (0..r.len()).rev() {
        x[i] = (z[i] - ((i + 1)..r.len()).map(|j| r[i][j] * x[j]).sum::<f64>()) / r[i][i];
    }
    x
}
fn factor(columns: Vec<Vec<f64>>) -> Result<Qr, f64> {
    let count = columns.len();
    let mut q: Vec<Vec<f64>> = Vec::new();
    let mut r = vec![vec![0.0; count]; count];
    let mut minimum: f64 = 1.0;
    for (j, mut column) in columns.into_iter().enumerate() {
        let original_norm = dot(&column, &column).sqrt();
        // Modified Gram-Schmidt with a second orthogonalization pass. No normal equations.
        for _ in 0..2 {
            for (i, basis) in q.iter().enumerate() {
                let projection = dot(basis, &column);
                r[i][j] += projection;
                for (v, b) in column.iter_mut().zip(basis) {
                    *v -= projection * b;
                }
            }
        }
        r[j][j] = dot(&column, &column).sqrt();
        let pivot = r[j][j] / original_norm;
        if !pivot.is_finite() || pivot < 1e-5 {
            return Err(if pivot.is_finite() { pivot } else { 0.0 });
        }
        minimum = minimum.min(pivot);
        for v in &mut column {
            *v /= r[j][j];
        }
        q.push(column);
    }
    // Squared row norms of R^-1 for target cosine/sine center coefficients.
    let mut noise_variance = 0.0;
    for j in 0..count {
        let mut unit = vec![0.0; count];
        unit[j] = 1.0;
        let inverse_column = backsolve(&r, &unit);
        noise_variance += inverse_column[1].powi(2) + inverse_column[2].powi(2);
    }
    Ok(Qr {
        q,
        r,
        minimum_pivot: minimum,
        target_noise_scale: noise_variance.sqrt(),
    })
}
fn design(frequencies: &[f64], size: usize, rate: f64) -> Vec<Vec<f64>> {
    let mut columns = vec![vec![1.0; size]];
    for frequency in frequencies {
        let mut carrier: Vec<_> = (0..6).map(|_| Vec::with_capacity(size)).collect();
        for i in 0..size {
            let u = 2.0 * i as f64 / (size - 1) as f64 - 1.0;
            let t = (i as f64 - (size - 1) as f64 * 0.5) / rate;
            let (s, c) = (TAU * frequency * t).sin_cos();
            for (j, value) in [c, s, u * c, u * s, u * u * c, u * u * s]
                .into_iter()
                .enumerate()
            {
                carrier[j].push(value);
            }
        }
        columns.extend(carrier);
    }
    columns
}
fn line(t: &[f64], y: &[f64]) -> (f64, f64) {
    let mt = t.iter().sum::<f64>() / t.len() as f64;
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let slope = t
        .iter()
        .zip(y)
        .map(|(t, y)| (t - mt) * (y - my))
        .sum::<f64>()
        / t.iter().map(|t| (t - mt).powi(2)).sum::<f64>();
    let rms = (t
        .iter()
        .zip(y)
        .map(|(t, y)| (y - my - slope * (t - mt)).powi(2))
        .sum::<f64>()
        / t.len() as f64)
        .sqrt();
    (slope, rms)
}
pub fn measure_short_envelope(
    clip: &AudioClip,
    options: ShortEnvelopeOptions,
) -> Result<ShortEnvelope, AudioError> {
    let rate = clip.metadata().sample_rate as f64;
    if [
        options.start_seconds,
        options.end_seconds,
        options.window_seconds,
        options.hop_seconds,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || options.start_seconds < 0.0
        || options.end_seconds > clip.duration()
        || options.end_seconds <= options.start_seconds
        || options.end_seconds - options.start_seconds > 0.3
        || !(0.016..=0.064).contains(&options.window_seconds)
        || options.hop_seconds < options.window_seconds / 8.0
        || options.hop_seconds > options.window_seconds / 2.0
        || !(1..=3).contains(&options.frequencies_hz.len())
        || options.frequencies_hz.iter().enumerate().any(|(i, f)| {
            !f.is_finite()
                || *f <= 4.0 / options.window_seconds
                || *f >= 0.45 * rate
                || options.frequencies_hz[..i].contains(f)
        })
    {
        return Err(AudioError(
            "invalid short-envelope interval, carriers, window or hop".into(),
        ));
    }
    let size = (rate * options.window_seconds).round() as usize;
    let hop = (rate * options.hop_seconds).round() as usize;
    let start = (rate * options.start_seconds).ceil() as usize;
    let end = (rate * options.end_seconds).floor() as usize;
    if end.saturating_sub(start) < size {
        return Err(AudioError(
            "short envelope requires a complete window".into(),
        ));
    }
    let frames = (end - start - size) / hop + 1;
    if frames > 64 || frames * size > 1_000_000 {
        return Err(AudioError(
            "short-envelope work exceeds 64 windows / 1 million observations".into(),
        ));
    }
    let mut report = ShortEnvelope {
        schema_version: 1,
        method: "joint-quadratic-carrier-envelope-v1",
        options,
        actual_window_seconds: size as f64 / rate,
        actual_hop_seconds: hop as f64 / rate,
        minimum_relative_qr_pivot: 0.0,
        points: Vec::new(),
        provisional_fit: None,
        qualified: false,
        rejection_reasons: Vec::new(),
        scope: "Conditional local polynomial regression around declared carriers, not an exact exponential model or mode identification. Quadratic complex envelopes plus common DC; unweighted reorthogonalized QR. Relative pivot is a diagnostic, not a condition-number bound. Residual-derived margin assumes the fitted nuisance model; colored noise and undeclared mixtures can evade it. Correlated overlapping windows, no confidence interval or T60. No natural-sustain or physical-loss claim.",
    };
    let qr = match factor(design(&report.options.frequencies_hz, size, rate)) {
        Ok(qr) => qr,
        Err(pivot) => {
            report.minimum_relative_qr_pivot = pivot;
            report.rejection_reasons.push("ill_conditioned_carriers");
            return Ok(report);
        }
    };
    report.minimum_relative_qr_pivot = qr.minimum_pivot;
    if clip.samples()[start..end].iter().any(|x| x.abs() >= 1.0) {
        report.rejection_reasons.push("full_scale_samples");
    }
    for frame in 0..frames {
        let offset = start + frame * hop;
        let y = &clip.samples()[offset..offset + size];
        let z: Vec<_> = qr.q.iter().map(|q| dot(q, y)).collect();
        let coefficients = backsolve(&qr.r, &z);
        let mut residual = y.to_vec();
        for (q, z) in qr.q.iter().zip(&z) {
            for (r, q) in residual.iter_mut().zip(q) {
                *r -= z * q;
            }
        }
        let squares = dot(&residual, &residual);
        let sigma = (squares / (size - qr.q.len()) as f64).sqrt();
        let amplitude = coefficients[1].hypot(coefficients[2]).max(1e-12);
        let center = (offset as f64 + (size - 1) as f64 * 0.5) / rate;
        let (s, c) = (TAU * report.options.frequencies_hz[0] * center).sin_cos();
        let coefficient = [
            coefficients[1] * c - coefficients[2] * s,
            -coefficients[2] * c - coefficients[1] * s,
        ];
        let mut phase = coefficient[1].atan2(coefficient[0]);
        if let Some(last) = report.points.last() {
            phase = last.unwrapped_phase_radians
                + (phase - last.unwrapped_phase_radians + PI).rem_euclid(TAU)
                - PI;
        }
        report.points.push(ShortEnvelopePoint {
            center_seconds: center,
            coefficient,
            amplitude_dbfs: 20.0 * amplitude.log10(),
            unwrapped_phase_radians: phase,
            residual_rms: (squares / size as f64).sqrt(),
            regression_margin_db: 20.0
                * (amplitude / (sigma * qr.target_noise_scale).max(1e-12)).log10(),
        });
    }
    if report
        .points
        .iter()
        .any(|p| p.regression_margin_db < 18.0 || p.amplitude_dbfs < -180.0)
    {
        report.rejection_reasons.push("low_regression_margin");
    }
    let times: Vec<_> = report.points.iter().map(|p| p.center_seconds).collect();
    let span = times[times.len() - 1] - times[0];
    if frames < 6 || span < 0.06 {
        report.rejection_reasons.push("insufficient_support");
        return Ok(report);
    }
    let levels: Vec<_> = report.points.iter().map(|p| p.amplitude_dbfs).collect();
    let phases: Vec<_> = report
        .points
        .iter()
        .map(|p| p.unwrapped_phase_radians)
        .collect();
    let (slope, residual) = line(&times, &levels);
    let (phase_slope, phase_residual) = line(&times, &phases);
    let scale = -std::f64::consts::LN_10 / 20.0;
    let decay = slope * scale;
    let half = frames / 2;
    let early = line(&times[..half], &levels[..half]).0 * scale;
    let late = line(&times[half..], &levels[half..]).0 * scale;
    if -slope * span < 3.0 {
        report.rejection_reasons.push("insufficient_decay");
    }
    if residual > 0.5 {
        report.rejection_reasons.push("non_exponential_amplitude");
    }
    if (early - late).abs() > 0.5_f64.max(0.15 * decay.abs()) {
        report.rejection_reasons.push("inconsistent_slopes");
    }
    if (phase_slope / TAU).abs() > 3.0 {
        report
            .rejection_reasons
            .push("carrier_offset_outside_validated_range");
    }
    if phase_residual > 0.05 {
        report.rejection_reasons.push("unstable_phase");
    }
    report.provisional_fit = Some(ShortEnvelopeFit {
        amplitude_decay_per_second: decay,
        carrier_offset_hz: phase_slope / TAU,
        amplitude_residual_rms_db: residual,
        phase_residual_rms_radians: phase_residual,
        early_decay_per_second: early,
        late_decay_per_second: late,
        fitted_drop_db: -slope * span,
    });
    report.qualified = report.rejection_reasons.is_empty();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> ShortEnvelopeOptions {
        ShortEnvelopeOptions {
            frequencies_hz: vec![1620.0, 1568.0],
            start_seconds: 0.02,
            end_seconds: 0.18,
            window_seconds: 0.032,
            hop_seconds: 0.008,
        }
    }
    fn signal(rate: u32, shift: f64) -> AudioClip {
        AudioClip::from_samples(
            rate,
            (0..rate / 4)
                .map(|i| {
                    let t = i as f64 / rate as f64;
                    0.03 * (-8.0 * t).exp() * (TAU * (1620.0 + shift) * t + 0.73).cos()
                        + 0.3 * (-t).exp() * (TAU * 1568.0 * t - 0.4).cos()
                        + 0.01
                })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn qr_recovers_independent_polynomial_coefficients_and_residual() {
        let columns = design(&[1620.0, 1568.0, 1425.0], 1536, 48000.0);
        let expected: Vec<_> = (0..columns.len())
            .map(|i| 0.03 * (i as f64 + 0.7).sin())
            .collect();
        let mut y = vec![0.0; 1536];
        for (column, coefficient) in columns.iter().zip(&expected) {
            for (y, x) in y.iter_mut().zip(column) {
                *y += x * coefficient;
            }
        }
        let qr = factor(columns).ok().unwrap();
        let z: Vec<_> = qr.q.iter().map(|q| dot(q, &y)).collect();
        for (actual, expected) in backsolve(&qr.r, &z).iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-10);
        }
        for (i, a) in qr.q.iter().enumerate() {
            for (j, b) in qr.q.iter().enumerate() {
                assert!((dot(a, b) - if i == j { 1.0 } else { 0.0 }).abs() < 1e-12);
            }
        }
        // Independent impulse-response norm of the target estimator verifies
        // the residual-variance propagation without using inverse-R entries.
        let gain: f64 = (0..1536)
            .map(|sample| {
                let impulse_projection: Vec<_> = qr.q.iter().map(|q| q[sample]).collect();
                let fitted = backsolve(&qr.r, &impulse_projection);
                fitted[1].powi(2) + fitted[2].powi(2)
            })
            .sum();
        assert!((qr.target_noise_scale.powi(2) / gain - 1.0).abs() < 1e-10);
    }
    #[test]
    fn short_mixture_recovers_free_decay_offset_and_phase_at_native_rates() {
        for rate in [44100, 48000, 96000] {
            for shift in [0.0, 2.0] {
                let report = measure_short_envelope(&signal(rate, shift), options()).unwrap();
                assert!(report.qualified, "{:?}", report.rejection_reasons);
                let fit = report.provisional_fit.unwrap();
                assert!(
                    (fit.amplitude_decay_per_second - 8.0).abs() < 0.1,
                    "{}",
                    fit.amplitude_decay_per_second
                );
                assert!((fit.carrier_offset_hz - shift).abs() < 0.02);
                if shift == 0.0 {
                    assert!((report.points[0].unwrapped_phase_radians - 0.73).abs() < 0.01);
                }
            }
        }
    }
    #[test]
    fn near_coincident_carriers_withhold_fit_instead_of_large_coefficients() {
        let mut o = options();
        o.frequencies_hz[1] = 1620.5;
        let r = measure_short_envelope(&signal(48000, 0.0), o).unwrap();
        assert!(r.rejection_reasons.contains(&"ill_conditioned_carriers"));
        assert!(r.provisional_fit.is_none());
        assert!(r.points.is_empty());
    }
    #[test]
    fn rejects_invalid_inputs_short_support_and_silence() {
        let clip = signal(48000, 0.0);
        for field in 0..5 {
            let mut o = options();
            match field {
                0 => o.window_seconds = f64::NAN,
                1 => o.end_seconds = 0.5,
                2 => o.hop_seconds = 0.0001,
                3 => o.frequencies_hz = vec![1620.0, 1620.0],
                _ => o.frequencies_hz = vec![20.0],
            };
            assert!(measure_short_envelope(&clip, o).is_err());
        }
        let mut o = options();
        o.end_seconds = 0.07;
        let r = measure_short_envelope(&clip, o).unwrap();
        assert!(r.rejection_reasons.contains(&"insufficient_support"));
        let silent = AudioClip::from_samples(48000, vec![0.0; 12000]).unwrap();
        let r = measure_short_envelope(&silent, options()).unwrap();
        assert!(r.rejection_reasons.contains(&"low_regression_margin"));
    }
}
