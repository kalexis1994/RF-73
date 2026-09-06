# Reference bank selection

Research and acquisition date: 2026-09-04. The project owner has no instrument recordings available. This review selects documented banks and starts a small real-recording analysis pilot. Product specifications below are publisher statements; no commercial library was purchased or independently auditioned.

## Commercial candidates

| Bank | Published content | Assessment for RF-73 |
| --- | --- | --- |
| [Soniccouture EP73 Deconstructed](https://www.soniccouture.com/en/products/24-vintage/g36-ep73-deconstructed/) | 12 velocity layers and 3 round-robin layers for keyboard playing; separate line, microphone and contact-microphone channels; 24-bit/44.1 kHz mono; 8 GB download, 15 GB core library; displayed price USD 99 at review | Preferred commercial candidate for our Stage 73 target. The line channel and repeated strikes are useful, but the public specification does not establish untreated, fixed-gain raw captures or measured hammer velocities. |
| [Scarbee Classic EP-88s](https://scarbee.com/products/scarbee-classic-ep-88s) | 88 keys, 30 velocities, 8,294 samples; 73/88-key model options; 5.6 GB Kontakt download | Strong candidate for listening and dynamics comparisons. The producer explicitly describes four noise-cleaning stages, so the samples should not be treated as untouched decay measurements. |

Both are Kontakt instruments rather than public WAV measurement datasets. Their musical quality and layer count do not establish physical identifiability. Before using commercial content for instrument-development analysis, resolve access to the relevant line-output data, capture/processing details and the intended use with its provider. A normal music-production purchase is not evidence of permission to extract or redistribute samples. No commercial samples or plugin code were acquired in this review.

## Immediately available pilot: jRhodes3d mono

Jeffrey Learman's [jRhodes3d](https://github.com/sfzinstruments/jlearman.jRhodes3d) documents a 1977 Mark I Stage 73 recorded at the harp connector. It supplies full-length, unlooped notes with up to five layers. Treble boost, low-mid scoop and CoolEdit noise reduction are baked in. The [author's WAV repository](https://github.com/jlearman/jRhodes3d-wav) avoids a format conversion for our Rust reader.

This is a documented musical reference, not an untreated calibration dataset. The [license](https://github.com/jlearman/jRhodes3d-wav/blob/a886e6cebf074c995a10634f82ebe4fdb90f5ca6/LICENSE) declares CC BY-NC for samples, CC0 for other materials and separate musical-use terms. Keep its attribution/terms with the local references; do not bundle these WAVs into RF-73.

The sparse sampling lacks original A3/A4 strikes. Inspection of the [pinned mono SFZ](https://github.com/sfzinstruments/jlearman.jRhodes3d/blob/aea5b8d3e11e2f7102593789a4e0a0e41b30271a/jRhodes3d-mono-no-xfade.sfz) shows A3 is mapped from G3. The pilot therefore uses G3, MIDI 55, at its recorded pitch. Layer numbering runs from strongest `1` to softest `5`; mapping ranges are:

| File suffix | SFZ MIDI velocity range |
| --- | --- |
| `_1.wav` | 112..127 |
| `_2.wav` | 96..111 |
| `_3.wav` | 73..95 |
| `_4.wav` | 48..72 |
| `_5.wav` | 1..47 |

These are playback mapping ranges, not measured strike speeds or RF-73 velocities. They must not be inserted into `fit-pickup-set` as calibrated physical inputs.

## Acquired pilot and initial measurements

Downloaded only the five original mono G3 WAV files, 8,397,544 bytes total, from author revision `a886e6cebf074c995a10634f82ebe4fdb90f5ca6`. Git blob identities were verified against repository metadata, and SHA-256 hashes were recorded. The [tracked acquisition inventory](../references/jrhodes-g3.inventory.json) provides URLs, exact sizes, hashes, layer labels and provenance. The upstream license and README are retained with the audio under ignored `references/audio/jrhodes-g3-a886e6c/`.

The Rust analyzer read all five without conversion, normalization or effects. They are mono 16-bit PCM at 44.1 kHz. No sustain boundary was assumed: note-off markers and original capture gain are not independently documented, so all decay estimates remain unqualified.

| Layer | Duration seconds | Fundamental estimate Hz | Tuning error cents | Peak amplitude |
| --- | --- | --- | --- | --- |
| 1, strongest | 16.200 | 196.487 | +4.316 | 0.8903 |
| 2 | 18.201 | 196.464 | +4.112 | 0.7349 |
| 3 | 20.400 | 196.386 | +3.422 | 0.8860 |
| 4 | 20.000 | 196.348 | +3.096 | 0.8138 |
| 5, softest | 20.401 | 196.338 | +3.005 | 0.8047 |

These are estimator outputs on processed distributed files, not precision guarantees or independently measured physical tuning. All reports have zero dropped track observations. Peaks are not ordered by nominal intensity; the files do not establish a common raw recording gain for dynamics fitting. Use them first to examine resolved attack components and time-varying spectral structure, treating level, noise-floor and decay interpretations cautiously.

Full reports are ignored under `renders/jrhodes-g3-a886e6c/` and occupy about 11.2 MB. Reproduce an analysis with a fresh report path:

```text
cargo run --locked --release -p rf-73-lab -- analyze references/audio/jrhodes-g3-a886e6c/A_055__G3_1.wav --output renders/jrhodes-g3-layer1.json --note 55 --partial-window-ms 128
```

The next useful experiment is a G3 attack/body residual comparison at original pitch, after inspecting the selected intervals. Shared-gain geometry fitting on these samples remains inappropriate until strike mapping and capture-level assumptions are justified. A documented commercial bank may improve coverage, but an untreated measured set remains the preferred physical calibration reference.
