# Analysis laboratory

This offline Rust tool measures rendered or recorded isolated notes. It does not alter the physical model, fit its parameters, normalize source files or establish equivalence to a real Rhodes. Single-note analysis uses schema version 2, adding `inharmonic_tracking` while retaining the existing harmonic outputs. Comparison remains schema version 1. Undefined measurements are JSON `null`, never fabricated zeroes or infinities.

## Inputs and commands

```text
cargo run --locked --release -p rf-73-lab -- analyze renders/a3.wav --output renders/a3-analysis.json --note 57 --sustain-end 1.8
cargo run --locked --release -p rf-73-lab -- analyze renders/a3.wav --output renders/a3-attack.json --note 57 --partial-window-ms 32
cargo run --locked --release -p rf-73-lab -- analyze references/audio/a3.wav --output renders/a3-sustain.json --note 57 --partial-window-ms 1024 --sustain-end 5
cargo run --locked --release -p rf-73-lab -- compare references/audio/a3.wav renders/a3.wav --output renders/comparison.json --align-ms 20
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
| Harmonic tracks (`partial_tracks`) | Six harmonic search bands, 128 ms Hann windows and 32 ms hop; missing peaks remain null |
| Independent tracks (`inharmonic_tracking`) | Local spectral peaks without assumed frequency ratios, contiguous frame association and qualified per-track decay |

Each spectrum removes its own DC mean and uses a symmetric Hann window. The one-sided amplitude normalization compensates coherent window gain. A pure stationary sinusoid therefore reports its peak amplitude, not its RMS. FFT size is twice the next power of two of the observed length. Each observation is capped at 32,768 samples; zero padding refines the interpolation grid, not the physical resolution. Reports include both bin spacing and reciprocal observation duration. Peaks use quadratic interpolation of log amplitude and must exceed −60 dB relative to that window's maximum. Spectral centroid is weighted by squared bin amplitudes.

Harmonic search tolerance is the larger of two FFT bins or 1.5% of the target harmonic, capped at 20% of the expected fundamental. Bands cannot identify arbitrary inharmonic mechanical modes; the unlabelled peak list is exploratory. Very low notes need longer observations than this first tool provides. Frequency error from overlapping partials, amplitude modulation and noise can exceed the isolated-sine test tolerance.

Onset detection assumes a reasonably quiet, isolated note. It is sensitive to leading noise, DC and clicks. Sustained chords, processed music and multiple strikes invalidate a single expected-fundamental interpretation. The tool does not classify these inputs automatically.

## Independent spectral tracks

The independent detector accepts `--partial-window-ms 32|128|512|1024`, defaulting to 128. Its hop is one quarter of the requested duration; window and hop sample counts are rounded independently. The default reuses harmonic track FFTs. Other choices use a separate pass over complete observations, without changing the original harmonic summaries. Actual durations, observed sample count, FFT size, bin spacing and resolution are reported. Clips shorter than one complete window return empty tracking arrays rather than silently reducing the requested duration. The detector does not use the expected note, harmonic bands or ideal beam ratios. A track can represent a harmonic, inharmonic component, intermodulation product or analysis artifact; its presence does not identify a mechanical mode.

| Independent window | Hop | Minimum separation at 48 kHz | Intended use |
| --- | --- | --- | --- |
| 32 ms | 8 ms | 62.5 Hz | Brief attack components; low-frequency separation is poor |
| 128 ms | 32 ms | 15.625 Hz | Default compromise |
| 512 ms | 128 ms | 3.90625 Hz | Closer sustained components |
| 1024 ms | 256 ms | 1.953125 Hz | Fine sustained separation; attacks are strongly smeared |

Window selection changes time/frequency resolution, spectral background and which tracks survive. Results from different windows are separate measurements; track IDs cannot be joined across runs. Longer windows need longer uninterrupted recordings for the same minimum fit-point count. Twelve eligible observations with a 1024 ms window require approximately 4.1 seconds before release, including the initial attack exclusion. A single long observation can separate frequencies without supporting any decay estimate.

The independent path is bounded by 196,608 observed samples and a 524,288-point FFT (1024 ms at the supported 192 kHz maximum). It does not inherit the 32,768-sample cap used by legacy snapshots/comparison. At 48 kHz the 32 ms option produces 7,497 frames for a 60-second recording, with at most 32 retained peaks per frame; other rates use their reported rounded hop. These costs belong to the offline laboratory, not the audio callback.

The search band starts at the larger of 20 Hz and twice the reciprocal observation duration, and ends at the smaller of 20 kHz and Nyquist minus that separation. Background is the upper median of spectral-bin amplitudes. Each peak also uses a local median within the larger of ±200 Hz or ±12 observation-resolution units, excluding the central ±2 units. Detection requires 18 dB above the larger background estimate and at least −70 dB relative to the current band maximum. Background amplitudes are floored at −180 dBFS for finite reporting. They are spectral background proxies, not calibrated broadband noise RMS or statistical confidence bounds.

Candidates are processed strongest first. Beyond two resolution units, a continuous-Hann sidelobe envelope with 6 dB margin rejects likely leakage from accepted stronger peaks: amplitude ratio `2 / (pi d (d*d - 1))`, where `d` is separation divided by reciprocal observation duration. This conservative heuristic can suppress a weak real neighbor, and modulated signals can still produce transient false peaks. It is tested for isolated off-bin tones, but it is not a guarantee of artifact-free detection.

Two retained peaks closer than `minimum_separation_hz` (twice the reciprocal observation duration) are flagged `ambiguous_neighbor`. A single blended peak cannot reveal hidden unresolved components. At 48 kHz the reported reciprocal duration is 7.8125 Hz and the separation guard is 15.625 Hz, regardless of the finer FFT grid. Interpolated frequency is not accompanied by an invented confidence interval.

One-to-one nearest-frequency association uses a gate of 0.75 observation-resolution units and deterministic tie ordering. Only adjacent frames can connect. Missing detections split a track; no extrapolation bridges silence or noise. Each observation retains its frame, time, frequency, amplitude, background margin and ambiguity/capacity flags. Track IDs belong to this report only, not to fixed physical modes across recordings. Rapid pitch variation and crossings are outside this tracker's intended use.

Limits are 32 retained peaks per frame and 2,048 tracks per recording. Frame summaries count weak, leakage and capacity exclusions; `dropped_track_observations` reports lifetime overflow. Capacity-limited frames invalidate per-track extrapolation. Input duration, FFT size and these limits bound work and report growth. Brief tracks are retained for attack inspection even when no decay fit is possible.

### Per-track decay qualification

An explicit `--sustain-end` is required. Fit observations must have their entire window after 100 ms from detected onset and before that boundary. A contiguous track needs at least 12 eligible observations. Its least-squares dB slope, actual center-time interval, R², drop and residual RMS are reported when available, even if extrapolation is rejected. Separate first-half and second-half slopes expose a common multistage-decay failure.

An extrapolated T60 additionally requires all of: at least 350 ms fitted span, no ambiguous or capacity-limited observations, frequency range no greater than half an observation-resolution unit, a decreasing slope, at least 5 dB fitted drop, R² ≥ 0.98, residual RMS ≤ 1 dB, decreasing early/late slopes differing by no more than 30% of the overall slope magnitude. These are provisional engineering gates, not published perceptual thresholds. Overlapping windows are correlated; point count is not a count of independent experiments.

`rejection_reasons` distinguishes `sustain_boundary_required`, `insufficient_points`, `insufficient_duration`, `ambiguous_neighbors`, `capacity_limited`, `frequency_variation`, `non_decaying`, `insufficient_drop`, `poor_fit` and `inconsistent_slopes`. A rejected fit has no T60. A short attack may therefore have visible observations and no decay estimate. Noise-floor censoring, interference and preprocessing can bias even an accepted estimate; recorded repetitions and held-out validation are still required.

## Decay estimation

Natural decay is fitted only when `--sustain-end` specifies the end of an uninterrupted sustain region, in seconds from file start. Set it before release, pedal change or a new strike. The regression begins at the later of 100 ms after onset or 50 ms after the envelope peak. Every included RMS window ends before the declared boundary, avoiding contamination by release. Points more than 60 dB below the envelope peak are excluded; this relative cutoff is not a measured noise floor.

The legacy `decay` field exposes the least-squares dB/second slope, R², fitted drop, point count and actual fit interval. An **extrapolated** T60 (`−60 / slope`) is returned only for a decreasing fit with at least ten points, 100 ms span, 5 dB fitted drop and R² ≥ 0.95. It does not mean 60 dB of decay was recorded. This field describes the total RMS envelope. Per-track fits use the separate, stricter qualification above; neither output automatically identifies individual mechanical loss constants.

## Comparison

The waveform `compare` command below is complemented by [partial comparison](PARTIAL-COMPARISON.md): `compare-partials` pairs independent spectral observations in explicit equal-duration regions and reports pitch, raw/level-matched amplitude and qualified decay differences.

Alignment searches ±20 ms by default (allowed 0–100 ms) using signed normalized correlation on the first 150 ms after reference onset. A coarse grid with 16-sample box averages is followed by full-rate local refinement. Positive delay means the candidate starts later. Correlation and a search-boundary flag are reported; this heuristic can choose the wrong period for tonal signals. It does not reverse polarity or stretch time. Use `--align-ms 0` for sample-synchronized renders.

Metrics use the common overlap after alignment; original lengths and overlap frame count remain in the report. Extra non-overlapping tails are not scored. Candidate RMS/reference RMS produces the original level difference. Its reciprocal is the positive gain used only for the separate level-matched metrics. Normalized RMSE is waveform-error RMS divided by reference RMS. It is phase-sensitive and is not a perceptual similarity score.

Log-spectral distance is the RMS dB difference across FFT bins from 20 Hz to below Nyquist, using one Hann observation of up to 32,768 aligned samples beginning at reference onset. The report identifies that observation's offset relative to the overlap. Both spectra use a floor 80 dB below the reference spectral peak. Bins below the floor in both files contribute zero, so broad quiet bands dilute the average. Raw and level-matched distances are reported separately. This single-window metric does not summarize the entire decay or establish an aliasing bound.

## Verification

Tests compare the FFT with a direct DFT, recover known sinusoidal pitch and amplitude, recover a known exponential slope, exclude release from decay windows, preserve null values for silence and missing fundamentals, recover injected delays, preserve gain/polarity differences, and handle leading silence. WAV fixtures cover integer widths, channel selection, float headroom, metadata chunks, truncation and nonfinite samples. CLI tests render, analyze, compare, parse JSON and verify no-overwrite/error behavior.

These tests validate the measurement implementation against known signals. Real-instrument calibration still requires documented direct recordings and held-out validation takes.

Independent-track tests cover noninteger frequencies and unequal exponential decays at 8/44.1/48/192 kHz, off-bin Hann leakage, silence/DC/noise, fading into noise, separated and unresolved beating tones, missing-frame splitting, brief attacks, release boundaries, flat/rising/multistage envelopes, short input and capacity overflow. In the two-component fixture, frequency error must stay below 0.15 Hz and extrapolated T60 error below 0.04 seconds; these isolated-fixture tolerances are not accuracy promises for recorded instruments.

Window-selection tests resolve 700.3/705.7 Hz with less than 0.04 Hz error using 1024 ms, verify that harmonic results remain unchanged, detect a tone occurring beyond the old sample cap at 192 kHz, retain a short attack using 32 ms, recover known decay with 512/1024 ms while excluding release, and reject invalid window values. CLI coverage checks the option, report metadata and duplicate/invalid flags without creating an output file.
