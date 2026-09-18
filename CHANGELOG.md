# Changelog

All notable RF-Tines changes are recorded here. Versions follow semantic
versioning while the public plugin contract is still below 1.0. Entries for
releases published before the rename keep the names that were current then.

## 0.2.1 - 2026-09-18

- Fixed the instrument faceplate on the PLAY surface, which still read `RF–73`.
  The wordmark is split across three nodes as `RF<span>–</span>73`, so the
  literal string `RF-73` never occurs and every text search over the source, the
  published package and the RackForge release reported it clean. It now reads
  `RF–Tines`.
- Kept that wordmark on one line. The dash is a line-break opportunity and the
  new name is four characters longer, so a narrow panel could have split it.

## 0.2.0 - 2026-09-18

- Renamed the project from RF-73 to RF-Tines across crates, the laboratory tool,
  package metadata, both Web surfaces, CI and documentation. See
  [naming and compatibility](docs/RENAMING.md).
- **Breaking:** the plugin ID is now `org.rackforge.rftines` and the instance ID
  `desktop.org.rackforge.rftines`. Sessions, host presets and custom programs
  saved under the previous `org.rackforge.rhodes` identity no longer resolve, and
  a host that already holds 0.1.14 treats this as a different plugin.
- **Breaking:** portable programs are now `.rftines` files with format string
  `rackforge.rftines-program` and media type
  `application/vnd.rackforge.rftines+json`. Previously exported `.rf73` files are
  rejected, both by the format check and by the plugin-identity check.
- Changed the desktop audition override to `RF_TINES_DESKTOP` and the library
  ownership marker to `.rf-tines-owned`. An existing library marked
  `.rf-73-owned` or `.rf-rhodes-owned` is still adopted with its settings intact.
- Replaced the catalog icon, banner and loading splash with artwork carrying the
  RF-Tines wordmark.
- Unchanged: the state schema, parameter IDs, factory program IDs, the state
  byte layout and all DSP behaviour. The packaged smoke check reports the same
  peak and state size as 0.1.14.

## 0.1.14 - 2026-09-16

Published as RF-73, before the rename.

- Added the RF-73 icon, catalog banner and loading splash and moved the package
  to RackForge manifest schema 3.
- Added a responsive Stage/Suitcase front panel with five era-inspired factory
  programs and compact physical controls.
- Added local program saving from PLAY and `.rf73` import/export from CONFIG.
- Added channel-aware MIDI 1.0 and MIDI 2.0 pitch bend with a fixed two-semitone
  range that preserves the state of ringing physical modes.
- Added a reproducible package job to CI and a tag-gated GitHub release workflow
  that publishes the validated `.rfplugin` with its SHA-256 checksum.
