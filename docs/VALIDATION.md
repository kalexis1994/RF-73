# Initial validation report

Historical results are retained below. The latest local results are in the component comparison section at the end.

Date: 2026-09-04. RF-Rhodes 0.1.0 research prototype.

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

The initial commit passed hosted CI on Windows and Linux: [run 33915204770](https://github.com/kalexis1994/RF-Rhodes/actions/runs/33915204770).

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
cargo run --locked --release -p rf-rhodes-lab -- render --output renders/tracking-a3.wav --note 57 --velocity 0.9 --seconds 3 --hold 2.5
cargo run --locked --release -p rf-rhodes-lab -- analyze renders/tracking-a3.wav --output renders/tracking-a3-analysis.json --note 57 --sustain-end 2.4
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
