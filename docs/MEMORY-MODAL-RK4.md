# Fourth-order contact coupled to the nine-coordinate assembly

This experiment connects the [fixed-wall RK4 hammer](MEMORY-CONTACT-RK4.md)
to all nine existing structural coordinates in the same integration stages.
It retains the full reciprocal mass matrix, the six geometry-derived tine
modes, moving-root translation/rotation and provisional tonebar coordinate.
Material and surface coefficients are unchanged and remain uncalibrated.

## Coupled dynamics and independent work

For structural coordinates `q,v`, hammer port vector `b`, and normal force `N`:

```text
q' = v
M*v' = b*N - K*q - C*v
surface_position = b^T*q
surface_velocity = b^T*v
gap = tip_position - surface_position
N = ks*gap^2                       (certified compression only)
structural_heat' = v^T*C*v
moving_port_work' = N*(b^T*v)
surface_potential_work' = N*(tip_velocity - b^T*v)
```

The hammer core/tip equations, material memory and quadratures use the same
continuous law as the wall experiment. Thirty integrated values comprise 23
physical state coordinates and seven independent quadratures: material heat,
material work, material force integral, normal impulse, surface-potential work,
structural damping heat and moving-port work. The same normal force drives
both subsystem equations within every RK stage. No delayed or separately
interpolated surface trajectory is used.

The relative contact-speed bound is
`sqrt(2*combined_energy*(1/mt + b^T*M^-1*b))`. A full-interval compression
certificate uses this bound before any trial, plus a floating-point margin.
All stages must remain finite, compressed and within the material domain.
The existing linear free propagator handles certified separation; unresolved
boundaries fall back to the original implicit base tick.

Each trial evaluates one whole RK4 step and two half steps, then commits the
half steps without extrapolation. The original modal weighted state-error
metric and `1e-11` tolerance remain in use. Independent defects check:

- Combined mechanical energy plus both heat increments.
- Material stored energy plus material heat minus material work.
- Surface potential energy minus relative surface-potential work.
- Structural energy plus structural heat minus moving-port work.
- Hammer energy plus material heat plus moving-port work.

Each defect is limited to `1e-13` times initial energy plus absolute impulse
work; energy growth is limited to `1e-14` times that scale. Heat and normal
impulse increments must be nonnegative. The committed probes are checked again
for combined, structural-port, hammer-port and material ledger defects. The
reported contact force is the whole interval's normal impulse divided by its
duration. Rejections preserve the complete assembly and prepared base tick.

These checks qualify accepted numerical trials; explicit RK4 is not an
unconditional stability/passivity guarantee. Fourth-order convergence is tested
on a smooth compressed branch, not asserted across contact/material branch
changes. No existing tolerance is relaxed to obtain longer intervals.

## Preparation, controller and reproduction

`prepare_rk4_contact()` prepares a boxed full inverse mass and relative port
mass bound without changing physical state. `try_rk4_contact_step(level)` tries
`2^level` base ticks for levels 0–12, capped at 1 ms. Preparation owns the only
new allocation; stages and trial states use fixed-size stack storage. The
prepared payload is reported separately from the inline voice and free bank.

The laboratory's explicit RK4 controller starts contact trials at level zero,
halves on rejection and grows only when both accepted state and energy defects
are below one sixty-fourth of their limits. Contact level resets on separation.
Intervals stop at observations, impulses and damper transitions. Other
controllers and the audible plugin retain their previous integration paths.

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-rk4-check --output renders/modal-rk4.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-rk4-timing --output renders/modal-rk4-timing.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-economical-check --output renders/modal-rk4-control.json
```

Each path is audited over the existing 12 profiles and 8 ms protocol: impact,
core impulse at 2 ms, damper engagement at 4 ms and disengagement at 6 ms.
The uniform reference remains 16672 base ticks per 48 kHz observation.
New output paths are required. Timing excludes preparation and per-step audit
checks, consumes returned diagnostics, alternates uniform/candidate order for
three repetitions per profile, and performs final-state checks separately.

Three new DSP tests cover atomic preparation/rejection and base-tick continuity,
smooth fourth-order convergence, and ungrounded total-momentum conservation
through an applied impulse. Accepted trials also match a fine implicit reference
with both damper states and close both reciprocal work ledgers.

## Retained accuracy and regressions

The [new audit](../references/memory-modal-rk4-validation.json) passes all
12 cases/24 takes with the existing global gates. Maximum candidate results are:

| Metric | Maximum |
| --- | ---: |
| Relative combined energy residual | 8.851e-11 |
| Relative structural work residual | 1.470e-12 |
| Relative hammer work residual | 8.997e-11 |
| Positive relative energy increment | 5.083e-16 |
| Kinetic-metric velocity RMSE / launch speed | 0.2612% |
| Pickup velocity relative RMSE | 0.03241% |
| Output mean-force relative RMSE | 0.04748% |

The velocity difference is below the existing 1% gate but larger than with the
previous economical controller (maximum 0.004014%). Higher formal order does
not establish that this candidate is closer to a real instrument or more
accurate than every comparison path. Longer trajectories and finer independent
reference work remain necessary before adoption; no tolerances were relaxed.
The subsequent [resolution study](MEMORY-MODAL-REFINEMENT.md) separates contact,
free-motion and implicit-reference sensitivity and extends four profiles to
32 ms. Its results qualify this comparison further without changing the DSP.

There are 311,813 accepted contact intervals, replacing 9,149,014 base ticks;
only 12,133 original fixed ticks remain. Accepted contact intervals reach about
160 ns. Including certified free motion, uniform-to-accepted-interval ratios
range from 102.0 to 142.9. Different candidate trajectories can have different
contact durations; interval counts are not timing measurements.

The [economical-controller control](../references/memory-modal-rk4-economical-control.json)
is byte-identical to its prior report, SHA256
`EDF1789FE963BA99221374080B127B3F1E785B5FB42AF1E326C6E6DC894EECAA`.
The [fixed-wall RK4 control](../references/memory-modal-rk4-wall-control.json)
is also byte-identical to its prior 24-case report, SHA256
`6D611B1AA6D8400DD1335963DFFEAA6671C458433125DEE87D708EBF77F3D2D5`.
Shared hammer force/energy helpers therefore retain the recorded wall results.
All 12 uniform-reference reports match between the new and control modal audits.
All 180 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Both new commands expose CLI help and preserve existing output reports;
invalid options are rejected before creating a report. CI includes the new
audit. Remote CI and GUI/audio tests were not performed.

## Native cost

The retained [previous economical timing](../references/memory-modal-rk4-before-timing.json)
uses the `c3b7902` release binary; the [coupled RK4 timing](../references/memory-modal-rk4-timing.json)
uses this implementation. Both use Windows GNU release builds and the same
four provisional profiles, 8 ms event protocol and fine base timestep.
No builds or audits ran concurrently with the timing batches.

| Length / speed | Previous economical | Coupled RK4 | Previous/new ratio |
| --- | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 0.281906 s | 0.044516 s | 6.33x |
| 50 mm / 0.8 m/s | 0.201663 s | 0.052542 s | 3.84x |
| 120 mm / 0.2 m/s | 0.384733 s | 0.064616 s | 5.95x |
| 120 mm / 0.8 m/s | 0.263726 s | 0.062828 s | 4.20x |

Values are medians of three executions per path/profile, with all observations
retained. All 48 timing runs match their corresponding audited final positions,
velocities and controller counts. The unchanged uniform-path medians differ by
about 0.9–9.3% between batches, indicating machine-load/compiler-layout effects.
These are sequential short batches, not confidence intervals or universal
speedups. The large observed reduction still leaves 5.6–8.1 seconds of native
calculation per simulated second for a single assembly, far from realtime.

The inline voice is 6,032 bytes, eight bytes larger for the optional RK4 pointer.
The RK4 prepared heap payload is 656 bytes, in addition to the 102,960-byte
certified free bank. This path does not allocate the 37,440-byte dyadic implicit
contact bank. Payload counts exclude allocator overhead and do not describe
polyphonic host memory or stack usage.

Physical calibration, longer/wider trajectory validation, polyphonic timing,
WASM/host deadlines and audible plugin integration remain open. The new method
is an explicitly selected offline experiment.
