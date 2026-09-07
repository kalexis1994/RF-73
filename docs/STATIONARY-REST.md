# Stationary initialization of the coupled model

The source baseline exposed output before the first key command: the cold
constructor placed the felt against the undeflected tine, then released that
unbalanced preload into the dynamic solver. `ElectromechanicalAssembly::new_at_rest`
now prepares the initial equilibrium of the same springs and unilateral contact
laws. The original `new` constructor remains available for historical controls.

## Static preparation

Let `q_free` be the unconstrained rest coordinates, `K` the block structural and
return stiffness, and `P` the four compression ports. The static solution obeys

```text
q = q_free - K^-1 P F
d = d_free - P^T K^-1 P F
F_i = k_i max(d_i, 0)^2
```

Preparation factors the positive anchored stiffness and constructs the four-port
compliance. Each Gauss-Seidel update solves the scalar quadratic contact law
with its rationalized positive root. The sweep budget is 128. Acceptance checks
the actual compressions again (relative constitutive defect <=1e-12) and each
generalized force/torque row independently (relative defect <=1e-10, normalized
with like units within that row). Nonfinite results and singular/free stiffness
are rejected. A failed preparation does not install the candidate state.

All velocities start at zero. The energy ledger starts with the actual elastic
and contact potential energy of this equilibrium, including felt preload.
No time is advanced, no work or heat is invented, and no output is muted or
thresholded. A contact already supporting the rest load is not counted as a new
impact. The constant-inductance circuit starts with zero current and voltage;
stationary flux generates no EMF. The public API only prepares a new model,
without exposing a reset that could discard a ringing instrument's history.

## Idle control experiment

```text
cargo run --locked --release -p rf-73-lab -- stationary-rest --output references/stationary-rest-validation.json
```

The retained receipt covers 70 mm at 48 kHz and 120 mm at 96 kHz, each with felt
angles 0 and 0.4 radians. Each cold/rest pair runs for 100 ms at 128 ticks/frame
with stationary actuators and the default 10k load. Raw voltage is observed as
well as midpoint averaging to 4x and the common 127-tap FIR. Repreparation at
half the time step must produce exactly the same initial probe and diagnostics.

All four pairs pass. Cold filtered peaks span 0.08537–0.09972 V. At rest, the
largest raw peak is 9.460e-14 V and filtered peak 9.451e-14 V, below the frozen
1e-9 V threshold. Maximum pickup drift is 3.331e-17 m and speed 1.284e-13 m/s.
The worst relative energy defect is 6.792e-16, with zero actuator work and no
new contact entries. Every prepared case retains positive stored energy and
felt force. All four solve in one sweep; force and constitutive defects remain
below 2.049e-16 and 7.180e-15 respectively. This is numerical silence, not a
claim of bitwise-zero voltage for every profile.

The receipt SHA-256 is
`e6e9ce002657457f0858858f6159dd82c44fbec7a8872492b8482043d9516f9c`.
Additional tests cover a disengaged felt with exactly zero energy, rejection of
free return/support profiles, time-step independence, and atomic failure.

## Source comparison from rest

```text
cargo run --locked --release -p rf-73-lab -- compare-loaded-bank references/g3-pitch-reference.manifest.json --output references/loaded-source-rest-validation.json --preview renders/loaded-bank-rest-soft.wav --striking --at-rest
```

The explicit flag changes only initialization. It retains the pinned sources,
training-only spring target, physical mode tracking, prescribed speeds and
spectral/level comparisons of [the cold baseline](LOADED-SOURCE-BASELINE.md).
Every long take additionally requires peak output below 1e-10 FS during the
first 30 ms, before key movement. This is distinct from the analysis pre-onset
peak, which also includes motion between key actuation and hammer contact.

All six 2.5-second takes and all three output comparisons pass. The worst
windowed 128/256-tick relative voltage RMSE is 3.395e-5 (0.003395%); relative
total energy and mechanical/electrical exchange defects are at most 1.567e-12
and 6.781e-19. Every take retains two hammer contacts and positive electrical
losses. Source measurements, training target and structural spring fit are
identical to the cold receipt. The spring remains at 55.740811 mm from the root.

| Pedestal speed (m/s) | Output pitch error (cents) | Late level relative to body (dB) |
| --- | --- | --- |
| 1.125 | -0.05424 | -14.57792 |
| 1.5 | -0.04573 | -13.94274 |
| 1.75 | -0.03774 | -13.63375 |

The 0.75 and 1.0 m/s preflight gestures still do not strike within 120 ms.
At 1.125 m/s, the pre-contact hammer velocity becomes 0.106230 m/s compared
with 0.095864 m/s in the cold control. Initial rest therefore matters to the
soft attack even though the late level changes little. No MIDI-velocity mapping
or material parameter is fitted here.

All 15 source/gesture pairs still fail descriptive agreement. The model's late
level is 8.057–12.148 dB below the reference-relative trajectory; maximum available
band discrepancies span 28.602–66.203 dB across pairs. Eliminating the idle
transient does not explain the fast sustain decay. Next isolate structural,
support, felt and electrical loss contributions with controlled changes and
energy ledgers before fitting losses to the processed bank. Source EQ and noise
reduction still prevent uniquely interpreting these differences as material
decay constants.

The retained soft preview is 120,000 finite samples at 48 kHz, peak 0.099134177
FS and RMS 0.004498494 FS on Rust readback. Its first-30-ms float-WAV peak is
4.135e-15 FS versus about 0.008654 FS in the cold preview. Analysis pre-onset
peaks remain about 0.0019 FS because felt/key motion begins before hammer contact;
that mechanically generated motion is retained.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| `references/loaded-source-rest-validation.json` | 267086 | `2af259730c424b6c6264ccd75c71bb758d8e2d864a1e3b3ac6cb271ee4cf3362` |
| `renders/loaded-bank-rest-soft.wav` | 480058 | `50a803c8ab26668244cd608c2771f6956a2a8367806668ade5475043ebe89757` |

## Scope

This establishes rest for the existing reduced physical model. Gravity, static
magnetic attraction, measured preload and material calibration remain outside
it. Preparation requires positive anchored stiffness, even though historical
dynamic constructors accept free mechanisms. Dynamic contact, circuit reaction,
energy accounting and output filtering are unchanged. The experiment remains
offline; it does not update the playable plugin or certify real-time cost.
