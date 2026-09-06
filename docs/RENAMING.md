# RF-73 project name

The project is named **RF-73**. Source folders, Cargo packages and command-line
tools use `rf-73-*`; Rust imports and generated WASM filenames use `rf_73_*`.
The repository and local project folder use `RF-73`. Visible plugin labels,
the Rust program editor, PLAY page and new package filenames use RF-73.

```text
cargo run --locked --release -p rf-73-lab -- audition --prepare-only
cargo run --locked --release -p rf-73-lab -- render --output renders/preview.wav
```

The existing RackForge plugin ID `org.rackforge.rhodes` and instance ID
`desktop.org.rackforge.rhodes` are retained as compatibility keys. Changing
these keys would disconnect existing saved sessions, presets and custom
programs from the instrument. They are not the visible product name. State
schema, parameter IDs, factory program IDs and DSP behavior are unchanged.

The audition command recognizes its earlier ownership marker and adds the
new `.rf-73-owned` marker after verification. It preserves the old marker and
audio/MIDI/session settings, and rejects a conflicting new marker. The desktop
override is now `RF_73_DESKTOP`; the previous `RF_RHODES_DESKTOP` remains a
fallback for existing local configuration. The new variable takes precedence.

Historical numeric receipts, source hashes, published paper titles, third-party
instrument names and original reference-bank paths retain their original text.
Previously generated archives also retain their original filenames. They are
evidence of earlier experiments, not new RF-73 builds. Documentation commands
use the current package names; use the original Git revision when reproducing
historical byte hashes. Git history is preserved.

The pitch-reference preparation already in progress was preserved during this
rename. This naming change does not modify the physical model or claim any
additional calibration.

## Verification on 2026-09-06

The local directory is `RF-73`, and GitHub repository metadata and `origin`
point to `https://github.com/kalexis1994/RF-73`. All 206 workspace tests pass,
including the existing ownership test extended to cover legacy-marker adoption,
preservation of settings and rejection of a conflicting new marker. Formatting
and strict workspace Clippy pass. The native release workspace and portable
plugin/UI build successfully; generated browser bindings use the renamed crate.

RackForge's CLI reports `PLUGIN_PACKAGE_VALID` with name `RF-73 Research` and
`PLUGIN_SMOKE_OK`. The smoke check loads the factory program, opens the program
editor contract, roundtrips gain and serializes the same 20-byte state. The
new local archive is `dist/RF-73-0.1.2.rfplugin`, 237215 bytes, SHA-256
`dae600bb03f7355b81ba6a517bdf209ad11448556918370940dbda19754fd8c2`.
No Desktop launch, listening test or installation into a running host occurred.

After validation, `cargo clean` removed 1617 regenerable build files, about
736.2 MiB, including artifacts under the old crate names. The archive,
`package/component.wasm`, generated UI assets, recordings and experiment
reports remain. The next Cargo command rebuilds the laboratory cache.
