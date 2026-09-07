# Constant-voltage weighting for nonlinear loss recovery

## Frozen protocol

Compare the preceding relative-window fit with equal sample weights on the
same synthetic observations. The protocol was written before the first run.
The baseline is the byte-pinned
`references/nonlinear-magnetic-loss-noise-validation.json` (Git blob
`5e32ac1255599fd8aae280eb6656d14921c0f174`, SHA-256
`5499232a9abd733a0e1a09169b14d8d17de6da855202923669e92cac8a54d1a1`).

```powershell
cargo run --locked --release -p rf-73-lab -- magnetic-loss-weighting --input references/nonlinear-magnetic-loss-noise-validation.json --output references/nonlinear-magnetic-loss-weighting-validation.json
```

Use the shared noise-study generator: three loss pairs, 48/96 kHz, a noiseless
control and 40/20 dB with seeds 17/71 for each fixture. These are 30 paired
observations. The known production pickup geometry, mechanical operators,
two training halves, damper event and held-out halves are unchanged.

Replace `r_i = (y_i - prediction_i)/(sqrt(2)*norm(y_window))` with
`r_i = (y_i - prediction_i)/norm(concatenated_training_voltage)`.
The new scalar is calculated once per training dataset and does not vary
with the candidate. Every sample has the same weight. Its squared objective
is proportional to squared voltage error and, for constant independent
Gaussian noise, to the negative log likelihood up to an additive constant.
The scalar keeps the numerical objective dimensionless and defined in the
noiseless case without supplying oracle sigma to the inverse. This is pooled
relative RMSE, not a noise-whitened chi-square statistic. It must not be
compared numerically against the old relative-window RMSE.

Apply this weighting consistently to the linearized seed, state residual,
analytic Jacobian, outer loss residual and known-loss state comparator.
Keep the same 18 continuous coordinates, three inner starts, two outer starts,
loss bounds, derivative steps, LM budgets and normalized residual thresholds.
The changed common normalization affects absolute stopping scale; this is
recorded rather than claiming complete numerical scaling invariance.

Read and verify baseline bytes before expensive work, but deserialize its
estimates and scores only after all new fits finish. Never warm-start from the
baseline or select with reference losses or held-out data. Verify paired
fixture/rate, SNR, seed and sigma, retain every candidate and failed start,
and compare new-minus-old loss errors and each held-out clean/measured
voltage and state error. Negative error changes mean improvement. Preserve
known-loss state control scores separately. Only the six noiseless controls
have required strict recovery gates; noisy results have no superiority gate.

Two seeds and a matched synthetic sensor cannot establish coverage, justify
confidence intervals, or qualify real recordings. This change is confined
to offline laboratory inference; production synthesis and presets are unchanged.

## Results

The command completed successfully and retained
`references/nonlinear-magnetic-loss-weighting-validation.json`: schema 1,
`nonlinear-magnetic-loss-weighting-v1`, 4009237 bytes, SHA-256
`c0629403f9187efc05f69c22a90ae6b777ba0e400fb69ddc5d46cdc97c4085ca`.
All six noiseless controls pass the original strict state/voltage gates and
recover both loss scales. No candidate fails or hits a loss boundary.

Each noisy column below contains 12 paired cases. Loss recovery means both
scale errors below 1%; state recovery means both held-out state errors below
1%. Maximum loss error is the maximum over both scales and all cases.

| Measure | 40 dB relative windows | 40 dB constant voltage | 20 dB relative windows | 20 dB constant voltage |
| --- | ---: | ---: | ---: | ---: |
| Both losses within 1% | 12/12 | 12/12 | 3/12 | 2/12 |
| State within 1% | 11/12 | 12/12 | 0/12 | 0/12 |
| Oracle prediction consistency | 12/12 | 12/12 | 12/12 | 12/12 |
| Maximum loss error | 0.7411% | 0.3854% | 6.7595% | 3.9983% |
| Maximum state error | 1.3141% | 0.9671% | 12.2712% | 9.5801% |
| Maximum known-loss state error | 0.8384% | 0.4077% | 7.8197% | 4.0598% |

This is a mixed result, not universal superiority. At each noisy level,
structural error decreases in 7/12 cases and damper error in 4/12. The first
held-out state window improves in 12/12 and the second in 7/12. The known-loss
state comparator improves in both windows of all 24 noisy cases, isolating a
clear benefit when losses are supplied in this small matched experiment.
At 20 dB, two cases newly recover both losses within 1%, while three previously
successful cases cross outside that threshold. The lower maximum error does
not imply more cases satisfy the threshold. Ten 20 dB fits now combine oracle
prediction consistency and agreeing loss starts with inaccurate losses.

The old late-window sample weight was approximately 3.21-4.80 times the early
weight at 40 dB and 3.07-4.41 times at 20 dB. The new residual assigns equal
sample weights in both windows, consistent with the injected constant sigma.
Training-window energies are retained so this ratio can be independently
reconstructed; no oracle noise scale enters optimization.

The run uses 2827 loss candidates and 8481 inner starts, plus 90 starts for
known-loss state controls. Of the 60 outer starts, 12 noiseless starts report
`residual_converged`, 47 noisy starts report `no_descent_step`, and one reaches
`iteration_limit`. The limited start is `(0.5,1.5)` for reference losses
`(1.13,0.57)` at 48 kHz, 40 dB, seed 17. The other start is selected by its
lower training objective. Their relative scale spread is below `1.611e-7`;
the maximum spread over all pairs is below `2.621e-7`. The limited start is
retained rather than rerun with a larger budget. Neither agreement nor a
no-descent exit certifies a global optimum.

Verification includes all 102 laboratory unit tests, strict Clippy and
formatting, a full manual execution of the new command, its CLI preflight
checks and a saved-receipt invariant test. The latter checks all paired
identities, copied baseline scores, six strict controls, selected inner/outer
training minima, candidate references and signed error differences. The
existing end-to-end local-resolution regression also passes and reproduces
all 30 relative-window centers. The expensive old outer-search/noise matrix
was not repeated; its pinned receipt supplies the baseline. No listening or
host test is claimed for this offline inference change.

## Next step

Recompute local profile sensitivity with the same constant-voltage weights
used by the inverse, retaining optimizer limitations explicitly. Then test
independent noise realizations before attaching coverage or confidence to
any local radius. The mixed 20 dB recovery and supplied sensor geometry remain
limitations; these results do not authorize fitting a real recording as if its
physical parameters were uniquely identified.

The [weighted local-resolution study](WEIGHTED-MAGNETIC-LOSS-RESOLUTION.md)
now replays all 30 estimates with matching voltage weights. It preserves the
unselected iteration limit above and withholds that row's descriptive radius.
