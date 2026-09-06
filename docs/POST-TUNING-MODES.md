# Post-tuning modal observations

This experiment examines the frozen spring-tuned pair and all five processed G3
reference layers. It changes no physical parameter and renders no new audio.
The purpose is to distinguish spectral evidence near proposed resonances from
an identified mechanical mode before fitting modal weights or losses.

## Reproduction and provenance

The [input manifest](../references/g3-post-tuning-observation.manifest.json)
records the exact Git blob identity, prior measured fundamental and proposed
resonances for each input. The model before/after use their respective coupled
spectra from the [spring tuning receipt](../references/g3-spring-tuning-validation.json).
Every reference is examined against the same frozen post-tuning proposals.
All five previously inspected layers remain in the report; their existing
training/validation labels do not imply a blind test or a new fit.

```powershell
$env:CARGO_INCREMENTAL = '0'
cargo build --locked --release -p rf-73-lab
$manifest = Get-Content references/g3-post-tuning-observation.manifest.json -Raw | ConvertFrom-Json
foreach ($inputTake in $manifest.inputs) {
    $modeList = ($inputTake.proposed_modes_hz | ForEach-Object { $_.ToString('R', [cultureinfo]::InvariantCulture) }) -join ','
    $fundamental = $inputTake.fundamental_hz.ToString('R', [cultureinfo]::InvariantCulture)
    & ./target/release/rf-73-lab.exe observe-modes $inputTake.file --blob-sha1 $inputTake.git_blob_sha1 --fundamental $fundamental --modes $modeList --output $inputTake.output
    if ($LASTEXITCODE -ne 0) { throw 'Modal observation failed' }
}
```

Each report must be a new file. The Rust command verifies the pinned blob with
Git and decodes those same bytes. Reference source and processing provenance
remain in [the frequency-reference protocol](PITCH-REFERENCE.md). No audio
sample-rate conversion or output gain fitting is involved.

## Observations and interpretation

The analyzer observes complete 32 ms and 128 ms attack windows at file offset
zero, then a complete 512 ms window starting at 250 ms. Native sample rates are
preserved; rounding to a whole sample is reported. Shorter inputs fail instead
of silently shortening a window. These are distributed-file offsets, not
measured hammer-contact times. The model retains its common FIR delay.

The existing Hann/background/leakage-qualified detector finds peaks without
using the proposed resonances. A resonance can subsequently refer to accepted
peaks within one reciprocal-window-duration on each side. This interval is an
observation tolerance, not a statistical confidence interval or a fitted mode
frequency. All independently accepted peaks are retained, including peaks with
no proposed-mode match.

A proposed resonance within two reciprocal-window-durations of an integer
multiple of the supplied fundamental is flagged as having unresolved origin.
A neighboring proposed mode, ambiguous detected neighbor or detector capacity
limit is also explicit. The full search interval must lie in the detector's
observable band. Non-detection means no peak passed these particular thresholds
and windows; it is not proof that the physical instrument lacks the mode.

An isolated frequency candidate still does not identify a structural mode:
magnetic intermodulation, electrical processing and unknown excitation can
produce or hide components. The harmonic-overlap flag covers integer multiples
of the supplied fundamental, not every possible nonlinear combination. Raw
spectral snapshots and harmonic searches are descriptive; they do not bypass
the background-qualified detector or confer modal identity.

No natural-decay curve is fitted because source note-off and processing remain
uncertain. Unknown recording gain, treble boost, low-mid scoop and noise
reduction also prevent treating relative peak levels as calibrated physical
hammer/pickup weights.

## Findings (2026-09-06)

The [retained observation receipt](../references/g3-post-tuning-observation-validation.json)
contains all accepted peaks, proposed-mode associations, detector counts and
artifact hashes. Seven detailed local reports occupy about 325 kB in total.
Their initially generated versions are retained as small diagnostic artifacts;
no new WAV files were generated.

In the tuned model's first 32 ms, accepted peaks lie near 1361.92, 3686.21,
6961.11, 11756.66 and 18196.06 Hz, matching the proposed higher coupled
resonances within the declared search interval. The 1361.92 Hz component is
only 6.29 dB below the fundamental in this window. That agreement checks the
model's audible consequences; it does not show agreement with the instrument.
Several of these candidates have unresolved origin relative to nearby integer
harmonics in the short window. At 128 ms, the 3686 and 6961 Hz candidates are
separated from the nearest integer harmonics, but the 1362 Hz candidate remains
within the 15.625 Hz separation limit around H7.

Reference peaks near the frozen model's higher-mode proposals do not pass the
same detector/search intervals. Three reference windows hit the 32-peak
capacity (layer 1 body, layer 2 attack-128/body); those windows are inconclusive.
Layer 1's 32 ms window has no accepted peaks despite substantial raw spectral
content: its 208 weak-peak rejections show why non-detection cannot mean absence.
The detector tolerances were not relaxed to manufacture agreement.

A separate candidate family appears in the 128 ms observations:

| Input | Accepted candidate | Nearest integer harmonic | Interpretation |
| --- | ---: | ---: | --- |
| Tuned model | 1361.86 Hz | H7, about 1374.65 Hz | Unresolved origin at this window length |
| Reference layer 1 | None in 1420..1430 Hz | H7, about 1375.22 Hz | No accepted candidate in this interval |
| Reference layer 2 | None retained in 1420..1430 Hz | H7, about 1374.78 Hz | Capacity limited; inconclusive |
| Reference layer 3 | 1425.06 Hz | H7, about 1374.49 Hz | Separated frequency candidate |
| Reference layer 4 | 1425.27 Hz | H7, about 1374.42 Hz | Separated frequency candidate |
| Reference layer 5 | 1425.50 Hz | H7, about 1374.40 Hz | Separated frequency candidate |

The 1425 Hz family was noticed after examining the full accepted peak lists.
It is a post-hoc hypothesis, not a frozen target, independent validation or an
identified second bending mode. The three listed detections are not flagged
as ambiguous and their windows are not capacity limited. All five layers,
including the non-detection and inconclusive case, remain in the record.
The supplied fundamental for harmonic proximity is each take's previously
measured three-window anchor, not a pitch refitted to this attack.

The current observations do not justify treating the spring-tuned 70 mm model
as timbrally calibrated. Fundamental pitch is close while several higher
components differ. They also do not identify one physical parameter uniquely:
processed captures and unknown strike speed leave geometry and spatial weights
underdetermined.

## Next physical experiment

Run a bounded sensitivity study of tine blank length and tuning-mass geometry,
restoring the same coupled fundamental with spring position at each setting.
Record higher-mode ratios and hammer/pickup weights together; distinguish
unreachable targets or ambiguous branches from usable settings. Use the
1425 Hz family as an exploratory consistency check, not an automatic fit.
Confirm modal identity with additional evidence before changing damping or
optimizing the hammer/pickup response. Keep the existing tuned listening pair
as the unchanged baseline for the next candidate.

## Verification

All 111 affected analysis/laboratory tests pass in release mode, together with
workspace-wide strict Clippy and formatting. Four new tests cover native-rate
physical windows, gain invariance, unresolved harmonic/modal neighbors,
capacity and missing-input limits, silence, pinned-byte mismatches and output
protection. No DSP equations, plugin parameters or source audio changed.
