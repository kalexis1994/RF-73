# Development

## Toolchain and dependency

Rust 1.98.0 is pinned. The DSP has no third-party dependencies. Offline analysis uses `hound` for WAV decoding and `serde`/`serde_json` for reports; its FFT is implemented and tested in Rust. These dependencies stay outside the audio plugin. Cargo.lock records exact versions. The plugin uses the public Rust SDK from a sibling `rackforge` checkout through an explicit Cargo path. For this prototype use RackForge revision `7c17bd4a480d1c0bd7fa18fa4d880e82429dffe1`. A local path dependency is not a reproducible distribution pin: before external releases, replace it with an exact published version or Git revision and regenerate Cargo.lock.

On this Windows GNU setup, put `C:/msys64/ucrt64/bin` on the current shell's PATH so Rust can find the linker and dlltool. No machine-wide environment changes are needed.

```powershell
$env:Path = 'C:/msys64/ucrt64/bin;' + $env:Path
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --locked --release --workspace
cargo build --locked --release --target wasm32-unknown-unknown -p rf-rhodes-plugin
```

The PowerShell line only configures the shell; every build tool, renderer and test in the project is Rust.

When disk space is limited, set `$env:CARGO_INCREMENTAL = '0'` in the build shell to avoid regenerating incremental compilation caches. This changes only that shell's builds and may increase rebuild time; normal dependencies and release artifacts still occupy space in `target`.

## Render and inspect

```text
cargo run --release -p rf-rhodes-lab -- render --output renders/a3.wav --trace
cargo run --release -p rf-rhodes-lab -- demo --output renders/demo.wav
cargo run --release -p rf-rhodes-lab -- inspect renders/demo.wav
cargo run --release -p rf-rhodes-lab -- stress
```

`--help` lists validated render parameters. Output files use create-new semantics; choose a new name to rerun. A failed disk write may leave a partial file; `inspect` checks the laboratory's WAV structure, exact length and finite samples. An existing report or trace also prevents accidental overwrite.

The raw WAV has no automatic normalization or clipping. Its JSON report exposes peak and RMS. Output gain is intentionally conservative for ordinary notes, but dense stress chords can exceed full scale. Use host gain when auditioning.

## Analyze recordings

```text
cargo run --locked --release -p rf-rhodes-lab -- analyze renders/a3.wav --output renders/a3-analysis.json --note 57 --sustain-end 1.8
cargo run --locked --release -p rf-rhodes-lab -- analyze references/audio/a3.wav --output renders/reference-analysis.json --channel 0 --note 57 --sustain-end 3
cargo run --locked --release -p rf-rhodes-lab -- analyze references/audio/a3.wav --output renders/reference-long.json --channel 0 --note 57 --sustain-end 5 --partial-window-ms 1024
cargo run --locked --release -p rf-rhodes-lab -- compare references/audio/a3.wav renders/a3.wav --reference-channel 0 --output renders/comparison.json
cargo run --locked --release -p rf-rhodes-lab -- compare-partials references/audio/a3.wav renders/a3.wav --reference-channel 0 --output renders/partial-comparison.json --seconds 2 --reference-start 0.2 --candidate-start 0.2
```

Use `analyze` to read external WAV files; `inspect` remains the strict checker for the renderer's own WAV format. Comparison requires matching sample rates. See [Analysis laboratory](ANALYSIS.md) before interpreting metrics or choosing a sustain boundary. No reference audio is included in this repository.

## Attack and body tone comparison

For short attack/body comparisons without a declared sustain boundary or velocity mapping:

```text
cargo run --locked --release -p rf-rhodes-lab -- compare-tone references/audio/jrhodes-g3-a886e6c/A_055__G3_1.wav renders/g3-baseline.wav --note 55 --output renders/g3-tone.json
```

Generate the candidate WAV first using [G3 residual pilot](G3-RESIDUAL-PILOT.md). [Tone comparison](TONE-COMPARISON.md) specifies the fixed observation windows and relative harmonic metrics.

## Pickup geometry sweep

For a bounded pickup-geometry experiment against an explicitly selected held-note region:

```text
cargo run --locked --release -p rf-rhodes-lab -- sweep-pickup references/audio/a3.wav --output renders/pickup-sweep.json --note 57 --velocity 0.7 --seconds 1 --reference-start 0.1 --model-start 0.1 --gaps-mm 1,1.5,2 --offsets-mm 0.25,0.5,0.75
```

See [Pickup sweep](PICKUP-SWEEP.md) for input bounds, ranking semantics, reference requirements and a reproducible synthetic recovery example. Candidate audio stays in memory; the only output is a create-new JSON report.

## Shared pickup fit and held-out validation

To fit several recordings and evaluate reserved notes/intensities with frozen geometry and gain:

```text
cargo run --locked --release -p rf-rhodes-lab -- fit-pickup-set references/pickup-set.synthetic.json --output renders/pickup-set-demo/result.json
```

First generate the example's three reference WAVs using [Pickup reference set](PICKUP-SET.md). That document also specifies provenance, shared capture gain, held-note regions and the strict JSON manifest contract.

## Contact refinement experiment

```text
cargo run --locked --release -p rf-rhodes-lab -- converge --output renders/treble-convergence.json --note 100 --velocity 0.2
```

The command compares fixed 4/8/16/32x integration and the production contact-refined voice against a finite 64x reference. It uses a common offline filter and writes mechanical, attack and full-window errors without automatic alignment. See [Numerical convergence](CONVERGENCE.md). Duration is limited to 0.05–1 second and velocity to 0.01–1 so the experiment stays bounded and above negligible excitation.

## Package

For the complete build/install/launch cycle, use `cargo run --locked --release -p rf-rhodes-lab -- audition`. See [Desktop audition](AUDITION.md) for the dedicated library, settings retention and repeat-run behavior. The standalone `package` command below remains useful for producing a versioned release archive without launching a host.

The research manifest uses supported legacy schema 1 and RackForge's generic appearance. No HTML, JavaScript or custom GUI is included. Gain and the Research Direct program are exposed through host contracts.

Build RackForge's current Rust `rackforge-store` and `rackforge-core` tools in its own repository (`cargo build --locked --release -p rackforge-store -p rackforge-core`). Then from RF-Rhodes run:

```text
cargo run --release -p rf-rhodes-lab -- package
```

The Rust laboratory resolves the sibling host tools, copies the current WASM to ignored `package/component.wasm`, validates metadata, renders through the host and creates the archive only after validation succeeds. It never overwrites an existing archive. Old prebuilt host binaries may not support the current API; rebuild them from the pinned source. To inspect or smoke-test manually:

```text
../rackforge/target/release/rackforge-core inspect package
../rackforge/target/release/rackforge-core smoke package --preset research-direct --data-root dist/smoke-data
```

The package is a research build, with one immutable physical profile and one user parameter. A dedicated instrument UI, branded schema 3 package, profile controls and calibrated presets are later milestones.

## Repository layout

```text
crates/rf-rhodes-dsp/       model, contact solver, voices, pickup and decimation
crates/rf-rhodes-analysis/  offline audio decoding, spectra, envelopes and comparison
crates/rf-rhodes-plugin/    SDK adapter, MIDI validation and versioned state
tools/rf-rhodes-lab/       Rust rendering, traces, reports and stress measurements
package/                  RackForge manifest and metadata
docs/                     English design, measurement and development documents
renders/                  ignored generated WAV, CSV and JSON
dist/                     ignored distributable and validation outputs
```

The DSP and laboratory forbid unsafe Rust. The plugin export macro contains the SDK's raw ABI implementation; handwritten adapter code uses safe Rust. All event lists are validated before mutation, and invalid blocks are silenced without applying partial edits.
