# Exact diagonal damping in coupled contact

The nine-coordinate assembly builds separate damping matrices for raised and
engaged dampers. The raised-damper matrix contains only diagonal losses for
the root, tine modes and tonebar. Engaging the damper adds its reciprocal outer
product, generally producing off-diagonal coupling. Coupled RK4 previously
used a full matrix-vector product for both states at every RHS evaluation.

Preparation now classifies each matrix independently by testing whether every
off-diagonal entry is exactly zero. A diagonal product uses nine products in
place of 81, retaining the dense sum's zero accumulator. Any nonzero coupling,
however small, selects the original full product. A zero-strength damper can
therefore use the diagonal path in either state; an engaged coupled damper
keeps the dense path. No loss coefficient or matrix entry is changed.

Only the coupled RK4 RHS uses the new operator. The same damping vector supplies
both structural acceleration and the independent heat quadrature `v^T C v`.
Full mass inversion, stiffness products, trial reuse, all stage validity and
energy/work checks, controller levels and acceptance tolerances remain intact.
Implicit midpoint and certified free motion retain their existing arithmetic.
The two classification flags fit existing inline padding; no per-step allocation
or additional prepared matrix is introduced.

`use_dense_contact_damping_reference()` selects the old dense multiplication in
both damper states for offline comparisons. It changes no physical state or
prepared step. Damper events select the corresponding classification immediately;
there is no state-dependent cache to invalidate.

## Qualification protocol

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-damping-timing --output renders/contact-damping-timing.json
```

The existing timing protocol compares four strong-strike profiles for 128 ms,
with an impulse at 2 ms and damper engagement/release at 4/6 ms. Both paths use
the default RK4 controller, diagonal stiffness and shared contact trials. Three
repetitions alternate the two damping paths in one executable. Every section
state and controller report must match exactly across paths and repetitions.
CI enforces this identity without a timing threshold. The numerical tail and
short-contact audits independently check trajectory and energy/work accuracy.

Tests compare products bitwise at three tine lengths, two damper strengths and
both damper states, including signed zero and small/large finite inputs. Tiny
off-diagonal entries must still affect the result. A separate trial regression
checks accepted/rejected levels, impulses, damper events, atomic rejection and
subsequent fixed ticks against the forced dense path.

## Paired native observations

The retained [Windows GNU release report](../references/memory-modal-contact-damping-timing.json)
contains 24 runs. No builds, tests or audits ran concurrently with this batch.
Median total execution time for each 128 ms trajectory is:

| Tine / relaxation | Dense contact damping | Diagonal when exact | Reduction |
| --- | ---: | ---: | ---: |
| 75 mm / 1 ms | 0.087273 s | 0.080640 s | 7.60% |
| 75 mm / 10 ms | 0.162464 s | 0.158626 s | 2.36% |
| 120 mm / 1 ms | 0.082144 s | 0.076858 s | 6.44% |
| 120 mm / 10 ms | 0.160639 s | 0.156295 s | 2.70% |

The first 8 ms costs 48.05, 48.03, 45.09 and 49.52 ms respectively, compared
with dense medians of 53.24, 52.59, 49.70 and 53.12 ms. This supports a local
contact-cost improvement in this batch. Free-motion arithmetic is unchanged;
varying later-section timings are not evidence of a free-path improvement.
Three short repetitions do not establish confidence intervals or universal gains.

Every section state and controller counter in all 24 runs also matches the
preceding trial-reuse report. The inline stateful voice remains 6040 bytes;
prepared free and RK4 payloads remain 102960 and 664 bytes respectively.

## Validation receipt

Date: 2026-09-06. All 189 workspace tests, strict Clippy, formatting and release
WASM compilation pass. CLI help, invalid-option rejection and overwrite
preservation pass. The repeated audits cover 36 takes and match the earlier
reports byte for byte:

| Audit | Retained report | SHA-256 |
| --- | --- | --- |
| 128 ms tail, 12 takes | [Tail validation](../references/memory-modal-contact-damping-tail-validation.json) | `37af85ad5d58dbeffa35710c75ada9b28a7ebffaf7f2fa6424304700639f05ba` |
| Coupled RK4, 24 takes | [RK4 validation](../references/memory-modal-contact-damping-rk4-validation.json) | `b6cabce3f2b5a9f12acbe4ed1e4af666f5e4e5728feab7d6a04593b6b0394e60` |

The timing dispatcher now uses one explicit comparison enum instead of combining
independent mode flags. Existing command names, path labels and protocols remain
available. All three existing commands were rerun after audits completed:
72 additional runs preserve path/repetition labels, section states and counters
against their retained earlier reports. These compatibility outputs are local
under `renders/contact-damping-compat-*.json`; their timings are not used for
the performance comparison above. The earliest tail timing report predates the
descriptive `comparison` field; its experiment and numerical records still match.
Remote CI and GUI/audio testing were not run.

Timing remains a native single-assembly observation with diagnostics. It excludes
pickup voltage, mixing, polyphony and host deadlines. It does not establish
realtime readiness, physical calibration or perceptual realism. The audible
plugin continues to use its existing engine.
