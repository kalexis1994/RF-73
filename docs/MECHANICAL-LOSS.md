# Controlled mechanical loss recovery

The nine-coordinate mechanical assembly now provides a controlled identification
test with known hammer and damper state. All six cases recover separate global
structural and damper loss scales, then predict energy changes in unused time
intervals. This is an inverse check on simulated full-state trajectories, not a
calibration from recordings or a claim of measured material properties.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- mechanical-loss --output renders/mechanical-loss.json
```

The [receipt](../references/mechanical-loss-validation.json), experiment
`controlled-mechanical-loss-recovery-v1`, schema 1, is 18476 bytes. It retains
all energy rows, fit/validation indices, estimated and prescribed scales,
mechanical checks and the omitted-damper control. A failed case is retained
before the command exits with failure. Outputs must be new JSON files; no
trajectory WAVs, device access or plugin installation are involved.

## Mechanical model and known events

Use the existing [nine-coordinate assembly](MODAL-ASSEMBLY.md): moving root,
six geometry-derived tine coordinates and one effective tonebar coordinate.
Its default uniform tine is 75 mm long, 1.5 mm in diameter, with Young's modulus
200 GPa, density 7850 kg/m3 and a 0.1 g point tuning mass at 85% of free length.
Hammer and pickup ports are at 20% and 98%; the damper port is at 80%.
These are provisional model values, not identified instrument dimensions.

The existing elastic single-mass hammer strikes with input 0.5, corresponding
to `0.8 * 0.5^1.4` m/s. This test uses the elastic reference contact, not the
later memory-material hammer. The binary spatial damper turns on at 0.14 s;
positions and velocities must remain identical at that state change. Contact
separates around 0.841..0.844 ms. Every fitted and held-out interval must remain
contact-free throughout, as checked at every mechanical tick.

Default baseline losses define `C0`; the added damper matrix is
`D = C_on - C0`. Generate three independently specified pairs of multipliers:
`(alpha, beta) = (0.5, 1.5), (1, 0.5), (1.5, 1)`.
Structural scaling changes both support damping coefficients and divides all
seven bending T60 parameters by alpha. Damper scaling multiplies its viscous
coefficient by beta. Geometry, stiffness, inertia and contact law stay fixed.
The resulting operator is `C(t) = alpha*C0 + beta*s(t)*D`.

Each pair is rendered independently at 48 and 96 kHz using four free-motion
ticks per sample, matrix-exponential free propagation and 64 contact
subdivisions. Refinement also slightly changes the contact trajectory; the
study does not claim identical launch states across the two rates.

## Inverse energy observation

In a contact-free interval with fixed stiffness and known damper state:

```text
E(t) = 0.5 * (q(t)^T K q(t) + v(t)^T M v(t))
E(start) - E(end) = alpha * integral(v^T C0 v dt)
                     + beta * integral(s(t) v^T D v dt)
```

Energy endpoints are reconstructed from state and immutable `M,K` inspection.
Power integrals use endpoint trapezoidal quadrature at every mechanical tick.
They do not read the integrator's accumulated dissipated energy. The simulation's
energy ledger is checked separately as a numerical safeguard. Thus finite
quadrature error remains visible instead of recovering scales algebraically
from the solver's own heat increments.

Six rows use fixed boundaries 0.02, 0.06, 0.10, 0.14, 0.18, 0.22 and 0.26 s.
Fit rows 0, 1 and 3; validate rows 2, 4 and 5 without refitting. Normalize both
power columns and solve with reorthogonalized two-column QR. A pivot below
`1e-4`, missing excitation, invalid data or contact prevents recovery. The pivot
is a rank diagnostic, not a statistical confidence bound.

All bounds and row choices precede the first experiment: each recovered scale
must be within 1% of its prescribed value; held-out relative energy RMSE below
0.005; relative mechanical balance residual below `1e-8`; positive energy step
below `1e-10`. The negative control fits the best single structural multiplier
to all six rows while omitting damper loss; its residual must remain above 0.01.

## Results

| True structural / damper scales | Rate kHz | Recovered structural | Recovered damper | Held-out relative energy RMSE |
|---|---:|---:|---:|---:|
| 0.5 / 1.5 | 48 | 0.499999585 | 1.499996654 | 8.798e-7 |
| 0.5 / 1.5 | 96 | 0.499999896 | 1.499999164 | 2.202e-7 |
| 1.0 / 0.5 | 48 | 0.999999081 | 0.499999670 | 1.071e-6 |
| 1.0 / 0.5 | 96 | 0.999999770 | 0.499999917 | 2.679e-7 |
| 1.5 / 1.0 | 48 | 1.499999160 | 0.999998311 | 7.357e-7 |
| 1.5 / 1.0 | 96 | 1.499999790 | 0.999999578 | 1.840e-7 |

Maximum relative structural/damper errors are `9.187e-7` and `2.231e-6`.
The held-out error drops by approximately four when the mechanical observation
rate doubles, consistent with the endpoint quadrature refinement in these cases.
The largest independent mechanical ledger residual is `1.795e-11` of injected
energy. Normalized QR pivots are above 0.9986.

The omitted-damper fit leaves relative energy residuals of 0.633..0.743, even
though it uses all rows. A good natural-loss estimate cannot compensate for
unmodeled release power. This complements the [temporal envelope counterexamples](BAND-EVENTS.md),
where a plausible decay slope could refer to either side of a release.

## Interpretation and next step

The estimator knows the correct operator shapes and every mechanical coordinate;
no observation noise is added. It recovers only two multipliers, not nine
independent loss coefficients, coupled modal decay rates, or real material
parameters. High accuracy here is an internal consistency/identifiability check
under those favorable conditions, not evidence that the reference-bank audio
provides the same information.

The next comparison should reduce the available state to modal projections and
pickup motion while preserving the known strike/release history. That will expose
which parameter combinations remain identifiable before attempting audio-based
calibration. No preset, physical loss value, magnetic conversion or audible
baseline was selected from this study. The new DSP methods expose copied
stiffness/damping matrices for research inspection only; they do not change the
equations or integration behavior.
