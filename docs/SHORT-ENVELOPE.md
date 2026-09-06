# Joint short-transient envelopes

The new estimator separates declared neighboring carriers while measuring a
short target envelope. Synthetic 32/64 ms observations recover an 8 /s decay
beside a ten-times-stronger neighbor and tolerate a 2 Hz target-frequency error.
This addresses the short detectability seen in the [G3 source pilot](SOURCE-ENVELOPES.md).
It does not yet identify source modes or calibrate physical damping.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- validate-short-envelope --output renders/short-envelope-validation.json
cargo run --locked --release -p rf-73-lab -- short-envelope INPUT.wav --output renders/short-envelope.json --frequencies-hz 1620,1568 --start 0.02 --end 0.18 --window-ms 32 --hop-ms 8
```

These frequencies and times are synthetic examples. The first carrier is the
target; up to two others describe nuisance components. The interval is explicit,
with no onset inference or automatic trimming. WAVs retain native rate/gain;
multichannel input requires zero-based `--channel`. Outputs must be new JSON
files. Rejected observations are valid command outputs; inspect `qualified`.
The validation command fails on unmet expectations and retains its report.

## Local model and numerical checks

For time `tau` centered on a frame and normalized time `u` ranging from -1 to 1,
fit unweighted samples jointly to:

```text
y(tau) = dc + sum_j Re[(c0_j + c1_j*u + c2_j*u^2) * exp(i*2*pi*f_j*tau)]
```

Each carrier contributes six real columns: cosine/sine times 1, u and u squared.
A common constant handles DC. The target's center coefficient is globally
demodulated by its supplied carrier and the absolute frame-center time. This
polynomial approximates an evolving complex envelope; it does not prescribe an
exponential decay or supply the expected rate to the fit.

Modified Gram-Schmidt QR with two orthogonalization passes solves the joint
least-squares problem without normal equations. The design is shared across
equal-size frames. A residual-column/original-column norm ratio below `1e-5`
stops the fit with `ill_conditioned_carriers`, without coefficients or a rate.
This relative pivot is a diagnostic, not a condition number or identification
guarantee.

Residual variance is `SSE/(sample_count-column_count)`. Propagation through the
target cosine/sine rows of `R^-1` gives the root-sum coefficient-noise scale used
for the reported regression margin. A test independently verifies it against
the estimator's impulse-response norm. Residuals include model error and noise;
this is not a confidence interval under colored noise or an incomplete model.

Free fits of dB amplitude and unwrapped phase against center time yield decay
and carrier offset. Early/late rates and both residuals remain visible. Finite
polynomial approximation biases amplitude/phase and overlapping observations are
correlated. An undeclared mixture can evade the gates; acceptance is conditional
on the carrier model and does not prove a physical mode.

## Gates and bounds

The earlier Hann estimator and source-pilot gates are unchanged. This distinct
model has the following declared requirements:

| Check | Requirement |
|---|---|
| Carriers | 1..3 distinct finite frequencies, more than four cycles per nominal window, below 0.45 times sample rate |
| Window/interval | 16..64 ms; hop from one eighth to one half window; interval at most 0.3 s |
| Work | At most 64 windows and one million window-sample observations |
| Support | At least six windows and 0.06 s between first/last centers |
| QR pivot | At least `1e-5` relative |
| Margin | Every point at least 18 dB on the regression scale and -180 dBFS |
| Input level | No full-scale samples in the interval |
| Decay | At least 3 dB fitted drop |
| Amplitude residual | At most 0.5 dB RMS |
| Early/late rate difference | At most `max(0.5 /s, 15% of absolute fitted rate)` |
| Carrier offset | At most 3 Hz absolute |
| Phase residual | At most 0.05 radians RMS |

Actual integer-sample durations are retained; incomplete windows are not padded.
Offset probes cover 0 and +2 Hz, not every possible trajectory within the allowed
band. Large offsets can alias under phase unwrapping. Neither a T60 nor a
natural-sustain or calibrated-loss claim is made.

## Controlled study

The [final receipt](../references/short-envelope-validation.json), experiment
`joint-short-envelope-validation-v1`, uses nine 0.25 s waveforms at 48 kHz,
32/64 ms windows, 8 ms hop and fixed interval 0.02..0.18 s. Rust generates the
samples in memory. Target: 1620 Hz, amplitude 0.03, phase 0.73, decay 8 /s.
Neighbor: 1568 Hz, amplitude 0.3, phase -0.4, decay 1 /s. Common DC: 0.01.

Variants are target alone, strong neighbor, uniform noise amplitude 0.0003,
target detuning +2 Hz, omitted true neighbor, near-coincident neighbor at 1620.5 Hz,
target amplitude 0.002 with noise 0.03, a continuous decay-rate switch from 4 to
18 /s at 0.09 s, and zero decay. Noise uses fixed LCG64 seed `0x735eed`, not a
statistical population. Accepted cases must have rate error below 0.1 /s and
carrier error below 0.02 Hz.

| Accepted case | 32 ms rate error /s | 64 ms rate error /s |
|---|---:|---:|
| Clean | 2.47e-7 | 7.20e-6 |
| Strong neighbor | 1.10e-5 | 1.49e-5 |
| Strong neighbor with noise | 0.016548 | 0.003010 |
| Detuned target | 0.000100 | 0.000169 |

All eighteen final expectations pass on 2026-09-06. Maximum accepted carrier
error is about 0.001603 Hz. Near-coincident carriers fail QR qualification;
omitted-neighbor, low-margin, changing-decay and stationary cases reject for
their declared reasons at both widths.

The [initial expectation receipt](../references/short-envelope-initial-expectation.json)
is retained. It had 17/18 expected outcomes because the omitted-neighbor case
at 32 ms was required to reject specifically on carrier offset. It rejected on
margin, drop, amplitude/slope and phase instability instead. The expectation
now requires margin rejection at both widths. No waveform, equation or threshold
changed; all measured coefficients and fitted diagnostics are identical across
the two receipts. This correction is not independent validation of a new criterion.

Tests also cover 44.1/48/96 kHz native mixtures, matched-carrier phase, an
independently specified three-carrier polynomial system, QR orthogonality and
noise-scale propagation, invalid input, short support and silence. CLI tests
cover the full study, WAV observations, source bytes and output protection.

## Integration and next work

The sibling RackForge checkout advanced to 0.1.15, so the two local
SDK/program-api entries in Cargo.lock were synchronized offline. No remote
dependency was updated. Full workspace tests cover DSP, analysis, laboratory,
plugin and UI compatibility; no host audition or new packaged release is claimed.

Next freeze a short-interval source pilot with independently declared nuisance
harmonics and compare both widths, retaining conditioning, residuals and rejected
fits. Synthetic success does not justify trimming a recording until a desired
rate appears. The earlier full-interval source pilot and audible baseline remain
unchanged; no geometry, mechanical loss or pickup law is selected here.
