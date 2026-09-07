# Nonlinear magnetic state: noise and sensor mismatch

This offline experiment qualifies the [nonlinear initial-state inverse](NONLINEAR-MAGNETIC-STATE.md)
under fixed additive noise and incorrect assumed magnetic geometry or field law.
Mechanical losses, longitudinal pickup/damper positions and event timing remain
known. This is still an initial-state experiment, not a loss calibration.

Run `rf-73-lab magnetic-state-robustness --output NEW.json`. Existing outputs
are protected. The report retains every condition, failed fits and compact
outcomes for all three starts; complete optimizer trial histories are not
duplicated in this larger matrix.

## Protocol fixed before the first run

The six physical trajectories, 48/96 kHz sampling, two voltage laws and
baseline/close sensor configurations match the preceding study. Centered pickups
are excluded here: their sign ambiguity remains covered by that study.
Each of the 24 trajectory/sensor pairs receives 12 conditions, for 288 rows:

- Noiseless, correctly specified sensor: required positive control.
- Additive Gaussian noise at 60, 40 and 20 dB, each with seeds 17 and 71.
- Incorrect assumed radial gap, scaled by 0.95 or 1.05.
- Incorrect assumed transverse offset, scaled by 0.95 or 1.05.
- Swapped production/point-pole field law, retaining its existing gain.

Perturbations are applied separately. The signal always comes from the true
sensor on the original mechanical trajectory. Geometry changes affect only the
inverse; they do not regenerate a different physical signal. Swapping field law
also changes rest sensitivity, so that comparison does not isolate field shape
from gain error. Longitudinal pickup geometry is unchanged.

The close pickup already uses the supported minimum gap of 0.5 mm. Its -5%
condition requests 0.475 mm and is retained as `withheld_invalid_sensor` in 12
rows; no fit is attempted and the supported range is not widened. Thus 276 rows
can run the inverse. The first execution exposed an error-propagation issue
that aborted the matrix on this invalid input; retaining the rejected condition
fixed that issue without changing any condition, optimizer or acceptance gate.

Noise uses SplitMix64 and Box-Muller, without rescaling the realized sequence.
Its standard deviation is the RMS of the first clean training window multiplied
by `10^(-SNR/20)`. That constant voltage noise level applies across both windows;
the quoted SNR is a nominal first-training-window value, not the SNR of every
later interval. Samples continue along the sequence without recycling training
noise into held-out samples. Equal seeds provide paired standardized noise
across conditions; the two replicates do not provide statistical confidence
intervals or broad stochastic coverage. Noise calibration uses a known clean
training signal only to generate the synthetic experiment.

The optimizer, budgets, starts, propagation and training/held-out split are
unchanged. Only training voltage selects a state. Clean voltage and mechanical
truth are reserved for scoring. Both measured and clean held-out voltage errors
are retained, alongside the full 18-coordinate state energy-norm relative error.

All 24 matched controls must pass the original strict limits: both held-out
voltage errors below `1e-6` and state errors below `1e-4`. Perturbed cases are
descriptive, with state recovery marked when both state errors are below 1%.
An additional **oracle prediction diagnostic** compares each measured held-out
voltage error with `max(1e-6, 1.25 * injected_noise_relative_RMS)`. Its noise term
uses actual injected noise energy divided by measured signal energy in that
held-out window. This diagnostic is never supplied to optimization or start
selection. It requires synthetic knowledge and is not a deployable acceptance
rule for recordings. Prediction consistency with state error above 1% is
explicitly flagged rather than treated as successful physical recovery.

## Retained results

The [receipt](../references/nonlinear-magnetic-state-robustness-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-state-robustness-v1`, with
1200966 bytes and SHA-256
`fba6bc11ed5db9c834992f3a8dbec32fc6f9558952ae0d0aaebd93685a45f219`.
All 24 matched controls pass. The 276 valid sensor cases complete all three
starts, retaining 828 outcomes. The 72 matched attempts reach the original
residual convergence threshold. All 756 perturbed attempts stop with
`no_descent_step`, which is not a claim of global convergence or true-state
recovery. None reaches the iteration limit or returns a solver error.

| Condition | Valid fits | State error below 1% | Oracle prediction consistent | Maximum held-out state error |
| --- | ---: | ---: | ---: | ---: |
| Matched, noiseless | 24 | 24 | 24 | 1.633e-8% |
| Noise, nominal 60 dB | 48 | 48 | 48 | 0.09585% |
| Noise, nominal 40 dB | 48 | 48 | 48 | 0.9584% |
| Noise, nominal 20 dB | 48 | 2 | 48 | 8.879% |
| Assumed gap -5% | 12 | 0 | 0 | 8.023% |
| Assumed gap +5% | 24 | 0 | 0 | 8.326% |
| Assumed offset -5% | 24 | 0 | 0 | 10.96% |
| Assumed offset +5% | 24 | 0 | 0 | 12.23% |
| Swapped field law | 24 | 0 | 0 | 139.6% |

The remaining 12 gap cases are withheld for invalid geometry. All 108 valid
sensor-mismatch fits fail the noiseless prediction gate and exceed 1% state
error. These results do not establish that those errors would remain detectable
with noise added.

At nominal 20 dB, **46 of 48 prediction-consistent fits exceed 1% state error**.
The receipt's `prediction_consistent_but_state_biased` flag denotes this finite
state discrepancy, not a statistical estimate of estimator bias. A residual
consistent with injected noise does not certify the mechanical state. At 40 dB
the worst state error is already close to the descriptive 1% boundary; this
small matrix does not establish a universal SNR cutoff.

Measured held-out voltage relative error reaches 20.54%, 91.33% and 100.06%
at nominal 60, 40 and 20 dB, respectively. Constant voltage noise can dominate
the later damped interval even when the nominal early-window SNR is high.
Clean held-out voltage error reaches 0.1085%, 1.085% and 10.04%, respectively.
The oracle diagnostic explicitly accounts for injected noise; its passing
counts should not be read as uniformly accurate voltage reconstruction.

Verification covers 128 laboratory tests: 92 unit tests and 36 CLI tests,
plus strict Clippy and formatting. The full run passed the 92 unit and 35
existing CLI tests; the new CLI check exposed an overly exact comparison after
JSON floating-point serialization. That assertion now allows eight machine
epsilons and the new CLI test passes on rerun. Experimental gates, scores and
receipt bytes did not change. Noise moment/repeatability tests and a test that
changes only held-out signal verify noise generation and training separation.
The CLI test covers all condition counts, rejected geometry, training-only
selection, retained starts and output protection.

## Limits and next step

Matched-model noise tolerance cannot certify sensor geometry. These tests keep
the mechanical model and losses exact, do not combine noise with mismatch, and
do not estimate gain, geometry, timing or losses. Three collinear starts are
not a proof of global uniqueness. Current field laws remain uncalibrated proxies;
point sampling does not establish antialiasing. No production DSP, preset,
audio asset or host process changes are part of this experiment.

Next combine noise with sensor mismatch and test whether an incorrect sensor
can pass the noise-aware prediction diagnostic while changing the inferred
state. This is needed before freeing loss parameters: otherwise sensor or state
errors could be absorbed into apparently plausible loss estimates. A later
loss search still needs a nonlinear state refit at each candidate and an
explicit uncertainty/identifiability assessment, not just a low voltage residual.
