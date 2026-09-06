# Geometry-derived tine modes and moving-root inertia

`TineModes::prepare` computes six bending modes of a uniform circular tine with
a movable tuning mass, either concentrated at a point or distributed over an
axial span. Frequencies, effective masses, hammer/pickup weights
and root coupling follow from the same spatial modes. This is an offline
structural preparation step used by the [nine-coordinate assembly](MODAL-ASSEMBLY.md).
The earlier `AssemblyVoice` reference and the 0.1.2 instrument remain unchanged.

## Physical scope and sources

The mechanical reduction uses an Euler-Bernoulli beam, clamped at its root,
with a translational added mass representing the tuning spring. The default
remains a point mass; the [finite-span extension](FINITE-SPRING-SPAN.md) integrates
uniform co-moving inertia over a specified interval. Pfeifle's Rhodes
work motivates treating the spring as added mass. Our finite-element reduction
does not reproduce that paper's full nonlinear, nonplanar instrument model.
[DAFx 2017](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

Cubic Hermite beam interpolation retains both displacement and slope at each
node. The stiffness matrix follows the bending-energy integral, as described
in TU Delft's computational modelling course.
[Euler-Bernoulli beam elements](https://teachbooks.tudelft.nl/computational-modelling/structural_linear/euler_bernouilli.html).

This model assumes a uniform, straight, circular, slender beam, linear elasticity
and small deflections in one plane. It omits shear deformation, rotary inertia,
large-deflection effects, coil elasticity and slip, cross-sectional rotary inertia
of the tuning spring, taper, root-block geometry and a second polarization. The accepted
length/diameter ratio of at least 10 is an input guard, not proof that every
retained high mode lies within Euler-Bernoulli theory's physical accuracy range.
No geometry or damping values have been identified from a real Rhodes.

## Parameter ledger

| Default quantity | Value | Status |
| --- | --- | --- |
| Free length | 75 mm | Illustrative |
| Diameter | 1.5 mm | Illustrative |
| Young modulus | 200 GPa | Assumed material value |
| Density | 7850 kg/m³ | Assumed material value |
| Tuning mass | 0.1 g | Illustrative point mass |
| Tuning axial span | 0 mm | Point-mass baseline; finite spans are experimental |
| Tuning position | 0.85 of free length | Illustrative |
| Hammer position | 0.20 of free length | Illustrative |
| Pickup observation position | 0.98 of free length | Illustrative |

All positions are measured from the root. They are evaluated continuously using
the element shape functions; a point mass is not snapped to a nearby node.
Hammer and pickup positions are observation/excitation locations, not independent
modal gain controls. Changing those positions leaves the eigenproblem unchanged.
There is no fitted loss, hammer contact law or magnetic conversion in this step.

## Spatial discretization and modal normalization

The constructor accepts 8/16/32/64 elements, bounded to 128 unconstrained degrees
of freedom. It uses dimensionless position `s=x/L` and nodal coordinates
`[w, dw/ds]` to avoid mixing tiny physical lengths with rotation units during
factorization. Root displacement and slope are eliminated. Consistent element
mass comes from integrating products of the cubic interpolation functions.

```text
A = pi d² / 4
I = pi d⁴ / 64
mb = rho A L
Kphysical = (EI/L³) Kdimensionless
Mphysical = mb Mdimensionless
Mdimensionless += (mtuning/mb) N(stuning)^T N(stuning)

Kdimensionless phi = lambda Mdimensionless phi
frequency = sqrt(lambda EI/(mb L³)) / (2 pi)
```

Mass whitening uses Cholesky; a cyclic symmetric Jacobi eigensolver performs at
most 64 sweeps. The six lowest modes are sorted by frequency, transformed back,
and normalized so each free-tip displacement is 1. A modal coordinate consequently
has units of meters. Frequencies use the Rayleigh quotient in the original
matrices, with an independently checked force residual:

```text
residual = ||K phi - lambda M phi|| / (||K phi|| + lambda ||M phi||)
```

Preparation rejects invalid geometry, nonpositive mass factors/eigenvalues,
iteration exhaustion, ill-conditioned tip normalization, residuals above `1e-4`
or mass-orthogonality errors above `1e-8`. All matrix allocation and eigenanalysis
are bounded constructor work, not suitable for an audio callback. Shape lookup
and the exported fixed-size reduced mass matrix require no allocation.

## Physically consistent connection to a moving root

These are fixed-root bending modes. A moving-root reduction uses:

```text
w(x,t) = uroot(t) + x theta_root(t) + sum(phi_i(x) q_i(t))
```

The same shapes determine `hammer_weight=phi_i(xhammer)` and
`pickup_weight=phi_i(xpickup)`, as well as the kinetic cross coefficients:

```text
mi = integral(rho A phi_i² dx) + mtuning phi_i(xtuning)²
pi = integral(rho A phi_i dx) + mtuning phi_i(xtuning)
ri = integral(rho A x phi_i dx) + mtuning xtuning phi_i(xtuning)
```

The exported 8x8 mass matrix has coordinates `[uroot, theta_root, q1..q6]`.
Its root block contains total mass, first mass moment and second mass moment.
Its cross blocks contain `pi` and `ri` symmetrically, and its modal diagonal
contains `mi`; computed off-diagonal modal mass is below the orthogonality gate.
Three-point quadrature integrates the polynomial translation/rotation cross
terms exactly up to floating-point error. Point-mass contributions are explicit.
The completed reduced matrix must also pass a positive-definite Cholesky check.

This provides reciprocal inertial coupling to root translation and rotation.
It does not specify the tonebar geometry, root stiffness or assembled modes.
When integrating it, add the other components' inertia and support stiffness;
do not add the tine's total mass a second time or substitute arbitrary coupling
gains. The present three-coordinate assembly cannot accept these coefficients
without extending its mass matrix and coordinate system.

## Verification and audit

```text
cargo test --locked -p rf-73-dsp tine::tests
cargo run --locked --release -p rf-73-lab -- tine-modes --output renders/tine-modes.json
```

The command refuses existing output files and opens no audio device. The
[tracked report](../references/tine-modes-validation.json) contains 12 cases:
tuning-mass fractions 0/0.1/0.5 of the beam mass at positions 0/0.37/0.85/1.
Each is evaluated with 16/32/64 elements, giving 216 reported modal results.
Every case includes full SI geometry, the moving-root mass matrix and 65 spatial
samples per mode from the 64-element calculation.

| Measured quantity | Maximum |
| --- | --- |
| Relative frequency difference, 32 vs 64 elements | 0.006338% |
| Absolute hammer-weight difference, 32 vs 64 | 0.0003516 |
| Absolute pickup-weight difference, 32 vs 64 | 0.0001315 |
| Relative eigen-equation residual, all cases/meshes/modes | 1.806e-7 |

All mesh results pass the report's frequency, port-weight, residual and
orthogonality gates. Finite mesh agreement does not bound errors against a real
instrument or certify the entire accepted geometry domain.

For the illustrative 75 mm tine with the mass at 85% of free length:

| Tuning mass / beam mass | Mode 1 | Mode 2 | Mode 3 |
| --- | --- | --- | --- |
| 0 | 188.304 Hz | 1180.082 Hz | 3304.263 Hz |
| 0.1 | 168.273 Hz | 1164.367 Hz | 3297.113 Hz |
| 0.5 | 125.173 Hz | 1138.087 Hz | 3284.136 Hz |

The added mass changes each mode differently. These are predictions for an
assumed beam, not the measured tuning scale of a Rhodes. Placing the point mass
exactly at the clamped root leaves the fixed-root modes unchanged but adds to
the exported root inertia, which is covered by tests.

Six tests independently check unloaded analytical frequencies and spatial
shapes, mesh convergence, truncated static tip compliance, the continuous
dynamic tip-mass characteristic equation, dimensional length/mass scaling,
continuous tuning-mass placement, geometry-derived port weights, positive
reciprocal moving-root inertia, and invalid inputs. The unloaded six-mode
frequencies at 64 elements must agree within 0.0005%; tip-loaded tests use a
0.0006% frequency tolerance against roots of the independent characteristic
equation for mass ratios 0.1/0.5/5. These validate the stated beam equations,
not the assumptions that reduce the real instrument to those equations.

## Next physical gate

The [nine-coordinate assembly](MODAL-ASSEMBLY.md) now connects this mass matrix
and all six modes to root translation/rotation, a provisional tonebar coordinate
and nonlinear hammer contact. Its numerical audit passes; geometry identification,
pickup/filter comparison and polyphonic performance remain open.

The extended mechanical state now uses these six modal coordinates and the
derived translation/rotation inertia. Identify the tonebar and root support
geometry and avoid double-counting mass. Recheck contact, energy balance,
sampling and performance with the higher frequencies; the previous assembly's
contact subdivision and 4x base-rate conclusions do not automatically transfer.
Then compare fixed pickup geometry on both trajectories and fit measured
assembly data. No new instrument version or Desktop audition accompanies this
offline preparation milestone.
