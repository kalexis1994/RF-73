# Finite tuning-mass span at fixed coupled pitch

The tuning mass can now occupy a finite axial interval instead of a single
point. Four provisional widths regain the same coupled G3 pitch but alter
upper frequencies and mechanical transfer weights. None explains the observed
1425 Hz candidate. No width is selected as instrument geometry or a new sound.

## Physical assumption

The original service manual describes a sliding, friction-fit length of coil
spring. It also distinguishes later tapered and swaged tines from the original
uniform design. It does not supply the spring dimensions needed to reconstruct
the acquired 1977 reference instrument. These widths are sensitivity probes,
not measured settings. [Service manual, chapter 7](https://fenderrhodes.com/org/manual/ch7.html).

For total tuning mass `mt`, center `xc` and axial width `w > 0`, assume uniform
line density `mt/w` whose transverse motion follows the tine at every axial
position. The added finite-element inertia is

```text
delta_M = integral[xc-w/2, xc+w/2] (mt/w) N(x)^T N(x) dx
delta_total_mass = mt
delta_first_mass_moment = mt xc
delta_second_mass_moment = mt (xc^2 + w^2/12)
```

This uses the consistent-mass integral described in the
[TU Delft dynamics notes](https://teachbooks.tudelft.nl/computational-modelling/dynamics/semi_discrete.html)
and their [cubic Hermite beam interpolation](https://teachbooks.tudelft.nl/computational-modelling/structural_linear/euler_bernouilli.html).
Those sources support the discretization, not the physical accuracy of this
particular spring reduction. No stiffness is added. Coil elasticity, friction,
slip and intrinsic cross-sectional rotary inertia remain absent. This is also
not a rigid sleeve: different axial locations follow different tine deflections.

`TineGeometry::tuning_span_m` defaults to zero and retains the exact point-mass
branch. Positive intervals must fit entirely on the tine and remain numerically
resolvable. Four-point Gauss integration on each intersected element integrates
the cubic-shape products exactly up to floating-point error. Root translation,
root rotation and modal coupling use that same mass distribution. The
`mt w^2/12` term is axial mass spread, not a separate helix rotary-inertia model.

## Frozen-pitch experiment

```text
cargo run --locked --release -p rf-73-lab -- sweep-spring-span references/g3-pitch-reference-validation.json --output renders/g3-spring-span.json
```

The output must not exist. The command opens no device and writes no WAV.
The [retained receipt](../references/g3-spring-span-validation.json) includes all
four cases, trials, parameters and the frozen reference. Each uses the same
70 mm uniform blank and 0.1 g mass, with widths 0/2/4/6 mm. Only the spring center
is fitted within 0.50..0.95 L, starting at 0.85 L, in 0.025 L coarse steps and
at most 32 bisections. The target is 196.38614697959488 Hz, with 0.0001-cent
tolerance. Local physical-field MAC must remain at least 0.98 and its runner-up
at most 0.05. The local metric integrates both compared spring distributions;
it does not reuse a point mass at the initial position.

All four cases retune. Coupled ranks below are ordered by frequency, not
harmonic numbers. Signed residues are products of the mass-normalized hammer
and pickup weights in kg^-1; they are linear mechanical transfer quantities,
not predicted electrical audio amplitudes.

| Width (mm) | Fitted center (mm) | Rank 5 (Hz) | Rank 7 (Hz) | Rank 9 (Hz) | Rank 9 residue (kg^-1) |
| --- | --- | --- | --- | --- | --- |
| 0 | 55.740822 | 1361.884 | 6961.230 | 18196.671 | -828.218 |
| 2 | 55.735962 | 1361.545 | 6965.721 | 18173.989 | -838.438 |
| 4 | 55.721355 | 1360.530 | 6978.569 | 18105.638 | -866.934 |
| 6 | 55.696976 | 1358.852 | 6998.545 | 17996.202 | -908.207 |

The fitted fundamental remains between 196.386143 and 196.386155 Hz. At 6 mm,
rank 5 moves down by about 3.03 Hz, away from the exploratory 1425 Hz family
reported in [post-tuning observations](POST-TUNING-MODES.md). The uppermost
retained rank moves down by about 200.47 Hz. These are model sensitivities;
the slender-beam reduction is not physically qualified for those high modes.
The processed reference layers do not identify a spring width, mechanical mode
identity, or natural damping. No upper frequency was fitted.

## Verification and next gate

All 227 workspace tests pass in release mode, as do strict workspace Clippy
and formatting. New checks cover polynomial mass moments, the exact uniform
density equivalence for a full-length mass distribution, the shrinking-span
point limit, and 16/32/64-element convergence for a 6 mm probe. The 32-vs-64
relative frequency difference in that test stays below 0.01%; this does not
bound the entire accepted geometry domain. Nine coupled modes retain mass
orthogonality at spring centers 0.50/0.75/0.95 L for point and 6 mm cases.

A finite-span coupled strike and damper regression checks contact separation,
passivity, dissipation and energy balance. It uses the single-mass hammer and
is not a qualification of finite-span memory-hammer audio. The existing
75 mm point-mass memory-hammer preview passes its render checks and is byte
identical to the preserved WAV, SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`.
No finite-span listening result, plugin update or realtime qualification is claimed.

Keep the tuned point-mass WAV pair as the audible baseline. The next useful
structural investigation is nonuniform tine geometry, supported by the service
manual's manufacturing history, with explicit provisional dimensions and
analytic/convergence checks. Additional measured modal or spatial evidence is
still required before selecting geometry. Any justified candidate must then
pass nonlinear render qualification and a listening comparison after retuning.
