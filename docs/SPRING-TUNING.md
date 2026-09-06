# Coupled spring-position tuning

This offline G3 experiment varies the tuning mass position on a fixed 70 mm
provisional tine. It produces a before/after listening pair and retains
mechanical, sampling, pitch and modal-ratio evidence. It does not change the
plugin engine or identify an actual instrument's geometry.

## Physical control and limits

The original [service manual, Chapter 5](https://fenderrhodes.com/org/manual/ch5.html)
describes the tuning spring as a counterweight: moving it toward the free end
lowers pitch, and moving it toward the root raises pitch.
[Chapter 6](https://www.fenderrhodes.com/org/manual/ch6.html) treats cutting a
replacement tine to length and sliding its spring for final tuning as separate
operations, and calls for timbre/volume adjustment and a pitch recheck.

Sliding that mass does not literally change the beam's free length. A uniform
unloaded Euler-Bernoulli beam scales all modal frequencies with inverse length
squared, preserving their ratios. The spring's moving mass distribution changes
both frequencies and their ratios, plus the spatial weights that couple hammer
and pickup to each mode. Root/tonebar coupling and magnetic conversion further
separate mechanical resonances from observed audio partials.

RF-73 currently represents the spring as a 0.1 g point mass on a uniform beam.
Reported millimeters locate this surrogate mass center from the fixed root;
they are not measured coil edges or a calibrated service recommendation. Coil
width, rotary inertia, local stiffening and real tine taper remain unmodeled.

The previous 75 mm blank reaches only about 188.57 Hz with its mass at the
mathematical root endpoint, below the frozen 196.386 Hz G3 target. The present
70 mm length is an explicit coarse modeling assumption, not an inferred real
G3 dimension. The listening pair holds it fixed and moves only the spring.
The search is restricted to centers 35..59.5 mm from the root and starts at
59.5 mm. These are experimental bounds, not verified hardware travel limits.

## Reproduction

```powershell
$env:CARGO_INCREMENTAL = '0'
cargo run --locked --release -p rf-73-lab -- tune-modal-pitch references/g3-pitch-reference-validation.json --output renders/g3-spring-tuned.wav
```

The output paths must be new. This writes the tuned WAV, a `-before.wav` sibling,
and one JSON report. Both WAVs are 2 seconds of mono float32 at 48 kHz, with a
0.4 m/s memory-hammer launch, damper engagement at 1.85 s and common gain 0.084.
Neither take is normalized or level matched. No device or host is opened.

## Structural branch and independent audio checks

`ModalSpectrum` solves the generalized eigenproblem of the exact mass/stiffness
operators used by the time-domain assembly. It exposes all nine coupled modes,
mass orthogonality, residuals and hammer/pickup projections. Regression tests
check static compliance reconstruction, independent lossless time propagation,
free-support rigid modes and the direction of spring tuning.

The search selects the dominant first-tine coordinate initially, then follows
physical displacement fields. Reusing modal coefficient vectors would be
incorrect because the fixed-root basis changes with the spring position. A
fixed positive reference inertia includes distributed beam mass, support,
tonebar and the spring at its initial position. Four-point Gauss integration
per cubic finite element evaluates the field products exactly. The squared
normalized overlap must remain at least 0.98, and the runner-up at most 0.05.
Both bracket endpoints must identify the same mode at each bisection step.
The undamped search tolerance is 0.0001 cent; this is solver precision, not a
claim of physical or measured accuracy.

Both complete nonlinear, damped renders must independently pass the existing
memory-hammer mechanical ledgers, time-step refinement, section-wise 16/32/64x
sampling and headroom checks. Pitch is then measured from the actual float32
quantization using the independent broad-band three-window anchor. The tuned
output must qualify and be within 5 cents of the frozen reference. A failed
gate retains the report and emits no listening WAVs.

The report includes the six fixed-root frequency ratios before/after, all nine
coupled frequencies by rank, raw attack-band powers and 10 ms RMS envelopes.
Only the selected coupled branch is explicitly tracked; frequency ranks do not
assert modal identity across every other branch. Mechanical ratios are not
harmonic-amplitude estimates, and numerical pass/fail does not assess timbre.

## Recorded result (2026-09-06)

The [compact validation receipt](../references/g3-spring-tuning-validation.json)
retains settings, checks, modal tables and SHA-256 digests of both WAVs, the
frozen reference receipt and the complete local report. The full report is
`renders/g3-spring-tuned.json` (about 1.2 MB); the pair occupies about 0.77 MB.

| Observable | Before | After |
| --- | ---: | ---: |
| Spring center from root | 59.500000 mm | 55.740822 mm |
| Coupled undamped selected mode | 192.834287 Hz | 196.386143 Hz |
| Measured float32 output component | 192.827431 Hz | 196.378585 Hz |
| Fixed-root second/first mode ratio | 6.938086 | 6.901167 |
| Fixed-root third/first mode ratio | 19.651561 | 18.765687 |
| Sample peak | 0.359787 | 0.406012 |
| Whole-take RMS | 0.030684 | 0.031807 |

The output error against the frozen target is -0.066667 cent. The second/first
mechanical ratio changes by -0.532%, the third/first by -4.508%. The latter is
not negligible and remains a property of this provisional model, not a measured
real-instrument effect. Pitch accuracy does not establish timbral accuracy.
Both takes pass their independent mechanical/refinement/sampling/headroom
checks and independent WAV inspection. Their higher modes and attack spectra
are explicitly retained for follow-up rather than accepted by a pitch-only fit.

## Listening and next experiment

Compare the pair at unchanged playback volume, attending to attack brightness,
bell components, beating and the decay/release character. The audible pitch
change itself can bias timbre judgments. Human listening remains pending; these
files are prepared for evaluation, not an accepted realism result.

The next modeling step is to compare resolved modal/partial structure and
spatial weights after tuning before fitting losses or excitation. The reused,
processed G3 recordings do not supply independent geometry, calibrated strike
speed, capture gain or known note-off. Preserve those uncertainty limits.
