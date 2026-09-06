# Initial validation report

Historical results are retained below. The latest local results are in the component comparison section at the end.

Date: 2026-09-04. RF-73 0.1.0 research prototype.

Environment: Windows, AMD Ryzen 5 5600X, Rust 1.98.0, Windows GNU toolchain. RackForge source revision `7c17bd4a480d1c0bd7fa18fa4d880e82429dffe1`; its host tools were rebuilt before integration validation.

## Passed locally

- 18 Rust tests: 11 DSP/filter/physics tests, 4 plugin contract tests, 3 CLI/WAV tests.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`.
- `cargo fmt --all -- --check`.
- Native release build of the entire workspace.
- `wasm32-unknown-unknown` release build of the plugin.
- RackForge metadata validation, WASM load, program selection, gain roundtrip, rendered output and 16-byte state smoke test.
- Rust inspection of both generated WAV files: correct frame counts, finite data and consistent headers.

The tests check mechanical energy through hammer contact, separation at multiple rates and registers, finite output under an extreme profile, continuous retriggering, channel ownership, late-pedal capture, state rejection and sample-identical block partitioning. Sustained mechanical pitch tests stay within 0.1 cents of the programmed frequency at the tested notes and rates; this is an oscillator test, not comparison to a real Rhodes.

## Short timing measurements

| Path | Workload | Result |
| --- | --- | --- |
| Native Rust | 73 keys, periodic full-velocity retriggers, 1125 blocks of 128 frames at 48 kHz | Worst 1.403 ms; p99 0.834 ms; zero deadline misses; zero numerical faults |
| RackForge WASM | 60 distinct notes, 256 blocks of 128 frames at 48 kHz | Wall/audio ratio 0.199; maximum 10,586,033 fuel units; completed successfully |

The 48 kHz block deadline is 2.667 ms. The native worst case exceeds our provisional half-deadline budget of 1.333 ms, despite meeting the full deadline in this run. These short desktop measurements include scheduling variation and are not real-time guarantees, latency measurements or mobile qualification. The host stress tool reports aggregate elapsed time, not a worst-callback time.

The raw native stress peak was 17.079; the WASM chord peak was 10.593. Float output intentionally retains signal headroom without a hidden limiter. Dense chords require lower monitoring gain. The ordinary ten-second demo peaks at 0.962; the single A3 reference peaks at 0.218.

## Artifacts

- `renders/first-demo.wav`: ten seconds, A3 dynamics and a sustained chord, 48 kHz mono float.
- `renders/a3-0.1.0.wav`: four-second A3 research render, 48 kHz mono float.
- `renders/a3-0.1.0.csv`: physical probes with post-step timestamps. Older exploratory files remain in the ignored renders directory.
- `dist/native-stress.json` and `dist/wasm-stress.txt`: timing output.
- `dist/RF-Rhodes-0.1.0.rfplugin`: 25,491 bytes.
- Package SHA-256: `70095e2fb1a2cee58fd4230d6af6b0c5b61f87f8dec0237bb5c998d5da978879`.

The archive was produced after host validation. Its physical model is unchanged from the demo; the later pedal fix adds recapture of released tails.

## Not yet demonstrated

Real-instrument timbral fidelity, measured hammer material response, high-order assembly modes, spectral convergence/aliasing bounds, browser execution, Android/Pi timing, actual audio-device latency and long-duration soak behavior.

The initial commit passed hosted CI on Windows and Linux: [run 33915204770](https://github.com/kalexis1994/RF-73/actions/runs/33915204770).

## Offline analysis milestone

The analysis addition passed 35 local Rust tests, strict Clippy, formatting, the native release build and the WASM plugin build on the same environment. The 17 new tests cover numerical measurement, external WAV input and the complete CLI flow. The plugin dependency tree still contains only the DSP and RackForge SDK; offline decoding and serialization dependencies do not enter its audio path.

The following measurements come from the unchanged research model: A3, 48 kHz, four-second files, key release at two seconds, default pickup geometry, sustain-fit boundary at 1.8 seconds. Values describe model output, not a real instrument.

| Normalized velocity | Peak dBFS | Full-file RMS dBFS | Fundamental Hz | Extrapolated T60 s | 32 ms attack centroid Hz |
| --- | --- | --- | --- | --- | --- |
| 0.20 | −29.302 | −44.664 | 220.0004 | 5.001 | 237.78 |
| 0.55 | −16.510 | −32.475 | 220.0004 | 5.016 | 250.85 |
| 1.00 | −8.306 | −25.332 | 220.0005 | 5.056 | 313.66 |

All three render reports show zero numerical faults. Decay-fit R² exceeds 0.9996, but only the declared sustain interval was observed: the roughly five-second T60 values are extrapolations. Comparing the quiet and loud takes without time adjustment gives +19.332 dB full-file RMS difference and 0.384 level-matched normalized waveform error. The centroid change supports a measurable attack-spectrum change; neither metric is a perceptual fidelity score.

Artifacts are ignored under `renders/`: `a3-velocity-{0.2,0.55,1.0}.wav`, their `-analysis.json` reports, `a3-dynamics-comparison.json`, `a3-baseline-analysis.json` and `a3-self-comparison.json`. The baseline file compared with itself gives exactly zero delay, level difference and normalized waveform errors.

To reproduce, render each listed velocity using default settings, then run `analyze` with `--note 57 --sustain-end 1.8`; compare the 0.2 and 1.0 files with `--align-ms 0`. See [Analysis laboratory](ANALYSIS.md) for commands and precise metric definitions. The CI workflow now also analyzes and compares its generated WAV.

## Version 0.1.1: contact convergence correction

The convergence laboratory revealed a soft-treble contact integration error. Contact-only subdivision reduces the 48 kHz/MIDI 100/velocity 0.2 attack discrepancy from 10.540% to 0.125% against the same finite 64x reference. The [convergence report](CONVERGENCE.md) documents the method, 21-case grid and limitations. A3 at velocity 0.7 remains sample-identical to its 0.1.0 render, verified with zero raw and matched waveform errors.

Validation on the same Windows/Ryzen environment:

- 41 Rust tests, including refined contact passivity, fixed-reference convergence, the soft-treble regression, offline FIR response/ring ordering and CLI failure behavior.
- Block-invariant plugin output with both A3 and the refined highest note, including retrigger and pedal events.
- Strict Clippy, formatting, native release and WASM release builds.
- RackForge metadata validation, WASM loading, program selection, gain roundtrip and 16-byte state smoke test.
- Native stress: 73 keys, 1125 blocks at 128 frames/48 kHz, worst 2.3625 ms, p99 0.716 ms, zero deadline misses and zero numerical faults.
- WASM stress: 60 distinct notes, 256 blocks at 128 frames, maximum fuel 14,176,281, wall/audio ratio 0.185, successful completion.

The native worst block still exceeds the 1.333 ms provisional half-deadline target, despite remaining below the full 2.667 ms deadline in this short run. Scheduling affects wall time; the lower aggregate WASM ratio does not prove a speedup. Maximum WASM fuel increased from the initial 10,586,033 to 14,176,281, consistent with the additional contact work. No mobile or long-duration real-time qualification is claimed.

New ignored artifacts: `renders/a3-0.1.1.wav`, `renders/treble-0.1.1.wav` and its physical CSV, `renders/a3-version-comparison.json`, `renders/refined-*.json`, `dist/native-stress-0.1.1.json`, `dist/wasm-stress-0.1.1.txt`, and the validated `dist/RF-Rhodes-0.1.1.rfplugin` archive (25,662 bytes, SHA-256 `5a119fb8001bbaee6511afbf9ab28b6d2c4605f633976b3e6b39b498993eb8cc`). The earlier 0.1.0 archive is retained separately.

## Independent partial tracking: analysis schema 2

Date: 2026-09-04. This is an offline laboratory addition following 0.1.1, with no new instrument release. The 55 current Rust tests passed locally (54 in the workspace run, followed by the additional capacity-qualification unit test and the existing analysis unit suite). Strict Clippy, formatting and native/WASM release builds passed. Eleven new tests cover independent tracking and its rejection behavior; the existing CLI test also checks the schema-2 payload and no-overwrite contract.

The deterministic two-component fixture recovers 220.37 and 731.23 Hz with independent 5.0 and 2.5 second extrapolated T60 values at 8, 44.1, 48 and 192 kHz. Assertions bound frequency error below 0.15 Hz and T60 error below 0.04 seconds. Other cases verify floor censoring, no persistent white-noise tracks, sidelobe rejection for an isolated tone, close-tone ambiguity, interrupted tracks, short attacks, release exclusion and rejection of inconsistent decay slopes. Capacity tests exercise both report limits and refusal to qualify a fit from capacity-limited frames.

Six production-model renders were analyzed: A3/A4 at normalized velocities 0.2, 0.5 and 0.9, 48 kHz, three seconds per file, release at 2.5 seconds, sustain-fit end at 2.4 seconds, default pickup geometry. Every render reports zero numerical faults. No recording of a real instrument was used.

| MIDI note | Velocity | Spectral tracks | Qualified partial decays |
| --- | --- | --- | --- |
| 57 (A3) | 0.2 | 9 | 3 |
| 57 (A3) | 0.5 | 12 | 3 |
| 57 (A3) | 0.9 | 17 | 5 |
| 69 (A4) | 0.2 | 8 | 2 |
| 69 (A4) | 0.5 | 9 | 3 |
| 69 (A4) | 0.9 | 15 | 3 |

These are track counts, not counts of physical modes: a missed frame starts a new track, and some tracks are short transients or mixing products. The loud A3 contains an early track near 1378.72 Hz, consistent with its programmed second beam mode, plus nearby components around 1158.72 and 1598.72 Hz, consistent with mixing with the 220 Hz fundamental. Those short tracks receive `insufficient_points` rather than a T60. This is a model-based interpretation, not experimental identification of a real tine or tonebar.

All six reports have zero dropped track observations. Minimum separation is reported as 15.625 Hz; interpolated frequencies do not improve that resolution limit. Generated WAVs, render receipts and schema-2 reports are under `renders/inharmonic-20260904-184419/` (ignored). For example, the loud A3 report is `note-57-v-0.9-analysis.json`.

Reproduce one case with fresh output names:

```text
cargo run --locked --release -p rf-73-lab -- render --output renders/tracking-a3.wav --note 57 --velocity 0.9 --seconds 3 --hold 2.5
cargo run --locked --release -p rf-73-lab -- analyze renders/tracking-a3.wav --output renders/tracking-a3-analysis.json --note 57 --sustain-end 2.4
```

This validates the new measurement path against known signals and current model output. Untreated reference recordings, physical parameter identification and an audible realism improvement remain the next experiment.

## Selectable tracking windows

Date: 2026-09-04. Following commit `2787676`, the laboratory adds `--partial-window-ms 32|128|512|1024`. All 60 Rust tests passed in the workspace run, including five new window tests and expanded CLI option/error coverage. Strict Clippy, formatting, native release and WASM release builds passed locally. The instrument remains the 0.1.1 research profile.

The 1024 ms synthetic fixture separates 700.3 and 705.7 Hz with less than 0.04 Hz frequency error and continuous, unambiguous tracks. A 192 kHz fixture places its tone after the previous 32,768-sample cap: detection and metadata verify use of all 196,608 samples, a 524,288-point FFT and one complete 1024 ms observation. Other fixtures cover a 96 ms attack recording, unchanged harmonic measurements, known decay with release exclusion, invalid windows and insufficient file duration. These are controlled cases, not general precision guarantees.

A six-second A3 production render at 48 kHz/velocity 0.9 was measured at every supported window, with release at 5.5 seconds and fit boundary at 5.4 seconds. Its render reports zero numerical faults.

| Window ms | Minimum separation Hz | Tracks | Qualified partial decays |
| --- | --- | --- | --- |
| 32 | 62.5 | 40 | 3 |
| 128 | 15.625 | 19 | 5 |
| 512 | 3.90625 | 17 | 3 |
| 1024 | 1.953125 | 38 | 2 |

Track counts include short artifacts, mixing components and separate tracks after missed frames. They are not physical-mode counts or a ranking of window quality. The fundamental estimate and legacy RMS decay match across all four reports, as expected from the independent measurement paths. Each report has zero dropped track observations. Long windows improve frequency separation but smear the attack and leave fewer fit observations.

Ignored artifacts: `renders/windows-20260904-185203/a3.wav`, its render receipt and `analysis-{32,128,512,1024}.json`. Reproduce with a new output name using `render --note 57 --velocity 0.9 --seconds 6 --hold 5.5`, then `analyze --note 57 --sustain-end 5.4 --partial-window-ms WINDOW` for each window. See [Analysis laboratory](ANALYSIS.md) for full command examples and resolution limits.

## Component comparison

Date: 2026-09-04. Added the offline `compare-partials` command. All 71 Rust tests passed locally, including ten new comparison fixtures and a CLI roundtrip/error test. Strict Clippy, formatting, native release and WASM release builds passed. A parallel CLI test exposed a timestamp collision in Windows temporary-directory naming; directory reservation now combines an atomic counter with the timestamp and bounded retries, never adopting an existing directory.

Comparison tests preserve identity and raw gain differences, recover a known +20-cent pitch change and +2-second extrapolated T60 difference, and verify unmatched components, silence, ambiguous alternatives, explicit sample offsets, mismatched fit intervals, incomplete pairing and invalid inputs. An amplitude-modulated fixture has near-zero mean level error but more than 1.8 dB RMS error, verifying that opposite errors do not cancel. A dense spectral fixture exceeds detection capacity and correctly disables matching.

The production-model experiment reuses the existing three-second A3/48 kHz renders at velocities 0.2 and 0.9. Both selected regions run from 0.15 to 2.15 seconds, before their 2.5-second release; tracking uses 128 ms and a 50-cent matching gate. No new reference recordings or physical parameters were introduced.

Self-comparison finds three track pairs, 136 paired observations, three qualified decay comparisons and no unmatched/ambiguous observations. Frequency and amplitude differences are zero. Comparing the quiet render against the loud render also finds three pairs and 136 paired observations, with 71 additional candidate observations lacking a counterpart. Both reports have complete detection under the configured capacity limits.

The loud selected region is 18.093 dB higher in RMS. Per-pair results are:

| Reference frequency Hz | Paired observations | Raw mean level difference dB | Level-matched mean difference dB | Level-matched RMS error dB | Decay comparison |
| --- | --- | --- | --- | --- | --- |
| 220.002 | 59 | 18.048 | −0.045 | 0.209 | Qualified; candidate T60 +0.106 s |
| 440.001 | 59 | 36.394 | 18.300 | 18.300 | Qualified; candidate T60 +0.004 s |
| 660.002 | 18 | 54.137 | 36.044 | 36.045 | Rejected: different fit intervals |

This demonstrates the current model's velocity-dependent spectrum after preserving the original level difference. It does not establish realistic Rhodes dynamics or identify mechanical losses from electrical-output decays. The third component's decay is intentionally not compared because the qualified fits cover different time spans.

Ignored reports: `renders/partial-pairs-20260904-192121/identity.json` and `dynamics.json`. Inputs are `renders/inharmonic-20260904-184419/note-57-v-{0.2,0.9}.wav`. Reproduce with a new report name using `compare-partials REFERENCE.wav CANDIDATE.wav --seconds 2 --reference-start 0.15 --candidate-start 0.15 --output REPORT.json`. The [comparison specification](PARTIAL-COMPARISON.md) defines the metrics and exclusions.

Builds for this milestone used session-local `CARGO_INCREMENTAL=0` after disk-space cleanup, so they did not recreate the deleted incremental caches. The instrument remains the 0.1.1 research profile; this milestone changes the measurement tools.

## Bounded pickup sweep

Date: 2026-09-04. Added `sweep-pickup` to the offline laboratory. All 75 Rust tests passed, including known-geometry CLI recovery, output preservation, invalid grid/region rejection, insufficient-energy handling and a global-gain fixture. Strict Clippy, formatting, native release and WASM release builds passed. Builds used session-local `CARGO_INCREMENTAL=0`.

The gain fixture compares a stationary sinusoid to a uniformly attenuated copy and to a copy with different levels in its first and second halves. Uniform attenuation produces less than 1e-10 dB objective error after matching; the time-varying case exceeds 0.1 dB. This verifies that each window uses the same whole-region gain and does not independently normalize away envelope differences.

Two synthetic A3 references use a non-default 2 mm gap and 0.75 mm offset, at normalized velocities 0.3 and 0.7. Both are 1.5-second production-engine renders at 48 kHz, with release at 1.4 seconds and zero numerical faults. Each sweep selects 0.1..1.1 seconds in both signals, testing gaps 1.5/2/2.5 mm against offsets 0.5/0.75/1 mm: nine candidates and 15 scored windows per candidate.

| Velocity | Best gap mm | Best offset mm | Best objective dB | Next-best objective dB | Candidates within 0.01 dB of best |
| --- | --- | --- | --- | --- | --- |
| 0.3 | 2 | 0.75 | 0 | 0.12 (rounded) | 1 |
| 0.7 | 2 | 0.75 | 0 | 0.20 (rounded) | 1 |

Both known geometries are recovered exactly, with zero raw waveform NRMSE and unity matching gain. All 18 candidate renders are accepted. These are two independent synthetic recovery runs, not held-out validation against a measured instrument or proof of uniquely identifiable physical dimensions.

Ignored artifacts are `renders/pickup-sweep-20260904-193143/reference-{0.3,0.7}.wav`, their render receipts and `sweep-{0.3,0.7}.json`. The [pickup sweep specification](PICKUP-SWEEP.md) provides reproduction commands, the window objective and its limitations. Candidate WAVs are not saved. The production instrument remains the 0.1.1 research profile; this milestone provides calibration tooling rather than a new sound version.

## Shared fit and held-out evaluation

Date: 2026-09-04. Added `fit-pickup-set` with a strict bounded reference manifest and a gain shared across fitting takes, frozen for validation. All 77 Rust tests passed. Strict Clippy, formatting, native release and WASM release builds passed with session-local `CARGO_INCREMENTAL=0`.

The new shared-gain fixture recovers a uniform 2x recording gain with less than 1e-10 dB objective error. Scaling the two fitting intensities differently leaves more than 0.1 dB error instead of independently normalizing them. The CLI fixture changes only a held-out reference to a different pickup geometry: every fitting candidate and the ranking remain exactly identical, the applied validation gain stays at 1, and the held-out objective exceeds 0.05 dB with waveform NRMSE above 0.1. Additional cases reject split leakage, reused files, missing provenance, unknown fields, invalid schema/grid, insufficient fitting pairs and regions crossing release, and preserve existing reports.

The checked-in `references/pickup-set.synthetic.json` was run against fresh A3/48 kHz renders at velocities 0.3 and 0.7 for fitting, reserving 0.5 for validation. All use a 2 mm gap and 0.75 mm offset, duration 1.5 seconds and release at 1.4 seconds; selected regions are 0.1..1.1 seconds. Nine geometries were fitted. The selected index is 4, exactly recovering the known geometry and unity gain, with zero fitting and validation objective. It is the only candidate within the 0.01 dB reporting band. The three source renders have zero numerical faults.

Ignored artifacts are under `renders/pickup-set-demo/`; `result.json` records every fit and the frozen validation result. This confirms the split/gain workflow against synthetic data. It does not establish real-instrument calibration. The instrument remains the 0.1.1 research profile.

## Real-reference tone residuals

Date: 2026-09-04. Added `compare-tone`, explicit 32/96 ms attack and 250..600 ms body spectra, per-harmonic ratios/differences and raw window levels without inferred sustain boundaries or decay estimates. Both pickup search tools now mark selection limits. All 84 Rust tests passed; strict Clippy, formatting, native release and WASM release builds passed with session-local `CARGO_INCREMENTAL=0`.

Five new analysis fixtures cover known gain and harmonic-balance changes, missing components/fundamental, silence, insufficient cycles, explicit offsets, complete 67,200-sample body observations at 192 kHz and rejected invalid/truncated regions. A CLI roundtrip verifies no-overwrite and option errors without a sustain claim. A boundary fixture distinguishes unsorted-grid endpoints, supported-profile limits, fixed axes and interior selections.

The [G3 residual pilot](G3-RESIDUAL-PILOT.md) compares five acquired real-instrument layers against six fresh model renders. Thirty compact tone reports and a tracked numeric summary preserve the results. A 25-candidate strong-layer sweep reduces its windowed spectral objective from 8.7878 to 7.3568 dB by selecting gap 0.5 mm/offset 0.25 mm. This is at the minimum supported gap. At model velocity 0.9, the H3/H1 deficit against the strongest reference falls from 21.90 to 6.49 dB in the 96 ms observation and from 25.97 to 9.71 dB in the body. Those model/reference strike intensities are not independently matched.

All six renders have zero numerical faults; the close-gap loud render exceeds unity peak in unclipped float output. That exploratory profile was not promoted to the plugin. The observed improvement is diagnostic, with processed-source, capture-gain, strike and alignment uncertainty; it is not a validated audible release or proof of realistic physical geometry.

## Isolated magnetic transfer

Date: 2026-09-04. Extracted the unchanged production transfer into `MagneticPickup` and added `pickup-transfer` for analytic periodic motion. All 90 Rust tests passed. Formatting, strict Clippy, native release and WASM release builds passed with session-local `CARGO_INCREMENTAL=0`. Six new tests cover independent finite-difference flux derivatives, validated geometry and reflection symmetry, Fourier phase/DC/Parseval energy, even-only centered-pickup harmonics, demanding-motion sampling convergence and CLI validation/no-overwrite.

Three new MIDI 55/44.1 kHz production renders at velocities 0.2/0.5/0.9, duration 3 seconds and hold 2.8 seconds, have exactly the same SHA-256 hashes as the preceding G3 pilot WAVs. All have zero numerical faults. A 73-key/48 kHz native stress run reports zero faults and deadline misses, p99 0.5876 ms and worst block 2.2261 ms against a 2.6667 ms deadline. This short desktop observation is not device qualification; its worst block also exceeds the provisional half-deadline budget.

The [experiment specification](PICKUP-TRANSFER.md) defines the two flux proxies and their limitations. Full numerical reports are tracked in `references/pickup-transfer/{default,close,treble}.json`. Original run artifacts and compatibility WAVs are under ignored `renders/pickup-transfer-20260904-230603/`.

At 196 Hz, the relative third harmonic increases with the more localized field proxy:

| Gap / offset mm | Amplitude mm | Production H3/H1 dB | Point-pole H3/H1 dB | Change dB |
| --- | --- | --- | --- | --- |
| 1.5 / 0.5 | 0.05 | -60.56 | -56.85 | +3.71 |
| 1.5 / 0.5 | 0.25 | -32.63 | -28.87 | +3.77 |
| 1.5 / 0.5 | 0.75 | -13.85 | -9.73 | +4.12 |
| 0.5 / 0.25 | 0.05 | -44.42 | -41.89 | +2.54 |
| 0.5 / 0.25 | 0.25 | -16.26 | -13.02 | +3.25 |
| 0.5 / 0.25 | 0.75 | +1.34 | +6.43 | +5.09 |

This is not simply a brighter copy of the same spectrum: at the close geometry and 0.05 mm amplitude, H2/H1 falls from -27.84 to -60.37 dB. The geometry is near a small-displacement curvature cancellation for the point-pole proxy. Raw sensitivity also changes: at default geometry and 0.75 mm amplitude, ideal-band RMS rises from 1.8131 to 4.3308 in arbitrary pre-engine units. Neither the amplitude probes nor their gain are matched to the recorded instrument.

All twelve 196 Hz observations have 4x sampling residuals below the -160 dB reporting floor in the ideal output band. The 64x residuals against 128x are also below that floor. This does not assess the production FIR's stopband leakage or nonperiodic attacks.

An intentionally demanding 2,756.25 Hz sinusoid, 3 mm amplitude and 0.5/0.25 mm geometry exposes a difference:

| Law | 4x ideal-band NRMSE | 4x error dB | 8x error dB | 64x error dB |
| --- | --- | --- | --- | --- |
| Production | 0.0004659 | -66.63 | -154.87 | Below -160 |
| Point-pole proxy | 0.0044734 | -46.99 | -129.22 | Below -160 |

That synthetic trajectory reaches approximately 52 m/s peak tine velocity; it is a numerical stress case, not a claimed attainable Rhodes motion. At the same frequency with 0.75 mm amplitude, both 4x residuals are below the reporting floor. These results justify testing the alternative on actual simulated mechanical trajectories with the production filter before selecting a sound change. They do not establish an audible realism gain. The plugin remains the 0.1.1 research profile; no new audition version was produced.

## Mechanical pickup pairs with production filtering

Date: 2026-09-04. Added `render-pickup-pair`, exposing the existing production decimator for offline reuse. One voice supplies both magnetic laws with the same internal-rate trajectory; the command writes two raw float WAVs and an attack/body comparison receipt. All 92 Rust tests passed, as did strict Clippy, formatting, native release and WASM release builds. Builds used session-local `CARGO_INCREMENTAL=0`.

The new identity fixture verifies every production sample against `Engine`, including release, for 24 combinations: four supported sample rates, notes 28/55/100 and two geometry/velocity pairs. A CLI fixture verifies byte-identical WAV output, readable alternative audio, internal sample count and tone-window coverage. It also rejects invalid parameters and preserves existing production, alternative and receipt files without creating a partial pair for those validation failures.

Six G3 pairs and thirty comparisons against all five source layers are complete. All six production WAVs match the previous pilot's SHA-256 hashes; all twelve outputs have zero numerical faults. The [experiment report](PICKUP-PAIR.md) and tracked `references/g3-pickup-pair-summary.json` preserve the full numeric matrix and its interpretation limits. At close geometry/velocity 0.9, the H3 deficit against the strongest source is now 1.51 dB in the 96 ms attack and 5.65 dB in the body, compared with 6.49/9.71 dB using the original law. Body H4 and H6 remain about 14.02/16.12 dB too low relative to H1, and raw peak rises to 2.41225.

This milestone establishes the candidate's effect under actual model motion and production filtering; it does not establish high-rate convergence or an audible realism gain. The plugin and its 0.1.1 research profile are unchanged. No new audition package was produced.

## Pickup convergence across registers

Date: 2026-09-04. Added `converge-pickup` with independent 8..128x paths, selectable finite 128x/256x references, the actual 4x production path and exact reference-state subsampling at 4x. Denser filters sample the production kernel's physical support and preserve its 15.75-output-sample delay. The standalone research voice now accepts powers of two through 256; production voice preparation is unchanged.

All 96 Rust tests passed, as did strict Clippy, formatting, native release and WASM release builds. Session-local `CARGO_INCREMENTAL=0` was used. Four new tests cover sampled-filter impulse identity and physical delay, raw gain/silence metric behavior, production-engine sample identity/frozen-path identity, and CLI 128x/256x reference structure, decreasing independent residuals, invalid bounds and output preservation. Existing research-constructor/passivity tests now exercise 128x and 256x; the original `converge` fixtures still pass.

The [23-case experiment](PICKUP-CONVERGENCE.md) includes 18 initial 128x comparisons and five 256x treble confirmations. Every observed trajectory separates and stays within the existing numerical passivity tolerance; the maximum positive energy step is 4.34e-19 J. In the tested point-pole paths, the largest attack/full production residual against 256x is 0.166644%/0.178419% for soft MIDI 100 at 192 kHz. The 44.1 kHz soft-treble case changes from 0.036837% attack error against 128x to 0.046044% against 256x, illustrating why a finite reference needs its own check.

The point-pole frozen-trajectory attack residual is at most 0.00002803% in the confirmations, much smaller than the mechanical/trajectory residual. This supports keeping the continuous pickup rate at 4x for these cases; it is not a pure aliasing bound or a whole-keyboard qualification. Single-note close-geometry peak reaches approximately 2.8661, so candidate output gain and headroom remain release work. Full reports stay under ignored `renders/pickup-convergence-20260904-232659/`; the tracked `references/pickup-convergence-summary.json` retains the numeric matrix and report hashes. No plugin sound change or audition package was produced.

## Global RMS listening tracks and observed headroom

Date: 2026-09-04. Added `pickup-listening` with a fixed 24-second performance, shared mechanical voices and three independently filtered pickup paths. One constant gain per complete track matches global RMS; a common attenuation enforces the requested sample ceiling after f32 conversion. A complete diagnostic pass measures every isolated key and repeated ten-/73-key strikes at full velocity.

All 100 Rust tests passed. Formatting, strict Clippy and native/WASM release builds passed with session-local `CARGO_INCREMENTAL=0`. Four new tests cover constant-gain/RMS and shared-peak behavior, bounded event timing, exact current-track agreement with `Engine` through overlapping notes/retriggers/late pedal, and complete CLI export verification. The latter reads all three WAVs independently, confirms matching RMS within 1e-7 relative tolerance and samples below -6 dBFS, checks all 73 diagnostic entries, and verifies input rejection/output preservation. The full debug export fixture is intentionally heavier than the unit tests.

The [listening study](PICKUP-LISTENING.md) ran at 44.1 and 192 kHz, with zero faults. Only the 44.1 kHz run wrote audio: three 24-second mono WAVs totaling approximately 12.7 MB. Their peaks are 0.487827/0.501187/0.464485 and RMS is approximately 0.028308 for all tracks. Raw whole-program RMS matching factors are 1/0.296794/0.136160, followed by common attenuation 0.507194. These are listening gains, not selected plugin gains or perceptual loudness calibration.

Across the two rates, 146 isolated-note observations and four repeated-chord observations expose headroom beyond the listening program. The largest observed point-pole isolated peak is 3.429340; repeated ten-/73-key peaks reach 19.385435/139.716248. Raw float output is not clipped or compressed. These bounded stress values are not a universal signal bound, and the exported WAV ceiling does not certify host processing or intersample peaks. Both receipts and WAV hashes are tracked in `references/pickup-listening-summary.json`. The production plugin is unchanged; this milestone provides listening artifacts and headroom measurements, not a new audition package or an asserted audible improvement.

## Version 0.1.2: real-time pickup laboratory and Rust PLAY UI

Date: 2026-09-05. The plugin now evaluates three continuously filtered pickup paths over one mechanical instrument, with frozen level matching and interruptible 20 ms fades. It exposes both a Rust WebAssembly PLAY panel and the RackForge declarative controller editor. All four settings are serialized; malformed state, automation and program documents reject atomically. The default gain is 0.100x. Schema-1 bytes can be decoded by the processor, but the pinned host still rejects cross-version preset references before decoding.

All 110 Rust tests passed: a complete 109-test workspace run followed by the added installed-UI integrity test. Native workspace Clippy, WASM UI Clippy and formatting passed. New coverage includes sample-identical independent pickup/filter paths at four rates and three register anchors, unchanged mechanical motion and pedal behavior through interrupted fades, block-invariant automation, complete editor/preview/install/state roundtrips, bounded catalogs, stale response rejection, serialized/coalesced UI writes, timeout recovery and rejection of missing/stale installed UI assets. Browser bindings are generated using wasm-bindgen-cli 0.2.127; the package workflow rebuilds the Rust UI before packing.

The [tracked receipt](../references/pickup-lab-validation.json) records observed local timing and package validation. At 48 kHz/128 frames, the native 73-key workload with rapid switching had p99 0.993 ms, worst 2.4208 ms and zero deadline misses. At 192 kHz/128 frames, it missed 1,289 of 4,500 deadlines, so that configuration is not qualified. Both runs had zero numerical faults but sample peaks around 2.437, confirming that the starting gain is not a universal bound. The fixed block-based restrike cadence also gives different strike intervals across these rates. RackForge's separate WASM workload completed at an elapsed/audio-duration ratio of 0.474 and maximum 18,481,867 fuel; its 73 Note On events cover 60 distinct pitches and do not exercise selector changes.

The final audition run is `dist/audition/0.1.2-1788579391438077200-17052/`. Its 237,223-byte archive has SHA-256 `6e26ce4eb3287739d8b4541f7b2c08834b30b9a97c08b364cfb43e1a1a0a964c`. Installation compared the DSP component and all four UI files byte for byte, then launched Desktop. Startup logs report WASAPI at 48 kHz; read-only host API requests confirm the active 0.1.2 PLAY surface and HTTP 200 for HTML, CSS, JavaScript bindings and UI WASM with appropriate MIME types. The earlier root-level `dist/RF-Rhodes-0.1.2.rfplugin` is a retained intermediate artifact without the screen UI; use the final audition archive identified here.

The final panel has not been visually exercised or judged by listening. Desktop automation was stopped at the user's request; subsequent verification used console commands, installed-file checks and read-only local API requests. No keyboard/mouse automation is part of the final validation. See [Pickup Lab UI](PICKUP-LAB-UI.md) for usage and limitations.

## Passive coupled assembly laboratory

Date: 2026-09-05. Added an independent three-coordinate tine/tonebar/common-support
mechanical candidate, with immutable SI parameters, reciprocal coupling,
nonlinear elastic hammer contact and an explicit stored/dissipated/escaped
energy ledger. The production engine and plugin state remain unchanged.
See [Coupled assembly](COUPLED-ASSEMBLY.md) for the reduction and its assumptions.

All 117 workspace tests passed, including seven new mechanics tests covering
analytic elastic collision, analytic fork/support normal modes, second-order
convergence, reciprocal impulse response, energy accounting, disconnected
components, overdamped parameter corners, invalid inputs and restrikes.
Strict native workspace Clippy, formatting and the release WASM plugin build passed.

The [tracked audit](../references/assembly-validation.json) reports 120 takes
across 24 note/rate/velocity cases. Maximum relative energy-balance residual was
`4.458e-10`; maximum positive single-step energy change was `5.146e-16` of injected
energy. Maximum 128/256-step displacement and velocity differences were
`0.01146%` and `0.01910%`. Every strike separated during the 50 ms observation.
The reproducible `assembly-check` command is included in CI; the updated remote
CI workflow has not been executed by this local validation.

The 4-step candidate reached `25.8869%` velocity error against the 256-step
reference in the treble despite remaining passive. Therefore this implementation
is an offline reference, not a numerically qualified 4x plugin replacement.
The current candidate omits higher bending modes, root rotation and calibrated
geometry. No new plugin version, installed package, listening result or real-time
qualification is claimed; no Desktop or native UI control was used for this milestone.

## Coupled free-motion and contact refinement

Date: 2026-09-05. The [refined assembly](ASSEMBLY-REFINEMENT.md) prepares exponential
free transitions and independent integrated dashpot work, with fine midpoint
steps only during hammer contact. Four base ticks per frame and 32 contact
subdivisions are validated for the unchanged provisional parameters.

All 121 workspace tests passed. New tests cover one-second analytic treble
phase/energy, free translation and overdamped decay/loss, exact interval ownership
at separation, and repeated damping/restrikes with parameter corners. Strict
native workspace Clippy, formatting and the release WASM plugin build passed.
After the full test run, the audit added a direct 32/64 contact comparison; the
final release audit and strict workspace Clippy also passed with that addition.

The [288-take audit](../references/assembly-refinement-validation.json) spans 48
note/rate/velocity cases. The 32-subdivision candidate's maximum velocity error
against midpoint 256 is `0.005829%`, versus `25.8869%` for the original 4x solver.
Comparing refined 32 directly with refined 64 yields at most `0.0005014%` velocity
and `0.0003628%` displacement differences. The independent energy-balance residual
is at most `4.023e-12` of injected energy. Shared original-midpoint scalar results
match the prior audit exactly. CI now runs both assembly audits; the updated
remote workflow has not been executed as part of this local verification.

An isolated native release probe measured median times of `2.1299 ms` for the
refined candidate versus `16.2842 ms` for continuous 128x, rendering 250 ms of one
voice with five strikes. This is not polyphonic, pickup/filter or host timing
qualification. The physical parameters remain uncalibrated and the candidate
still omits higher bending modes. No plugin version, installed package, listening
claim or Desktop interaction is part of this numerical milestone.

## Geometry-derived tine modes

Date: 2026-09-05. The [structural preparer](TINE-MODES.md) now derives six
fixed-root bending modes from a uniform circular Euler-Bernoulli beam and a
movable point mass. It exports effective masses, spatial hammer/pickup weights
and an 8x8 reciprocal inertia matrix for root translation, rotation and the six
modal coordinates. Preparation validates the generalized eigensystem, modal
mass orthogonality and positive-definite reduced inertia.

All 127 workspace tests passed. Six new tests cover analytical unloaded beam
frequencies/shapes and static compliance, continuous tip-mass boundary roots,
geometry scaling, continuous mass placement, spatial port weights, moving-root
inertia and invalid inputs. The targeted six-test suite also passed with the
final reduced-inertia preparation guard. Native workspace Clippy, formatting
and the release WASM plugin build passed.

The [tracked mesh audit](../references/tine-modes-validation.json) contains 12
tuning-mass/position configurations at 16/32/64 elements, with 216 modal results.
The maximum 32/64-element frequency difference is `0.006338%`; maximum absolute
hammer/pickup weight differences are `0.0003516` and `0.0001315`. Maximum relative
eigen-equation residual across all rows is `1.806e-7`. The geometry remains
illustrative, and convergence to these beam equations does not establish
instrument realism. CI includes the new audit; the updated remote workflow
was not executed during this local verification.

This preparer has not yet been connected to the time-domain assembly or plugin.
It does not model tonebar geometry, distributed spring inertia, damping,
polarization or nonlinear beam motion. The next integration must extend the
coupled mass matrix and recheck energy/contact accuracy for the higher modes.
No audio-device or Desktop interaction, new package or listening claim accompanies
this offline structural milestone.

## Nine-coordinate modal assembly

Date: 2026-09-05. The [multimode time-domain assembly](MODAL-ASSEMBLY.md) now uses
the tine's six derived bending modes and full moving-root inertia, with support
translation/rotation and a provisional tonebar bending coordinate. Hammer and
damper forces use their spatial shape ports; component masses are counted once.
Generalized midpoint contact, prepared mass-whitened free transitions and
independent dissipated-work integration preserve the energy ledger.

All 133 workspace tests passed. Six new tests cover component inertia/virtual
work, continuous rigid-motion limits, reciprocal hammer/tonebar impulses,
free-transition composition and independent loss, energy through contact/restrike/
damper changes, and invalid inputs. Strict workspace Clippy, formatting and the
release WASM plugin build passed. Original plugin/assembly behavior is unchanged.

The [final audit](../references/modal-assembly-validation.json) passed 84 takes
across 12 length/rate/velocity cases. With 32 contact subdivisions, maximum
relative RMSE against refined contact 256 was `0.0005283%` for mass-weighted
displacement, `0.01012%` for mass-weighted velocity and `0.01176%` for velocity at
the pickup observation point. Independent uniform midpoint 1024 agreed within
`0.0009772%` pickup velocity RMSE. Maximum candidate energy-balance residual was
`1.267e-11` of injected energy; maximum positive base-tick energy change was
`1.044e-15`. Repeating the final audit preserved all case data; timing is variable.
The new CI audit has not been run remotely during this local verification.

The native release timing probe took a median `28.6175 ms` to generate 250 ms
of one nine-coordinate voice with five strikes at 48 kHz, including the energy
ledger and excluding preparation. This cost is substantial; full polyphony,
pickup/filter processing and host/WASM timing remain unqualified. No calibrated
tonebar/support geometry or magnetic comparison is claimed. This milestone
produces no new instrument package and uses no Desktop controls or audio device.

## Physical-coordinate free transition and block timing

Date: 2026-09-05. [Prepared operator folding](MODAL-PERFORMANCE.md) removes the
four per-tick coordinate transforms and stores viscous work as a packed quadratic
form. Work still comes from an independent power integral. The contact model,
geometry, six-mode reduction and provisional profile are unchanged.

The new [84-take audit](../references/modal-assembly-folded-validation.json)
passes the existing gates. At 32 contact subdivisions the maximum pickup velocity
RMSE remains `0.01176%` against refined contact 256; the maximum relative energy
residual is `1.193e-11`. Two additional tests compare 48 normalized/physical
trajectories and check a lossless 20,000-step impulse response. The complete
workspace has 135 passing tests; strict Clippy, formatting and release WASM plugin
compilation pass. An existing timing report is rejected without changing its SHA256;
invalid timing-report extensions are rejected before file creation.

Identical native block workloads were measured before and after the change,
with 1/8/32/73 simultaneous voices at 48/192 kHz, five runs per case and preparation
excluded. The observed speed ratio ranges from 1.12x to 1.29x; native voice storage
falls from 31,824 to 21,712 bytes. The [timing protocol and complete table](MODAL-PERFORMANCE.md)
record the synthetic geometry, synchronized strikes, timer boundaries and limits.
Eight voices at 48 kHz still miss 26 of 470 measured block deadlines; 73 voices
miss every block at both rates. Average throughput alone would hide contact bursts.

This remains offline mechanics with no magnetic conversion, antialias filter,
host timing or measured physical calibration. The production instrument and
package version stay at 0.1.2. No audio device or Desktop controls were used.

## Bounded acceleration of modal contact

Date: 2026-09-05. The [refined contact root solver](MODAL-CONTACT-SOLVER.md)
uses up to eight safeguarded Newton evaluations and a 48-bisection fallback.
The potential, mechanical integration, contact subdivisions and energy ledger
are unchanged; the uniform-midpoint reference keeps its original bisection.
The worst scalar path is bounded but can cost more than the original method.

All 138 workspace tests pass, including three new tests for 882 scalar root
cases with forced fallback, cubic contact work, and twelve paired trajectories
through damping/restrikes with default and stiff/high-speed contact profiles.
Strict workspace Clippy, formatting and release WASM plugin compilation pass.
The [84-take audit](../references/modal-assembly-contact-validation.json) passes
the existing gates: contact-32 pickup velocity RMSE remains `0.01176%` against
refined contact 256, and the maximum relative energy residual is `1.193e-11`.
All 24 uniform-midpoint rows retain identical mechanical metrics from the
preceding audit; RMSE fields change only because their refined reference changed.

A fresh [before](../references/modal-timing-before-contact.json) run of the
retained `fe4cf96` release executable and an [after](../references/modal-timing-after-contact.json)
run of the rebuilt laboratory use the same block workload. Median render speed
improves by 1.70x to 2.03x across eight cases. At eight voices/48 kHz, p99 block
time falls from 6.514 to 1.183 times the deadline; 25 of 470 blocks still exceed
it. Full polyphony and the 192 kHz single-voice contact bursts remain over budget.
Voice storage stays at 21,712 bytes; the worst block-boundary energy residual
is `1.467e-11` of cumulative injected energy. These native observations exclude
pickup/filter/mixing, host and WASM timing, and are not realtime qualification.

This offline optimization does not add a material law or measured calibration.
The plugin and package remain at version 0.1.2, with no new listening release,
audio-device access or Desktop interaction.

## Dissipative modal hammer

Date: 2026-09-05. The [rate-dependent contact experiment](DISSIPATIVE-HAMMER.md)
adds a projected Hunt-Crossley-type loss coefficient, independent material heat
and separate diagnostics for nonadhesive unloading. The default beta is zero;
the existing elastic solver is called directly in that case. The new law has
no internal material relaxation state and is not a calibrated neoprene model.

All 141 workspace tests passed. Three additions check force/work/heat over 216
scalar cases with two solvers, convergence of isolated rigid-wall restitution
to a continuous analytic invariant, and coupled energy/state behavior through
restrikes, damping and reset. The coupled test was rerun after adding the final
limited-heat telemetry. Strict workspace Clippy, formatting and release WASM
plugin compilation passed.

The [material audit](../references/modal-hammer-validation.json) passes 108 takes
across 36 length/rate/velocity/beta cases. Contact-32 pickup-velocity RMSE remains
within `0.01176%` of refined contact 256 and relative energy residual within
`1.193e-11`. All prior metrics of 36 matching zero-loss takes are identical.
Nonzero beta dissipates 0.2391–7.6782% of injected energy at the contact in this
matrix. Sensitivity relative to the elastic trajectory can be large; it does
not establish acoustic accuracy. The nonadhesive limit activates in two reference
cases, with limited heat at most `1.024e-5` of injected energy; it is reported
separately and already included in contact/total dissipated energy.

Existing report paths are rejected without changing their bytes. CI includes the
new audit; its updated remote workflow was not executed locally. This remains
offline mechanics, with no pickup voltage, material calibration, listening
release or new realtime qualification. Plugin version 0.1.2 is unchanged, and
no audio device or Desktop controls were used.

## Isolated material memory and relaxation

Date: 2026-09-05. The [material coupon](HAMMER-MEMORY.md) introduces an internal
Maxwell deformation in parallel with linear/cubic equilibrium elasticity.
Prepared analytic ramp moments update memory, external work and independent
positive heat. This bilateral, displacement-controlled experiment is separate
from the contact, modal assembly and plugin; clamped recovery is not free recovery.

All 146 workspace tests passed. Five additions cover ramp composition and energy,
analytic relaxation and recovery of clamped reaction, sinusoidal dissipation,
32 parameter/timestep corner combinations with signed travel, invalid inputs and
reset. Strict workspace Clippy, formatting and release WASM plugin compilation
passed. No new dependencies were added.

The [coupon audit](../references/hammer-memory-validation.json) passed 36 cases
and 72 takes, with two rates, three relaxation times, three amplitudes and two
rest intervals. The maximum energy residual relative to absolute external work
was `1.574e-14`; the normalized analytic hold-extension error was `2.257e-14`;
endpoint-force RMSE between equivalent one/two-substep paths was `1.061e-14`.
Second-loading endpoint force was 73.31–99.97% of first-loading force after
short rest and 98.66–100% after long rest. These are sensitivity results for an
uncalibrated specimen, not observations of a real Rhodes hammer.

An existing report was rejected with its SHA256 preserved. CI includes the
new audit; its remote workflow was not run during this local validation.
The next gate is a nonadhesive coupling that preserves stored internal energy
through separation and unrestrained recovery. There is no new listening package,
audio-device or Desktop interaction, or realtime qualification; version 0.1.2
and the existing instrument mechanics remain unchanged.

## Stateful hammer impacts and free recovery

Date: 2026-09-05. The [two-mass hammer](MEMORY-HAMMER.md) couples the internal
material state to core/tip inertia and a fixed cubic penalty surface. External
contact is nonadhesive; bilateral material tension acts only inside the hammer.
Both masses retain their motion and memory through separation. An explicitly
accounted core impulse drives reimpact without resetting or repositioning.

The 151-test workspace suite passed, followed by a sixth focused hammer test
against the analytic damped relative mode of a free linear Maxwell hammer
(152 total tests). The other five additions cover common free flight, nested
pure-bisection reference agreement, energy/momentum balance, repeated impact and
free recovery, 16 mass/contact/relaxation corner cases, and invalid/atomic updates.
Strict workspace Clippy, formatting and release WASM plugin compilation passed.

The [final audit](../references/memory-hammer-validation.json) passes 24 cases /
48 takes over two output rates, three relaxation times, two launch speeds and
two tip masses at a fixed 4 g total mass. It uses at most 5 ns candidate steps
and a fourfold finer finite reference. Maximum relative energy residual is
`3.546e-11`; momentum residual is `2.268e-12`. Maximum normalized velocity RMSE
is 0.6996%; normalized cumulative surface-impulse error is 0.1224%; output-frame
mean-contact-force RMSE is 0.1238%. Every take demonstrates post-impulse contact
and positive heat during force-free recovery. These are numerical checks of a
provisional mechanical hypothesis, not comparisons with a measured Rhodes.

The [resolution pilot](../references/memory-hammer-resolution-pilot.json) records
the failed coarse attempts: 20/24 accuracy failures at 32/128 substeps and 8/24
at 256/1024. Their energy ledgers pass despite large trajectory differences.
The fine audit's extremely small intervals establish a laboratory reference,
not an audio-rate implementation or realtime qualification. Efficient free
motion, event handling and connection to the moving tine are the next gates.

The previous 72-take material coupon audit is byte-identical after the internal
reaction/tangent refactor. Rejection of an existing impact report preserves its SHA256.
CI includes the new impact audit; the remote workflow was not executed locally.
This milestone produces no new listening package, audio-device access or Desktop
interaction. Plugin mechanics and version 0.1.2 remain unchanged.

## Memory hammer coupled to the moving tine

Date: 2026-09-05. The [stateful modal connection](MEMORY-MODAL-COUPLING.md) adds
the full reciprocal structural compliance to the hammer's implicit contact
solve. Eleven inertial coordinates and the material memory persist through
separation, externally driven reimpact and damper changes. Signed port work
links the independent hammer and structural energy ledgers.

All 156 workspace tests passed. Four additions exercise coupled energy/port
balance and all-coordinate response, free recovery/reimpact, pure-bisection
agreement, invalid/atomic updates, disconnected motion and passive damper
switching, and total linear momentum with an ungrounded translation support.
Strict workspace Clippy, formatting and release WASM plugin compilation passed.

The [coupled audit](../references/memory-modal-validation.json) passes twelve
cases / 24 takes over three tine lengths, two launch speeds and two relaxation
times. Observations at 48 kHz cover 8 ms with a core impulse at 2 ms, damper on
at 4 ms and off at 6 ms. Candidate/reference subdivisions are 8336/16672, about
2.5/1.25 ns. Maximum global energy residual is `9.512e-10` relative to initial
energy plus absolute impulse work; structural and hammer work residuals are
`9.507e-10` and `3.513e-12`. Kinetic-metric velocity RMSE is at most 0.7693%,
pickup-port velocity RMSE 0.09629%, and output mean-contact-force RMSE 0.1406%.

The [coarse pilot](../references/memory-modal-resolution-pilot.json) records two
accuracy failures at 2084/8336 subdivisions despite passing energy checks.
Refinement preserves the physical coefficients and error gates. The very small
reference timestep is an offline numerical requirement for this experiment,
not a realtime implementation or proof of calibrated Rhodes behavior.

The existing 48-take fixed-wall report is byte-identical after generalizing the
surface port. Attempting to overwrite the new coupled report is rejected with
its SHA256 preserved. CI includes the new audit; remote CI was not run during
this validation. No new dependencies, listening package, audio-device access or
Desktop interaction were introduced. Plugin version 0.1.2 and its audible
mechanics remain unchanged. Accuracy per unit cost is the next numerical gate.

## Reusing structural energy in stateful diagnostics

Date: 2026-09-05. The [diagnostic optimization](MEMORY-MODAL-PERFORMANCE.md)
retains the structural energy already evaluated and validated by each successful
step. Returned probes reuse that value and the current hammer probe. Subsequent
observations use the cached structural value with the current hammer state.
Only one f64 is added to the voice, increasing native storage from 5944 to 5952
bytes. Coordinates, physical forces, heat integration and timesteps are unchanged.

All 157 workspace tests passed. The added test compares every diagnostic field
with a fresh reconstruction through 10,000 steps, positive and energy-removing
core impulses, damper engagement/release and invalid impulse rejection. Existing
tests retain atomic failure checks for excessive material travel. Strict Clippy,
formatting and release WASM plugin compilation passed.

The [24-take audit](../references/memory-modal-cache-validation.json) is
byte-identical to the pre-optimization report: maximum relative global energy
residual remains `9.512e-10`, kinetic-metric velocity RMSE 0.7693%, pickup-velocity
RMSE 0.09629%, and output mean-contact-force RMSE 0.1406%. The numerical reference
and all error gates are unchanged. Existing timing output paths are rejected
with their SHA256 preserved.

The new `memory-modal-timing` command measures the native single-voice tick and
its returned probe over the same 8 ms event sequence. It excludes construction,
pickup voltage, mixing and a host. Timing has no machine-dependent speed gate;
finite clocks and final energy are checked. The existing CI physical audit now
exercises the optimized path; remote CI was not executed locally. No new
dependencies, listening package, audio-device or Desktop interaction were added.
Plugin version 0.1.2 and its audible engine remain unchanged.

The [native comparison](../references/memory-modal-cache-timing-comparison.json)
pools two batches of three repetitions per version/profile, retaining all
observations. Median speedups are 1.202x, 1.191x, 1.196x and 1.023x across four
length/speed combinations. The last profile changes direction between batches,
so its small pooled gain is not a reliable margin. Final mechanical diagnostics
agree exactly across all timing runs. Optimized medians still cost 125–150
seconds per simulated second; reducing the required integration steps remains
the substantial performance gate.

## Certified longer intervals for hammer free recovery

Date: 2026-09-05. The [free-motion primitive](MEMORY-FREE-MOTION.md) combines a
conservative whole-interval tip-travel bound with RK4 step doubling for nonlinear
material recovery. Independent heat and work quadratures, half-step and actual
endpoint energy defects, stage domains and local state error are checked before
any state is committed. Rejected intervals leave the original hammer unchanged;
the original fine implicit tick remains available for contact and refinement.

All 161 workspace tests pass. Four additions cover rigid translation and retained
fixed-tick preparation, non-advancing clearance/invalid-input rejection, nonlinear
recovery versus fine implicit stepping with momentum and retained heat/memory,
and the analytic linear Maxwell relative mode. Strict Clippy, formatting and
release WASM plugin compilation pass. No dependencies were added.

The [48-take audit](../references/memory-free-validation.json) retains 24
fixed-wall profiles and the original 16 ms reimpact protocol. Candidate global
energy and material-work relative residuals are at most `1.656e-10` and
`1.007e-10`; relative momentum residual is `7.292e-14`. Maximum normalized
velocity RMSE is `3.631e-6`, mean-force RMSE `3.779e-7`, and cumulative impulse
error `2.298e-7`. All 24 uniform reference rows match their retained original
diagnostics exactly. Existing output rejection preserves the report's SHA256.

Accepted interval counts fall by 30.15x–62.68x, with free intervals up to 5.119
microseconds and unchanged approximately 1.25 ns contact ticks. Each free attempt
can cost twelve RHS evaluations, and rejected attempts add work; this count
reduction is not a measured runtime speedup. RK4 is error-controlled here, not
unconditionally energy preserving. The recorded global checks do not constitute
a proof for arbitrary profiles or durations.

CI includes the new audit; remote CI was not executed locally. This milestone
does not yet connect free intervals to the moving modal structure or plugin.
That connection needs a moving-surface certificate and a new coupled audit.
No listening package, audio-device access or Desktop interaction was introduced;
the audible instrument remains version 0.1.2 with its previous mechanics.

## Certified free recovery against the moving modal structure

Date: 2026-09-05. The [moving modal experiment](MEMORY-MODAL-FREE.md) uses the
full structural mass metric to exclude contact throughout each attempted free
interval. Prepared linear motion and independent structural heat are combined
with the checked nonlinear hammer evolution. Every rejection preserves all
physical coordinates and work/heat ledgers; fixed contact remains unchanged.

All 165 workspace tests pass, including four new moving-surface and transactional
tests. Strict Clippy, formatting and release WASM plugin compilation pass. The
[coupled audit](../references/memory-modal-free-validation.json) passes all 12
cases/24 takes with unchanged error gates and exact original uniform-reference
rows. Maximum candidate relative energy and structural/hammer port-work residuals
are `1.166e-10`, `1.178e-10` and `8.609e-11`. Kinetic velocity, pickup velocity and
output mean-force RMSE are at most 0.006220%, 0.000772% and 0.001313%, respectively.

Accepted interval counts fall by 4.16x–12.58x. The separate
[native timing](../references/memory-modal-free-timing.json), at the same fine
base resolution for both paths, observes median speedups of 3.66x–7.63x over
four profiles with three paired repetitions each. Preparation and reserved
operator heap payload are reported separately. The adaptive mechanics still
cost 31–70 seconds per simulated second; this is not realtime qualification.
Both new commands reject existing outputs without changing their SHA256.

The original 24-case/48-take fixed-wall free audit was also rerun after sharing
the certified hammer endpoint implementation. Its report remains byte-identical
to the retained baseline (SHA256
`6D6AC93E57E3DE6DB279BB55160701D636293396A4FA68BAD36785F903DB1B0C`).
All 24 new timing runs match their respective audited final mechanical states.

CI now includes the coupled free audit; remote CI was not executed locally.
No dependencies, audio-device access, Desktop interaction or listening package
were introduced. Physical calibration, longer/wider numerical qualification and
remaining contact cost are still open before integration into the audible plugin.

## Checked quadratic material roots within contact

Date: 2026-09-05. The [direct material solver](MEMORY-MATERIAL-SOLVE.md) replaces
repeated inner iterations with a stable quadratic root when deformation retains
its sign. The original residual and monotone bracket must accept the candidate;
crossings and rejected trials retain safeguarded Newton/bisection. The constitutive
law, contact response, heat integration and time resolution are unchanged.

All 167 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Two added tests cover 3,240 signed/history/profile/target trials, linear
limits, crossings and nonfinite inputs. Existing tests still compare coupled
motion with the independent bisection path and check energy and momentum.

The new [coupled](../references/memory-modal-material-solve-validation.json) and
[fixed-wall](../references/memory-material-solve-wall-validation.json) reports
pass all 36 cases/72 takes with unchanged gates. Coupled candidate relative global
energy residual is at most `1.166e-10`, kinetic velocity RMSE 0.006220%, pickup
velocity RMSE 0.000772% and mean-force RMSE 0.001312%. Retained modal event/final
checkpoints differ by at most `3.854e-8` in launch-speed-normalized kinetic metric;
root arithmetic changes are not described as byte-identical trajectories.

The retained [before](../references/memory-modal-material-solve-before-timing.json)
and [after](../references/memory-modal-material-solve-after-timing.json) native
batches show 1.208x–1.269x adaptive median speedups (17–21% less execution time)
over four profiles, with three repetitions per path/profile. No measurements
are discarded. Remaining adaptive cost is 26–59 seconds per simulated second;
this remains an offline prototype. Existing CI audits exercise the changed
solver, but remote CI was not executed locally. No dependencies or listening
package were introduced, and no audio device or Desktop was used.

## Adaptive compressed contact: accuracy passes, runtime remains experimental

Date: 2026-09-05. The [contact experiment](MEMORY-ADAPTIVE-CONTACT.md) combines
a whole-interval compression certificate with prepared midpoint/material
operators and step doubling. It commits two half steps only after state and
independent work/energy checks, preserving memory and restoring the original
fixed-step preparation. Rejections are transactional. Output mean contact force
includes both accepted reactions.

All 170 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Three additions check preparation/history, boundary/accuracy rejection,
analytic Maxwell extension, independent half-step agreement and the subsequent
base tick. The prior free-only modal report remains byte-identical, and all
12 uniform reference rows match the retained material-solver report exactly.

The [loose 1e-8 trial](../references/memory-modal-adaptive-preliminary-1e-8.json)
failed 4 of 12 trajectory comparisons despite good energy accounting. The
[1e-10 trial](../references/memory-modal-adaptive-intermediate-1e-10.json) passed
with only a small velocity-error margin (0.9854% against 1%). The selected
[1e-11 audit](../references/memory-modal-adaptive-validation.json) passes all
12 cases/24 takes with maximum kinetic velocity RMSE 0.03909%, pickup velocity
RMSE 0.004859% and mean-force RMSE 0.007111%. Relative combined energy and
structural/hammer work residuals are at most `9.118e-11`, `9.420e-11` and
`7.939e-11`. Global gates are unchanged; all experiments are retained.

The [timing control](../references/memory-modal-contact-control-timing.json) and
[adaptive timing](../references/memory-modal-adaptive-timing.json) show that the
strict path is slower in three of four profiles, with control/adaptive median
ratios of 0.927x, 0.534x, 1.352x and 0.824x. Fewer intervals do not compensate
consistently for three solves and local checks. Adaptive cost remains 41–51
seconds per simulated second. This is an explicitly selected offline experiment;
the existing contact path and audible plugin are unchanged. CI includes its
audit; remote CI and GUI/audio tests were not run.

## Scheduling strict contact estimates without relaxing acceptance

Date: 2026-09-05. The [scheduling experiment](MEMORY-CONTACT-SCHEDULING.md)
adds a separate controller with an eight-base-tick minimum trial and 64 fine
ticks between retries after a rejected minimum interval. Every deferred tick
still executes the original mechanics and responds to external events. Accepted
contact retains the complete three-solve estimator, `1e-11` tolerance,
compression bound, independent work/heat checks and two-half-step commit.
The estimator also reuses its already computed coarse/fine hammer probes.

All 172 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Two new tests check short frame remainders and deferred ticks through
impulse/damper changes. The [strict regression report](../references/memory-modal-estimator-reuse-control.json)
is byte-identical to the retained strict audit (SHA256
`D209D20B9F6D8E73581FB1157BF66B55A7F2C1B372A64A20C7EAF156DD2123AB`).

The [new audit](../references/memory-modal-economical-validation.json) passes
all 12 cases/24 takes with unchanged uniform-reference rows. Candidate maximum
kinetic velocity RMSE is 0.004014%, pickup velocity RMSE 0.000328% and mean-force
RMSE 0.001120%. Relative combined energy and structural/hammer work residuals
are at most `1.093e-10`, `1.096e-10` and `8.079e-11`. Contact attempts fall by
90.4%, from 3,292,012 to 315,855; physical fine ticks continue throughout.

Retained [new](../references/memory-modal-economical-timing.json),
[strict](../references/memory-modal-scheduling-strict-timing.json) and
[fixed-contact](../references/memory-modal-scheduling-fixed-timing.json) native
batches show improved medians in three of four profiles versus strict scheduling,
with strict/new ratios 1.083x, 1.829x, 0.878x and 1.318x. The soft 120 mm case
regresses about 14%; short sequential timings remain load-sensitive. New cost
is still 30–55 seconds per simulated second. The option remains experimental,
and neither default mechanics nor audible plugin behavior is replaced. CI
includes the new audit; remote CI and GUI/audio testing were not performed.

## Reusing the converged contact material reaction

Date: 2026-09-05. The [reaction reuse optimization](MEMORY-CONTACT-FORCE-REUSE.md)
keeps the last material reaction evaluated by the outer contact solver. It
reuses that force only at an exactly matching returned normal force, with
the original solve retained for an unevaluated fallback midpoint. The zero
normal branch reuses its existing open-contact solution. No integrator,
material parameter, tolerance, work check or controller policy changes.

All 172 workspace tests, strict Clippy, formatting and release WASM compilation
pass. The new economical and strict modal audits each pass 12 cases/24 takes;
the fixed-wall audit passes 24 cases/48 takes. All three complete JSON reports
are byte-identical to their retained pre-change references; hashes and links
are recorded in the optimization document.

Separate native before/after batches retain three repetitions per path/profile.
The economical controller's median elapsed times decrease by 2.9–7.3% in these
four cases; the uniform path decreases by 4.5–12.9%. All 24 before/after run
pairs preserve their reported final mechanical states and controller counters.
Sequential batches are sensitive to machine load and do not establish a
universal speedup. Economical execution still costs about 27–51 seconds per
simulated second. This remains offline numerical work, with calibration and
realtime contact integration still open. Remote CI and GUI/audio tests were
not run; existing CI already executes all three affected audit commands.

## Read-only contact resolution and error attribution

Date: 2026-09-05. The [contact resolution diagnostic](MEMORY-CONTACT-RESOLUTION.md)
adds read-only prepared trials and the seven squared-error terms of the existing
contact estimator. The commit path uses the same trial implementation, summation
order, physical checks and tolerance. A new test exercises both accepted and
rejected inspections without state changes and verifies subsequent committed
motion with both damper states.

All 173 workspace tests, strict Clippy, formatting and release WASM compilation
pass. The strict 12-case/24-take regression audit is byte-identical to the
pre-inspection reference. CLI discovery, invalid arguments and overwrite
protection pass. The new command is included in CI; remote CI and GUI/audio
tests were not performed.

The retained diagnostic covers 528 snapshots and 6,336 trials over 12 profiles.
For the 162 snapshots with a finite state-error rejection, the first failing
level is dominated by surface contact in 87, structural terms in 62, tip
velocity in 12 and equilibrium material in one. A largest term need not exceed
half of the metric. The 159 usable adjacent-level slopes range from 2.983 to
3.031, consistent with approximately cubic local error growth. These observations
support investigating a more accurate coupled contact integrator; they do not
justify changing physical coefficients or relaxing acceptance. This is an
offline diagnosis, with no runtime improvement or calibration claim.

## Fourth-order fixed-wall material-memory contact

Date: 2026-09-05. The [RK4 contact experiment](MEMORY-CONTACT-RK4.md) adds
continuously certified compressed intervals for the existing two-mass hammer
against a fixed wall. Heat, material work, normal impulse and surface-potential
work are integrated independently. Coarse/two-half-step comparisons retain a
`1e-11` state limit, `1e-13` local energy/work defect limit and independent
passivity/momentum checks. Rejections are atomic, base preparation is preserved,
and unresolved boundaries use original implicit ticks.

All 177 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Four new tests cover smooth fourth-order convergence, rejection and
preparation, selected parameter corners, and comparison with a 1 ns implicit
trajectory through an impulse. The 10 ns reference was too coarse for the
pointwise comparison; its step was refined without relaxing the error gate.
CLI help, invalid arguments and existing-report preservation pass. CI includes
the new audit; remote CI and GUI/audio tests were not performed.

The new 24-case/48-take audit passes all existing global gates. Maximum relative
combined energy, material work and momentum residuals are `1.683e-10`,
`1.012e-10` and `1.518e-14`. Velocity RMSE / launch speed is at most 0.04681%,
output mean-force RMSE 0.008282% and impulse error / initial momentum 0.008213%.
All cases retain free recovery heat and impulse-driven reimpact.

Accepted contact intervals replace 22.42 base ticks on average and reach
approximately 160 ns. Including free motion, uniform-to-accepted-interval ratios
are 74.8–669.7. These counts do not establish runtime improvement: each contact
attempt evaluates 12 RHS stages plus work/energy checks. The free-only control
report remains byte-identical to its previous reference, and all 24 uniform
reference reports match. Timing, moving modal coupling, physical calibration
and realtime qualification remain open; the audible plugin is unchanged.

## Reciprocal modal RK4 contact and native cost

Date: 2026-09-05. The [coupled RK4 experiment](MEMORY-MODAL-RK4.md) connects
the two-mass hammer and all nine structural coordinates in common stages.
Independent quadratures account for material heat/work, structural damping,
normal impulse, surface-potential work and moving-port work. Compression
certification, `1e-11` state tolerance, `1e-13` independent defect checks and
atomic rejection remain mandatory. Base ticks handle unresolved boundaries.

All 180 workspace tests, strict Clippy, formatting and release WASM compilation
pass. Three new tests verify smooth fourth-order convergence, reciprocal work
and base-step continuity with both damper states, and ungrounded total momentum
through an impulse. CLI help, invalid-option handling and overwrite protection
pass. CI includes the new audit; remote CI and GUI/audio testing were not run.

The 12-case/24-take audit passes the existing global gates. Maximum combined,
structural and hammer relative work/energy residuals are `8.851e-11`,
`1.470e-12` and `8.997e-11`. Velocity RMSE / launch speed reaches 0.2612%, pickup
velocity RMSE 0.03241% and mean-force RMSE 0.04748%. The velocity difference is
larger than the economical controller's 0.004014%, although below the 1% gate;
formal order is not a claim of superior accuracy or physical calibration.
The previous economical and fixed-wall RK4 reports remain byte-identical, and
all uniform-reference reports match between modal candidates.

Native timing retains three paired repetitions per path/profile, consumes
diagnostics and excludes preparation/audits. The four coupled medians are
0.044516, 0.052542, 0.064616 and 0.062828 seconds for 8 ms of motion, versus
0.281906, 0.201663, 0.384733 and 0.263726 seconds for the previous economical
binary. Ratios are 6.33x, 3.84x, 5.95x and 4.20x; all 48 timing runs match their
audited final mechanical states and controller counters. Short sequential
batches remain sensitive to machine load and do not define a universal speedup.
Execution still costs 5.6–8.1 seconds per simulated second. The new path remains
offline pending longer/finer reference validation, further cost reduction,
polyphonic host qualification and calibration.

## Coupled RK4 resolution and longer trajectories

Date: 2026-09-05. The [resolution study](MEMORY-MODAL-REFINEMENT.md) adds
independent laboratory contact/free caps and three implicit-reference grids.
Twelve original 8 ms profiles and four selected strong-strike 32 ms profiles
produce 96 takes and 112 pair comparisons. All pass existing global energy,
port-work and whole-record accuracy gates; no DSP tolerance or coefficient changes.

In the 8 ms matrix, maximum kinetic velocity RMSE / launch speed is 0.0001941%
for contact refinement, 0.0002981% for additional free refinement and 0.0001040%
for both caps versus the default. The old versus finest uniform reference
differs by 0.09511%; capped RK4 versus finest reference differs by 0.1661%.
This supports reference sensitivity as a substantial contributor to the
previous 0.2612% difference, without asserting an exact continuous solution.
The finest implicit step is about 1.000064 ns; unequal adjacent refinements
do not establish convergence order.

The largest local peak across all pairs is 1.9691%, between the coarser uniform
paths, despite passing the whole-record 1% gate. Two-millisecond windows expose
late growth in both extended 10 ms material-relaxation profiles. The finest
uniform path's structural-work residual reaches 7.327e-9 against a 1e-8 gate;
reference-ledger drift remains to be explained before extending it further.

All 182 workspace tests, strict Clippy, formatting and release WASM compilation
pass. The two new tests check diagnostic caps against explicit accepted steps,
including short remainders and events, and known metric normalization/silence
and window localization. CLI help, invalid arguments and overwrite protection
pass. The default RK4 control is byte-identical to its prior report and all
24 default/reference take reports match. CI includes the new study on both
native runners; remote CI and GUI/audio testing were not run. The audible
plugin remains unchanged, and no new performance or calibration claim is made.

## Incremental midpoint and structural roundoff drift

Date: 2026-09-05. The [incremental midpoint correction](MODAL-MIDPOINT-INCREMENTS.md)
forms small velocity increments directly, avoiding repeated multiplication by
a rounded near-identity matrix. It preserves the same implicit equations,
contact force/compliance and independent heat/work formulas. A dense-inertia
force-free test fails with the previous form and passes exactly with the new
form. A second test checks forced/damped midpoint momentum and energy/work
identities across step sizes and force signs.

The repeated 96-take resolution study passes all 112 pair comparisons. The
finest uniform reference's maximum 32 ms structural-work residual decreases
from 7.327e-9 to 2.654e-13, about 27,600 times smaller; its combined energy
residual decreases to 7.135e-12. No heat/work summation or acceptance tolerance
was changed. Default RK4 versus the old uniform grid still differs by up to
0.2595% in the 8 ms velocity metric, so energy closure must not be confused with
trajectory accuracy or instrument calibration.

The shared operator additionally passes 108 dissipative modal hammer takes and
24 takes each for adaptive contact, economical contact and certified free
motion: 276 audited takes in total. All 184 workspace tests, strict Clippy,
formatting and release WASM compilation pass. Uniform reference reports agree
between the three stateful audits and the corresponding resolution-study
paths. Existing reports remain preserved as pre-correction evidence; new reports
are not expected to match them byte for byte. Existing CI covers these checks;
remote CI, timing and GUI/audio tests were not run. The audible plugin engine
remains unchanged.

## Stateful modal tails through 128 ms

Date: 2026-09-05. The [tail study](MEMORY-MODAL-TAIL.md) extends four strong
75/120 mm profiles at 1/10 ms relaxation to 128 ms. Default RK4, capped RK4 and
the finest uniform incremental reference retain the original impulse/damper
protocol. All twelve takes, twelve whole-record comparisons and 48 separate
0–8, 8–32, 32–64 and 64–128 ms comparisons pass without tolerance changes.

Maximum default/reference section kinetic velocity RMSE / launch speed is
0.1665%, 0.07094%, 0.04776% and 0.02316%, respectively. Whole-record RMSE is at
most 0.05194%; a lower whole-record number does not certify every tail section.
A new synthetic regression proves that a failing final section cannot be hidden
by a passing whole-record average. The uniform reference's structural residual
stays below 5.381e-13, while hammer-work residual grows to 4.562e-10 and remains
below the existing 1e-8 gate. Its duration dependence remains to be investigated.

All 185 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CLI help, invalid/missing arguments and overwrite preservation pass. All
24 candidate/reference control reports and eight overlapping attack comparisons
match the previous study exactly; adaptive interval counts cover every base tick.
CI includes the new command on both native runners. Remote CI, timing and
GUI/audio testing were not run. The audible plugin and all DSP equations remain
unchanged; this is selected numerical qualification, not calibrated realism.

## Diagonal stiffness products and native tail cost

Date: 2026-09-05. The [stiffness cost study](MODAL-STIFFNESS-COST.md) adds
continuous section timing and an explicitly selected dense arithmetic reference.
Preparation detects exactly diagonal stiffness; nonzero off-diagonal entries
retain the dense path. Full mass and damping products, energy checks and
controller tolerances are unchanged. A new test verifies bitwise agreement
including signed zero and finite scale extremes, plus tiny off-diagonal coupling.

Separate before/after timing batches showed mixed results and remain retained.
A subsequent same-executable alternating comparison measures total 128 ms
medians of 0.090120, 0.164085, 0.084557 and 0.149495 seconds for the diagonal path,
versus 0.099934, 0.177809, 0.095282 and 0.164832 seconds for dense arithmetic.
Observed reductions are 7.7–11.3%; the first 8 ms still takes 51.68–55.76 ms,
so aggregate averages do not qualify realtime operation. No builds/tests/audits
ran concurrently with timing. These short native observations are not universal
speedups, confidence intervals or host/polyphonic deadline measurements.

All 72 timing runs match the earlier tail audit's final states and counters.
The repeated tail, RK4 and implicit adaptive audits (60 takes total) are
byte-identical to their earlier reports. All 186 workspace tests, strict Clippy,
formatting and release WASM compilation pass. CLI help, invalid options and
overwrite protection pass. The inline stateful voice grows by eight bytes;
prepared heap payloads are unchanged. CI checks paired arithmetic identity
without a timing threshold. Remote CI and GUI/audio testing were not run;
the audible plugin engine remains unchanged.

## Shared coupled contact trial calculations

Date: 2026-09-05. The [contact trial reuse study](MODAL-CONTACT-TRIAL-REUSE.md)
shares the identical initial derivative of the full and first half RK4 steps,
reducing a complete trial from twelve to eleven RHS evaluations. Local endpoint
energy reuse reduces six energy-vector evaluations to four and supplies the
accepted structural-energy cache. Every original stage, passivity and work
check remains active; no physical coefficient or acceptance tolerance changes.

All 187 workspace tests, strict Clippy, formatting and release WASM compilation
pass. A new regression compares accepted and rejected levels, impulses, both
damper states and subsequent fixed ticks against independent recomputation.
The repeated 128 ms tail and coupled RK4 audits cover 36 takes and are
byte-identical to the preceding reports. All 24 paired timing runs preserve
every section state and controller report; their final states and counters also
match the prior independent tail audit. CLI help, invalid options and overwrite
preservation pass. CI checks paired equivalence without a timing threshold.

Observed total median reductions are 6.19%, 2.63%, 5.68% and -0.19% across the
four profiles. The first 8 ms improves in every profile but still requires
48.32–58.08 ms of native execution for one assembly. The fourth total is
essentially unchanged, with more variable later sections. No builds, tests or
audits ran concurrently with timing; three repetitions do not establish a
universal improvement or host deadline qualification. The inline voice remains
6040 bytes and the prepared RK4 payload grows from 656 to 664 bytes. Remote CI
and GUI/audio tests were not run; the audible plugin engine is unchanged.

## Exact diagonal contact damping

Date: 2026-09-06. The [contact damping study](MODAL-CONTACT-DAMPING-COST.md)
classifies both damping matrices independently and uses a diagonal product only
when every off-diagonal entry is exactly zero. Engaged coupled damper matrices
retain dense arithmetic. Acceleration and the reciprocal heat quadrature use
the same damping vector; equations, coefficients and acceptance checks are
unchanged. Only coupled RK4 uses this optimization.

All 189 workspace tests, strict Clippy, formatting and release WASM compilation
pass. New regressions cover signed zero, finite scale extremes, tiny nonzero
coupling, both damper states, accepted/rejected trials and subsequent fixed
ticks. Both repeated audits (36 takes) are byte-identical to their earlier
reports. All 24 paired timing runs also match the preceding timing report's
states and counters at every section. CLI help, invalid options and overwrite
preservation pass; CI enforces paired identity without a speed threshold.

The measured 128 ms total medians decrease by 7.60%, 2.36%, 6.44% and 2.70%
across the four profiles. Attack execution still costs 45.09–49.52 ms for 8 ms
of one assembly's motion. No builds, tests or audits ran concurrently with the
timing batch. Memory remains 6040 inline bytes plus 102960/664 bytes of prepared
free/RK4 payloads. These observations do not qualify realtime operation or
physical realism. Remote CI and GUI/audio testing were not run; the audible
plugin engine remains unchanged.

## Late repeated-excitation qualification failure

Date: 2026-09-06. The [late reimpact study](MEMORY-MODAL-REIMPACT.md) adds
0.008 Ns core impulses at 32/80 ms and damper cycles at 40/56 and 96/112 ms,
retaining all earlier events and the full evolving physical/material state.
Twelve takes compare default RK4, capped RK4 and a uniform midpoint reference
over 128 ms. Each take must observe separation followed by contact in both
late epochs; each trajectory pair retains whole-record and four section gates.

The experiment fails all four profile qualifications. All twelve takes pass
their individual work/energy and reimpact gates, but default/capped RK4 fails
trajectory agreement in three profiles and every profile fails against the
uniform reference. Default/capped final-section kinetic velocity RMSE reaches
37.359% of initial launch speed; default/uniform reaches 60.204%. Energy closure
does not establish trajectory convergence. The cause remains unresolved, and
all thresholds are preserved. The retained report records failure and the CLI
exits unsuccessfully; this experiment is not presented as passing CI coverage.

All 191 workspace tests, strict Clippy, formatting and the release laboratory
build pass. Two new tests enforce the event schedule and independent separation
requirements for both late epochs. The original 24-take short audit is
byte-identical to its earlier report; all 24 unchanged pair/section comparisons
before 32 ms match the prior tail study exactly. CLI help, invalid options and
failed-report overwrite protection pass. No DSP/plugin changes, new timing,
remote CI or GUI/audio qualification are included. The next investigation must
separate inherited trajectory differences from local late-impact integration
by using an identical complete preimpact checkpoint.

## Late collisions from identical physical checkpoints

Date: 2026-09-06. The [checkpoint study](MEMORY-MODAL-CHECKPOINT.md) adds an
opaque physical checkpoint with validated timestep re-preparation. It preserves
all modal/hammer/material state, damper state and work/heat ledgers while
starting fresh integration banks and controllers. The same-step restart agrees
exactly with the original fixed trajectory through signed impulses and damper
changes; invalid restart steps leave the source and checkpoint unchanged.

Eight donor checkpoints immediately precede the first and second late impacts
in the four repeated-excitation profiles. Six 8 ms continuations independently
cap contact/free steps and compare two uniform midpoint grids. All 48 takes,
64 whole-record pairs and 256 separate 2 ms pairs pass existing gates. Maximum
default/finer-midpoint section kinetic velocity RMSE is 0.019647% of launch
speed; default/both-caps reaches 0.00001759%. The two midpoint grids still differ
by up to 0.011135%, so no exact-reference claim is made.

All donor impact timestamps match the earlier repeated-excitation report;
initial full probes match before/after each restart and bank preparation.
Maximum relative combined, structural and hammer ledger residuals are
7.105e-11, 2.660e-12 and 6.864e-11. The original 24-take short audit remains
byte-identical. All 193 workspace tests, strict Clippy, formatting, native lab
build and release WASM compilation pass. CLI help, invalid options and overwrite
preservation pass. CI now includes the passing local checkpoint command.

The earlier full-repetition qualification remains failed. Large divergence does
not recur in these short local continuations from equal physical history;
inherited differences and longer amplification remain unresolved. Next isolate
the approach from an identical checkpoint at the late impulse. No equations,
coefficients or tolerances changed, and no realtime timing, remote CI or
GUI/audio qualification was performed.

## Shared late-impulse approach and post-impact recovery

Date: 2026-09-06. The [shared-impulse approach study](MEMORY-MODAL-APPROACH.md)
restarts six integrators from identical donor states before the 32/80 ms core
impulses. Each continuation spans 48 ms with original absolute events. A common
force-free prefix excludes the earliest contact-containing frame across all
paths, so the approach comparison never includes an impact in only one path.

All 48 take-level energy/work/heat/force and contact/separation checks pass, as
do all eight common prefixes. Seven of eight full checkpoint cases pass. The
second 120 mm / 10 ms case fails five pairings with the finer midpoint grid,
including the uniform-grid pair; the retained report has failure status and
the command exits nonzero. Default/capped RK4 remains within all trajectory
gates. Default/finer-midpoint section kinetic RMSE reaches 10.228% of launch
speed, and the two midpoint grids reach 5.880%. Pickup and force gates pass.
The failing kinetic section starts at 92 ms, after contact force has ceased.

All checkpoint snapshots match their earlier donor event states; all 48 initial
states match their checkpoints. The original local 8 ms checkpoint report is
byte-identical after shared-helper changes. All 194 workspace tests, strict
Clippy, formatting and release lab compilation pass. CLI help, invalid options
and failed-report overwrite protection pass. A new regression covers contact
frame exclusion, missing/empty prefixes, invalid bounds and separation of
prefix trajectory gates from later failures. No DSP/plugin changes or new
timing, remote CI or GUI/audio qualification are included.

The new exploratory failure is not installed as a passing CI gate. The next
experiment must use identical post-separation state to distinguish inherited
collision differences from free hammer/material propagation error. No tolerance
or physical coefficient changes, and no full-repetition or reference-convergence
claim is made.

## Recovery from identical post-separation history

Date: 2026-09-06. The [recovery study](MEMORY-MODAL-RECOVERY.md) reproduces the
default branch restarted before the 80 ms impulse, then captures full state
at 96 ms before the damper event. Four profiles restart five integrators with
identical physical history and fresh controllers. Three uniform midpoint grids
and two free-step caps cover 32 ms through the original damper events.

All 20 continuations, 24 whole-record pairs and 384 separate 2 ms pairs pass.
Interval endpoints remain separated and observed mean forces remain zero.
Maximum default/finest-midpoint section kinetic RMSE is 0.000017105% of launch;
the 16672/20832 midpoint pair reaches 0.000007049%. Maximum relative combined,
structural and hammer ledger residuals are 7.713e-11, 2.976e-12 and 7.455e-11.
Separate hammer/structural metrics preserve the original combined gate.

All 196 workspace tests, strict Clippy, formatting and release lab compilation
pass. Help, invalid-option rejection and overwrite protection pass. Two new
regressions cover kinetic component separation and force/surface separation
with invalid heat rejection. No DSP/plugin equations or tolerances changed,
and no timing, remote CI or GUI/listening claim is added.

The earlier large error is not regenerated by this selected free recovery from
equal history. Its origin before the 96 ms checkpoint remains unresolved;
the earlier repeated-excitation and approach failure reports remain valid.
The next audible milestone is a selected-strike offline render with qualified
pickup sampling and gain/headroom, before realtime engine integration.

## First offline memory-modal audio

Date: 2026-09-06. [Four selected previews](MEMORY-MODAL-AUDIO.md) now connect
the new structure and memory hammer to the existing scalar magnetic conversion
and a common FIR. The 75 mm tine uses 0.2/0.4/0.8 m/s launch speeds; the 120 mm
tine uses 0.4 m/s. Every take lasts 1.5 seconds with damper engagement at 0.9 s.
All files retain fixed gain 0.084 and pass independent WAV inspection.

Primary 8192-tick and refined/capped 16384-tick trajectories pass energy/work
and every 2 ms mechanical comparison. Frozen 16/32/64x observation and the two
64x integration outputs pass whole/attack/body/release audio gates. Maximum
relative sampling and integration audio RMSE are 1.875e-7 and 1.347e-9. Maximum
relative ledger residual is 8.005e-11. The strongest output peaks at -2.452 dBFS.
A gain-10 negative control retains failure status and creates no WAV.

All 199 workspace tests, strict Clippy, formatting and the release laboratory
build pass. CLI regressions protect both outputs and reject invalid/bounded
options. New audio tests reject silence/nonfinite samples and detect attack
error hidden by whole-file averaging. The standalone CI workflow includes a
short render; remote CI was not run. The tracked summary and full medium receipt
are linked in the preview document. No new listening, realism, tuning, repetition
or realtime qualification is claimed, and DSP equations/gates remain unchanged.

## Controlled hammer alternatives

Date: 2026-09-06. The [equal-launch comparison](CONTROLLED-HAMMERS.md) qualifies
elastic, rate-dependent and two-mass memory hammers on the same 75 mm modal
structure, pickup and gain at 0.2/0.4/0.8 m/s. All nine 1.5-second WAVs pass
independent inspection. Single-mass 256/512x midpoint paths pass their energy,
heat, escape-momentum and separate 2 ms mechanical/contact refinement checks.
The memory paths retain their existing energy/work and mechanical gates.
Every candidate passes frozen 16/32/64x pickup and 64x integration audio checks.

Maximum relative audio integration RMSE is 2.953e-5; sampling RMSE is 1.880e-7.
Maximum single-mass balance residual is 2.807e-11 of launch energy. Memory
energy/work residuals remain below 8.005e-11. Peak output is -1.734 dBFS.
The memory renderer's new contact diagnostics preserve all three previous WAVs
byte for byte. Its 13/22/24 force-positive episodes are reported separately from
the first separation; both integration paths reproduce those episode counts.
The simple models remove the hammer at separation, a documented action-boundary
difference. These are numerical results for provisional coefficients, not a
comparison of fitted physical fidelity or repeated-key performance.

All 201 workspace tests, strict Clippy, formatting and release laboratory build
pass. Spectral Parseval/gain checks and protection of all four CLI destinations
are covered. A short pilot passes; a gain-10 negative control retains failure
and creates none of the three WAVs. CI includes the new short comparison, but
remote CI and host/listening tests were not run. Tracked summary/full medium
receipts and local audio links are in the comparison document. No DSP equations,
plugin engine, calibrated parameters or repeated-excitation status changed.

## Frequency target preparation

Date: 2026-09-06. The [G3 reference preparation](PITCH-REFERENCE.md) verifies
the five original source blobs, rejects ambiguous/unstable observations and
fits a frequency-only target from training layers 1/3/5. The target is
196.386147 Hz; validation layers 2/4 differ by +0.0999/-0.3584 cents. Maximum
within-take span across three nonoverlapping 512 ms windows is 0.929 cents.
This declared split reuses previously inspected recordings and is not blind
physical validation. Unknown capture gain and processing prevent an inferred
hammer-speed, gain or natural-decay fit.

The provisional 75 mm memory assembly measures 169.570540 Hz, -254.169 cents
from that target. Its new 2-second diagnostic WAV holds until 1.85 seconds,
passes the existing numerical/audio qualification and independent WAV inspection,
and supplies all three observations before damping. The broad-band anchor avoids
the earlier hinted analyzer selecting a weak component near the expected note.
An accepted output peak is still not an identified structural eigenmode.

All 103 affected analysis/laboratory tests, workspace-wide strict Clippy,
formatting and release laboratory build pass. Five new regressions cover broad
peak selection/gain invariance, ambiguity/silence/short inputs, temporal drift,
training/validation isolation and CLI blob verification/overwrite protection.
DSP equations and plugin parameters are unchanged. No remote CI or listening
claim is made. Cleanup after verification removed 1565 regenerable debug-cache
files (about 1.2 GiB); release tools, source audio and validation artifacts remain.

## RF-73 rename

Date: 2026-09-06. [Naming and compatibility](RENAMING.md) records the new local
folder, GitHub repository, Cargo names, UI labels and package filename. Existing
host/program identity and serialized state remain compatible. All 206 workspace
tests, strict Clippy, formatting, native release workspace and plugin/UI WASM
builds pass. RackForge CLI validation and smoke pass for `RF-73 Research`.
The 237215-byte `dist/RF-73-0.1.2.rfplugin` archive is retained; no Desktop launch
or listening claim is made. Cargo cleanup removed about 736.2 MiB of build cache
after verification, preserving the package and all reference/experiment assets.

## Coupled spring-position tuning

Date: 2026-09-06. [Spring-position tuning](SPRING-TUNING.md) adds an undamped
spectrum of the shared nine-coordinate mass/stiffness operators and a bounded
one-parameter offline fit. The provisional 70 mm blank stays fixed while a
0.1 g point-mass center moves from 59.5 to 55.740822 mm from the root. The
float32 output component measures 196.378585 Hz, -0.066667 cent from the frozen
processed G3 reference. The second/first and third/first fixed-root ratios
change by -0.532% and -4.508%; pitch-only acceptance cannot establish timbre.

The before/after pair independently passes all original mechanical ledgers,
2 ms refinement checks, section-wise 16/32/64x sampling and -1 dBFS headroom
limits. Both 48 kHz mono float WAVs contain 96000 finite samples. Common gain
and hammer speed are preserved; only spring position differs within the pair.
The [compact receipt](../references/g3-spring-tuning-validation.json) retains
modal tables, checks and artifact hashes. The complete 1.2 MB local report is
`renders/g3-spring-tuned.json`. No listening, hardware-geometry identification,
whole-keyboard calibration, plugin update or remote CI claim is made.

All 214 workspace tests, strict Clippy and formatting pass; the release lab
build succeeds. Eight new regressions exercise structural compliance, analytic
free motion, rigid modes, spring-dependent ratios, spatial branch tracking,
frozen-target fitting, reference rejection and protection of all CLI outputs.
Physical-field overlap at the reference geometry reconstructs mass
orthonormality, avoiding comparisons in a changing modal coordinate basis.
The prior 75 mm preview re-renders byte-identically after the audio refactor
(SHA-256 `0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`).

After verification, Cargo removed 651 regenerable debug files (532.1 MiB).
The temporary baseline regression WAV/report were removed after the byte check.
About 55 MiB of release build artifacts remain; reference recordings, the
new listening pair, validation reports and packaged plugin are preserved.

## Post-tuning spectral evidence

Date: 2026-09-06. [Post-tuning modal observations](POST-TUNING-MODES.md) introduces
`observe-modes`: exact pinned-byte decoding, native-rate 32/128 ms attack and
512 ms body windows, independently detected peaks and explicit harmonic,
neighbor, capacity and observable-band limitations. All seven fixed inputs
were analyzed without new audio or parameter fitting. The retained receipt
preserves every accepted peak, all proposed-mode associations and report hashes.

The tuned model has a prominent 1361.92 Hz short-window component. Reference
layers 3/4/5 contain separated candidates at 1425.06/1425.27/1425.50 Hz in their
128 ms windows; no structural identity is asserted. Three reference windows
hit detector capacity and remain inconclusive. The initial layer-1 window has
no accepted peaks, which does not imply absent spectral content. This evidence
motivates a geometry sensitivity study at fixed pitch, not a timbre acceptance.

All 111 affected release tests, strict workspace Clippy and formatting pass.
Four new regressions bring the workspace total to 218 tests; unaffected DSP,
plugin and UI tests were not rerun for this analysis-only change. No Desktop,
listening, remote CI or measured-geometry qualification is claimed.

Cleanup after verification removed 342 regenerable debug files (about 101.4 MiB)
and the temporary exploratory analysis. The release tools/cache remain bounded
at about 67 MiB. Existing audio, packaged plugin and numeric receipts are
preserved; the detailed seven-report observation set totals 325178 bytes.

## Fixed-pitch geometry sensitivity

Date: 2026-09-06. [Fixed-pitch geometry](FIXED-PITCH-GEOMETRY.md) records nine
uniform-tine/point-mass cases at 68/70/72 mm and 0.08/0.10/0.12 g. Eight regain
G3 within 0.0001 cent; 68 mm/0.08 g remains unreachable at the outward limit.
Branch checks pass using the pairwise average actual physical inertia.
Using initial spring inertia at remote positions had caused false ambiguity;
all-mode orthogonality at 0.50/0.75/0.95 L validates the local metric without
relaxing the 0.98/0.05 gates. The preserved baseline's fitted center, nine
coupled frequencies and hammer/pickup weights match exactly.

Solved rank-5 frequencies span 1182.461..1390.271 Hz at constant fundamental,
with materially different linear port residues. No cell reaches or is fitted
to the exploratory 1425 Hz family. All trials, unreachable status and explicit
shared parameters are retained in the 145388-byte numerical receipt.

All 63 laboratory unit/CLI tests, strict workspace Clippy and formatting pass.
Four new regressions bring the workspace to 222 tests; unaffected DSP,
analysis, plugin and UI tests were not rerun for this laboratory-only change.
No nonlinear audio, listening, measured-geometry or remote-CI claim is made.

After verification, Cargo removed 342 regenerable debug files (101.4 MiB).
Release artifacts remain at about 67 MiB. No new audio files were generated;
all existing WAVs, packaged plugin and numeric receipts are preserved.

## Finite tuning-mass span

Date: 2026-09-06. [Finite spring span](FINITE-SPRING-SPAN.md) adds consistent
uniform co-moving inertia over an axial interval, retaining the zero-width
point baseline. All four 0/2/4/6 mm probes on the 70 mm, 0.1 g geometry regain
the frozen G3 target within 0.0001 cent. At 6 mm, coupled rank 5 moves from
1361.884 to 1358.852 Hz, away from the exploratory reference family. The retained
receipt includes all trials; no width, timbre or measured geometry is selected.

All 227 workspace release tests, strict Clippy and formatting pass. Five new
regressions cover quadrature moments and bounds, uniform-density equivalence,
the point limit and mesh convergence, coupled contact/release energy, and
fixed-pitch upper-mode sensitivity. Existing local-inertia orthogonality and
exclusive CLI output tests now cover finite spans as well. This is an inertia
approximation without coil elasticity, slip or intrinsic rotary inertia.

The default point-mass 75 mm memory-hammer preview passes its independent
render checks and reproduces the preserved WAV byte-for-byte (SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`).
The finite-span contact regression uses the single-mass hammer; finite-span
memory-hammer audio, listening and realtime integration remain unqualified.

Cleanup removed 342 regenerable debug files (101.4 MiB) and the duplicate
regression WAV/report after the hash check. Release artifacts remain at about
151 MiB after the full workspace test run. The new numerical receipt occupies
74596 bytes; existing audio, reference recordings and packaged plugin remain.

## Linear tine-section variation

Date: 2026-09-06. [Tine taper](TINE-TAPER.md) adds a linear root-to-tip diameter
with consistent variable-section mass and bending stiffness. Five-point element
quadrature integrates degree-eight Hermite mass products. Root mass moments and
physical-field tracking use that same section; the uniform branch stays exact.

All four 0.90/0.95/1/1.05 ratios on a 70 mm blank with 1.5 mm root diameter and
0.1 g point mass retune within 0.0001 cent of frozen G3. Coupled rank 5 spans
1301.832..1373.127 Hz; none reaches the exploratory 1425 Hz family. The 74853-byte
receipt preserves all trials and shared parameters. No geometry is selected.

All 230 workspace release tests, strict Clippy and formatting pass. Three new
tests cover analytic frustum inertia/static flexibility, convergence/scaling
and geometry validation, and retuned modal sensitivity. Existing nine-mode
orthogonality, contact/damper energy and CLI output-protection tests also cover
the new section study. These checks qualify the equations for the tested
profiles, not a real manufactured part or the whole accepted domain.

The uniform study cell retains its fitted spring center, all nine frequencies
and hammer/pickup weights exactly. The 75 mm memory-hammer baseline re-renders
byte-identically, SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`.
No tapered-memory-hammer audio, listening, plugin or remote-CI result is claimed.

Cleanup removed 342 regenerable debug files (101.4 MiB) and the temporary
duplicate regression WAV/report after the hash check. Release artifacts remain
at about 151 MiB. Source recordings, existing listening WAVs, numerical receipts
and the packaged plugin are preserved.

## Localized root-section transition

Date: 2026-09-06. [Localized transition](TINE-TRANSITION.md) ends a linear taper
at a bounded free-length fraction and continues with a cylinder. Integration
splits at the corner even when it falls inside an element. Prepared root moments,
modal coupling and branch-tracking inertia all use the same section.

All four endpoints 1/0.5/0.25/0.137 L at fixed 0.95 tip/root ratio retune to G3
within 0.0001 cent. Coupled rank 5 spans 1281.005..1336.386 Hz and remains below
the exploratory 1425 Hz family. All trials are retained in the 77887-byte receipt.
The full-length case preserves its prior spring center, nine frequencies and
hammer/pickup weights exactly. No section or timbre is selected.

All 233 workspace release tests, strict Clippy and formatting pass. Three new
regressions cover analytic segment moments and independent static flexibility,
off-grid convergence/continuity and the uniform limit, and fixed-pitch modal
sensitivity. Existing orthogonality and contact/damper energy checks now include
three localized profiles. CLI output protection also covers the new command.

The default 75 mm memory-hammer preview passes its render checks and remains
byte-identical to the preserved WAV, SHA-256
`0dbd0cd29a7929fe16f9eccfdc05b6b23554d406a4fa0add5f73408ec123be2e`.
No localized-transition memory-hammer audio or listening result is claimed.
Cleanup removed 342 regenerable debug files (101.4 MiB) and the duplicate
regression WAV/report after the hash check. Release artifacts remain near
151 MiB; source recordings, listening WAVs, receipts and the package are preserved.

## Cross-take spectral recurrence

Date: 2026-09-06. [Spectral families](SPECTRAL-FAMILIES.md) adds a bounded,
strict-manifest command using all independently accepted peaks from pinned
native-rate WAVs. It rejects chained/duplicate components as ambiguous and
requires three distinct uncapped, unambiguous detections separated from integer
harmonics. Same-take low-order frequency relations retain their residuals.

Four attack-128 components recur near 886/1425/1620/7108 Hz. In layers 3/4/5,
the 1425/1620 pair differs by approximately one fundamental, within the declared
15.624 Hz tolerance. This is a possible sideband relation, not proof of mixing
or two independent structural modes. Three low-frequency body components also
recur; processing/background remain possible origins. All capped, missing,
harmonic and ambiguous cases remain in the 299317-byte report.

All 123 affected analysis/laboratory release tests, strict workspace Clippy
and formatting pass. Four unit tests and one CLI regression bring the workspace
total to 238; unchanged DSP/plugin/UI suites were not rerun. A synthetic test
initially expected a peak 7 Hz from H16 to be isolated at 128 ms; correcting
that expectation verifies the existing separation limit without changing any
threshold. All 15 source windows retain their prior peak counts and frequencies
within 1e-9 Hz. No DSP equation, plugin setting or audio asset was changed.

Cleanup removed 342 regenerable debug files (101.4 MiB). Release artifacts
remain near 151 MiB. No new WAVs or source downloads were produced; existing
audio, numeric receipts and the packaged plugin are preserved. Listening,
cross-note modal identity and natural-decay calibration remain open.

## Neighboring-note spectral hypotheses

Date: 2026-09-06. [D3/G3/B3 families](REGISTER-FAMILIES.md) selects the nearest
recorded anchors on both sides of G3 before inspecting their audio and retains
all five layers per note. Ten original mono WAVs total 13955116 bytes; Git blob
IDs and byte counts match the pinned author revision, and SHA-256 hashes and
upstream license/README are retained. No resampling or source redistribution in
the plugin occurs.

All 15 per-take pitch anchors qualify. The existing family detector finds
fundamental-offset chains in both new notes. G3 attack-128 groups have no
correspondences under either fixed-Hz or constant-ratio predictions, while some
body components match fixed frequencies and others match scaled predictions.
These competing descriptions do not identify mechanical modes or prove hum.
The 1027206-byte report retains all source observations, limits and matches;
no geometry, damping, pickup parameter or audible baseline changed.

All 127 affected analysis/laboratory release tests, strict workspace Clippy and
formatting pass. Four new regressions bring the workspace total to 242;
unchanged DSP/plugin/UI tests were not rerun. Tests cover hypothesis separation,
missing/shared candidates, withheld unqualified groups, strict manifests and
CLI output/source-byte protection. All five G3 anchors and 239 previously
accepted peaks remain exactly equal. No listening, independent-instrument
validation or remote-CI result is claimed.

Cleanup removed 342 regenerable debug files (101.5 MiB). Release artifacts
remain near 152 MiB. New reference audio occupies about 13.3 MiB; no synthetic
WAVs were generated. Existing listening files, source audio, numeric receipts
and the packaged plugin are preserved.

## Controlled two-mode pickup mixing

Date: 2026-09-06. [Pickup mixing](PICKUP-MIXING.md) adds analytic two-mode
motion with combined nonlinear, independently transduced and equilibrium-linear
controls for both existing pickup laws. A coherent FFT retains phase, DC and
every bin through 20159.1796875 Hz across seven sampling densities. The
73064-byte receipt retains all eight fixed probes and finite-reference residuals.

Both laws produce fundamental-spaced cross components absent in the controls.
For production, the upper first sideband rises from -44.75 dB relative to the
output fundamental at 0.005 mm primary motion to -10.60 dB at 0.25 mm. These
are prescribed amplitudes, not calibrated source velocities. Centered geometry
has zero linear response but nonzero nonlinear output. Neither observation
identifies the recorded families or justifies a structural parameter fit.

All eight cases pass 32x versus 64x full-band NRMSE below `1e-8` for all three
paths. The numerical-only stress case exposes base-rate residuals of
`2.43895e-4` (production) and `1.48946e-3` (point-pole); both fall below the
reporting floor at 2x. This is finite ideal-band refinement, not a production
decimator or complete instrument antialias qualification.

All 133 affected analysis/laboratory release tests, strict workspace Clippy and
formatting pass. Six new tests bring the workspace total to 248; unchanged
DSP/plugin/UI suites were not rerun. New checks cover an independent quadratic
mixing expansion including phase, zero-parent and centered controls, stress
refinement, FFT normalization/invalid inputs, and CLI output protection. The
eight-case command also completes successfully. No DSP equation, preset,
mechanical geometry or existing audio file changed. No listening result is
claimed. Independent modal envelopes and sideband decay/phase remain next.

Cleanup removed 342 regenerable debug files (101.5 MiB); release artifacts
occupy 151.8 MiB. No source downloads or WAVs were produced. Existing source
recordings, listening files, numeric receipts and the packaged plugin remain.

## Phase-resolved pickup decay

Date: 2026-09-06. [Pickup decay](PICKUP-DECAY.md) prescribes two independent
exponential modal envelopes and exact displacement derivatives through both
unchanged magnetic laws. Phase-torus quadrature isolates sum/difference
coefficients at six ages; a free log-amplitude regression estimates decay.
This is not an estimator for recorded audio or a temporal FFT of decaying motion.
The 143454-byte receipt keeps eight cases, complex controls, phase errors and
finite-grid refinement.

Weak unequal probes recover the predicted 5.8 /s amplitude decay as 5.799998 /s
(production) and 5.800008 /s (point-pole). Equal and zero-loss controls recover
4 and 0 /s. Larger probes fit 5.793595 and 5.816038 /s, with nonzero departure
from one exponential. Omitting envelope velocity introduces the predicted phase
error. These observations constrain how pickup mixing should be interpreted;
they do not identify source modes or calibrate mechanical losses.

All eight probes qualify: 64/128 grid interaction differences stay below
`1.5e-14` relative and null controls below `1.7e-12`. Weak quadratic complex
errors and absolute fitted rate errors satisfy the declared `1e-3` bounds.
Large-amplitude approximation errors are retained without weak-motion gates.
The refinement covers two phase coefficients, not a full temporal spectrum or
the production decimator.

All 138 affected analysis/laboratory release tests, strict workspace Clippy and
formatting pass. Five new tests bring the workspace total to 253; unchanged
DSP/plugin/UI suites were not rerun. Tests cover an independent displacement
derivative, analytic complex mixing, an intentionally omitted derivative,
free rate estimation and invalid observations, phase-grid refinement, removal
of one parent, and CLI report/output protection. No source download, WAV,
DSP equation or preset change was produced; no listening result is claimed.
Next qualify a temporal envelope estimator before applying decay constraints
to source recordings.

Cleanup removed 342 regenerable debug files (101.5 MiB). Release artifacts
remain at 151.8 MiB; source audio, listening WAVs, numeric receipts and the
packaged plugin are preserved.

## Temporal component-envelope qualification

Date: 2026-09-06. [Component envelopes](COMPONENT-ENVELOPE.md) adds bounded
fixed-carrier Hann demodulation of native WAV samples. Reports retain complex
coefficients, local guard background, free amplitude/phase fits and explicit
rejections. Observation bounds and known neighbors come from the caller;
qualification does not establish natural sustain or mechanical modal identity.

All 18 expectations pass for nine prescribed temporal mixtures measured at
128/256 ms. Accepted cases include clean exponential decay, deterministic noise,
a 40 Hz neighbor and a stronger-parent sideband mixture. Maximum accepted
absolute rate error is 0.000635 /s, below the declared 0.05 /s threshold.
The 4 Hz known neighbor, insufficient local margin, stationary amplitude,
changing decay and 2 Hz carrier mismatch are rejected for their declared reasons.
The 242929-byte receipt retains every provisional fit and rejected observation.

An additional test demonstrates the estimator's identifiability limit: an
undeclared component 0.05 Hz away can pass, and is rejected when declared.
This limitation remains explicit in every report. Finite windows bias absolute
amplitude/phase and overlapping windows are correlated. No confidence interval,
physical damping fit or T60 extrapolation is claimed.

All 145 affected analysis/laboratory release tests pass. Six new analysis tests
and one CLI regression bring the workspace total to 260; unchanged DSP/plugin/UI
suites were not rerun. Coverage includes native 44.1/48/96 kHz rates, carrier
offset, gain/phase preservation, silence, full scale, short/invalid/bounded input,
the complete study, PCM16 roundtrip, source bytes and output protection.
Strict workspace Clippy and formatting pass. A test fixture initially attempted
to use a transitive WAV dependency; it now encodes its independent PCM16 header
directly, without adding a dependency. CLI argument chunking was updated for
strict Clippy and its end-to-end regression rechecked.

No source recording, physical parameter, preset or user-facing audio was changed.
Synthetic study samples live only in memory; the temporary CLI fixture is removed
by the test harness. No listening or remote-CI result is claimed. Next apply
conditional observations to a bounded, byte-verified source-family pilot without
relaxing rejection thresholds or assuming unknown release boundaries.

Cleanup removed 342 regenerable debug files (101.5 MiB). Release artifacts
occupy 152.2 MiB. Existing source audio, listening WAVs, numeric receipts and
the packaged plugin are preserved; this stage adds no permanent WAV files.

## Pinned G3 source envelopes

Date: 2026-09-06. [Source envelopes](SOURCE-ENVELOPES.md) fixes a five-take G3
pilot before temporal observation, using the prior pitch anchors and unique
attack-128 peaks in 1410..1440/1605..1635 Hz bands. The prior register receipt
and every WAV are Git-blob verified on exactly the bytes read. Native 44.1 kHz,
file offsets 0.1..1.5 s and unchanged 128/256 ms envelope gates are retained.

The report contains 15 component slots, four missing selections and 22 envelope
measurements. Four individual measurements qualify; only the layer-3 fundamental
passes both windows and prior-selection checks, with descriptive mean amplitude
decay 0.329510 /s. Layers 4/5 miss the existing minimum drop in the longer window.
All twelve higher-family measurements reject for margin, amplitude, slope and
phase instability; nine also fail frequency offset. All five mixing-rate
relations are withheld. No rejected provisional rate becomes a model parameter.

Post-observation inspection shows initial contiguous above-margin support for
the upper family shorter than the existing 0.4 s fit requirement in every
take/window. This is a detectability diagnostic, not a fitted physical lifetime
or a selected shorter interval. The fixed-interval receipt remains unchanged;
short-transient estimation needs independent synthetic validation with nearby
harmonics and uncertain carriers before further source fitting.

All 150 affected analysis/laboratory release tests, strict workspace Clippy and
formatting pass. Four new unit tests and one CLI regression bring the workspace
total to 265; unchanged DSP/plugin/UI suites were not rerun. Tests cover strict
manifests, identity matching, missing/ambiguous/capped selection, cross-window
withholding/agreement, retained missing components, evidence/audio-byte mismatch
and output preservation. The 417084-byte receipt reproduces byte-for-byte with
SHA-256 `fbf55a24583080668c55dae63c9cd8733cf66c1ab7f01d326fae0b50b9dccf4d`.
No source download, production DSP change, listening or natural-loss fit is
claimed. Temporary CLI renders are removed by the test harness.

Cleanup removed 342 regenerable debug files (101.5 MiB) and the duplicate
reproduction receipt after its hash check. Release artifacts occupy 152.6 MiB.
Source recordings, listening WAVs, retained numeric receipts and the packaged
plugin are preserved; no permanent WAV was added.

## Joint short-transient envelope qualification

Date: 2026-09-06. [Short envelopes](SHORT-ENVELOPE.md) fits quadratic local
complex envelopes for one target and up to two nuisance carriers, using
reorthogonalized QR. Fixed 32/64 ms synthetic observations recover an 8 /s target
beside a ten-times-stronger neighbor, with noise and with +2 Hz detuning. The
method has explicit conditioning, regression-margin, support, phase and rate
stability gates. The preceding Hann estimator and source-pilot gates are unchanged.

All 18 final study expectations pass. Maximum accepted rate error is
0.016548 /s and carrier error is 0.001603 Hz, below declared 0.1 /s and 0.02 Hz
limits. Near-coincident carriers withhold coefficients; omitted-neighbor,
low-margin, changing-decay and stationary probes reject. This is conditional
synthetic coverage, not calibration of source recordings or mechanical losses.

Both receipts are retained: the initial 129947-byte report had 17/18 expected
outcomes because an omitted neighbor was expected to reject specifically on
carrier offset at 32 ms. It already rejected on margin and multiple instability
gates. The corrected expectation requires margin rejection at both widths;
the final receipt is 129911 bytes. All numerical measurements are unchanged.
No waveform or acceptance threshold was adjusted to obtain the final result.

All 270 workspace release tests pass, including DSP, plugin and UI. Four new
analysis tests and one CLI regression cover known polynomial coefficients,
three-carrier QR orthogonality, independently checked noise propagation, native
rates/phase/offset, ill-conditioning, invalid/support/silent inputs, full study
and source/output preservation. Strict workspace Clippy and formatting pass.
Clippy's allocation check prompted explicit capacity reservation for all six
carrier columns; the focused estimator tests were rechecked afterward.

The sibling RackForge workspace is now 0.1.15; Cargo.lock was synchronized
offline for only `rackforge-plugin-sdk` and `rackforge-program-api`. No remote
package changed. Full workspace verification covers this local dependency
transition, but no packaged release, host audition or listening result is claimed.
Next freeze short source intervals and nuisance harmonics before applying this
estimator to the recordings. No physical equation, preset or source WAV changed.

Cleanup removed 342 regenerable debug files (101.5 MiB). Release artifacts
occupy 160.8 MiB after full compatibility testing. Both numeric expectation
receipts are retained; temporary CLI WAVs are removed by the test harness.
Existing source recordings, listening WAVs and the packaged plugin are preserved.
