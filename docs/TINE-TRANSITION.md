# Localized tine-section transition

The offline section model can now finish its linear taper before the tip and
continue with constant diameter. This separates the amount of diameter change
from the location over which it occurs. It is a provisional mechanical profile,
not a reconstruction of a measured swaged tine.

## Definition and numerical treatment

With free-length fraction `s`, tip/root diameter ratio `r`, and transition end `q`:

```text
d(s) = d_root [1 + (r-1) min(s/q, 1)]
0 <= s <= 1,  0.05 <= q <= 1
A(s) = pi d(s)^2/4
I(s) = pi d(s)^4/64
```

`TineGeometry::taper_end_fraction` is `q`; `q=1` preserves the preceding
[full-length taper](TINE-TAPER.md). At `r=1`, any accepted endpoint is the same
uniform beam. Diameter is continuous at `q`, but its slope changes there.
The endpoint is a geometric coordinate, not a fitted modal-frequency correction.
The existing endpoint-diameter and maximum-diameter slenderness guards apply.

Five-point Gauss integration is split at the section corner within any
intersected element. This exactly integrates each polynomial mass/stiffness
integrand up to floating-point error without snapping the corner to the mesh.
An unsplit five-point rule would not exactly integrate a piecewise polynomial
across that corner. The beam basis is still the bounded cubic Hermite mesh;
exact integration does not remove its approximation error near the transition.
At most one element needs an extra set of five quadrature points.

Root mass moments combine the tapered segment and the cylindrical remainder.
Modal coupling and the physical inertia used for branch tracking use the same
split quadrature. The tuning mass remains separately integrated and counted once.
The default uniform branch and the previous full-length taper remain available.

The service manual's manufacturing history motivates investigating section
variation, but does not identify this profile or its dimensions for our source
instrument. [Service manual, chapter 7](https://fenderrhodes.com/org/manual/ch7.html).
This model does not add a root block, local stress model, manufacturing-induced
material changes, shear deformation, rotary inertia or a second polarization.

## Independent checks

Analytic mass moments are computed independently as a conical frustum followed
by a cylinder, about the root. Tests compare those expressions with both the
prepared moments and quadrature at 8/16/32/64 elements for `q=0.137/0.25/0.5/1`
and `r=0.9`. The off-grid endpoint 0.137 exercises an element cut on every mesh.
Six-mode static tip flexibility agrees within 0.1% with an independent 20000-step
midpoint integration of `(L-x)^2/[E I(x)]`, approaching from below.

The `q=0.137, r=0.9` probe with a 6 mm tuning-mass span converges at 16/32/64
elements; the maximum 32-vs-64 relative modal-frequency difference is below
0.03% and smaller than the 16-vs-64 difference. Moving the endpoint by 1e-7
to either side of a mesh node changes frequencies by less than 1e-6 relatively.
These checks cover specific profiles, not the whole accepted parameter domain.

The uniform limit preserves frequencies and effective masses exactly. Existing
coupled tests additionally cover `(r,q)=(0.9,0.137)/(0.95,0.25)/(1.05,0.5)`:
nine-mode physical-field orthogonality at three spring positions with point
and finite-span mass, plus strike/separation/damper passivity and energy balance
using the single-mass hammer. Memory-hammer audio needs separate qualification.

## Fixed-pitch experiment

```text
cargo run --locked --release -p rf-73-lab -- sweep-tine-transition references/g3-pitch-reference-validation.json --output renders/g3-tine-transition.json
```

The output must be a new JSON file. Four cases use `q=1/0.5/0.25/0.137` at
fixed 70 mm length, 1.5 mm root diameter, 0.95 tip/root ratio and 0.1 g point
tuning mass. Material, assembly and normalized excitation/observation positions
remain fixed. Moving the transition changes total beam mass as implied by
the geometry; mass is not independently compensated.

Each case fits only the spring center to the frozen 196.38614697959488 Hz G3
anchor. The existing 0.50..0.95 L travel, 0.025 L coarse steps, 32 bisections,
0.0001-cent tolerance and 0.98/0.05 MAC gates remain unchanged. Higher modes are
reported by frequency rank without assigning identity across different cells.
No higher-mode target or best-profile selection is introduced.

## Results and limits

The [retained receipt](../references/g3-tine-transition-validation.json) preserves
all four retuned cases and their trials. Frequencies below are coupled ranks,
not harmonic numbers. The signed residue is the product of mass-normalized
hammer and pickup weights; it is a linear mechanical transfer quantity.

| Transition end (L) | End from root (mm) | Fitted spring center (mm) | Rank 5 (Hz) | Rank 6 (Hz) | Rank 5 residue (kg^-1) |
| --- | --- | --- | --- | --- | --- |
| 1.000 | 70.00 | 58.333151 | 1336.386 | 3687.761 | -1058.913 |
| 0.500 | 35.00 | 56.093834 | 1326.229 | 3563.479 | -1118.210 |
| 0.250 | 17.50 | 51.473982 | 1312.872 | 3410.713 | -1173.862 |
| 0.137 | 9.59 | 48.018082 | 1281.005 | 3387.478 | -1154.226 |

The fitted fundamental spans 196.386139..196.386149 Hz and the minimum local
MAC is 0.999995167. Finishing the transition earlier lowers rank 5 by about
55.38 Hz across this grid after retuning, with different hammer/pickup residues.
None approaches the exploratory 1425 Hz family. This does not rule out real
section variation; it rules out these four provisional cells as explanations
of that candidate through this particular mode assignment.

The full-length cell retains the preceding 0.95-ratio fitted center, all nine
frequencies and hammer/pickup weights exactly. All 233 workspace release tests,
strict Clippy and formatting pass. No localized-transition WAV, listening,
measured-geometry or realtime qualification is claimed.

The default 75 mm memory-hammer render passes its independent checks and remains
byte-identical to the preserved uniform WAV, SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`.

Before selecting further geometry or fitting losses, compare several observed
spectral families and cross-note evidence against alternative modal assignments.
The processed G3 pilot does not identify the real section or prove that its
1425 Hz family corresponds to the model's fifth coupled mode. Additional
parameters alone cannot resolve that ambiguity. Keep the existing tuned WAV
pair as the audible baseline; any justified candidate still needs independent
nonlinear render qualification and listening after retuning.
