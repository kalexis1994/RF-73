//! Gain-independent, pitch-aware spectral and level trajectories for G3 comparisons.
use crate::{AudioClip, AudioError, spectrum::Spectrum};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimbreWindow {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub rms: f64,
    pub level_relative_to_body_db: Option<f64>,
    pub band_power_fractions: [f64; 4],
    pub band_db_relative_to_fundamental_band: [Option<f64>; 3],
}
#[derive(Debug, Serialize)]
pub struct TimbreProfile {
    pub qualified: bool,
    pub onset_seconds: f64,
    pub fundamental_hz: f64,
    pub sample_rate: u32,
    pub band_edges_hz: [f64; 5],
    pub pre_onset_peak: f64,
    pub windows: Vec<TimbreWindow>,
    pub rejection_reasons: Vec<&'static str>,
}

/// First run of four complete 1 ms RMS bins above -40 dB relative to the
/// largest bin in the first 250 ms. This is an operational onset, not contact time.
pub fn detect_timbre_onset(clip: &AudioClip) -> Result<f64, AudioError> {
    let rate = f64::from(clip.metadata().sample_rate);
    let mut bins = Vec::new();
    for i in 0..250 {
        let a = (f64::from(i) * rate / 1000.0).round() as usize;
        let b = (f64::from(i + 1) * rate / 1000.0).round() as usize;
        let samples = clip
            .samples()
            .get(a..b)
            .ok_or_else(|| AudioError("onset requires 250 ms of real audio".into()))?;
        bins.push(crate::rms(samples));
    }
    let peak = bins.iter().copied().fold(0.0_f64, f64::max);
    if peak <= 1e-12 {
        return Err(AudioError("no measurable onset".into()));
    }
    let index = bins
        .windows(4)
        .position(|w| w.iter().all(|v| *v > peak * 0.01))
        .ok_or_else(|| AudioError("no sustained onset".into()))?;
    Ok(index as f64 / 1000.0)
}

/// Raw levels plus four disjoint Hann spectral-power fractions. Frequency
/// edges follow the independently observed fundamental, not an equal-tempered bin.
/// No resampling, time warping, per-window gain fit or fitted decay constant.
pub fn measure_timbre_profile(
    clip: &AudioClip,
    onset: f64,
    fundamental: f64,
) -> Result<TimbreProfile, AudioError> {
    let rate = f64::from(clip.metadata().sample_rate);
    if !onset.is_finite()
        || !(0.0..=0.25).contains(&onset)
        || !fundamental.is_finite()
        || !(150.0..=250.0).contains(&fundamental)
        || rate < 32000.0
    {
        return Err(AudioError(
            "timbre profile requires G3 range, rate >=32 kHz and onset 0..250 ms".into(),
        ));
    }
    let edges = [
        0.5 * fundamental,
        1.5 * fundamental,
        4.0 * fundamental,
        12.0 * fundamental,
        8000.0,
    ];
    let mut windows = Vec::new();
    let mut reasons = Vec::new();
    for (lo, hi) in [
        (0.0, 0.064),
        (0.064, 0.192),
        (0.256, 0.512),
        (0.64, 1.152),
        (1.152, 1.664),
    ] {
        let start = ((onset + lo) * rate).round() as usize;
        let end = ((onset + hi) * rate).round() as usize;
        let samples = clip.samples().get(start..end).ok_or_else(|| {
            AudioError("timbre profile requires all five complete windows".into())
        })?;
        let rms = crate::rms(samples);
        let spectrum = Spectrum::for_tracking(samples, clip.metadata().sample_rate);
        let mut power = [0.0; 4];
        for (i, a) in spectrum.amplitudes.iter().enumerate() {
            let f = i as f64 * spectrum.spacing;
            if let Some(band) = (0..4).find(|&b| f >= edges[b] && f < edges[b + 1]) {
                power[band] += a * a;
            }
        }
        let total = power.iter().sum::<f64>();
        if rms <= 1e-12 || total <= 1e-24 {
            reasons.push("unmeasurable_window");
        }
        let fractions = power.map(|p| p / total.max(1e-300));
        let relative = core::array::from_fn(|b| {
            (fractions[0] > 1e-8 && fractions[b + 1] > 1e-8)
                .then(|| 10.0 * (power[b + 1] / power[0]).log10())
        });
        windows.push(TimbreWindow {
            start_seconds: start as f64 / rate,
            end_seconds: end as f64 / rate,
            rms,
            level_relative_to_body_db: None,
            band_power_fractions: fractions,
            band_db_relative_to_fundamental_band: relative,
        });
    }
    let body = windows[2].rms;
    for w in &mut windows {
        w.level_relative_to_body_db =
            (body > 1e-12 && w.rms > 1e-12).then(|| 20.0 * (w.rms / body).log10());
    }
    let pre = (onset * rate).round() as usize;
    Ok(TimbreProfile {
        qualified: reasons.is_empty(),
        onset_seconds: onset,
        fundamental_hz: fundamental,
        sample_rate: clip.metadata().sample_rate,
        band_edges_hz: edges,
        pre_onset_peak: clip.samples()[..pre]
            .iter()
            .map(|x| x.abs())
            .fold(0.0_f64, f64::max),
        windows,
        rejection_reasons: reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signal(rate: u32, gain: f64) -> AudioClip {
        AudioClip::from_samples(
            rate,
            (0..2 * rate)
                .map(|i| {
                    let t = f64::from(i) / f64::from(rate) - 0.05;
                    if t < 0.0 {
                        0.0
                    } else {
                        gain * (-t).exp()
                            * ((core::f64::consts::TAU * 196.0 * t).sin()
                                + 0.3 * (core::f64::consts::TAU * 196.0 * 7.1 * t).sin())
                    }
                })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn gain_invariance_native_rates_and_known_decay() {
        let a = signal(48000, 0.2);
        let b = signal(44100, 0.002);
        let oa = detect_timbre_onset(&a).unwrap();
        let ob = detect_timbre_onset(&b).unwrap();
        assert!((oa - 0.05).abs() <= 0.001 && (ob - 0.05).abs() <= 0.001);
        let a = measure_timbre_profile(&a, oa, 196.0).unwrap();
        let b = measure_timbre_profile(&b, ob, 196.0).unwrap();
        assert!(a.qualified && b.qualified);
        for (x, y) in a.windows.iter().zip(&b.windows) {
            assert!(
                (x.level_relative_to_body_db.unwrap() - y.level_relative_to_body_db.unwrap()).abs()
                    < 0.02
            );
            for i in 0..4 {
                assert!((x.band_power_fractions[i] - y.band_power_fractions[i]).abs() < 0.001);
            }
        }
        let drop = a.windows[4].level_relative_to_body_db.unwrap()
            - a.windows[3].level_relative_to_body_db.unwrap();
        assert!((drop + 20.0 / core::f64::consts::LN_10 * 0.512).abs() < 0.02);
        assert!(
            (a.windows[2].band_db_relative_to_fundamental_band[1].unwrap()
                - 20.0 * 0.3_f64.log10())
            .abs()
                < 0.02
        );
    }
    #[test]
    fn silence_invalid_support_and_spectral_changes_remain_explicit() {
        let silent = AudioClip::from_samples(48000, vec![0.0; 96000]).unwrap();
        assert!(detect_timbre_onset(&silent).is_err());
        assert!(
            !measure_timbre_profile(&silent, 0.0, 196.0)
                .unwrap()
                .qualified
        );
        assert!(measure_timbre_profile(&silent, f64::NAN, 196.0).is_err());
        assert!(measure_timbre_profile(&silent, 0.0, 0.0).is_err());
        let short = AudioClip::from_samples(48000, vec![0.0; 4800]).unwrap();
        assert!(measure_timbre_profile(&short, 0.0, 196.0).is_err());
        let tone = AudioClip::from_samples(
            48000,
            (0..96000)
                .map(|i| 0.1 * (core::f64::consts::TAU * 196.0 * f64::from(i) / 48000.0).sin())
                .collect(),
        )
        .unwrap();
        let p = measure_timbre_profile(&tone, 0.0, 196.0).unwrap();
        assert!(p.windows[2].band_power_fractions[0] > 0.999);
        assert!(p.windows[2].band_db_relative_to_fundamental_band[1].is_none());
    }
}
