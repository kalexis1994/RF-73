# Smooth-register pickup calibration

Date: 2026-09-15. **The selected candidate still fails development constraints.**
The new offline register model is implemented and tested; no factory preset or
realtime DSP path was changed. Reserved validation audio was not loaded.

## Model and protocol

The continuous-disk gap, lateral offset and radius now interpolate between two
anchors at MIDI 43 and 72. For each parameter, interpolation takes place in log
space with `w(t) = t²(3-2t)`. Outside these anchors the endpoint geometry stays
constant. This gives a continuous first derivative at the boundaries and avoids
per-note correction tables. Endpoint values obey the existing physical-profile
bounds. Maximum hammer speed and velocity exponent remain shared by all notes.

The search has eight coordinates: the previous five central log parameters and
three geometry half-differences, each bounded to ±0.5. Each note prepares its
own numerical slope table; the existing mechanics, filter and spectral scorer
are reused. Zero half-differences recover the shared-geometry model.

Development remains MIDI 43/47/50/55/59/64/72 × four layers. Acceptance remains
at least 10% lower global harmonic MSE, no note mean regression above 10%, and
no E4/f or B2/mp regression above 10%. The normalized constraint violation must
be at most 1.0 before held-out audio is loaded. These are engineering gates;
the harmonic score is not a perceptual quality measurement.

The coarse phase used seven seeds and three coordinate rounds at log steps
0.16/0.08/0.04: 55 evaluations. A development-only refinement used its winner
and steps 0.02/0.01/0.005/0.0025: 65 evaluations. The second schedule was fixed
before running it. The **120 evaluations** include the repeated seed. This is
a bounded local search, not evidence that all smooth-register models fail.

## Results

Normalized constraint violation falls from the shared model's 1.1577 to 1.1106
after the coarse phase and **1.0683** after refinement. It still exceeds 1.0.

| Measure | Current Calibrated MSE (dB²) | Register candidate | Change |
| --- | ---: | ---: | ---: |
| E4/f critical case | 179.63 | 119.61 | -33.41% |
| B2/mp critical case | 19.75 | 20.95 | +6.06% |
| B2 note mean | 98.76 | 115.81 | **+17.26%** |
| D3 note mean | 25.55 | 29.94 | **+17.16%** |
| G3 note mean | 66.13 | 77.71 | **+17.51%** |
| Global development mean | ratio 1.0 | ratio 0.94574 | -5.43% |

Both critical cases now satisfy their individual limits, but the three note
means above still fail, and global improvement remains below 10%. The search
therefore stops before held-out or long-envelope validation. It makes no claim
of a verified audible improvement or of improved sustain.

Selected endpoint geometry:

| Parameter | MIDI 43 and below | MIDI 72 and above |
| --- | ---: | ---: |
| Gap | 0.856415 mm | 0.960789 mm |
| Offset | 1.020201 mm | 0.980199 mm |
| Radius | 0.705995 mm | 0.555355 mm |

Shared maximum hammer speed is 1.447740 m/s; velocity exponent is 1.224460.
These are model fit variables, not measured properties of the source instrument.
The original performance labels remain ordinal, with unknown strike speeds.

## Numerical and software checks

The selected geometry at **all 73 model notes** passes the existing slope-table
check against direct boundary integration at 2,001 off-grid displacements per
note. The largest table error / peak slope is 4.1819e-6, below 1e-4. This is a
numerical geometry check without source audio; it does not qualify all-key audio,
aliasing, CPU use, or physical realism.

Two new tests verify recovery of the shared profile, constant outer registers,
bounded monotone interpolation and absence of large adjacent-note jumps in a
stress geometry. Shared constraint and analysis tests remain active. Validation:
**27 Rust tests and six Python gate tests pass**, formatting passes, and release
Clippy passes with warnings denied.

The [retained receipts](../references/register-pickup-fit-2026-09-15/) contain both
search histories, full selected development observations, every note's numerical
qualification, decisions and provenance. No WAV matrix was generated.

## Reproduction

From the workspace in PowerShell, using new output directories:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$runner = 'references/matts-reference-runner/Cargo.toml'
$samples = 'references/audio/matts-fender-rhodes/samples/original'
$seed = 'references/robust-pickup-fit-2026-09-15/refined/search.json'
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --register $samples renders/matts-reference/register-new $seed
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --register $samples renders/matts-reference/register-refined-new renders/matts-reference/register-new/search.json
```

A prior register receipt selects fine refinement; a prior robust shared-geometry
receipt selects the coarse stage. Both require `held_out_loaded: false`. Passing
development and numerical checks would trigger the existing split evaluation;
it would still require the held-out decision, numerical render checks and the
standard release workflow before becoming an audition version.

## Remaining work

Completed follow-up: [ordinal velocity mapping sensitivity](VELOCITY-MAPPING-SENSITIVITY.md)
compares 37,034 mappings with both model families frozen. It reduces some
residuals but does not pass the development gates; reserved notes stay unused.

The reserved notes MIDI **45/53/60/67/76/86** remain unused by these fits. Before
another geometry search, examine the residual spectra in B2/D3/G3 and whether
the fixed ordinal velocity pairing can account for them. The current results
do not justify adding independent corrections for individual keys or relaxing
the acceptance thresholds. No new playable version was produced, so RackForge
audition was not invoked.
