//! Temporal coverage of the fixed paired band-envelope decision.
use rf_73_analysis::{AudioClip, BandEnvelope, ShortEnvelopeOptions, measure_band_envelope};
use serde_json::{Value, json};
use std::{error::Error, f64::consts::TAU, path::Path};

pub const HELP: &str = "Band-envelope temporal coverage:
  study-band-events --output REPORT.json
Fixed onset/release/rate-change grid at 44.1/48/96 kHz, unchanged 32/64 ms decision.
All outcomes retained, including accepted event-containing intervals. No source calibration.
";
const TIMES: [f64; 13] = [
    0.0, 0.008, 0.02, 0.028, 0.04, 0.06, 0.10, 0.14, 0.164, 0.176, 0.18, 0.188, 0.204,
];

#[derive(Clone, Copy, PartialEq)]
enum Event {
    None,
    Absent,
    Onset,
    Release,
    Increase,
    Decrease,
}
impl Event {
    fn label(self) -> &'static str {
        match self {
            Self::None => "steady_control",
            Self::Absent => "absent_control",
            Self::Onset => "onset",
            Self::Release => "release",
            Self::Increase => "loss_increase",
            Self::Decrease => "loss_decrease",
        }
    }
    fn after_rate(self) -> f64 {
        match self {
            Self::Release => 40.0,
            Self::Increase => 12.0,
            Self::Decrease => 4.0,
            _ => 8.0,
        }
    }
    fn amplitude(self, sample: usize, rate: u32, event_sample: usize) -> f64 {
        if self == Self::Absent || (self == Self::Onset && sample < event_sample) {
            return 0.0;
        }
        let t = sample as f64 / rate as f64;
        let change = event_sample as f64 / rate as f64;
        let exponent = 8.0 * t.min(change) + self.after_rate() * (t - change).max(0.0);
        0.002 * (-exponent).exp()
    }
}

fn synthesize(event: Event, event_sample: usize, rate: u32) -> Result<AudioClip, Box<dyn Error>> {
    let mut seed = 0x73_ba11_u64;
    let samples = (0..rate as usize / 4)
        .map(|i| {
            let t = i as f64 / rate as f64;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = 2.0 * ((seed >> 11) as f64 / (1_u64 << 53) as f64) - 1.0;
            event.amplitude(i, rate, event_sample) * (TAU * 1620.0 * t + 0.73).cos()
                + 0.6 * (-0.33 * t).exp() * (TAU * 196.35 * t + 0.1).cos()
                + 0.02 * (-t).exp() * (TAU * 1568.0 * t - 0.4).cos()
                + 0.05 * (-0.66 * t).exp() * (TAU * 392.7 * t + 0.9).cos()
                + 0.015 * (-0.99 * t).exp() * (TAU * 589.05 * t - 0.7).cos()
                + 0.00001 * noise
        })
        .collect();
    Ok(AudioClip::from_samples(rate, samples)?)
}

fn pair(a: &BandEnvelope, b: &BandEnvelope) -> (Option<f64>, Vec<&'static str>) {
    let mut reasons = Vec::new();
    if !a.measurement.qualified {
        reasons.push("window_32_rejected");
    }
    if !b.measurement.qualified {
        reasons.push("window_64_rejected");
    }
    let rates = a
        .measurement
        .provisional_fit
        .as_ref()
        .zip(b.measurement.provisional_fit.as_ref());
    let Some((x, y)) = rates else {
        reasons.push("missing_fit");
        return (None, reasons);
    };
    let (x, y) = (x.amplitude_decay_per_second, y.amplitude_decay_per_second);
    if (x - y).abs() > 0.5_f64.max(0.15 * x.abs().max(y.abs())) {
        reasons.push("window_rate_disagreement");
    }
    (reasons.is_empty().then_some((x + y) * 0.5), reasons)
}

fn compact(r: &BandEnvelope, rate: u32) -> Value {
    let m = &r.measurement;
    let used_end = (!m.points.is_empty()).then(|| {
        r.filter.filtered_start_sample
            + (m.points.len() - 1) * (m.actual_hop_seconds * rate as f64).round() as usize
            + (m.actual_window_seconds * rate as f64).round() as usize
    });
    json!({"filter":r.filter,"options":m.options,"qualified":m.qualified,"rejection_reasons":m.rejection_reasons,
        "provisional_fit":m.provisional_fit,"minimum_relative_qr_pivot":m.minimum_relative_qr_pivot,
        "point_count":m.points.len(),"actual_window_seconds":m.actual_window_seconds,"actual_hop_seconds":m.actual_hop_seconds,
        "first_center_seconds":m.points.first().map(|p|p.center_seconds),"last_center_seconds":m.points.last().map(|p|p.center_seconds),
        "last_window_end_sample_exclusive":used_end,"used_source_end_sample_exclusive":used_end.map(|n|n+r.filter.half_support_samples),
        "minimum_regression_margin_db":m.points.iter().map(|p|p.regression_margin_db).reduce(f64::min)})
}

fn region(sample: usize, r: &BandEnvelope) -> &'static str {
    if sample < r.filter.source_start_sample {
        "before_filter_support"
    } else if sample < r.filter.filtered_start_sample {
        "leading_filter_halo"
    } else if sample < r.filter.filtered_end_sample_exclusive {
        "measurement_interval"
    } else if sample < r.filter.source_end_sample_exclusive {
        "trailing_filter_halo"
    } else {
        "after_filter_support"
    }
}

fn observe(event: Event, time: Option<f64>, rate: u32) -> Result<Value, Box<dyn Error>> {
    let event_sample = time.map_or(0, |t| (t * rate as f64).ceil() as usize);
    let clip = synthesize(event, event_sample, rate)?;
    let mut results = Vec::new();
    for window in [0.032, 0.064] {
        results.push(measure_band_envelope(
            &clip,
            ShortEnvelopeOptions {
                frequencies_hz: vec![1620.0, 1568.0],
                start_seconds: 0.02,
                end_seconds: 0.18,
                window_seconds: window,
                hop_seconds: 0.008,
            },
        )?);
    }
    let (mean, reasons) = pair(&results[0], &results[1]);
    let location = time.map(|_| region(event_sample, &results[0]));
    Ok(
        json!({"event":event.label(),"sample_rate_hz":rate,"requested_event_seconds":time,
        "event_sample":time.map(|_|event_sample),"actual_event_seconds":time.map(|_|event_sample as f64/rate as f64),
        "ground_truth_region":location,"post_event_target_decay_per_second":event.after_rate(),
        "paired_qualified":mean.is_some(),"paired_rejections":reasons,"conditional_amplitude_decay_per_second":mean,
        "accepted_event_in_measurement_interval":mean.is_some() && location==Some("measurement_interval"),
        "natural_sustain_status":"not_identified_by_measurement",
        "measurements":results.iter().map(|r|compact(r,rate)).collect::<Vec<_>>() }),
    )
}

fn study() -> Result<Value, Box<dyn Error>> {
    let mut cases = Vec::new();
    let mut controls = Vec::new();
    for rate in [44100, 48000, 96000] {
        let steady = observe(Event::None, None, rate)?;
        let absent = observe(Event::Absent, None, rate)?;
        let passed = steady["conditional_amplitude_decay_per_second"]
            .as_f64()
            .is_some_and(|r| (r - 8.0).abs() < 0.1)
            && absent["paired_qualified"] == false;
        controls.push(json!({"sample_rate_hz":rate,"steady_and_absent_passed":passed}));
        cases.push(steady);
        cases.push(absent);
        for event in [
            Event::Onset,
            Event::Release,
            Event::Increase,
            Event::Decrease,
        ] {
            for time in TIMES {
                cases.push(observe(event, Some(time), rate)?);
            }
        }
    }
    let accepted_inside = cases
        .iter()
        .filter(|c| c["accepted_event_in_measurement_interval"] == true)
        .count();
    let inside = cases
        .iter()
        .filter(|c| c["ground_truth_region"] == "measurement_interval")
        .count();
    Ok(
        json!({"schema_version":1,"experiment":"paired-band-event-coverage-v1","cases":cases,"control_checks":controls,
        "controls_passed":controls.iter().all(|c|c["steady_and_absent_passed"]==true),
        "summary":{"event_pairs_in_measurement_interval":inside,"accepted_event_pairs_in_measurement_interval":accepted_inside},
        "protocol":"Fixed before first run: 0.25 s clips at 44.1/48/96 kHz. Steady and absent controls, plus onset, release (8 to 40 /s), loss increase (8 to 12 /s), loss decrease (8 to 4 /s) at 0, 0.008, 0.02, 0.028, 0.04, 0.06, 0.10, 0.14, 0.164, 0.176, 0.18, 0.188, 0.204 s. Event index is ceil(time*rate); actual event time is that index/rate. Onset is zero before its index then the absolute-time 8 /s exponential. Other transitions preserve amplitude using exp(-8*min(t,event)-after_rate*max(t-event,0)). Target 1620 Hz, amplitude 0.002, phase 0.73. Interference: 196.35/1568/392.7/589.05 Hz, amplitudes 0.6/0.02/0.05/0.015, phases 0.1/-0.4/0.9/-0.7, rates 0.33/1/0.66/0.99 /s. Uniform LCG64 seed 0x73ba11 noise amplitude 0.00001. Fixed 0.02..0.18 s interval, target/1568 carriers, unchanged centered FIR and 32/64 ms windows with 8 ms hop. Pair requires both qualified and rate agreement within max(0.5 /s, 15% of larger absolute rate). Every event outcome is descriptive; only steady/absent controls have pass expectations. No expectation that arbitrary in-interval events must reject, no adaptive timing or gate search.",
        "scope":"Ground-truth event location is supplied by synthesis, not inferred from audio. Filter support is the conservative whole crop plus halos; per-window used end also retained. Paired agreement does not identify an uninterrupted exponential over the requested interval or natural mechanical sustain. A release here is only a prescribed amplitude-decay switch, not a validated damper model. No source audio, physical parameters, production DSP or estimator gates changed. Compact diagnostics retained instead of redundant sample/point arrays; no WAV output."}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = study()?;
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("band event study retained failed controls".into());
    }
    println!("Band event coverage: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_decision_requires_each_window_and_consistent_rates() {
        let clip = synthesize(Event::None, 0, 48000).unwrap();
        let mut results: Vec<_> = [0.032, 0.064]
            .into_iter()
            .map(|window_seconds| {
                measure_band_envelope(
                    &clip,
                    ShortEnvelopeOptions {
                        frequencies_hz: vec![1620.0, 1568.0],
                        start_seconds: 0.02,
                        end_seconds: 0.18,
                        window_seconds,
                        hop_seconds: 0.008,
                    },
                )
                .unwrap()
            })
            .collect();
        assert!((pair(&results[0], &results[1]).0.unwrap() - 8.0).abs() < 0.1);
        results[0].measurement.qualified = false;
        assert!(pair(&results[0], &results[1]).0.is_none());
        results[0].measurement.qualified = true;
        results[1]
            .measurement
            .provisional_fit
            .as_mut()
            .unwrap()
            .amplitude_decay_per_second = 12.0;
        assert!(
            pair(&results[0], &results[1])
                .1
                .contains(&"window_rate_disagreement")
        );
        results[1].measurement.provisional_fit = None;
        assert!(pair(&results[0], &results[1]).1.contains(&"missing_fit"));
    }
    #[test]
    fn prescribed_events_use_sample_indices_and_continuous_rate_changes() {
        let rate = 48000;
        let sample = 2880;
        for e in [Event::Release, Event::Increase, Event::Decrease] {
            let at = e.amplitude(sample, rate, sample);
            assert!((at - 0.002 * (-8.0 * 0.06_f64).exp()).abs() < 1e-15);
            assert!(
                (e.amplitude(sample + 1, rate, sample) / at
                    - (-e.after_rate() / rate as f64).exp())
                .abs()
                    < 1e-14
            );
            assert_eq!(
                e.amplitude(sample - 1, rate, sample),
                Event::None.amplitude(sample - 1, rate, 0)
            );
        }
        assert_eq!(Event::Onset.amplitude(sample - 1, rate, sample), 0.0);
        assert_eq!(
            Event::Onset.amplitude(sample, rate, sample),
            Event::None.amplitude(sample, rate, 0)
        );
    }
    #[test]
    fn events_after_source_support_are_observationally_identical_to_their_controls() {
        let rate = 48000;
        let event_sample = (0.204 * rate as f64) as usize;
        let start = 192;
        let end = 9408; // Real support for the fixed 20..180 ms crop plus 16 ms halos.
        for event in [
            Event::Onset,
            Event::Release,
            Event::Increase,
            Event::Decrease,
        ] {
            let control = if event == Event::Onset {
                Event::Absent
            } else {
                Event::None
            };
            let source = synthesize(event, event_sample, rate).unwrap();
            let reference = synthesize(control, 0, rate).unwrap();
            assert_eq!(
                &source.samples()[start..end],
                &reference.samples()[start..end]
            );
        }
    }
}
