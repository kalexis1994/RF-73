# Cross-take spectral recurrence before modal identification

This experiment groups independently detected frequencies across the five
processed G3 layers. It does not use a proposed structural frequency to find
peaks, select a geometry, or fit damping. The main new observation is that
the recurrent 1425 Hz and 1620 Hz families have a separation compatible with
the G3 fundamental in the same three takes. They cannot yet be treated as two
independent mechanical resonances.

## Reproduction and provenance

```text
cargo run --locked --release -p rf-73-lab -- observe-families references/g3-spectral-families.manifest.json --output renders/g3-spectral-families.json
```

The strict [manifest](../references/g3-spectral-families.manifest.json) includes
one MIDI note, 3..8 distinct inputs, exact Git blob hashes, prior fundamental
anchors and processing/exposure information. Duplicate IDs or content, invalid
anchors, unknown fields and oversized manifests are rejected. Each WAV's exact
bytes are verified by Git and those same bytes are decoded. Output creation is
exclusive. The [retained report](../references/g3-spectral-families-validation.json)
contains the manifest, all accepted peaks, source metadata, detector limits,
family membership and same-take frequency-relation residuals.

Source and processing remain those in [Reference banks](REFERENCE-BANKS.md):
jRhodes3d, one 1977 Mark I Stage 73, treble boost, low-mid scoop and noise
reduction. All five layers have already been examined. They are distinct takes
from one source, not independent instruments or a newly held-out validation
set. Capture gain and strike speed remain unknown. No new WAV is acquired,
generated, resampled, normalized or played by this experiment.

## Method and ambiguity handling

The existing native-rate detector runs full 32/128 ms attack windows at file
offset zero and a 512 ms window starting at 250 ms. These are file offsets,
not inferred hammer-onset times. Its thresholds and 32-peak capacity are
unchanged. Although the observation wrapper accepts a target, this command
uses only its independently detected peak lists, never target associations.

Each window is grouped separately. Peaks connect within one reciprocal-window
duration; the common tolerance is the largest value across rounded native
windows. Connected components wider than that tolerance, or containing two
peaks from the same take, are explicitly ambiguous. This prevents a chain of
nearby frequencies or duplicate observations from manufacturing recurrence.
No temporal track is inferred by matching different-duration windows.

A recurrent nonharmonic candidate requires at least three distinct takes with
uncapped windows, unambiguous detections and separation from the nearest integer
harmonic by at least two reciprocal-window durations. Harmonic proximity uses
each take's previously measured fundamental, not a refit to the attack. These
widths describe observation limits, not statistical confidence intervals.
All missing, capped, ambiguous and harmonic-overlap cases remain visible in
per-family input evidence. Non-detection is not physical absence.

Family ranges and means summarize all retained members, including flagged
ones; only reliable members count toward recurrence. They are descriptive
locations, not fitted modal frequencies. The broad 886 Hz component below
illustrates why membership and limits matter more than a rounded mean.

For pairs of recurrent nonharmonic candidates, the command checks differences
of 1..4 fundamentals and frequency multiples of 2 or 3. Support must occur in
the same three or more reliable takes. Difference tolerances add both frequency
widths; multiple tolerances scale the lower frequency's width by its order.
The fundamental anchors are treated as fixed; their estimation uncertainty is
not a confidence interval in this calculation. This limited list does not
enumerate every possible nonlinear mixture. Coincidence cannot establish mixing
or its direction, and failure of these tests cannot establish independence.

## G3 findings

At 128 ms, four components pass the recurrence rule:

| Retained frequency range (Hz) | Reliable layers | Interpretation |
| --- | --- | --- |
| 882.358..890.042 | 1, 3, 4 | Broad component near the association-width limit; layer 2 is capped |
| 1425.058..1425.496 | 3, 4, 5 | Repeats the exploratory family found in the earlier pilot |
| 1618.564..1620.825 | 3, 4, 5 | Possible fundamental-offset relation to the 1425 Hz family |
| 7106.345..7108.403 | 3, 4, 5 | Additional high-frequency candidate, no assigned structural identity |

For the 1425/1620 Hz pair, `(upper - lower) - fundamental` is -0.589, -3.057
and -2.265 Hz in layers 3, 4 and 5. All fall within the declared 15.624 Hz
difference tolerance. This is consistent with a sideband hypothesis but does
not prove one; two unrelated resonances could also happen to have this spacing.
The prior model's higher-mode proposals did not seed any of these groups.

At 32 ms, no component meets the isolated recurrence rule. At 512 ms, three
components pass near 90.0..91.1, 99.92..100.02 and 386.29..387.73 Hz. Those are
not automatically low mechanical modes: noise, processing and source pitch
behavior remain possible explanations. Layer 1 body and layer 2 attack-128/body
are capacity limited. Four connected components across the longer windows are
ambiguous. Every component, including insufficient recurrence and harmonic
overlap, is retained; the candidate table is not the complete peak list.

## Validation and next discriminating observation

Synthetic tests cover transitive chains, duplicate-take inflation, capped and
harmonic evidence, same-take support for sideband/multiple relations, malformed
manifests and mixed-note anchors. A multi-rate/gain example verifies two known
frequencies and the resolution limit: a 3207 Hz tone only 7 Hz from H16 of a
200 Hz fundamental is unresolved at 128 ms but separated at 512 ms. Its initial
test expectation incorrectly treated detection as separation; the test was
corrected without changing detector or grouping thresholds. CLI tests verify
output preservation, invalid manifests and altered source-byte rejection.

All 123 affected analysis/laboratory release tests, strict workspace Clippy
and formatting pass. Five new regressions bring the workspace total to 238;
unaffected DSP/plugin/UI tests were not rerun for this analysis-only change.
All 15 real-source windows reproduce the earlier peak counts and frequencies
within 1e-9 Hz. The retained report occupies 299317 bytes. No listening,
cross-note confirmation, decay calibration or remote-CI result is claimed.

The next useful evidence is the behavior of these families on independently
recorded neighboring notes: compare absolute frequency, frequency relative to
the fundamental and same-take combination relations. Persistent near-100 Hz
content should also be checked before interpreting it mechanically. Do not
force geometry to fit a single unassigned component. New-note observations
must preserve source provenance and remain distinct from the already-examined
G3 pilot. The existing physical model and audible baseline remain unchanged.
