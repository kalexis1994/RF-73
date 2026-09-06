# Direct material roots inside hammer contact

The stateful hammer's outer contact solve repeatedly evaluates an inner material
root. On a ramp whose endpoint retains the previous deformation's sign, that
inner equation is quadratic. The solver now attempts a stable direct solution
on that branch, checks it against the original equation, and retains safeguarded
Newton/bisection for crossing ramps or failed checks.

This changes numerical evaluation cost, not the material law, mass split,
surface stiffness, time resolution or free-motion controller. The audible plugin
remains version 0.1.2 with its previous mechanics.

## Derivation and acceptance

Let a be the committed material deformation, x its trial endpoint, F(x) the
unchanged mean ramp reaction, C=ac+at the positive inertial compliance, and
d=c_free-t_free+at*N. The inner equation is:

```text
R(x) = x + C F(x) - d = 0
R0 = C F(0) - d
```

Since R'(x)>=1, the unique root lies between 0 and -R0. Its sign is opposite
to R0. If that sign matches a (or a is zero), the signed cubic-potential
discrete gradient is a quadratic polynomial. With y=|x|:

```text
A y^2 + B y = |R0|
A = C kc/3
B = 1 + C (kc |a|/3 + k0/2 + k1 mean_delta)
y = 2 |R0| / (B + sqrt(B^2 + 4 A |R0|))
x = -sign(R0) y
```

`mean_delta` is the already prepared Maxwell ramp coefficient. B>=1, and the
conjugate formula avoids subtraction of nearly equal quantities. It also covers
kc=0 and C=0 without dividing by A. The force at zero is computed once per hammer
tick because material state remains fixed throughout the nested contact solve.

The direct endpoint must be finite and inside the original monotone bracket.
Its original residual must satisfy:

```text
|x + C F(x) - d| <= 8 epsilon (|x| + |C F(x)| + |d|)
```

Nonfinite inputs, endpoints, residuals or residual scales request fallback. Opposite-sign
ramps are not approximated by this polynomial. The fallback still uses at most
12 safeguarded Newton iterations and 64 bisections. The outer nonadhesive contact
root and its bracket are unchanged; the bisection-only reference path bypasses
the direct formula entirely.

No state is mutated by a trial. Actual material displacement limits, independent
heat integration, momentum exchange and transactional commit remain in the
original update. A small local residual alone does not establish global accuracy,
so complete trajectories and balances are audited separately.

## Verification protocol

Two new tests exercise signed loading/unloading from retained material history,
linear limits, tiny and large compliance, timestep/relaxation extremes, crossing
ramps and nonfinite trials. Existing tests independently compare the complete
hammer and moving assembly with pure bisection, check passivity at mass/material
corners, conserve ungrounded momentum and reject excessive travel atomically.

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-free-check --output renders/material-solve-modal.json
cargo run --locked --release -p rf-73-lab -- memory-free-check --output renders/material-solve-wall.json
cargo run --locked --release -p rf-73-lab -- memory-modal-free-timing --output renders/material-solve-timing.json
```

The new modal and fixed-wall reports retain the previous physical cases,
observation/event times, fine base steps and error gates. Both the adaptive and
uniform paths now use the direct root where accepted. The retained pre-change
reports therefore remain separate evidence; floating-point trajectory identity
is not assumed.

Before/after native timing uses the existing command, the same 1.25 ns base
resolution and three paired uniform/adaptive runs per profile. The pre-change
executable preserves the DSP implementation from commit `3d5e82e`. Construction
is outside execution timing, and all observations are retained. This experiment
does not measure audio, mixing, host overhead or WASM deadlines.

## Recorded results

All 167 workspace tests, strict Clippy, formatting and release WASM plugin
compilation pass. The direct-root test contains 3,240 history/profile/target
trials and requires more than 90% to accept the formula; rejected trials retain
the iterative path. The existing physical audits remain in CI; no remote CI
execution is claimed.

The [new coupled audit](../references/memory-modal-material-solve-validation.json)
passes 12 cases/24 takes. Candidate maximum global energy, structural work and
hammer work residuals are `1.166e-10`, `1.178e-10` and `8.243e-11`.
Kinetic-velocity, pickup-velocity and output mean-force RMSE remain at most
0.006220%, 0.000772% and 0.001312%, against the same fine resolution.

The [fixed-wall audit](../references/memory-material-solve-wall-validation.json)
passes 24 cases/48 takes. Maximum candidate global energy, material work and
momentum residuals are `1.658e-10`, `1.007e-10` and `7.210e-14`.

Comparing the retained old/new modal reports at final states and both sides of
each of three events, in both candidate/reference paths, gives a maximum
kinetic-metric velocity difference of `3.854e-8` times launch speed. Maximum
mechanical-energy difference is `9.887e-10` times initial energy. These are
checkpoint comparisons, not a bound on every intervening microstep. The finer
trajectory audit and independent bisection tests provide additional evidence.
No exact byte identity is claimed after changing root arithmetic.

The [before](../references/memory-modal-material-solve-before-timing.json) and
[after](../references/memory-modal-material-solve-after-timing.json) timing reports
retain all 24 runs per version. Median adaptive times for 8 ms of motion are:
All 48 timing runs match their respective audited final mechanical states.

| Length / speed | Before | After | Before/after ratio |
| --- | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 0.336 s | 0.275 s | 1.219x |
| 50 mm / 0.8 m/s | 0.265 s | 0.209 s | 1.269x |
| 120 mm / 0.2 m/s | 0.572 s | 0.473 s | 1.208x |
| 120 mm / 0.8 m/s | 0.364 s | 0.298 s | 1.221x |

Uniform-path median speedups are 1.149x–1.194x. Adaptive execution time falls
17–21% in this local measurement. Timings include all direct-root checks and
fallback work. They are observations from short sequential batches on Windows
GNU, not confidence intervals or a realtime guarantee. Adaptive cost remains
26–59 seconds per simulated second. Reducing required contact resolution and
qualifying longer trajectories and wider physical profiles remain open.
