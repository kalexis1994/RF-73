# Two-plane tine motion and coupled action

The offline action now supports **twenty mechanical coordinates**: two transverse
blocks of nine structural coordinates, one persistent hammer and one moving
felt arm. Both directions participate in the same four-contact solve and work
ledger. This is a small-deflection mechanical extension, not a playable plugin
update or a complete three-dimensional instrument model.

## Evidence and scope

Pfeifle reports transverse tine motion in two planes and includes both in the
physical model. That supports observing a spatial trajectory rather than only
one displacement. His model also discusses large deflections; those nonlinear
terms are not implemented by this block.
[Pfeifle, DAFx 2017, sections 4–5](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

The current implementation's anisotropic support and tonebar parameters are
explicit hypotheses. The paper does not supply the ratios used here. We retain
the circular Euler–Bernoulli tine's identical bending rigidity, damping profile
and tuning-mass distribution in the two transverse directions. No modal
frequency offset, random lateral excitation, output chorus or phase modulator
creates the second component.

## Structural construction

In each block the coordinates are support translation, support bending angle,
six relative tine modes and tonebar bending. Vertical occupies indices 0–8;
horizontal occupies 9–17. The hammer and arm occupy 18 and 19. Only indices 1
and 10 are angular coordinates; other displacements are meters.

The two mass blocks come from the same prepared tine and common support/bar
inertia. They are components of one object's motion: kinetic energy contains
`m*(v_vertical^2 + v_horizontal^2)/2`, not two copies of its energy for the
same velocity. A rotation-invariance test checks that distinction. The scalar
tuning mass, its position and optional finite span enter both directions through
the existing geometry-derived mass operator.

Boundary anisotropy affects only the support translations, support angles and
tonebar bending. For each corresponding pair, with principal stiffnesses
`a,b`, angle `theta` and rotation `R`:

```text
K_pair = R * diag(a,b) * transpose(R)
K_vv = a + (b-a)*sin(theta)^2
K_hh = b - (b-a)*sin(theta)^2
K_vh = K_hv = -(b-a)*sin(theta)*cos(theta)
```

Boundary damping is constructed in the same way from nonnegative principal
coefficients. The mass matrix remains invariant under the common rotation.
Thus the coupling is reciprocal and comes from stored elastic energy and
viscous work. Equal principal values give exactly zero cross-coupling; aligned
anisotropic axes also leave the orthogonal component unforced by a vertical
strike. Coupled frequencies emerge from the assembled mass and stiffness,
rather than assigning a detuning factor to individual tine partials.

Provisional `PolarizationProfile::default()`:

| Parameter | Value |
| --- | --- |
| Boundary principal-axis angle | 0.3 rad |
| Transverse support stiffness ratio | 1.20 |
| Transverse support rotation stiffness ratio | 1.15 |
| Transverse tonebar frequency ratio | 1.08 |
| Transverse boundary damping ratio | 1.10 |
| Hammer and felt contact-normal angles | 0 rad |

`PolarizationProfile::isotropic()` provides a neutral control. The tonebar
frequency ratio squares into its stiffness ratio. It remains an effective
second bending coordinate, not an identified three-dimensional tonebar shape.

## Contact, action and observation

A contact normal `n=(cos(alpha),sin(alpha))` projects the local tine displacement
onto the hammer/felt direction. The same projected spatial port distributes
its reaction force back to both planes. Displacement and force therefore remain
a reciprocal pair. The oblique-contact control uses hammer/felt angles of 0.08
and 0.04 radians with an isotropic boundary; these are illustrative angles, not
measured hammer imperfections or a tangential-friction law.

Each scalar hammer/arm coordinate and its drive are measured along that mass's
contact normal. Tilting the normal also tilts its reduced motion axis. This does
not represent a beveled contact face on a separately constrained vertical guide;
that would require another kinematic projection and possibly tangential forces.

The pedestal, persistent hammer, bridle and felt still use the
[joint action solver](ACTION-REPETITION.md). It advances all twenty coordinates
together, retaining simultaneous contact, nonnegative forces, heat and explicit
pedestal/pedal work. No state is replaced at a repeated strike. Invalid inputs
and exhausted solves retain the prior state.

`PolarizedActionAssembly::new_polarized(...)` prepares the model.
`advance(pedestal_m,pedal_m)` accepts the same physical drive contract as the
planar action. `probe()` returns the full twenty-coordinate state plus
`pickup_displacement_xy_m` and `pickup_velocity_xy_m_s` in laboratory axes.
These are mechanical observations, not stereo audio channels or pickup voltage.
The existing scalar `pickup_velocity_m_s` remains the vertical observation.

The fixed-size midpoint matrices and action state were generalized rather than
copying the nonlinear solver. Default `ActionAssembly` and `ActionProbe` retain
their planar dimensions. A live 512-step replay is compared with the committed
planar receipt after identical JSON serialization/readback, with exact equality
of all reported states, work, heat, contact counts and pass criteria.

## Independent plane work

Total mechanical energy and contact heat are audited as before. Additionally,
each physical plane has a separate diagnostic ledger:

```text
E_p = (v_p' M_pp v_p + q_p' K_pp q_p)/2
Q_p += h * v_mid_p' C_pp v_mid_p
W_contact_p += h * v_mid_p' (Bh_p*Fh + Bf_p*Ff)
W_cross_p -= h * v_mid_p' (K_po*q_mid_o + C_po*v_mid_o)
plane_residual = E_p + Q_p - initial_E_p - W_contact_p - W_cross_p
```

The cross-plane elastic storage and damping terms prevent interpreting
`E_vertical + E_horizontal` as total structural energy: those are **diagonal
subsystem energies**. Cross work accounts for the interaction explicitly and
may have either sign. The complete global ledger uses the full matrices.

For a vertical strike with a rotated anisotropic boundary, horizontal contact
work is exactly zero. Its positive final energy and diagonal heat are supplied
by boundary coupling work. With oblique contact and an isotropic boundary,
cross-boundary work is zero and lateral motion is driven through the contact
normal. These separate controls distinguish structural transfer from direct
off-axis excitation.

## Validation and reproduction

```text
cargo run --locked --release -p rf-73-lab -- polarized-action --output NEW.json
```

The [retained receipt](../references/polarized-action-validation.json) passes
**8/8 cases and 24/24 takes** at 64/128/256 midpoint ticks per frame. The matrix
uses 75 mm at 48 kHz and 120 mm at 96 kHz, with four configurations each:
isotropic, aligned anisotropy, rotated anisotropy and oblique contacts. These
are selected paired regimes, not a full factorial register/sample-rate grid.

Each run starts from the same resting/preloaded configuration and simulates
180 ms, with key down during 10–45 and 95–130 ms, 1.5 m/s pedestal slew and
closed pedal. Both strikes must occur. Symmetry controls must have exactly
zero lateral motion. The two active spatial configurations must exceed 1 nm
lateral peak and a 0.001 nonplanarity score. `orbit_covariance_rank` in the
receipt denotes the continuous score `sqrt(1-cov(x,y)^2/(var(x)*var(y)))` over
30–180 ms; it is not an integer matrix rank or a measured ellipse fit.

Frozen gates cover every tick's energy, all four contact work identities,
stationary-drive passivity, nonnegative forces, monotone heat and coordinate
continuity. Each plane's independent work balance is checked every output
frame. Both lower resolutions must stay below 1% velocity RMSE **separately
in each axis**, and below 10 micrometers in hammer/arm position, in each of
0–45, 45–95, 95–140 and 140–180 ms. Zero lateral controls are not divided by
an undefined signal; their zero-error result remains exact.

| Maximum observed | Result |
| --- | --- |
| Total relative energy/work defect | 1.156e-12 |
| Individual relative contact-work defect | 1.744e-14 |
| Vertical relative plane-work defect | 1.139e-12 |
| Horizontal relative plane-work defect | 2.706e-14 |
| Stationary-drive energy growth | 0 |
| Vertical velocity RMSE, 64 vs 256 | 0.0460% |
| Horizontal velocity RMSE, 64 vs 256 | 0.0564% |
| Vertical velocity RMSE, 128 vs 256 | 0.00919% |
| Horizontal velocity RMSE, 128 vs 256 | 0.0113% |
| Hammer/arm position RMSE, 64 vs 256 | 0.00464 / 0.00412 micrometers |
| Joint solver sweeps | 2 |

In the highest-resolution rotated-boundary takes, lateral peaks are approximately
87 and 176 micrometers. Net work delivered to the horizontal plane is 47.35
and 14.67 microjoules respectively, with no direct horizontal contact input.
These numbers characterize the assumed configurations; they do not calibrate
an actual instrument's orbit or coupling strength.

Receipt schema 1, 109578 bytes, SHA-256
`c958b3abaa9fddcd83c7d21a59e3478b0269f8ec2989536f6067a561aa3236e7`.
The optional `--refined` command selects 256/512/1024 ticks for further studies;
the saved qualification uses the default resolutions. Existing outputs cannot
be overwritten.

## Remaining physics

This adds linear two-plane mechanics coupled to nonlinear contacts. It does
not add geometric large-deflection coupling, torsion, shear/rotary-inertia
corrections, eccentric three-dimensional coil geometry, pivot friction or
measured register-dependent anisotropy. The current boundary ratios and contact
angles need identification. The subsequent [electromechanical block](ELECTROMECHANICAL.md)
adds a spatial flux proxy, reciprocal current force, passive electrical loading
and a filtered offline render. Human listening, measured-field calibration,
realtime scheduling and integration with the playable plugin are still open.
