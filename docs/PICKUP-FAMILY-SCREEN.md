# Retained pickup law screen

Date: 2026-09-15. **No evaluable configuration passes all development gates.**
This offline study compares the existing Production and
PointPole formulas under the same Calibrated mechanics and ordinal strike
mapping freedom. It does not change a plugin preset.

## Controlled comparison

Each law gets four configurations: gap 0.5/1.0 mm crossed with maximum hammer
speed 0.8/1.6 m/s. Offset stays at 0.75 mm, velocity exponent at 1.4, decay at
20 s and bar decay at 2.3 s, with Calibrated bar excitation. These are not
measurements of the Original preset, which uses different mechanics.

For each configuration, the common grid is 19 velocities from 0.10 to 1.00
in steps of 0.05. The selector exhausts 3,876 strictly increasing p/mp/mf/f
mappings, shared across all seven development notes. Each configuration gets
its own selected mapping. This treats undocumented source strike strength as
a fitting variable, not a recovered physical or MIDI velocity law.

Development uses MIDI 43/47/50/55/59/64/72, four source layers each. The
objective remains H2-H4 squared dB error relative to H1 over attack/body
windows, equally weighted by case. The baseline is current Calibrated at
0.25/0.45/0.65/0.85. Each note and the B2/mp and E4/f critical cases must stay
within 1.10 times baseline error; aggregate error must fall to at most 0.90.
Selection minimizes the worst normalized constraint violation until feasible,
then aggregate error. An aggregate-only winner is not the selected winner.

## Results

Seven complete grids yield **27,132 mapping evaluations**. One configuration
cannot complete the common grid and is excluded as described below.

| Law / gap / maximum speed | Selected p / mp / mf / f | Aggregate MSE ratio | Worst note or critical ratio |
| --- | --- | ---: | ---: |
| Production / 0.5 mm / 0.8 m/s | .50 / .75 / .85 / 1.00 | 1.3157 | 2.5520 |
| Production / 0.5 mm / 1.6 m/s | .30 / .45 / .55 / .85 | 1.2909 | 2.2205 |
| Production / 1.0 mm / 0.8 m/s | .55 / .70 / .95 / 1.00 | .7528 | 3.6344 |
| Production / 1.0 mm / 1.6 m/s | .40 / .55 / .65 / 1.00 | .6993 | 2.6369 |
| PointPole / 0.5 mm / 0.8 m/s | .30 / .65 / .90 / .95 | 1.5941 | 3.7172 |
| PointPole / 0.5 mm / 1.6 m/s | Not evaluable | — | — |
| PointPole / 1.0 mm / 0.8 m/s | .40 / .70 / .75 / 1.00 | .8906 | 1.5047 |
| PointPole / 1.0 mm / 1.6 m/s | .20 / .40 / .60 / .80 | .9210 | 1.5565 |

The selected Production configuration is 0.5 mm / 1.6 m/s, with the smallest
constraint violation in that family. Its error rises 29.09% overall and
122.05% on G3. The selected PointPole configuration is 1.0 mm / 0.8 m/s:
aggregate error falls 10.94%, but B2/mp rises 50.47% and D3 mean rises 39.22%.
Neither is a promotion candidate. Even the Production configuration with a
30.07% aggregate reduction regresses D3 mean by 163.69%.

Reserved MIDI 45/53/60/67/76/86 were not loaded. No envelope validation or
listening result is claimed. Plugin code and source audio are unchanged; no
playable version was produced, so RackForge audition was not invoked.

Together with the preceding [velocity study](VELOCITY-MAPPING-SENSITIVITY.md),
this does not support replacing Calibrated with one of these fixed profiles.
The next useful diagnostic is to separate residual errors by harmonic and
attack/body window in the already retained spectra before broadening the
parameter search. These results alone do not identify whether excitation,
geometry, or reference performance explains the mismatch.

## Measurement fixes and exclusions

The first attempt stopped because the old 0.7 s render did not cover a full
body window after a late detected onset. Short renders now last 0.9 s: the
detector searches the first 250 ms, and the body window ends 600 ms after its
anchor. The measurement windows and detector thresholds are unchanged. A
delayed synthetic signal tests this previously failing path. Receipts now
include candidate onset times in the grid and both onset times in selected
spectra.

The second attempt exposed an onset-detection failure at velocity 1.00 for
PointPole, 0.5 mm gap, 1.6 m/s. The completed third run records that entire
configuration as **not evaluable**, without selecting from a partial velocity
grid. Unexpected errors still abort the run. This is a limitation of the
comparison pipeline, not evidence that the formula is physically impossible.

The detector requires four consecutive 1 ms RMS bins above 1% of the peak
within its search interval. Highly impulsive output can delay or defeat that
criterion. A detected offset is therefore not necessarily physical latency.
All reported spectral scores depend on this alignment rule.

The five complete grids from the first attempt reproduce exactly after the
duration fix, including every spectral cost and selected mapping. Both final
family winners have detected candidate onset 0 s for every selected case.
Across the seven complete grids, the largest detected onset is 4 ms. The late
anchor occurs within the excluded configuration, so it does not explain the
accepted grids' constraint failures.

## Verification and reproduction

The adapter matches the production Engine waveform with level compensation
disabled (relative RMS difference below 1e-7). PointPole agrees with an
independent finite difference of its flux equation. The standalone runner
passes 32 Rust tests; the gate suite passes six Python tests. Release Clippy
passes with warnings denied.

An independent Python enumeration reproduces all seven selected mappings and
their scores from the retained costs. Build storage is approximately 377 MB;
incremental compilation stays disabled.

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
cargo run --locked --release --manifest-path references/matts-reference-runner/Cargo.toml --bin continuous_pickup_fit -- --families references/audio/matts-fender-rhodes/samples/original renders/matts-reference/families-new
python references/pickup-family-screen-2026-09-15/replay.py
```

[Receipts](../references/pickup-family-screen-2026-09-15/) retain costs,
selected spectra, the excluded configuration, independent selection replay,
and source/receipt hashes. No WAV matrix is written. The standalone locked
runner avoids the existing workspace SDK dependency mismatch.

This bounded screen has only four fixed configurations per law. It cannot
establish a global optimum, rule out other geometry, or establish an audible
preference. Source capture gain and actual strikes remain unknown.
