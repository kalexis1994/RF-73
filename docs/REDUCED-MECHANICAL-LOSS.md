# Loss recovery with reduced mechanical observations

Three exact modal projections recover the two prescribed loss scales within the
existing 1% criterion in all six controlled cases. Six modes reduce the error
further. One mode and the specified instantaneous pickup reconstruction fail.
This does not show that three modes are enough to simulate the instrument or
that those coordinates can already be extracted from recorded audio.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- reduced-mechanical-loss --output renders/reduced-mechanical-loss.json
```

The [receipt](../references/reduced-mechanical-loss-validation.json), experiment
`reduced-state-mechanical-loss-v1`, schema 1, contains six trajectories and 36
observation results in 101844 bytes. It retains every fit, including negative
scales, row diagnostics, reconstruction errors and failed reduced observations.
Only full-state and nine-mode reconstruction controls have pass requirements.
The command writes failures before returning an error if a control fails.
All outputs must be new JSON files. No waveform assets are generated.

## Fixed experiment and shared trajectories

The three scale pairs, 48/96 kHz rates, geometry, elastic hammer strike, 0.14 s
damper event, six intervals and fit/held-out split are exactly those of the
[full-state experiment](MECHANICAL-LOSS.md). A read-only laboratory callback
observes each mechanical tick; every reduction sees the same before/after state.
No reduced model is resimulated and no strike or row is selected independently.
Full-state rows match both the shared simulator's original rows and the previous
receipt exactly.

The full solver still has nine coordinates. Observation choices were fixed
before the first run: identity, the lowest 1/3/6/9 undamped structural modes,
and a specified instantaneous reconstruction from pickup displacement/velocity.
Modes are ordered by ascending structural frequency, including mounting motion;
the first is not the musical fundamental. Frequencies for this provisional
75 mm geometry are approximately 28.92, 80.69, 169.58, 641.72, 1173.42, 3302.03,
6278.94, 10113.96 and 15208.47 Hz. This is not a tuned G3 model.

## Reconstruction hypotheses

For mass-normalized undamped shapes `phi_i`, with `phi_i^T M phi_j = delta_ij`:

```text
z_i = phi_i^T M q
q_hat = sum_retained(phi_i * z_i)
v_hat = sum_retained(phi_i * (phi_i^T M v))
```

The simulation supplies these exact modal coordinates. There is no FFT fitting,
measurement noise or audio-based mode identification in this experiment.
Truncation discards motion and the damping-mediated power exchange with it.
Substituting reconstructed state into the full energy/loss forms does not restore
that missing exchange.

For a mechanical pickup port `b` and weights `w_i = b^T phi_i`, the separate
pickup hypothesis uses the minimum-M-norm instantaneous lift:

```text
h = sum_i(phi_i * w_i) / sum_i(w_i^2)
q_hat = h * pickup_displacement
v_hat = h * pickup_velocity
```

This preserves the measured port but does not reconstruct all coordinates.
The test receives only the two scalar pickup values in this path, even though
the simulator has the full state available. Pickup here means mechanical motion,
not magnetic voltage. The minimum mass norm does not minimize stiffness energy;
unobserved high-frequency deformation can make the reconstructed energy much
too large.

All reconstructed states use the original `M,K,C0,D`, endpoint energy,
trapezoidal power quadrature and two-column QR fit. Internal consistency requires
positive scales, contact-free rows and held-out relative energy RMSE below 0.005.
The separate known-scale check requires both relative parameter errors below
1%. The latter uses synthetic ground truth for evaluation, not for choosing
the observation or fitting its values.

## Results

All twelve full-state/nine-mode controls pass. Maximum nine-mode mass-weighted
state reconstruction error is below `6.2e-16`. The reduced outcomes are:

| Observation | Recovery and internal consistency | Maximum structural relative error | Maximum damper relative error | Maximum held-out energy relative RMSE |
|---|---:|---:|---:|---:|
| Full state | 6/6 | 9.187e-7 | 2.231e-6 | 1.072e-6 |
| Lowest 1 mode | 0/6 | 0.021329 | 4.374088 | 0.259989 |
| Lowest 3 modes | 6/6 | 0.003279 | 0.000144 | 0.003300 |
| Lowest 6 modes | 6/6 | 6.920e-7 | 2.091e-5 | 2.124e-6 |
| Lowest 9 modes | 6/6 | 9.187e-7 | 2.231e-6 | 1.072e-6 |
| Instantaneous pickup lift | 0/6 | 25.567244 | 6126.496273 | 6.037781 |

Three modes retain roughly 93.0..99.45% of integrated mechanical energy in these
intervals, but their velocity reconstruction error reaches 26.53%. Thus passing
the two-parameter test is not equivalent to accurately reconstructing all motion.
The structural scale bias reaches about 0.328%, persists under temporal refinement,
and remains visible even with a small held-out residual.

Six-mode errors also need not decrease fourfold with time-step refinement as the
full-state quadrature errors did. For scales `(0.5,1.5)`, held-out residual is
about `2.12e-6` at 48 kHz and `2.03e-6` at 96 kHz. Missing-state error remains
after numerical integration error shrinks.

The pickup lift yields negative structural scales in two observations and
damper scales thousands of times too large in others. Its reconstructed energy
is about 124..143 times the true integrated energy. Those values are failed
inverse-model diagnostics, not physical losses or an instability of the solver.

## Verification and next step

Tests verify mass orthogonality, idempotence and mode-sign invariance of the
projections, nine-mode reconstruction, preservation of the scalar pickup port,
and a nonzero instantaneous motion invisible at that port. CLI checks retain
all 36 outcomes and shared-trajectory controls, and protect existing outputs.
The original mechanical-loss command retains its prior behavior through the
same simulator with an empty observation callback.

An instantaneous null motion does not establish that a dynamic observer using
the whole time history is unobservable. The next study should use a known
mechanical evolution and pickup history to reconstruct selected modal states,
testing sensitivity to noise and unmodeled modes before fitting losses. Three
exact oracle modes passing this study are a useful bound, not evidence that
recorded pickup audio provides those same states.

No physical equation, source recording, loss preset, plugin engine or audible
baseline changed. No host or listening test is claimed.

The subsequent [dynamic pickup-state study](DYNAMIC-PICKUP-STATE.md) now tests
that time-history reconstruction with known damping. Nine-mode noiseless
controls pass, while three inferred modes differ materially from these oracle
projections. No unknown loss scale is recovered by that observer yet.
