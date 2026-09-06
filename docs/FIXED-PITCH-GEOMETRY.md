# Geometry sensitivity at fixed coupled pitch

This bounded structural experiment changes tine length and point tuning mass,
then restores the frozen G3 coupled frequency with spring position. Its purpose
is to test whether fundamental pitch leaves materially different higher modes
and hammer/pickup weights. It does not select a replacement sound profile or
fit the exploratory 1425 Hz reference candidate.

## Fixed protocol

```powershell
$env:CARGO_INCREMENTAL = '0'
cargo run --locked --release -p rf-73-lab -- sweep-tuned-geometry references/g3-pitch-reference-validation.json --output references/g3-fixed-pitch-geometry-validation.json
```

The destination must be a new JSON file. The command revalidates and embeds the
frozen training-only frequency receipt. The grid is fixed at 68/70/72 mm and
0.08/0.10/0.12 g, nine cases with no adaptive choice based on higher frequencies.
These values bracket the existing 70 mm/0.10 g pilot; they are hypothetical
uniform-beam and point-mass values, not measured instrument geometry.

Each cell starts with its spring at 0.85 L and moves inward or outward as needed
in 0.025 L increments, bounded to 0.50..0.95 L. The initial branch must have a
dominant first-tine coordinate. Frequency must change monotonically in the
expected direction; a bracket is refined with at most 32 bisections to within
0.0001 cent of the frozen undamped target. That tolerance is numerical precision,
not physical or measurement accuracy. Unreachable cases are retained, while
numerical or branch rejections cause a failing exit after preserving all cases.

The diameter, material, root/tonebar reduction and normalized hammer/pickup
locations remain at their defaults. Their absolute tine locations therefore
scale with length. No damper/hammer/pickup law is fitted. There is no audio
render, resampling, normalization, device launch or plugin update.

## Physical branch tracking

The study reconstructs physical displacement fields with the same finite-element
basis and mass/stiffness operators as the coupled model. Modal coordinates
alone are not comparable as that basis changes. Each local comparison uses the
average actual inertia of its two spring positions, with distributed beam,
support and tonebar contributions plus half the spring mass at each position.
Four-point Gauss integration evaluates products of cubic shapes exactly.

This corrects a limitation exposed when extending the narrow spring pilot:
using its initial 0.85 L inertia everywhere can mark remote positions as
ambiguous because their modes need not be orthogonal in the initial metric.
The wider study keeps the existing acceptance thresholds: squared normalized
field overlap at least 0.98 and runner-up at most 0.05. Both bracket endpoints
must select the same mode. Tests reconstruct orthogonality for all nine modes
at spring fractions 0.50, 0.75 and 0.95 under the local metric.

The original narrow `tune-modal-pitch` workflow retains its original metric,
search and settings. The generalized preparation supplies the same default
mass to that path; regression tests retain its prior fitted spring position.
No cross-cell mode identity is inferred by the study. Higher coupled modes are
reported by frequency rank, alongside the six ordered fixed-root tine modes.

## Linear transmission diagnostic

Each mass-normalized coupled mode reports its hammer and pickup weights and
their product. For a linear, undamped force impulse at the hammer port, this
product is the coefficient of that mode's pickup-velocity cosine response per
unit impulse. Unlike individual signed eigenvector weights, the product does
not depend on the arbitrary sign of the eigenvector. Its units are inverse
kilograms for these displacement ports.

This is a mechanical transmission diagnostic. It does not predict actual
nonlinear hammer excitation, magnetic voltage, processed recording level or
subjective brightness by itself. Changes in frequency ratios and transmission
must both remain visible before choosing a geometry for an audible experiment.

## Results (2026-09-06)

The [retained receipt](../references/g3-fixed-pitch-geometry-validation.json)
contains all nine initial structures, coarse/bisection observations, solutions
or rejection reasons, and explicit shared structural parameters. Eight cases
retune, one is unreachable, and none has a numerical or branch rejection.

| Length (mm) | Mass (g) | Center from root (mm) | Coupled rank 5 (Hz) | Coupled rank 6 (Hz) | Rank-5 linear residue (1/kg) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 68 | 0.08 | Unreachable | -- | -- | -- |
| 68 | 0.10 | 64.17993 | 1336.601 | 3867.223 | -756.694 |
| 68 | 0.12 | 60.27702 | 1390.271 | 4012.816 | -889.324 |
| 70 | 0.08 | 60.06199 | 1343.357 | 3795.957 | -979.532 |
| 70 | 0.10 | 55.74082 | 1361.884 | 3686.393 | -1068.890 |
| 70 | 0.12 | 52.50911 | 1357.257 | 3550.319 | -1106.861 |
| 72 | 0.08 | 46.12055 | 1243.460 | 3424.858 | -1022.539 |
| 72 | 0.10 | 43.07547 | 1211.507 | 3467.596 | -978.957 |
| 72 | 0.12 | 40.77328 | 1182.461 | 3518.181 | -939.067 |

All solved cases share the 196.38614698 Hz undamped target with maximum error
0.00009243 cent. Minimum local MAC across all trials is 0.999993646.
The 68 mm/0.08 g case remains at 201.319721 Hz (+42.954407 cents) at 0.95 L;
it is retained as unreachable rather than exceeding the spring bounds.
The 70 mm/0.10 g solution exactly reproduces the prior fitted center, all nine
coupled frequencies and both port weights. The frozen target also matches
exactly; embedded reference diagnostics pass through normal JSON float parsing.

At fixed fundamental, solved rank-5 frequencies span 1182.461..1390.271 Hz and
linear residue magnitudes span about 756.7..1106.9 per kilogram. At 70 mm,
rank 5 rises from 0.08 to 0.10 g and falls at 0.12 g after retuning. A simple
monotonic mass-to-brightness mapping would misrepresent this reduced model.

None of these cells reaches the exploratory 1425 Hz candidate at rank 5.
This delimits the fixed grid; it does not prove that other geometry cannot
reach it. Frequency rank across cells is not an independently established
structural identity, and the reference family remains unidentified. No closest
cell has been selected or rendered as a candidate sound.

## Follow-up and qualification

Preserve the existing audible baseline. Further identification must use more
than one independent frequency or spatial response before adopting geometry,
retaining the reference detector's inconclusive cases. The uniform tine and
point-mass spring remain explicit reductions, not measured physical dimensions.
An eventual geometry candidate needs nonlinear time-domain, sampling/headroom
and listening checks before a sound-profile change.

All 63 laboratory unit/CLI tests pass in release mode, including four new
regressions for mass-dependent modes/residues, both tuning directions and
unreachable cases, local-metric orthogonality, and reference/output protection.
Strict workspace Clippy and formatting pass. DSP equations and the audio
renderer are unchanged; no new WAVs, GUI launch or listening result are claimed.
