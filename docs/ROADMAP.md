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

## Next milestone: a calibrated A3

1. Obtain dry recordings from a documented instrument at multiple intensities.
2. Add Rust spectral analysis: partial tracking, multi-resolution attack spectra, energy envelopes and decay estimates.
3. Compare the current solver to a finer-time-step reference; quantify contact integration error and aliasing.
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
