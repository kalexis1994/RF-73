# Physics for continuous voicing

Voicing controls in the plugin, pickup distance, tine alignment, hammer
hardness, sustain, bell and dynamics, need three things the engine did not
have: the pickup law as a property of the voice rather than of a laboratory
path, a velocity curve that is a parameter, and a level compensation so that
moving the pickup changes the timbre and not the loudness. This block adds
those to the DSP core and measures the compensation over a grid of
geometries. No plugin control changes yet; the defaults are byte-identical.

```text
cargo run --locked --release -p rf-73-lab -- voicing-level --output REPORT.json
cargo run --locked --release -p rf-73-lab -- render --output g3.wav --note 55 --velocity 0.6 --law aperture --gap-mm 0.5 --offset-mm 0.5 --sustain calibrated --bar-strike -0.02 --compensate
```

## What the profile carries now

`Profile` gains `pickup_law`, Production or Aperture, `pickup_pole_radius_m`
for the aperture law and `velocity_exponent`, the power the hammer speed
follows (1.4 as before). A voice with the aperture law builds its own
`AxialAperture` from its gap, offset and pole radius, so the law that the
Close Aperture laboratory path evaluates on the side is now available as the
voice's own pickup at any geometry; a test holds that the two give the same
voltage sample for sample at the path's geometry. `Engine::set_profile`
rebuilds the pickup with the rest, so all of this can move while notes ring.
The four-path laboratory engine keeps the production law under its frozen
level factors and rejects the aperture law, as it rejects moved geometry.

## Level compensation

The reference motion is a sine of 0.4 mm at 196 Hz, the engine's traced G3
swing at velocity 0.6. `Profile::pickup_sensitivity` is the RMS voltage of
the profile's law and geometry under that motion, sampled over one period at
256 points, and `Profile::level_compensation` is the default profile's
sensitivity over the profile's own. When enabled with
`Engine::set_level_compensation`, the engine multiplies its output by the
compensation, smoothed with the same 5 ms constant as the gain, and jumps to
it on reset; it is off by default, so the raw engine keeps its retained
output at every geometry, and for the default pickup the ratio is exactly
one in any case. The renderers apply it with `--compensate` and report it
either way.

The compensation equalizes one motion, not the instrument: a law that is
more nonlinear at a geometry still plays soft notes softer and loud notes
louder relative to the default, and that difference is the timbre the
control is meant to reach.

## Measured over the grid

`voicing-level` renders one-second notes with the default mechanics for both
laws over six gaps, 0.5 to 3 mm, and six offsets, 0 to 1.5 mm, each engine
carrying its own compensation, and reports every note's RMS against the
default pickup's. The reference note, G3 at velocity 0.6, holds within
2.3 dB in all 72 cells and within 1 dB in every off-centre production cell.
D3 at 0.6 holds within 0.5 dB everywhere; B3 drifts up to 4.4 dB low at
centred pickups because its tine swings less than the reference amplitude
and a centred pickup's output grows with the square of the swing. The same
physics shows in the dynamics: with the offset at zero the soft note falls
9 to 13 dB below the reference and the loud note rises 2 to 6 dB above it,
at every gap and both laws, which is why a real instrument is voiced with
the tine off centre. Off-centre cells keep the soft and loud notes within
about 2 dB of the default's spread, so a voicing control over gap and
offset will not change the dynamic range unless it crosses the centre.

The compensation itself ranges from 0.29 at a close, slightly offset
production pickup to 8.5 at a 3 mm gap on centre, and up to 14 for the
aperture law near a 1 mm gap on centre, where the node ring cancels the
reference swing; the Close Aperture geometry's compensation of 2.62 sits
within 5% of the laboratory path's frozen factor 2.4996, derived from the
whole 24-second performance. That agreement is the check that the
reference motion stands for a musical level.

## Scope

One reference motion, one register anchor and one-second isolated notes of
the default mechanics; no loudness weighting. The gate qualifies the
reference note only; the other notes and dynamics are reported as the law's
amplitude dependence, not as errors. Nothing here changes the plugin's
parameters, its default profile or the laboratory paths.

The [receipt](../references/voicing-level-validation.json) holds all 72
cells with their sensitivities, compensations and five note levels. A
receipt test holds the reference cell's exact zero, the reference note's
bound in every cell, the centred cells' dynamic spread, the monotone
compensation over the gap and the aperture geometry's agreement with the
frozen factor; DSP and CLI tests hold the new fields, the aperture voice's
identity with the laboratory path, the compensation's smoothing and the
renderer options.
