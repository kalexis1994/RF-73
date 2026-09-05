# Nine-coordinate tine/tonebar assembly

`ModalAssembly` connects the six geometry-derived tine modes to a translating
and rotating support and one provisional tonebar bending coordinate. It is a
time-domain mechanical experiment with nonlinear hammer contact, spatial damper
loss and a complete energy ledger. It does not replace the existing plugin or
the earlier three-coordinate `AssemblyVoice` reference.

## Coordinates and physical connection

```text
x = [u, theta, q1, q2, q3, q4, q5, q6, b]
```

`u` is root translation in meters, `theta` is root rotation in radians, the six
`qi` are tip-normalized tine bending coordinates in meters, and `b` is tonebar
deflection relative to the rotating root, also in meters.

The tine uses the complete 8x8 inertia from [Tine modes](TINE-MODES.md), including
its tuning mass and all root/modal cross terms. Additional support mass and
rotational inertia are added only to their respective root entries. The tonebar
is represented by an effective point mass whose absolute transverse displacement
is `u + a theta + b`, with arm `a`:

```text
rbar = [1, a, 0, 0, 0, 0, 0, 0, 1]
M = embedded_tine_mass + support_mass_and_inertia
  + mbar rbar rbar^T
```

The tonebar's effective mass is counted once. This construction provides
reciprocal inertial coupling, including torque at the root. It is still a
lowest-order tonebar hypothesis, not a measured distributed tonebar mode shape.
The root rotation and tine higher modes extend the earlier assembly's physics;
the tonebar geometry and support parameters remain provisional.

The stiffness matrix is diagonal in these relative deformation coordinates:
support translation/rotation stiffness, the six `mi omega_i^2` tine modal
stiffnesses, and one `mbar omega_bar^2` bending stiffness. Coupling occurs through
the nondiagonal physical mass matrix rather than independently mixed oscillators.

## Hammer, pickup observation and spatial damper

At position `s` along the tine, the transverse displacement and force port are:

```text
B(s) = [1, L s, phi1(s), ..., phi6(s), 0]
w(s) = B(s)^T x
generalized_force = B(s) F
```

The same shape controls displacement and generalized force, preserving virtual
work. Hammer contact uses its actual configured position, including both root
force and moment. Pickup probes report displacement and velocity at their own
position relative to stationary ground. These probes are not pickup voltage.

Base losses are positive viscous terms for support translation/rotation and
each relative bending coordinate. Enabling the binary damper adds the positive
semidefinite term `cd B(sd) B(sd)^T` to the damping matrix. This accounts for
velocity at the felt's location instead of independently damping each tine mode.
It remains a linear grounded dashpot approximation, not a unilateral felt
contact model or continuous pedal simulation.

## Integration and energy

`ModalIntegration::Refined` uses four base ticks per output frame and a bounded
power-of-two contact subdivision from 1 to 256. It generalizes the previous
[free-motion refinement](ASSEMBLY-REFINEMENT.md) to the full inertia matrix:
Cholesky mass whitening, a balanced 18-state generator, a prepared exponential
transition and independently integrated viscous work. Preparation retains the
scaled degree-18 Taylor approximation, positive-weight quadrature and bounded
squaring. Both damper states and contact-remainder intervals are prepared.
The runtime transition and independent work are now folded into physical
coordinates during preparation; see [cost and block timing](MODAL-PERFORMANCE.md).
An optional [dissipative hammer law](DISSIPATIVE-HAMMER.md) now adds contact heat
with a separate ledger. Its default coefficient is zero, preserving this elastic
baseline and the measurements below.

Contact uses the existing nonnegative cubic potential and its discrete gradient.
The coupled midpoint response solves the positive-definite matrix
`A = M + h C/2 + h^2 K/4`. Its scalar compliance includes the complete hammer port:

```text
compliance = h^2/(2 mhammer) + (h^2/2) Bhammer^T A^-1 Bhammer
```

The refined contact force now uses a [bounded safeguarded Newton solve](MODAL-CONTACT-SOLVER.md)
with bisection fallback; the uniform reference retains 48 bisections. Separation transfers the
outgoing hammer kinetic energy to the escaped-energy ledger. Remaining intervals
inside a contact tick advance freely; force is averaged over the base tick to
preserve impulse. Retriggers retain all mechanical coordinates and velocities,
and unfinished hammer contacts are rejected. Damper changes affect dissipation,
not potential energy. Reset clears motion and the ledger.

```text
E = (v^T M v + x^T K x)/2 + active hammer kinetic/contact energy
residual = E + dissipated + escaped - injected
```

`ModalIntegration::Midpoint` supplies a separate uniform-step reference from
4 to 1024 internal steps per output frame. Modal/eigen preparation may allocate;
ticks, strikes, damper changes and reset use fixed-size state without allocation,
locking, factorization or I/O. No energy normalization or output limiter hides
integration error.

## Provisional assembly profile

| Quantity | Default assumption |
| --- | --- |
| Additional support mass / inertia | 0.030 kg / 1e-5 kg m² |
| Translation stiffness | `0.030 (2 pi 80)^2` N/m |
| Rotation stiffness | `1e-5 (2 pi 100)^2` N m/rad |
| Translation / rotation viscous loss | 5 N s/m / 0.002 N m s/rad |
| Effective tonebar mass / arm | 0.015 kg / 0.080 m |
| Isolated tonebar frequency / T60 | 190 Hz / 8 s |
| Isolated tine modal T60 | 5, 0.16, 0.055, 0.035, 0.025, 0.020 s |
| Damper position / coefficient | 0.80 of tine length / 0.2 N s/m |
| Hammer mass / maximum speed | 0.004 kg / 0.8 m/s |
| Hammer speed mapping / contact coefficient | `velocity^1.4` / 4e10 N/m² |
| Contact rate-loss coefficient beta | 0 s/m (elastic baseline) |

All values above are design assumptions. Isolated T60/frequency values are not
the eigenfrequencies and decays of the assembled system. Tine modes come from
64-element preparation of the supplied geometry; geometry is still illustrative.
This does not establish a Rhodes tuning scale or measured regulation.

## Audit and interpretation

```text
cargo run --locked --release -p rf-rhodes-lab -- modal-assembly-check --output renders/modal-assembly.json
```

The [tracked report](../references/modal-assembly-validation.json) contains 12
cases: tine lengths 50/75/120 mm, velocities 0.1/1 and rates 44.1/192 kHz. Each
40 ms take applies the damper halfway through, rounded down to an output frame.
Seven integrations per case give 84 takes: refined contact subdivisions
1/32/64/128/256 and uniform midpoint at 512/1024. The report includes complete
SI profile metadata, the physical mass matrix, coordinate peaks, impulse and
energy accounting. It refuses existing output paths and opens no audio device.

Coordinate differences use the complete physical mass matrix as their quadratic
weight, so translations and rotation are not compared as arbitrary mixed units.
Pickup velocity is also compared independently at its spatial observation point.
All samples are taken at equal physical times, without alignment or gain fitting.
Refined energy/separation observations occur at base ticks, not every microstep.

| Maximum relative difference vs refined contact 256 | Contact 32 | Uniform midpoint 1024 |
| --- | --- | --- |
| Mass-weighted displacement | 0.0005283% | 0.000004952% |
| Mass-weighted velocity | 0.01012% | 0.0008379% |
| Pickup observation velocity | 0.01176% | 0.0009772% |

The contact-32 candidate's maximum relative energy-balance residual is
`1.267e-11`; the maximum positive base-tick mechanical energy change is
`1.044e-15` of injected energy. All takes pass the energy gates, and both the
contact-resolution and independent-midpoint trajectory comparisons pass.
Finite-reference agreement is not an absolute accuracy bound or perceptual test.
These lengths do not cover the whole keyboard or the full configurable domain.

Six additional unit tests cover component mass accounting and virtual work,
continuous rigid translation/rotation when ground stiffness and damping vanish,
reciprocal hammer/tonebar impulse responses, free-transition composition and
independently integrated loss, contact/retrigger/damper energy accounting, and
invalid input rejection. The complete workspace has 133 passing tests.

## Remaining limits

The report also times one native release voice with five strikes over 250 ms
at 48 kHz, excluding preparation and including the energy ledger. This is a
substantially heavier solver than the earlier three-coordinate model. It has
not been qualified for polyphonic deadlines, WASM execution time or host use;
the timing is not a claim that a full instrument can run in real time.

The model still needs identified tonebar/support geometry and losses, further
beam validity checks, and a magnetic pickup/filter comparison on these new
trajectories. No shear/rotary beam inertia, large-deflection coupling, second
polarization, neoprene hysteresis or full action has been added here. The prepared
transition has since been optimized and synthetic polyphony measured, but contact
cost and host qualification still precede plugin integration. The audible 0.1.2
instrument remains unchanged; this offline milestone
does not produce a new listening package or invoke Desktop controls.
