# Linear tine-section sensitivity at fixed coupled pitch

The offline tine model now supports a linearly varying circular diameter.
Mass, bending stiffness, root inertia and spatial coupling derive from the
same section. This is an explicit geometry experiment, not an identified
manufacturing profile or a selected sound.

## Physical scope

The service manual describes tapered tines produced by centerless grinding
and later by swaging. It motivates examining a nonuniform section but does
not establish a full-length linear taper or its dimensions for the acquired
reference instrument. The 1.905 mm diameter in that chapter belongs to the
original design and is not used here as a measured 1977 dimension.
[Service manual, chapter 7](https://fenderrhodes.com/org/manual/ch7.html).

For normalized position `s=x/L`, root diameter `d0` and tip/root ratio `r`:

```text
d(s) = d0 [1 + (r-1)s]
A(s) = pi d(s)^2 / 4
I(s) = pi d(s)^4 / 64
Mij = integral rho A(x) Ni(x) Nj(x) dx
Kij = integral E I(x) Ni''(x) Nj''(x) dx
```

This extends the existing Euler-Bernoulli weak form. Cubic Hermite mass
products times quadratic area have degree eight; five Gauss points per
element are therefore used. Four points would not integrate those products
exactly. Curvature products times quartic rigidity have degree six. The same
distributed mass weights reconstruct physical fields for mode tracking and
the moving-root cross coefficients. Spring mass remains a separate point or
finite-span contribution, counted once.

`TineGeometry::diameter_m` denotes the root diameter and
`tip_diameter_ratio` defaults to one. The uniform branch retains its original
element matrices and modal-coupling arithmetic. Ratios must be finite within
0.5..1.5, both endpoint diameters within 0.3..3 mm, and free length divided by
the maximum diameter at least ten. These are numerical/domain guards, not
physical accuracy guarantees. `beam_mass_kg()` integrates the actual section;
`bending_rigidity_n_m2()` returns root rigidity. The internal scaling mass is
the equivalent uniform cylinder at the root diameter, not the actual total
mass of the tapered beam.

The model still omits shear deformation, material rotary inertia, nonplanar
motion, large-deflection effects and local manufacturing features. A real
swaged part need not taper along its entire free length. A ratio above one is
a reverse-taper sensitivity control, not a claim about the instrument.

## Independent checks

For `m0=rho pi d0^2 L/4`, a conical frustum has the exact moments

```text
mass = m0 (1+r+r^2)/3
first moment = m0 L (1+2r+3r^2)/12
second moment = m0 L^2 (1+3r+6r^2)/30
tip flexibility = integral (L-x)^2/[E I(x)] dx = L^3/(3 E Iroot r)
```

Tests compare the prepared root moments and integrated mass samples against
these expressions for ratios 0.5/0.9/1/1.5. Six-mode tip flexibility approaches
the analytic value from below within 0.1%. That static comparison checks
stiffness independently of the mass distribution. Additional tests use
16/32/64 elements at ratios 0.5/0.9/1.05/1.5 with a 6 mm tuning-mass span;
32-vs-64 relative frequency errors stay below 0.02% and decrease by more than
a factor of five from the coarser comparison. Doubling length and tuning-mass
span, and doubling the tuning mass, divides modal frequencies by four and
doubles effective masses.

Coupled-field tests check all nine modes at centers 0.50/0.75/0.95 L, ratios
0.9/1/1.05 and spring widths 0/6 mm. A single-mass-hammer strike and damper
regression at those three ratios checks separation, energy balance, passivity
and dissipation. This is not finite-taper memory-hammer audio qualification.

## Frozen-pitch protocol

```text
cargo run --locked --release -p rf-73-lab -- sweep-tine-taper references/g3-pitch-reference-validation.json --output renders/g3-tine-taper.json
```

The output must not exist. Four fixed ratios, 0.90/0.95/1/1.05, use the same
70 mm length, 1.5 mm root diameter, 0.1 g point mass and assembly defaults.
Material and normalized hammer/pickup positions remain fixed. Beam mass is
allowed to change as the specified physical section changes. Each case fits
only the spring center to the frozen 196.38614697959488 Hz G3 anchor, within
0.50..0.95 L and 0.0001 cent. The original bounded search and 0.98/0.05 MAC
gates remain in force, with the physical inertia appropriate to each section.
No cross-case mode identity or higher-frequency fitting is performed.

## Results

The [retained receipt](../references/g3-tine-taper-validation.json) includes all
four retuned cases and their complete trials. Coupled ranks are ordered by
frequency and are not harmonic numbers. Residues are the signed product of
mass-normalized hammer/pickup weights, not electrical output amplitudes.

| Tip/root ratio | Fitted center (mm) | Rank 5 (Hz) | Rank 6 (Hz) | Rank 9 (Hz) | Rank 5 residue (kg^-1) |
| --- | --- | --- | --- | --- | --- |
| 0.90 | 60.426083 | 1301.832 | 3642.804 | 16377.692 | -1031.159 |
| 0.95 | 58.333151 | 1336.386 | 3687.761 | 17189.819 | -1058.913 |
| 1.00 | 55.740822 | 1361.884 | 3686.393 | 18196.671 | -1068.890 |
| 1.05 | 52.530762 | 1373.127 | 3671.938 | 18621.459 | -1048.498 |

The selected fundamental spans 196.386138..196.386149 Hz and the minimum local
MAC is 0.999994329. Narrowing the tip by 10% lowers rank 5 by about 60.05 Hz
after retuning. None of these cells reaches the exploratory 1425 Hz family;
the full-length narrowing moves this mode away from it. A reverse taper is
not selected merely because it moves that one mode closer. Predictions for
the highest retained modes are particularly limited by the beam assumptions.

The ratio-one cell reproduces the prior fitted spring center, all nine
frequencies and hammer/pickup weights exactly. The default 75 mm memory-hammer
preview also passes its render checks and reproduces the preserved WAV
byte-for-byte, SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`.
All 230 workspace release tests, strict Clippy and formatting pass. No tapered
WAV, listening result, plugin update or realtime qualification is claimed.

The next structural question is where section variation occurs, including a
localized root transition instead of a full-length slope. That requires a
separately stated profile and independent mechanical checks. Geometry selection
still needs several independent modal or spatial observations: fitting the
fundamental and one unassigned spectral peak would not identify the real part.
The existing tuned point-mass WAV pair remains the audible baseline.
