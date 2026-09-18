# RF-Tines project name

The project is named **RF-Tines**. Source folders, Cargo packages and
command-line tools use `rf-tines-*`; Rust imports and generated WASM filenames
use `rf_tines_*`. The repository and local project folder use `RF-Tines`.
Visible plugin labels, the Rust program editor, PLAY page, CONFIG page and new
package filenames use RF-Tines.

```text
cargo run --locked --release -p rf-tines-lab -- audition --prepare-only
cargo run --locked --release -p rf-tines-lab -- render --output renders/preview.wav
```

The project was previously named RF-Rhodes and then RF-73. Both earlier renames
kept the original host identity as a compatibility key. This rename does not.

## Deliberate compatibility breaks

The plugin ID is now `org.rackforge.rftines` and the instance ID is
`desktop.org.rackforge.rftines`. The previous `org.rackforge.rhodes` and
`desktop.org.rackforge.rhodes` keys are not accepted anywhere. A host that
already holds the instrument under the old identity treats RF-Tines as a
different plugin: saved sessions, host presets and custom programs stored under
the old identity no longer resolve, and an existing audition or user library
keeps its old packages beside the new ones until they are removed by hand.

The portable program file is now `.rftines`, with format string
`rackforge.rftines-program` and media type
`application/vnd.rackforge.rftines+json`. Previously exported `.rf73` files are
rejected on import, both by the file-format check and by the plugin-identity
check. There is no conversion path in the plugin; a `.rf73` file can only be
recovered by editing its `format` and `plugin_id` fields by hand and renaming
the file.

State schema, parameter IDs, factory program IDs, the state byte layout and all
DSP behavior are unchanged. Nothing about the physical model moved, and this
rename claims no additional calibration.

## Retained across the rename

The audition command still owns only its own library. It writes the new
`.rf-tines-owned` marker after verification and adopts a directory that carries
either earlier marker, `.rf-73-owned` or `.rf-rhodes-owned`, preserving every
old marker and the library's audio, MIDI and session settings. It refuses an
unmarked directory and refuses to overwrite a conflicting new marker. Adoption
covers the directory only; the instrument identity inside it is not carried
over. The desktop override is `RF_TINES_DESKTOP`, and `RF_73_DESKTOP` and
`RF_RHODES_DESKTOP` remain fallbacks for existing local configuration, in that
order of precedence.

Historical numeric receipts, source hashes, published paper titles, third-party
instrument names and original reference-bank paths retain their original text.
The factory program names `Stage 73` and `Suitcase 73` name historical
instruments, not this product, and are unchanged; so is the 73-key range.
Everything under `references/` is retained evidence and keeps the package and
path names that were current when it was recorded. Previously generated
archives also retain their original filenames. They are evidence of earlier
experiments, not new RF-Tines builds. Documentation commands use the current
package names; use the original Git revision when reproducing historical byte
hashes. Git history is preserved, and all directory renames were made with
`git mv`.

## Verification on 2026-09-18

The rename covers the workspace manifest, the four crates and the laboratory
tool, the RackForge package manifest and notices, both Web surfaces and the
program-file module, the Node and Rust contract tests, both CI workflows, the
`references/matts-reference-runner` build manifest and all tracked
documentation.

Formatting passes, as does strict Clippy over all workspace targets and over
`rf-tines-ui` on `wasm32-unknown-unknown`. The production crates pass 72
analysis, 137 DSP, 23 plugin and 9 UI tests, matching the counts recorded
before the rename. The Web surface contract, the three audition tests including
the ownership test extended to cover adoption of both earlier markers, and both
retained physics replays pass. The Node program-format contract test passes.
The native release workspace, the portable `wasm32-unknown-unknown` plugin and
the generated browser bindings build.

RackForge's CLI reports `PLUGIN_PACKAGE_VALID id=org.rackforge.rftines
name="RF-Tines" version=0.1.14` and `PLUGIN_SMOKE_OK peak=0.170631
state_bytes=124`. That peak and state size are identical to the values recorded
for 0.1.14 before the rename, so the model and the state layout did not move.
The archive is `dist/RF-Tines-0.1.14.rfplugin`, 3471452 bytes, SHA-256
`7f7d72c70ea1d310c8ff726642b48a0dfa9b10756ae5becc5bc3fcd379c26552`.

A prepare-only audition adopted the existing local library: it preserved the
`.rf-73-owned` marker, added `.rf-tines-owned`, kept the host settings, and
installed under `org.rackforge.rftines` beside the untouched
`org.rackforge.rhodes` packages. No Desktop launch, listening test or
installation into a running host occurred as part of this rename.

The full `cargo test --workspace` run was not completed here; the laboratory
suite is long-running and is not a CI or release gate. The gates listed above
are the ones CI and the release workflow enforce.

## Branding

The icon, banner and splash under `package/branding/` carry the RF-Tines
wordmark. They were regenerated from WebP sources and encoded as the manifest
and the host validator require: PNG at exactly 512x512, 1600x400 and 1920x1080,
8-bit, non-animated, and RGB rather than a palette, which the validator rejects.
Palette quantization is therefore unavailable, so they are compressed losslessly
with zopflipng over all filter strategies. They occupy 12%, 17% and 27% of the
per-asset size limits.
