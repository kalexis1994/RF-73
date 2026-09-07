# Moving felt damper: first functional physics block

This block adds a moving damper to the nine-coordinate tine/tonebar assembly.
It includes a massive arm, a spring and viscous arm loss, nonlinear unilateral
felt contact, continuous prescribed actuation, and a ringing-state handoff from
the existing hammer model. It is an offline physical candidate, with assumed
material parameters, not a calibrated replacement for the playable plugin.

## Sources and reduction

The original [service manual, chapter 4](https://www.fenderrhodes.com/service-manuals/1979/ch4.html)
describes a leaf-spring damper arm, adjustable tension and alignment, and
withdrawal by the bridle strap. This motivates an elastic arm rather than a
binary velocity loss. The chosen effective mass, stiffness, drive travel and
contact law are our hypotheses; they are not measurements from that manual.
The diagnostic's 2.2 mm travel is not the manual's full-action clearance.

The energy accounting follows the passive interconnected-component principle
described by [Falaize and Helie, DAFx 2015](https://dafx.de/paper-archive/2015/DAFx-15_submission_33.pdf).
That paper does not establish our damper material constants or full action
geometry. The felt uses the project's existing projected rate-dependent
contact solver, with a nonnegative discrete reaction and explicit heat.

## Equations and power ports

The nine-coordinate structure retains its full inertia, stiffness and lifted
structural damping. Its old binary damper term is excluded. At the configured
damper position, `x = B^T q` includes root translation, root rotation and all
six tine modes. The new arm has position `z`, effective mass `m`, spring `k`
and viscous loss `c`, referenced to prescribed drive position `r`:

```text
delta = z - B^T q
U_felt = k_felt * max(delta, 0)^3 / 3
M q'' + C0 q' + K q = B F
m z'' = -k (z-r) - c (z'-r') - F
F >= 0
```

Compression produces equal and opposite contact work on structure and arm.
There is no attractive felt reaction during rapid unloading. The arm inertia
and spring continue moving through separation and recontact. The felt loss is
a quadratic-elastic, rate-dependent reduction, without internal felt-memory
variables; material hysteresis and distributed leaf bending remain open.

Implicit midpoint gives the free structural and arm endpoints. A single
bounded scalar contact solve uses the sum of their positive compliances.
The same average force updates both velocities and positions. The contact
law is `F = max(0, G(delta_old,delta_new)*(1+beta*Delta_delta/h))`, with `G`
the discrete gradient of the felt potential. Its heat is evaluated from the
constitutive law, including the projected unloading branch.

For a linearly moving drive over each tick, record external work separately:

```text
W_drive = -[k*(z_mid-r_mid) + c*(v_arm_mid-v_drive)] * Delta_r
Q_arm = h*c*(v_arm_mid-v_drive)^2
Q_structure = h*v_mid^T*C0*v_mid
E + Q_felt + Q_arm + Q_structure = E_initial + W_impulses + W_drive
```

Moving the actuator can inject or extract energy. Treating it as a passive
gain reduction would hide that work. The ledger also records absolute actuator
work for diagnostic normalization. A diagnostic spatial impulse records its
work as impulse times the midpoint port velocity.

## API and gestures

`FeltDamperAssembly::from_released` copies every structural position and velocity
after hammer separation. It rejects an active hammer or enabled legacy damper.
The old ledger ends at handoff; current stored energy starts the new ledger.
No hammer energy is silently discarded. `new` also supports an isolated arm
experiment. `advance(next_drive_m)` is transactional, fixed-size and bounded;
invalid/nonfinite controls or drive speed beyond 2 m/s leave state unchanged.

`DamperDrive` maps continuous key/pedal lift to the larger requested lift,
then limits drive speed. Zero lift closes and unit lift retracts. This supplies
partial positions, release-rate variation and shared key/pedal ownership for
experiments. It is a prescribed equivalent drive, not the mechanical bridle,
release bar or calibrated MIDI half-pedal response. Simultaneous live hammer
and felt contacts are explicitly outside this block's handoff interface.

## Frozen audit and refinement

```powershell
cargo run --locked --release -p rf-73-lab -- felt-damper --output references/moving-felt-damper-validation.json
cargo run --locked --release -p rf-73-lab -- felt-damper --output references/moving-felt-damper-refined-validation.json --refined
```

Two tine lengths (75/120 mm), 48/96 kHz and six gestures give 24 cases:
held, fast release, slow release, partial lift, pedal catch, and reopen/reclose.
Every case starts from the same structure within its length/rate group,
20 ms after an elastic strike, then runs for 160 ms. The last gesture includes
a diagnostic port impulse after reopening; it is not a simulated action restrike.

The first protocol, frozen before execution in the command, uses 16/32/64
midpoint ticks per output frame. Independent gates require relative balance
defect <1e-8, felt work defect <1e-9, stationary-drive energy growth <1e-10,
monotone heat, nonnegative reaction, zero held contact, and nonzero contact/heat
after release. Pickup velocity and arm position RMSE must each stay below 1%
in three separate windows for both lower resolutions against the finest.

The original receipt is retained even though one case fails: 75 mm, 48 kHz,
reopen/reclose has 1.0324% pickup velocity error in 30-90 ms for 16 versus 64
ticks. Every take still passes its independent energy/contact checks; agreement
of energy alone cannot qualify temporal accuracy. The follow-up doubles all
three subdivisions to 32/64/128 with identical physics, gestures and thresholds.
It does not replace the failed record or relax the error gate.

## Results and limits

The original receipt retains 23/24 passing cases and the one failed coarse
recontact. Its 124844 bytes have SHA-256
`69d669c1944c03c1b607e1c5cdb08a40f102735262648ca0a9adaf2eb7bbdd97`.
The refined receipt passes all 24 cases / 72 takes. Its 125071 bytes have
SHA-256 `ab8de0ba046ce4475384b3772e74f29bde86bca0e30aac9edccddadf6ba1593b`.

| Maximum across refined cases | Result |
| --- | ---: |
| Relative total energy/work defect | 1.567e-13 |
| Relative felt constitutive work defect | 2.093e-16 |
| Stationary-drive energy growth | 0 |
| Pickup velocity RMSE, 32 vs 128 ticks | 0.2598% |
| Pickup velocity RMSE, 64 vs 128 ticks | 0.0520% |
| Arm position RMSE, 32 vs 128 ticks | 0.0188% |
| Arm position RMSE, 64 vs 128 ticks | 0.0063% |

All held controls have zero contact and felt heat. All other gestures contact
and dissipate, with up to seven contact entries in a take. The maximum average
reaction is approximately 0.9515 N for these assumed parameters. The projected
unloading branch is exercised; it dissipates released contact potential while
preventing tensile force. This is an explicit constitutive choice, not measured
felt behavior. The largest net actuator input is approximately 0.959 mJ.

Final structural energy includes static deflection under preload as well as
vibration. Ratios against a held reference must not be interpreted as audio
attenuation, monotonic half-pedal damping or acoustic realism. The reclose case
also includes an extra injected impulse. Material identification and filtered
audio measurements are needed before such conclusions.

Verification passes 95 DSP unit tests plus 14 DSP integration tests, 105 lab
unit tests and two damper CLI/receipt checks, with strict Clippy and formatting.
New unit tests independently verify the free arm against its analytic damped
oscillation, separate structural/contact work, nonnegative heat, continuity,
key/pedal ownership, post-hammer state preservation and transactional rejection.
Receipt checks preserve the failed coarse case and compare the complete
overlapping 32/64-tick summaries across independent runs. No old physics gate
was relaxed. No plugin package, audio render or host session is claimed.

The subsequent [action/repetition block](ACTION-REPETITION.md) now connects a
persistent hammer and reciprocal bridle with simultaneous contacts. The next
physical blocks are orthogonal tine motion and its couplings, and improved
magnetic and electrical transduction. Each should include a usable simulation
path and gesture-level qualification. The present block still needs measured arm/felt
parameters, register-specific geometry, longer trajectories, audio qualification
and polyphonic performance before host integration.
