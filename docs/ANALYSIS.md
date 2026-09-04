# Analysis laboratory

This offline Rust tool measures rendered or recorded isolated notes. It does not alter the physical model, fit its parameters, normalize source files or establish equivalence to a real Rhodes. Reports use schema version 1; undefined measurements are JSON `null`, never fabricated zeroes or infinities.

## Inputs and commands

```text
cargo run --locked --release -p rf-rhodes-lab -- analyze renders/a3.wav --output renders/a3-analysis.json --note 57 --sustain-end 1.8
cargo run --locked --release -p rf-rhodes-lab -- compare references/audio/a3.wav renders/a3.wav --output renders/comparison.json --align-ms 20
```

Input supports PCM 8/16/24/32-bit and IEEE float32 WAV, 8–192 kHz, 1–8 channels, up to 60 seconds and 12 million frames. At least 128 samples are required. Multichannel input requires explicit zero-based selection: `--channel`, or `--reference-channel` and `--candidate-channel`. No implicit downmixing or resampling occurs. Float headroom is preserved; nonfinite or absurdly large samples (absolute value above 1,000,000) are rejected. Reports use create-new writes and never replace an existing file.

For comparison, use equal sample rates, comparable note durations and matching capture points. Keep raw reference files and their provenance separate from generated reports. The expected MIDI note defaults to 57 (A3, 220 Hz at A4=440); specify another note explicitly.

## Single-note analysis

| Output | Definition |
| --- | --- |
| Peak, RMS and DC | Computed over the complete selected channel; dBFS uses amplitude 1 as 0 dB |
| Full-scale sample count | Samples with absolute value at least 1; float headroom is not proof of clipping |
| Onset | First sample reaching 1% of file peak (−40 dB), with an absolute minimum of 1e−12 |
| Envelope | Rectangular 20 ms RMS window, 5 ms hop, center timestamps |
| Envelope peak | Largest envelope point within 500 ms after onset; timing is window-dependent |
| Spectra | Hann windows of up to 32 and 96 ms from onset, plus up to 350 ms starting 250 ms later |
| Fundamental candidate | Local spectral maximum within ±6% of the expected frequency; longest available successful observation containing at least four cycles |
| Partial tracks | Six harmonic search bands, 128 ms Hann windows and 32 ms hop; missing peaks remain null |

Each spectrum removes its own DC mean and uses a symmetric Hann window. The one-sided amplitude normalization compensates coherent window gain. A pure stationary sinusoid therefore reports its peak amplitude, not its RMS. FFT size is twice the next power of two of the observed length. Each observation is capped at 32,768 samples; zero padding refines the interpolation grid, not the physical resolution. Reports include both bin spacing and reciprocal observation duration. Peaks use quadratic interpolation of log amplitude and must exceed −60 dB relative to that window's maximum. Spectral centroid is weighted by squared bin amplitudes.

Harmonic search tolerance is the larger of two FFT bins or 1.5% of the target harmonic, capped at 20% of the expected fundamental. Bands cannot identify arbitrary inharmonic mechanical modes; the unlabelled peak list is exploratory. Very low notes need longer observations than this first tool provides. Frequency error from overlapping partials, amplitude modulation and noise can exceed the isolated-sine test tolerance.

Onset detection assumes a reasonably quiet, isolated note. It is sensitive to leading noise, DC and clicks. Sustained chords, processed music and multiple strikes invalidate a single expected-fundamental interpretation. The tool does not classify these inputs automatically.

## Decay estimation

Natural decay is fitted only when `--sustain-end` specifies the end of an uninterrupted sustain region, in seconds from file start. Set it before release, pedal change or a new strike. The regression begins at the later of 100 ms after onset or 50 ms after the envelope peak. Every included RMS window ends before the declared boundary, avoiding contamination by release. Points more than 60 dB below the envelope peak are excluded; this relative cutoff is not a measured noise floor.

The report exposes the least-squares dB/second slope, R², fitted drop, point count and actual fit interval. An **extrapolated** T60 (`−60 / slope`) is returned only for a decreasing fit with at least ten points, 100 ms span, 5 dB fitted drop and R² ≥ 0.95. It does not mean 60 dB of decay was recorded. Beating, multiple decay slopes, room sound and noise require more specific models. Current fits describe the total RMS envelope, not individual modal loss constants.

## Comparison

Alignment searches ±20 ms by default (allowed 0–100 ms) using signed normalized correlation on the first 150 ms after reference onset. A coarse grid with 16-sample box averages is followed by full-rate local refinement. Positive delay means the candidate starts later. Correlation and a search-boundary flag are reported; this heuristic can choose the wrong period for tonal signals. It does not reverse polarity or stretch time. Use `--align-ms 0` for sample-synchronized renders.

Metrics use the common overlap after alignment; original lengths and overlap frame count remain in the report. Extra non-overlapping tails are not scored. Candidate RMS/reference RMS produces the original level difference. Its reciprocal is the positive gain used only for the separate level-matched metrics. Normalized RMSE is waveform-error RMS divided by reference RMS. It is phase-sensitive and is not a perceptual similarity score.

Log-spectral distance is the RMS dB difference across FFT bins from 20 Hz to below Nyquist, using one Hann observation of up to 32,768 aligned samples beginning at reference onset. The report identifies that observation's offset relative to the overlap. Both spectra use a floor 80 dB below the reference spectral peak. Bins below the floor in both files contribute zero, so broad quiet bands dilute the average. Raw and level-matched distances are reported separately. This single-window metric does not summarize the entire decay or establish an aliasing bound.

## Verification

Tests compare the FFT with a direct DFT, recover known sinusoidal pitch and amplitude, recover a known exponential slope, exclude release from decay windows, preserve null values for silence and missing fundamentals, recover injected delays, preserve gain/polarity differences, and handle leading silence. WAV fixtures cover integer widths, channel selection, float headroom, metadata chunks, truncation and nonfinite samples. CLI tests render, analyze, compare, parse JSON and verify no-overwrite/error behavior.

These tests validate the measurement implementation against known signals. Real-instrument calibration still requires documented direct recordings and held-out validation takes.
