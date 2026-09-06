//! Explicit spectral evidence near proposed resonances; no modal identification.
use crate::{
    AudioClip, AudioError, PartialObservation,
    spectrum::{SpectralSnapshot, Spectrum},
    tracking::{PartialTracking, TrackingFrame},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ModeEvidence {
    pub proposed_frequency_hz: f64,
    pub search_half_width_hz: f64,
    pub nearest_harmonic: u32,
    pub nearest_harmonic_hz: f64,
    pub harmonic_overlap: bool,
    pub other_proposed_mode_overlap: bool,
    /// Indices into this window's independently accepted peaks, never forced fits.
    pub accepted_peak_indices: Vec<usize>,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ModeWindow {
    pub label: &'static str,
    pub start_frame: usize,
    pub minimum_separation_hz: f64,
    pub detection_band_hz: [f64; 2],
    pub capacity_limited: bool,
    pub noise_margin_db: f64,
    pub detector: TrackingFrame,
    pub accepted_peaks: Vec<PartialObservation>,
    pub spectrum: SpectralSnapshot,
    pub modes: Vec<ModeEvidence>,
}

#[derive(Debug, Serialize)]
pub struct ModalObservation {
    pub schema_version: u32,
    pub method: &'static str,
    pub sample_rate: u32,
    pub fundamental_hz: f64,
    pub windows: Vec<ModeWindow>,
}

fn evidence(
    targets: &[f64],
    fundamental: f64,
    resolution: f64,
    band: [f64; 2],
    capacity: bool,
    peaks: &[PartialObservation],
) -> Vec<ModeEvidence> {
    targets
        .iter()
        .enumerate()
        .map(|(index, &f)| {
            let harmonic = (f / fundamental).round().max(1.0) as u32;
            let harmonic_hz = harmonic as f64 * fundamental;
            let harmonic_overlap = (f - harmonic_hz).abs() < 2.0 * resolution;
            let mode_overlap = targets
                .iter()
                .enumerate()
                .any(|(j, g)| j != index && (f - g).abs() < 2.0 * resolution);
            let candidates: Vec<_> = peaks
                .iter()
                .enumerate()
                .filter(|(_, p)| (p.frequency_hz - f).abs() <= resolution)
                .map(|(i, _)| i)
                .collect();
            // A whole search interval must be observable before absence is meaningful.
            let status = if f - resolution < band[0] || f + resolution > band[1] {
                "outside_observable_band"
            } else if capacity {
                "capacity_limited"
            } else if candidates.is_empty() {
                "no_accepted_peak"
            } else if candidates.len() > 1
                || candidates
                    .iter()
                    .any(|&i| peaks[i].ambiguous_neighbor || peaks[i].capacity_limited)
            {
                "ambiguous_detection"
            } else if harmonic_overlap || mode_overlap {
                "unresolved_origin"
            } else {
                "isolated_frequency_candidate"
            };
            ModeEvidence {
                proposed_frequency_hz: f,
                search_half_width_hz: resolution,
                nearest_harmonic: harmonic,
                nearest_harmonic_hz: harmonic_hz,
                harmonic_overlap,
                other_proposed_mode_overlap: mode_overlap,
                accepted_peak_indices: candidates,
                status,
            }
        })
        .collect()
}

/// Observe full 32/128 ms attack windows and a 512 ms window starting at 250 ms.
/// All starts are explicit file offsets. Each clip keeps its native sample rate.
/// No onset shift, resampling, amplitude normalization or decay fitting occurs.
pub fn observe_modes(
    clip: &AudioClip,
    targets: &[f64],
    fundamental_hz: f64,
) -> Result<ModalObservation, AudioError> {
    if targets.is_empty()
        || targets.len() > 16
        || !fundamental_hz.is_finite()
        || !(20.0..=5000.0).contains(&fundamental_hz)
        || targets
            .iter()
            .any(|f| !f.is_finite() || !(20.0..=20000.0).contains(f))
        || targets.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(AudioError("need 1..16 finite, strictly increasing proposed modes in 20..20000 Hz and a fundamental in 20..5000 Hz".into()));
    }
    let rate = clip.metadata().sample_rate;
    let mut windows = Vec::new();
    for (label, start_seconds, seconds) in [
        ("attack_32_ms", 0.0, 0.032),
        ("attack_128_ms", 0.0, 0.128),
        ("body_512_ms", 0.25, 0.512),
    ] {
        let start = (start_seconds * rate as f64).round() as usize;
        let size = (seconds * rate as f64).round() as usize;
        let samples = clip.samples().get(start..start + size).ok_or_else(|| {
            AudioError("audio lacks a complete requested modal observation window".into())
        })?;
        let spectrum = Spectrum::for_tracking(samples, rate);
        let mut tracking = PartialTracking::new(size, size, rate);
        tracking.push(&spectrum, (start as f64 + size as f64 / 2.0) / rate as f64);
        let capacity = tracking.frames[0].dropped_capacity_peaks > 0
            || tracking.dropped_track_observations > 0;
        let peaks: Vec<_> = tracking
            .tracks
            .into_iter()
            .flat_map(|t| t.observations)
            .collect();
        let band = [
            20.0_f64.max(tracking.minimum_separation_hz),
            20000.0_f64.min(rate as f64 / 2.0 - tracking.minimum_separation_hz),
        ];
        let modes = evidence(
            targets,
            fundamental_hz,
            spectrum.resolution,
            band,
            capacity,
            &peaks,
        );
        windows.push(ModeWindow {
            label,
            start_frame: start,
            minimum_separation_hz: tracking.minimum_separation_hz,
            detection_band_hz: band,
            capacity_limited: capacity,
            noise_margin_db: tracking.noise_margin_db,
            detector: tracking.frames.remove(0),
            accepted_peaks: peaks,
            spectrum: spectrum.snapshot(start as f64 / rate as f64, fundamental_hz),
            modes,
        });
    }
    Ok(ModalObservation {
        schema_version: 1,
        method: "modal-evidence-v1; independent-background-qualified-peaks; explicit-full-windows; proximity-is-not-identity",
        sample_rate: rate,
        fundamental_hz,
        windows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn peak(f: f64) -> PartialObservation {
        PartialObservation {
            frame: 0,
            center_seconds: 0.0,
            frequency_hz: f,
            amplitude_dbfs: -20.0,
            background_dbfs: -100.0,
            margin_above_background_db: 80.0,
            ambiguous_neighbor: false,
            capacity_limited: false,
        }
    }
    #[test]
    fn proximity_never_identifies_a_mode_and_absence_keeps_observability_limits() {
        let targets = [29.0, 200.0, 1001.0, 1353.0, 18000.0];
        let peaks = [peak(200.0), peak(1000.0), peak(1353.0)];
        let e = evidence(&targets, 200.0, 7.8125, [20.0, 16000.0], false, &peaks);
        assert_eq!(e[0].status, "no_accepted_peak");
        assert_eq!(e[1].status, "unresolved_origin");
        assert_eq!(e[2].status, "unresolved_origin");
        assert_eq!(e[3].status, "isolated_frequency_candidate");
        assert_eq!(e[4].status, "outside_observable_band");
        let close = evidence(
            &[1353.0, 1354.0],
            200.0,
            7.8125,
            [20.0, 16000.0],
            false,
            &peaks,
        );
        assert_eq!(close[0].status, "unresolved_origin");
        let capped = evidence(&targets, 200.0, 7.8125, [20.0, 16000.0], true, &[]);
        assert_eq!(capped[3].status, "capacity_limited");
        let multiple = evidence(
            &[1353.0],
            200.0,
            7.8125,
            [20.0, 16000.0],
            false,
            &[peak(1352.0), peak(1354.0)],
        );
        assert_eq!(multiple[0].status, "ambiguous_detection");
    }
    #[test]
    fn equal_physical_windows_keep_native_rates_and_detect_nonharmonic_content() {
        for rate in [44100, 48000, 96000] {
            let samples: Vec<_> = (0..rate)
                .map(|i| {
                    let t = i as f64 / rate as f64;
                    0.2 * (std::f64::consts::TAU * 200.0 * t).sin()
                        + 0.04 * (std::f64::consts::TAU * 1353.0 * t).sin()
                })
                .collect();
            let clip = AudioClip::from_samples(rate, samples.clone()).unwrap();
            let a = observe_modes(&clip, &[200.0, 1353.0, 3000.0], 200.0).unwrap();
            assert_eq!(a.sample_rate, rate);
            assert_eq!(
                a.windows[2].start_frame,
                (rate as f64 * 0.25).round() as usize
            );
            assert_eq!(
                a.windows[2].spectrum.observed_samples,
                (rate as f64 * 0.512).round() as usize
            );
            assert_eq!(a.windows[2].modes[1].status, "isolated_frequency_candidate");
            assert_eq!(a.windows[2].modes[2].status, "no_accepted_peak");
            let p = &a.windows[2].accepted_peaks[a.windows[2].modes[1].accepted_peak_indices[0]];
            assert!((p.frequency_hz - 1353.0).abs() < 0.1);
            let quieter =
                AudioClip::from_samples(rate, samples.iter().map(|s| s * 0.1).collect()).unwrap();
            let b = observe_modes(&quieter, &[200.0, 1353.0, 3000.0], 200.0).unwrap();
            let q = &b.windows[2].accepted_peaks[b.windows[2].modes[1].accepted_peak_indices[0]];
            assert!((p.frequency_hz - q.frequency_hz).abs() < 1e-6);
            assert!((q.amplitude_dbfs - p.amplitude_dbfs + 20.0).abs() < 1e-6);
        }
    }
    #[test]
    fn incomplete_windows_invalid_proposals_and_silence_do_not_become_evidence() {
        let short = AudioClip::from_samples(48000, vec![0.0; 24000]).unwrap();
        assert!(observe_modes(&short, &[200.0], 200.0).is_err());
        let silent = AudioClip::from_samples(48000, vec![0.0; 48000]).unwrap();
        for targets in [
            vec![],
            vec![200.0, 200.0],
            vec![400.0, 200.0],
            vec![f64::NAN],
            vec![200.0; 17],
        ] {
            assert!(observe_modes(&silent, &targets, 200.0).is_err());
        }
        assert!(observe_modes(&silent, &[200.0], f64::INFINITY).is_err());
        let r = observe_modes(&silent, &[200.0, 1353.0], 200.0).unwrap();
        assert!(r.windows.iter().all(|w| w.accepted_peaks.is_empty()
            && w.modes.iter().all(|m| m.status == "no_accepted_peak")));
    }
}
