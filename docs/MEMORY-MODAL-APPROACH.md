# Shared late-impulse checkpoints and force-free approach

The [local collision study](MEMORY-MODAL-CHECKPOINT.md) passes when integrators
start from an identical state immediately before impact. The full repeated
trajectory still fails. This experiment moves the common starting state back
to each late external impulse so the comparison includes the intervening
material recovery and motion toward the tine.

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-approach-check --output renders/modal-approach.json
```

## Protocol

The default-RK4 donor follows the earlier 128 ms repeated-excitation schedule
for four profiles (75/120 mm tines, 1/10 ms material relaxation). Capture its
complete physical state immediately before the 32 ms and 80 ms impulses. Each
checkpoint independently starts six 48 ms continuations with fresh controllers
and integration banks; every fork preserves all motion, material, damper,
work and heat history. Apply the corresponding 0.008 Ns impulse once, then
retain the scheduled absolute damper events. The first epoch ends before the
80 ms impulse; the second epoch starts from the donor's own checkpoint, not
from any first-epoch candidate endpoint.

The six paths and eight pairings match the local checkpoint study: default
RK4, independently capped contact/free intervals, both caps, and uniform
midpoint at 16672 and 20832 ticks per 48 kHz frame. Every continuation must
contact then separate and retain the previous independent energy/work/heat/
force checks. Whole-record and separate 2 ms trajectory gates are unchanged.

## Common force-free prefix

Locate the earliest observation frame containing positive contact force among
all six paths. The common prefix ends at the **start** of that frame, excluding
every observation that might contain part of an impact. Every retained force
sample, endpoint force and endpoint surface energy must be exactly zero. All
pairs therefore cover identical absolute times before any path contacts.

Prefix kinetic velocity and pickup velocity use the existing 1% RMSE limits.
Force power is zero, so relative force error remains null and only zero
difference passes. Endpoint differences retain separate hammer positions,
velocities, material displacement/branch extension and structural coordinates.
Structural coordinate 1 is rotational; it is not combined with translations
in a unitless maximum. Two-millisecond prefix windows are diagnostic and may
end with a shorter final window.

Prefix trajectory gates do not inherit a failure occurring later in the take.
The full continuation still requires its independent physical checks and all
trajectory gates. A missing contact or empty prefix stays unqualified, rather
than manufacturing a force-free success. Completed failures retain the JSON
report and return nonzero; existing reports are never overwritten.

This is numerical localization with an approximate donor. No physical equation,
coefficient or tolerance changes. It does not qualify realistic action, an
unconstrained hammer return, listening quality or realtime execution.

## Regression coverage

All 194 workspace tests, strict Clippy, formatting and the release laboratory
build pass. The new prefix regression excludes the earliest contact-containing
frame, distinguishes missing/empty prefixes from measured zero force, rejects
out-of-range contact frames and prevents later failures from contaminating the
prefix's trajectory-only gate.

The repeated [local checkpoint control](../references/memory-modal-approach-checkpoint-control.json)
passes all 48 continuations and is byte-identical to the previous report
(SHA-256 `06e286dd5621ef178af72763cfc53d100212fc2112fad2282e123a749365dec9`).
The continuation helper now supports bounded 8 ms or 48 ms observations;
the original 8 ms output schema and numerical results remain unchanged.
CLI help and invalid-option rejection pass. No DSP or plugin code is changed,
and no new timing, remote CI or GUI/audio qualification was performed.

## Results: approach passes, one post-impact continuation fails

The [retained study](../references/memory-modal-approach-validation.json) has
`status: fail` and returned nonzero. All 48 takes pass their individual energy,
work, heat/force and contact/separation checks. All eight common free prefixes
pass. Seven of the eight complete 48 ms checkpoint cases pass every pairing;
the second impulse of the 120 mm / 10 ms profile fails five pairings involving
the finer uniform midpoint reference. No tolerance was relaxed.

Every checkpoint state matches the corresponding pre-impulse event snapshot
in the earlier donor report, and all 48 serialized initial states match their
checkpoint. The excluded contact-containing frame leaves 4.85–34.48 ms of
common force-free observations, depending on the case.

Maximum errors across common prefixes are:

| Pair | Kinetic velocity / launch | Relative pickup velocity |
| --- | ---: | ---: |
| Default / contact cap | 0% | 0% |
| Default / free cap | 0.000000136% | 0.000000000126% |
| Default / finer midpoint | 0.000039535% | 0.00000005017% |
| Two midpoint grids | 0.000019184% | 0.00000002816% |

Contact caps cannot affect these pre-contact observations. Changing the free
cap introduces small differences, but this selected approach is well within
the existing trajectory gates when all paths inherit identical impulse states.

All default/capped RK4 pairs also pass the full continuations. Across 2 ms
sections, default/free-cap kinetic RMSE reaches at most 0.0003875% of launch
speed. The difficult second-impulse case instead diverges between RK4 and
midpoint, and between the two midpoint grids. Default/finer-midpoint kinetic
RMSE first fails in 92–94 ms (1.470%) and reaches 10.228% in 126–128 ms; the
two midpoint grids reach 5.880%. Default/finer-midpoint pickup and force errors
remain below 0.036882% and 0.176267% respectively, passing their gates.

In that case, mean-force reference power is exactly zero in all sections from
92 ms onward. At the final 128 ms endpoint, using the retained structural mass
matrix and `0.0038*delta_v_core^2 + 0.0002*delta_v_tip^2` for hammer inertia,
over 99.9999% of the default/finer-midpoint kinetic difference belongs to the
hammer. This endpoint observation does not assign every earlier error to the
hammer or establish its cause. Maximum combined, structural and hammer ledger
residuals across the whole study remain 7.683e-11, 3.086e-12 and 7.422e-11.

The findings move the next investigation to post-impact hammer/material
recovery and reference-grid sensitivity. Restart from an identical complete
state after separation and compare the ensuing force-free recovery, while
retaining the two uniform grids. That will distinguish inherited differences
from the collision from error generated during subsequent free motion. Neither
the finest supported grid nor energy closure alone establishes convergence.
The full earlier repeated-excitation failure remains unresolved.

Follow-up: the [identical-state post-separation study](MEMORY-MODAL-RECOVERY.md)
passes all selected 96–128 ms recoveries. It narrows the investigation without
changing this experiment's retained failure or establishing its root cause.

Overwrite protection preserves this failed report. The new exploratory command
is not a required passing CI qualification; its prefix regression runs in the
workspace suite, and the earlier local checkpoint qualification remains in CI.
