# Shared post-separation recovery

The [late-impulse study](MEMORY-MODAL-APPROACH.md) found a kinetic trajectory
failure after contact ceased. This experiment restarts the subsequent recovery
from identical complete physical state, removing differences inherited before
that checkpoint.

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-recovery-check --output renders/modal-recovery.json
```

## Protocol

For each 75/120 mm tine and 1/10 ms material relaxation profile, restart the
default adaptive path from the earlier donor's checkpoint before the 80 ms
impulse. Propagate to 96 ms, requiring an observed impact, energy/work closure
and a separated endpoint. Capture the complete state before the 96 ms damper
event. This reproduces the independently restarted second branch of the
approach study; it is not the uninterrupted original donor's 96 ms state.

Five continuations share that exact state and retain every work/heat ledger:
default RK4, RK4 with free-step cap 8, and uniform midpoint at 8336, 16672 and
20832 ticks per 48 kHz observation frame. Controllers and integration banks
start fresh. The 96/112 ms damper events occur once at their original times;
there are no further impulses. Observe 32 ms through 128 ms.

Every accepted interval endpoint must have exactly zero contact force and
surface energy. Audit energy and independent structural/hammer work residuals,
nonnegative heat, monotonic structural heat, finite nonnegative contact force
and positive energy increments. Compare integrated mean forces as well as
velocities. Six pairings per profile retain the existing 1% kinetic velocity,
1% pickup velocity and 2% mean-force gates over the whole record and each
separate 2 ms section. A zero-force reference requires exactly zero difference;
its relative force metric remains null.

Additional whole-record metrics separate hammer and structural kinetic errors.
Hammer inertia uses `0.0038*delta_v_core^2 + 0.0002*delta_v_tip^2`; structural
inertia uses the full coupled mass matrix. Both are normalized by the original
launch scale, then square-rooted. Core/tip velocity RMSE retains m/s units.
These diagnostics do not remove hammer motion from acceptance.

## Results

The [retained report](../references/memory-modal-recovery-validation.json)
passes all four checkpoints, 20 continuations, 24 whole-record pairings and
384 separate section pairings. All continuation interval endpoints remain
separated, and all observed mean forces are zero. Maximum section errors are:

| Pair | Kinetic velocity / launch | Relative pickup velocity |
| --- | ---: | ---: |
| Default / free cap | 0.000017184% | 0.000000001241% |
| Default / finest midpoint | 0.000017105% | 0.000000117060% |
| Midpoint 16672 / 20832 | 0.000007049% | 0.000000065706% |
| Midpoint 8336 / 16672 | 0.000030944% | 0.000000548298% |

Maximum relative combined, structural and hammer ledger residuals across
continuations are 7.713e-11, 2.976e-12 and 7.455e-11. There are no positive
mechanical-energy increments. Default/finest-midpoint whole-record hammer and
structural kinetic RMSE reach 9.534e-8 and 2.143e-10 of launch respectively.

The large earlier divergence does not recur during this selected recovery
when all paths start from identical post-impact history. This narrows the
unresolved difference to history accumulated before 96 ms and its subsequent
propagation. It does not identify the root cause, validate the earlier failing
trajectory, or prove that the finest midpoint grid is exact. The interval
around collision and early recovery still needs investigation.

## Validation and next audible milestone

Two regressions verify separate hammer/structural metrics and force/surface
separation checks, including invalid heat. Strict Clippy, formatting, release
lab compilation and all 196 workspace tests pass. CLI help, invalid-option
rejection and existing-report preservation pass. The standalone matrix is a
local research qualification; its unit regressions run in the workspace suite.
No DSP equations, coefficients, tolerances or plugin behavior changed. No new
timing, remote CI or GUI/listening qualification was performed.

The next integration milestone is an offline WAV of selected single strikes
from the new assembly. Connect its pickup displacement/velocity to the existing
magnetic conversion, qualify oversampling and decimation for this trajectory,
and retain explicit output gain, headroom and numerical receipts. Include a
decay and damper release long enough to evaluate timbre. Preserve the existing
audible plugin as a comparison. Such a preview can support listening before
realtime performance is ready, while keeping unresolved repeated-impact cases
explicitly unqualified.

Playing the new model in RackForge additionally requires substantial CPU
reduction, note lifecycle and bounded polyphony integration, followed by host
validation. Neither an offline preview nor numerical agreement establishes
Rhodes realism; physical parameter identification and action/repetition remain.
