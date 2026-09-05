# Desktop audition workflow

Run from RF-Rhodes whenever preparing a version for hands-on testing:

```text
cargo run --locked --release -p rf-rhodes-lab -- audition
```

Once the laboratory is built, `target/release/rf-rhodes-lab.exe audition` runs the same workflow. Rebuild the laboratory after changing its source or workspace version. The first Cargo build on Windows GNU needs the linker setup described in [Development](DEVELOPMENT.md); subprocess builds add the known MSYS2 linker directory to their own PATH when available.

## What it does

1. Acquires an exclusive workflow lock and checks that RackForge is closed.
2. Rebuilds the plugin's release WASM and the sibling RackForge core/store tools with Cargo.lock.
3. Builds the Rust PLAY UI and generated bindings, creates a uniquely named test-run directory, validates metadata, smoke-tests the plugin in RackForge and creates its `.rfplugin` archive. This requires wasm-bindgen-cli 0.2.127.
4. Installs and enables the package through `rackforge-store`, restricted to the marked development library. Same-version replacement is supported here; normal release archives remain untouched.
5. Checks installed version metadata and compares installed DSP WASM and all four UI assets with the freshly built package.
6. Selects the Rhodes instance in a backed-up session checkpoint, preserving existing master controls and Rhodes program selection.
7. Opens the native RackForge window and records its process ID and startup log.

The workflow uses the workspace package version for artifact names. Update the workspace and package metadata together for a new release; inconsistent installed version metadata fails verification. Stable three-part versions are supported. A newer installed version blocks a downgrade because RackForge selects the highest enabled version for a plugin ID.

## Files and settings

The dedicated RackForge Root is `dist/audition/RackForgeData`. It contains the plugin store, session, configuration and private plugin data. Its ownership marker prevents adopting an unrelated existing library. The workflow does not modify the normal RackForge library.

On the first run only, audio/MIDI preferences are copied from `%LOCALAPPDATA%/RackForge/config/audio.toml` if it exists. Otherwise RackForge chooses its defaults. For a custom regular-library location, set audio and MIDI once in the audition window. Later runs keep the audition library's settings.

Every attempt has its own `dist/audition/<version>-<timestamp>-<pid>/` directory, containing the archive, `audition.json` receipt and, when launched, `rackforge.log`. An existing session checkpoint is retained as `session-before.json` before selecting RF-Rhodes. Source files and normal versioned release archives are not overwritten. Generated data remains ignored by Git.

The Desktop executable is the most recently modified existing file among the sibling's `dist/windows-x86_64/rackforge.exe` and `target/release/rackforge-desktop.exe`. This avoids automatically choosing an older packaged shell over a newer local build. To select a specific compatible host, set `RF_RHODES_DESKTOP` to its executable path. The receipt records the exact executable. The workflow does not rebuild the Desktop UI or enable a network listener.

## Repeating a test

Close the RackForge window after testing, then run `audition` again. RackForge allows one Desktop instance, so the command stops before installation if a normally named RackForge process is already running. It never force-terminates the application. A second check after compilation catches a host opened during the build. Keep other launchers closed while installing; the external application's global mutex is not shared with the laboratory's workflow lock.

Installation, activation and launch failures return a nonzero exit status. A successful launch check means the child process remained running through the first three seconds; it is not proof of audible output. Read the startup log and RackForge's Audio & MIDI status if a device fails. There is no automatic test note or MIDI playback.

For installation without opening a GUI:

```text
cargo run --locked --release -p rf-rhodes-lab -- audition --prepare-only
```

Do not use the GUI-launching command in CI. Unit tests cover checkpoint preservation, library ownership, installed-byte verification and numeric version ordering; normal CI also covers the renderer, DSP and plugin contracts. Real Windows launch testing is performed locally.

## Local verification

On 2026-09-04, the workflow completed preparation and then a second installation/launch of version 0.1.1 in the same test library. Installed bytes matched the freshly built WASM, and the host's loopback API reported RF-Rhodes 0.1.1 active and managed. The saved session selected `desktop.org.rackforge.rhodes` with `research-direct`. Startup logs confirmed WASAPI at 48 kHz and the configured KeyLab MIDI connection. A third attempt while Desktop was open returned an error before creating a new test-run directory. No listening judgement or automatic note playback was part of this check.

All 44 Rust tests and strict Clippy passed locally. This verification used the existing local Desktop executable; its revision is independent of the core/store source rebuilt by the workflow.
