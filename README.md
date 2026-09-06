# RF-Rhodes

A Rust physical-model electric piano research project for RackForge.

The first working prototype includes a nonlinear hammer-contact solver, a three-mode resonator per key, a geometry-dependent magnetic pickup, sustain and sample-accurate MIDI. It renders audio offline and compiles to a portable RackForge WASM plugin.

Version 0.1.2 adds a Rust pickup laboratory editor in RackForge: three continuously filtered variants, fixed level matching, smooth A/B selection and complete saved settings. The physical profile remains provisional.

The offline [coupled assembly experiment](docs/COUPLED-ASSEMBLY.md) now models a tine, tonebar and compliant common support with reciprocal forces, nonlinear hammer contact and a complete energy ledger. Its parameters remain provisional; high-resolution validation precedes plugin integration.

[Prepared free motion and contact refinement](docs/ASSEMBLY-REFINEMENT.md) now reduce its measured treble integration error while retaining the independent energy ledger. This remains an offline candidate awaiting higher-mode identification and calibration.

[Geometry-derived tine modes](docs/TINE-MODES.md) now prepare six bending modes,
a movable tuning mass, spatial hammer/pickup weights and reciprocal inertia for
root translation and rotation. This structural basis is validated independently
and is used by the coupled solver described below.

The [nine-coordinate assembly](docs/MODAL-ASSEMBLY.md) now connects that structural
basis to nonlinear hammer contact and a spatial damper. Its time-domain energy
and convergence audits pass; it remains an offline experiment pending calibration
and polyphonic performance work.

**This is an uncalibrated research instrument, not yet a high-fidelity Rhodes recreation.** The provisional target is a Mark I Stage 73 with direct output. A measured reference instrument has not been selected.

## Quick start

To build, validate, install and open the current instrument in RackForge Desktop on Windows:

```text
cargo run --locked --release -p rf-rhodes-lab -- audition
```

The [audition workflow](docs/AUDITION.md) keeps a dedicated test library, retains audio/MIDI preferences and supports repeated builds of the same version.

```text
cargo test --locked --workspace
cargo run --release -p rf-rhodes-lab -- demo --output renders/demo.wav
cargo run --release -p rf-rhodes-lab -- render --output renders/a3.wav --trace
cargo run --release -p rf-rhodes-lab -- inspect renders/demo.wav
cargo run --release -p rf-rhodes-lab -- analyze renders/a3.wav --output renders/a3-analysis.json --note 57 --sustain-end 1.8
cargo run --release -p rf-rhodes-lab -- compare renders/a3.wav renders/a3.wav --output renders/self-comparison.json
cargo run --release -p rf-rhodes-lab -- stress
cargo run --release -p rf-rhodes-lab -- converge --output renders/convergence.json --note 100 --velocity 0.2
```

Requires Rust 1.98 and a sibling RackForge checkout for its public SDK. See [Development](docs/DEVELOPMENT.md) for Windows linker setup, WASM builds and packaging. Existing audio and report files are never overwritten.

## What is here

- `rf-rhodes-dsp`: safe Rust DSP with bounded contact integration, 73 fixed key states, per-key pickups and 4x antialias filtering.
- `rf-rhodes-plugin`: RackForge adapter with MIDI 1.0/2.0, matched A/B, declarative program editing and versioned state.
- `rf-rhodes-ui`: Rust WebAssembly PLAY panel with A/B controls, host synchronization and day/stage styling.
- `rf-rhodes-analysis`: offline WAV input, FFT spectra, harmonic and independent partial tracks, qualified decay estimates and aligned comparisons.
- `rf-rhodes-lab`: Rust WAV renderer, physical CSV traces, measurement commands, JSON reports and timing diagnostics.
- Tests for mechanical passivity, repeated strikes, dampers, MIDI ownership, block invariance, malformed input and file integrity.

The rendering and parameter-automation paths use no allocation, locks or I/O. The offline laboratory does not open an audio device. No samples, reverb, amplifier, compressor or limiter hide the direct model output. The plugin applies documented, fixed pickup level compensation.

## Read next

- [Pickup Lab UI](docs/PICKUP-LAB-UI.md): controls, A/B, saving, transition and gain policy.
- [Physical model ledger](docs/MODEL.md): equations, constants and known approximations.
- [Coupled assembly](docs/COUPLED-ASSEMBLY.md): mechanical reduction, energy balance, analytic tests and convergence limits.
- [Modal performance](docs/MODAL-PERFORMANCE.md): prepared nine-coordinate mechanics, independent work and native block timing.
- [Modal contact solver](docs/MODAL-CONTACT-SOLVER.md): bounded root-search acceleration with bisection and energy references.
- [Dissipative hammer](docs/DISSIPATIVE-HAMMER.md): rate-dependent contact loss, nonadhesive unloading and independent material heat.
- [Hammer material memory](docs/HAMMER-MEMORY.md): internal deformation, relaxation and repeated loading in an isolated material coupon.
- [Memory hammer impacts](docs/MEMORY-HAMMER.md): two inertial masses, nonadhesive surface contact, free recovery and impulse-driven reimpact.
- [Stateful modal hammer](docs/MEMORY-MODAL-COUPLING.md): reciprocal memory-hammer/tine coupling, persistent recovery and independent port work.
- [Stateful modal performance](docs/MEMORY-MODAL-PERFORMANCE.md): reused structural energy, exact diagnostic checks and native kernel timing.
- [Hammer free recovery](docs/MEMORY-FREE-MOTION.md): conservative clearance bounds and error-controlled longer free intervals.
- [Moving modal free recovery](docs/MEMORY-MODAL-FREE.md): moving-port contact exclusion, transactional propagation and coupled validation.
- [Direct material roots](docs/MEMORY-MATERIAL-SOLVE.md): checked quadratic branches inside the stateful contact solver.
- [Adaptive contact](docs/MEMORY-ADAPTIVE-CONTACT.md): continuous compression bounds, step doubling and retained fine references.
- [Contact scheduling](docs/MEMORY-CONTACT-SCHEDULING.md): minimum useful trial lengths and deferred retries with unchanged physical checks.
- [Contact reaction reuse](docs/MEMORY-CONTACT-FORCE-REUSE.md): avoid repeating the material solve at an already evaluated normal force.
- [Contact resolution](docs/MEMORY-CONTACT-RESOLUTION.md): read-only trial sweeps identify which coupled state errors limit the timestep.
- [Fourth-order hammer contact](docs/MEMORY-CONTACT-RK4.md): certified fixed-wall RK4 contact with independent heat, work and impulse integration.
- [Coupled fourth-order contact](docs/MEMORY-MODAL-RK4.md): moving tine/tonebar integration with reciprocal work checks and native timing.
- [Roadmap](docs/ROADMAP.md): implemented work and next milestones.
- [Measurement protocol](docs/MEASUREMENTS.md): reference recordings and evaluation.
- [Analysis laboratory](docs/ANALYSIS.md): commands, metric definitions and interpretation limits.
- [Partial comparison](docs/PARTIAL-COMPARISON.md): compare corresponding spectral components and qualified decays across recordings.
- [Pickup sweep](docs/PICKUP-SWEEP.md): rank a bounded geometry grid against a reference with one global level correction.
- [Pickup reference set](docs/PICKUP-SET.md): fit several takes with shared gain and evaluate reserved notes or intensities.
- [Reference banks](docs/REFERENCE-BANKS.md): commercial candidates and the acquired five-layer real-recording pilot.
- [Tone comparison](docs/TONE-COMPARISON.md): explicit attack/body windows and harmonic balance without inferred velocity or decay.
- [G3 residual pilot](docs/G3-RESIDUAL-PILOT.md): measured baseline and exploratory pickup differences against the acquired recordings.
- [Isolated pickup transfer](docs/PICKUP-TRANSFER.md): compare two magnetic laws under identical motion and measure internal sampling error.
- [Mechanical pickup pairs](docs/PICKUP-PAIR.md): compare the two laws on the production trajectory and filter, with a complete G3 reference matrix.
- [Pickup convergence](docs/PICKUP-CONVERGENCE.md): separate mechanical and pickup/filter sampling residuals across registers, with finite 128x/256x references.
- [Pickup listening](docs/PICKUP-LISTENING.md): three performances with fixed global RMS matching, sample-peak control and measured full-keyboard headroom.
- [Numerical convergence](docs/CONVERGENCE.md): the treble-contact correction, experiment and residual errors.
- [Desktop audition](docs/AUDITION.md): build, install and launch each test version.
- [Sources](docs/SOURCES.md): primary research and evidence scope.
- [September research review](docs/RESEARCH-2026-09.md): papers, Rust projects, recording candidates and prioritized experiments.
- [Development](docs/DEVELOPMENT.md): commands, integration and output formats.
- [Validation results](docs/VALIDATION.md): tests, native/WASM timing and remaining limitations.

The next milestone is a calibrated A3: compare multiple intensities and decay phases against documented direct recordings, then extend validated parameters across the keyboard. All project code, tools, tests and documentation are in English; executable project code is Rust.
