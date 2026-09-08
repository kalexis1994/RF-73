# The Sound page

The plugin is now voiced like an instrument rather than compared like a
laboratory. Its parameters are physical quantities of the model, every one
mapped onto the `Profile` the engine runs, with the pickup's level
compensated so the controls change the timbre and not the loudness. The
pickup A/B laboratory of 0.1.2, documented in [Pickup Lab UI](PICKUP-LAB-UI.md),
is retired from the plugin; the four-path engine and its receipts remain in
the DSP crate and the offline laboratory.

## Parameters

| Index | Control | Range | Default | Physics |
| --- | --- | --- | --- | --- |
| 0 | Output Gain | 0 to 2 x | 0.100 | Output multiplier after the level compensation; no limiter |
| 1 | Pickup Law | Production, Aperture | Production | The original flux surrogate or the laboratory's finite aperture with a 2 mm pole |
| 2 | Pickup Distance | 0.5 to 3 mm | 1.5 | The gap |
| 3 | Tine Alignment | −1 to 1.5 mm | 0.5 | The lateral offset; on centre the fundamental thins |
| 4 | Hammer Hardness | 0 to 1 | 0.5 | Contact stiffness 4e10 × 25^(2h − 1) N/m², 1.6e9 to 1e12 |
| 5 | Sustain | 0 to 1 | 0 | First partial T60 at A3 of 5 × 16^s seconds, 5 to 80 s; the bar partial 0.16 × 14.375^(2s) capped at 10 s |
| 6 | Bell | 0 to 1 | 1 | Second partial strike weight −0.3 × b² |
| 7 | Dynamics | 0 to 1 | 0.5 | Velocity exponent 1.4 × 2^(2d − 1), 0.7 to 2.8 |

The defaults reproduce the retained 0.1.2 engine exactly; a unit test holds
that the default settings map to `Profile::default` field for field. Sustain
at 0.5 and Bell at 0.258 reproduce `Profile::calibrated` within rounding.
The distances and the aperture pole radius are in the model's own units;
the unit controls are logarithmic in the quantity they scale, so the
original sits at a round value in each.

Every change goes through `Engine::set_profile`, which keeps every modal and
hammer state and applies the new frequencies, losses, strike weights,
contact law and pickup from the next sample, and through the level
compensation of the [voicing physics](VOICING-PHYSICS.md), which the plugin
enables at preparation and which follows a profile change with the gain's
5 ms smoothing. A held note carries across any control, including the law.

## Presets

Four factory presets replace the former pickup slots and profile parameter:
Original (the defaults), Close Original (production law at 0.5 / 0.25 mm),
Close Aperture (aperture law at 0.5 / 0.5 mm) and Calibrated (Close Aperture
with Sustain 0.5 and Bell 0.258). Loading a preset sets the eight parameters
and keeps ringing notes. Custom programs, up to eight per instance, seed
from the current settings or from a factory preset and carry the whole page.

## Surfaces

The PLAY panel shows the seven Sound controls as sliders with their values
in millimetres or thousandths, the law as a selector and the output gain as
before; a slider being dragged keeps its own value until the host answers,
as the gain field did. The controller editor, **RF-73 Voicing**, shows the
same eight fields with live preview, the distances in hundredths of a
millimetre, the unit controls in thousandths and the gain in millionths.

## State

Schema 4 stores `RFRH`, version 4, seven f64 values in parameter order
without the law, then the law byte and three zero bytes, 68 bytes. Schema 3
and 2 snapshots load onto the voicing they were listening to: the selected
path becomes law and geometry (Current to production 1.5 / 0.5 mm, both
close production paths to 0.5 / 0.25 mm, Close Aperture to the aperture law
at 0.5 / 0.5 mm) and the profile index becomes Sustain and Bell; schema 1
keeps its gain. As before, the pinned host rejects preset references from
another plugin version or schema before the processor sees them.

## Scope

The mappings are design choices that place the retained and calibrated
values at round positions; none is a measurement. The level compensation
equalizes a medium note, so centred pickups play softer soft notes and
louder loud notes by the law's own physics. The aperture law costs one flux
slope of ten terms per voice per oversampled tick, a fraction of the retired
four-path engine. The gain policy is unchanged and still open; no limiter
is added.

Tests: the plugin's settings test holds the default mapping and the
presets' validity and profile ranges at every parameter extreme; the
program tests hold editor round trips for all eight fields, preset loading
and seeding, atomic rejection of invalid parameters, payloads and edits,
schema 1 to 4 loading with the documented mappings and every corrupted
schema 4 field, catalog bounds, and audio continuity across changes of every
control while a note rings; the contract tests hold block-size invariance
with law, geometry and loss automation; the UI client tests hold the eight
parameter domains.
