//! Offline measurements. No dependency from the realtime DSP to this crate.
mod audio;
mod compare;
mod component_envelope;
mod measurement;
mod modal_observation;
mod partial_comparison;
mod pitch_anchor;
mod short_envelope;
mod spectrum;
mod tone_comparison;
mod tracking;

pub use audio::{AudioClip, AudioError, AudioMetadata};
pub use compare::{Comparison, compare};
pub use component_envelope::{
    ComponentEnvelope, ComponentEnvelopeFit, ComponentEnvelopePoint, EnvelopeOptions,
    EnvelopeRejection, measure_component_envelope,
};
pub use measurement::{Analysis, AnalysisOptions, analyze};
pub use modal_observation::{ModalObservation, ModeEvidence, ModeWindow, observe_modes};
pub use partial_comparison::{
    MatchedPartial, PairedDecayStatus, PartialComparison, PartialComparisonOptions,
    compare_partials,
};
pub use pitch_anchor::{PitchAnchor, PitchWindow, pitch_anchor};
pub use short_envelope::{
    ShortEnvelope, ShortEnvelopeFit, ShortEnvelopeOptions, ShortEnvelopePoint,
    measure_short_envelope,
};
pub use spectrum::coherent_coefficients;
pub use tone_comparison::{
    HarmonicBalance, ToneComparison, ToneComparisonOptions, ToneWindow, compare_tone,
};
pub use tracking::{
    DecayRejection, PartialDecay, PartialObservation, PartialTrack, PartialTracking,
};

pub(crate) fn db(amplitude: f64) -> Option<f64> {
    (amplitude > 0.0).then(|| 20.0 * amplitude.log10())
}

pub(crate) fn rms(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|x| x * x).sum::<f64>() / samples.len() as f64).sqrt()
}
