# Development roadmap

All application code, analysis tools and tests are Rust. Documentation, identifiers and user-facing strings are English.

## Implemented in 0.1.0

- Independent Cargo workspace and locked toolchain.
- Safe Rust DSP core with bounded implicit hammer contact and free modal decay.
- Provisional nonlinear pickup, 4x processing and FIR decimation.
- 73-key research engine with sustain, channel ownership and retrigger continuity.
- Rust WAV renderer, physical CSV probes, signal reports and native stress runner.
- RackForge SDK adapter with MIDI 1.0/2.0, gain, program and versioned state.
- Native and WASM build targets; development package metadata.
- Numerical, MIDI, block-invariance, input-validation and WAV integrity tests.

## Implemented in 0.1.1

- A reproducible Rust convergence experiment with fixed 4/8/16/32x paths and a 64x reference.
- A common offline filter, equal physical time sampling, contact diagnostics and attack comparisons.
- Contact-only subdivision that reduces the measured soft-treble integration error without raising the continuous pickup processing rate.
- Regression coverage for refined passivity, unchanged A3 behavior, treble accuracy, filter integrity and CLI reports.

## Next milestone: a calibrated A3

Implemented research tooling after 0.1.1: independent spectral peak tracking, local/global background estimates, resolution and capacity flags, contiguous association, and qualified per-track decay with explicit rejection reasons. Single-note analysis now uses schema 2. The production instrument profile is still 0.1.1.

The [September research review](RESEARCH-2026-09.md) specifies the next experiment: document reference metadata and run an A3/A4 pilot with held-out intensities using the new independent tracking. A4 provides a second anchor; neither note is calibrated yet.

The measurement foundation is implemented: external WAV input, multi-resolution spectra, bounded harmonic searches, RMS envelopes, qualified decay estimates, and raw/level-matched comparison reports. Known synthetic signals validate these estimators; they do not calibrate the instrument. See [Analysis laboratory](ANALYSIS.md).

1. Obtain dry recordings from a documented instrument at multiple intensities.
2. Evaluate the implemented independent tracking on those recordings, refine background qualification, and use longer observations where close components remain unresolved; distinguish observed spectral peaks from identified mechanical modes.
3. Extend the current convergence measurements to extreme profiles, retriggers and isolated nonlinear aliasing; establish explicit error budgets.
4. Identify modal frequencies, weights, losses and pickup geometry jointly, keeping a held-out validation set.
5. Add hammer hysteresis and assembly modes where measured residuals justify them.
6. Produce matched-level blind listening pairs and an error report.

## Later milestones

| Stage | Deliverable | Exit criterion |
| --- | --- | --- |
| Registers | Calibrated bass, mid and treble anchors | Unfitted notes interpolate acceptably |
| Full action | Dampers, release dynamics and regulated repetition | Recorded gestures reproduce plausibly |
| Electronics | Documented Stage/Suitcase circuit profiles | Stage-by-stage bypass comparisons |
| Product | Branded package and musical controls | Validated install and saved-session roundtrip |
| Qualification | Native, browser, Android and Pi measurements | Device-specific deadlines and listening criteria |

Initial performance target: keep the plugin below half the block deadline on each target. A 128-frame block at 48 kHz lasts 2.667 ms; 1.333 ms is the provisional plugin budget. A short desktop run does not qualify a stage instrument or establish mobile performance.

The current sound is an audible research result, not a claim of high-fidelity Rhodes reproduction. Hitting numerical tolerances is necessary but does not establish perceptual equivalence.
