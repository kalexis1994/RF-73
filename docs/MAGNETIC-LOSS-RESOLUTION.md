# Local resolution of profiled magnetic losses

This study examines nearby alternatives to the loss estimates from the
[noisy loss experiment](NONLINEAR-MAGNETIC-LOSS-NOISE.md). It reuses those
estimates instead of rerunning the expensive outer search. Every alternative
still refits the continuous initial state using the unchanged nonlinear inner
optimizer and all three state starts.

Run `rf-73-lab magnetic-loss-resolution --input NOISE_RECEIPT.json --output NEW.json`.
The input must match the retained noisy-loss receipt exactly. Its bytes are
checked with the repository's existing `git hash-object --stdin` convention,
against blob `5e32ac1255599fd8aae280eb6656d14921c0f174`; the pinned source SHA-256
is `5499232a9abd733a0e1a09169b14d8d17de6da855202923669e92cac8a54d1a1`.
Reading is bounded to 8 MB, modified input is rejected before simulation, and
an existing output is protected. No new dependencies or outer-fit cache is
introduced.

## Protocol fixed before the first run

The input projection reads only sample rate, noise condition, estimated losses
and selected training RMSE. It omits reference losses, reference state, fitted
states and held-out scores. The same six synthetic fixtures regenerate voltage
at 48/96 kHz and the same fixed noise sequences. Fixture truth supplies only
signal generation, not local candidate selection or resolution scoring.

For each of the 30 prior estimates, the diagnostic first refits state at the
saved loss pair. Its training RMSE must reproduce the source value within an
absolute `1e-8`. It then refits state at ten alternatives:

- Each loss axis separately at log offsets `+/-ln(1.01)` and `+/-ln(1.05)`.
- The weakest raw-voltage derivative direction at log-vector length `ln(1.05)`,
  in both directions.

Positive axial offsets multiply a scale by 1.01 or 1.05. Negative offsets use
the reciprocals, so the labels do not mean an exact arithmetic -1% or -5%.
The weak direction has unit length in the two-dimensional log-scale space;
its components generally change both losses. The original candidate bounds
remain enforced. A failed candidate retains its evaluation/error and withholds
the row's complete diagnosis; it is not silently substituted or clipped.

The two small axial perturbations define central finite differences of the
profiled training residual. There are no held-out samples in this calculation.
The existing per-window normalization is removed by multiplying each residual
by `sqrt(2)*norm(measured_training_window)`, restoring voltage units. Differences
between candidate and center residuals equal differences between their predicted
voltages up to sign, since they observe the same measured samples.

Two-column reorthogonalized QR supplies the determinant used to compute the
smaller singular value as `abs(det(R))/largest_singular_value`. This avoids
subtracting nearly equal Gram determinants. The two-column Gram orientation
supplies the weak right-singular direction. Both the original relative-window
and raw-voltage singular values are retained.

For noisy rows, raw-voltage derivatives and each alternative's predicted-voltage
L2 change are divided by the injected constant voltage standard deviation
`sigma`. This gives the descriptive distance
`D = norm(prediction_alternative - prediction_center) / sigma`.
The report flags `D < 1` and retains the local log-vector radius
`sigma / minimum_raw_voltage_singular_value`. The radius is the first-order
distance along the weak direction that would produce `D = 1`. Actual weak-axis
alternatives test finite changes separately; no new fit is selected from them.

This is a total vector distance, not waveform RMS noise: it does not divide
by sample count, and one unit is not the norm of the whole noise realization.
It is **not a confidence interval, likelihood-ratio acceptance rule or calibrated
hypothesis test**. The injected sigma is an oracle quantity. In addition, the
existing state fit minimizes relative-window residuals, not a whitened-noise
likelihood. These distinctions prevent interpreting a convenient local radius
as a statistically validated uncertainty bound. With zero noise, noise-scaled
quantities and flags are explicitly withheld instead of dividing by zero.

## Retained results

The [receipt](../references/nonlinear-magnetic-loss-resolution-validation.json)
uses schema 1 and experiment `nonlinear-magnetic-loss-resolution-v1`, with
556837 bytes and SHA-256
`3f57ff47bc6275436e4bbeaa84199130649be03e547f2d572ae059986a291d7d`.
All 30 centers replay within `1.150e-16` absolute training RMSE. The report
retains 330 candidate evaluations, 990 inner state starts and 300 alternatives.
No candidate fails. Eighteen noiseless center starts reach the inner residual
threshold; the remaining 972 starts stop with `no_descent_step`. All 300
alternatives increase the original relative-window training objective; none
is promoted into a replacement estimate.

| Nominal SNR | Rows | Rows with an alternative at D < 1 | Structural-axis approximately 1% distance | Damper-axis approximately 1% distance | Local log radius for D = 1 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 40 dB | 12 | 0 | 5.521-9.270 | 13.19-20.19 | 0.001448-0.003021 |
| 20 dB | 12 | 12 | 0.5484-0.8963 | 1.235-1.866 | 0.01481-0.03090 |

At 20 dB, both small structural-axis alternatives lie below one noise unit in
every row: 24 alternatives in total. Neither small damper-axis alternative
does. All 5% axial alternatives and both 5% weak-direction alternatives exceed
one unit at both noise levels. These statements describe the chosen distance
threshold, not acceptance or rejection under a statistical test.

The weak-direction 5% distances are 1.576-3.310 at 20 dB and 16.14-33.99 at
40 dB. Both columns retain nonzero local sensitivity: the smallest/largest
raw-voltage singular ratio spans approximately 0.229-0.366 across all cases.
The six noiseless rows retain singular values and voltage changes but explicitly
withhold noise-scaled distances, radii and below-threshold flags.

For the noisy weak-direction alternatives, the measured 5% distances differ
from the first-order prediction `ln(1.05)/local_radius` by at most 0.877%.
This is a descriptive check of local linearization in the tested directions,
not a new acceptance gate or a coverage test.

These local results expose a practical resolution limit for the structural
loss at 20 dB despite the almost identical outer-start estimates in the
preceding study. The radii are in log-vector norm; they must not be relabeled
as per-parameter percentage confidence bounds or checked as such against truth.

Verification passes all 98 laboratory unit tests, the new end-to-end resolution
CLI test, strict Clippy and formatting. The CLI test covers all 30 centers and
300 alternatives, pinned-source rejection (including semantically unchanged
but byte-modified JSON), zero-noise withholding, inner training-only selection,
candidate references, log-offset reconstruction, distance units and output
protection. Unit tests independently check the two-column singular system,
near-dependent columns and residual conversion back to voltage. The existing
outer-search/noise matrix is reused from its verified receipt instead of being
rerun as part of this verification; the inverse algorithms are unchanged.

## Limits and next step

This is a local, training-only study around previous point estimates. A nearby
alternative that changes prediction little need not be equally plausible under
a calibrated probability model; a larger change does not establish global
identifiability. Finite-difference step size, nonlinear branch effects and
the accuracy of the inner state fit constrain the derivative interpretation.
Two noise realizations do not establish coverage. True sensor geometry, field
law and gain remain supplied. No production DSP, preset, audio asset, real-source
calibration, antialiasing or host process change is part of this experiment.

Next compare the current per-window normalization with a fit weighted for the
constant voltage noise used in these experiments, using paired observations
and preserving all loss/state/prediction scores. This is necessary before
turning profile geometry into an uncertainty claim: the current objective and
the noise-scaled diagnostic use different weighting. Coverage and sensor
uncertainty still need independent qualification before source calibration.

The paired weighting comparison is specified in
[MAGNETIC-LOSS-WEIGHTING.md](MAGNETIC-LOSS-WEIGHTING.md). It uses a separate
command and retains the relative-window behavior of this local diagnostic.
