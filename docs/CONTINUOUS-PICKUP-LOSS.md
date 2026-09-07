# Loss inference with continuous mechanical state

Carrying one inferred state through the damper event reduces prediction-consistent
but biased fits from 46 to 28 on the existing 102-case position-error grid.
All six matched controls still pass. The constraint rejects 18 additional wrong
fits; it does not establish correct geometry or increase the number of scale
estimates within 1% of truth, which remains 54.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-loss-continuity --output renders/continuous-pickup-loss.json
```

The [receipt](../references/continuous-pickup-loss-validation.json) contains all
102 continuous fits and paired independent-window summaries, experiment
`continuous-pickup-loss-v1`, schema 1, 1090267 bytes. SHA-256:
`cf65badf0289cd4ac127da17aa655f1bc4ee3a741c26b54e85cb69fd7d2c52b6`.
Existing outputs are protected. Failed required controls are retained before
the command returns an error. No WAV or host process is created.

## Fixed experiment and state constraint

The trajectories, off-grid loss pairs, rates, 17 position variants, observation
windows and numerical criteria are those of the
[position-error study](PICKUP-LOSS-POSITION-ERRORS.md). Six physical trajectories
each supply matched, damper-only, pickup-only and combined assumptions, with
longitudinal offsets of 0.075, 0.375 and 0.75 mm. Mass, stiffness, structural
damping and undamped modes remain unchanged across those assumptions.

The new inverse performs these steps using only training observations:

1. Fit structural loss and the 18-coordinate initial state at 0.02 s from
   pickup velocity in [0.02,0.06) s, with the damper off.
2. Freeze both and propagate that state for 0.12 s to the known 0.14 s event.
   No signal samples from the intervening gap or held-out interval are used.
3. Enable the candidate viscous damper without changing position or velocity.
   Search its loss scale using [0.14,0.18) s. Every candidate starts from the
   same propagated state; no on-window initial state is fitted.
4. Predict unused [0.06,0.10) and [0.18,0.22) s intervals.

The interval leading to the event must be free of hammer contact. Integer binary
exponentiation of the prepared sample-step matrix advances to the event. The
modal coordinates use the same frequency scaling on both sides, so continuity
of the state is continuity of physical position and velocity. A purely viscous
damper changes the subsequent evolution without an instantaneous impulse.

This is sequential conditional estimation, not joint optimization of both loss
scales and the initial state. In particular, on-window data cannot revise the
off-window state estimate. The method may be sensitive to errors in that estimate.
The current comparison is noiseless and assumes the event time is known.

Each scalar search retains the same bounds, 17 coarse nodes and 32 refinements
as before. Receipt profiles preserve the selected values, 51-evaluation counts,
coarse nodes and final brackets. The local sensitivity calculation perturbs
each scale by +/-1%, refits only the off-window state and carries it through
the event. It measures sensitivity for this constrained inverse, not the
previous independent-state inverse or a confidence interval.

Prediction consistency still requires both held-out pickup relative RMSEs below
0.005, interior scales, local minimum singular value above `1e-8` and singular
ratio above `1e-4`. Matched controls additionally require both scale errors below
0.1%, both pickup errors below `1e-5` and mechanical checks. Ground-truth scoring
uses the unchanged 1% scale criterion. Nothing was retuned after the first run.

## Paired results

The independent-window method is rerun on each exact same trajectory and assumed
operator, reproducing 100 accepted predictions and 46 accepted-but-biased fits.
The continuous method accepts 82 predictions, including 28 biased fits. Eighteen
previously accepted biased fits are rejected; no previously rejected fit becomes
accepted. All 102 continuous fits complete and all six matched controls pass.

| Family | Absolute offset (mm) | Pairs | Independent accepted but biased | Continuous accepted but biased | Continuous accepted |
|---|---:|---:|---:|---:|---:|
| Matched | 0 | 6 | 0 | 0 | 6 |
| Damper only | 0.075 | 12 | 0 | 0 | 12 |
| Damper only | 0.375 | 12 | 12 | 8 | 8 |
| Damper only | 0.750 | 12 | 10 | 4 | 4 |
| Pickup only | 0.075 | 12 | 0 | 0 | 12 |
| Pickup only | 0.375 | 12 | 0 | 0 | 12 |
| Pickup only | 0.750 | 12 | 0 | 0 | 12 |
| Combined | 0.375 each | 24 | 24 | 16 | 16 |

In matched controls, maximum structural/damper relative errors are `4.686e-9`
and `4.043e-9`; held-out pickup relative RMSE stays below `7.365e-9`. Requiring
continuity does not break the original matched-model recovery.

The worst remaining accepted damper error is about 3.907%, for actual scales
`(1.13,0.57)` and a -0.75 mm damper-position assumption. Its held-out pickup error
is about 0.3514%, still below the existing 0.5% criterion. This residual is more
revealing than the independent method's 0.0953%, but it still does not certify
the inferred loss value.

Pickup-position error is no longer completely absorbed by separate window
states. With a 0.75 mm pickup offset, maximum held-out pickup error rises to
0.1655%, compared with roughly `6.024e-9` relative error for independent fits.
That mismatch is now observable in the residual but all 36 pickup-only variants
still pass prediction. Damper-scale error remains below 0.0108%; full-state
energy-norm error across the two held-out windows still reaches 1.5396%.

The off-window reconstruction is unchanged, so continuity does not retroactively
correct its state error. These results distinguish improved detection of wrong
assumptions from recovery of the correct physical parameters. They do not identify
pickup or damper position, establish mechanical tolerances, or cover noisy data.

## Verification and next step

The laboratory release suite passes 119 tests (86 unit, 33 CLI), with strict
laboratory Clippy and formatting checks. New tests compare switched damping
against analytic piecewise oscillator motion at both rates, verify that every
damper candidate starts from the same state, retain all 102 paired variants,
reproduce independent-method counts and protect existing output files.

Keep continuity as a physical constraint while treating the geometry as uncertain.
Next qualify the existing [magnetic transfer](PICKUP-TRANSFER.md) on these
mechanical trajectories, measuring how its nonlinear voltage observation affects
the linear inverse. The remaining ambiguity must stay explicit before fitting
recordings; state continuity alone does not validate physical loss coefficients.

No production solver, preset, plugin engine or audio asset changed. This is an
offline inference experiment, not a new audible version or a listening test.

The subsequent [magnetic observation study](MAGNETIC-OBSERVATION-LOSS.md) applies
the existing voltage laws to these trajectories. All 30 mechanical/linearized
controls pass, but all 24 full nonlinear voltage fits fail the linear inverse's
prediction and loss-recovery criteria. Centered zero-slope cases are withheld.
