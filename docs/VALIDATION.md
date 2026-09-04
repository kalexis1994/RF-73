# Initial validation report

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

Real-instrument timbral fidelity, measured hammer material response, high-order assembly modes, spectral convergence/aliasing bounds, browser execution, Android/Pi timing, actual audio-device latency, long-duration soak behavior and hosted CI execution. A Windows/Linux CI workflow is provided; it has not been run remotely.
