# Loaded two-strike repetition

The [return study](LOADED-HAMMER-RETURN.md) found that constant return damping
can improve settling while substantially changing the first attack. This
experiment tests a second key gesture from the actual recovered state and
keeps that first-attack tradeoff explicit.

```text
cargo run --locked --release -p rf-73-lab -- loaded-repetition --output REPORT.json
```

A new JSON output path is required. Numerical failure retains a report and
returns an error. Functional failures are retained separately and do not
turn a converged measurement into a numerical failure.

## Frozen matrix and state continuity

Twenty-four takes cross three configurations, two speeds and two waiting
intervals at 128/256 ticks per 48 kHz observation frame. Configurations are
the same conditional 70 mm G3 baseline, return damping 0.1 Ns/m instead of
0.025, and pedestal rate loss 10 s/m instead of 2. The fitted tuning-spring
position, first-basis tine decay, pickup, circuit and all other coefficients
remain unchanged. Speeds are 1.125 and 1.5 m/s for both key directions and
both strikes. The pedal stays closed.

The first key-down begins at 30 ms and key-up at 150 ms. The second key-down
begins either 60 or 300 ms after key-up, at 210 or 450 ms. It lasts 120 ms,
followed by 70 ms of release observation, for total durations of 400 or
640 ms. Waiting time is measured from the key-up command, including physical
pedestal travel; it is not all stationary time at the bottom of the key.

Each take prepares one stationary electromechanical assembly. The same
assembly advances through both gestures with bounded pedestal slew. There
is no reinitialization of tine modes, hammer, damper arm, contact storage,
pickup or circuit between notes. The receipt retains all 20 positions and
velocities, contact forces/compressions/counts, pedal/pedestal positions,
mechanical/electrical energy, current and output voltage immediately before
the second drive step. Historical snapshots before that step and the first
impact can be compared directly with the preceding one-strike experiment.

## Separate contact and functional evidence

Four phases divide the run: first strike/hold, recovery, second strike/hold
and second release. Each retains hammer/tine contact entries and pre-entry
states, impulse, peak force, active contact duration, boundary carry-in/out
and raw voltage RMS. Contact-entry lists are capped at 16 per phase and
overflow fails qualification. The RMS includes residual vibration and circuit
memory; it is not an isolated second-note loudness or timbre measurement.

Numerical qualification retains independent hammer, pedestal, bridle, arm
and felt energy ledgers, monotone heat, reciprocal pickup exchange and quiet
initial rest. Refinement additionally requires matching phase entry counts
and contact carry, entry-time differences below 0.1 ms, and impact refinement
below 1%, with exact paired zeros for phases without impulse. Hammer, arm and
both pickup velocity channels must separately refine within 1% in each phase
(first drive begins at 30 ms), using a 1e-8 m/s RMS floor.

Action readiness is a separate diagnostic over the 20 ms before repetition:
both hammer and arm positions within 0.1 mm of their prepared rest, speeds
below 0.01 m/s, and felt contact for at least 90% of the window. Residual tine
and circuit state is retained regardless of this decision.

Repeatability compares the second strike with the first **within the same
configuration**, requiring one contact entry each, less than 5% change in
impulse, peak force, duration and pre-impact hammer speed, and less than 1 ms
change in latency from the respective key-down. A clean repeatable pair also
requires no contact during either recovery/release phase, no boundary carry,
and at least 0.1 mm felt clearance with no contact during 50–110 ms after
each key-down. Action readiness is reported independently: an action need
not be stationary to produce a repeatable second strike. Terminal return
uses the last 50 ms after the second release and is also separate.

These fixed limits are engineering diagnostics, not measured regulation
tolerances, a perceptual test or a prescribed maximum repetition rate.

## Retained results

All 24 takes and all twelve refinement pairs qualify numerically. Every
take produces exactly two hammer/tine contacts, one during each held gesture,
with none during recovery or release and no phase-boundary contact carry.
Both held felt-lift windows pass in every take. Sixteen takes (eight paired
rows) meet the clean repeatable-strike criteria; all baseline rows fail it.

The table reports second/first impulse ratios at 256 ticks. A repeatability
pass also requires the other impact quantities, latency, felt lift and
contact separation; the impulse ratio alone is not the decision.

| Configuration | Drive (m/s) | Wait from key-up (ms) | Second/first impulse | Repeatability | Action ready |
| --- | ---: | ---: | ---: | --- | --- |
| Baseline | 1.125 | 60 | 13.509446 | Fail | No |
| Baseline | 1.125 | 300 | 6.706007 | Fail | No |
| Baseline | 1.5 | 60 | 0.885518 | Fail | No |
| Baseline | 1.5 | 300 | 0.910346 | Fail | No |
| Return damping 0.1 | 1.125 | 60 | 0.973388 | Pass | No |
| Return damping 0.1 | 1.125 | 300 | 1.013230 | Pass | Yes |
| Return damping 0.1 | 1.5 | 60 | 1.007518 | Pass | No |
| Return damping 0.1 | 1.5 | 300 | 0.999980 | Pass | Yes |
| Pedestal rate loss 10 | 1.125 | 60 | 1.007982 | Pass | No |
| Pedestal rate loss 10 | 1.125 | 300 | 1.000188 | Pass | No |
| Pedestal rate loss 10 | 1.5 | 60 | 1.001341 | Pass | No |
| Pedestal rate loss 10 | 1.5 | 300 | 0.999695 | Pass | No |

The baseline soft-drive first pre-impact speed is 0.106164 m/s; its second
speed is 0.563726 m/s after the shorter wait and 0.331532 m/s after the longer
wait. These discrepancies converge and occur without extra strikes or failed
felt lift. In the strong-drive rows, second impulse instead falls by 11.45%
and 8.97%. The state-dependent attack sensitivity remains unresolved.

Higher return damping limits the maximum second/first impulse difference
to 2.662%; higher pedestal loss limits it to 0.799%. Only the two long-wait
return-damping rows pass action readiness. In the short-wait return-damping
soft row, hammer speed just before the second command is -0.107624 m/s,
yet impact repeatability passes. The high-pedestal-loss rows also repeat
while failing readiness. Stationary return within the declared limits is
therefore not a necessary condition for these particular repeatable gestures.
It must not be used as a stand-in for testing repetition itself.

All terminal-return diagnostics fail during the last 50 ms of this short
post-second-release observation. This does not contradict the previous
800 ms return result: the current run observes only 70 ms after releasing
the second key gesture.

The maximum relative total energy defect is `1.832e-12`, independent hammer
work defect `1.428e-12`, coupling defect `1.391e-12` and pickup exchange defect
`4.056e-19`. Maximum windowed velocity refinement RMSE is 0.009586%, impact
refinement error 0.042590% and contact-entry time difference 0.0814
microseconds. Repeatability decisions agree at both resolutions.

All first-impact receipts and historical snapshots strictly before the
second drive exactly replay the preceding return study for each configuration
and speed. Tests additionally reconstruct phase impulse totals, entry counts,
readiness, lift, separation and repeatability decisions from retained values.
Verification passes 126 lab unit tests, 21 loaded CLI/receipt tests, strict
Clippy and formatting.

The [receipt](../references/loaded-repetition-validation.json) is 963517 bytes,
SHA-256 `59c569cd6829e9a64c0c7d652be1a13078989deb128829710cdaf0151402813e`.
The release cache remains approximately 195 MiB, with no new WAVs.

## Scope and next step

Passing second/first consistency does not remove a configuration's changed
first attack. No candidate is automatically adopted, and no source fit,
audio render or production default is added. Two intervals and two speeds
cannot establish arbitrary-gesture behavior, long repeated-note sequences,
half-pedal operation, full-keyboard realism or realtime performance.

Neither intervention is adopted. Both inherit roughly twelvefold soft
first-impulse changes from the preceding return experiment, despite their
improved within-profile repetition. The [launch study](LOADED-LAUNCH.md) now
records pedestal/bridle work and force impulses from the incoming state.
Next address launch regulation while retaining the successful repeated-strike
controls. Any proposed physical change must preserve both first-strike
dynamics and repetition; slowing the return until it meets a position limit
alone is not sufficient evidence.
