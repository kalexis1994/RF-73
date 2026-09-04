//! Offline measurements. No dependency from the realtime DSP to this crate.
mod audio;
mod compare;
mod measurement;
mod spectrum;

pub use audio::{AudioClip, AudioError, AudioMetadata};
pub use compare::{Comparison, compare};
pub use measurement::{Analysis, AnalysisOptions, analyze};

pub(crate) fn db(amplitude: f64) -> Option<f64> {
    (amplitude > 0.0).then(|| 20.0 * amplitude.log10())
}

pub(crate) fn rms(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|x| x * x).sum::<f64>() / samples.len() as f64).sqrt()
}
