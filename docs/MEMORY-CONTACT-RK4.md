# Fourth-order contact experiment for the memory hammer

The [resolution diagnostic](MEMORY-CONTACT-RESOLUTION.md) identified temporal
error in surface contact and structural motion as the main limits of sampled
compressed modal steps. This milestone starts a higher-order experiment with
the same two-mass hammer against a fixed wall. The physical law and its
provisional coefficients are unchanged. Moving modal coupling remains separate.

## Continuous equations and quadratures

Let `c,t` be core/tip positions, `vc,vt` their velocities, `x=c-t` material
deformation and `e` Maxwell extension. Inside certified compression (`t>0`):

```text
F = k0*x + kc*x*abs(x) + k1*e
N = ks*t*t
c' = vc                  t' = vt
mc*vc' = -F              mt*vt' = F-N
e' = vc-vt-e/tau
Q' = k1*e*e/tau          Wmaterial' = F*(vc-vt)
IF' = F                  Jnormal' = N
Wsurface' = N*vt
```

Classical RK4 integrates these ten quantities together. Heat, material work,
material force integral, normal impulse and work into the contact potential
are independent quadratures, not reconstructed from an energy residual.
RK4's positive quadrature weights preserve nonnegative stage heat and normal
impulse. Signed surface-potential work permits unloading without adhesion.

Each attempt evaluates one whole step and two half steps (12 RHS evaluations),
then commits the two half steps without extrapolation. A finite, nonnegative
normal impulse divided by the whole interval supplies the reported mean
contact force. The stored material mean force likewise uses its integrated
reaction. A fixed wall receives no moving-port work; `Wsurface` checks the
elastic penalty potential rather than adding work to that external ledger.

## Boundary, accuracy and passivity checks

`MemoryHammer::try_contact_step(h)` accepts requested intervals between 1 ns
and 1 ms. Before any stages, continuous passive energy bounds tip travel by
`h*sqrt(2*E/mt)`. The initial penetration must exceed this bound plus a floating
point margin, and the wall position must be zero. This excludes an interior
separation; endpoint-only penetration checks would not establish that.

Every RK stage and endpoint must be finite, positively compressed, and within
the existing +/-10 mm material deformation domain. The coarse/fine weighted
state metric uses core/tip kinetic, memory-coordinate, equilibrium-material
and surface-contact terms, with the same `1e-11` limit used by the modal contact
experiment. Its scale is initial energy plus absolute impulse work.

The whole trial and each accepted half step must satisfy independent checks:

- Nonnegative heat and normal-impulse increments.
- Mechanical-energy increase no greater than `1e-14` times the scale.
- Combined energy plus heat, material energy plus heat minus material work,
  and surface-potential energy minus surface work defects at most `1e-13` times
  the scale.

The assembled committed state is checked again against these energy/work
limits and a momentum defect of `1e-13` times `sqrt(2*scale*(mc+mt))`.
Rejections preserve all physical state and return no mean force. All work is
bounded and uses fixed-size stack storage. The base implicit timestep and
prepared linear-ramp material coefficients remain available for later ticks.
The internal material quadrature commit helper was renamed to reflect its use
in both free motion and contact; its numerical update is unchanged.

This explicit method is not unconditionally stable or unconditionally passive.
Accepted trials satisfy the stated numerical checks. Fourth-order convergence
is demonstrated on a smooth compressed material branch; no fourth-order claim
is made across contact boundaries or the material law's zero-deformation change
of branch. Boundaries and rejected minimum intervals use the original fine
implicit tick. The public modal contact method still uses its previous solver.

## Laboratory controller and reproduction

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-contact-check --output renders/contact-rk4.json
cargo run --locked --release -p rf-rhodes-lab -- memory-free-check --output renders/contact-rk4-free-control.json
```

Output paths must be new. The contact experiment is explicitly selected by its
command. It retains the previous certified free-motion controller and uniform
reference. A separate contact level starts at zero, halves on rejection and
grows only when both state and energy estimates are below one sixty-fourth of
their limits. Levels are bounded by 12 and by the next observation/impulse.

Each of 24 cases runs for 16 ms, with a core impulse equal to 2.5 times initial
momentum near 4 ms at the original observation boundary. Observation rates are
44.1/192 kHz, relaxation times 0.1/1/10 ms, launch speeds 0.2/0.8 m/s and tip
masses 0.1/0.5 g at 4 g total mass. The candidate's base tick and uniform
reference are approximately 1.25 ns. Every accepted interval contributes its
mean force with its actual duration to the output-frame average.

## Retained results

The [24-case/48-take audit](../references/memory-contact-rk4-validation.json)
passes the existing global energy/work, momentum, velocity, force and impulse
gates. All cases retain free recovery heat and reimpact following the impulse.
Maximum candidate errors are:

| Metric | Maximum |
| --- | ---: |
| Relative combined energy residual | 1.683e-10 |
| Relative material work residual | 1.012e-10 |
| Relative momentum residual | 1.518e-14 |
| Mass-weighted velocity RMSE / launch speed | 0.04681% |
| Output mean-force relative RMSE | 0.008282% |
| Impulse error / initial momentum | 0.008213% |

There are 265,156 accepted contact intervals replacing 5,944,539 base ticks,
or 22.42 ticks per accepted contact interval on average. Maximum accepted
contact intervals range from about 40 to 160 ns across profiles. Accuracy and
boundary rejections total 1,908 and 9,301. Only 2,793 original fine ticks remain
in the candidate trajectories, versus 5,947,333 in the free-only control.
The two adaptive trajectories need not have identical contact duration.

Including free motion, the ratio of uniform ticks to accepted intervals is
74.8–669.7 across cases. **These are interval counts, not measured speedups.**
Each RK4 attempt evaluates 12 RHS stages and performs independent checks.
Native timing, modal coupling and polyphonic/WASM deadlines remain unqualified.

The [free-only regression audit](../references/memory-contact-rk4-free-control.json)
is byte-identical to the retained pre-change report: SHA256
`BEA1EA0D727D9A0A60180AB804BE2224FEF0CB6527C049C285B466DDB3B0DB32`.
All 24 uniform-reference reports also match between the new and control audits.

Four new tests cover smooth fourth-order refinement against a finer RK4
trajectory, atomic rejection and base-preparation preservation, selected
mass/stiffness/relaxation corners, and mixed RK4/implicit motion against an
independent 1 ns implicit reference through an applied impulse. The latter
checks velocity, independent work and momentum along the trajectory. A 10 ns
implicit comparison was insufficiently resolved for its pointwise `1e-4 m/s`
gate; the reference was refined to 1 ns without loosening that gate.

All 177 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CLI help, invalid arguments and existing-report preservation were checked.
CI includes the new audit; remote CI and GUI/audio tests were not performed.

The coefficients remain uncalibrated. This is an offline integration experiment,
not a new audible plugin release. The next gate is measured execution cost and
reciprocal coupling to the moving tine/tonebar structure without relaxing work
accounting, boundary certification or the existing reference comparisons.
