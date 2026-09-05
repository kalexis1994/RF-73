# Passive common-support assembly experiment

Implemented in Rust as `AssemblyVoice`, independently of the 0.1.2 plugin engine.
This is a lowest-order mechanical research candidate, not a calibrated replacement
for the existing three-mode voice. It provides an explicit reciprocal coupling
path and an auditable energy balance before higher-order structural work.

## Physical basis and reduction

The Rhodes service manual describes the tine and tonebar as the two unequal
prongs of a tuning fork and describes the loss of sustain when the tonebar is
restrained. This motivates treating their interaction mechanically.
[Rhodes manual, chapter 1](https://www.fenderrhodes.com/org/manual/ch1.html).

Pfeifle describes the tine's attachment block and its connection to the other
prong, and studies a more detailed beam and pickup model. The three-coordinate
network below is our engineering reduction; it is not a reproduction of that
paper or a set of its measured parameters.
[DAFx 2017, sections 3 and 5](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

Three absolute displacement coordinates describe the effective tine tip `xt`,
effective tonebar tip `xb`, and common support `xs`. Each prong has a spring and
dashpot relative to the support; the support has its own spring and dashpot to
ground. The hammer acts on the tine coordinate. Motion of the support transmits
equal-and-opposite forces, allowing either prong to excite the other.

This assumes one translational effective coordinate per part, unity attachment
participation and small deflections. The effective masses are not asserted to be
the total masses of the real parts. Root rotation, distributed beam motion,
actual mounting locations and modal participation are omitted. The support
represents local mounting compliance, not a shared full-keyboard harp model.
No arbitrary detuned oscillator is mixed into the output.

## Equations and energy

With `x = [xt, xb, xs]`, `M = diag(mt, mb, ms)` and `b = [1, 0, 0]`:

```text
M x'' + C x' + K x = b F

K = [ kt    0       -kt        ]
    [  0    kb      -kb        ]
    [-kt   -kb   kt + kb + ks  ]

C has the same network structure using ct, cb, cs.
An applied binary damper adds cd to C[0,0].

Eassembly = (mt vt^2 + mb vb^2 + ms vs^2)/2
          + (kt (xt-xs)^2 + kb (xb-xs)^2 + ks xs^2)/2
Ploss = ct (vt-vs)^2 + cb (vb-vs)^2 + cs vs^2 + cd vt^2

delta = xhammer - xt
Vcontact = kcontact max(delta,0)^3/3
mhammer vhammer' = -F
```

Positive masses and nonnegative stiffness/damping make stored energy
nonnegative and dissipation nonnegative. Zero stiffness is supported for
disconnected/free-body verification. Overdamped cases do not require an
underdamped square root. Parameters are finite, range-validated and immutable
after preparation; damping switches change losses without changing potential.

The solver uses implicit midpoint for the entire coupled linear system and the
existing elastic contact potential's discrete gradient for the hammer. With
step size `h`, preparation factors the positive-definite matrix
`A = M + h C/2 + h^2 K/4` using Cholesky, separately for both damper states.
Fixed matrices then give the free endpoint and response to a scalar force.

```text
v1 = vfree + h A^-1 b F
x1 = xfree + (h^2/2) A^-1 b F
delta1 = delta_free - compliance F
compliance = h^2/(2 mhammer) + (h^2/2) b^T A^-1 b > 0
F = discrete_gradient(Vcontact, delta0, delta1)
```

The same proven nonnegative bracket as the baseline contact solver is used,
with 48 bounded bisections. No Newton iteration, allocation, factorization,
locking or I/O occurs in the tick loop. Midpoint velocities measure dissipated
work independently of the energy calculation. Upon separation, outgoing
hammer kinetic energy is accounted for instead of silently disappearing.

```text
balance_residual = Eassembly + Eactive_hammer + Vactive_contact
                 + accumulated_dissipation + escaped_hammer_energy
                 - injected_strike_energy
```

The elastic-contact law has no material hysteresis. A new strike preserves all
assembly positions and velocities and explicitly adds hammer energy. A strike
during unfinished contact is rejected without mutation: full action/repetition
is not implemented. Reset clears motion and the energy ledger. No inaudibility
threshold truncates motion during an audit. There is no mechanical output clipper.

## Provisional parameter ledger

For MIDI note `n`, `f = 440 * 2^((n-69)/12)` and
`s = clamp(220/f, 0.15, 4)`:

| Quantity | Provisional value | Evidence |
| --- | --- | --- |
| Effective masses: tine, tonebar, support | `0.0015 s`, `0.015 s`, `0.030` kg | Assumed |
| Isolated frequencies: tine, tonebar, support | `f`, `f`, `80` Hz | Assumed |
| Edge stiffness | `mi (2 pi fi)^2` N/m | Derived from assumed values |
| Isolated T60: tine, tonebar, support | `5 sqrt(s)`, `8 sqrt(s)`, `0.08` s | Assumed |
| Edge damping | `2 mi ln(1000)/T60i` N s/m | Derived isolated-oscillator loss |
| Applied damper | `110 mt` N s/m | Assumed viscous approximation |
| Hammer mass | `0.004 sqrt(s)` kg | Assumed |
| Contact stiffness | `4e10` N/m² | Assumed quadratic force law |
| Hammer speed | `0.8 velocity^1.4` m/s | Assumed velocity mapping |

Isolated frequencies and T60 values are **not** the eigenfrequencies and decays
of the assembled, coupled system. The note label is not a tuning guarantee.
Do not fit these parameters directly to all observed electrical harmonics:
magnetic conversion can create additional components and intermodulation.
The existing five processed G3 recordings do not independently identify this
network's geometry, masses, mounting losses or mode shapes.

## Reproduce and interpret the audit

```text
cargo test --locked -p rf-rhodes-dsp assembly::tests
cargo run --locked --release -p rf-rhodes-lab -- assembly-check --output renders/assembly-audit.json
```

Existing report files are never overwritten. The command opens no audio device.
The tracked [audit](../references/assembly-validation.json) contains 24 cases:
notes 28/40/55/69/88/100, velocities 0.01/1 and rates 44.1/192 kHz. Each runs
at 4/16/64/128/256 internal steps per output frame for 50 ms. The binary damper
engages at the midpoint output frame (rounded down). Energy is checked at every
internal step, not only at the output grid. Every row includes its SI parameters.

Displacement/velocity errors use all three absolute coordinates, weighted by
their masses, sampled at identical physical times. They are root-sum-square
differences divided by the reference root-sum-square, without time alignment,
gain fitting or pickup/filter effects. The 256-step reference is finite;
128/256 agreement is not an absolute error bound. These measurements cover only
the documented provisional profiles; passivity corner tests do not establish
accuracy across the full configurable parameter range.

Observed on Windows GNU Rust 1.98:

| Measurement across the audit | Maximum |
| --- | --- |
| Absolute energy-balance residual / injected energy | `4.458e-10` |
| Positive single-step mechanical energy change / injected energy | `5.146e-16` |
| Displacement relative RMSE, 128 vs 256 | `0.01146%` |
| Velocity relative RMSE, 128 vs 256 | `0.01910%` |
| Displacement relative RMSE, 4 vs 256 | `15.5343%` |
| Velocity relative RMSE, 4 vs 256 | `25.8869%` |

All 120 takes pass the energy gates; 128-step results pass the 0.5% finite-reference
agreement gate. Every strike separates during the observation. The worst listed
trajectory errors occur at MIDI 100, velocity 1, 44.1 kHz.

**The 4-step candidate is stable but not accurate enough in the treble.** Midpoint
warps oscillator frequencies, and accumulated phase error matters during free
motion as well as contact. This differs from the production voice's exact free
transitions. The candidate therefore remains offline; copying it directly into
the 4x plugin would regress numerical accuracy. High-resolution offline results
are a reference for developing efficient coupled free transitions and bounded
contact refinement. No CPU deadline or listening qualification is claimed.

Independent unit tests cover the analytic two-body elastic collision,
disconnected-prong isolation, all three normal-mode families of a symmetric
fork (including a moving support), second-order convergence against continuous
sinusoidal motion, reciprocal impulse responses, conservative and dissipative
energy accounting, separation, damper engagement, retriggers and invalid input.
Mixed mass/stiffness/damping/contact corners include strongly overdamped cases.

## Next gates

1. Obtain measured assembly modal frequencies, mode shapes and losses; decide
   whether translational/rotational support coordinates and more bending modes
   are needed. Preserve the distinctions between measured, inferred and assumed.
2. Develop efficient coupled free motion and contact refinement against this
   high-resolution reference, with explicit treble phase and contact budgets.
3. Add higher tine modes and two-polarization behavior with justified participation;
   then connect the same pickup geometry to both models for controlled comparisons.
4. Fit multiple register/intensity observations, keep held-out recordings, and
   qualify real-time behavior before adding this candidate to the plugin UI.

The existing plugin, its three pickup choices and its saved-state format are
unchanged. This milestone produces a reproducible mechanical experiment, not a
new hands-on instrument version; no Desktop audition is needed for this change.
