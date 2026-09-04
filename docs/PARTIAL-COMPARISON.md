# Comparing spectral components

`compare-partials` compares detected components in equal-duration, explicitly selected regions of two WAV files. It uses the existing independent tracker. It does not fit the physical model, identify a mechanical mode, or modify either recording. This report has its own schema version 1; `analyze` and waveform `compare` keep their existing schemas.

## Select corresponding regions

```text
cargo run --locked --release -p rf-rhodes-lab -- compare-partials references/audio/a3.wav renders/a3.wav --output renders/a3-partials.json --seconds 2 --reference-start 0.2 --candidate-start 0.2
```

Both regions must represent the same note and intended gesture phase, with uninterrupted sustain through the end. Set the start offsets to comparable note ages. `--seconds` is required; starts default to zero. No onset alignment, time warping or resampling is performed. Start offsets and duration are rounded to samples, and the report records actual start/end frames and times. An out-of-file region is rejected rather than cropped or padded.

Both inputs need equal sample rates and retain their original gain. Multichannel WAVs require explicit `--reference-channel` and/or `--candidate-channel`, using zero-based indices. The existing WAV input limits apply: 8–192 kHz, up to 60 seconds and 12 million frames. Output must be a new `.json` file.

`--partial-window-ms` accepts 32, 128 (default), 512 or 1024, with a quarter-window hop. The selected duration must contain at least one full window. Both regions use identical sample counts and analysis grids. Choose a long window to distinguish close frequencies and a short one to inspect brief components. See [Analysis laboratory](ANALYSIS.md) for physical resolution limits and minimum decay-fit requirements.

## Match observations before comparing tracks

For each simultaneous frame, candidate/reference frequency distance is `1200 log2(candidate_hz / reference_hz)`. `--match-cents` defaults to 50 and accepts finite values from 0 through 100. A match requires exactly one eligible candidate for the reference and exactly one eligible reference for that candidate. The algorithm refuses ambiguous alternatives instead of choosing the closest by force. The tolerance is an association gate, not a pitch-accuracy claim; deviations outside it appear as unmatched observations rather than large scored errors.

Observations already flagged for unresolved neighbors or capacity limits are excluded. If either tracker exceeds any frame or lifetime capacity limit, `detection_complete` is false and all matching is disabled for that report. Both full tracking reports remain available for diagnosis. Even when that flag is true, weak or unresolved physical components may remain undetected.

Counts on each side partition stored observations into `matched`, `no_counterpart`, `ambiguous_match` and `excluded`. `no_counterpart` means no eligible detection within the gate in that frame; it does not prove that a resonance is physically absent. Tracker IDs and matches apply to this report only. Missed detections can split a physical component into multiple tracks. Each matched row groups one reference/candidate track pair and includes every paired center time relative to the start of the selected regions. Missing pairings are not interpolated.

## Read differences and their limits

| Output | Meaning |
| --- | --- |
| Mean reference/candidate frequency | Arithmetic mean over the paired observations only |
| Mean candidate-minus-reference cents | Signed average pitch difference |
| RMS frequency error in cents | Nonnegative error magnitude; opposite errors do not cancel |
| Mean candidate-minus-reference dB | Signed difference between interpolated spectral amplitudes |
| RMS level error in dB | Nonnegative error magnitude across paired observations |
| Level-matched mean/RMS dB error | Same measurements after applying one whole-region RMS gain analytically |

The normalization gain is reference RMS divided by candidate RMS over the complete selected regions, including unmatched components and noise. The original region-level difference and raw per-component differences remain in the report. No per-track normalization is used. Silence or effectively zero region RMS makes the normalization fields null. The mean dB difference is a mean of logarithmic differences, not the ratio of average linear amplitudes.

Metrics score only paired observations. They can look favorable when many observations are missing or excluded: always inspect the counts and full tracks. Broad noise, preprocessing, window smearing and spectral interference can affect the measurements. These metrics are diagnostic, not an aggregate perceptual score.

## Compare decay only on the same evidence

The user-selected region supplies the sustain boundary. The tracker conservatively excludes complete windows intersecting the first 100 ms of that region or its end. Both tracks must pass the existing decay qualification. A comparison then requires identical fit start/end times, equal fit-point counts, and a pair at every included fit time.

`decay.status` is one of:

- `qualified`: reports candidate-minus-reference dB/second slope and extrapolated T60 difference.
- `unqualified_track`: at least one track has no qualified T60; consult its rejection reasons.
- `different_fit_intervals`: the two qualified fits used different observations.
- `incomplete_pairing`: their fit intervals agree, but some fit observations could not be paired.

Every rejected comparison has null decay differences. A more positive slope difference means the candidate decays less steeply when both slopes are negative. T60 differences refer to extrapolations, not necessarily observed 60 dB decays, and do not directly identify mechanical damping behind a nonlinear pickup.

## Verification

Controlled fixtures cover identity, gain scaling, a known +20-cent detuning, known slope/T60 differences, missing components, silence, ambiguous alternatives, explicit sample offsets, differing fit intervals, incomplete pairing, opposing level errors, invalid regions/rates/options, and capacity truncation. The CLI test parses the report, verifies identity and original-file metadata, and checks no-overwrite and error behavior. Real-instrument calibration still requires documented recordings and validation takes.
