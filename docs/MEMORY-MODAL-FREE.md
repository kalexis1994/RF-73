# Certified free recovery with a moving tine

`MemoryModalAssembly` now combines the [stateful hammer](MEMORY-FREE-MOTION.md)
with prepared structural free propagation. It retains nine structural coordinates,
two hammer masses, material memory, independent heat and reciprocal contact work.
The existing implicit contact tick is unchanged. This experiment remains offline;
the audible 0.1.2 plugin does not use it.

## Separation certificate

Let the hammer port be `s = b^T q`, structural mass matrix M, tip mass mt,
current hammer energy Eh and structural energy Es. Until first contact, both
disconnected continuous subsystems are passive, so:

```text
|tip_velocity| <= sqrt(2 Eh/mt)
|surface_velocity| <= sqrt(2 Es (b^T M^-1 b))
gap = tip_position - b^T q
gap + h (sqrt(2 Eh/mt) + sqrt(2 Es (b^T M^-1 b))) + margin < 0
```

Cauchy-Schwarz in the full mass metric gives the structural port bound. The
sum bounds relative travel throughout the interval, including an approach that
would separate again before its endpoint. The roundoff margin is 64 machine
epsilons times `max(|tip| + |surface| + h*speed_bound, 1e-12 m)`.
Failure means separation is uncertified, not necessarily that contact occurs.
The certificate assumes no external impulse or control event inside the interval.

## Prepared propagation and transactional acceptance

`prepare_free_steps(max_level)` allocates dyadic structural propagators for
levels 0..max_level and both damper states. Level L advances `2^L` fixed ticks.
Levels are limited to 0..12 and intervals to 1 ms. Preparation computes the full
inverse port mass once and changes no physical state; failed preparation keeps
the previous bank. Existing voices need not prepare a bank.

`try_free_step(level)` performs no allocation, factorization, I/O or unbounded
iteration. It uses the existing matrix-exponential structural transition and
independently prepared damping-work quadrature. It checks finite state,
nonnegative heat, a structural energy defect below `1e-12 S`, and positive
structural energy change below `1e-14 S`. Here S is initial hammer energy plus
accumulated absolute external impulse work, floored at `1e-30 J`.

The hammer then attempts its existing RK4 step-doubling primitive on a copy,
preserving all material checks and its tighter `1e-13 S` local work/energy limit.
Its surface coordinate follows the structural endpoint; zero contact force
means no new surface impulse or work. A combined energy defect below `2e-12 S`
is required before committing both subsystems and their ledgers together.
Neither heat nor work is inferred by subtracting an energy residual.

Unprepared/invalid levels return errors. Clearance and accuracy rejections leave
the complete voice unchanged. Free acceptance also preserves the fixed contact
timestep and its precomputed material coefficients. This is checked numerical
integration, not an unconditional passivity or global-error proof.

## Coupled audit

```text
cargo run --locked --release -p rf-73-lab -- memory-modal-free-check --output renders/memory-modal-free.json
cargo run --locked --release -p rf-73-lab -- memory-modal-free-timing --output renders/memory-modal-free-timing.json
```

Both commands refuse existing output files. The audit shares the original
coupled protocol and its uniform-reference implementation: lengths 50/75/120 mm,
launch speeds 0.2/0.8 m/s and relaxation times 1/10 ms, 384 observations at 48 kHz.
An external core impulse at 2 ms drives reimpact; dampers switch on at 4 ms and
off at 6 ms. The candidate and reference share 16672 fixed ticks per observation
(about 1.25 ns); candidate free intervals can replace up to 4096 ticks.

The laboratory controller halves rejected intervals and uses one original fixed
tick if level zero fails. A search has at most thirteen attempts. Contact uses
the original fixed tick; a growing free interval must fit the remaining output
frame, so impulses, damper changes and observations stay aligned. Growth requires
state error below `1e-10/64` and reported combined defect below `1e-13/64`.
Heat, mechanical energy and both port-work balances are checked after every
accepted interval. Comparison uses the full structural mass matrix plus the two
hammer masses, pickup-port velocity, and duration-weighted mean contact force.

The [retained report](../references/memory-modal-free-validation.json) passes
all 12 cases/24 takes with the unchanged 1% kinetic-velocity, 1% pickup-velocity,
2% mean-force and `1e-8` energy/port-work gates. All 12 uniform reference rows
match the original coupled report exactly.

| Maximum candidate error | Result |
| --- | ---: |
| Relative global energy residual | 1.166e-10 |
| Relative structural work residual | 1.178e-10 |
| Relative hammer work residual | 8.609e-11 |
| Positive relative mechanical energy step | 5.083e-16 |
| Kinetic-metric velocity RMSE / launch speed | 0.006220% |
| Pickup velocity relative RMSE | 0.000772% |
| Output mean-force relative RMSE | 0.001313% |

Accepted interval counts fall by 4.16x–12.58x, including remaining contact ticks.
Rejected attempts are counted separately. The maximum accepted free interval
is 1.280 microseconds. The moving-port speed bound is conservative, especially
when structural energy remains high after the hammer separates. An interval
count reduction is not itself a runtime speedup.

Four added tests cover unchanged state on preparation/clearance rejection,
an approaching structural surface with a stationary hammer tip, transactional
accuracy rejection after structural propagation, and mixed free/contact motion
against uniform ticks with an external impulse and damper transitions.

## Performance and remaining gates

The timing command uses the same fine timestep and event sequence for both
paths, with three paired repetitions for four profiles and alternating order.
Preparation is measured separately. Execution includes adaptive control and
rejections and consumes each accepted interval's returned probe. Final energy
is checked outside timing; per-interval audit overhead is excluded. Native
timing does not include pickup voltage, mixing, a host or WASM.

The [native report](../references/memory-modal-free-timing.json) records these
medians for 8 ms of simulated motion on the local Windows GNU release build:

| Length / speed | Uniform fine path | Adaptive path | Speedup |
| --- | ---: | ---: | ---: |
| 50 mm / 0.2 m/s | 2.110 s | 0.387 s | 5.45x |
| 50 mm / 0.8 m/s | 1.907 s | 0.250 s | 7.63x |
| 120 mm / 0.2 m/s | 2.047 s | 0.560 s | 3.66x |
| 120 mm / 0.8 m/s | 1.989 s | 0.361 s | 5.50x |

All repetitions are retained, including the slower first uniform run. These
short measurements are observations, not confidence intervals. Adaptive cost
still amounts to 31–70 seconds per simulated second. The comparison uses the
same 1.25 ns base resolution in both paths; the earlier cache benchmark used
2.5 ns and is not the denominator for these speedups.

The prepared bank uses heap storage in addition to the inline voice. The report
records reserved operator payload separately, excluding allocator overhead.
This build reports 5,992 inline bytes and 102,960 reserved free-operator bytes
per prepared voice (13 levels, two damper states).
The current per-voice bank is a prototype; sharing immutable prepared operators
and reducing contact cost remain possible future improvements.

Material coefficients, geometry and loss profiles remain provisional. Numerical
agreement does not establish Rhodes realism. Longer trajectories, wider profiles,
native/WASM deadlines and measured physical calibration remain required before
integrating these mechanics into an audible release.
