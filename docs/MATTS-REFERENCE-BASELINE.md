# Matt's Fender Rhodes measured baseline

Date: 2026-09-15. This is an offline measurement study, not a plugin release or
listening test. No engine settings or factory presets were changed.

## Findings

All 292 mono 48 kHz float WAVs decode with finite samples. None reaches digital
full scale; the largest peak is approximately -10.15 dBFS. This does not rule
out clipping earlier in the recording chain. All files have a body fundamental
consistent with filename C3 = MIDI 60: G2 is concert G3, MIDI 55.

The 219 adjacent-layer pairs are not exact scaled copies under a sample-aligned
least-squares test: the smallest relative RMS residual is 0.0843. This is not
proof of independent capture. Dynamic labels remain ordinal, with unknown
recorded hammer velocity and capture gain.

At illustrative model velocity 0.65 paired with the mf recording, fundamental
amplitude slopes over onset +0.5 to +4.5 seconds are:

| Concert note | Reference (dB/s) | Original (dB/s) | Calibrated (dB/s) |
| --- | ---: | ---: | ---: |
| D3, MIDI 50 | -2.79 | -9.76 | -1.52 |
| G3, MIDI 55 | -2.74 | -11.31 | -2.42 |
| B3, MIDI 59 | -3.45 | -12.70 | -2.96 |

Calibrated broadly improves the observed sustain compared with Original.
These are component-envelope observations, not estimates of mechanical loss.
Nonlinear pickup behavior can change the measured fundamental envelope.
Calibrated D3/f even has a rising fundamental and fails the decay gates;
Calibrated G3/f also fails the exponential/slope consistency gates. Reference
D3/f and B3/f fail the phase gate. Their provisional slopes must not be used
as qualified decay targets. All 12 source pitch anchors pass their gates.

The strong-hit harmonic balance needs further work. G3 body harmonics,
relative to each signal's own fundamental:

| Layer / model velocity | Reference H2 / H3 | Original H2 / H3 | Calibrated H2 / H3 |
| --- | ---: | ---: | ---: |
| p / 0.25 | -31.99 / -32.64 dB | -25.84 / -53.97 dB | -19.30 / -40.25 dB |
| f / 0.85 | -3.49 / -3.64 dB | -10.59 / -24.41 dB | -13.67 / +1.64 dB |

Under this provisional pairing, Calibrated's strong G3 has too little H2 and
too much H3. A velocity mismatch could contribute; this does not uniquely
identify a pickup geometry correction.

## Method and limits

- Rust performs decoding, spectral measurements and in-memory model renders.
  Each candidate is held for six seconds at 48 kHz; gain is 0.1 and level
  compensation is enabled. No redundant audio render matrix is saved.
- Model velocities p/mp/mf/f = 0.25/0.45/0.65/0.85 are illustrative, not fitted
  or measured. Profiles reproduce the current plugin's Original and Calibrated.
- Attack/body windows are onset aligned. Harmonic balance is normalized to H1;
  raw level differences do not establish a plugin loudness error.
- Component windows are 512 ms, with 128 ms hops. Unknown neighboring modes
  can evade qualification gates; passing is not proof of modal isolation.
- G2/f reaches a flat approximately -162.7 dBFS tail at 7.92 seconds. The source
  note-off/edit boundary is undocumented. No full-file natural T60 is inferred.
- The short body spectrum supports octave identification, not precision tuning
  calibration across the entire keyboard. A#2 (MIDI 58) is an author-reported
  weak tine and should not be a normal-tine fit target.

## Receipts and reproduction

The [compact validation receipt](../references/matts-reference-validation.json)
retains per-file measurements, all layer-pair checks, qualification reasons,
24 harmonic comparisons, profiles and hashes of the detailed local receipts.
The [acquisition inventory](../references/matts-fender-rhodes.inventory.json)
identifies the source recordings. Audio remains in ignored local storage.

The root locked build currently encounters an SDK dependency-resolution mismatch
against the local RackForge checkout. The standalone
[runner manifest](../references/matts-reference-runner/Cargo.toml) and its lock
file build only the existing DSP/analysis crates and the laboratory example,
without modifying the root Cargo.lock. From the RF-73 workspace in PowerShell:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$runner = 'references/matts-reference-runner/Cargo.toml'
cargo build --locked --release --manifest-path $runner
cargo test --locked --release --manifest-path $runner
cargo clippy --locked --release --manifest-path $runner --all-targets -- -D warnings
$samples = 'references/audio/matts-fender-rhodes/samples/original'
& target/release/matts_reference.exe audit $samples renders/matts-reference/audit-new
& target/release/matts_reference.exe compare $samples renders/matts-reference/comparison-new
```

Output directories must be new. Validation: 14 tests pass (including a known
exponential envelope and proportional-copy discrimination); release Clippy
passes with warnings denied. The study's target directory was approximately
90 MB and detailed reports approximately 30 MB.

## Next calibration step

Completed follow-up: [bounded pickup/velocity search](MATTS-CALIBRATION-SEARCH.md).
The candidate improved the aggregate metric but failed the per-note validation
gate and was not promoted.

Fit a shared velocity response together with pickup geometry using the three
anchor notes, preserving qualification gates. Validate on held-out bass and
treble notes and avoid using absolute recorded gain as a known strike measure.
Require improvement in both harmonic balance and envelope behavior before
changing a factory preset and preparing a RackForge audition build.
