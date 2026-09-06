# Longer stateful modal trajectories

The [incremental midpoint correction](MODAL-MIDPOINT-INCREMENTS.md) reduced the
uniform reference's structural-work drift without eliminating trajectory
differences. This study extends four demanding profiles from 32 ms to 128 ms
and applies accuracy gates to separate attack and tail sections. No DSP
equation, coefficient, integrator tolerance or audible plugin behavior changes.

## Protocol and reproduction

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-tail-check --output renders/modal-tail.json
```

The output path must be a new JSON file. The command retains a completed report
even if a numerical gate fails, and then exits unsuccessfully. The four cases
use tine lengths 75/120 mm, launch speed 0.8 m/s and material relaxation times
1/10 ms. This is the selected strong-strike subset previously extended to 32 ms,
not a whole-keyboard or velocity-range qualification.

Each take retains the same zero-gap impact, core impulse at 2 ms, damper on at
4 ms and off at 6 ms. No additional events occur through 128 ms. Three paths
run per case, for twelve takes:

| Path | Integration |
| --- | --- |
| `rk4_default` | Existing contact/free controller, 16672 base ticks per observation |
| `rk4_contact_2_free_8` | Same controller capped at four contact ticks and 256 free ticks |
| `uniform_20832` | Uniform incremental midpoint at approximately 1.000064 ns |

Observations and mean-force averages remain at 48 kHz. Step rejection, contact
clearance and external-event boundaries are unchanged. All mechanical and
material states persist; the section boundaries do not restart the simulation.
The fine implicit reference remains a finite numerical path, not an exact
continuous solution. This experiment neither models the piano action nor adds
new repetition gestures.

## Independent section gates

Each of the three path pairs must pass the existing comparison thresholds over
the whole 128 ms and separately over four contiguous sections:

| Section | Observation index range, end excluded |
| --- | --- |
| 0–8 ms | 0–384 |
| 8–32 ms | 384–1536 |
| 32–64 ms | 1536–3072 |
| 64–128 ms | 3072–6144 |

Every observation belongs to exactly one section. The full mass-matrix kinetic
velocity RMSE is normalized by the original launch speed and must be below 1%.
Pickup-velocity RMSE uses each section's own reference signal power and must be
below 1%; observation-averaged force RMSE uses the same local normalization and
must be below 2%. There is no alignment, gain fitting or rescaling of the launch
speed during decay. Null relative metrics for zero-power reference signals pass
only for exactly zero difference; a silent reference does not hide candidate force.

All takes also retain global energy and both port-work residual gates of 1e-8,
positive energy-step gate of 1e-10, nonnegative heat/force, free recovery, reimpact
and activation of all nine coordinates. A section cannot override a failed take.
Peak errors and 2 ms windows remain diagnostics, not additional pointwise gates.
Window timestamps are absolute even when calculated within a later section.

A new synthetic regression inserts a core-velocity difference only in the final
64 ms. Its whole-record RMSE passes 1%, but the last section fails, and therefore
the complete comparison fails. The test also verifies contiguous section
coverage, absolute window times and invalid/truncated range rejection. The
laboratory's bounded duration increases to 6144 frames; existing commands retain
their original durations and output schemas.

## Retained results

The [128 ms report](../references/memory-modal-tail-validation.json) passes all
four cases and twelve takes. All twelve whole-record comparisons and all
48 section comparisons pass. Maximum default-RK4 versus finest-reference errors
over the four profiles are:

| Section | Kinetic velocity RMSE / launch speed | Pickup velocity relative RMSE |
| --- | ---: | ---: |
| 0–8 ms | 0.1665% | 0.02064% |
| 8–32 ms | 0.07094% | 0.01273% |
| 32–64 ms | 0.04776% | 0.004491% |
| 64–128 ms | 0.02316% | 0.003236% |

The whole-record velocity RMSE is at most 0.05194%, illustrating how the longer
record reduces the aggregate relative to the attack. The maximum pointwise
kinetic difference is 0.4235%, also in the attack. Mean-force RMSE is at most
0.03025% in the first section. Every later force comparison has exactly zero
reference and candidate force, so its relative metric is null, not a fabricated
zero percentage. These selected tails have no later reimpact.

Default and capped RK4 agree much more closely: maximum section kinetic RMSE is
0.0001134% and whole-record RMSE is 0.00003536%. Capped RK4 versus the finest
reference has maximum section RMSE 0.1664%. This still points to finite-reference
sensitivity as a substantial contributor; agreement between two candidates
sharing a base grid is not independent proof of continuous-time accuracy.

The decreasing maxima across the table do not establish monotonic decay of
every case's difference. In particular, the 75 mm / 10 ms relaxation case reaches
its largest section velocity RMSE in 32–64 ms. Both local and whole-record
diagnostics remain necessary.

Across all twelve takes, maximum relative combined, structural and hammer
work/energy residuals are 4.563e-10, 3.376e-12 and 4.562e-10; maximum positive
relative energy increment is 5.083e-16. The uniform reference's structural
residual remains at most 5.381e-13, while its hammer-work residual grows to
4.562e-10 compared with 7.315e-12 in the earlier 32 ms study. This passes 1e-8
but leaves a duration-dependent reference issue to monitor; the current report
does not isolate its cause. No tolerance was relaxed.

## Regression coverage

All 185 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CLI help, missing/invalid arguments and overwrite protection are checked. The
[short-protocol control](../references/memory-modal-tail-control.json) reproduces
all 24 candidate/reference take reports from the increment-form resolution
study exactly. The only shared laboratory configuration change is the upper
duration bound; the existing commands retain their behavior. CI runs the new
tail command on both native runners. Remote CI, timing and GUI/audio testing
were not performed.

All eight overlapping first-8-ms comparison reports also match the earlier
study exactly, and controller counts account for every base tick in all eight
adaptive takes. The next work is wider gesture/profile coverage and native cost
measurement over these longer paths, while tracking reference work residuals.
There is still no whole-keyboard, multisecond sustain, action/repetition,
perceptual calibration or realtime qualification for this assembly.
