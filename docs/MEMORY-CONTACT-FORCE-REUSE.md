# Reusing the converged material reaction

The two-mass memory hammer solves a material deformation inside each normal
contact residual. Previously, it solved that material equation again after
the outer root returned, even when the last residual evaluation had already
computed the reaction at that exact normal force.

The contact step now retains that pair of scalar values locally. It starts
with the already solved open-contact reaction at zero normal force. Each
outer residual evaluation updates the pair. After convergence, the step
reuses the reaction only if the returned normal force equals the evaluated
normal force exactly. If the bounded bisection fallback returns a new midpoint,
the original material solve runs at that point.

This removes a redundant inner solve on matching endpoints, including empty
contact brackets. It does not change Newton or bisection iterates, brackets,
iteration limits, material roots, acceptance tolerances, force integration or
energy accounting. The pair lives only within one call: it cannot survive a
material update, impulse, step-size change, rejected trial or moving-surface
change. No allocation or persistent state is added. Transactional material
updates and rejection behavior remain in place.

## Reproduction

Run the timing command on the parent commit and this commit in separate
release builds. Keep compilation and other audits outside timing runs. Every
output path must be new.

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-economical-timing --output renders/force-reuse-timing.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-economical-check --output renders/force-reuse-economical.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-adaptive-check --output renders/force-reuse-strict.json
cargo run --locked --release -p rf-rhodes-lab -- memory-free-check --output renders/force-reuse-wall.json
```

## Trajectory regression

Both 12-case/24-take modal audits pass and preserve their previous reports
byte for byte, including every reported candidate/reference state, work and
energy metric, trajectory error, acceptance count and rejection count:

- [Economical controller](../references/memory-contact-force-reuse-economical-validation.json),
  compared with `memory-modal-economical-validation.json`: SHA256
  `EDF1789FE963BA99221374080B127B3F1E785B5FB42AF1E326C6E6DC894EECAA`.
- [Strict controller](../references/memory-contact-force-reuse-strict-validation.json),
  compared with `memory-modal-adaptive-validation.json`: SHA256
  `D209D20B9F6D8E73581FB1157BF66B55A7F2C1B372A64A20C7EAF156DD2123AB`.

The [fixed-wall audit](../references/memory-contact-force-reuse-wall-validation.json)
also passes all 24 cases/48 takes and is byte-identical to
`memory-material-solve-wall-validation.json`: SHA256
`BEA1EA0D727D9A0A60180AB804BE2224FEF0CB6527C049C285B466DDB3B0DB32`.
Together these audits cover 48 cases/96 takes. Identity is established for
these retained protocols and this toolchain, not every possible input.

All 172 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Existing tests cover independent nested bisection, passivity, separation,
reimpact, reciprocal port work, signed impulses and atomic rejected updates.
No GUI/audio test or remote CI run was performed.

## Native timing

The retained [before](../references/memory-contact-force-reuse-before-timing.json)
batch uses `9970b0b`; the [after](../references/memory-contact-force-reuse-after-timing.json)
batch changes only reaction reuse. Both use Windows GNU release builds,
three repetitions per path/profile, and the same 8 ms impulse/damper protocol.
Within each batch, uniform and economical paths alternate their execution order.
Preparation is excluded from the execution timer.

| Length / speed | Economical before | Economical after | Before/after ratio |
| --- | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 0.302138 s | 0.282466 s | 1.070x |
| 50 mm / 0.8 m/s | 0.232702 s | 0.216593 s | 1.074x |
| 120 mm / 0.2 m/s | 0.439156 s | 0.407084 s | 1.079x |
| 120 mm / 0.8 m/s | 0.310126 s | 0.301016 s | 1.030x |

These are median execution times. Economical-path elapsed time decreases by
2.9–7.3%; uniform-path medians decrease by 4.5–12.9%. All 24 before/after run
pairs have identical reported final positions, velocities, energy residuals
and controller counters. No observations are discarded.

Before and after are sequential batches, not interleaved binaries; machine
load and compiler layout can influence these short measurements. The result
supports retaining this small optimization but is not a confidence interval
or a universal speedup. The economical path still takes about 27–51 seconds
per simulated second on these cases, far from realtime operation.

This optimization affects the offline memory hammer and its coupled modal
experiment. Physical calibration, longer-trajectory accuracy and realtime
qualification remain open; the audible 0.1.2 plugin still uses its existing
mechanics.
