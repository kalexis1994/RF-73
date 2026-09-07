# Nonlinear magnetic initial-state recovery

This offline study fits the existing nonlinear magnetic voltage laws with one
continuous mechanical state through a known damper event. Mechanical losses,
geometry and magnetic gain are supplied exactly. Only the 18 initial modal
coordinates are estimated. This qualifies a component of a future loss inverse;
it does not yet recover unknown losses from nonlinear voltage.

Run `rf-73-lab magnetic-state --output NEW.json`. The command refuses to replace
an existing output. The retained [receipt](../references/nonlinear-magnetic-state-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-state-v1`: 252573 bytes, SHA-256
`86857559abb5b22d9c43265362f409c486ac30f05a39aa25a8e62c16c7b62f43`.

## Frozen protocol

The six physical trajectories match the [preceding observation study](MAGNETIC-OBSERVATION-LOSS.md):
structural/damper scales `(0.63, 1.37)`, `(1.13, 0.57)` and `(1.47, 0.91)`, each
sampled at 48 and 96 kHz. The default untuned 75 mm tine receives an elastic
single-mass hammer strike at normalized position 0.5. The damper engages at
0.14 s. Mechanical integration uses four ticks per output sample and retains
the original contact and energy checks. Pickup and damper positions are 0.98
and 0.8. Both fitted intervals are contact-free.

Each trajectory supplies both existing magnetic laws (`production` and
`point_pole_proxy`) at three gap/offset pairs: baseline 1.5/0.5 mm, close
0.5/0.25 mm, and centered 1.5/0 mm. These are direct point samples of voltage,
not an antialiased audio rendering.

The initial state at 0.02 s is energy-normalized as `[Omega*z, zdot]` for nine
mass-normalized modes. Known off/on damping operators propagate this same state
through the 0.14 s event. No window gets an independent state. Displacement and
velocity observation templates are prepared for `[0.02, 0.10)` and `[0.14, 0.22)`.
Training uses only `[0.02, 0.06)` and `[0.14, 0.18)`, with each voltage residual
divided by that training window's signal L2 norm and `sqrt(2)`. The remaining
halves are reserved for prediction and state validation.

The nonlinear prediction calls the existing voltage implementation. An analytic
Jacobian differentiates its displacement and velocity dependence and then the
state templates. A rest-linearized least-squares estimate supplies three fixed
starts, scaled by 0.5, 1 and 1.5. No reference state supplies a start.

Levenberg-Marquardt updates normalize Jacobian columns and solve an augmented
system with the existing two-pass QR, without forming normal equations. Damping
starts at `1e-3`, decreases by a factor of five after acceptance (floor `1e-12`),
and increases tenfold after rejection (ceiling `1e12`). Only finite, strictly
descending steps are accepted. The budget is 40 iterations with eight trials
per iteration; training relative RMSE below `1e-9` terminates successfully.
All starts and trial histories are retained. Selection uses training cost only;
held-out samples never update or select the state.

The 12 baseline cases are required positive controls: both held-out voltage
relative RMSEs must be below `1e-6`, and both held-out state energy-norm relative
errors below `1e-4`. The latter measures all 18 scaled coordinates, not only
scalar energy. The 12 close cases are descriptive and scored against the same
limits. All 12 centered cases must retain nonzero voltage and demonstrate
mirror-state voltage relative RMSE below `1e-12`; unique sign recovery is
explicitly withheld. Gates, starts and budgets were fixed before the first run.

## Results

All 24 noncentered selected states recover within the stated limits. All 72
attempts converge, including the close pickup cases, in at most five iterations.
The largest final training relative RMSE across all attempts is `7.025e-10`.
Selected-state maxima across the six trajectories in each group are:

| Law | Geometry | Held-out voltage relative RMSE | Held-out state energy-norm relative error |
| --- | --- | ---: | ---: |
| Production | Baseline | 1.634e-10 | 1.550e-10 |
| Production | Close | 5.799e-11 | 5.555e-11 |
| Point-pole proxy | Baseline | 1.722e-10 | 1.633e-10 |
| Point-pole proxy | Close | 9.133e-11 | 9.180e-11 |

All 12 centered controls confirm the ambiguity. At zero offset both laws obey
`V(-x, -v) = V(x, v)`. The known linear mechanics preserve this mirrored state
through the damper event, so voltage alone cannot select its sign. Additional
strike information could resolve that ambiguity; this experiment does not use
it. Centered cases are not optimized or counted as recovered states.

The laboratory release suite passes 125 tests (90 unit and 35 CLI), with strict
laboratory Clippy and formatting checks. Independent finite differences check
both magnetic Jacobians; direct state propagation checks the switched templates.
The CLI test checks all cases, training-only selection, descending accepted
steps, centered ambiguity and output protection. After correcting Jacobian
column capacity allocation, the affected tests passed again and the regenerated
receipt was byte-identical. No acceptance gate changed.

## Interpretation and next step

The previous linear observer failed on nonlinear voltage while estimating loss
scales. This experiment solves the narrower initial-state problem with the true
losses supplied. Its small residual therefore does not demonstrate unknown-loss
recovery, geometric identification or agreement with a real instrument.

These noiseless, matched-law results establish numerical consistency for the
tested trajectories. Three collinear starts do not establish global uniqueness
or robustness to arbitrary initial guesses. Noise, wrong geometry, wrong field
law, gain error and unknown event timing remain unqualified.

Next measure noise and assumed-geometry/field-law sensitivity before freeing
loss parameters. A future loss search must refit the nonlinear state for every
candidate while preserving continuity and training-only selection. Physical
field validation and antialiasing remain separate requirements before recorded
source calibration. No production equation, preset, audio asset or host process
changed, and no listening validation is claimed.
