# RF-Rhodes

A Rust physical-model electric piano research project for RackForge.

The first working prototype includes a nonlinear hammer-contact solver, a three-mode resonator per key, a geometry-dependent magnetic pickup, sustain and sample-accurate MIDI. It renders audio offline and compiles to a portable RackForge WASM plugin.

Version 0.1.1 adds measured numerical convergence and contact-only refinement for upper-register accuracy. The physical profile remains provisional.

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
- `rf-rhodes-plugin`: RackForge adapter with MIDI 1.0/2.0, output gain, program and versioned state.
- `rf-rhodes-analysis`: offline WAV input, FFT spectra, harmonic and independent partial tracks, qualified decay estimates and aligned comparisons.
- `rf-rhodes-lab`: Rust WAV renderer, physical CSV traces, measurement commands, JSON reports and timing diagnostics.
- Tests for mechanical passivity, repeated strikes, dampers, MIDI ownership, block invariance, malformed input and file integrity.

The audio path uses no explicit allocation, locks or I/O. The laboratory does not open an audio device. No samples, reverb, amplifier or normalization hide the direct model output.

## Read next

- [Physical model ledger](docs/MODEL.md): equations, constants and known approximations.
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
- [Numerical convergence](docs/CONVERGENCE.md): the treble-contact correction, experiment and residual errors.
- [Desktop audition](docs/AUDITION.md): build, install and launch each test version.
- [Sources](docs/SOURCES.md): primary research and evidence scope.
- [September research review](docs/RESEARCH-2026-09.md): papers, Rust projects, recording candidates and prioritized experiments.
- [Development](docs/DEVELOPMENT.md): commands, integration and output formats.
- [Validation results](docs/VALIDATION.md): tests, native/WASM timing and remaining limitations.

The next milestone is a calibrated A3: compare multiple intensities and decay phases against documented direct recordings, then extend validated parameters across the keyboard. All project code, tools, tests and documentation are in English; executable project code is Rust.
