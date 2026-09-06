# Temporal component envelopes

The laboratory can now measure the amplitude envelope and phase of a supplied
frequency in a WAV. A controlled temporal study recovers known decay rates in
clean, noisy, neighboring-tone and sideband mixtures, and retains explicit
rejections for unsuitable observations. This connects the
[phase-resolved pickup study](PICKUP-DECAY.md) to ordinary sampled audio without
assuming every measured component is a mechanical mode.

## Commands

```text
cargo run --locked --release -p rf-73-lab -- validate-envelope --output renders/envelope-validation.json
cargo run --locked --release -p rf-73-lab -- component-envelope INPUT.wav --output renders/envelope.json --frequency-hz 1426.7578125 --start 0.1 --end 1.5 --window-ms 128 --hop-ms 32
```

The second example's frequency and interval are illustrative synthetic values,
not selected source-recording measurements. `--start` and `--end` are mandatory
observation boundaries in seconds from file start. They do not establish an
uninterrupted natural sustain. `--neighbors-hz F1,F2` supplies independently
identified nearby components; an empty list is not proof of isolation.
Multichannel WAVs require explicit zero-based `--channel`. Native samples, gain
and rate are retained. Reports must be new `.json` files.

The measurement command writes rejected observations successfully, including
their reasons and provisional fit. Consumers must inspect `qualified` before
using a rate. The validation command exits unsuccessfully if an expected result
fails, retaining its receipt. Neither command modifies input audio, produces
playback or changes the plugin.

## Estimator and limits

A symmetric Hann window removes its own weighted mean and projects the signal
onto a fixed complex carrier. Peak normalization is `2/sum(weights)`. Rotation
by the window's absolute start time removes the carrier globally, retaining
phase relative to the source time origin. The report stores each actual center
time, complex coefficient, amplitude, unwrapped phase and local background.

Ten guard projections lie at offsets `+/-4..8` times `sample_rate/window_size`.
Their median is a local spectral-background heuristic, not a calibrated
broadband noise RMS or a guarantee against leakage. A strong undeclared neighbor
or a narrow interferer can evade it. The absolute numerical guard floor is
`1e-12` amplitude.

Free least-squares fits of dB amplitude and unwrapped phase against time yield
the amplitude-decay rate and carrier-frequency offset. The rate is
`-slope_db_per_second*ln(10)/20`. Separate early/late fits and both residuals
check stability. No expected rate is supplied to the fit. Finite Hann windows
bias absolute amplitude and phase; with one constant-rate exponential and a
constant carrier error this largely acts as a fixed complex scale. The synthetic
tests qualify rate and phase slope, not unbiased instantaneous amplitude for
arbitrary transients. Absolute phase comparisons across different decay rates
or windows still require accounting for that bias.

The following fixed gates are part of `fixed-carrier-hann-envelope-v1`:

| Condition | Requirement |
|---|---|
| Observation support | At least 8 complete windows and 0.4 s between first/last centers |
| Known frequency separation | At least `2*sample_rate/window_size` Hz |
| Local margin | Every point at least 18 dB above guard background and at least -180 dBFS |
| Input level | No sample at or above full scale in the selected interval |
| Decay | At least 3 dB fitted drop |
| Amplitude residual | At most 0.5 dB RMS |
| Early/late rate difference | At most `max(0.2 /s, 0.1*abs(fitted_rate))` |
| Carrier offset | At most 0.5 Hz absolute |
| Phase residual | At most 0.05 radians RMS |

Windows are 64..512 ms; hop is between one eighth and one full window. Work is
bounded to 128 windows and 8 million window-sample observations, with at most
32 declared neighbors. Carrier guards must clear DC and Nyquist by nine
resolution units. Incomplete windows are excluded rather than padded; too-short
input or excessive work is an error. Insufficient fit support yields no fit.
All actual integer-sample window/hop durations are retained.

**Qualification is conditional, not proof of an isolated mode.** A regression
test deliberately mixes two tones only 0.05 Hz apart: the observation passes
when the neighbor is undeclared, and is rejected when it is declared. A smooth
unresolved mixture can resemble one decaying component. Overlapping windows
are correlated; there is no confidence interval, automatic source identity,
natural-loss calibration or T60 extrapolation.

## Controlled temporal study

The retained [receipt](../references/component-envelope-validation.json) uses
`temporal-component-envelope-validation-v1`: nine deterministic two-second
waveforms at 48 kHz, each measured with 128 and 256 ms windows and quarter-window
hops over 0.1..1.5 s. Waveforms are generated in Rust memory, not stored as WAVs.
The target is 1426.7578125 Hz except for the 1623.046875 Hz sideband-mixture case.
Base amplitude is 0.2 and initial phase is 0.73 radians.

| Probe | Prescribed change | Expected result |
|---|---|---|
| Clean | Amplitude decay 3 /s | Accept |
| Moderate noise | Uniform noise amplitude 0.001 | Accept |
| Resolved neighbor | +40 Hz, amplitude 0.18, decay 0.7 /s | Accept |
| Known unresolved neighbor | +4 Hz, amplitude 0.18, decay 0.7 /s | Reject neighbor |
| Low margin | Target amplitude 0.01, uniform noise amplitude 0.02 | Reject margin |
| Stationary | Zero decay | Reject insufficient decay |
| Changing decay | 1 /s before 0.7 s, then 6 /s continuously | Reject inconsistent slopes |
| Wrong carrier | True frequency 2 Hz above supplied carrier | Reject frequency mismatch |
| Sideband mixture | Target decay 5.8 /s among stronger parent tones | Accept |

The sideband mixture contains prescribed sinusoids at the prior study's parents
and their sum/difference. It is not a new nonlinear pickup render. Its target
amplitude is 0.04, lower sideband 0.03, fundamental 0.5 and upper parent 0.2;
their amplitude-decay rates are respectively 5.8, 5.8, 0.8 and 5 /s. All known
companions are declared. Noise uses one fixed LCG64 sequence per probe, seed
`0x735eed`; this is reproducible coverage, not a statistical noise study.

All 18 expectations pass on 2026-09-06. Accepted rates are required to be within
0.05 /s of truth. Actual absolute errors are:

| Accepted probe | 128 ms error /s | 256 ms error /s |
|---|---:|---:|
| Clean | 8.37e-12 | 1.40e-10 |
| Moderate noise | 0.000479 | 0.000339 |
| Resolved neighbor | 0.000460 | 0.000635 |
| Sideband mixture | 0.00000180 | 0.00000273 |

Rejected cases retain all observations and may have several simultaneous reasons.
Longer windows reduce residuals in the accepted neighbor mixture, but do not
necessarily reduce fitted-rate error monotonically. Extremely small clean-case
errors concern these analytic probes only.

Unit tests also check 44.1/48/96 kHz native rates, a 0.2 Hz carrier offset, gain
invariance, absolute phase in the matched-carrier probe, silence, full scale,
short support, invalid/bounded work and the undeclared-neighbor limitation.
The CLI regression checks the entire validation command, an independently
encoded PCM16 WAV, input preservation and malformed/duplicate/existing outputs.

## Next source work

Apply the estimator to a bounded set of pinned source families, retaining exact
frequencies, byte identities, observation intervals, known neighbors and both
window lengths. Frequency mismatch, weak tails, changing slopes and unknown
release boundaries must remain visible; do not relax gates to force a decay
fit. Compare qualified envelope observations with the pickup mixing predictions
before selecting mechanical losses. No physical parameter or audible baseline
has changed in this stage.

The subsequent [G3 source pilot](SOURCE-ENVELOPES.md) retains all five takes and
finds one fundamental with qualified cross-window agreement. All available
1425/1620 Hz family envelopes reject over the fixed interval; their rate relation
is withheld. Limited initial detectability motivates independent validation of
a short-transient estimator before further source fitting.
