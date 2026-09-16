# Upper-register geometry experiment

The frozen candidate passes the existing development and held-out spectral
and envelope gates against the original Calibrated pairing: **14.42% lower
development error and 23.35% lower held-out error**. This qualifies an offline
candidate for implementation work; it is not a released preset or listening
validation.

## Model and search

The experiment retains Calibrated's existing discrete Aperture law, mechanics,
filter and level compensation. It freezes the previously fitted ordinal
p/mp/mf/f mapping at .29/.44/.61/.85. Those values match source layers for
research; they are not a measured replacement for the plugin's MIDI curve.

Three log multipliers shape gap, offset and pole radius above MIDI 55:

```text
t = clamp((note - 55) / 17, 0, 1)
w = t*t*(3 - 2*t)
geometry(note) = Calibrated_geometry * exp(w * multiplier)
```

At and below G3 the geometry is unchanged. At and above C5 it is constant.
This preserves the lower-register spectra under the frozen mapping while
allowing the excess C5 H2 to fall. There is no independent adjustment per note.

Coarse search: 15 seeds and two coordinate rounds (.10/.05), 27 evaluations.
Its best candidate reduced aggregate error 9.78%, below the required 10%, so
reserved audio was not loaded. One refinement used that seed with steps
.02/.01/.005, 19 evaluations. All **46 trials** retain full development spectra
and constraint assessments. Selection uses the unchanged constraint-first
policy; after feasibility it minimizes aggregate error.

The final log multipliers are **[.135, -.020, 0]**. At C5 and above, gap is
0.572268 mm, offset 0.490099 mm and radius 2 mm. These are fitted effective
parameters, not identified physical dimensions. Search bounds keep the gap
within .5-1 mm, offset .25-1 mm and radius 1-3 mm throughout the keyboard.

## Frozen validation

Parameters were written before loading the reserved split. No further search
or candidate selection occurred after that point. The reserved notes are the
existing validation split, not a new external sample bank.

| Reserved MIDI note | Candidate error / original Calibrated error |
| --- | ---: |
| 45 | .9059 |
| 53 | .4840 |
| 60 | .9048 |
| 67 | .6233 |
| 76 | .8190 |
| 86 | .8817 |

Every development note's mean error also improves. The worst constrained
individual case is E4/f, up 7.52%, within its 10% limit. B2/mp improves 2.00%.

Twelve held-out envelope comparisons qualify in common; twelve are excluded
by the existing source/baseline qualification rules. There are no newly failed
candidate envelope cases. Mean absolute envelope-slope error changes by
-0.00116 dB/s, effectively unchanged; this is not evidence of an envelope
improvement. The qualification gate accepts the frozen candidate.

C5 body H2 remains too strong, but its signed errors fall in every layer:

| Layer | Original error, dB | Candidate error, dB |
| --- | ---: | ---: |
| p | 30.31 | 28.44 |
| mp | 31.18 | 27.23 |
| mf | 29.39 | 24.97 |
| f | 29.07 | 24.95 |

This is a measured improvement, not a solved high-register match. The p/mp
attack uncertainty documented in the [C5 probe](C5-H2-QUALIFICATION.md) does
not change the objective used here.

## Mapping-matched control

After freezing the candidate, a separate control renders unchanged Calibrated
with the same .29/.44/.61/.85 mapping on both splits. Candidate parameters are
not changed or reselected. Against this control, the geometry change reduces
error **10.073% in development and 15.167% held out** and passes the existing
per-note/coverage/envelope gate. Thus the original comparison's gain is not
entirely explained by ordinal remapping.

The development margin over the 10% gate is only 0.073 percentage points. This
is a narrow pass in a bounded search, not broad robustness evidence. The next
step is implementation and controlled listening, without copying the fitted
ordinal mapping into the MIDI response. Geometry-only results should be
remeasured at the plugin's normal velocity inputs during that integration.

Reproduce the control after the candidate is frozen:

```powershell
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --upper-control $samples renders/matts-reference/control-new renders/matts-reference/upper-refine-new/search.json
Copy-Item renders/matts-reference/upper-refine-new/*-candidate.json renders/matts-reference/control-new/
python tools/matts_fit_gate.py renders/matts-reference/control-new renders/matts-reference/control-new/decision.json
```

## Verification and reproduction

An independent receipt replay reconstructs every trial's score and selection,
and confirms exact equality of all lower-register cases to the remapped seed.
The zero-geometry seed also reproduces the preceding remapped-Calibrated
receipt exactly. A Rust test checks lower-key invariance and valid profiles
over all 73 keys at extreme multipliers.

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$runner = 'references/matts-reference-runner/Cargo.toml'
$samples = 'references/audio/matts-fender-rhodes/samples/original'
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --upper-register $samples renders/matts-reference/upper-new
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --upper-register $samples renders/matts-reference/upper-refine-new renders/matts-reference/upper-new/search.json
python tools/matts_fit_gate.py renders/matts-reference/upper-refine-new renders/matts-reference/upper-refine-new/decision.json
python references/upper-register-2026-09-16/replay.py
```

Use new output paths. Refinement rejects a prior search whose held-out flag
is already set. The test runner passes 36 Rust tests and ten Python tests;
release Clippy passes with warnings denied. No audio render matrix is written.

[Retained receipts](../references/upper-register-2026-09-16/) include the
full trial spectra, detailed held-out reports, gate result and provenance.
`decision.json` uses the existing gate's `promote` field for numerical
eligibility; `search.json` correctly retains `plugin_promoted: false`.

The realtime engine and presets remain unchanged. Integrating note-dependent
geometry requires verifying voice initialization/profile updates and state
compatibility, then producing an audition build. No playable version was
produced in this experiment, so RackForge audition was not invoked.
