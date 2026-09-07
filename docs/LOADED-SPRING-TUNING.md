# Spring tuning through the loaded two-plane output

The offline electromagnetically coupled action now has a spring-position tuning
and before/after audio experiment. The spring moves on a fixed 70 mm tine;
neither tine length nor tuning mass changes. All six geometry-derived bending
modes and their spatial ports are rebuilt at each position. Thus tuning also
changes nonharmonic modal ratios and strike/pickup participation.

This extends the earlier [planar tuning pilot](SPRING-TUNING.md) to the
[two-plane transducer and circuit](ELECTROMECHANICAL.md). The frozen target comes
from the existing [G3 reference receipt](../references/g3-pitch-reference-validation.json).
Only its training-derived frequency enters the fit; the source recordings are
not fitted for timbre, amplitude, geometry or loss parameters here.

## Coupled structural modes and identity

`ModalSpectrum<18>::prepare_polarized` uses exactly the mass, stiffness and ports
prepared for the polarized action. It returns eighteen mass-normalized modes,
eigen-equation residuals and an orthogonality audit. The default
`ModalSpectrum<9>` API and its planar calculation remain available through the
same numerical implementation. Contact, damping and magnetic/electrical loading
are excluded from this undamped spectrum; its frequency is a tuning proposal.

The initial branch has the largest mass-inner-product projection onto the first
vertical tine coordinate. Subsequent positions are selected by physical shape
continuity. Coordinates alone cannot be compared because their modal basis
changes when the spring moves. For each transverse plane the code reconstructs
the physical tine displacement field and weights it with beam quadrature mass,
support translation/rotation inertia, tonebar displacement inertia and the
spring mass at its initial reference position. The same positive reference
metric is used for every comparison in this narrow fixed-geometry pilot.

The squared normalized inner product must be at least 0.98, with runner-up no
greater than 0.05. Invalid scores and ambiguous branch changes are rejected.
The spring advances inward in 0.35 mm increments from 59.5 mm until it brackets
the target, stopping no farther than 35 mm. Frequency must increase along the
tracked branch. Bisection accepts only when both bracket endpoints select the
same branch, with undamped target error below 0.0001 cent. The budgets are 70
bracketing increments and 32 bisections. No audio frequency is fed back into
this structural fit.

Tests recover the planar spectrum twice in the isotropic limit and preserve
polarized frequencies under rotation of the boundary axes. Other tests check
mode tracking from both directions, rejection of invalid/unreachable-direction
targets and changes in the fixed-root second-to-first frequency ratio.

## Output experiment

```text
cargo run --locked --release -p rf-73-lab -- tune-electromechanical references/g3-pitch-reference-validation.json --output renders/NEW.wav
```

The command validates the qualified G3 reference, including its training mean
and held-out consistency, before creating output. A reference larger than 256
KiB is rejected. It preserves existing `NEW.wav`, `NEW-before.wav` and `NEW.json`.
Runtime or qualification failures produce a failed receipt; generated audio
remains available for diagnosis.

Four takes share the same action, excitation and circuit: before and after
tuning, each at 128 and 256 midpoint ticks per 48 kHz frame. Each lasts 2.5 s.
The first key gesture holds from 0.03 to 1.85 s and the second from 2.10 to
2.32 s, with pedestal slew 1.5 m/s and closed pedal. The massive felt and
persistent hammer follow the existing reciprocal action, without strike resets.
The default 10 kohm circuit receives the full spatial motion and reacts on it.

Voltage is averaged to 4x and passed through the common 127-tap FIR, with fixed
gain 0.1 FS/V. Only the fine pair is written as mono float WAV. The initial
preload relaxation remains audible data. There is no normalization, limiter,
resampling-based tuning or artificial partial-frequency adjustment.

Frozen gates:

- Total relative energy defect below 1e-8; exchange and stationary-drive total
  energy growth below 1e-10; monotone electrical heat; at least two hammer
  contacts and output peak strictly between zero and full scale in every take.
- Coarse/fine voltage RMS error below 1% independently in five windows:
  0–0.25, 0.25–0.762, 0.762–1.274, 1.274–1.786 and 1.786–2.5 seconds.
- The established broad-band G3 pitch observer must qualify all three 512 ms
  windows in each fine WAV. The tuned aggregate must be within 5 cents of the
  frozen target. A structural fit alone cannot pass this output check.

The receipt retains all eighteen modal frequencies and ratios, plus the six
fixed-root tine ratios. The existing tone observer compares attack and body
spectra at common key-command anchors, retaining absolute level and harmonic
balance. Its harmonic bins are output observations, not identifications of the
inharmonic structural modes. A common key-command time is not an assertion of
identical physical hammer-contact onset. No timbre-improvement gate is inferred
from a pitch match.

## Retained result

The [receipt](../references/loaded-polarized-spring-tuning-validation.json)
passes both before/after cases and all four takes. The spring center moves from
59.5 to 55.740811 mm on the fixed 70 mm blank. The undamped selected branch
moves from 192.834287 to 196.386153 Hz against the frozen 196.386147 Hz target.

| Audio observation | Before | Tuned |
| --- | --- | --- |
| Qualified output frequency | 192.829360 Hz | 196.380958 Hz |
| Estimated error against target | -31.6421 cents | -0.0457 cents |
| Three-window frequency span | 0.0747 cents | 0.0758 cents |
| Fine output peak | 0.186048 FS | 0.215825 FS |
| Fine output RMS | 0.012812 FS | 0.013314 FS |

These are numerical pitch-observer results, not a claim of 0.05-cent measurement
accuracy or of real-instrument calibration. The source pilot retains its 5-cent
qualification limit and the observer uses interpolated peaks in finite windows.

The maximum voltage refinement error is 0.003402% across all ten windows,
below the fixed 1% limit. Worst relative total energy defect is 1.477e-12 and
exchange defect 4.071e-19. Stationary-drive energy never increases, electrical
heat remains monotone and the coupling solve uses at most three iterations.
Both gestures generate physical hammer contacts in all four takes.

Tuning changes the fixed-root mode ratios from approximately
`1, 6.93809, 19.65156, 37.33398, 60.08540, 90.43704` to
`1, 6.90117, 18.76568, 35.45845, 59.89977, 92.72319`.
Raw attack/body RMS levels change by about +0.38/+0.41/+0.32 dB in the three
tone windows, without level matching. The body harmonic-balance entries are
withheld: the nominal-note harmonic detector does not provide the before-tuning
fundamental needed for normalization. Those nulls are not silence or zero timbre
change. Broad pitch qualification and retained spectral peaks remain separate.
These observations motivate listening to the pair; they do not rank realism.

Retained local artifacts, each WAV 480058 bytes, 120,000 finite samples:

| Artifact | SHA-256 |
| --- | --- |
| `renders/loaded-g3-spring-before.wav` | `d0cc09ba48f4eeb5ecb268a3f78d0bfdc8eb652b5025791e4830265c5a50835b` |
| `renders/loaded-g3-spring.wav` | `26e432dcd8a7363ffe5b635c627699241082a4f6dc8f19c7b945a16828a7b756` |
| Receipt, 269324 bytes | `cac1fca3346a22841c66b9bddfc5366d4a08c7e1ad333cded13db35bcfa10a50` |

The source receipt is embedded, and a live regression replays the structural
fit using the current validator. Only the fine WAV pair is retained; coarse
trajectories are discarded after comparison. No host was opened.

## Limits

This is a single provisional G3 cell and a point-mass spring approximation.
Reference pitch does not identify actual spring geometry or materials. Full
keyboard calibration, magnetic field identification and listening against
recorded sources remain open. Selected temporal refinement does not prove a
complete aliasing bound. The high-resolution solver remains offline and has not
replaced the playable plugin.

The subsequent [recorded-source baseline](LOADED-SOURCE-BASELINE.md) compares
multiple prescribed gestures with the pinned G3 layers. It retains processing
uncertainty and the cold-start preload contribution rather than treating pitch
agreement as a timbre or physical-parameter calibration.
