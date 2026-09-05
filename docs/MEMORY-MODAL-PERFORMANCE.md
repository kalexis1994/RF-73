# Stateful modal diagnostics and native kernel cost

The stateful modal solver now evaluates structural mechanical energy once per
successful integration step and reuses it for the returned diagnostics and
subsequent observations. The physical equations, contact iterations, timestep,
heat integration and material state are unchanged.

## Reuse of an already validated result

Previously, a tick calculated the structural quadratic energy to validate the
candidate state and then requested a fresh probe, which calculated it again.
External observations repeated that calculation. The structural energy uses
the full M and K matrices; it is expensive compared with assembling a report.

The new implementation stores one f64 containing the last committed structural
energy. A tick uses its computed hammer probe and structural energy directly
when producing the returned probe. A later observation assembles a current
probe using that stored structural energy and the current hammer state. This
retains current signed impulse work and material diagnostics without storing
a second copy of the complete mechanical state.

The value starts at zero for the stationary structure. It changes only after a
successful coupled step, together with the new coordinates and heat. Rejected
ticks leave it untouched. A core impulse changes only hammer state; changing the
damper selects a damping matrix and changes no stored energy. Neither operation
invalidates the structural value. No coordinate or material update uses the
cached value: it serves diagnostics and energy accounting only.

An added test reconstructs diagnostics from the live coordinates and matrices
through 10,000 steps, positive/energy-removing core impulses and damper changes.
Every reconstructed probe must equal the public probe exactly. The returned
tick probe must also equal an immediate observation. Existing tests cover
atomic rejection of invalid impulses and excessive material travel.

## Reproducible timing workload

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-timing --output renders/memory-modal-timing.json
```

The native single-voice workload uses four combinations of tine length
50/120 mm and launch speed 0.2/0.8 m/s, with default material and structural
profiles. Each of three repetitions simulates 8 ms using 8336 steps per 48 kHz
observation frame: 3,201,024 ticks. An impulse at 2 ms, damper engagement at 4 ms
and release at 6 ms exercise the same events as the coupled accuracy audit.

Constructor/matrix preparation and final energy validation are outside the
timed region. Every returned tick probe is consumed by `black_box` inside the
timed region, so its construction is part of the measured kernel. There are no
extra per-tick probe calls. Each report records elapsed times, final states,
energy residuals, voice storage and medians. Output paths must be new.

Timing passes require finite positive clock measurements and a final relative
energy residual below 1e-8, not a machine-dependent speed threshold. Three
sequential observations do not establish statistical confidence. Background
load, CPU frequency and compiler choices can change the results. This benchmark
does not measure audio block deadlines, multiple voices, pickup voltage,
filtering, mixing, WASM or a host.

## Observed comparison

The [pooled comparison](../references/memory-modal-cache-timing-comparison.json)
retains two batches of three repetitions per version/profile, without dropping
observations. The first batch ran after/before and the repeat ran before/after.
The baseline executable contains the `84ded26` mechanics and the same timing
command. No assistant-launched builds, tests or audits overlapped these batches;
external background load and CPU clocks were not controlled.

| Tine length | Launch speed | Before median | After median | Observed speedup |
| --- | --- | --- | --- | --- |
| 50 mm | 0.2 m/s | 1.207 s | 1.003 s | 1.202x |
| 50 mm | 0.8 m/s | 1.281 s | 1.076 s | 1.191x |
| 120 mm | 0.2 m/s | 1.268 s | 1.060 s | 1.196x |
| 120 mm | 0.8 m/s | 1.224 s | 1.196 s | 1.023x |

These are medians of six observations, with the two middle sorted values
averaged. The last profile was slower in the first batch and faster in the
repeat. Its small pooled improvement should not be interpreted as a reliable
performance margin. All final positions, velocities and energy residuals in
the timing runs agree exactly across both versions and repetitions.

Raw reports retain [before](../references/memory-modal-timing-before-cache.json),
[after](../references/memory-modal-timing-after-cache.json),
[before repeat](../references/memory-modal-timing-before-cache-repeat.json) and
[after repeat](../references/memory-modal-timing-after-cache-repeat.json).
Voice storage grows by eight bytes, from 5944 to 5952. Even the pooled optimized
medians require 125–150 seconds per simulated second at this fine resolution.
This remains far from realtime.

## Accuracy and remaining work

The existing coupled temporal audit still compares 8336/16672 subdivisions over
twelve cases. Its physics and gates are retained. The cached structural value
is not a replacement for independent structural damping, material heat or port
work; each ledger continues to be computed from its own physical expression.

All 157 workspace tests, strict Clippy, formatting and release WASM plugin
compilation pass. The [new accuracy report](../references/memory-modal-cache-validation.json)
is byte-identical to the retained pre-optimization audit, including all three
energy residuals, state snapshots and temporal comparisons.

This change reduces diagnostic overhead. It does not reduce the extremely large
number of integration steps required by the current reference. Efficient free
motion and event-aware contact refinement remain the next numerical gates.
Material identification remains a separate realism requirement. The audible
plugin and version 0.1.2 have not changed, and no listening package is produced.
