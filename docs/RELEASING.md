# Releasing RF-73

RF-73 releases are built from an annotated or lightweight Git tag whose name
matches the workspace version exactly. Pushing `v0.1.14`, for example, can only
publish a package whose Cargo and RackForge manifests both declare `0.1.14`.

## Release contract

Before tagging a version:

1. Keep `Cargo.toml`, `Cargo.lock`, `package/rackforge-plugin.toml` and
   `package/metadata/runtime.json` on the same version.
2. Run `cargo fmt --all -- --check`, Clippy with warnings denied, the production
   crate suites, Web surface contracts and the retained action and tuning
   replays. Long exploratory laboratory matrices remain explicit research runs;
   they are not duplicated in every release build.
3. Build the portable component and run `rf-73-lab package`. This command uses
   RackForge's own inspector, runtime smoke test and store packer.
4. Install the resulting package in the browser and Desktop audition libraries
   and retain the validation receipt for the tested version.
5. Push the release commit and wait for both operating-system CI jobs and the
   validated-package job to pass.

The repository expects a sibling RackForge checkout at the commit pinned in the
workflows. Local release preparation uses:

```text
cargo build --locked --release --manifest-path ../rackforge/Cargo.toml -p rackforge-core -p rackforge-store
cargo build --locked --release --target wasm32-unknown-unknown -p rf-73-plugin
cargo run --locked --release -p rf-73-lab -- package
```

## Publishing

Create and push the matching tag only after the release commit is on `main`:

```text
git tag -a v0.1.14 -m "RF-73 0.1.14"
git push origin v0.1.14
```

`.github/workflows/release.yml` independently repeats formatting, tests,
Clippy, portable builds, RackForge validation and packaging. It writes a
SHA-256 checksum, preserves both files as a workflow artifact and creates the
GitHub release only after every previous step succeeds. The workflow never
publishes from a branch or from a tag that disagrees with the source version.

Each release carries both the versioned archive and the stable
`RF-73.rfplugin` asset. RackForge pins the stable asset URL together with the
declared plugin version and SHA-256, so future pin updates do not depend on a
version embedded in the asset name.

Source reference recordings, fit renders and audition libraries are excluded
from the package. A release contains the DSP component, metadata, factory
programs, static Web surfaces, validated branding assets, the license and the
distribution notice only.
