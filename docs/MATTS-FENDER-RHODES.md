# Matt's Fender Rhodes reference acquisition

Acquired from the user-supplied Pianobook download on 2026-09-15. The original
download and extracted directory remain in Downloads. The working reference
copy is under ignored `references/audio/matts-fender-rhodes/`; no sample audio
is included in tracked project files.

## Verified file inventory

- 292 WAV files: 73 distinct filename pitches, each with p, mp, mf and f.
- Mono, 48,000 Hz, 32-bit IEEE floating-point WAV delivery format.
- Durations range from 7.9700208333 to 8 seconds.
- Approximately 448.84 MB copied, including the Kontakt NKI, README and artwork.
- Original ZIP CRC verification passed; all copied SHA-256 hashes match.
- WAV RIFF sizes, chunk bounds and data frame alignment were checked.
- No `smpl` root-note metadata is present. Subsequent audio measurements confirm
  filename C3 = MIDI 60; the NKI velocity/key mapping remains unverified.

These acquisition checks were followed by the
[measured baseline](MATTS-REFERENCE-BASELINE.md), including finite-sample, peak,
pitch and spectral checks. No listening qualification was performed.
Float delivery does not establish the original
ADC bit depth. Nearly uniform eight-second files do not establish complete
natural decays or documented note-off times.

The [inventory](../references/matts-fender-rhodes.inventory.json) records every
retained file, hash, size and WAV header, plus the ZIP identity.

## Provenance and limits

Matt Blostein's included README identifies a Fender Rhodes Mark I 73 recorded
through a Countryman Type 10 active DI into a Chandler TG2 microphone input,
with no processing. It identifies A#2 as a weak/dead tine: the measured octave
mapping places this at MIDI 58 (concert A#3).

Source: [Pianobook pack](https://www.pianobook.co.uk/packs/matts-fender-rhodes/).
Terms: [Pianobook EULA](https://www.pianobook.co.uk/terms-conditions/).
The supplied folder has no separate license file. Reference acquisition does
not imply permission to redistribute the WAVs inside RF-Tines.

Fixed gain across takes, measured hammer velocities, release boundaries and
sample editing are not documented in the supplied README. Dynamic labels are
ordinal performance labels, not known MIDI ranges or measured strike speeds.

## Comparison status

The baseline audits all 292 files and compares Original and Calibrated at
MIDI 50/55/59 across all four layers (24 model cases). Harmonic balance is
expressed relative to each signal's fundamental; absolute capture gain and
source strike velocities remain unknown. Bass and treble are not fitted.
See the baseline report for qualification gates and the next calibration step.

Wonder Rhodes has not yet been supplied or acquired.
