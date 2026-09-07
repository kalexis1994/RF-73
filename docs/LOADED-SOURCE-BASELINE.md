# Loaded model against the G3 source bank

This block compares the complete tuned action, two-plane tine, spatial pickup
and loaded circuit with all five locally retained G3 layers. It records a
baseline for subsequent physical calibration. It does not optimize material
losses, pickup geometry, EQ, gain or a mapping from sample layers to gestures.

## Sources and excitation

The [manifest](../references/g3-pitch-reference.manifest.json) pins the jRhodes3d
bank revision and each Git blob identity. All bytes are hashed before those same
bytes are decoded. Metadata records a 1977 Mark I Stage 73 harp-output source
with treble boost, low-mid scoop and noise reduction, unknown capture gain and
prior exposure of all layers. These are processed reference signals. Their
spectral differences cannot uniquely identify a mechanical or magnetic error.

The existing three training layers alone set the mean log-frequency target.
The two validation layers do not choose tuning or any other parameter. Source
roles remain attached to all comparisons. No source is selected or discarded
because it matches a particular synthetic gesture. Layer numbers are not
measured hammer velocities or MIDI velocity calibration.

The synthetic cell is the previously tuned 70 mm blank with a 0.1 g point mass.
All action, boundary, magnetic and electrical constants remain unchanged.
The initial planned pedestal-speed set, 0.75/1.125/1.5 m/s, failed because its
first gesture never produced hammer contact over the 2.5-second take. The
[failed receipt](../references/loaded-source-timbre-baseline-validation.json)
remains retained; the initial output was not relabeled as a soft note.

A bounded 120 ms contact preflight now precedes long rendering. Its six speeds
are 0.75, 1.0, 1.125, 1.25, 1.5 and 1.75 m/s. The first two have no contact
within the probe. The latter four have approximate pre-contact hammer speeds
0.096, 0.567, 1.082 and 1.442 m/s. These speeds are sampled immediately before
the first contact tick, not measured human touch. The `--striking` follow-up
chooses 1.125/1.5/1.75 m/s after that feasibility result. Qualification limits
and physical parameters are unchanged. The default command now rejects a
non-striking requested profile at preflight, retaining the evidence.

Every selected gesture runs at 128 and 256 midpoint ticks per frame, preserving
the [long-gesture tuning protocol](LOADED-SPRING-TUNING.md): 2.5 s, 48 kHz,
two key gestures, closed pedal and fixed output gain 0.1 FS/V. The same relative
energy, exchange, monotone-heat, headroom, contact-count and per-window voltage
refinement gates must pass. Every fine candidate also needs a qualified broad
pitch anchor within 5 cents of the training target.

## Time and frequency observations

`measure_timbre_profile` measures the original sample rate without resampling.
The reference onset is operational: the first four complete 1 ms RMS bins above
-40 dB relative to the largest bin in the first 250 ms. For the model, the
first physical hammer contact plus the nominal FIR group delay anchors the
observation. A pre-onset peak is retained. No time warping or attack fitting is
performed. Source onset detection and mechanical onset are different timing
observations; short-window comparisons remain sensitive to that uncertainty.

Five intervals relative to onset cover 0–64, 64–192, 256–512, 640–1152 and
1152–1664 ms. Every window requires full real sample support. Its RMS level is
reported both raw and relative to the same clip's 256–512 ms body window.
Thus the level trajectory preserves attack/decay differences without requiring
a known capture gain or fitting each window independently.

Hann-windowed spectral power is integrated in four disjoint bands with edges
`0.5*f0, 1.5*f0, 4*f0, 12*f0, 8000 Hz`, using each clip's independently
observed fundamental. This avoids the nominal-note harmonic-bin mismatch in
the earlier before/after pilot. Fractions are normalized by the total power
inside those bands; band balance is relative to the first band. A fraction
below 1e-8 withholds its dB comparison instead of manufacturing a finite floor.
These are broad output bands, not identified tine modes or harmonic amplitudes.

All five layers are compared with all three gestures, producing fifteen pairs
and seventy-five paired time windows. Descriptive differences exceeding 6 dB
in band balance or 3 dB in the relative level trajectory flag disagreement.
Missing bands cannot establish agreement; missing estimates do not hide a
measurable disagreement elsewhere. These limits describe this baseline and
are not thresholds for perceptual realism or release qualification.

The report's `measurement_qualified` field concerns input and numerical
measurement validity. `within_descriptive_tolerances` describes individual
comparisons. `reference_match_claimed` is explicitly false. A successful
command does not mean the instrument matches the recordings.

## Commands and storage

```text
cargo run --locked --release -p rf-73-lab -- compare-loaded-bank references/g3-pitch-reference.manifest.json --output references/NEW.json --preview renders/NEW.wav --striking
```

Only the 1.125 m/s fine WAV is optionally written. Other waveforms remain in
memory and are discarded after measurement; source WAVs are not copied. Existing
report and preview paths are rejected before source verification or rendering.
Invalid manifests and hash mismatches cannot generate synthetic output. Runtime
failure and failed measurement qualification retain a failed receipt.

## Retained result

The [striking follow-up](../references/loaded-source-timbre-striking-validation.json)
qualifies all five source profiles and all six synthetic takes. Each of the
fifteen pairwise comparisons exceeds at least one descriptive tolerance. Nine
pairs retain training roles and six retain validation roles; neither group
selects a best-match gesture or fitted parameter set.

| Observation | Retained result |
| --- | --- |
| Worst per-window voltage refinement error | 0.003402% |
| Worst total relative energy defect | 1.616e-12 |
| Worst relative exchange defect | 6.796e-19 |
| Maximum coupling iterations | 3 |
| Output pitch errors for the three gestures | -0.0540, -0.0457, -0.0377 cents |
| Last-window level relative to body, source range | -5.58 to -2.43 dB |
| Last-window level relative to body, model range | -14.58 to -13.63 dB |
| Pairwise excess late attenuation in the model | 8.06 to 12.15 dB |

All takes contain two hammer contacts. Their initial state and event schedules
remain continuous. The 1.5 m/s take summaries reproduce both resolutions from
the earlier loaded tuning receipt exactly. The same structural fit also
replays, showing that factoring the renderer for multiple drives did not alter
that baseline. Audio-derived pitch here uses the retained f64 trajectory;
the optional listening WAV is f32. Tiny differences from prior WAV-readback
pitch estimates are not changes in physical tuning.

Maximum available band-balance discrepancies range from 28.67 to 66.19 dB
across pairs. Only 8, 10 and 11 of the 15 possible band entries are available
for the three respective gestures: very weak synthetic upper bands are withheld
at later times. Those missing comparisons cannot count as agreement. The
level-trajectory mismatch is independently measurable in every pair. The model
loses substantially more wideband level over the observed interval; this does
not by itself identify a modal decay constant or separate pickup and structural
causes.

All source onset detections are at file time zero, consistent with their
distributed starts but not proof of physical hammer-contact timing. Model
analysis onsets are approximately 42.68, 38.52 and 37.28 ms after including
the FIR delay. The physical contact preflight remains in the follow-up receipt.

The optional `renders/loaded-bank-soft.wav` contains 120,000 finite samples at
48 kHz, peak 0.099136651 FS and RMS 0.004493842 FS after Rust readback. No
additional waveform matrix is written. Retained identities:

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Initial failed receipt | 162 | `66fe7167ef673664099c51e942da86c6ac2c8ce18ab1c02fe000548a7f4c1858` |
| Striking follow-up receipt | 264644 | `8c7babfc94504e15476a586642bc4316a79e87b2b4c6fa499bed9353a685fc99` |
| Optional soft preview | 480058 | `103a058b98f3abc09684faa09c5f40f31289b9209582d1dfe48b696982a87645` |

## Interpretation limits

This historical cold baseline begins with the action's unrelaxed preload. The onset anchor
prevents identifying the initial transient as hammer contact, but does not
remove any remaining preload vibration from later windows. The optional soft
preview has a peak of approximately 0.008654 FS in the first 30 ms, before the
key command moves. This is an initialization issue to resolve before using
quiet notes to identify physical parameters. No baseline waveform is subtracted
from the nonlinear simulation.

The sample bank's EQ and noise reduction also affect spectral and temporal
observations. Fixed-gain invariance does not remove those processing effects.
The comparisons therefore locate observable disagreement and useful next
controls; they do not uniquely assign it to hammer material, modal damping or
the magnetic field. No human listening, new plugin version or host test is
claimed by this experiment.

The subsequent [stationary initialization block](STATIONARY-REST.md) prepares a
rest state with the same laws and verifies quiet loaded idle behavior. Its
`--at-rest` comparison is separate; these cold receipts remain unchanged.

The original next-step decision was to prepare a stationary rest state with the same
contact laws and energy ledger, verify quiet idle behavior and preserve physical
soft-strike thresholds. Repeat the recorded-source baseline from that state
before changing modal losses or the magnetic observation. The observed fast
level decay and weak late upper bands are calibration targets, not justification
for an unverified EQ or envelope correction.
