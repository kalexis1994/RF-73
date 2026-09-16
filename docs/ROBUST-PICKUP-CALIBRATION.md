# Constraint-first pickup calibration

Date: 2026-09-15. **No candidate passed the development constraints.** The
plugin is unchanged, and the new held-out split remains unused by this study.

## What changed

The preceding continuous-disk fit reduced average error while regressing E4/f
and B2/mp. The new `--robust` mode makes those failures part of selection, not
just observations after selection. It shares the existing continuous pickup,
mechanics, spectral scorer and interpolation qualification.

Development uses MIDI 43/47/50/55/59/64/72, each at p/mp/mf/f: 28 cases. The
candidate must satisfy all three conditions relative to the current Calibrated:

1. At least 10% reduction in the overall mean harmonic MSE.
2. No development note's four-layer mean may increase by more than 10%.
3. Neither E4/f nor B2/mp may increase by more than 10% individually.

Until a feasible candidate exists, selection minimizes the largest normalized
constraint violation. Among feasible candidates, it minimizes the overall mean
ratio. A violation of 1.0 or less passes. No weighted mean can override a failed
constraint. These are engineering selection criteria, not perceptual thresholds.

## Bounded search

- Coarse phase: six deterministic seeds and four coordinate rounds with log
  steps 0.4/0.2/0.1/0.05, totaling 46 evaluations.
- Refinement: the coarse winner, two coordinate rounds at 0.025/0.0125, and
  all signed pairs of coordinates at 0.05/0.025, totaling 101 evaluations.
- Total: **147 evaluations**, including seed reevaluation and bound-clamped
  trials; this is not a count of distinct geometries.
- The second phase was chosen after inspecting only development results. Its
  schedule was fixed before running it; neither phase loaded the reserved
  MIDI 45/53/60/67/76/86 samples.
- Variables and bounds are unchanged from the continuous-disk study. This is
  a local search, not a proof that the entire parameter family is infeasible.

## Result

The normalized constraint violation falls from 1.2115 at the end of the coarse
phase to **1.1577** after refinement, still above the acceptance limit of 1.0.

| Measure | Current MSE (dB²) | Selected MSE (dB²) | Change |
| --- | ---: | ---: | ---: |
| E4/f critical case | 179.63 | 117.48 | -34.60% |
| B2/mp critical case | 19.75 | 24.84 | **+25.77%** |
| B2 four-layer mean | 98.76 | 122.23 | **+23.76%** |
| G3 four-layer mean | 66.13 | 84.21 | **+27.35%** |
| Overall development mean | ratio 1.0 | ratio 0.9470 | -5.30% |

The E4/f regression is reversed within development, but the B2 and G3
constraints still fail. The overall reduction also misses 10%. Therefore no
held-out spectral/envelope evaluation or plugin release was attempted.

Selected continuous-disk parameters:

| Parameter | Value |
| --- | ---: |
| Gap | 0.963194 mm |
| Lateral offset | 1.000000 mm |
| Radius | 0.588224 mm |
| Maximum hammer speed | 1.447740 m/s |
| Velocity exponent | 1.221403 |

The table passes its existing 2,001-point check: maximum interpolation error
over peak slope is 3.9583e-6, and 128-to-256-node refinement error is 1.6771e-15.
This rules out a large table approximation error at the selected geometry;
it does not establish realism or excuse the measured regressions. The rejected
candidate was not subjected to another direct-render or release qualification.

## What the history supports

The [per-note diagnostic](../references/robust-pickup-fit-2026-09-15/note_diagnostics.json)
selects each note's lowest observed error from the retained trials, without
additional audio renders. Different notes favor different parameter sets.
G3 is especially resistant: even its best observed note mean is 1.021 times
Calibrated's error, while several other notes improve substantially.

This supports investigating whether a smoothly varying geometry by register
helps. It does not establish that interpolation will work, that parameters are
identifiable from these recordings, or that per-note corrections are justified.
A register-dependent model must still pass fresh held-out measurements. The
source velocity labels remain ordinal, and the recording chain is not known
well enough to equate absolute gain with hammer speed.

## Reproduction and verification

From the workspace in PowerShell, using new output directories:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$runner = 'references/matts-reference-runner/Cargo.toml'
$samples = 'references/audio/matts-fender-rhodes/samples/original'
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --robust $samples renders/matts-reference/robust-new
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --robust $samples renders/matts-reference/robust-refined-new renders/matts-reference/robust-new/search.json
```

Refinement rejects a seed receipt whose held-out split was already loaded.
Only development-feasible, numerically qualified winners proceed to the usual
long-window split evaluation. Such a run still needs `matts_fit_gate.py` and
direct-render numerical checks before any release decision; feasibility alone
is not permission to promote a preset.

[Retained receipts](../references/robust-pickup-fit-2026-09-15/) include both
protocols, all trial constraints, selected full development observations,
explicit no-promotion decisions, per-note diagnostics and source hashes.

Validation: **25 Rust tests and six Python gate tests pass**, release Clippy
passes with warnings denied, and formatting passes. Five new tests cover
critical-case regressions hidden by averages, noncritical note regressions,
feasible-candidate priority, missing critical evidence, and the minimum global
improvement. An initial test build encountered Windows locking the running
research executable; the complete test suite passed after that run finished.
No process was terminated. Reused build storage is approximately 147 MB;
new retained receipts are approximately 0.52 MB. No WAV matrix was created.
