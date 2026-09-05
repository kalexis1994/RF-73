# Scheduling contact-error estimates

The [strict contact experiment](MEMORY-ADAPTIVE-CONTACT.md) retains its full
step-doubling estimator and `1e-11` state-error limit. A separate laboratory
controller now avoids attempting short contact intervals that cannot amortize
three implicit solves, and defers retries after a rejected minimum interval.
No contact law, material memory, energy gate or original fine tick is relaxed.

## Why scheduling matters

In the retained strict audit, the 50 mm / 0.8 m/s / 1 ms profile accepts 186,268
contact macrointervals but rejects another 100,935 for accuracy. Its accepted
intervals replace only 380,484 base ticks: almost all replace two ticks at the
cost of three solves plus an error estimate. More aggressive subdivision is
therefore often cheaper than repeatedly attempting adaptive contact.

The new controller uses these fixed, reported scheduling constants:

- Minimum contact trial: level 3, or eight base ticks.
- Rejected minimum trial: execute 64 subsequent original fine ticks before retrying.
- Every accepted interval: unchanged compression certificate, three implicit
  solves, coarse/fine state check, independent energy/work checks, and two-half-step commit.
- Growth: unchanged threshold of one eighth of the `1e-11` local state limit.

The failed attempt itself also falls back to a fine tick. Short observation-frame
remainders use fine ticks. Deferred retries still advance one physical base tick
per call; this is not an audio/control-event delay or skipped simulation time.
Impulses and damper changes continue to apply at their original boundaries.
On separation, the retry count clears and the previous certified free controller
continues. Contact/free intervals never cross an observation or external event.

The new policy is explicit through `with_economical_contact` in the laboratory;
the previous strict and free-only controllers remain available as controls.
It allocates no memory while stepping. Counters report accepted contact intervals,
accuracy/boundary rejections, delayed fine ticks, minimum trial length and delay.

## Reusing the estimator's existing probes

The DSP contact primitive already computes full hammer probes for its coarse
and fine trial endpoints. Its phase-error metric now receives those probes
instead of evaluating them again. Metric arithmetic and acceptance decisions
are unchanged. This removes two redundant probe evaluations per completed
contact estimate without replacing the estimator or omitting a physical check.

## Reproducibility and accuracy

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-economical-check --output renders/contact-scheduling.json
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-economical-timing --output renders/contact-scheduling-timing.json
```

The existing `memory-modal-adaptive-check`/`memory-modal-adaptive-timing` retain
the previous strict controller; `memory-modal-free-timing` retains fixed contact
plus certified free motion. Output files must be new.

The [new audit](../references/memory-modal-economical-validation.json) passes
all 12 cases/24 takes with the same 8 ms impulse/damper protocol, geometry,
material profiles, 16672 base ticks per 48 kHz observation, and global gates.
The contact limit remains `1e-11`; accepted contact intervals reach 19.994 ns.
Candidate maxima against the uniform fine reference are:

| Error | Maximum |
| --- | ---: |
| Relative combined energy residual | 1.093e-10 |
| Relative structural work residual | 1.096e-10 |
| Relative hammer work residual | 8.079e-11 |
| Kinetic velocity RMSE / launch speed | 0.004014% |
| Pickup velocity relative RMSE | 0.000328% |
| Output mean-force relative RMSE | 0.001120% |

Using more original fine ticks changes the adaptive trajectory, so this policy
does not promise identity with the previous adaptive candidate. Lower error on
these profiles is an observation, not a global accuracy proof. The
[strict-controller regression audit](../references/memory-modal-estimator-reuse-control.json)
checks that probe reuse preserves the previous complete report byte for byte.

Two new tests compare every deferred tick with an independent original-tick
voice through an impulse and damper change, and verify that short frame
remainders never attempt contact estimation or skip a tick.

Across the 12 cases, total contact attempts (accepted plus both rejection
categories) fall from 3,292,012 to 315,855, a 90.4% reduction. This counts attempts,
not arithmetic operations or runtime. Most physical contact motion now uses
the original fine ticks, particularly in the stiff/strong-strike profiles.

## Recorded runtime and limitations

The same release build produced three retained native batches:
[new scheduling](../references/memory-modal-economical-timing.json),
[strict scheduling](../references/memory-modal-scheduling-strict-timing.json),
and [fixed-contact control](../references/memory-modal-scheduling-fixed-timing.json).
Each uses three repetitions per path/profile, the same event sequence and base
resolution, and excludes preparation. All observations are retained. The two
adaptive policies share the reused-probe estimator; this is a scheduling
comparison, not an isolated timing claim for probe reuse.
All 72 timing runs match their respective audited final mechanical states.

| Length / speed | Fixed contact | Strict adaptive | New scheduling | Strict/new ratio |
| --- | ---: | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 0.514 s | 0.330 s | 0.305 s | 1.083x |
| 50 mm / 0.8 m/s | 0.259 s | 0.440 s | 0.241 s | 1.829x |
| 120 mm / 0.2 m/s | 0.510 s | 0.383 s | 0.437 s | 0.878x |
| 120 mm / 0.8 m/s | 0.315 s | 0.416 s | 0.316 s | 1.318x |

Values are median seconds for 8 ms of simulated motion on the local Windows GNU
build. New scheduling improves three profiles versus strict scheduling but
regresses by about 14% in the soft 120 mm case. Relative to fixed contact, the
strong 120 mm case is effectively tied in this batch. Sequential short batches
are sensitive to machine load and are not confidence intervals or a universal
performance ordering; in particular, the fixed-contact 50 mm soft timing differs
substantially from earlier recorded runs.

New scheduling still costs 30–55 seconds per simulated second. It is a separate
experimental option, not the new default or a realtime qualification. A cheaper
contact integrator/estimator remains a more substantial gate than retry policy.
All 172 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Existing reports are protected against overwrite. CI includes the new
audit; remote CI and GUI/audio testing were not performed.

Physical calibration, longer trajectories, additional profiles and realtime
qualification remain open. This work changes when the laboratory attempts its
existing estimator; it does not replace the audible 0.1.2 plugin mechanics.
