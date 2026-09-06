# Shared calculations in coupled contact trials

The coupled RK4 controller evaluates one full step and two half steps before
acceptance. The full step and first half step start from exactly the same state,
material profile, structural operators and damper state. Several independent
energy checks also refer to the same unchanged trial endpoints. This change
reuses those identical calculations without altering the integration formulas
or dropping checks.

## What is reused

The first RK4 derivative is evaluated once for the full step and reused as the
initial derivative of the first half step. This derivative does not depend on
step duration. All other stages are evaluated separately, including the initial
derivative of the second half step. Classical RK4's last stage is not the
derivative of its accepted endpoint, so it is not reused for the following step.
A complete three-step trial therefore evaluates eleven rather than twelve RHS
stages. Stage validity checks remain in the same sequence.

Energy vectors are computed once for the initial, coarse, half and fine states.
The original three interval checks still verify combined energy, material work,
surface-potential work, structural port work and hammer port work in their
original order. Only repeated evaluation of the same endpoint energy is removed.
This reduces six energy-vector evaluations to four. The fine state's structural
energy also supplies the committed structural cache, removing one additional
structural-energy evaluation. The committed hammer/material work checks still
run independently after construction of the proposed state.

Caches are local to one attempt and never persist across steps, events, rejection
or parameter changes. No approximation, extrapolation, acceptance tolerance,
physical coefficient, contact boundary rule or controller level changes. The
new cached values use the same arithmetic as the recomputed values.

`use_recomputed_contact_trial_reference()` selects the original independent
calculations after RK4 preparation for offline comparisons. It changes only a
prepared-bank flag and returns an error if preparation is missing. The stateful
voice stays at 6040 inline bytes; the RK4 prepared payload grows from 656 to
664 bytes, including padding. Temporary derivative/energy values use fixed-size
stack storage, with no allocation in a trial. This does not quantify total stack
usage or polyphonic memory.

## Paired timing

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-trial-reuse-timing --output renders/trial-reuse-timing.json
```

The command uses the same four 128 ms strong-strike profiles, default controller
and diagonal stiffness in both paths. Three repetitions alternate reused and
recomputed trials in one executable. Preparation and section-end serialization
are outside execution timers; every accepted interval's probe is consumed.
All section states and controller reports must agree exactly across both paths
and every repetition. The existing independent trajectory audit remains required.

The retained [Windows GNU release timing](../references/memory-modal-trial-reuse-timing.json)
contains 24 runs, all matching the prior tail audit's final states and counters.
No builds, tests or audits ran concurrently with the timing batch. Median total
execution times for each 128 ms trajectory are:

| Tine / relaxation | Recomputed | Reused | Total-time reduction |
| --- | ---: | ---: | ---: |
| 75 mm / 1 ms | 0.089620 s | 0.084075 s | 6.19% |
| 75 mm / 10 ms | 0.165759 s | 0.161401 s | 2.63% |
| 120 mm / 1 ms | 0.084733 s | 0.079923 s | 5.68% |
| 120 mm / 10 ms | 0.194314 s | 0.194678 s | -0.19% |

The first 8 ms improves from 57.20 to 51.91 ms, 56.86 to 51.50 ms, 53.42 to
48.32 ms, and 73.86 to 58.08 ms respectively. The first three reductions are
approximately 9.2–9.6%; the fourth is larger and accompanies more variable later
sections. In that fourth profile the total median is essentially unchanged and
slightly slower. Free recovery is unmodified, so its varying timings should not
be attributed to contact reuse. The data do not establish an improvement in
every total trajectory or a universal speedup.

The attack still costs 48.32–58.08 ms for 8 ms of one assembly's motion. This
remains an offline research path without pickup conversion, mixing, polyphonic
or host deadline qualification. Three short sequential repetitions are not
confidence intervals, and section timers/diagnostics can affect cache behavior.

## Regression coverage

A new test compares reused and recomputed trials through accepted/rejected
levels 0–12, impulses, both damper states and subsequent original fixed ticks.
It checks state, force, error estimates, energy defects, rejection atomicity,
and reference-mode preparation requirements. Existing smooth-order, reciprocal
work and ungrounded momentum tests remain applicable. CI runs the paired command
to enforce equivalence without any machine-dependent speed threshold.

All 187 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CLI help, invalid-option rejection and existing-output preservation pass.
The repeated audits cover 36 takes and produce byte-identical reports:

| Audit | Retained report | SHA-256, matching the preceding audit |
| --- | --- | --- |
| 128 ms tail, 12 takes | [Tail validation](../references/memory-modal-trial-reuse-tail-validation.json) | `37af85ad5d58dbeffa35710c75ada9b28a7ebffaf7f2fa6424304700639f05ba` |
| Coupled RK4, 24 takes | [RK4 validation](../references/memory-modal-trial-reuse-rk4-validation.json) | `b6cabce3f2b5a9f12acbe4ed1e4af666f5e4e5728feab7d6a04593b6b0394e60` |

The implicit adaptive path is unchanged and was not re-audited for this change.
Remote CI and GUI/audio tests were not run. The audible plugin still uses its
existing engine; numerical equivalence does not establish physical calibration.
