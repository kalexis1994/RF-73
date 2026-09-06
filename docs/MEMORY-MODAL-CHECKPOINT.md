# Shared physical checkpoints for late impacts

The [repeated-excitation study](MEMORY-MODAL-REIMPACT.md) failed trajectory
agreement despite closing energy and work. Its paths had evolved independently
from the initial strike, so each late collision inherited a slightly different
preimpact state. This study holds that state exactly equal before comparing
integrators around one late collision.

## Physical checkpoint contract

`MemoryModalAssembly::checkpoint()` captures an opaque in-memory
`MemoryModalCheckpoint`. It includes all nine positions/velocities, full
structural operators and diagnostic arithmetic flags, damper state, both hammer
masses, internal material history, last force/heat diagnostics, surface state,
structural energy cache and all accumulated work/heat/impulse ledgers. Capture
does not mutate the source. It is not a serialized user-editable state format.

`restart(h)` returns a fresh assembly with identical physical state. It validates
the existing 1 ns–1 ms step domain and rebuilds midpoint/material ramp
preparation for that step. It copies material history rather than resetting it.
Adaptive free/contact banks are deliberately absent until explicitly prepared;
caller controller history is not part of the checkpoint. Invalid restart steps
leave both source and checkpoint unchanged. This is an offline preparation API,
not a callback operation or plugin preset format.

## Experiment

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-checkpoint-check --output renders/modal-checkpoint.json
```

A default-RK4 donor follows the earlier repeated-excitation protocol for each
75/120 mm tine and 1/10 ms material-relaxation profile. Before each first late
positive-force interval (one in 32–80 ms, another in 80–128 ms), save the start
of its 48 kHz observation frame. The saved frame must have zero force and zero
surface energy. This selects eight separated preimpact checkpoints without
changing positions to manufacture a desired gap.

Each checkpoint starts six 8 ms continuations:

| Path | Contact / free maximum level | Base ticks per 48 kHz frame |
| --- | --- | ---: |
| Default coupled RK4 | 12 / 12 | 16672 |
| Contact cap | 2 / 12 | 16672 |
| Free cap | 12 / 8 | 16672 |
| Both caps | 2 / 8 | 16672 |
| Uniform midpoint | No adaptive steps | 16672 |
| Finer uniform midpoint | No adaptive steps | 20832 |

Every fresh path must have an exactly equal full probe before and after bank
preparation. Controllers start fresh in all adaptive paths. Original events
retain their absolute times and occur before their observation frames. The
donor is approximate, and choosing its checkpoint does not declare its prior
trajectory physically correct.

Each continuation must contact then separate, pass independent combined,
structural and hammer work residuals below 1e-8, and preserve nonnegative heat
and force with positive relative energy increments below 1e-10. Eight pairs
compare the default with each cap, all adaptive paths with the finer midpoint
reference, and the two midpoint grids with each other. Every pair must pass
whole-record and separate 2 ms gates: kinetic velocity RMSE / original launch
speed below 1%, relative pickup velocity RMSE below 1%, and relative mean-force
RMSE below 2%. No tolerance is relaxed.

Contact timestamps are interval-end observations, not interpolated roots.
`absolute_two_ms_sections` uses donor time; `whole_record.two_ms_windows` uses
time relative to the checkpoint. Zero reference power is null and passes only
with zero difference. Failed qualifications retain the report and exit nonzero;
existing reports are never overwritten. No audio or realtime timing is measured.

## Results and interpretation

The [retained report](../references/memory-modal-checkpoint-validation.json)
passes all eight checkpoints, 48 continuations, 64 whole-record comparisons and
256 separate 2 ms comparisons. All donor contact timestamps exactly match the
earlier repeated-excitation report. Every continuation's initial full probe
matches its checkpoint before and after preparation; serialized initial states
also agree across all six paths.

Maximum section errors across the eight checkpoints are:

| Pair | Kinetic velocity / launch | Relative pickup velocity | Relative mean force |
| --- | ---: | ---: | ---: |
| Default / contact cap | 0.00000341% | 0.000000285% | 0.00000199% |
| Default / free cap | 0.00001450% | 0.000001138% | 0.00000188% |
| Default / both caps | 0.00001759% | 0.000001245% | 0.00000202% |
| Default / finer midpoint | 0.019647% | 0.00030443% | 0.00040188% |
| Two midpoint grids | 0.011135% | 0.00017251% | 0.00022781% |

Maximum relative combined, structural and hammer residuals across all
continuations are 7.105e-11, 2.660e-12 and 6.864e-11. These absolute ledger
residuals include the donor's inherited balance, rather than clearing it at
restart.

The large late-trajectory divergence does not reproduce in these 8 ms local
continuations when the full preimpact state is equal. This supports inherited
state differences as an important part of the earlier failure; it does not
identify where they originate or exclude later amplification. The midpoint
paths remain approximate, and the common donor is not ground truth. Controller
history is intentionally reset in all branches, another limit on comparisons
with uninterrupted runs. The complete repeated-excitation qualification still
fails and has not been repaired by this experiment.

The next isolation should move the common checkpoint back to the 32/80 ms
impulse, compare the intervening free/material recovery and approach to contact,
and track state differences before the first collision. Continue the same
contact/free resolution separation without modifying physical coefficients or
loosening tolerances.

## Validation receipt

Date: 2026-09-06. All 193 workspace tests, strict Clippy, formatting, native lab
build and release WASM compilation pass. A new DSP regression covers complete
history preservation, damper state, signed impulses, invalid-step atomicity,
independent repeated restarts, absent adaptive banks and exact same-step fixed
continuation. A new comparison regression catches a failing last 2 ms section
hidden by the whole-record average and rejects incomplete trajectories.

The repeated [original short audit](../references/memory-modal-checkpoint-control.json)
contains 24 takes and is byte-identical to its earlier report (SHA-256
`b6cabce3f2b5a9f12acbe4ed1e4af666f5e4e5728feab7d6a04593b6b0394e60`).
CLI help, invalid-option rejection and overwrite preservation pass. CI includes
the new passing checkpoint qualification; the known failing full-repetition
experiment remains separate. No physical equation or acceptance tolerance
changes; no remote CI, GUI/audio or realtime timing qualification was performed.

Follow-up: the [shared-impulse approach study](MEMORY-MODAL-APPROACH.md) passes
all pre-contact prefixes but retains one failure during post-impact recovery,
including disagreement between the two uniform midpoint grids. Its next
checkpoint should be after separation.
