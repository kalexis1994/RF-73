# Stateful hammer connected to the modal assembly

`MemoryModalAssembly` connects the two-mass material-memory hammer to the full
nine-coordinate tine/tonebar/support structure. It retains all eleven inertial
coordinates and the viscous material state during impact, separation, reimpact
and damper changes. This is an offline experiment; the production plugin still
uses its previous engine.

## Reciprocal mechanical connection

The structural coordinates q comprise root translation, root rotation, six tine
bending modes and one tonebar coordinate. Their mass M, stiffness K, damping C
and spatial hammer port b reuse the existing geometry-derived modal preparation.
The hammer has core coordinate c, tip coordinate t and material deformation c-t.
With nonnegative contact reaction N:

```text
M q_ddot + C q_dot + K q = b N
mc c_ddot = -F
mt t_ddot = F-N
surface_position s = b^T q
contact_penetration d = t-s
Vc(d) = ks max(d,0)^3/3
```

F is the bilateral material reaction with the retained Maxwell state. N is the
nonadhesive external reaction between tip and tine. The structural force b N
and tip force -N act at the same spatial port. Tine motion can return energy to
the hammer; transferred work is signed. The finite contact potential permits
elastic penetration and is not an exact impenetrability constraint.

This step joins existing mechanical hypotheses. It does not identify hammer-tip
mass, material coefficients, modal damping or geometry from Rhodes measurements.
The core/tip split remains 3.8/0.2 g by default, with contact coefficient 1e12
N/m². The memory material is unchanged. A core impulse is an external research
driver, not a model of the key, hammer action, escapement or backcheck.

## One coupled implicit step

Both the structure and hammer use midpoint kinematics. Structural response
matrices are prepared separately for damper off/on. For a step h:

```text
A = M + h C/2 + h^2 K/4
v_free = A^-1 [(M-h C/2-h^2 K/4) v0 - h K q0]
q_free = q0 + h (v0+v_free)/2
r = h A^-1 b
s_free = b^T q_free
compliance = h b^T r/2 >= 0
v1 = v_free + r N
q1 = q_free + h r N/2
s1 = s_free + compliance N
```

The hammer's existing nested scalar solve now accepts s_free and the prepared
surface compliance. Its trial contact gap subtracts the moving surface endpoint
s1; its outer tangent gains the positive compliance term. The material solve
and exact ramp heat integral remain unchanged. Both nonlinear residuals remain
monotone, with the same bounded safeguarded Newton/bisection strategy.

The same solved N updates all nine structural coordinates and both hammer
velocities. No contact force is clipped, no hammer is deleted at separation,
and the memory variable is not reset. On a force-free step the hammer continues
its internal recovery while the structure vibrates and dissipates independently.

Each tick advances one caller-selected fixed interval, not an output audio
frame. Preparation may allocate and factor matrices; stepping uses fixed-size
arithmetic and transactional copies without allocation, locks or I/O. Invalid
or nonfinite updates, including excessive material travel, preserve the previous
combined state. The caller receives an error rather than a partially advanced
instrument. Accepted timestep bounds are numerical input limits, not accuracy
guarantees.

`ModalAssemblyProfile` supplies structural and damper fields. Its legacy scalar
hammer mass, contact stiffness/rate and launch-speed fields are not used by this
new model; `MemoryHammerProfile` and the explicit constructor speed control the
stateful hammer. Both profiles are validated at construction.

## Three energy checks and a momentum reference

Structural heat is integrated independently as h v_mid^T C v_mid. Switching C
at a step boundary changes no coordinate, material state or stored energy.
Material heat is the analytic positive ramp integral from the coupon. Contact
potential energy belongs to the hammer-side ledger. With Wport=sum N (s1-s0):

```text
structural_residual = Estructure + structural_heat - Wport
hammer_residual = Ehammer_and_contact + material_heat + Wport
                  - initial_energy - signed_core_impulse_work
global_residual = Estructure + Ehammer_and_contact
                  + structural_heat + material_heat
                  - initial_energy - signed_core_impulse_work
```

The public probes expose each residual, transferred work, all coordinates,
pickup-port displacement/velocity and separate heat totals. Pickup velocity is
a mechanical observable; no magnetic voltage, filtering or audio is produced.
Impulse work and cumulative absolute impulse work retain their prior meanings.

Four added unit tests cover:

- All-coordinate excitation and the three energy checks through free recovery,
  externally driven reimpact and damping.
- The accelerated nested solve versus pure bisection, invalid impulse rejection
  and atomic rejection of an excessive-travel tick for the entire assembly.
- A stationary disconnected structure and a damper switch with no energy jump.
- Total linear momentum during impact when the translation support is ungrounded:
  row zero of M times structural velocity, plus both hammer momenta, stays equal
  to the initial momentum within 1e-12 Ns. Internal damping remains enabled.

## Reproducible temporal audit

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-check --output renders/memory-modal.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-check --output renders/memory-modal-coarse.json --coarse
```

Twelve cases combine tine lengths 50/75/120 mm, launch speeds 0.2/0.8 m/s and
relaxation times 1/10 ms. All other geometry and structural values retain their
provisional defaults. The 48 kHz output grid is only an observation/event grid.
Both masses start at zero gap. At 2 ms the core receives an impulse equal to
2.5 times initial momentum. The damper engages at 4 ms and releases at 6 ms;
the simulation ends at 8 ms. Every take must show contact after the impulse,
positive material heat during force-free recovery and motion in all coordinates.

The default candidate has 8336 microsteps per observation frame (about 2.5 ns)
and the reference has 16672 (about 1.25 ns). The finite reference is twice as
fine; it is not an exact continuous solution. The candidate uses midpoint for
free motion as well as contact. Such small steps are a laboratory accuracy
reference, not a proposed realtime processing rate.

Gates are 1e-8 for each relative energy/work residual and 1e-10 for positive
relative mechanical-energy increments between impulses. The normalizer is
initial energy plus accumulated absolute impulse work. Velocity comparison uses
the full structural mass matrix, including off-diagonal inertia, plus both
hammer masses, normalized by initial launch kinetic scale. Its RMSE and the
pickup-port velocity relative RMSE must each be below 1%; output-frame mean
contact-force RMSE must be below 2%. No waveform alignment, level fitting or
parameter changes are used to reduce errors.

The [preliminary resolution pilot](../references/memory-modal-resolution-pilot.json)
at 2084/8336 substeps closes its energy ledgers but fails
accuracy in two strong-strike cases. Its maximum kinetic-metric velocity RMSE
is 6.435%, pickup-velocity RMSE 2.824% and output mean-force RMSE 2.870%. The
`--coarse` option preserves this experiment instead of weakening the gates.
Existing report files are rejected before simulation.

The [final audit](../references/memory-modal-validation.json) passes all twelve
cases. Maximum global, structural-port and hammer-port relative residuals are
`9.512e-10`, `9.507e-10` and `3.513e-12`. Maximum kinetic-metric velocity RMSE
is 0.7693%, pickup-velocity RMSE is 0.09629%, and mean-contact-force RMSE is
0.1406%. The prior 48-take fixed-wall audit is byte-identical after the moving
surface extension, including all its stored diagnostics and comparisons.

## Remaining gates

Efficient free-motion integration, event-aware contact refinement and measured
timing remain necessary before this model can approach polyphonic realtime use.
The next numerical milestone should improve accuracy per unit cost against the
retained fine reference. Material/geometry identification and comparisons with
direct recordings remain separate realism gates. There is no new plugin package
or listening release in this milestone.
