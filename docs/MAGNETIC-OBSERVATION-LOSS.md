# Magnetic voltage observed by the linear loss inverse

This study applies the existing magnetic laws to the continuous mechanical
trajectories and tests the existing linear inverse against that observation.
It does not implement a nonlinear magnetic inverse or select a validated
real-instrument field law.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- magnetic-pickup-loss --output renders/magnetic-observation-loss.json
```

The output must be a new JSON file. Every observation and fit failure is retained;
failed required controls are written before returning an error. No WAV is written.

## Protocol fixed before the first run

The six matched trajectories are those of the
[continuous-state loss study](CONTINUOUS-PICKUP-LOSS.md): three off-grid loss
pairs `(0.63,1.37)`, `(1.13,0.57)`, `(1.47,0.91)`, each at 48/96 kHz, default
untuned geometry, elastic single-mass hammer and a known 0.14 s damper event.
Longitudinal pickup and damper positions remain 0.98 and 0.8 of tine length.
All magnetic variants read the same mechanical displacement and velocity.

The existing production law and research point-pole proxy are reused unchanged.
For `z=(offset+x)/gap` and arbitrary scale `C=0.015`, their voltage proxies are:

```text
Production: V = C*z*v / [gap*(1+z*z)^(3/2)]
Point pole: V = 3*C*z*v / [gap*(1+z*z)^(5/2)]
```

See [pickup transfer](PICKUP-TRANSFER.md) for provenance and limitations. Neither
law has independently calibrated voltage gain, a finite pole surface, electrical
circuit, magnetic loading or two-dimensional motion in this experiment.

Each law uses three `(gap, lateral offset)` pairs in millimeters: `(1.5,0.5)`,
`(0.5,0.25)` and centered `(1.5,0)`. These dimensions concern the magnetic law,
not longitudinal placement or tuning-spring position. The 11 observations per
trajectory are:

- One mechanical-velocity reference.
- Four rest-linearized voltage controls, one for each noncentered law/geometry.
- Four corresponding full nonlinear voltage observations.
- Two centered nonlinear observations with zero rest sensitivity.

The assumed linear observation is `H=k0*b^T`, where `k0=V(0,1)`. Linear controls
observe `k0*v`; nonlinear cases observe `V(x,v)`. The inverse receives neither
true displacement nor an instantaneous displacement-dependent gain correction.
Its fitted states remain in mechanical energy coordinates, so their errors can
be scored against the original trajectory even when the observation has changed.

With the pickup centered, `k0=0` for both laws, so this linear observer is
unobservable. Those cases must be explicitly withheld before fitting losses.
This does not mean the nonlinear voltage is silent or that every possible
nonlinear observer would fail.

The continuous-state constraint, sequential bounded loss searches, QR fitting,
fit/held-out windows and numerical gates are unchanged. Thirty positive controls
require both scale errors below 0.1% and both held-out observation relative RMSEs
below `1e-5`, plus the original prediction and mechanical checks. Twelve centered
controls require explicit withholding. The 24 noncentered nonlinear outcomes
remain descriptive under the existing 0.5% prediction and 1% scored-loss criteria.

Forward diagnostics accumulate over every mechanical tick at 192/384 kHz in each
80 ms window: voltage RMS, relative error of `k0*v`, peak displacement/gap ratio,
and fraction of samples whose sensitivity reverses sign relative to rest. They
include fit and held-out portions and are used only for reporting, never selection.

The inverse consumes direct point samples at 48/96 kHz. Nonlinear harmonics can
alias; these are not antialiased audio signals, and comparing the two rates is
not an antialiasing proof. No production FIR or resampling chain participates.
This scope isolates the observation assumption without qualifying host audio.

## Results

The [receipt](../references/magnetic-observation-loss-validation.json), experiment
`magnetic-observation-loss-v1`, schema 1, contains 66 observations in 616210 bytes.
SHA-256: `a846f053c157154bf488d90fea007506241c9ade96ef05c42b10acbfd1e79480`.
All 30 positive controls pass; all 12 centered observations are explicitly
withheld and have nonzero nonlinear voltage RMS. All 24 noncentered nonlinear
fits complete, but none passes prediction consistency or recovers both scales
within 1%. These are rejected inverse-model results, not solver failures.

The positive controls retain maximum structural/damper relative errors of
`4.686e-9` / `4.043e-9` and maximum held-out observation relative RMSE of
`7.365e-9`. Changing observation units by the correct constant rest sensitivity
does not break recovery. The failure appears when the actual nonlinear voltage
is supplied to an observer whose observation law remains linear.

The table gives maxima across six rate/scale cases per row. Relative errors are
dimensionless fractions; each maximum need not belong to the same trajectory.

| Law | Geometry | Structural scale error | Damper scale error | Held-out observation relative RMSE | Forward rest-linearization relative RMSE |
|---|---|---:|---:|---:|---:|
| Production | Baseline | 0.407806 | 0.074599 | 1.244360 | 0.270092 |
| Point-pole proxy | Baseline | 0.347361 | 0.077730 | 0.924960 | 0.233069 |
| Production | Close | 0.829932 | 0.221025 | 3.414127 | 0.609996 |
| Point-pole proxy | Close | 0.829932 | 0.234035 | 2.676195 | 0.695707 |

In baseline geometry, peak displacement reaches about 0.290 of the gap in these
windows. The close geometry reaches about 0.870, with sensitivity opposite to
its rest sign during up to 19.36% of a window's mechanical samples. Even without
a sign reversal, the baseline laws differ substantially from a constant gain.
The centered zero-slope controls are not assigned fitted losses; the sign-reversal
fraction relative to a zero rest slope is degenerate and must not be interpreted
as absence of magnetic sign changes.

Some close-geometry fits reach the structural search's lower bound of 0.25,
explaining the retained relative error near 83% for true scale 1.47. The searched
range and gates were not widened to accommodate the mismatched observation.
The largest held-out observation relative RMSE exceeds 3.4: a fit to the early
waveform can predict the later nonlinear signal very poorly with frozen state.

All noncentered nonlinear variants fail at both rates. This does not quantify the contribution
of aliasing or validate either magnetic law against an instrument. It establishes
failure of this linear observation assumption for these point-sampled histories.
The forward diagnostics, sampled more densely, independently show that the
instantaneous rest-gain approximation is inadequate at the tested motion levels.

## Verification and next step

The laboratory release suite passes 122 tests (88 unit, 34 CLI), plus strict
laboratory Clippy and formatting checks. New tests check the small-motion limit,
nonzero centered voltage, zero-slope withholding before any data fit, all 66
observations, the 30 positive controls, observation units and output protection.
The rest-slope equivalence check allows eight machine epsilons because the
preserved production operation order can differ from `V(0,1)*v` by roundoff.
No experimental acceptance gate was relaxed.

Next implement and qualify a nonlinear observation fit using the existing
magnetic law, first with known mechanics and sensor geometry. Keep one continuous
state and test initial-state sensitivity, sign/branch ambiguities and held-out
prediction before freeing loss parameters. Numerical correctness of that inverse
will still not identify the real field law; physical geometry, magnetic model
validation and antialiasing remain separate requirements before source calibration.

No production magnetic equation, mechanical solver, preset or audio asset changed.
This is an offline observation study, with no host launch or listening claim.

The follow-up [nonlinear state study](NONLINEAR-MAGNETIC-STATE.md) recovers all
24 noncentered states with the true losses and geometry supplied. All 72 fixed
start attempts converge; 12 centered cases retain the sign ambiguity. This
qualifies the nonlinear state fit, not unknown-loss recovery. Next test noise
and sensor-model mismatch before freeing loss parameters.
