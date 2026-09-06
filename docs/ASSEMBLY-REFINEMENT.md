# Coupled assembly: free transition and contact refinement

The original assembly's 4x midpoint solver remained passive but accumulated
large treble phase errors. `AssemblyVoice::new_refined(rate, contact_substeps, p)`
now prepares an exponential transition for free motion, while retaining the
discrete-gradient hammer solver at a finer contact rate. The physical network
and all parameter values from [Coupled assembly](COUPLED-ASSEMBLY.md) are unchanged.
The original constructor and its midpoint reference remain available.

## Prepared free motion

The linear system for `z = [q, v]` has generator:

```text
G = [      0          I     ]
    [ -M^-1 K     -M^-1 C   ]
z(t+h) = exp(hG) z(t)
```

The implementation balances coordinates using square-root masses and a common
frequency scale derived from the mass-normalized stiffness row-sum bound.
The scale is at least 1/s, so zero stiffness/free translation is supported.
It prepares both binary damper states, including non-proportional damping and
overdamped systems; no assumption of independent damped normal modes is needed.

A power-of-two scaling brings the balanced generator's infinity norm below
`1/32` per small interval. A fixed degree-18 Taylor polynomial computes the
small exponential; repeated squaring recovers the full interval. The scalar
norm bound on the Taylor remainder is below double-precision roundoff.
Scaling has a checked limit of 32 squarings and rejects nonfinite results.
This is a small fixed-matrix implementation, not the adaptive Padé algorithm
published by Al-Mohy and Higham. Their work provides context on scaling,
squaring and floating-point accuracy.
[University of Manchester publication record](https://eprints.maths.manchester.ac.uk/1442/).

The computed transition approximates the continuous linear solution to floating-
point accuracy in the tested domain; it is not an exact-arithmetic guarantee.
All exponentials, matrix products and quadrature preparation occur in the
constructor. A free tick applies a fixed 6x6 matrix and a quadratic loss form.

## Independent dissipation accounting

Loss is not inferred by subtracting endpoint energies. The constructor integrates
the physical dashpot power along the free trajectory:

```text
Q(h) = integral_0^h exp(tG)^T W exp(tG) dt
loss = z0^T Q(h) z0
Q(2h) = Q(h) + E(h)^T Q(h) E(h)
```

Here `W` represents the three network dashpots and, when enabled, the tine's
grounded damper. Three-point Gauss-Legendre quadrature on the scaled interval
uses positive weights and sums of each dashpot observation's outer product.
It therefore constructs a nonnegative loss form in exact arithmetic.
The work-composition identity doubles its duration alongside each exponential
squaring. The measured balance residual exposes errors in either calculation;
there is no energy renormalization or negative-loss clipping.

## Contact and interval ownership

The refined voice advances four base ticks per output frame. Contact subdivisions
are a constructor argument, restricted to powers of two from 1 to 64. The validated
candidate uses 32: 128 physical contact steps per output frame while the hammer is
engaged, then four exponential free transitions per frame after separation.
Both midpoint response matrices and small free transitions are prepared for the
contact interval and both damper states.

If separation occurs during a tick, only its remaining micro-intervals advance
freely. Averaging contact force over the whole base tick preserves impulse.
The original strike/rejection/reset behavior and outgoing hammer energy ledger
remain intact. Rendering uses fixed-size storage and bounded work, with no heap
allocation, locking, matrix factorization or I/O in a tick.

## Reproducible audit

```text
cargo run --locked --release -p rf-73-lab -- assembly-refinement --output renders/assembly-refinement.json
```

The command refuses to overwrite existing reports and opens no audio device.
The [tracked report](../references/assembly-refinement-validation.json) covers
48 note/rate/velocity cases: MIDI 28/40/55/69/88/100, velocities 0.01/1, and
44.1/48/96/192 kHz. Every take lasts 50 ms with the binary damper applied halfway
through, rounded down to an output frame. Each case compares six integrations:
midpoint at 4/128/256 and exponential free motion at 4x with 1/32/64 contact
subdivisions. All 288 takes pass the documented gates.

The trajectory metric is the mass-weighted relative RMSE of all three absolute
coordinates at identical physical times, without alignment or fitted gain.
`contact_substeps` identifies refined rows; its absence identifies the original
midpoint solver. Refined energy and separation are observed at base ticks;
reported force is their microstep average. The original audit still observes
each of its uniform internal steps.

| Integration | Maximum displacement error vs midpoint 256 | Maximum velocity error vs midpoint 256 |
| --- | --- | --- |
| Original midpoint 4x | 15.5343% | 25.8869% |
| Exponential free, contact 1 | 0.48977% | 0.67342% |
| Exponential free, contact 32 | 0.003502% | 0.005829% |
| Exponential free, contact 64 | 0.003736% | 0.006229% |

The 256-step midpoint reference has its own free-motion phase error. Increasing
contact resolution can therefore slightly increase the difference from that
reference. A second comparison holds exponential free motion fixed and compares
32 against 64 contact subdivisions directly: maximum displacement difference
is `0.0003628%`, and maximum velocity difference is `0.0005014%`. Both are below
the separate 0.01% gate. Agreement between finite references is not an absolute
error estimate or a claim about audible differences.

For the 32-subdivision candidate, maximum energy-balance residual is `4.023e-12`
of injected energy, and maximum positive base-tick energy change is `3.157e-15`.
These pass the `1e-8` balance and `1e-10` positive-energy gates. The unchanged
midpoint rows shared with the earlier audit reproduce all scalar values exactly.

Four additional tests cover a one-second analytic treble oscillation at
44.1/192 kHz, analytic free translation and viscous exponential decay including
independent integrated loss, separation inside a tick versus explicit microsteps,
and energy balance through repeated damper changes/restrikes and extreme
mass/stiffness/damping combinations. The analytic treble test bounds normalized
position, velocity and energy errors by `1e-8`. All 121 workspace tests pass.

## Native timing and remaining gates

An isolated native release timing probe renders one MIDI-100 voice at 48 kHz
for 250 ms with five strong strikes and binary damping. Each integration runs
five times; preparation is excluded and the energy ledger is included.

| Integration | Median wall time |
| --- | --- |
| Original midpoint 4x | 0.4668 ms |
| Original midpoint 128x | 16.2842 ms |
| Exponential free, contact 32 | 2.1299 ms |

The refined candidate is about 7.6 times faster than continuous 128x in this
probe, and slower than the inaccurate 4x baseline. These are local observations,
not CPU deadlines: there is no full-keyboard workload, pickup/filter, host or
WASM timing qualification in this experiment. Results will vary by machine.

This resolves the measured integration obstacle for the provisional profiles.

The subsequent [tine structural preparation](TINE-MODES.md) now supplies six
geometry-derived fixed-root modes and reciprocal moving-root inertia. It has
not yet been connected to this three-coordinate time-domain solver.
It does not calibrate them, restore omitted higher bending modes, or establish
the correct support geometry. The next physical work is higher-mode and support
identification, including root rotation and polarization where evidence supports
them. Validate the pickup on those trajectories and the full polyphonic workload
before integrating the assembly into the instrument. Extreme-parameter passivity
tests do not guarantee physical or sampling accuracy over every accepted profile.

The existing 0.1.2 plugin, UI and saved-state schema are unchanged. This is an
offline numerical milestone; no new listening package or Desktop audition is
produced and no native UI control is used.
