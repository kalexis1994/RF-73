# Changelog

All notable RF-73 changes are recorded here. Versions follow semantic
versioning while the public plugin contract is still below 1.0.

## 0.1.14 - Unreleased

- Added the RF-73 icon, catalog banner and loading splash and moved the package
  to RackForge manifest schema 3.
- Added a responsive Stage/Suitcase front panel with five era-inspired factory
  programs and compact physical controls.
- Added local program saving from PLAY and `.rf73` import/export from CONFIG.
- Added channel-aware MIDI 1.0 and MIDI 2.0 pitch bend with a fixed two-semitone
  range that preserves the state of ringing physical modes.
- Added a reproducible package job to CI and a tag-gated GitHub release workflow
  that publishes the validated `.rfplugin` with its SHA-256 checksum.

