# Native tail cost and diagonal stiffness

The [128 ms qualification](MEMORY-MODAL-TAIL.md) establishes a longer numerical
comparison, but not its runtime cost. Two commands now measure the same four
strong-strike profiles in continuous 0–8, 8–32, 32–64 and 64–128 ms sections:

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-tail-timing --output renders/tail-timing.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-stiffness-timing --output renders/stiffness-timing.json
```

The first compares default and contact/free-capped RK4 controllers. The second
compares diagonal and forced-dense stiffness products with the default controller
in the same executable. Both use three repetitions per path with alternating
order. Preparation is timed separately. Execution includes controller decisions,
rejected trials and consumed probes; JSON generation and section-end energy/work
checks are outside each timer. Sections do not restart the physical trajectory.
Controller reports are cumulative, so section counts require subtraction.

All section-end combined, structural and hammer-work residuals must remain below
1e-8, and clocks must be finite and positive. There is no machine-dependent timing
gate. Stiffness timing additionally requires every section state and controller
report to match exactly across paths and repetitions. Per-step checks remain in
the independent numerical audits. The reported final states allow each timing
run to be matched against those audits.

## Exact structural sparsity

The current nine-coordinate assembly has diagonal stiffness, although its mass
matrix is reciprocal and dense and its engaged damper introduces off-diagonal
damping. Preparation now checks whether every off-diagonal stiffness entry is
exactly zero. For diagonal stiffness, `K*q` evaluates nine diagonal products
instead of 81 products. A nonzero off-diagonal entry of any magnitude retains
the original dense multiplication; no approximate sparsity threshold is used.

The optimization applies to stateful structural energy, coupled RK4 forces and
state-error metrics, and implicit contact's elastic error metric. Full mass and
damping products remain intact. The dense row sum's initial positive zero is
preserved explicitly, including signed-zero inputs, and the energy dot product
retains its original order. The finite physical trajectories therefore need
not change. Matrices are immutable after preparation.

`MemoryModalAssembly::use_dense_stiffness_reference()` selects the old arithmetic
for offline comparisons. It changes no physical state, coefficient, timestep,
prepared contact response or acceptance tolerance. The inline stateful voice
grows from 6032 to 6040 bytes for the classification flag and padding; prepared
free/contact heap payloads are unchanged. The audible three-mode plugin does
not use this research assembly.

A new test compares all result bits with the dense implementation across three
tine lengths, signed zero, positive/negative and very small/large finite motion.
It also verifies that off-diagonal entries as small as 1e-300 select the dense
path and contribute to the force. This preserves future coupled stiffness terms.

## Initial measurements and interpretation

The [pre-optimization timing](../references/memory-modal-tail-before-timing.json)
and [first optimized timing](../references/memory-modal-tail-diagonal-timing.json)
retain 24 runs each. All 48 final states/controllers match the earlier numerical
audit, and all 96 corresponding section snapshots match between binaries.
However, the separately executed batches give mixed results: default-path total
medians improve by 7.9–20.3% at 75 mm but worsen by 13.4–16.0% at 120 mm.
The reports are retained rather than presenting only the faster cases.

This inconsistency motivates the same-executable alternating comparison. Short
native batches remain subject to machine load, cache behavior and compiler
layout. Section diagnostics between timers can also affect subsequent timing;
neither three repetitions nor an arithmetic operation count establishes a
universal speedup or a host deadline guarantee.

## Paired results

The retained [same-executable comparison](../references/memory-modal-stiffness-paired-timing.json)
uses the default controller for both arithmetic paths in a Windows GNU release
build. No builds, tests or audits ran concurrently with any timing batch.
Median execution time for each 128 ms trajectory is:

| Tine / material relaxation | Dense | Diagonal | Time reduction |
| --- | ---: | ---: | ---: |
| 75 mm / 1 ms | 0.099934 s | 0.090120 s | 9.82% |
| 75 mm / 10 ms | 0.177809 s | 0.164085 s | 7.72% |
| 120 mm / 1 ms | 0.095282 s | 0.084557 s | 11.26% |
| 120 mm / 10 ms | 0.164832 s | 0.149495 s | 9.30% |

All three diagonal total-time observations are below all three dense observations
within each profile in this batch. The largest section reductions occur in
0–8 ms: dense medians of 62.16–65.51 ms become 51.68–55.76 ms, approximately
14.9–16.9% less time. Individual late sections are noisier; for example, the
120 mm / 1 ms 32–64 ms section is slightly slower in the diagonal batch.
The retained observations support a local improvement, not a universal speedup.

The diagonal total medians correspond to 0.661–1.282 seconds of computation per
simulated second. These averages must not be interpreted as realtime readiness:
the first 8 ms still takes 51.68–55.76 ms of computation for one assembly.
No pickup conversion, mixing, polyphony or host deadline work is included.
Material relaxation also matters: the 10 ms profiles take substantially more
time in free recovery than the 1 ms profiles. Future cost work should address
the coupled attack and material/free recovery without loosening existing gates.

## Validation and reproducibility

All 72 timing runs across the three reports match the original tail audit's
final physical states and controller counters. The paired command itself checks
that all section states and counters agree across the six runs per profile.
The repeated numerical audits are byte-identical to the corresponding reports:

| Repeated report | Original | SHA256 |
| --- | --- | --- |
| [Tail](../references/memory-modal-diagonal-tail-validation.json) | `memory-modal-tail-validation.json` | `37AF85AD5D58DBEFFA35710C75ADA9B28A7EBFFAF7F2FA6424304700639F05BA` |
| [RK4](../references/memory-modal-diagonal-rk4-validation.json) | `memory-modal-tail-control.json` | `B6CABCE3F2B5A9F12ACBE4ED1E4AF666F5E4E5728FEAB7D6A04593B6B0394E60` |
| [Implicit adaptive](../references/memory-modal-diagonal-adaptive-validation.json) | `modal-incremental-adaptive-validation.json` | `62458DD52C86488A9851B21DDA0E0EB223C176C315887A22C7D17E7C26A443DC` |

These cover twelve 128 ms takes and 48 short-protocol takes. No numerical gate,
controller decision, model coefficient or audible engine behavior changes in
the retained comparisons. All 186 workspace tests, strict Clippy, formatting
and release WASM compilation pass. CLI help, invalid arguments and overwrite
protection pass. CI runs the paired command to check arithmetic equivalence,
with no timing threshold; remote CI and GUI/audio testing were not performed.
