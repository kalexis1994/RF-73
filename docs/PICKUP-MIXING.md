# Controlled two-mode pickup mixing

The existing nonlinear pickup laws generate fundamental-spaced sidebands from
two prescribed displacement modes. Those components disappear in the linearized
control and in the sum of independently transduced modes. This establishes a
mechanism in the present equations; it does not identify the origin of any
recorded spectral family or select a new physical profile.

## Reproduce and scope

```text
cargo run --locked --release -p rf-73-lab -- pickup-mixing --output renders/pickup-mixing.json
```

The output must be a new JSON file. The retained receipt is
[pickup-mixing-validation.json](../references/pickup-mixing-validation.json),
experiment `coherent-two-mode-pickup-mixing-v1`. No WAV, device, mechanical solve
or plugin build is needed. The command uses the unchanged production and
experimental point-pole laws described in [Pickup transfer](PICKUP-TRANSFER.md).
Their raw gain is arbitrary, not a calibrated voltage measurement.

The [cross-note study](REGISTER-FAMILIES.md) motivates testing mixing before
assigning an independent mechanical mode to each observed peak. Probe frequencies
are 196.2890625 and 1426.7578125 Hz: coherent neighbors of the G3 fundamental and
exploratory 1425 Hz family, **not fitted source resonances**. Two cosines with zero
initial phases and their exact derivatives prescribe identical motion for three
paths:

```text
x = x1 + x2 = a*cos(w1*t) + b*cos(w2*t)
v = -a*w1*sin(w1*t) - b*w2*sin(w2*t)
combined    = V(x1+x2, v1+v2)
independent = V(x1, v1) + V(x2, v2)
linear      = V(0, 1) * (v1+v2)
interaction = combined - independent
```

Subtracting complex coefficients isolates cross interaction, including changes
at the parents themselves. Subtracting spectral magnitudes would discard phase
and would not measure this difference correctly.

## Independent small-motion check

Write the unchanged law as `V=K(x)*v`. Around equilibrium,
`K(x)=K0+K1*x+O(x^2)`. For `z=o/g` and `C=0.015`:

```text
Production K1 = C/g^2 * (1-2*z^2) / (1+z^2)^(5/2)
Point-pole K1 = 3*C/g^2 * (1-4*z^2) / (1+z^2)^(7/2)
```

The quadratic cross term is `K1*(x1*v2+x2*v1)`. Its sum and difference
components are `-K1*a*b*w_plus_or_minus/2 * sin(w_plus_or_minus*t)`.
With the forward FFT convention, their imaginary peak coefficients are
`K1*a*b*w_plus_or_minus/2`. A test uses 0.0001/0.00002 mm amplitudes and checks
both laws against this expansion to relative error below `1e-6`, including phase
and absence in the two controls. This is a weak-motion prediction, not an exact
finite-amplitude formula. The receipt keeps the prediction alongside measured
complex coefficients, even for probes outside the weak-motion regime.

## Fixed probes and results

The secondary amplitude is always one fifth of the primary. Each row runs both
laws; there is no parameter search or source-audio score.

| Probe | Gap mm | Offset mm | Primary amplitude mm |
|---|---:|---:|---:|
| Small motion | 1.5 | 0.5 | 0.005 |
| Larger motion | 1.5 | 0.5 | 0.25 |
| Centered control | 1.5 | 0 | 0.25 |
| Numerical stress only | 0.5 | 0.25 | 3 |

The last row deliberately stresses sampling of the transfer and is not an
inferred instrument trajectory. Results from 2026-09-06, in dB relative to the
combined output at `f1`:

| Law | Probe | f2-f1 | f2+f1 | f2-2*f1 | f2+2*f1 |
|---|---|---:|---:|---:|---:|
| Production | Small | -47.15 | -44.75 | -100.11 | -95.20 |
| Production | Larger | -13.00 | -10.60 | -32.26 | -27.35 |
| Point-pole | Small | -50.08 | -47.67 | -96.39 | -91.49 |
| Point-pole | Larger | -15.63 | -13.22 | -28.51 | -23.60 |

The same sideband bins are at numerical residue in the independent and linear
controls. Increasing motion produces stronger relative mixing and higher-order
components. These are consequences of the existing equations, not evidence that
the larger probe matches any recorded strike velocity. At centered geometry the
linear response is exactly zero, while the combined nonlinear RMS is 0.40909
(production) or 1.20261 (point-pole), in arbitrary output units. The absent parent
makes ratios to `f1` undefined; the report retains null ratios rather than large
misleading dB values. Centering does not silence the nonlinear model.

## Sampling qualification

The common period is 16384 frames at 48 kHz, lasting 0.3413333333 seconds. Parent
bin indices 67 and 487 are integers, so the complete trajectory is periodic.
An unwindowed FFT retains DC and **all** bins through 6881 (20159.1796875 Hz),
not just the ten reported lines. There is no zero padding, interpolation or
window leakage to confuse with a generated sideband.

Each trajectory is resampled analytically at 1/2/4/8/16/32/64x density. DC uses
`1/N` normalization and other complex peak coefficients use `2/N`; Nyquist is
excluded. Parseval band power is `DC^2 + 0.5*sum(|coefficient|^2)`. Each path's
complex residual against its own finite 64x reference is normalized by the
combined nonlinear reference band power. The shared denominator also supports
the exactly zero centered linear control.

All eight cases pass the declared 32x versus 64x refinement bound of `1e-8`
NRMSE for all three paths. For combined stress output, base-rate NRMSE is
`2.43895e-4` for production and `1.48946e-3` for point-pole; at 2x it falls to
`2.73579e-11` and `4.20106e-10`, respectively. The other probes and the higher
stress densities lie below the conservative -160 dB reporting floor. Values
below that floor are unresolved residuals, not proof of zero aliasing. Small
roundoff changes need not decrease monotonically with density.

This is projection into a common ideal band, not the production FIR response.
Finite-reference agreement does not bound all possible sampling errors or
qualify instrument antialiasing. No transient, changing mode envelope, second
polarization, finite magnetic pole, electrical circuit or real-time timing is
tested here. Unit tests additionally cover FFT phase/DC/Parseval normalization,
invalid FFT input, removal of one parent, centered linear gain, stress refinement,
CLI argument rejection and preservation of existing output.

## Next model work

Use prescribed modes with independent decay envelopes to derive and verify
sideband decay and phase relations. Those predictions can constrain subsequent
comparisons with the pinned recordings, whose processing and strike velocities
remain uncertain. Keep the tuned listening pair as the audible baseline until
modal identity justifies a structural or loss change; this experiment alone does
not justify moving a mechanical resonance to 1425 Hz.

The subsequent [phase-resolved decay study](PICKUP-DECAY.md) now verifies the
weak-motion sum of modal amplitude-decay rates, its complex phase relation and
finite-amplitude departures. It uses independent phase quadrature rather than
treating a decaying trajectory as periodic.
