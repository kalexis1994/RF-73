use crate::db;
use core::f64::consts::TAU;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpectralPeak {
    pub frequency_hz: f64,
    pub amplitude_dbfs: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Partial {
    pub harmonic: u32,
    pub target_hz: f64,
    pub peak: Option<SpectralPeak>,
}

#[derive(Debug, Serialize)]
pub struct SpectralSnapshot {
    pub start_seconds: f64,
    pub observed_samples: usize,
    pub fft_size: usize,
    pub bin_spacing_hz: f64,
    pub observation_resolution_hz: f64,
    pub power_centroid_hz: Option<f64>,
    pub peaks: Vec<SpectralPeak>,
    pub harmonics: Vec<Partial>,
}

pub(crate) struct Spectrum {
    pub amplitudes: Vec<f64>,
    pub spacing: f64,
    pub resolution: f64,
    pub observed: usize,
    pub fft_size: usize,
}

impl Spectrum {
    /// Symmetric Hann, DC removal, coherent-gain amplitude correction, 2x padding.
    pub fn new(samples: &[f64], sample_rate: u32) -> Self {
        let observed = samples.len().min(32_768);
        Self::from_observation(&samples[..observed], sample_rate)
    }

    /// Tracking accepts a complete, validated observation up to 1024 ms at 192 kHz.
    /// Unlike the legacy snapshot path, this never silently truncates the window.
    pub(crate) fn for_tracking(samples: &[f64], sample_rate: u32) -> Self {
        assert!((4..=196_608).contains(&samples.len()));
        Self::from_observation(samples, sample_rate)
    }

    fn from_observation(samples: &[f64], sample_rate: u32) -> Self {
        let observed = samples.len();
        let fft_size = (observed * 2).next_power_of_two().max(8);
        let mut re = vec![0.0; fft_size];
        let mut im = vec![0.0; fft_size];
        let mean = samples.iter().sum::<f64>() / observed as f64;
        let mut sum = 0.0;
        for (i, sample) in samples.iter().enumerate() {
            let window = 0.5 - 0.5 * (TAU * i as f64 / (observed - 1) as f64).cos();
            re[i] = (sample - mean) * window;
            sum += window;
        }
        fft(&mut re, &mut im);
        let amplitudes = (0..=fft_size / 2)
            .map(|i| {
                re[i].hypot(im[i])
                    * if i == 0 || i == fft_size / 2 {
                        1.0
                    } else {
                        2.0
                    }
                    / sum
            })
            .collect();
        Self {
            amplitudes,
            spacing: sample_rate as f64 / fft_size as f64,
            resolution: sample_rate as f64 / observed as f64,
            observed,
            fft_size,
        }
    }

    pub(crate) fn peak(&self, i: usize) -> SpectralPeak {
        let a = self.amplitudes[i - 1].max(1e-150).ln();
        let b = self.amplitudes[i].max(1e-150).ln();
        let c = self.amplitudes[i + 1].max(1e-150).ln();
        let denominator = a - 2.0 * b + c;
        let delta = if denominator.abs() > 1e-15 {
            (0.5 * (a - c) / denominator).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        SpectralPeak {
            frequency_hz: (i as f64 + delta) * self.spacing,
            amplitude_dbfs: 20.0 / core::f64::consts::LN_10 * (b - 0.25 * (a - c) * delta),
        }
    }

    pub fn strongest_near(&self, frequency: f64, tolerance: f64) -> Option<SpectralPeak> {
        let maximum = self.amplitudes.iter().copied().fold(0.0, f64::max);
        if maximum < 1e-12 {
            return None;
        }
        let first = ((frequency - tolerance).max(0.0) / self.spacing).floor() as usize;
        let last = ((frequency + tolerance) / self.spacing).ceil() as usize;
        let mut candidates = (first.max(1)..=last.min(self.amplitudes.len() - 2)).filter(|&i| {
            ((i as f64 * self.spacing) - frequency).abs() <= tolerance
                && self.amplitudes[i] >= maximum * 0.001
                && self.amplitudes[i] >= self.amplitudes[i - 1]
                && self.amplitudes[i] > self.amplitudes[i + 1]
        });
        let i = candidates
            .by_ref()
            .max_by(|&a, &b| self.amplitudes[a].total_cmp(&self.amplitudes[b]))?;
        Some(self.peak(i))
    }

    pub fn snapshot(&self, start_seconds: f64, fundamental: f64) -> SpectralSnapshot {
        let maximum = self.amplitudes.iter().copied().fold(0.0, f64::max);
        let mut indices: Vec<_> = (1..self.amplitudes.len() - 1)
            .filter(|&i| {
                self.amplitudes[i] > (maximum * 0.001).max(1e-12)
                    && self.amplitudes[i] >= self.amplitudes[i - 1]
                    && self.amplitudes[i] > self.amplitudes[i + 1]
            })
            .collect();
        indices.sort_by(|&a, &b| self.amplitudes[b].total_cmp(&self.amplitudes[a]));
        let mut peaks: Vec<SpectralPeak> = Vec::new();
        for i in indices {
            let peak = self.peak(i);
            if peaks
                .iter()
                .all(|p| (p.frequency_hz - peak.frequency_hz).abs() > 2.0 * self.resolution)
            {
                peaks.push(peak);
                if peaks.len() == 16 {
                    break;
                }
            }
        }
        let mut power = 0.0;
        let mut weighted = 0.0;
        for (i, amplitude) in self.amplitudes.iter().enumerate().skip(1) {
            power += amplitude * amplitude;
            weighted += i as f64 * self.spacing * amplitude * amplitude;
        }
        let harmonics = (1..=12)
            .filter(|&n| n as f64 * fundamental < self.spacing * (self.amplitudes.len() - 1) as f64)
            .map(|n| Partial {
                harmonic: n,
                target_hz: n as f64 * fundamental,
                peak: self.strongest_near(
                    n as f64 * fundamental,
                    (2.0 * self.spacing)
                        .max(n as f64 * fundamental * 0.015)
                        .min(fundamental * 0.2),
                ),
            })
            .collect();
        SpectralSnapshot {
            start_seconds,
            observed_samples: self.observed,
            fft_size: self.fft_size,
            bin_spacing_hz: self.spacing,
            observation_resolution_hz: self.resolution,
            power_centroid_hz: (power > 1e-24).then(|| weighted / power),
            peaks,
            harmonics,
        }
    }

    pub fn amplitude_db(&self, frequency: f64) -> Option<f64> {
        let bin = frequency / self.spacing;
        let lower = bin.floor() as usize;
        let a = *self.amplitudes.get(lower)?;
        let b = *self.amplitudes.get(lower + 1)?;
        db(a + (b - a) * (bin - lower as f64))
    }
}

/// Unwindowed coherent peak coefficients: DC has 1/N scaling, others 2/N.
/// Caller must provide complete periodic motion; arbitrary captures need windowing.
pub fn coherent_coefficients(
    mut samples: Vec<f64>,
    last: usize,
) -> Result<Vec<[f64; 2]>, crate::AudioError> {
    let count = samples.len();
    if !(16..=1_048_576).contains(&count)
        || !count.is_power_of_two()
        || last >= count / 2
        || samples.iter().any(|x| !x.is_finite())
    {
        return Err(crate::AudioError("coherent spectrum needs 16..1048576 power-of-two finite samples and bins below Nyquist".into()));
    }
    let mut im = vec![0.0; count];
    fft(&mut samples, &mut im);
    Ok((0..=last)
        .map(|k| {
            let scale = if k == 0 {
                1.0 / count as f64
            } else {
                2.0 / count as f64
            };
            [samples[k] * scale, im[k] * scale]
        })
        .collect())
}

/// In-place radix-2 FFT. Its output is tested against a direct DFT.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        let (wi, wr) = (-TAU / width as f64).sin_cos();
        for start in (0..n).step_by(width) {
            let (mut r, mut v) = (1.0, 0.0);
            for offset in 0..width / 2 {
                let a = start + offset;
                let b = a + width / 2;
                let tr = r * re[b] - v * im[b];
                let ti = r * im[b] + v * re[b];
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                (r, v) = (r * wr - v * wi, r * wi + v * wr);
            }
        }
        width *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coherent_coefficients_preserve_dc_phase_and_parseval_without_a_window() {
        let samples: Vec<_> = (0..1024)
            .map(|i| {
                let t = TAU * i as f64 / 1024.0;
                0.3 + 0.4 * (17.0 * t).cos() + 0.2 * (91.0 * t).sin()
            })
            .collect();
        let actual_power = samples.iter().map(|v| v * v).sum::<f64>() / 1024.0;
        let c = coherent_coefficients(samples, 511).unwrap();
        assert!((c[0][0] - 0.3).abs() < 1e-12);
        assert!((c[17][0] - 0.4).abs() < 1e-12);
        assert!((c[91][1] + 0.2).abs() < 1e-12);
        let power = c[0][0].powi(2)
            + 0.5
                * c[1..]
                    .iter()
                    .map(|v| v[0] * v[0] + v[1] * v[1])
                    .sum::<f64>();
        assert!((power - actual_power).abs() < 1e-12);
    }
    #[test]
    fn coherent_coefficients_reject_invalid_lengths_values_and_nyquist() {
        for samples in [
            Vec::new(),
            vec![0.0; 17],
            vec![f64::NAN; 16],
            vec![0.0; 1_048_577],
        ] {
            assert!(coherent_coefficients(samples, 0).is_err());
        }
        assert!(coherent_coefficients(vec![0.0; 16], 8).is_err());
    }
    #[test]
    fn fft_matches_direct_dft() {
        let input: Vec<_> = (0..32)
            .map(|i| (i as f64 * 1.37).sin() + 0.03 * i as f64)
            .collect();
        let mut re = input.clone();
        let mut im = vec![0.0; 32];
        fft(&mut re, &mut im);
        for k in 0..32 {
            let expected_re: f64 = input
                .iter()
                .enumerate()
                .map(|(i, x)| x * (TAU * k as f64 * i as f64 / 32.0).cos())
                .sum();
            let expected_im: f64 = input
                .iter()
                .enumerate()
                .map(|(i, x)| -x * (TAU * k as f64 * i as f64 / 32.0).sin())
                .sum();
            assert!((re[k] - expected_re).abs() < 1e-11);
            assert!((im[k] - expected_im).abs() < 1e-11);
        }
    }
    #[test]
    fn hann_amplitude_and_interpolated_pitch_are_calibrated() {
        let input: Vec<_> = (0..16_384)
            .map(|i| 0.5 * (TAU * 220.37 * i as f64 / 48_000.0).sin() + 0.2)
            .collect();
        let spectrum = Spectrum::new(&input, 48_000);
        let peak = spectrum.strongest_near(220.37, 5.0).unwrap();
        assert!((peak.frequency_hz - 220.37).abs() < 0.04);
        assert!((peak.amplitude_dbfs + 6.0206).abs() < 0.06);
    }
}
