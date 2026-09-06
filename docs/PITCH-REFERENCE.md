# Frequency reference before resonator fitting

Date: 2026-09-06. The first calibration preparation now establishes a
frequency-only target from the acquired G3 recordings. It does not infer physical
geometry, modal damping, hammer speed or pickup gain from processed audio.

```text
cargo run --locked --release -p rf-73-lab -- prepare-pitch-reference references/g3-pitch-reference.manifest.json --output renders/g3-pitch-target.json
```

The [manifest](../references/g3-pitch-reference.manifest.json) pins the five
original mono files by Git blob identity. The command checks each blob on the
exact bytes passed to the Rust WAV decoder. It needs Git, rejects duplicate
content across roles, limits manifests to 64 KiB and files to 32 MB, and never
overwrites reports. It loads one reference at a time and produces a compact
JSON report; no audio downloads, resampling, render grid or analysis cache is
created. Provenance, processing and license remain documented in the
[acquisition inventory](../references/jrhodes-g3.inventory.json) and
[bank review](REFERENCE-BANKS.md). Source WAVs are not redistributed.

## Selection and split

Training layers are 1, 3 and 5; validation layers are 2 and 4. Each qualified
training take has equal weight in a mean of log frequency. No recording level,
MIDI velocity range or inferred physical strike speed weights the fit. Invalid
training takes are not silently dropped. Validation observations evaluate the
frozen target and cannot choose or modify it. All five layers were inspected
in previous pilots, so this is a reproducible protocol split, not blind data.

Each take uses three nonoverlapping 512 ms observations starting at file times
0.25, 0.762 and 1.274 seconds. There is no automatic onset alignment and no
shortening of incomplete windows. A symmetric Hann window with DC removal and
2x zero padding feeds the existing spectral estimator. The actual unpadded
observation resolution is about 1.953 Hz; interpolated decimal precision is
not a confidence interval. The same physical window duration is used at every
supported sample rate, without the older short-spectrum sample cap.

The expected note supplies a broad +/-400-cent search interval. The strongest
local peak in that interval must have at least 24 dB margin over the median
band background, no other resolved local maximum within 20 dB of its amplitude,
and at least two observation-resolution units of clearance from a search edge.
All windows must qualify and their frequency span must stay within 5 cents.
Every take must agree with the training target within 5 cents for the reference
set to qualify. These are explicit pilot engineering limits, not perceptual
or metrological guarantees. Nearby unresolved components can remain hidden.

This broad search matters for the present assembly. Asking the older generic
`analyze --note 55` for a fundamental on the provisional 75 mm render finds a
weak component near 198 Hz. Its dominant body component is around 169.57 Hz,
outside that analyzer's narrow expected-note search. The generic tool remains
a hinted estimator; its result must not be treated as proof of a fundamental.
The new preparation reports a dominant component in a declared band, not an
identified structural eigenmode or an automatic universal pitch detector.

## Measured pilot

The [complete compact receipt](../references/g3-pitch-reference-validation.json)
records the exact protocol, all windows and rejected/accepted status. All five
reference takes qualify. The training target is **196.386147 Hz**.

| Layer | Role | Anchor Hz | Error from frozen target, cents |
| --- | --- | ---: | ---: |
| 1 | Training | 196.459364 | +0.6453 |
| 2 | Validation | 196.397482 | +0.0999 |
| 3 | Training | 196.355662 | -0.2688 |
| 4 | Validation | 196.345501 | -0.3584 |
| 5 | Training | 196.343436 | -0.3766 |

Maximum within-take temporal span is 0.929 cents. These observed differences
include analysis and recording variability; there are no repeated captures
at each intensity to estimate repeatability independently.

One additional default memory-modal render lasts 2 seconds with damper
engagement at 1.85 seconds, keeping all three observation windows before the
damper event. It passes the existing physical/audio preview gates and measures
**169.570540 Hz**, or **-254.169 cents** from the reference target. That default
geometry was never assigned a calibrated MIDI note. This diagnostic does not
participate in fitting or reference qualification, and its mismatch does not
make the source target invalid. Reproduce it with:

```text
cargo run --locked --release -p rf-73-lab -- render-memory-modal --output renders/pitch-reference-default-75mm.wav --seconds 2 --hold 1.85
cargo run --locked --release -p rf-73-lab -- prepare-pitch-reference references/g3-pitch-reference.manifest.json --output renders/g3-pitch-target-with-candidate.json --candidate renders/pitch-reference-default-75mm.wav
```

The retained local WAV/report use the first path above. Fresh reproduction
requires fresh destinations. The WAV is about 384 kB; no velocity or geometry
matrix was generated for this stage.

## What the next fit can establish

The next resonator experiment can vary one declared tuning parameter against
this frozen frequency target while holding the other assumptions explicit.
It must measure the coupled structure: fixed-root beam modes alone are not
the assembled instrument modes. Track the selected component through the
parameter sweep and reject peak switching or ambiguity. Confirm the resulting
output with a finer integration and the same hammer/pickup comparison protocol.

A single frequency cannot independently identify beam length, diameter,
tuning-mass position, material modulus and support properties. A useful initial
tuning setting is not a measurement of those quantities. The processed samples
and unknown sustain boundaries also do not authorize natural-decay fitting.
Higher modes, decay, pickup loading and material identification remain separate
tasks requiring additional observations. No DSP profile or plugin changed here.

## Verification

Synthetic regressions exercise dominant-component selection against a weak
expected-note neighbor at 44.1/48/96 kHz, preserved frequency under gain changes,
harmonic-rich output, silence, missing-band content, competing peaks, incomplete
windows and temporal drift. Laboratory tests verify training-only target fitting,
rejection of failed training observations, pinned-byte mismatches and protection
of existing output. The synthetic CLI roundtrip uses the current Rust engine.

All 103 tests in the affected analysis/laboratory crates pass, along with
workspace-wide strict Clippy, formatting and the native release lab build.
The additional physical WAV passes independent inspection: 48 kHz mono float,
96000 frames, finite, peak 0.358247459. Remote CI and listening were not run.
The prior 201-test full-workspace qualification belongs to the hammer-comparison
commit; unaffected DSP/plugin runtime tests were not rerun for this analysis-only
change. The workspace now contains five additional regressions.

After verification, local `target/debug` was cleaned with Cargo: 1565 regenerable
files, about 1.2 GiB. The release executable, original references, previous test
WAVs and qualification reports were preserved. Incremental compilation remains
disabled for local verification commands to bound future cache growth.

The next [coupled spring-position experiment](SPRING-TUNING.md) uses this frozen
frequency target without modifying the reference split or receipts. It separates
an explicit provisional tine length from the movable tuning-mass control and
retains a before/after pair for evaluating pitch and timbre together.
