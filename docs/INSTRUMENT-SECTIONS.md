# RF-73 instrument sections

## Reference: RF Concert Grand

Reviewed RackForge's `plugins/concert-grand`: the PLAY surface, parameter
metadata and model controls in `src/lib.rs`. Concert Grand separates everyday
voicing from Room & Microphones, Action & Noises, laboratory controls and
individual model subsystems. RF-73 adopts the subsystem organization while
keeping its controls tied to the implemented electric-piano model.

The relevant RF-73 signal path is:

```text
MIDI touch -> hammer contact -> tine / tonebar motion -> magnetic pickup -> output
                              ^
                         key / damper / pedal
```

## First implemented structure

The same four sections are used by the PLAY surface, RackForge parameter
metadata and the native preset editor. Existing parameter indices, field IDs,
values, preset IDs and saved state are unchanged.

| Section | Existing controls | Meaning and limits |
| --- | --- | --- |
| Hammer & Touch | Hammer Hardness; Dynamics | Contact stiffness and MIDI-velocity-to-hammer-speed mapping. Dynamics is a performance mapping, not a separate physical component. |
| Tine & Tonebar | Sustain; Bell | Sustain currently couples tine and tonebar decay. Bell changes excitation of the second bending mode; it is not a bell oscillator or a proven one-to-one tonebar adjustment. |
| Pickup | Pickup Distance; Tine Alignment; Pickup Model | Base gap, lateral offset and transfer model. Register Aperture includes the frozen upper-register geometry. |
| Output | Output Gain | Final gain after shared pickup compensation. No implied amplifier, cabinet, limiter or room simulation. |

The PLAY page uses four accessible tabs with one visible panel, starting in
Hammer & Touch. Switching panels does not navigate to a fragment or scroll
the page. Left/Right arrows, Home and End move between tabs; Tab enters the
visible controls. The native editor presents the same sections as pages. The pickup
model choice remains available but is placed after the physical position
controls. Original preset/state behavior is preserved.

## Next controls, ordered by physical usefulness

1. **Separate resonator losses.** Split tine sustain from upper-mode decay;
   migrate the existing coupled Sustain parameter explicitly, keeping old
   presets identical. Do not silently change what its automation means.
2. **Dampers & Pedal.** Expose validated release damping and contact behavior.
   The current keyboard/pedal mechanism already operates, but there is no
   editable release control or validated continuous half-pedal control in the
   playable preset. Add a section only with working controls.
3. **Hammer voicing.** Consider strike speed/range and effective contact
   parameters after checking response and output across velocities. Hardness
   alone is not all hammer-tip material behavior.
4. **Instrument setup.** Tuning and bounded register voicing belong here once
   they are implemented and persist independently of the preset model choice.
   Per-key tuning is not currently exposed by the eight-parameter interface.
5. **Amplification & Effects.** A future electrical signal chain can contain
   preamp, tremolo and cabinet. Keep it distinct from tine/pickup physics and
   make bypass explicit. These blocks are not implemented by this refactor.

Only add an advanced Model section when useful controls exceed the musical
pages. Raw solver constants, numerical quadrature settings and fit objectives
belong in research tools, not in the instrument's playing interface.

## Why not copy the piano pages directly?

Concert Grand's strings, unison, soundboard and microphone controls correspond
to its model. RF-73 currently models a struck resonator read by a magnetic
pickup; the matching user concepts are hammer contact, resonator behavior,
pickup placement and electrical output. A microphone or room section would
require an actual amplifier/acoustic output stage rather than a renamed DSP
constant.

This change reorganizes existing controls and clarifies their descriptions.
It introduces no DSP change and does not claim a new audible improvement.

Version 0.1.5 gives PLAY a brushed-metal nameplate, dark control panel,
wood-tone side rails and metal faders. Both host lighting modes and narrow
layouts remain supported. Tab behavior is implemented in Rust alongside the
existing host protocol; no DSP or state format change is introduced.
