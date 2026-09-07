# Unknown nonlinear magnetic losses under voltage noise

This study adds controlled voltage noise to the
[unknown-loss profile](NONLINEAR-MAGNETIC-LOSS-PROFILE.md). It retains known
production pickup geometry and the same physical model. Each selected loss fit
is compared with a separate state-only inverse that receives the true losses
on the identical noisy waveform. That reference comparator runs after the
unknown-loss selection and cannot influence it.

Run `rf-73-lab magnetic-loss-noise --output NEW.json`. Existing outputs are
protected. The report retains all loss candidates, inner-start summaries,
outer histories, failures, start agreement and paired state checks. No audio
matrix is generated.

## Protocol fixed before the first run

The three reference structural/damper scale pairs `(0.63,1.37)`, `(1.13,0.57)`
and `(1.47,0.91)` each run at 48 and 96 kHz. The production pickup remains at
1.5 mm gap and 0.5 mm transverse offset, with longitudinal geometry and event
timing fixed. Each of the six physical trajectories receives a noiseless
control and Gaussian noise at nominal 40/20 dB with seeds 17 and 71: 30 fits,
including 24 descriptive noise conditions.

The existing SplitMix64/Box-Muller generator is reused. Its constant voltage
standard deviation is the first clean training window's RMS multiplied by
`10^(-SNR/20)`. It is not rescaled using held-out samples or the later window.
The same actual noisy samples feed the unknown-loss and known-loss inverses.
The nominal SNR refers to the first training window; later damped intervals
can be dominated by noise. Equal seeds also match the earlier robustness study's
standardized noise sequences.

Neither true loss scales, clean voltage, noise level, reference state nor
held-out data enter the unknown-loss profile. It retains the same two loss
starts, three state starts per candidate, bounded log-scale search, derivative
step, iteration/trial budgets, relative-window training weights and strict
descent rule. The true losses are used for scoring and for the explicitly
labeled known-loss comparator only. All starts and stopping outcomes remain
visible; disagreement and exhausted budgets do not silently disappear.

After selection, the study records measured and clean held-out voltage errors,
full 18-coordinate state error, each relative loss error, and boundary status.
The synthetic oracle prediction diagnostic is unchanged: each measured
held-out relative RMSE must be below
`max(1e-6,1.25*actual_injected_noise_relative_RMS)`. Loss fits must additionally
avoid a search boundary. This diagnostic uses actual injected noise and is not
an acceptance rule available for recordings. Both state errors and both loss
errors are separately scored against a 1% descriptive limit. Only the six
noiseless controls require the earlier strict voltage/state gates and loss
recovery; their known-loss comparators must also pass strict recovery.

For two completed outer starts, per-scale agreement is
`abs(a-b)/max(abs(a),abs(b)) < 1%`. Missing results explicitly withhold that
comparison. Start agreement is not a confidence interval or correctness test.
`agreement_hides_loss_error` flags prediction-consistent fits whose starts agree
but whose selected losses fail the 1% criterion. Reference loss errors are
attached to each start only after selection.

The paired state comparison flags a newly hidden state error when the
known-loss state is within 1%, but the unknown-loss state exceeds 1% despite
passing the prediction diagnostic. Incomplete fits withhold that comparison
rather than implying either a successful or failed physical recovery.

## Retained results

The [receipt](../references/nonlinear-magnetic-loss-noise-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-loss-noise-v1`, with 4014816
bytes and SHA-256
`5499232a9abd733a0e1a09169b14d8d17de6da855202923669e92cac8a54d1a1`.
All 30 unknown-loss fits and all 30 known-loss comparators complete. The six
noiseless controls pass. Their candidate histories reproduce the previous
unknown-loss receipt exactly; all 30 known-loss controls reproduce the matching
results in the earlier robustness receipt exactly.

| Nominal noise condition | Fits | Oracle prediction accepted | Both losses within 1% | State within 1%, losses unknown | State within 1%, losses supplied | Accepted but incorrect losses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Noiseless | 6 | 6 | 6 | 6 | 6 | 0 |
| 40 dB | 12 | 12 | 12 | 11 | 12 | 0 |
| 20 dB | 12 | 12 | 3 | 0 | 0 | 9 |

At 40 dB the largest selected relative loss error is 0.7411%; at 20 dB it is
6.760%. The largest state energy-norm errors are 1.314% and 12.28%, respectively.
For the known-loss comparators these state maxima are 0.8384% and 7.820%.
These are maxima in this small paired matrix, not confidence bounds.

**All 24 noisy fits have agreeing loss starts**, even though nine at 20 dB pass
the prediction diagnostic with incorrect losses. The maximum relative spread
between starts is only `1.159e-7` at 40 dB and `2.310e-7` at 20 dB. Agreement
therefore gives no guarantee of physical correctness, even when much tighter
than the descriptive 1% agreement gate.

One 40 dB comparison exposes a newly hidden state error when losses are freed:
reference scales `(1.47,0.91)`, 96 kHz, seed 71. The estimated pair is
`(1.47708223,0.90920926)`, with loss errors 0.4818% and 0.08690%. The known-loss
state error is 0.8141%; the unknown-loss state reaches 1.314% while prediction
still passes. Passing a loss-parameter tolerance does not ensure the same
tolerance in reconstructed motion. At 20 dB all known-loss controls already
exceed the state gate, so no newly hidden state-error flag is set there.

Measured held-out voltage relative RMSE reaches 91.32% at nominal 40 dB and
100.04% at 20 dB, because the later damped window can be dominated by noise.
The corresponding maxima against clean voltage are 1.392% and 12.89%. The
oracle gate accounts for actual injected noise; its acceptance does not mean
uniformly accurate reconstruction of the clean waveform.

The report retains 3042 loss-candidate evaluations and 9126 inner state-start
outcomes. Thirty inner starts reach the residual threshold and 9096 stop at
`no_descent_step`. Among 60 outer starts, the 12 noiseless starts converge;
all 48 noisy starts stop at `no_descent_step`, within 15 outer iterations.
No fit hits a search boundary, returns a solver error or exhausts the outer
iteration budget. Stopping without a descending trial is not a global optimum
or uncertainty certificate. The known-loss comparators separately retain
their 90 state-start outcomes.

The full laboratory release suite passes 134 tests (95 unit and 39 CLI), plus
strict Clippy and formatting checks. The new tests cover positive/negative
start agreement, symmetry, withheld incomplete starts, all 30 rows, training-only
selection, identical injected-noise diagnostics between paired fits, separate
loss/state classifications, retained candidate records and output protection.
No optimizer, objective weight, gate, noise level or search budget was retuned
after the first run.

## Scope and next step

This is a small synthetic matrix with two noise realizations and three loss
pairs. It does not establish population error rates, confidence intervals,
global uniqueness or universal SNR limits. Sensor law, geometry and gain remain
exact; the earlier sensor/noise ambiguities still constrain calibration.
Gaussian noise is generated directly on each rate's sample grid; a common
band-limited analog noise process is not resampled between rates. Cross-rate
differences therefore cannot isolate sampling accuracy from the noise model.
Antialiasing and real-source calibration remain unqualified. No production DSP,
preset, audio asset or host process changes are part of this experiment.

Next quantify how much the training data constrain each fitted loss using
profile sensitivity and nearby alternative loss pairs, before interpreting a
point estimate as a calibration. Any proposed uncertainty diagnostic needs
separate validation under controlled noise; agreement between optimizer starts
cannot substitute for that check. Sensor uncertainty must still be included
before applying the results to recordings.

The [local resolution study](MAGNETIC-LOSS-RESOLUTION.md) now replays these 30
estimates from pinned evidence and refits state at 300 nearby alternatives.
At 20 dB, both approximately 1% structural changes lie below one descriptive
noise-distance unit in every case; none of the tested alternatives does at
40 dB. This is a local signal-resolution diagnostic, not a confidence interval.
Next compare noise-consistent weighting against the existing per-window
normalization before interpreting uncertainty in the fitted losses.
