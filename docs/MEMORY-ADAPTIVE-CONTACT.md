# Error-controlled compressed contact

`MemoryModalAssembly` can now attempt longer intervals while the hammer remains
compressed against the moving tine. It compares one implicit midpoint step with
two half steps and commits the two-half-step trajectory only after contact,
state-error and independent energy/work checks. No extrapolation or state reset
is used. This is an offline numerical experiment, not a new audible release.

## Continuous contact certificate

Let g=t-b^T q be penetration, E the current combined mechanical energy, mt the
tip mass, M the complete structural mass matrix, and b the hammer port. With
no external event inside the interval, the complete coupled model is passive.
Cauchy-Schwarz in its block mass metric bounds relative velocity:

```text
|g_dot| <= sqrt(2 E (1/mt + b^T M^-1 b))
g - h sqrt(2 E (1/mt + b^T M^-1 b)) - margin > 0
```

Unlike the free-motion certificate, individual subsystem energies cannot be
treated as constant bounds during contact: energy crosses the reciprocal port.
The combined bound excludes separation anywhere inside the proposed interval.
The margin is 64 machine epsilons times the maximum of 1e-12 m and absolute tip
position plus absolute surface position plus bounded travel. Uncertified
intervals are shortened; inability to certify the smallest attempt returns to
one original fine tick. Contact transitions are therefore still resolved at
the base resolution, not claimed as exact continuous event times.

## Preparation and transactional stepping

`prepare_contact_steps(max_level)` prepares midpoint matrices for both damper
states and analytic material-ramp coefficients for dyadic levels 0..max_level.
Maximum level is 1..12, and the largest interval must not exceed 1 ms. All
allocation, matrix factorization and exponential material preparation happen
here. Invalid or failed preparation leaves the physical state and previous bank
unchanged. `contact_operator_bytes` reports reserved heap payload separately.

`try_contact_step(level)` accepts levels 1..max_level: half steps are never
shorter than the original base tick. An attempt uses at most three bounded
implicit solves, with a fixed-size physical-state copy. Prepared material
coefficients change between trials, while deformation, Maxwell extension,
heat and work remain persistent. The original base preparation is restored
after acceptance without changing the accepted physical state.

Each coarse/half-step result must retain positive penetration and reaction,
nondecreasing structural/material heat, mechanical energy growth below `1e-14 S`,
and local combined energy and each port-work defect below `1e-12 S`. Here S is
initial energy plus accumulated absolute external impulse work, floored at
`1e-30 J`. The work and heat remain independently integrated quantities.

The coarse/fine state difference uses the full M-weighted structural velocity,
K-weighted structural displacement, both hammer kinetic terms, absolute hammer
position differences, material deformation and Maxwell extension differences,
and contact penetration difference. Material/contact displacement weights use
their stiffnesses and the larger endpoint deformation. The norm is the square
root of that quadratic sum divided by `2 S`. The accepted limit is `1e-11`,
exposed as `MemoryContactStep::STATE_ERROR_LIMIT`.

Rejected trials preserve all coordinates and ledgers. Invalid API requests
return an error. Boundary and accuracy rejections have separate statuses. A
successful result supplies the average of both half-step contact reactions;
the ordinary endpoint probe still contains the last half-step reaction and
last half-step material heat. Whole-interval heat is available from differences
of cumulative ledgers. The laboratory weights force by the full accepted duration.

The shared fixed-tick path uses the same arithmetic as before. Its uniform
reference diagnostics are checked against the retained pre-change report.

## Controller and validation

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-adaptive-check --output renders/modal-adaptive.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-adaptive-timing --output renders/modal-adaptive-timing.json
```

The laboratory combines the existing certified free-motion controller with
compressed-contact attempts. It halves rejected intervals and grows them when
the contact state error is below one eighth of the limit. Levels must fit the
remaining observation frame, so no interval crosses a core impulse, damper
change or observation. Each contact search has at most twelve attempts. The
report separates fixed ticks, accepted free/contact intervals, implicit half
steps, accuracy rejections and boundary rejections. These counts are not a
runtime speedup: every accepted contact interval also needs a coarse trial.

The retained 12-case protocol spans tine lengths 50/75/120 mm, speeds 0.2/0.8 m/s
and relaxation times 1/10 ms. Initial contact is followed by a core impulse at
2 ms, damper on at 4 ms and off at 6 ms; each take ends at 8 ms. The candidate
and uniform reference share 16672 base ticks per 48 kHz observation (about
1.25 ns), and all error gates remain unchanged.

The [preliminary run](../references/memory-modal-adaptive-preliminary-1e-8.json)
used a local state limit of `1e-8` and failed 4 of 12 trajectory comparisons,
despite passing energy/work checks. Maximum velocity error was 6.567%, pickup
error 4.529% and mean-force error 4.049%. That raw report is retained; its generic
protocol text predates the specific contact description. This failed experiment
is evidence that passivity and a small local defect do not suffice for long-term
phase/contact accuracy. The global gates were not weakened.

An [intermediate run](../references/memory-modal-adaptive-intermediate-1e-10.json)
at `1e-10` also passed, but maximum kinetic velocity error reached 0.9854%,
close to the unchanged 1% gate. That setting was not selected: the final
experimental API retains `1e-11` for its larger measured accuracy margin.
The reports preserve all three tolerance experiments; no performance result
is used to excuse a trajectory error.

The [stricter audit](../references/memory-modal-adaptive-validation.json) uses
`1e-11` and passes all 12 cases/24 takes. All 12 uniform reference rows match
the previous material-solver audit exactly. Candidate maxima are:

| Error | Maximum |
| --- | ---: |
| Relative combined energy residual | 9.118e-11 |
| Relative structural work residual | 9.420e-11 |
| Relative hammer work residual | 7.939e-11 |
| Kinetic velocity RMSE / launch speed | 0.03909% |
| Pickup velocity relative RMSE | 0.004859% |
| Output mean-force relative RMSE | 0.007111% |

The maximum accepted contact interval is 19.994 ns. Total accepted macrointerval
counts fall by 18.81x–23.30x relative to uniform ticks; each accepted contact
macrointerval contains two actual half steps. These results qualify the tested
profiles and duration only. Longer trajectories, additional mass/stiffness and
damper settings, measured physical calibration and realtime deadlines remain open.

Three new tests verify history-preserving material preparation against analytic
Maxwell extension, preparation/boundary rejection without changing fixed ticks,
and atomic accuracy rejection followed by exact agreement with two independently
prepared half ticks and a subsequent original base tick.

## Runtime result and current use

The [control timing](../references/memory-modal-contact-control-timing.json) uses
the previous certified-free/fixed-contact controller in the same build as the
[strict contact timing](../references/memory-modal-adaptive-timing.json).
Each records three repetitions for each path/profile, excluding preparation.
Both paths share the 1.25 ns base resolution and the same event sequence.
All 48 timing runs match their respective audited final mechanical states.
Both new commands refuse existing outputs with their SHA256 unchanged.

| Length / speed | Fixed-contact control | Adaptive contact | Control/adaptive ratio |
| --- | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 0.303 s | 0.327 s | 0.927x |
| 50 mm / 0.8 m/s | 0.216 s | 0.405 s | 0.534x |
| 120 mm / 0.2 m/s | 0.491 s | 0.363 s | 1.352x |
| 120 mm / 0.8 m/s | 0.298 s | 0.362 s | 0.824x |

These are medians for 8 ms of simulated motion. Three of four profiles regress:
the extra coarse solve, two half solves and state/energy checks outweigh the
step reduction. Adaptive cost is 41–51 seconds per simulated second. No
consistent performance improvement or realtime readiness is claimed. The
ordinary tick and previous free-only controller remain available and unchanged;
adaptive contact is explicitly prepared and selected by its separate commands.

The build reports 6,024 inline voice bytes, 37,440 reserved contact-operator
bytes and 102,960 reserved free-operator bytes. Payload excludes allocator
overhead. Preparation is separate from execution timing; operators are still
stored per voice. No new dependency, audio-device access or Desktop interaction
is introduced. All 170 workspace tests, strict Clippy, formatting and release
WASM compilation pass; CI includes the new audit, but remote CI was not run.

The next numerical gate is a cheaper error estimator or a more accurate contact
method per solve, followed by longer and wider-profile validation. This milestone
establishes a checked experimental path and exposes its cost/accuracy tradeoff;
it does not select new Rhodes material parameters or replace the audible engine.
