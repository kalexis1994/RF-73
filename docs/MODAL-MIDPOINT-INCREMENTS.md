# Incremental modal midpoint propagation

The [32 ms resolution study](MEMORY-MODAL-REFINEMENT.md) exposed a structural
work residual of 7.327e-9 in the finest uniform reference, close to its 1e-8
gate. The prepared modal midpoint operator multiplied velocity by a rounded
near-identity matrix at every tick. Repeated preparation error could therefore
accumulate even in the absence of any physical forces.

## Same discrete equations, smaller prepared coefficients

For the nine-coordinate structure, constant interval force `F`, and port `b`:

```text
M * (v1 - v0) = h * (b*F - K*(q0 + q1)/2 - C*(v0 + v1)/2)
q1 - q0 = h * (v0 + v1)/2
A = M + h*C/2 + h^2*K/4
```

The previous prepared free-velocity update was:

```text
v_free = A^-1 * (-h*K*q0) + [A^-1 * (M - h*C/2 - h^2*K/4)] * v0
```

It is now evaluated in increment form:

```text
delta_v = [A^-1 * (-h*K)] * q0 + [A^-1 * (-h*C - h^2*K/2)] * v0
v_free = v0 + delta_v
v1 = v_free + h*A^-1*b*F
```

The small velocity coefficients are formed directly from `C` and `K`. They are
not obtained by subtracting the identity from the old rounded propagator.
The force response, contact compliance, position update, independent damping
quadrature and hammer port work retain their existing formulas. No energy is
projected back onto a target, and heat is not inferred from endpoint energy.

For `K=C=0`, every prepared increment coefficient is exactly zero. An unforced
nonzero velocity therefore remains exactly constant despite dense reciprocal
inertia. The previous `inverse(M)*M` product need only differ from the identity
by roundoff to introduce a systematic error at each tick. Reducing `h` creates
more such multiplications per simulated second without eliminating that error.
The increment form removes this particular mechanism; state additions, inverse
preparation, force solves and work accumulation still have finite precision.

This shared internal operator is used by both the older modal assembly and
the stateful hammer assembly, including prepared implicit contact half steps
and RK4 boundary fallbacks. The certified exponential free propagator and RK4
stage equations are unchanged. The audible three-mode plugin does not use this
nine-coordinate research operator. Prepared storage and allocation behavior
are unchanged; no timing or realtime improvement is claimed.

## Regression tests

A new test advances an unforced dense positive-definite mass matrix for 10,000
steps at each of 1 ns, 100 ns and 100 microseconds. The old implementation fails
exact velocity preservation at 1 ns: one component changes from `0.094` to
`0.09400000000041633` on the local Windows GNU build. The increment form preserves
every velocity component exactly at all three steps. The position accumulation
is still floating point; no exact position claim is made.

A second test verifies the original forced midpoint momentum equation and
independently integrated work/heat over dense inertia, three damping strengths,
four step sizes from 1 ns to 1 ms, and negative/zero/positive forces. Residual
limits are 5e-14 times the corresponding momentum/energy scale. These checks
verify the equations directly rather than comparing with another spelling of
the increment update. Existing coupled contact, work, momentum, refinement and
atomic-rejection tests exercise the complete assembly.

## Coupled results

The retained [increment-form resolution report](../references/memory-modal-incremental-refinement.json)
reruns the same 16 cases, 96 takes and 112 comparisons as the
[pre-correction report](../references/memory-modal-rk4-refinement.json) from
`e450acb`. All existing gates pass without changes. Maximum relative structural
work residuals over the four 32 ms cases are:

| Uniform ticks per observation | Previous form | Increment form |
| --- | ---: | ---: |
| 8336 | 2.640e-9 | 1.600e-13 |
| 16672 | 4.125e-9 | 3.000e-13 |
| 20832 | 7.327e-9 | 2.654e-13 |

For the finest reference, the ratio of these maxima is about 27,600. Its maximum
combined energy residual decreases from 7.321e-9 to 7.135e-12. Across every path
and duration in the new study, maximum combined, structural and hammer residuals
are 8.867e-11, 1.945e-12 and 9.001e-11; maximum positive energy increment is
6.777e-16. These measured residuals do not establish a universal error bound.

The force-free reproducer and the coupled reduction together identify the
prepared near-identity propagation as a major source of the observed structural
drift. No change to cumulative heat/work summation was needed for this reduction.
This does not prove that every remaining ledger error has the same origin.

Trajectory differences remain distinct from energy closure. In the 8 ms matrix,
default RK4 versus the old-grid uniform reference changes from 0.2612% to 0.2595%
in kinetic velocity RMSE / launch speed. The capped RK4 versus finest reference
is now 0.1664%, and the two finer uniform references differ by 0.09304%.
At 32 ms these latter comparisons are 0.1034% and 0.05783%. The finer energy
balance does not eliminate timestep/phase error or establish acoustic fidelity.
The small changes in these comparisons also mean old timing/audit final-state
receipts are historical evidence, not byte-identical expectations for this build.

## Reproduction and wider regression

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-rk4-refinement --output renders/increment-refinement.json
cargo run --locked --release -p rf-73-lab -- modal-hammer-check --output renders/increment-hammer.json
cargo run --locked --release -p rf-73-lab -- memory-modal-adaptive-check --output renders/increment-adaptive.json
cargo run --locked --release -p rf-73-lab -- memory-modal-economical-check --output renders/increment-economical.json
cargo run --locked --release -p rf-73-lab -- memory-modal-free-check --output renders/increment-free.json
```

Output paths must be new. The existing commands and report schemas are unchanged;
the source revision and retained filenames distinguish operator versions.

The shared operator also passes the retained
[dissipative modal hammer audit](../references/modal-incremental-hammer-validation.json)
(36 cases/108 takes),
[adaptive contact audit](../references/modal-incremental-adaptive-validation.json),
[economical contact audit](../references/modal-incremental-economical-validation.json)
and [certified free audit](../references/modal-incremental-free-validation.json)
(12 cases/24 takes each). Together with the resolution study this is 276 takes.
The original uniform reference reports match between the three stateful audits
and the corresponding resolution-study paths.

All 184 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Existing CI already runs these audits and the new unit tests. Remote CI,
native timing and GUI/audio tests were not run for this correction. Longer/wider
trajectory qualification, performance reduction, physical calibration and host
integration remain open; the audible plugin engine remains unchanged.
