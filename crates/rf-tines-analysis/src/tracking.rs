//! Conservative, bounded offline peak tracking. A spectral track is not a mechanical mode.
use crate::{measurement::DecayEstimate, spectrum::Spectrum};
use serde::Serialize;

const MAX_PEAKS: usize = 32;
const MAX_TRACKS: usize = 2048;
const FLOOR_DB: f64 = -180.0;
const NOISE_MARGIN_DB: f64 = 18.0;

#[derive(Debug, Serialize)]
pub struct PartialObservation {
    pub frame: usize,
    pub center_seconds: f64,
    pub frequency_hz: f64,
    pub amplitude_dbfs: f64,
    /// Median spectral background, not a calibrated time-domain noise RMS.
    pub background_dbfs: f64,
    pub margin_above_background_db: f64,
    pub ambiguous_neighbor: bool,
    pub capacity_limited: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecayRejection {
    SustainBoundaryRequired,
    InsufficientPoints,
    InsufficientDuration,
    AmbiguousNeighbors,
    CapacityLimited,
    FrequencyVariation,
    NonDecaying,
    InsufficientDrop,
    PoorFit,
    InconsistentSlopes,
}

#[derive(Debug, Serialize)]
pub struct PartialDecay {
    pub estimate: Option<DecayEstimate>,
    /// Empty only when the extrapolation passes every qualification.
    pub rejection_reasons: Vec<DecayRejection>,
    pub residual_rms_db: Option<f64>,
    pub early_slope_db_per_second: Option<f64>,
    pub late_slope_db_per_second: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PartialTrack {
    pub id: usize,
    pub observations: Vec<PartialObservation>,
    pub decay: PartialDecay,
}

#[derive(Debug, Serialize)]
pub struct TrackingFrame {
    pub center_seconds: f64,
    pub background_dbfs: f64,
    pub accepted_peaks: usize,
    pub rejected_weak_peaks: usize,
    pub rejected_leakage_peaks: usize,
    pub dropped_capacity_peaks: usize,
}

#[derive(Debug, Serialize)]
pub struct PartialTracking {
    pub method: &'static str,
    pub window_seconds: f64,
    pub hop_seconds: f64,
    pub observed_samples: usize,
    pub fft_size: usize,
    pub bin_spacing_hz: f64,
    /// Reciprocal observation duration. This is not a confidence interval.
    pub observation_resolution_hz: f64,
    pub minimum_separation_hz: f64,
    pub matching_tolerance_hz: f64,
    pub noise_margin_db: f64,
    pub max_peaks_per_frame: usize,
    pub max_tracks: usize,
    pub dropped_track_observations: usize,
    pub frames: Vec<TrackingFrame>,
    pub tracks: Vec<PartialTrack>,
}

fn undecided() -> PartialDecay {
    PartialDecay {
        estimate: None,
        rejection_reasons: Vec::new(),
        residual_rms_db: None,
        early_slope_db_per_second: None,
        late_slope_db_per_second: None,
    }
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 10.0_f64.powf(FLOOR_DB / 20.0);
    }
    let middle = values.len() / 2;
    let (_, center, _) = values.select_nth_unstable_by(middle, f64::total_cmp);
    *center
}

fn level(amplitude: f64) -> f64 {
    20.0 * amplitude.max(10.0_f64.powf(FLOOR_DB / 20.0)).log10()
}

impl PartialTracking {
    pub(crate) fn new(size: usize, hop: usize, rate: u32) -> Self {
        let resolution = rate as f64 / size as f64;
        Self {
            method: "free-peaks-v1; median-background; hann-leakage-guard; contiguous-nearest",
            window_seconds: size as f64 / rate as f64,
            hop_seconds: hop as f64 / rate as f64,
            observed_samples: size,
            fft_size: (size * 2).next_power_of_two().max(8),
            bin_spacing_hz: rate as f64 / (size * 2).next_power_of_two().max(8) as f64,
            observation_resolution_hz: resolution,
            minimum_separation_hz: 2.0 * resolution,
            matching_tolerance_hz: 0.75 * resolution,
            noise_margin_db: NOISE_MARGIN_DB,
            max_peaks_per_frame: MAX_PEAKS,
            max_tracks: MAX_TRACKS,
            dropped_track_observations: 0,
            frames: Vec::new(),
            tracks: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, spectrum: &Spectrum, center: f64) {
        assert_eq!(
            spectrum.observed, self.observed_samples,
            "tracking window was truncated"
        );
        let frame = self.frames.len();
        // Exclude DC/main-lobe edges and Nyquist. The upper analysis band is 20 kHz.
        let low_hz = 20.0_f64.max(self.minimum_separation_hz);
        let high_hz = 20_000.0_f64.min(
            spectrum.spacing * (spectrum.amplitudes.len() - 1) as f64 - self.minimum_separation_hz,
        );
        let first = ((low_hz / spectrum.spacing).ceil() as usize).max(1);
        let last =
            ((high_hz / spectrum.spacing).floor() as usize).min(spectrum.amplitudes.len() - 2);
        let mut background_bins = spectrum.amplitudes[first..=last].to_vec();
        let background = median(&mut background_bins);
        let maximum = background_bins.iter().copied().fold(0.0, f64::max);
        let mut indices: Vec<_> = (first..=last)
            .filter(|&i| {
                spectrum.amplitudes[i] >= spectrum.amplitudes[i - 1]
                    && spectrum.amplitudes[i] > spectrum.amplitudes[i + 1]
            })
            .collect();
        indices.sort_by(|&a, &b| spectrum.amplitudes[b].total_cmp(&spectrum.amplitudes[a]));
        let mut summary = TrackingFrame {
            center_seconds: center,
            background_dbfs: level(background),
            accepted_peaks: 0,
            rejected_weak_peaks: 0,
            rejected_leakage_peaks: 0,
            dropped_capacity_peaks: 0,
        };
        let mut peaks: Vec<PartialObservation> = Vec::new();
        let radius = (200.0_f64.max(12.0 * spectrum.resolution) / spectrum.spacing).ceil() as usize;
        let exclusion = (2.0 * spectrum.resolution / spectrum.spacing).ceil() as usize;
        for i in indices {
            let amplitude = spectrum.amplitudes[i];
            if level(amplitude) < (level(background) + NOISE_MARGIN_DB).max(level(maximum) - 70.0) {
                summary.rejected_weak_peaks += 1;
                continue;
            }
            let mut local: Vec<_> = (i.saturating_sub(radius).max(first)
                ..=i.saturating_add(radius).min(last))
                .filter(|&j| j.abs_diff(i) > exclusion)
                .map(|j| spectrum.amplitudes[j])
                .collect();
            let floor = level(median(&mut local).max(background));
            let peak = spectrum.peak(i);
            if peak.amplitude_dbfs < floor + NOISE_MARGIN_DB {
                summary.rejected_weak_peaks += 1;
                continue;
            }
            // Conservative continuous-Hann sidelobe envelope with 6 dB margin.
            // This is a leakage heuristic, not a guarantee for arbitrary modulated signals.
            let leakage = peaks.iter().any(|stronger| {
                let distance =
                    (stronger.frequency_hz - peak.frequency_hz).abs() / spectrum.resolution;
                distance >= 2.0
                    && peak.amplitude_dbfs
                        < stronger.amplitude_dbfs
                            + 20.0
                                * (2.0
                                    / (core::f64::consts::PI
                                        * distance
                                        * (distance * distance - 1.0)))
                                    .log10()
            });
            if leakage {
                summary.rejected_leakage_peaks += 1;
                continue;
            }
            if peaks.len() == MAX_PEAKS {
                summary.dropped_capacity_peaks += 1;
                continue;
            }
            peaks.push(PartialObservation {
                frame,
                center_seconds: center,
                frequency_hz: peak.frequency_hz,
                amplitude_dbfs: peak.amplitude_dbfs,
                background_dbfs: floor,
                margin_above_background_db: peak.amplitude_dbfs - floor,
                ambiguous_neighbor: false,
                capacity_limited: false,
            });
        }
        for i in 0..peaks.len() {
            peaks[i].capacity_limited = summary.dropped_capacity_peaks > 0;
            for j in i + 1..peaks.len() {
                if (peaks[i].frequency_hz - peaks[j].frequency_hz).abs()
                    < self.minimum_separation_hz
                {
                    peaks[i].ambiguous_neighbor = true;
                    peaks[j].ambiguous_neighbor = true;
                }
            }
        }
        summary.accepted_peaks = peaks.len();
        // Only the immediately preceding frame can continue a track; never bridge missing data.
        let mut edges = Vec::new();
        for (track_index, track) in self.tracks.iter().enumerate() {
            let previous = track
                .observations
                .last()
                .expect("tracks contain observations");
            if previous.frame + 1 != frame {
                continue;
            }
            for (peak_index, peak) in peaks.iter().enumerate() {
                let distance = (previous.frequency_hz - peak.frequency_hz).abs();
                if distance <= self.matching_tolerance_hz {
                    edges.push((distance, track_index, peak_index));
                }
            }
        }
        edges.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut assigned = vec![None; peaks.len()];
        let mut used = vec![false; self.tracks.len()];
        for (_, track, peak) in edges {
            if assigned[peak].is_none() && !used[track] {
                assigned[peak] = Some(track);
                used[track] = true;
            }
        }
        for (peak, track) in peaks.into_iter().zip(assigned) {
            if let Some(track) = track {
                self.tracks[track].observations.push(peak);
            } else if self.tracks.len() < MAX_TRACKS {
                self.tracks.push(PartialTrack {
                    id: self.tracks.len(),
                    observations: vec![peak],
                    decay: undecided(),
                });
            } else {
                self.dropped_track_observations += 1;
            }
        }
        self.frames.push(summary);
    }

    pub(crate) fn finish(&mut self, onset: Option<f64>, sustain_end: Option<f64>) {
        for track in &mut self.tracks {
            track.decay = fit(
                track,
                onset,
                sustain_end,
                self.window_seconds,
                self.observation_resolution_hz,
            );
        }
    }
}

fn regression(points: &[&PartialObservation]) -> Option<(DecayEstimate, f64)> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p.center_seconds).sum::<f64>() / n;
    let my = points.iter().map(|p| p.amplitude_dbfs).sum::<f64>() / n;
    let xx = points
        .iter()
        .map(|p| (p.center_seconds - mx).powi(2))
        .sum::<f64>();
    let yy = points
        .iter()
        .map(|p| (p.amplitude_dbfs - my).powi(2))
        .sum::<f64>();
    let xy = points
        .iter()
        .map(|p| (p.center_seconds - mx) * (p.amplitude_dbfs - my))
        .sum::<f64>();
    if xx <= 1e-15 {
        return None;
    }
    let slope = xy / xx;
    let residual = (points
        .iter()
        .map(|p| (p.amplitude_dbfs - my - slope * (p.center_seconds - mx)).powi(2))
        .sum::<f64>()
        / n)
        .sqrt();
    let first = points.first()?.center_seconds;
    let last = points.last()?.center_seconds;
    Some((
        DecayEstimate {
            start_seconds: first,
            end_seconds: last,
            slope_db_per_second: slope,
            r_squared: if yy > 1e-15 {
                (xy * xy / (xx * yy)).clamp(0.0, 1.0)
            } else {
                0.0
            },
            fitted_drop_db: -slope * (last - first),
            extrapolated_t60_seconds: None,
            points: points.len(),
        },
        residual,
    ))
}

fn fit(
    track: &PartialTrack,
    onset: Option<f64>,
    end: Option<f64>,
    window: f64,
    resolution: f64,
) -> PartialDecay {
    let mut result = undecided();
    let Some(end) = end else {
        result
            .rejection_reasons
            .push(DecayRejection::SustainBoundaryRequired);
        return result;
    };
    // Whole windows must be outside the first 100 ms and before the supplied release.
    let start = onset.unwrap_or(0.0) + 0.1;
    let points: Vec<_> = track
        .observations
        .iter()
        .filter(|p| {
            p.center_seconds - window / 2.0 >= start && p.center_seconds + window / 2.0 <= end
        })
        .collect();
    if points.len() < 12 {
        result
            .rejection_reasons
            .push(DecayRejection::InsufficientPoints);
        return result;
    }
    let (mut estimate, residual) = regression(&points).expect("twelve distinct frame centers");
    let early = regression(&points[..points.len() / 2])
        .unwrap()
        .0
        .slope_db_per_second;
    let late = regression(&points[points.len() / 2..])
        .unwrap()
        .0
        .slope_db_per_second;
    let reasons = &mut result.rejection_reasons;
    if estimate.end_seconds - estimate.start_seconds < 0.35 {
        reasons.push(DecayRejection::InsufficientDuration);
    }
    if points.iter().any(|p| p.ambiguous_neighbor) {
        reasons.push(DecayRejection::AmbiguousNeighbors);
    }
    if points.iter().any(|p| p.capacity_limited) {
        reasons.push(DecayRejection::CapacityLimited);
    }
    let min_frequency = points
        .iter()
        .map(|p| p.frequency_hz)
        .fold(f64::INFINITY, f64::min);
    let max_frequency = points.iter().map(|p| p.frequency_hz).fold(0.0, f64::max);
    if max_frequency - min_frequency > resolution * 0.5 {
        reasons.push(DecayRejection::FrequencyVariation);
    }
    if estimate.slope_db_per_second >= -1e-6 {
        reasons.push(DecayRejection::NonDecaying);
    }
    if estimate.fitted_drop_db < 5.0 {
        reasons.push(DecayRejection::InsufficientDrop);
    }
    if estimate.r_squared < 0.98 || residual > 1.0 {
        reasons.push(DecayRejection::PoorFit);
    }
    if early >= 0.0
        || late >= 0.0
        || (early - late).abs() > estimate.slope_db_per_second.abs() * 0.3
    {
        reasons.push(DecayRejection::InconsistentSlopes);
    }
    if reasons.is_empty() {
        estimate.extrapolated_t60_seconds = Some(-60.0 / estimate.slope_db_per_second);
    }
    result.estimate = Some(estimate);
    result.residual_rms_db = Some(residual);
    result.early_slope_db_per_second = Some(early);
    result.late_slope_db_per_second = Some(late);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_and_lifetime_limits_report_dropped_peaks_without_growing_tracks() {
        let mut tracking = PartialTracking::new(8000, 512, 16000);
        for frame in 0..65 {
            // Synthetic post-FFT input: forty separated equal peaks, shifting beyond
            // the match gate on every frame. Exercises worst-case track churn.
            let mut spectrum = Spectrum {
                amplitudes: vec![0.0; 8001],
                spacing: 1.0,
                resolution: 2.0,
                observed: 8000,
                fft_size: 16000,
            };
            for n in 0..40 {
                spectrum.amplitudes[100 + 160 * n + 40 * (frame % 2)] = 0.1;
            }
            tracking.push(&spectrum, 0.25 + frame as f64 * 0.032);
        }
        assert!(
            tracking
                .frames
                .iter()
                .all(|f| f.accepted_peaks == MAX_PEAKS && f.dropped_capacity_peaks == 8)
        );
        assert_eq!(tracking.tracks.len(), MAX_TRACKS);
        assert_eq!(tracking.dropped_track_observations, MAX_PEAKS);
    }

    #[test]
    fn capacity_limited_frames_cannot_produce_a_qualified_decay() {
        let mut tracking = PartialTracking::new(8000, 512, 16000);
        for frame in 0..25 {
            let mut spectrum = Spectrum {
                amplitudes: vec![0.0; 8001],
                spacing: 1.0,
                resolution: 2.0,
                observed: 8000,
                fft_size: 16000,
            };
            for n in 0..40 {
                spectrum.amplitudes[100 + 160 * n] = 0.1 * 10.0_f64.powf(-frame as f64 * 0.032);
            }
            tracking.push(&spectrum, 0.25 + frame as f64 * 0.032);
        }
        tracking.finish(Some(0.0), Some(1.5));
        for track in &tracking.tracks {
            assert_eq!(
                track.decay.rejection_reasons,
                vec![DecayRejection::CapacityLimited]
            );
            assert!(
                track
                    .decay
                    .estimate
                    .as_ref()
                    .unwrap()
                    .extrapolated_t60_seconds
                    .is_none()
            );
        }
    }
}
