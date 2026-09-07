# Nonlinear magnetic profiling of unknown mechanical losses

This experiment estimates two unknown dimensionless damping multipliers from
nonlinear pickup voltage. The structural scale multiplies the existing viscous
matrix `C0`; the damper scale multiplies `D` after the known 0.14 s event.
Unlike the preceding nonlinear state studies, neither true scale is supplied
to the inverse. Mass, stiffness, event timing and the production pickup law and
baseline geometry remain known. The nine-mode physical model is unchanged.

Run `rf-73-lab magnetic-loss-profile --output NEW.json`. Existing outputs are
protected. The report retains every evaluated loss pair, compact outcomes for
all state starts, both outer search histories and final held-out validation.

## Protocol fixed before the first run

Six positive controls use the existing reference pairs `(0.63, 1.37)`,
`(1.13, 0.57)` and `(1.47, 0.91)`, each at 48 and 96 kHz. A seventh, negative
control uses structural scale 2.2 and damper scale 0.91 at 48 kHz; its structural
loss is outside the fixed search range. The baseline production pickup uses
1.5 mm gap and 0.5 mm transverse offset. No noise or geometry mismatch is added
in this initial inverse qualification.

The same physical simulator generates an elastic hammer strike on the default
untuned tine, with the damper engaging at 0.14 s. It integrates at four ticks
per output sample and retains the contact and mechanical-ledger checks. Both
observation windows, `[0.02,0.10)` and `[0.14,0.22)`, are contact-free. Their first
halves supply training; their second halves are held out. Initial state lives at
0.02 s and propagates continuously through the event with no reset.

Each loss candidate rebuilds the off/on operators and displacement/velocity
templates. It then fits all 18 initial modal coordinates jointly to the two
training windows using the existing nonlinear voltage law and state optimizer.
Each candidate restarts from its own rest-linearized estimate multiplied by
0.5, 1 and 1.5; no previous candidate or reference motion supplies its state.
The lowest training error selects the inner state. Its normalized training
residual vector defines the loss profile.

The outer search estimates both log loss scales jointly, with each scale bounded
to `[0.25,2.0]`. It uses two fixed starts, `(0.5,1.5)` and `(1.5,0.5)`, neither
chosen from the true losses. Central log perturbations of `1e-3` estimate the
profile-residual Jacobian; each perturbed pair refits the state from all three
starts. Perturbations clip to the fixed bounds and use the actual log span in
the derivative denominator. State is therefore refitted during differentiation
as well as candidate evaluation.

Column-scaled, augmented QR computes damped Gauss-Newton steps without normal
equations. Trial log scales are projected into the bounds. Damping starts at
`1e-3`, multiplies by 0.2 after accepted descent and by 10 after rejection, and
is bounded to `[1e-12,1e12]`. There are at most 16 outer iterations and eight
trials per iteration. Only strict training-objective descent is accepted;
training relative RMSE below `1e-8` terminates with `residual_converged`.
No descent, unresolved derivatives and exhausted iteration budgets remain
explicit stopping outcomes. Both outer starts are retained, and selection again
uses only training error. The inner optimizer and its budgets are unchanged.

The inverse object owns only assumed operators, sensor geometry, sample rate
and training voltage. Reference losses, mechanical truth and held-out voltage
are used only after final selection. The six positive controls require both
loss errors below 1%, both held-out voltage errors below `1e-6`, both held-out
state energy-norm errors below `1e-4`, and neither scale within `1e-5` of a search
boundary. The negative control requires a completed fit that fails prediction
acceptance and two-loss recovery; the range must not widen to include its truth.
Prediction acceptance uses only held-out voltage and boundary status, while
the loss/state correctness checks explicitly require synthetic truth.

## Retained results

The [receipt](../references/nonlinear-magnetic-loss-profile-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-loss-profile-v1`, with 485448
bytes and SHA-256
`09a6e413c340e48b3de2efa97f7b537e0634b0a40c0d453d3cda23a3c0e95ef1`.
All six positive controls recover both losses and the held-out state. All 12
outer starts in those controls reach `residual_converged`, in four or five
iterations. Each positive case evaluates 47 loss candidates across its two
starts. Final selection uses training cost only.

| Reference structural/damper scales | Sample rate | Largest relative loss error | Largest held-out voltage relative RMSE | Largest held-out state energy-norm relative error |
| --- | ---: | ---: | ---: | ---: |
| 0.63 / 1.37 | 48 kHz | 3.251e-11 | 4.915e-11 | 4.392e-11 |
| 0.63 / 1.37 | 96 kHz | 3.312e-11 | 5.531e-11 | 4.940e-11 |
| 1.13 / 0.57 | 48 kHz | 1.663e-11 | 5.152e-11 | 6.145e-11 |
| 1.13 / 0.57 | 96 kHz | 9.276e-11 | 9.697e-11 | 1.129e-10 |
| 1.47 / 0.91 | 48 kHz | 2.062e-10 | 4.600e-10 | 4.585e-10 |
| 1.47 / 0.91 | 96 kHz | 2.330e-10 | 5.134e-10 | 5.120e-10 |

The out-of-range control returns scales `(2.0, 0.9101168)`, reaching the upper
structural bound. Both outer starts exhaust the 16-iteration budget; no optimum
or convergence is certified. The selected fit has 1.661% training relative
RMSE, up to 27.49% held-out voltage error and 31.00% held-out state error. Its
structural-scale error is 9.091%. Boundary status and poor prediction reject it;
the nearly correct damper scale does not establish two-loss recovery. All 183
candidate evaluations from this negative case are retained without widening
the range or increasing the budget.

Across all seven cases the report retains 465 loss candidates and 1395 inner
state starts. Thirty inner fits reach the residual threshold; 1365 stop at
`no_descent_step`, as expected for states fitted under incorrect candidate
losses with a nonzero residual floor. This stopping status is not evidence of
a global inner optimum. No candidate or state-start solver error occurs.

The full laboratory release suite passes 132 tests (94 unit and 38 CLI), plus
strict Clippy and formatting checks. The new CLI regression covers all seven
controls, candidate bounds, all three inner and both outer starts, training-only
selection, descending accepted steps, finite-difference evaluation references,
the rejected out-of-range control and output protection. No experimental gate,
start, finite-difference step or optimizer budget changed after the first run.

## Interpretation limits and next step

These dimensionless scales qualify recovery of the modeled damping operators;
they are not measured material constants for a real instrument. Two outer starts
and local profile derivatives do not establish global uniqueness, confidence
intervals or geometry identification. An optimizer stopping status alone does
not demonstrate physical recovery. Finite-difference derivatives profile a
numerically optimized state and inherit its accuracy and possible local minima.

The [paired noise/sensor experiment](MAGNETIC-STATE-COMBINED.md) remains a limit
on calibration: an acceptable voltage residual can conceal incorrect motion or
sensor geometry. This study uses exact geometry and noiseless observations;
it does not remove that ambiguity. Other pickup laws/geometries, noise, unknown
gain/timing, recorded-source calibration and antialiasing remain unqualified
for this loss inverse. No production DSP, preset, audio asset or host process
changes are part of this experiment.

Next measure how this unknown-loss inverse behaves under controlled voltage
noise before introducing sensor uncertainty. Retain prediction, state error
and loss error separately and compare both loss starts. The earlier paired
sensor study requires uncertainty or competing sensor explanations to remain
visible before applying these estimates to recorded sources; low residuals
alone must not promote a fitted loss into production calibration.
