//! Pair detected spectral components over explicitly selected, equal-duration regions.
use crate::{
    AudioClip, AudioError, PartialObservation, PartialTracking, db, rms, spectrum::Spectrum,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub struct PartialComparisonOptions {
    pub reference_start_seconds: f64,
    pub candidate_start_seconds: f64,
    /// Both regions must be uninterrupted sustains of the same note/gesture phase.
    pub seconds: f64,
    pub partial_window_ms: u32,
    pub match_cents: f64,
}

impl Default for PartialComparisonOptions {
    fn default() -> Self {
        Self {
            reference_start_seconds: 0.0,
            candidate_start_seconds: 0.0,
            seconds: 1.0,
            partial_window_ms: 128,
            match_cents: 50.0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComparisonRegion {
    pub start_frame: usize,
    pub end_frame_exclusive: usize,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairedDecayStatus {
    Qualified,
    UnqualifiedTrack,
    DifferentFitIntervals,
    IncompletePairing,
}

#[derive(Debug, Serialize)]
pub struct PairedDecay {
    pub status: PairedDecayStatus,
    pub candidate_minus_reference_slope_db_per_second: Option<f64>,
    pub candidate_minus_reference_t60_seconds: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MatchedPartial {
    pub reference_track_id: usize,
    pub candidate_track_id: usize,
    /// Relative to the beginning of each selected region, not either original file.
    pub paired_center_seconds: Vec<f64>,
    pub mean_reference_frequency_hz: f64,
    pub mean_candidate_frequency_hz: f64,
    pub mean_candidate_minus_reference_cents: f64,
    pub rms_frequency_error_cents: f64,
    pub mean_candidate_minus_reference_db: f64,
    pub rms_level_error_db: f64,
    pub level_matched_mean_candidate_minus_reference_db: Option<f64>,
    pub level_matched_rms_level_error_db: Option<f64>,
    pub decay: PairedDecay,
}

#[derive(Debug, Default, Serialize)]
pub struct ObservationCounts {
    pub total: usize,
    pub matched: usize,
    pub no_counterpart: usize,
    pub ambiguous_match: usize,
    pub excluded: usize,
}

#[derive(Debug, Serialize)]
pub struct PartialComparison {
    pub schema_version: u32,
    pub method: &'static str,
    pub sample_rate: u32,
    pub reference_region: ComparisonRegion,
    pub candidate_region: ComparisonRegion,
    pub match_cents: f64,
    /// False disables all matching when either detector exceeded a frame or lifetime limit.
    pub detection_complete: bool,
    /// Computed from the complete selected regions, before peak detection.
    pub candidate_level_minus_reference_db: Option<f64>,
    pub candidate_gain_to_match_reference: Option<f64>,
    pub reference_observations: ObservationCounts,
    pub candidate_observations: ObservationCounts,
    pub matches: Vec<MatchedPartial>,
    pub reference_tracking: PartialTracking,
    pub candidate_tracking: PartialTracking,
}

fn region(
    clip: &AudioClip,
    start: f64,
    frames: usize,
) -> Result<(ComparisonRegion, &[f64]), AudioError> {
    if !start.is_finite() || start < 0.0 || start > clip.duration() {
        return Err(AudioError(
            "region start must be finite and inside its recording".into(),
        ));
    }
    let rate = clip.metadata.sample_rate as f64;
    let first = (start * rate).round() as usize;
    let last = first
        .checked_add(frames)
        .ok_or_else(|| AudioError("region size overflow".into()))?;
    let samples = clip
        .samples
        .get(first..last)
        .ok_or_else(|| AudioError("selected region extends beyond its recording".into()))?;
    Ok((
        ComparisonRegion {
            start_frame: first,
            end_frame_exclusive: last,
            start_seconds: first as f64 / rate,
            end_seconds: last as f64 / rate,
        },
        samples,
    ))
}

fn track(samples: &[f64], rate: u32, window_ms: u32) -> PartialTracking {
    let size = (rate as f64 * window_ms as f64 / 1000.0).round() as usize;
    let hop = (rate as f64 * window_ms as f64 / 4000.0).round() as usize;
    let mut result = PartialTracking::new(size, hop, rate);
    for start in (0..=samples.len() - size).step_by(hop) {
        result.push(
            &Spectrum::for_tracking(&samples[start..start + size], rate),
            (start as f64 + size as f64 / 2.0) / rate as f64,
        );
    }
    // The caller explicitly selected a sustain region. Conservatively exclude its first 100 ms.
    result.finish(Some(0.0), Some(samples.len() as f64 / rate as f64));
    result
}

fn frames(tracking: &PartialTracking) -> Vec<Vec<(usize, &PartialObservation)>> {
    let mut result = vec![Vec::new(); tracking.frames.len()];
    for track in &tracking.tracks {
        for observation in &track.observations {
            result[observation.frame].push((track.id, observation));
        }
    }
    result
}

fn excluded(p: &PartialObservation) -> bool {
    p.ambiguous_neighbor || p.capacity_limited
}

fn count(
    observations: &[(usize, &PartialObservation)],
    edges: &[Vec<usize>],
    reverse: &[Vec<usize>],
    counts: &mut ObservationCounts,
) {
    for (i, (_, point)) in observations.iter().enumerate() {
        counts.total += 1;
        if excluded(point) {
            counts.excluded += 1;
        } else if edges[i].is_empty() {
            counts.no_counterpart += 1;
        } else if edges[i].len() == 1 && reverse[edges[i][0]].len() == 1 {
            counts.matched += 1;
        } else {
            counts.ambiguous_match += 1;
        }
    }
}

fn paired_decay(a: &crate::PartialTrack, b: &crate::PartialTrack, centers: &[f64]) -> PairedDecay {
    let rejected = |status| PairedDecay {
        status,
        candidate_minus_reference_slope_db_per_second: None,
        candidate_minus_reference_t60_seconds: None,
    };
    let (Some(a), Some(b)) = (&a.decay.estimate, &b.decay.estimate) else {
        return rejected(PairedDecayStatus::UnqualifiedTrack);
    };
    let (Some(at), Some(bt)) = (a.extrapolated_t60_seconds, b.extrapolated_t60_seconds) else {
        return rejected(PairedDecayStatus::UnqualifiedTrack);
    };
    if (a.start_seconds - b.start_seconds).abs() > 1e-9
        || (a.end_seconds - b.end_seconds).abs() > 1e-9
        || a.points != b.points
    {
        return rejected(PairedDecayStatus::DifferentFitIntervals);
    }
    if centers
        .iter()
        .filter(|&&t| t >= a.start_seconds && t <= a.end_seconds)
        .count()
        != a.points
    {
        return rejected(PairedDecayStatus::IncompletePairing);
    }
    PairedDecay {
        status: PairedDecayStatus::Qualified,
        candidate_minus_reference_slope_db_per_second: Some(
            b.slope_db_per_second - a.slope_db_per_second,
        ),
        candidate_minus_reference_t60_seconds: Some(bt - at),
    }
}

#[derive(Default)]
struct Pair {
    centers: Vec<f64>,
    reference_frequency: f64,
    candidate_frequency: f64,
    cents: f64,
    cents_squared: f64,
    db: f64,
    db_squared: f64,
}

pub fn compare_partials(
    reference: &AudioClip,
    candidate: &AudioClip,
    options: PartialComparisonOptions,
) -> Result<PartialComparison, AudioError> {
    if reference.metadata.sample_rate != candidate.metadata.sample_rate {
        return Err(AudioError(
            "partial comparison requires equal sample rates".into(),
        ));
    }
    if ![32, 128, 512, 1024].contains(&options.partial_window_ms) {
        return Err(AudioError(
            "partial window must be 32, 128, 512 or 1024 ms".into(),
        ));
    }
    if !options.seconds.is_finite()
        || options.seconds < options.partial_window_ms as f64 / 1000.0
        || options.seconds > 60.0
    {
        return Err(AudioError(
            "region duration must cover a complete window and be at most 60 seconds".into(),
        ));
    }
    if !options.match_cents.is_finite() || !(0.0..=100.0).contains(&options.match_cents) {
        return Err(AudioError(
            "matching tolerance must be finite and within 0..100 cents".into(),
        ));
    }
    let rate = reference.metadata.sample_rate;
    let size = (options.seconds * rate as f64).round() as usize;
    let (reference_region, a) = region(reference, options.reference_start_seconds, size)?;
    let (candidate_region, b) = region(candidate, options.candidate_start_seconds, size)?;
    let (ar, br) = (rms(a), rms(b));
    let gain = (ar > 1e-12 && br > 1e-12).then(|| ar / br);
    let gain_db = gain.and_then(db);
    let reference_tracking = track(a, rate, options.partial_window_ms);
    let candidate_tracking = track(b, rate, options.partial_window_ms);
    let detection_complete = reference_tracking.dropped_track_observations == 0
        && candidate_tracking.dropped_track_observations == 0
        && reference_tracking
            .frames
            .iter()
            .all(|f| f.dropped_capacity_peaks == 0)
        && candidate_tracking
            .frames
            .iter()
            .all(|f| f.dropped_capacity_peaks == 0);
    let af = frames(&reference_tracking);
    let bf = frames(&candidate_tracking);
    let mut reference_observations = ObservationCounts::default();
    let mut candidate_observations = ObservationCounts::default();
    let mut pairs: BTreeMap<(usize, usize), Pair> = BTreeMap::new();
    for (a, b) in af.iter().zip(&bf) {
        if !detection_complete {
            reference_observations.total += a.len();
            reference_observations.excluded += a.len();
            candidate_observations.total += b.len();
            candidate_observations.excluded += b.len();
            continue;
        }
        let mut forward = vec![Vec::new(); a.len()];
        let mut reverse = vec![Vec::new(); b.len()];
        for (i, (_, ap)) in a.iter().enumerate() {
            for (j, (_, bp)) in b.iter().enumerate() {
                if !excluded(ap)
                    && !excluded(bp)
                    && (1200.0 * (bp.frequency_hz / ap.frequency_hz).log2()).abs()
                        <= options.match_cents
                {
                    forward[i].push(j);
                    reverse[j].push(i);
                }
            }
        }
        count(a, &forward, &reverse, &mut reference_observations);
        count(b, &reverse, &forward, &mut candidate_observations);
        for (i, neighbors) in forward.iter().enumerate() {
            if neighbors.len() != 1 || reverse[neighbors[0]].len() != 1 {
                continue;
            }
            let (aid, ap) = a[i];
            let (bid, bp) = b[neighbors[0]];
            let pair = pairs.entry((aid, bid)).or_default();
            pair.centers.push(ap.center_seconds);
            pair.reference_frequency += ap.frequency_hz;
            pair.candidate_frequency += bp.frequency_hz;
            let cents = 1200.0 * (bp.frequency_hz / ap.frequency_hz).log2();
            let level_delta = bp.amplitude_dbfs - ap.amplitude_dbfs;
            pair.cents += cents;
            pair.cents_squared += cents * cents;
            pair.db += level_delta;
            pair.db_squared += level_delta * level_delta;
        }
    }
    let matches = pairs
        .into_iter()
        .map(|((aid, bid), pair)| {
            let n = pair.centers.len() as f64;
            MatchedPartial {
                reference_track_id: aid,
                candidate_track_id: bid,
                decay: paired_decay(
                    &reference_tracking.tracks[aid],
                    &candidate_tracking.tracks[bid],
                    &pair.centers,
                ),
                paired_center_seconds: pair.centers,
                mean_reference_frequency_hz: pair.reference_frequency / n,
                mean_candidate_frequency_hz: pair.candidate_frequency / n,
                mean_candidate_minus_reference_cents: pair.cents / n,
                rms_frequency_error_cents: (pair.cents_squared / n).sqrt(),
                mean_candidate_minus_reference_db: pair.db / n,
                rms_level_error_db: (pair.db_squared / n).sqrt(),
                level_matched_mean_candidate_minus_reference_db: gain_db.map(|g| pair.db / n + g),
                level_matched_rms_level_error_db: gain_db.map(|g| {
                    (pair.db_squared / n + 2.0 * g * pair.db / n + g * g)
                        .max(0.0)
                        .sqrt()
                }),
            }
        })
        .collect();
    Ok(PartialComparison {
        schema_version: 1,
        method: "explicit-equal-regions; simultaneous-mutual-unique-peaks-v1",
        sample_rate: rate,
        reference_region,
        candidate_region,
        match_cents: options.match_cents,
        detection_complete,
        candidate_level_minus_reference_db: gain_db.map(|g| -g),
        candidate_gain_to_match_reference: gain,
        reference_observations,
        candidate_observations,
        matches,
        reference_tracking,
        candidate_tracking,
    })
}
