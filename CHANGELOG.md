# Changelog

All notable RF-Tines changes are recorded here. Versions follow semantic
versioning while the public plugin contract is still below 1.0.

## 0.1.14 - Unreleased

- Renamed the project from RF-73 to RF-Tines across crates, the laboratory tool,
  package metadata, both Web surfaces, CI and documentation. See
  [naming and compatibility](docs/RENAMING.md).
- **Breaking:** the plugin ID is now `org.rackforge.rftines` and the instance ID
  `desktop.org.rackforge.rftines`. Sessions, host presets and custom programs
  saved under the previous `org.rackforge.rhodes` identity no longer resolve.
- **Breaking:** portable programs are now `.rftines` files with format string
  `rackforge.rftines-program`. Previously exported `.rf73` files are rejected.
- Changed the desktop audition override to `RF_TINES_DESKTOP` and the library
  ownership marker to `.rf-tines-owned`. An existing library marked
  `.rf-73-owned` or `.rf-rhodes-owned` is still adopted with its settings intact.
- Added the catalog icon, banner and loading splash and moved the package to
  RackForge manifest schema 3. The artwork still carries the previous RF 73
  wordmark and has not been regenerated.
- Added a responsive Stage/Suitcase front panel with five era-inspired factory
  programs and compact physical controls.
- Added local program saving from PLAY and `.rftines` import/export from CONFIG.
- Added channel-aware MIDI 1.0 and MIDI 2.0 pitch bend with a fixed two-semitone
  range that preserves the state of ringing physical modes.
- Added a reproducible package job to CI and a tag-gated GitHub release workflow
  that publishes the validated `.rfplugin` with its SHA-256 checksum.

