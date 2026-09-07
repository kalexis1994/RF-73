# Local resolution with constant-voltage weighting

## Frozen protocol

This protocol is fixed before the first run. Replay the 30 point estimates in
`references/nonlinear-magnetic-loss-weighting-validation.json`, pinned to Git
blob `72ef1d0838548765e3955eaf673da4ed82ad0494` and SHA-256
`c0629403f9187efc05f69c22a90ae6b777ba0e400fb69ddc5d46cdc97c4085ca`.
The earlier relative-window diagnostic retains its own command and source.

```powershell
cargo run --locked --release -p rf-73-lab -- magnetic-weighted-loss-resolution --input references/nonlinear-magnetic-loss-weighting-validation.json --output references/nonlinear-magnetic-weighted-loss-resolution-validation.json
```

Read only schema/weighting, rate, noise condition, estimated loss scales,
training RMSE and outer-start completion metadata. Reference motion, loss
errors, held-out scores, prior fitted state and historical inner-search paths
are excluded from the projection. Recreate the same six synthetic trajectories
and five conditions per trajectory using the existing simulation/noise code.
No outer loss optimization is repeated.

For each estimate refit one continuous 18-coordinate state at the center and
ten alternatives: axial log offsets `+/-ln(1.01)`, axial `+/-ln(1.05)`, and
the weakest raw-voltage derivative direction at `+/-ln(1.05)`. Decreases are
reciprocal scale factors. The smaller axial offsets form centered derivatives.
Keep all original three-start inner optimizer settings and require central
training RMSE replay within absolute `1e-8`.

Use the same constant-voltage weights as the fitted source: one reciprocal
norm of concatenated measured training voltage, applied to seed, residual and
Jacobian. Multiply residuals by that common norm to recover voltage. Retain
pooled-relative and raw-voltage singular values; their ratio must equal the
common normalization within numerical precision. The weak direction therefore
uses the same metric as the inverse. Reuse the stable two-column QR diagnostic.

The descriptive alternative distance is `D = norm(pred_alt-pred_center)/sigma`,
where sigma is the injected noise standard deviation. It is a total vector
distance, not a waveform RMS ratio or a calibrated statistical test. The
first-order local log-vector radius for D=1 is `sigma/smallest_raw_singular`.

Withhold that radius for zero noise, failed center replay, an error, unknown
stop status or exhausted budget in any source outer start or new local inner
start, an improving alternative, or unresolved sensitivity. Unselected starts
also count. Keep signal distances and raw/noise-scaled singular values for
inspection when an optimizer limit withholds the radius. Record all source
outer statuses and every new candidate/start outcome. No-descent is a completed
numerical attempt for this descriptive diagnostic; it does not establish
stationarity or a global optimum. Historical inner-search paths are not
requalified. No confidence interval or coverage claim is permitted by this
experiment, even for rows with a reported radius.

Only mechanical validity and center replay are required controls; optimizer
limitations are retained without enlarging budgets. The geometry and field law
remain exact synthetic inputs. Production DSP, presets, audio assets and host
processes are outside this offline study.

## Results

The command completed and retained
`references/nonlinear-magnetic-weighted-loss-resolution-validation.json`:
schema 1, `nonlinear-magnetic-weighted-loss-resolution-v1`, 591705 bytes,
SHA-256 `a0666500489a5ee289dc99a0241209d88217d51800688768734075c00599f948`.
All 30 centers replay with maximum absolute training RMSE difference below
`3.990e-17`. All 330 candidate refits succeed, using 990 inner starts: 18
report `residual_converged` and 972 `no_descent_step`. No new local start
exhausts its budget. All 300 alternatives raise the central training objective.

The raw/pooled singular scaling identity has maximum relative discrepancy
below `7.106e-15`. The new metric is therefore consistent with the one used
by the loss inverse at numerical precision for these cases.

| Diagnostic | 40 dB | 20 dB |
| --- | ---: | ---: |
| Paired observations | 12 | 12 |
| Structural approximately 1% alternatives with D < 1 | 0/24 | 24/24 |
| Damper approximately 1% alternatives with D < 1 | 0/24 | 18/24 |
| Any 5% alternative with D < 1 | 0 | 0 |
| Reported local radii | 11/12 | 12/12 |
| Reported log-vector radius range | 0.0016233-0.0034520 | 0.016092-0.034628 |

At 20 dB all 12 cases have both structural alternatives below the descriptive
one-noise-unit threshold, and nine cases also have both damper alternatives
below it. This totals 42 alternatives, compared with 24 in the earlier
relative-window diagnostic. This change reflects different state refits and
point estimates under the common voltage metric, not a change in the physical
instrument or proof that the new point estimates are worse. The preceding
paired fit study already records mixed loss-recovery results.

At 40 dB the structural approximately 1% distances span 3.586-6.349 and the
damper distances 6.880-12.80; at 20 dB they span 0.3547-0.6410 and
0.6830-1.277. Weak-direction 5% distances span 14.00-30.32 at 40 dB and
1.395-3.060 at 20 dB. The raw smallest/largest singular ratio spans
0.3880-0.6368 over all cases, including noiseless controls. Nonzero local
sensitivity does not imply practically precise parameter recovery.

There are 23 descriptive radii, six withheld for zero noise and one withheld
for an incomplete source optimizer. The latter is source case 2, 48 kHz,
40 dB, seed 17: its unselected outer start reached `iteration_limit` in the
preceding study. All local refits succeed, but that does not remove the
historical limitation. The selected outer start's `no_descent_step` status
and the unselected limit remain visible together. Its raw singular values
and distances are retained; its radius is null.

For the 23 noisy rows with reported radii, measured weak-direction distances
differ from `ln(1.05)/radius` by at most 1.323%. This is a descriptive check
of local linearization in tested directions, not a tuned acceptance gate or
a calibration of confidence coverage. Zero-noise distances and radii remain
null rather than being assigned infinite precision.

## Verification and next step

All 105 laboratory unit tests and strict Clippy pass. The new end-to-end CLI
test reproduces the full 30-center matrix and checks pooled/raw unit scaling,
source completion metadata, radius withholding, inner training-only selection,
candidate references, distances, output protection and byte-modified input
rejection. Unit tests separately exercise an unselected budget limit, unknown
or absent statuses, an improving alternative, failed replay, zero noise, and
voltage reconstruction with unequal window energies. The earlier relative-window
resolution CLI regression is also checked; its weighting and source remain
unchanged. No outer-search matrix, listening test or host session is required
for this offline stage.

Next use independent noise realizations to measure estimator dispersion and
test the behavior of these local diagnostics, retaining all failed or limited
fits. A coverage rule needs separate qualification; D=1 must not silently
become a confidence threshold. Exact sensor geometry and the white-voltage
noise model remain supplied assumptions before recorded-source calibration.
