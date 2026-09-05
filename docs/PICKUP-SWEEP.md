# Pickup geometry sweep

`sweep-pickup` is a bounded offline experiment against a mono or explicitly selected WAV channel. It renders a grid of provisional pickup gaps and offsets, then ranks their spectral errors. It writes one JSON report and keeps candidate audio in memory. It does not apply a profile to the instrument.

For multiple takes with one shared gain and a separate validation split, use [Pickup reference set](PICKUP-SET.md). The single-take report also exposes `applied_candidate_gain` and `applied_gain_normalized_rmse`; these equal its existing per-take matching gain and matched NRMSE.

## Reproduce a known-geometry experiment

Use fresh output paths:

```text
cargo run --locked --release -p rf-rhodes-lab -- render --output renders/known-pickup.wav --note 57 --velocity 0.7 --seconds 1.5 --hold 1.4 --gap-mm 2 --offset-mm 0.75
cargo run --locked --release -p rf-rhodes-lab -- sweep-pickup renders/known-pickup.wav --output renders/pickup-sweep.json --note 57 --velocity 0.7 --seconds 1 --reference-start 0.1 --model-start 0.1 --gaps-mm 1.5,2,2.5 --offsets-mm 0.5,0.75,1
```

The reference contains the known geometry on the search grid. Recovering it tests the implementation, not resemblance to an acoustic instrument. See [Validation results](VALIDATION.md) for the two-intensity experiment.

For a recorded instrument, replace the input path and supply the corresponding note, model strike velocity and region positions. Use `--channel 0` when selecting the first channel of a multichannel file. Document the instrument, recording chain and strike procedure using the [measurement protocol](MEASUREMENTS.md). Normalized model velocity is not a measured hammer speed and cannot be inferred from a recording's peak level.

## Inputs and bounds

| Input | Requirement |
| --- | --- |
| `--note` | Required, MIDI 28..100 |
| `--velocity` | Required, finite normalized strike 0.01..1 |
| `--seconds` | Required, selected duration 0.128..4 seconds |
| `--gaps-mm` | Required, 1..5 distinct finite values in 0.5..5 mm |
| `--offsets-mm` | Required, 1..5 distinct finite values in -3..3 mm |
| `--reference-start` | Nonnegative seconds from file start; default 0 |
| `--model-start` | 0..2 seconds from the model strike; default 0 |
| Sample rate | Reference must be 44100, 48000, 96000 or 192000 Hz |
| Output | Required new `.json` path; never overwrite |

The WAV reader also limits input to 60 seconds and 12 million frames. Starts and duration round to samples, with actual frame counts in the report. The complete requested region must exist. No automatic alignment, resampling, padding or truncation is performed. Select corresponding held-note regions before release: the candidate holds its key throughout, with no pedal or repeated strike.

At most 25 candidates run sequentially. Each starts from a fresh engine with the current default hammer, resonator and decay parameters, changing only gap and offset. The report records the fixed profile. Numerical faults, nonfinite output and insufficient candidate energy reject that candidate with a reason. An invalid or silent reference fails before rendering. If every candidate fails, a diagnostic report is saved and the command exits unsuccessfully.

## Objective and report

For each candidate, compute one gain `g = RMS(reference) / RMS(candidate)` over the entire selected region. Apply that same positive gain to every scoring window. This removes a constant recording-level difference while retaining time-varying level and decay errors. Polarity is not corrected.

Requested windows span 128 ms with 64 ms hops; a final full window covers the tail when it does not land on the regular grid. Every measured window contributes equally. Each uses the existing comparison spectrum: DC removal, symmetric Hann, coherent-gain correction and a zero-padded FFT. Observation begins at the reference window's 1%-of-peak onset, leaving at least 128 samples; both signals use that same start. Requested windows fit below the legacy 32768-sample cap at all supported rates. Each row reports its requested sample count, actual spectral start relative to the selected region and actual observed samples.

The per-window distance is the RMS difference between magnitude spectra in dB, over FFT bins from 20 Hz to below Nyquist, with both magnitudes floored 80 dB below that reference spectrum's maximum. `objective_db` is the square root of the mean squared window distances after the global gain correction. `raw_objective_db` uses the same process without that correction. Silent reference windows have null errors and are excluded; `scored_windows` reports how many contribute. Missing spectral energy also yields null errors. Inspect coverage before interpreting a ranking.

Schema 1 includes all candidates in input order, `ranking_indices` from lowest error to highest, `best_candidate_index`, and `near_best_candidate_indices`. Exact score ties preserve input order. The near-best band is 0.01 dB above the lowest objective: an engineering reporting threshold, not a confidence interval or audibility threshold. Original RMS difference, the normalization gain, raw and level-matched waveform NRMSE and every window error remain available for inspection. Waveform NRMSE is diagnostic, not the ranking objective.

## Interpretation and next experiment

The additive `selection_limits` field reports the selected geometry's `gap_grid_min/max`, `offset_grid_min/max`, `gap_profile_min/max` or `offset_profile_min/max` flags. A single-value axis receives `gap_fixed` or `offset_fixed` instead of a grid-edge flag. Null means no candidate was selected; an empty list means an interior selection on both axes. The same diagnostics are included in `fit-pickup-set`. They are bounds checks, not uncertainty estimates.

This magnitude-spectrum objective is not a perceptual metric. Quiet bins can dilute its average; its floor can conceal weak partials, and the onset and Hann window reduce sensitivity to brief attacks. Results depend on sample rate, region, strike velocity and the chosen grid. Use [partial comparison](PARTIAL-COMPARISON.md), attack observations and listening to inspect residuals, rather than accepting the lowest scalar error alone.

A grid minimum is not a continuous optimum. Similar geometries can be indistinguishable after gain matching; a minimum at a grid edge calls for a revised range. Model gap and offset are provisional parameters, not identified physical measurements. A wrong hammer, modal profile, tuning or recording chain can shift their apparent optimum.

The next calibration experiment should use a documented dry A3 reference, constrain its strike consistently, select geometry on one intensity and evaluate the chosen geometry on held-out intensities without retuning. A4 then supplies a second anchor. Joint fitting of mechanical parameters and measured pickup response remains separate work. No untreated reference instrument has been calibrated by this command yet.
