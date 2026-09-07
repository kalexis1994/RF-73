# Paired noise and magnetic sensor mismatch

This experiment combines additive voltage noise with an incorrect assumed
pickup gap, transverse offset or field law. It follows the
[separate-perturbation study](MAGNETIC-STATE-ROBUSTNESS.md) and asks whether noise
can hide sensor mismatch while changing the inferred mechanical state.
Mechanical losses, longitudinal pickup/damper positions and event timing remain
known. Only the continuous initial state is fitted.

Run `rf-73-lab magnetic-state-combined --output NEW.json`. The output must be a
new JSON file. Full sample histories are not stored; compact results retain all
three starts, failed fits, unsupported geometry and explicit paired references.

## Protocol fixed before the first run

The six physical trajectories at 48/96 kHz, two magnetic laws and baseline/close
pickup geometries are unchanged. Each of these 24 trajectory/sensor pairs uses
six assumed sensors: matched, gap -5%/+5%, offset -5%/+5%, and swapped field law.
Those six assumptions are crossed with five noise conditions: no noise, 40 dB
with seeds 17/71, and 20 dB with seeds 17/71. The two noisy levels were selected
from the preceding experiment to test one where matched-state recovery passed
and one where it generally failed, not as universal noise boundaries.

This gives 720 rows: 24 noiseless matched controls, 120 noiseless sensor-error
cases, 96 noise-only controls, and 480 combined cases. The close pickup's -5%
gap remains outside the supported range. Its 60 rows are retained as invalid,
including 48 combined rows. Thus 660 fits and 432 complete paired comparisons
are possible; failed fits would remain visible and withhold their comparisons.
Only the 24 noiseless matched controls carry mandatory recovery gates.

Noise-only and combined rows use exactly the same clean signal and Gaussian
noise samples for a given trajectory, true sensor, SNR and seed. The sensor-only
control uses that signal without noise. Noise standard deviation still comes
only from the first clean training window and remains constant over both full
windows. Changing the assumed sensor never changes that noise calibration.

The study reuses the original mechanical-history construction, nonlinear
optimizer, initial guesses, budgets, training-only start selection and held-out
scores. No state is reset at the damper event. Neither paired controls nor
held-out scores influence a fitted state. Longitudinal sensor geometry stays
fixed; swapping field law also changes rest sensitivity and does not isolate
field shape from gain.

Each combined row references exactly one noise-only and one noiseless
sensor-error row within the same trajectory and true sensor. Missing or
duplicate references are rejected. Incomplete fits are explicitly withheld
instead of being counted as either successful or unsuccessful comparisons.
Completed comparisons retain the maximum of the two held-out state errors for
each member, plus these descriptive flags:

- `mismatch_masked_by_noise`: the noiseless sensor error fails prediction,
  while the combined condition passes the oracle noise-aware prediction gate.
- `prediction_consistent_state_error`: the combined condition passes that
  prediction gate but exceeds 1% state error in at least one held-out window.
- `new_hidden_state_error_vs_noise_only`: the preceding flag is true and the
  paired noise-only control passes prediction with state error below 1% in both
  windows. This separates cases where noise alone already failed the state gate.

The gates remain exactly those of the preceding study: state energy-norm error
below 1% in both windows; measured voltage relative RMSE below
`max(1e-6, 1.25 * injected_noise_relative_RMS)` in each window. Noiseless matched
controls instead retain the original strict voltage/state limits of `1e-6`
and `1e-4`. The noise-aware diagnostic requires actual injected noise and is
**not a deployable rule for recordings**. All physical-state checks also require
synthetic truth. Flags describe these realizations, not statistical estimator
bias, confidence intervals or general identifiability guarantees.

## Retained results

The [receipt](../references/nonlinear-magnetic-state-combined-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-state-combined-v1`, with
3268672 bytes and SHA-256
`ac7bfb41f7b9854e076304609dd2628beb260ce4d5a070507e0e62f6be1c8f0e`.
All 24 required controls pass. All 660 valid sensor cases complete three starts;
the 60 invalid gap cases and their 48 incomplete combined comparisons are
retained. There are 432 completed paired comparisons. The 240 controls shared
with the preceding receipt reproduce identical serialized fit results.

The table counts complete comparisons, with maxima over both held-out windows.
Every masked case also exceeds 1% state error. No swapped-law case passes the
prediction diagnostic at either tested noise level.

| Nominal SNR | Assumed sensor error | Complete pairs | Masked by noise | New hidden state errors versus noise-only | Maximum combined state error |
| --- | --- | ---: | ---: | ---: | ---: |
| 40 dB | Gap -5% | 24 | 0 | 0 | 8.209% |
| 40 dB | Gap +5% | 48 | 0 | 0 | 8.429% |
| 40 dB | Offset -5% | 48 | 0 | 0 | 11.13% |
| 40 dB | Offset +5% | 48 | 0 | 0 | 12.47% |
| 40 dB | Swapped field law | 48 | 0 | 0 | 139.6% |
| 20 dB | Gap -5% | 24 | 24 | 0 | 14.12% |
| 20 dB | Gap +5% | 48 | 48 | 2 | 10.68% |
| 20 dB | Offset -5% | 48 | 48 | 2 | 15.06% |
| 20 dB | Offset +5% | 48 | 48 | 2 | 18.49% |
| 20 dB | Swapped field law | 48 | 0 | 0 | 136.3% |

At 20 dB, all 168 valid radial-gap/transverse-offset cases become
prediction-consistent while retaining state errors above 1%. Six of these
comparisons start from a paired noise-only state within 1%: two distinct
trajectory/noise controls, each tested against three geometry errors. Both use
the point-pole proxy, close pickup, 96 kHz and seed 17. Their noise-only maximum
state errors are 0.9559% and 0.9052%; with sensor mismatch the errors range from
4.322% to 8.423%, while prediction still passes. The other 162 masked cases
already have a noise-only state outside the 1% limit. These are paired
realization counts, not independent population samples.

At 40 dB, all 216 complete comparisons reject the mismatched sensor using the
existing diagnostic. This does not establish a universal safe SNR or prove
identifiability for smaller geometry errors, another noise spectrum, or an
unknown magnetic law.

All 1980 start outcomes are retained: 72 matched attempts reach the residual
convergence criterion and 1908 perturbed attempts stop at `no_descent_step`.
No solver error or iteration-limit result occurs, but the latter stopping
status does not certify global convergence or correct physics.

The full laboratory release suite passes 131 tests (94 unit, 37 CLI), plus
strict Clippy and formatting checks. New tests verify exact paired noise,
missing/duplicate control rejection, incomplete-pair withholding, all row and
pair counts, training-only selection, classification against the referenced
controls and output protection. The original robustness CLI also passes after
sharing the mechanical-history builder. No experimental gate or optimizer
budget changed.

## Scope and next step

This remains a synthetic initial-state qualification with exact mechanics and
losses. It does not estimate geometry, gain or losses. Two seeds and three fixed,
collinear starts do not establish population statistics or global uniqueness.
Centered sign ambiguity, physical field calibration and antialiasing remain
separate concerns. No production equation, preset, audio asset or host process
changes are part of this experiment.

Next implement controlled nonlinear loss profiling with known geometry, fitting
one continuous initial state again for every candidate loss pair. These paired
results must remain explicit limits on its interpretation: a low voltage
residual cannot certify the state, sensor geometry or physical losses. Before
recorded-source calibration, loss estimates must account for sensor uncertainty
and retain competing explanations or withhold a unique result when the data
cannot distinguish them.
