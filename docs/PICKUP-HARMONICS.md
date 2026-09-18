# Pickup harmonic generation

The [playable G3 diagnostic](PLAYABLE-G3-DIAGNOSTIC.md) found the plugin's
engine far too dark at loud dynamics: the recording's third harmonic sits
7 dB above its fundamental while the engine's sits 12 dB below it. This
study asks whether any of the retained pickup transfer laws can generate the
recorded harmonic balance from a sinusoidal tine motion, at which geometry,
and at which amplitude, with no mechanics, circuit or audio involved.

```text
cargo run --locked --release -p rf-tines-lab -- pickup-harmonics --output REPORT.json
```

The command writes one JSON receipt at a new path. It adopts nothing.

## Method

Three flux laws are driven with `x(t) = A sin(ωt)` at 196 Hz over one period
sampled at 4096 points: the production law `(1+z²)^(-1/2)`, the point-pole
proxy `(1+z²)^(-3/2)` with `z = (offset + x)/gap`, and the laboratory's
finite-aperture 16-node flux with a pole radius. The induced voltage is the
time derivative of the flux, so its n-th harmonic is n times the n-th flux
Fourier coefficient; only ratios to the fundamental are compared, and flux
scales are arbitrary. A centered pickup, whose flux is an even function of
an odd motion, outputs only even harmonics, the octave; a unit test holds
that and the return of the fundamental with a lateral offset.

Targets are the recordings' harmonic levels relative to their fundamental in
the 96 ms attack window of the diagnostic: loud layer 1 at harmonics 2 to 5
(+2.4, +7.0, −9.2, +1.8 dB), medium layer 3 (−17.4, −13.6, −17.3, −24.6 dB)
and soft layer 5 at harmonics 2 and 3 (−21.7, −41.6 dB). The sixth harmonic
is excluded because at G3 it coincides with the tine's second bending mode.
For every geometry on a grid of six gaps, six offsets and, for the aperture
law, four pole radii, each dynamic's amplitude is chosen freely on a
61-point geometric grid from 0.02 to 3 mm and refined within 10% to minimize
the RMS dB error; the geometry is shared across dynamics and the combined
error is the RMS over the three. Every geometry's harmonics at the engine's
own traced amplitudes, 0.807, 0.397 and 0.117 mm at velocities 1.0, 0.6 and
0.25, are retained as well.

## The transfer model explains the engine

Before fitting anything, the production law at the plugin's default geometry
(gap 1.5 mm, offset 0.5 mm) was evaluated at the engine's traced amplitudes:

| Dynamic | Engine measured in the diagnostic, H2 / H3 / H4 / H5 (dB) | Static transfer prediction |
| --- | --- | --- |
| Loud | −3.3 / −12.4 / −32.8 / −32.9 | −3.5 / −12.6 / −33.2 / −33.3 |
| Medium | −10.6 / −24.5 / −56.2 / −57.6 | −10.7 / −24.6 / −56.4 / −57.5 |
| Soft | −21.6 / −45.7 | −21.7 / −45.9 |

The worst difference is 0.4 dB. The engine's harmonic content is entirely
the static pickup transfer acting on a nearly sinusoidal tine, and its
darkness is a matter of geometry and amplitude: at loud the tine swings
0.54 of the gap, which the production law barely bends.

## A geometry that reproduces the recordings at the engine's amplitudes

| Law | Best shared geometry | Combined error (dB) | Fitted amplitudes loud / medium / soft (mm) | Loud amplitude over engine's |
| --- | --- | ---: | --- | ---: |
| Finite aperture, 16 nodes | gap 0.5 mm, offset 0.5 mm, pole radius 2 mm | 2.36 | 0.841 / 0.408 / 0.104 | 1.04 |
| Point pole | gap 1 mm, offset 0.75 mm | 2.64 | 1.758 / 0.766 / 0.192 | 2.18 |
| Production | gap 0.5 mm, offset 0.75 mm | 3.11 | 1.758 / 0.759 / 0.184 | 2.18 |

All three laws can reach the loud balance, with the third harmonic above the
fundamental, but the two point laws need the tine to swing more than twice
as far as the engine's does. The finite-aperture law with a wide pole close
to the tine reaches the balance with amplitudes within 4%, 3% and 11% of the
engine's own, and its fitted loud-over-soft amplitude ratio of 8.1 is close
to the engine's 6.9. At the engine's amplitudes, unfitted, it gives:

| Dynamic | Aperture law at engine amplitude, H2 / H3 / H4 / H5 (dB) | Recording |
| --- | --- | --- |
| Loud | −3.1 / +8.1 / −7.5 / −1.1 | +2.4 / +7.0 / −9.2 / +1.8 |
| Medium | −14.6 / −11.5 / −17.0 / −29.7 | −17.4 / −13.6 / −17.3 / −24.6 |
| Soft | −19.2 / −40.1 | −21.7 / −41.6 |

The remaining errors are 2 to 5 dB on individual harmonics, with the loud
second harmonic 5.5 dB low. A pole radius of 2 mm is the grid's largest
value and the gap its smallest, so the optimum sits on the grid edge in both
and is not a continuous identification; a Rhodes pickup's pole face is a few
millimetres across, so the shape is plausible while the numbers remain
provisional.

## Reading

The plugin engine's darkness is not a missing mechanism. Its tine already
moves the right amount; its pickup law and geometry turn that motion into
too few harmonics. The laboratory's finite-aperture law, at a close gap and
a wide pole, turns the same motion into the recorded balance across three
dynamics within a few decibels, and it does so with velocity-dependent
brightness that the production law cannot supply at these amplitudes. The
[Close Aperture path](PICKUP-APERTURE-PATH.md) puts that law and geometry
into the playable engine as a fourth selectable pickup, matches its level
and measures the G3 strikes and the nocturne through it.

## Scope

A static transfer characterization: one frequency, one lateral coordinate,
sinusoidal motion of constant amplitude, targets from one processed
recording per layer with unknown capture gain, and a grid whose best
aperture geometry lies on two of its edges. The engine's amplitudes come from
a single trace window 50 to 100 ms after the strike. Nothing here changes the
plugin or the laboratory model.

The [receipt](../references/pickup-harmonics-validation.json) is
818179 bytes, SHA-256
`e0a42dcbbbcc4387cc1bd916611fa95a46e7b38b6395183757b71a36d692aeaa`. A receipt test holds the target
provenance against the diagnostic reports, the 0.4 dB agreement between the
default geometry's prediction and the diagnostic's engine measurement, and
the aperture law's best geometry, error and amplitude agreement.
