# C5 second-harmonic qualification

**C5 body H2 is stable and clearly separated from local spectral background.
The p/mp attack observations do not meet the exploratory support criteria.**
Removing those two nominal attack terms in a post-hoc sensitivity check does
not make any frozen candidate satisfy all existing numeric limits.

## Method

Only the four C5 development recordings are opened (`C4-*.wav` in the source's
octave convention). Each fundamental is independently qualified with the
existing pitch-anchor method. H2 is searched around twice that frequency.

The new offline `probe_second_harmonic` uses the existing complete-window Hann
FFT, with DC removal, coherent-gain correction and interpolated local peaks.
Unlike the usual snapshot, its local peak search does not discard components
below -60 dB relative to the spectrum maximum. Its absolute amplitude floor
is 1e-12. The ordinary detector's presence/absence is also retained.

Background is measured in two side regions, 3 to 8 observation-resolution
units away from expected H2. The exploratory support rule, implemented before
running the recordings, requires:

- H1 and H2 local peaks present.
- H2 at least 24 dB above median background and 12 dB above its 90th percentile.
- Frequency within the larger of one observation resolution or 0.2% of H2.

These are diagnostic thresholds, not calibrated confidence probabilities or
a substitute for listening. Nearby signal components and Hann sidelobes can
raise the measured background. Failure means the observation lacks support
under this rule, not that the recording contains no H2.

The fixed sensitivity grid has 18 observations per layer, 72 total:

- Attack: starts at detected onset +0/+2/+5 ms, durations 80/96/112 ms.
- Body: starts at onset +230/+250/+270 ms, durations 300/350/400 ms.

All combinations are retained. No negative onset perturbation is used because
the source begins only 1-2 ms before the detected onset. Window changes test
both estimator sensitivity and actual temporal evolution; they are not
independent repeated performances.

## Results

H2 levels below are relative to H1, in dB. Support counts are out of nine.

| Layer | Attack supported | Attack H2 range | Body supported | Body H2 range | Minimum body median / p90 margin |
| --- | ---: | ---: | ---: | ---: | ---: |
| p | 0 | -59.36 to -56.14 | 9 | -57.84 to -57.19 | 32.33 / 26.04 |
| mp | 0 | -52.12 to -50.11 | 9 | -51.76 to -51.21 | 41.72 / 38.59 |
| mf | 8 | -44.05 to -43.85 | 9 | -46.02 to -45.54 | 48.14 / 42.76 |
| f | 9 | -39.41 to -38.67 | 9 | -43.47 to -43.05 | 50.94 / 44.10 |

The ordinary detector reports H2 in all 72 observations, showing why presence
alone does not establish local-background separation. The p attack peak also
wanders from 1049.93 to 1056.52 Hz; its body peak stays at 1047.05-1047.08 Hz.
Every body's level span is below 0.65 dB. The weak body harmonic is therefore
usable as a bounded diagnostic target under this probe, even in p.

## Frozen-fit sensitivity

The nominal windows are onset +0 ms for 96 ms and onset +250 ms for 350 ms.
Only C5/p attack H2 and C5/mp attack H2 fail the nominal support rule.
An explicitly post-hoc calculation removes these cells from both each frozen
candidate and the baseline, then renormalizes within each case. It does not
refit parameters, change production scoring, or amend acceptance gates.

| Frozen model | Aggregate error / baseline after exclusions | Worst note or critical-case ratio |
| --- | ---: | ---: |
| Remapped Calibrated | .9486 | 1.0231 |
| Smooth register | .9194 | 1.1363 |
| Selected Production | 1.2935 | 2.2205 |
| Selected PointPole | .8854 | 1.5047 |

The numeric limits remain aggregate <=.90 and every note/critical ratio <=1.10.
None passes both. The previous no-promotion conclusions are insensitive to
this particular exclusion, although the score values do move.

## Consequence for model work

Use C5 body H2 as the primary high-register diagnostic in the next bounded
geometry/excitation study. Keep G3/f H2 as the opposing-sign constraint and
retain all existing full-score gates. Treat p/mp C5 attack residuals as
uncertain evidence rather than directly tuning the model to their peak level.
This experiment does not establish the correct pickup geometry or physical
strike strength, nor does it remove recording-chain uncertainty.

Follow-up: the [upper-register geometry experiment](UPPER-REGISTER-GEOMETRY.md)
uses this diagnostic and passes the existing frozen validation gates against
the original Calibrated pairing.

No reserved notes were opened. No DSP engine, preset, source recording or
playable package changed, so RackForge audition was not run.

## Verification and reproduction

Three new tests recover a synthetic -64 dB H2 below the ordinary detector's
cutoff, reject fundamental-only leakage with low-level deterministic noise
across six durations, and reject incomplete/invalid observations. The locked
runner passes **35 Rust tests**, release Clippy with warnings denied, and
formatting. The existing Python suites pass **10 tests**.

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
cargo run --locked --release --manifest-path references/matts-reference-runner/Cargo.toml --bin c5_harmonic_probe -- references/audio/matts-fender-rhodes/samples/original renders/matts-reference/c5-h2-new
python references/c5-h2-qualification-2026-09-15/summarize.py
```

The Rust output directory must be new. The Python script replays the retained
observations and frozen-fit sensitivity, without audio processing.
[Receipts](../references/c5-h2-qualification-2026-09-15/) include every probe,
summary, source hashes and provenance. No rendered WAV matrix is produced.
