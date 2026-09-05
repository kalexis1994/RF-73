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

Independent tracking now supports 32/128/512/1024 ms observations with reported sample counts and FFT grids. Long observations use the full requested duration, including at 192 kHz. The default remains 128 ms, and harmonic summaries retain their original window.

The [September research review](RESEARCH-2026-09.md) specifies the next experiment: document reference metadata and run an A3/A4 pilot with held-out intensities using the new independent tracking. A4 provides a second anchor; neither note is calibrated yet.

The measurement foundation is implemented: external WAV input, multi-resolution spectra, bounded harmonic searches, RMS envelopes, qualified decay estimates, and raw/level-matched comparison reports. Known synthetic signals validate these estimators; they do not calibrate the instrument. See [Analysis laboratory](ANALYSIS.md).

`compare-partials` now pairs simultaneous detections across explicit equal-duration regions, preserving raw gain and reporting unmatched/ambiguous observations. Decay differences require qualified fits over identical fully paired intervals. See [Partial comparison](PARTIAL-COMPARISON.md).

`sweep-pickup` now evaluates up to 25 gap/offset combinations against an explicit held-note region, using one global RMS gain and multiple spectral windows. It preserves raw errors, reports near ties and leaves the instrument profile unchanged. Synthetic references verify recovery of a known grid geometry at two intensities; measured calibration and held-out validation remain outstanding. See [Pickup sweep](PICKUP-SWEEP.md).

`fit-pickup-set` now selects shared geometry and gain from multiple fitting takes, then evaluates only that frozen choice on held-out note/velocity pairs. Its strict manifest records provenance and explicit sustain boundaries. The [reference-set workflow](PICKUP-SET.md) is validated synthetically; the [sample-bank review](REFERENCE-BANKS.md) identifies commercial candidates and a small processed real-instrument pilot.

The [G3 residual pilot](G3-RESIDUAL-PILOT.md) now compares all five acquired source layers against three model velocities and two pickup geometries. A closer pickup reduces a strong-probe H3 deficit but selects the minimum supported gap and leaves substantial residuals. `compare-tone` provides explicit attack/body diagnostics without inferred velocities or decay fits; the production profile remains unchanged. The next isolated physical experiment targets pickup transfer shape and excitation scale, including nonlinear aliasing.

The [isolated pickup experiment](PICKUP-TRANSFER.md) compares the current law with a more localized point-pole field proxy under identical periodic motion. It reports raw sensitivity, harmonic balance and internal sampling residuals separately. [Mechanical pickup pairs](PICKUP-PAIR.md) now extend this to the production trajectory and FIR, preserving baseline WAVs exactly. The close-gap strong probe further reduces the G3 third-harmonic deficit but leaves substantial upper-harmonic/body residuals and increased output level. Cross-register convergence and gain/headroom evaluation remain necessary before a sound-profile release.

The [pickup convergence matrix](PICKUP-CONVERGENCE.md) now covers three register anchors, two intensities and two geometries, with treble checks at all output rates and 256x reference confirmations. The tested frozen-trajectory residuals are much smaller than the remaining contact/trajectory differences. Candidate gain/headroom and level-matched listening comparisons are the next release gates; no whole-keyboard or perceptual qualification is claimed.

1. Obtain dry recordings from a documented instrument at multiple intensities.
2. Evaluate independent tracking across the available observation lengths on those recordings, refine background qualification, and distinguish observed spectral peaks from identified mechanical modes.
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
