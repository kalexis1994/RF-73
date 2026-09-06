# Phase-resolved pickup decay

The existing pickup laws recover the sum of two modal amplitude-decay rates in
weak-motion sum/difference sidebands. At larger motion, higher-order mixing
slightly changes the fitted rate. A measured sideband's decay therefore cannot
be assigned directly to an independent mechanical mode.

This extends [controlled periodic mixing](PICKUP-MIXING.md) with independent
exponential modal envelopes and exact damped-motion velocity. It is an offline
mechanism experiment, not a fit to recordings or a new plugin sound.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-decay --output renders/pickup-decay.json
```

The output must be a new JSON file. The retained receipt is
[pickup-decay-validation.json](../references/pickup-decay-validation.json),
experiment `phase-resolved-pickup-decay-v1`. Code is Rust; no WAV or device is
used. Both production and point-pole transfer laws are reused without changing
their equations. See [Pickup transfer](PICKUP-TRANSFER.md) for their derivation,
arbitrary gain and physical limitations.

## Damped-motion prediction

For each prescribed mode, amplitude decay is `lambda_i >= 0` in inverse seconds:

```text
theta_i = omega_i*t + phi_i
x_i = a_i*exp(-lambda_i*t)*cos(theta_i)
v_i = a_i*exp(-lambda_i*t)*[-lambda_i*cos(theta_i) - omega_i*sin(theta_i)]
```

The envelope derivative `-lambda_i*x_i` is part of velocity. Retaining only the
carrier derivative violates `v=dx/dt` and changes the pickup phase. These rates
describe displacement amplitude, not energy: for an isolated linear mode, the
energy envelope has twice the amplitude-decay exponent. They are prescribed
probes here, not inferred loss coefficients for the coupled instrument.

Write the magnetic law as `V=K(x)*v`, with equilibrium expansion
`K(x)=K0+K1*x+O(x^2)`. Subtracting the independently transduced modes from the
combined response leaves, to leading order,

```text
V_cross = K1*(x1*v2+x2*v1) = K1*d(x1*x2)/dt
Lambda = lambda1 + lambda2
Omega_plus_or_minus = omega2 +/- omega1
psi_plus_or_minus = phi2 +/- phi1

C_plus_or_minus(t) = K1*a1*a2/2 * exp(-Lambda*t)
                     * (-Lambda + i*Omega_plus_or_minus)
                     * exp(i*psi_plus_or_minus)
```

Here `C` is a complex peak coefficient using the forward Fourier convention;
the real carrier contribution is `Re[C(t)*exp(i*Omega*t)]`. This predicts both
the decay rate and the phase, including the envelope derivative. The expression
uses the independently derived `K1` from the preceding mixing study. It is a
quadratic approximation, not an exact finite-amplitude identity for the full
pickup laws.

## Measurement method

A decaying signal is not periodic across the earlier FFT window. Instead, this
experiment computes two-dimensional periodic quadrature over independent carrier
phases at each fixed envelope age. Amplitudes and their derivatives stay frozen
at that age. Projection onto `exp(-i*(theta2 +/- theta1))` isolates the two
phase-index combinations. Initial phases remain in the coefficients; carrier
time evolution is demodulated out.

This is an instantaneous description of prescribed motion through a memoryless
pickup. It is **not a temporal FFT or a method for recovering modal envelopes
from a single recording**. A recording follows only one trajectory through these
phases. Overlapping components, windowing and noise need a separate estimator.

Each age retains combined, independent, linearized and cross-interaction complex
coefficients, plus a deliberately incorrect response omitting envelope velocity.
Grids have 32, 64 and 128 points per phase, with peak normalization `2/N^2`.
The receipt keeps 32/128 and 64/128 interaction-coefficient differences; the
latter must be below `1e-8` relative to the reference interaction. Independent
and linear controls must also be below `1e-8` of that interaction. This checks
the two selected phase coefficients, not an entire temporal spectrum or a
production decimator.

An unweighted straight-line fit to log amplitude across six ages estimates the
decay rate freely. The expected sum is used only afterward for comparison.
The report retains log-amplitude fit RMS, exposing deviations from one exponential.
Weak probes additionally require complex quadratic relative error below `1e-3`
and fitted decay absolute error below `1e-3 /s`. Larger probes retain approximation
errors without using them as pass/fail thresholds.

## Fixed cases and results

Both laws use gap 1.5 mm, offset 0.5 mm, parent frequencies 196.2890625 and
1426.7578125 Hz, and initial phases 0.37 and -0.61 radians. These are the prior
study's probes, not measured resonance identities. Secondary amplitude is one
fifth of primary. Ages are 0, 0.05, 0.1, 0.2, 0.35 and 0.5 seconds.

| Probe | Primary amplitude mm | lambda1 /s | lambda2 /s |
|---|---:|---:|---:|
| Weak unequal decay | 0.005 | 0.8 | 5 |
| Larger unequal decay | 0.25 | 0.8 | 5 |
| Weak equal decay | 0.005 | 2 | 2 |
| Weak zero decay | 0.005 | 0 | 0 |

Results from 2026-09-06; lower and upper sideband fits agree to the displayed
precision, but their full coefficients and separate fits remain in the receipt:

| Law | Probe | Expected rate /s | Fitted rate /s | Max quadratic complex relative error |
|---|---|---:|---:|---:|
| Production | Weak unequal | 5.8 | 5.799998 | 1.95010e-6 |
| Point-pole | Weak unequal | 5.8 | 5.800008 | 7.40951e-6 |
| Production | Larger unequal | 5.8 | 5.793595 | 0.00545021 |
| Point-pole | Larger unequal | 5.8 | 5.816038 | 0.01562069 |
| Production | Weak equal | 4 | 3.999997 | 1.95010e-6 |
| Point-pole | Weak equal | 4 | 4.000013 | 7.40951e-6 |
| Production | Weak zero | 0 | 0 | 1.95010e-6 |
| Point-pole | Weak zero | 0 | 0 | 7.40951e-6 |

All eight cases qualify. Maximum 64/128 interaction refinement is below
`1.5e-14`; maximum null-control relative amplitude is below `1.7e-12`.
These are finite numerical comparisons, not an absolute error bound.

In the weak unequal production probe at age zero, omitting the envelope derivative
changes lower sideband phase by about -0.0007502 radians and upper by -0.0005687.
Correct quadratic phase discrepancy is below `5e-10` radians. Unit tests also
use faster 35/85 /s decay probes to expose the omission above 0.01 radians.
Those rates are numerical probes, not instrument measurements.

Larger-motion fits depart from 5.8 /s because higher powers of the amplitudes
contribute to the same phase-index combination with different decay rates.
The exact envelope is therefore not generally one exponential. Fit RMS is about
`1.71e-4` (production) and `2.81e-4` (point-pole) in natural-log amplitude,
compared with below `2e-7` in the weak unequal probes. No law or geometry is
selected by these results.

## Validation and next step

Tests check velocity against a centered finite difference of actual damped
displacement, weak complex mixing against the analytic prediction, null controls,
the deliberate velocity omission, free decay recovery and invalid observations,
phase-grid refinement, removal of one parent, and the CLI report/output contract.

The [recorded families](REGISTER-FAMILIES.md) still lack independently known
modal identity and displacement. Next validate a temporal envelope estimator on
synthetic mixtures with known rates, nearby components and noise, then apply
qualified observations to the pinned source recordings. Decay and phase relations
should constrain possible explanations before fitting mechanical losses. The
current tuned listening pair and production model remain unchanged.

The subsequent [temporal envelope study](COMPONENT-ENVELOPE.md) implements
fixed-carrier complex measurements on audio and validates known-rate mixtures,
neighbors and noise at two window lengths. It also demonstrates that an
undeclared unresolved mixture can pass the conditional gates.
