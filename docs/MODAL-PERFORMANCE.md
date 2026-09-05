# Modal free-motion cost and block timing

The nine-coordinate research assembly now evaluates its prepared free transition
directly in physical coordinates. The equations, contact resolution and independent
energy ledger are retained. This is an offline optimization; the audible 0.1.2
plugin still uses the original engine.

## Prepared operators

With `M = L L^T`, the exponential is still prepared in balanced mass coordinates:

```text
y = [q, v]
D = block_diag(scale L^T, L^T)
z = D y
z_next = E z
work = z^T Q z

P = D^-1 E D
R = D^T Q D
y_next = P y
work = y^T R y
```

`Q` continues to come from independently integrated viscous power and squaring,
not subtraction of endpoint energies. All basis changes occur during preparation.
For work evaluation, each diagonal coefficient is stored once and each pair of
off-diagonal coefficients is summed. The 18-state quadratic form therefore stores
171 coefficients rather than 324. Neither negative-work clipping nor energy
renormalization is used. Matrix-vector evaluation borrows rows without copying
the entire matrix. Contact still uses the same midpoint equations and 48 bisections.

One new test compares 48 damped trajectories against the previous normalized
evaluation, including light support inertia, a negative tonebar arm, both damper
states, two rates and base/contact-remainder intervals. It checks state error
in the mechanical energy norm and accumulated independent work. A second, lossless
20,000-step test requires exactly zero reported dissipation and energy conservation
within tolerance. Existing rigid-motion, reciprocity, composition, contact and
retrigger tests continue to apply. These are finite-domain checks, not proof of
accuracy over every accepted parameter combination.

## Reproducible native experiment

```text
cargo run --locked --release -p rf-rhodes-lab -- modal-timing --output renders/modal-timing.json
```

The command refuses existing output paths. It prepares 1/8/32/73 independent
voices at 48/192 kHz, using the default profile and cycling 50/75/120 mm tine
lengths. These lengths are illustrative, not a calibrated keyboard. Each case
runs one warmup and five measured 250 ms gestures. All voices strike together
at velocity 1 every 50 ms and apply the damper 25 ms after each strike. There is
no voice sleeping or stealing. Processing is serial in 128-frame blocks, with
four base ticks and 32 contact subdivisions. The final block may be shorter;
its deadline uses its actual frame count.

The timer includes mechanical stepping, events and dissipated-work accounting.
Preparation, resets and block-boundary state/energy probes are excluded. Every
probe must remain finite and close its energy ledger within `1e-8` of cumulative
injected energy. Wall-clock block times include timer overhead and scheduling
interference. The p99 is the nearest-rank percentile over all five measured runs;
render totals sum the timed blocks. Block medians use the upper middle order
statistic. Probes between blocks also affect cache state.
No thread priority is changed. Pickup, filtering, mixing, host and driver costs
are absent, so these results cannot qualify realtime performance.

The tracked [before](../references/modal-timing-before-folding.json) and
[after](../references/modal-timing-after-folding.json) reports use the same Rust
benchmark, toolchain and computer. The before run uses DSP commit `fe06f40`;
the after run includes physical-coordinate folding, packed work and borrowed
matrix rows. They are sequential local observations, not an interleaved controlled
benchmark. Absolute timings and speed ratios vary with machine load.

## Local observations

Measured on Windows x86-64 with the pinned Rust 1.98.0 release profile:

| Rate | Voices | Before median / 250 ms | After median / 250 ms | Speed ratio | After p99 block / deadline | After late blocks |
| --- | --- | --- | --- | --- | --- | --- |
| 48 kHz | 1 | 21.246 ms | 16.519 ms | 1.29x | 0.733 | 0 / 470 |
| 48 kHz | 8 | 169.224 ms | 139.616 ms | 1.21x | 6.345 | 26 / 470 |
| 48 kHz | 32 | 699.434 ms | 554.029 ms | 1.26x | 24.049 | 221 / 470 |
| 48 kHz | 73 | 1604.304 ms | 1401.186 ms | 1.14x | 53.820 | 470 / 470 |
| 192 kHz | 1 | 84.602 ms | 75.426 ms | 1.12x | 10.865 | 25 / 1875 |
| 192 kHz | 8 | 712.348 ms | 593.165 ms | 1.20x | 86.664 | 870 / 1875 |
| 192 kHz | 32 | 2749.146 ms | 2147.214 ms | 1.28x | 305.839 | 1875 / 1875 |
| 192 kHz | 73 | 6005.679 ms | 5027.452 ms | 1.19x | 683.482 | 1875 / 1875 |

The native voice object shrank from 31,824 to 21,712 bytes (31.8%). This excludes
temporary preparation storage, allocator overhead and test-only reference
operators. No new dependencies were added.

The worst after-run block-boundary energy residual was `1.467e-11` of injected
energy. Lower average cost does not solve contact bursts: eight voices at 48 kHz
render faster than real time overall but miss 26 individual deadlines. Even the
single voice at 192 kHz misses blocks during this gesture. The 73-voice workload
is far beyond its available time at both rates. The new
[84-take audit](../references/modal-assembly-folded-validation.json) separately
checks contact and energy accuracy against refined and uniform-midpoint references.

## Next gate

Contact bursts, sustained polyphony and WASM/host execution need separate
optimization and qualification. Any contact optimization must retain the
independent midpoint reference, energy checks and spatial force coupling before
the six-mode assembly can become a listening candidate.
