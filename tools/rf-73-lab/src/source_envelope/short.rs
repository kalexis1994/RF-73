//! Fixed short-source protocol using the same pinned evidence as the long pilot.
use super::*;
use rf_73_analysis::{ShortEnvelope, ShortEnvelopeOptions, measure_short_envelope};

pub const HELP: &str = "Pinned short source envelopes:
  observe-short-source-envelopes MANIFEST.json --output REPORT.json
Strict schema-1 source selection, interval 0.128..0.3 s; all pinned takes retained.
Two nearest other prior attack peaks, fixed 32/64 ms windows and 8 ms hop.
No carrier/interval search, loss calibration, source editing or playback.
";

struct Carriers {
    frequencies: Vec<f64>,
    nuisance_indices: Vec<usize>,
    omitted_indices: Vec<usize>,
    reasons: Vec<&'static str>,
    supported: bool,
}

fn carriers(window: &Window, selection: &Selection, sample_rate: u32) -> Carriers {
    let mut result = Carriers {
        frequencies: Vec::new(),
        nuisance_indices: Vec::new(),
        omitted_indices: Vec::new(),
        reasons: Vec::new(),
        supported: false,
    };
    let Some(target) = selection.frequency else {
        result.omitted_indices = (0..window.accepted_peaks.len()).collect();
        return result;
    };
    let mut others: Vec<_> = window
        .accepted_peaks
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != selection.excluded_peak)
        .collect();
    others.sort_by(|(ia, a), (ib, b)| {
        (a.frequency_hz - target)
            .abs()
            .total_cmp(&(b.frequency_hz - target).abs())
            .then(ia.cmp(ib))
    });
    result.frequencies.push(target);
    for (index, peak) in others.iter().take(2) {
        result.nuisance_indices.push(*index);
        result.frequencies.push(peak.frequency_hz);
        if peak.ambiguous_neighbor && !result.reasons.contains(&"nuisance_peak_ambiguous") {
            result.reasons.push("nuisance_peak_ambiguous");
        }
        if peak.capacity_limited && !result.reasons.contains(&"nuisance_peak_capacity_limited") {
            result.reasons.push("nuisance_peak_capacity_limited");
        }
    }
    result.omitted_indices = others.iter().skip(2).map(|(i, _)| *i).collect();
    if result.nuisance_indices.len() < 2 {
        result.reasons.push("fewer_than_two_prior_nuisance_peaks");
    }
    // Do not replace unsupported/duplicate carriers with more convenient peaks.
    result.supported = result.frequencies.iter().enumerate().all(|(i, f)| {
        *f > 4.0 / 0.032 && *f < 0.45 * sample_rate as f64 && !result.frequencies[..i].contains(f)
    });
    if !result.supported {
        result
            .reasons
            .push("selected_carriers_outside_estimator_support");
    }
    result
}

fn consensus(
    measurements: &[ShortEnvelope],
    selection: &Selection,
    carriers: &Carriers,
) -> (Option<f64>, Vec<&'static str>) {
    let mut reasons = Vec::new();
    if !selection.reasons.is_empty() || !carriers.reasons.is_empty() {
        reasons.push("prior_selection_unqualified");
    }
    if measurements.len() != 2 {
        reasons.push("missing_window_measurements");
    }
    if measurements
        .iter()
        .any(|m| !m.qualified || m.provisional_fit.is_none())
    {
        reasons.push("envelope_gate_rejected");
    }
    if !reasons.is_empty() {
        return (None, reasons);
    }
    let a = measurements[0]
        .provisional_fit
        .as_ref()
        .unwrap()
        .amplitude_decay_per_second;
    let b = measurements[1]
        .provisional_fit
        .as_ref()
        .unwrap()
        .amplitude_decay_per_second;
    if (a - b).abs() > 0.5_f64.max(0.15 * a.abs().max(b.abs())) {
        reasons.push("window_rate_disagreement");
        (None, reasons)
    } else {
        (Some((a + b) * 0.5), reasons)
    }
}

fn observe(clip: &AudioClip, input: &Input, manifest: &Manifest) -> Result<Value, Box<dyn Error>> {
    let window = attack_window(clip, input)?;
    let mut components = Vec::new();
    for (label, band) in [
        ("fundamental", None),
        ("lower_family", Some(manifest.lower_family_band_hz)),
        ("upper_family", Some(manifest.upper_family_band_hz)),
    ] {
        let selection = select(input, window, band);
        let carriers = carriers(window, &selection, clip.metadata().sample_rate);
        let mut measurements = Vec::new();
        if carriers.supported {
            for duration in [0.032, 0.064] {
                measurements.push(measure_short_envelope(
                    clip,
                    ShortEnvelopeOptions {
                        frequencies_hz: carriers.frequencies.clone(),
                        start_seconds: manifest.start_seconds,
                        end_seconds: manifest.end_seconds,
                        window_seconds: duration,
                        hop_seconds: 0.008,
                    },
                )?);
            }
        }
        let (rate, reasons) = consensus(&measurements, &selection, &carriers);
        components.push(json!({"label": label,
            "selected_frequency_hz": selection.frequency,
            "excluded_prior_peak_index": selection.excluded_peak,
            "selection_rejections": selection.reasons,
            "declared_frequencies_hz": carriers.frequencies,
            "nuisance_prior_peak_indices": carriers.nuisance_indices,
            "omitted_prior_peak_indices": carriers.omitted_indices,
            "nuisance_selection_rejections": carriers.reasons,
            "measurements": measurements,
            "cross_window_rejections": reasons,
            "conditional_amplitude_decay_per_second": rate}));
    }
    Ok(
        json!({"audio": clip.metadata(), "prior_attack_window": window,
        "prior_fundamental_hz": input.pitch_anchor.frequency_hz, "components": components}),
    )
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let bytes = read_bounded(Path::new(&args[1]), 65536)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate_interval(0.128, 0.3)?;
    let evidence = verified_evidence(
        Path::new(&manifest.evidence_file),
        &manifest.evidence_git_blob_sha1,
    )?;
    let (prior, group) = group(&evidence, manifest.note)?;
    let mut takes = Vec::new();
    for take in &prior.takes {
        let input = group
            .inputs
            .iter()
            .find(|i| i.id == take.id)
            .ok_or("missing source observation")?;
        let clip =
            crate::pitch_reference::verified_audio(Path::new(&take.file), &take.git_blob_sha1)?;
        let observation = observe(&clip, input, &manifest)?;
        takes.push(json!({"take": take, "observation": observation}));
        println!("Short source envelopes: {}", take.id);
    }
    crate::analysis::write_report(
        output,
        &json!({"schema_version": 1,
        "experiment": "pinned-short-source-envelopes-v1", "manifest": manifest, "takes": takes,
        "protocol": "All takes retained. Fundamental uses the qualified prior anchor; families use a unique attack-128 peak in the frozen band. Exclude the unique associated target peak, then select the two nearest other prior peaks by absolute frequency distance, ties by prior index. No amplitude ranking, harmonic snapping or frequency refinement. Keep omitted indices and reliability flags. Unsupported carriers retain a selection rejection without substitution. Fixed 32/64 ms windows, 8 ms hop. Both unchanged estimator gates and reliable selections required; rates must agree within max(0.5 /s, 15% of larger absolute rate) for their descriptive mean. No fitting or trimming of the declared interval.",
        "scope": "Processed mono harp-output recordings, unknown capture gain, strike speed and note-off. Times are file offsets. Three carriers do not span the whole waveform: omitted partials and transients enter the residual. The residual-based regression margin is not a colored-noise confidence bound. Nearest peaks are nuisance candidates, not established harmonics or modes. Window agreement remains conditional and does not identify natural mechanical loss, pickup mixing or physical parameters. No source audio, DSP or preset changes; no listening claim. Byte/format errors fail before report publication; rejected observations remain evidence."}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_carriers_keep_ties_omissions_and_reliability_failures() {
        let mut input = super::super::tests::input(vec![1620.0, 1672.0, 1568.0, 1425.0, 196.0]);
        let window = &mut input.observation_windows[0];
        window.accepted_peaks[2].ambiguous_neighbor = true;
        let selection = Selection {
            frequency: Some(1620.0),
            excluded_peak: Some(0),
            reasons: vec![],
        };
        let c = carriers(window, &selection, 48000);
        assert_eq!(c.frequencies, [1620.0, 1672.0, 1568.0]);
        assert_eq!(c.nuisance_indices, [1, 2]);
        assert_eq!(c.omitted_indices, [3, 4]);
        assert!(c.reasons.contains(&"nuisance_peak_ambiguous"));
        assert!(c.supported);
        window.accepted_peaks[1].frequency_hz = 1620.0;
        let c = carriers(window, &selection, 48000);
        assert!(!c.supported);
        assert!(
            c.reasons
                .contains(&"selected_carriers_outside_estimator_support")
        );
        window.accepted_peaks.truncate(1);
        assert!(
            carriers(window, &selection, 48000)
                .reasons
                .contains(&"fewer_than_two_prior_nuisance_peaks")
        );
    }

    #[test]
    fn short_manifest_has_separate_support_and_rejects_unknown_fields() {
        let text = include_str!("../../../../references/short-source-envelope.manifest.json");
        let manifest: Manifest = serde_json::from_str(text).unwrap();
        manifest.validate_interval(0.128, 0.3).unwrap();
        assert!(manifest.validate().is_err());
        for (key, value) in [
            ("end_seconds", json!(0.5)),
            ("start_seconds", json!(-0.1)),
            ("selection", json!("")),
            ("lower_family_band_hz", json!([1400.0, 1500.0])),
        ] {
            let mut v: Value = serde_json::from_str(text).unwrap();
            v[key] = value;
            assert!(
                serde_json::from_value::<Manifest>(v)
                    .unwrap()
                    .validate_interval(0.128, 0.3)
                    .is_err()
            );
        }
        let mut v: Value = serde_json::from_str(text).unwrap();
        v["nuisance_override"] = json!([1568.0]);
        assert!(serde_json::from_value::<Manifest>(v).is_err());
    }

    #[test]
    fn short_consensus_withholds_unreliable_and_disagreeing_measurements() {
        let clip = AudioClip::from_samples(
            48000,
            (0..12000)
                .map(|i| {
                    let t = i as f64 / 48000.0;
                    0.03 * (-8.0 * t).exp() * (std::f64::consts::TAU * 1620.0 * t + 0.73).cos()
                        + 0.3 * (-t).exp() * (std::f64::consts::TAU * 1568.0 * t - 0.4).cos()
                })
                .collect(),
        )
        .unwrap();
        let input = super::super::tests::input(vec![1620.0, 1568.0, 1425.0]);
        let mut selection = Selection {
            frequency: Some(1620.0),
            excluded_peak: Some(0),
            reasons: vec![],
        };
        let mut c = carriers(&input.observation_windows[0], &selection, 48000);
        let mut measurements: Vec<_> = [0.032, 0.064]
            .into_iter()
            .map(|window_seconds| {
                measure_short_envelope(
                    &clip,
                    ShortEnvelopeOptions {
                        frequencies_hz: c.frequencies.clone(),
                        start_seconds: 0.02,
                        end_seconds: 0.18,
                        window_seconds,
                        hop_seconds: 0.008,
                    },
                )
                .unwrap()
            })
            .collect();
        assert!((consensus(&measurements, &selection, &c).0.unwrap() - 8.0).abs() < 0.01);
        assert!(consensus(&measurements[..1], &selection, &c).0.is_none());
        selection.reasons.push("source_window_capacity_limited");
        assert!(consensus(&measurements, &selection, &c).0.is_none());
        selection.reasons.clear();
        c.reasons.push("nuisance_peak_ambiguous");
        assert!(consensus(&measurements, &selection, &c).0.is_none());
        c.reasons.clear();
        measurements[1].qualified = false;
        assert!(consensus(&measurements, &selection, &c).0.is_none());
        measurements[1].qualified = true;
        measurements[1]
            .provisional_fit
            .as_mut()
            .unwrap()
            .amplitude_decay_per_second = 12.0;
        assert!(
            consensus(&measurements, &selection, &c)
                .1
                .contains(&"window_rate_disagreement")
        );
    }
}
