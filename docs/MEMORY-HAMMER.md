# Two-mass hammer with material memory

`MemoryHammer` connects the [material-memory law](HAMMER-MEMORY.md) to two
inertial coordinates and a stationary elastic contact surface. Both masses and
the internal viscous deformation continue evolving after separation. An external
core impulse can drive another impact without resetting or repositioning them.
This is an offline mechanical experiment, separate from the tine assembly and
the audible version 0.1.2 instrument.

## Mechanical hypothesis and boundaries

The core coordinate c and surface-tip coordinate t increase toward the wall at
zero. The bilateral material between them has deformation x=c-t and retains the
Maxwell extension e=x-z. Its mean ramp reaction F acts with equal and opposite
forces on the two masses. Internal tension during recovery is allowed. External
contact acts only on the tip through a nonnegative normal reaction N:

```text
mc c_ddot = -F
mt t_ddot = F-N
Vc(t) = ks max(t,0)^3/3
E = mc vc^2/2 + mt vt^2/2 + U(x,e) + Vc(t)
```

The wall uses a finite penalty stiffness, so the tip can penetrate it elastically.
This is not an exact rigid impenetrability constraint. The two-mass architecture,
mass split and surface potential are project hypotheses, not measurements of a
Rhodes tip. The small surface mass introduces an internal vibration mode whose
frequency and damping need physical identification. No source cited for the
material coupon establishes that this reduction fits a real neoprene hammer.

| Parameter | Default | Accepted domain |
| --- | --- | --- |
| Core mass mc | 3.8 g | 1..20 g |
| Tip mass mt | 0.2 g | 0.01..1 g |
| Surface coefficient ks | 1e12 N/m² | 1e10..1e13 N/m² |
| Fixed interval h | Supplied by caller | 1 ns..1 ms |
| Initial wall gap | Supplied by caller | 0..10 mm |
| Common launch speed | Supplied by caller | 0..3 m/s |
| External core impulse | Supplied by caller | -0.02..0.02 Ns |

Material parameters and the +/-10 mm internal deformation domain are inherited
from the coupon. These accepted input ranges do not guarantee accuracy or that
arbitrary repeated forcing remains within the material domain. An invalid setup,
nonfinite result or material travel beyond the domain returns an error. A fixed
size transactional material copy prevents a rejected tick from partially changing
either mass or any ledger. It does not allocate.

## Coupled step and bounded nonlinear solve

Coordinates use midpoint kinematics. The material reaction is its exact average
along the linear ramp between the old and new relative coordinates. Surface
reaction is the discrete gradient of Vc between old and new tip coordinates.
Heat uses the coupon's independent positive integral, not an energy residual.

Let ac=h²/(2mc), at=h²/(2mt), and let c_free and t_free be the old positions
advanced using the old velocities. For a trial nonnegative wall reaction N:

```text
x + (ac+at) F(x) = c_free - t_free + at N
c1 = c_free - ac F
t1 = t_free + at (F-N)
vc1 = vc0 - h F/mc
vt1 = vt0 + h (F-N)/mt
N = discrete_gradient(Vc, t0, t1)
```

The mean material tangent K=dF/dx is positive. The inner residual has derivative
1+(ac+at)K >= 1. If its value at zero is R0, the unique root lies between zero
and -R0. Moreover dF/dN=at K/(1+(ac+at)K)<1, so t1 decreases with N. The outer
residual is also monotone with derivative at least one; its bracket runs from
zero to the wall reaction evaluated at the open-contact tip endpoint.

The [direct material branch](MEMORY-MATERIAL-SOLVE.md) now attempts a checked
quadratic solution when the inner root retains the old deformation's sign.
Other cases retain the original solve. Each iterative root search has at most
12 safeguarded Newton iterations followed by at
most 64 bisections. A zero residual or floating-point stagnation terminates it
earlier. There is no open-ended iteration, attractive-contact clipping, memory
reset on separation, or deletion of an escaped hammer. The fixed-size stepping
path has no allocation, locks or I/O. These facts do not establish realtime cost.

## Independent ledgers and observables

An ideal core impulse J changes its velocity by J/mc and supplies signed work
J (vc_before + J/(2mc)). Positions and material state are untouched. Negative
work removes kinetic energy. The accumulated absolute impulse work is used only
as a residual scale. The global energy and momentum residuals are:

```text
energy_residual = E + material_heat - initial_energy - signed_impulse_work
momentum_residual = mc vc + mt vt + surface_impulse
                    - initial_momentum - applied_core_impulse
surface_impulse_increment = h N
```

Between impulses, mechanical energy cannot increase except for roundoff.
Internal stored energy can return to either mass or dissipate after contact
ends. The probe exposes both positions/velocities, the complete material state,
surface energy, mean normal force and both ledgers. Mean force is associated
with the last integration interval, not an instantaneous endpoint reaction.
Audit contact event times label step ends rather than exact crossing times.

## Validation

Six unit tests cover common free translation; comparison with nested pure
bisection through impact and separation; retained memory and explicit impulse
work during reimpact; invalid inputs and atomic rejection of excessive travel;
16 combinations of mass, surface stiffness and relaxation extremes at 3 m/s;
and an independent analytic free-motion solution in the linear Maxwell limit.

For that last test, zero equilibrium elasticity and no wall contact give
e'' + e'/tau + omega0² e=0 with omega0²=k1(1/mc+1/mt). Following an initial core
impulse, the exact damped relative mode and uniform center-of-mass translation
provide displacement/velocity references. Four times finer steps reduce the
maximum normalized mode error by more than a factor of 14.

```text
cargo run --locked --release -p rf-73-lab -- memory-hammer-check --output renders/memory-hammer.json
cargo run --locked --release -p rf-73-lab -- memory-hammer-check --output renders/memory-hammer-coarse.json --substeps 32
```

The audit runs 24 cases / 48 takes: output rates 44.1/192 kHz, relaxation times
0.1/1/10 ms, launch speeds 0.2/0.8 m/s and tip masses 0.1/0.5 g, keeping total
mass at 4 g. Both masses start at the wall. After 4 ms, rounded up to an output
frame, a core impulse equal to 2.5 times the initial momentum drives reimpact.
The state continues for 16 ms, also rounded up. Every take must demonstrate
post-impulse contact and positive heat during force-free recovery.

Candidate and reference observations share output-frame boundaries. The default
candidate interval is at most 5 ns; the reference subdivides it by four. Thus
the candidate uses 4536/1042 substeps at 44.1/192 kHz, and the reference uses
18144/4168. Optional explicit substeps reproduce coarser experiments. Existing
report paths are rejected before simulation. Auditing opens no audio device.

Energy tolerance is 1e-8 relative to initial energy plus absolute impulse work;
momentum tolerance is 1e-10 relative to initial plus applied impulse magnitude.
Temporal accuracy gates are 1% for mass-weighted velocity RMSE divided by launch
speed, 1% for maximum cumulative surface-impulse error divided by initial
momentum, and 2% for output-frame mean-force relative RMSE. Those are finite
reference comparisons, not error bounds against the continuous nonlinear model.

The [coarse resolution pilot](../references/memory-hammer-resolution-pilot.json)
at 32/128 substeps passes energy and momentum checks but fails
temporal accuracy in 20 of 24 cases. Maximum velocity RMSE reaches 92.92% of
launch speed. At 256/1024 substeps, eight cases still fail. Long relaxation,
internal oscillation and the phase-sensitive reimpact make energy conservation
alone an insufficient accuracy check. The fine reference deliberately uses much
smaller steps; it is not a proposed audio processing rate.

The [fine audit](../references/memory-hammer-validation.json) passes all 24 cases.
Maximum relative energy and momentum residuals across both resolutions are
`3.546e-11` and `2.268e-12`. Maximum normalized velocity RMSE is 0.6996%, maximum
normalized cumulative impulse error is 0.1224%, and output-frame mean-force RMSE
is 0.1238%. All takes retain nonzero heat during force-free recovery. The prior
coupon report is byte-identical after exposing its mean reaction and tangent
for this coupling.

## Next integration gate

The [stateful modal coupling](MEMORY-MODAL-COUPLING.md) now connects this hammer
to the reciprocal nine-coordinate tine assembly, with independent material,
structural and transferred-work ledgers. Its moving-surface port generalizes the
fixed-wall boundary without changing this experiment. Efficient free motion and event handling need investigation
before polyphonic or WASM timing qualification. The fixed wall does not establish
how a moving tine will respond, and the impulse driver is not a key/action model.

Material calibration, a relaxation spectrum, temperature/aging dependence and
Rhodes-specific hammer geometry remain identification work. No new listening
package is produced by this milestone.
