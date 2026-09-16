# RF-73 player panel and programs

Research date: 2026-09-16. Original design proposal; see the implementation update below.

## Implementation update: 0.1.7

The player panel now switches between Stage (Volume, Bass Boost) and Suitcase
(Volume, Bass, Treble, Vibrato On/Off, Speed, Intensity). Programs follow the
user-requested instrument-family and era direction: early-1970s Stage,
mid-1970s Suitcase, late-1970s Stage/Suitcase and 1980s Stage. Each changes
physical-model parameters as well as electronics. These are original RF-73
approximations, not measured replicas of individual instruments. The earlier
musical-character program proposal below is superseded.

See [Instrument sections](INSTRUMENT-SECTIONS.md) for implemented parameters,
program values and compatibility. Comparative listening against historical
instruments is still needed; automated DSP checks do not establish fidelity.

## Decision

Use a Suitcase-inspired **Instrument** page as the default player surface.
Separate player electronics, internal voicing and controller setup. A program
is a complete RF-73 sound configuration, not a claim that another historical
piano has been modeled. Preserve the current visual language: brushed metal,
dark instrument case, amber markings and restrained wood accents.

## Hardware evidence

Original owner guides distinguish the simple Stage Volume/Bass Boost panel
from the Suitcase Volume, Bass/Treble and Vibrato Speed/Intensity controls.
These are different electronics packages; averaging them into a supposedly
universal historical panel would obscure that distinction.

Sources: [Suitcase owner control panel](https://www.fenderrhodes.com/img/service/guides/suitcase-mark1/p2.jpg),
[Stage owner operation](https://www.fenderrhodes.com/img/service/guides/stage-mark2/p5.jpg),
[original owner-guide archive](https://www.fenderrhodes.com/service/guides.html).

The service manual separates pickup spacing/alignment, hammer-tip hardness,
escapement, striking line and damper adjustment from front-panel operation.
Pickup spacing affects level and dynamic response. These maintenance and
voicing adjustments justify separate internal-editing pages. The historical
dimensions are not automatically safe or correct ranges for our simplified
model; retain its validated bounds until separately tested.

Source: [1979 service manual, chapter 4](https://www.fenderrhodes.com/org/manual/ch4.html).

The modern MK8 adds Drive, an envelope-controlled filter, three-band active EQ
and Vari-Pan. Optional processing includes compressor, phaser, chorus and
delay. These provide a legitimate future expanded panel, but do not establish
that those controls existed on a vintage Stage or Suitcase.

Source: [Rhodes MK8 specifications](https://rhodesmusic.com/rhodes-mk8-specifications/).

Historical context: the Suitcase effect named Vibrato modulates amplitude;
stereo versions alternate the channels. It is not pitch vibrato. Preserve the
familiar name with a clear subtitle: "Stereo tremolo". External amplifier and
pedal coloration should remain explicitly separate from mechanical voicing.

Source: [Rhodes Super Site effects history](https://www.fenderrhodes.com/history/effects.html)
(secondary historical commentary).

## Page structure

| Page | Controls | Implementation status |
| --- | --- | --- |
| Instrument | Volume, Bass, Treble; Vibrato On, Speed, Intensity | Volume exists. EQ and stereo modulation need new DSP. |
| Hammer | Hammer Hardness | Existing contact-stiffness mapping. Future strike-position and escapement controls require validated mechanics. |
| Resonator | Sustain, Bell | Existing coupled losses and upper-mode excitation. Later split losses without redefining existing automation. |
| Pickup | Distance, Alignment; advanced Pickup Model selector | Existing geometry and transfer laws. Keep algorithm selection visually secondary. |
| Setup | Touch/Dynamics; eventual controller calibration | Existing velocity mapping. This is controller setup rather than instrument mechanics. |

Sustain on the Resonator page means decay duration, not sustain-pedal amount.
Bell is a modeling macro, not a documented physical front-panel knob. A pedal
indicator can display the real pedal state once the host supplies it; do not
invent an independent control that disagrees with MIDI.

### Initial Instrument controls

These ranges are RF-73 engineering proposals, not measured vintage circuit
specifications. Circuit emulation would require schematic analysis and response
validation beyond this control-layout research.

| Control | Proposed presentation | Behavior |
| --- | --- | --- |
| Volume | Knob with explicit level readout | Preserve parameter 0 and its saved values; do not change its gain law silently. |
| Bass | Center-detented knob, -12 to +12 dB | New low shelf; neutral at 0 dB. Choose corner frequency by response/listening tests. |
| Treble | Center-detented knob, -12 to +12 dB | New high shelf; neutral at 0 dB. |
| Vibrato | Dedicated On/Off switch | Bypass independent of remembered speed/intensity. |
| Speed | Knob, 0.5 to 12 Hz | Logarithmic frequency mapping; range subject to audition. |
| Intensity | Knob, 0 to 100% | Linked alternating channel gains; define depth, loudness and mono-sum behavior deliberately. |

Use five separate knobs rather than small concentric controls. Do not add Drive
until saturation exists. Keep Mid, Envelope and modulation waveform options for
a later Modern/Effects expansion. Tempo sync is a useful software extension,
but requires a verified host tempo API and should not masquerade as vintage hardware.

Proposed initial signal path: physical model -> compensated pickup -> neutral
EQ -> stereo tremolo -> clean output level. Validate output headroom with both
shelves boosted. Smooth all continuous electronic changes; avoid clicks on
bypass and program changes. Preserve stereo output through the host.

## Factory program brief

Start with eight distinct targets. These are original sound-design briefs,
not historical factory presets, measured settings or finished audio results.
Do not attach artist names or exact model years without reference validation.

| Program | Intended sound/use | Main design levers | Dependency |
| --- | --- | --- | --- |
| Studio Clean | Balanced dry starting point | Calibrated Register baseline; neutral electronics | Current engine |
| Warm Ballad | Rounded attack, gentle upper partials, lingering chords | Softer hammer, reduced Bell, moderately longer decay | Current engine; optional treble cut later |
| Velvet Touch | Intimate, subdued and less ringing than Warm Ballad | Soft contact, low Bell, shorter decay | Current engine |
| Bell Light | Clear melodic attacks and upper-register definition | Moderate Bell increase and firmer contact | Current engine |
| Dry Bark | Percussive comping with pronounced velocity contrast | Pickup alignment/spacing, firmer contact, shorter decay | Current engine; audition whether it reaches the intended bark |
| Open Tines | Exposed resonant tone for sparse playing | Longer decay and restrained upper-mode excitation | Current engine |
| Slow Suitcase | Warm chords with slow stereo movement | Warm voicing, mild tone shaping, slow/moderate-depth tremolo | New EQ and stereo tremolo |
| Bright Suitcase | Clear rhythmic playing with stronger movement | Firmer voicing, modest treble lift, faster tremolo | New EQ and stereo tremolo |

Do not prescribe arbitrary exact parameter vectors from manuals: finish them
by listening across the keyboard and multiple velocities. Compare at matched
perceived level, retain headroom for dense pedal chords, and verify mono output.
Avoid changing the user's keyboard response just to make a preset sound louder.
Future programs may use genuine Drive, Phaser or Chorus after those processors
exist; changing pickup distance is not a substitute for implementing overdrive.

Keep the five existing preset IDs and values intact under a **Reference** bank.
New musical programs belong to **Factory**; host-managed user programs belong
to **User**. Metadata and plugin preset implementations must agree.

## Program sidebar and responsive layout

Breakpoints refer to the plugin iframe's available width, not monitor width.
Use container sizing where practical. The host already consumes sidebar space.

- At approximately 960 px and above: persistent left program column around
  200-220 px, flexible instrument area with `min-width: 0`.
- Between approximately 640 and 959 px: compact selected-program bar with
  previous/next and a button opening the program list. Keep room for useful knobs.
- Below approximately 640 px: the list opens as an accessible drawer; controls
  form two columns, falling to one only when necessary. Never scale the entire
  interface down to fit.
- Program list: bank filters, selected state, concise description, search once
  the catalog warrants it. Keep selection when switching tabs. Show modified
  state and provide host-backed Save As for user programs when supported.
- Five tabs can wrap into rows; they must not become anchor navigation or move
  the page scroll. Switching to a shorter panel should not collapse the frame
  around the pointer. Avoid oversized fixed minimum heights on mobile.
- Knobs: vertical drag, keyboard arrows, fine adjustment, reset gesture and
  editable numeric values. Prefer semantic range inputs styled as knobs. Use
  at least 44 px touch targets, visible focus and legible labels. Wheel changes
  must not accidentally alter parameters during page scrolling.
- Drawer: focus entry/return, Escape to close, no background focus while modal.
  Handle safe areas, text zoom and long user-program names.

Validate at 320, 390, 640, 800 and 1200 px available widths, day/stage themes,
200% text zoom, keyboard-only input, touch gestures, and inside actual RackForge.
No horizontal document overflow or invisible program-selection feedback.

## Integration and delivery order

Local review: `crates/rf-73-plugin/src/settings.rs` defines eight parameters
and five presets. The UI client currently supports parameter fetch/set only.
Concert Grand loads factory voices through `plugin.select_sound` with a
`sound_id`, documented in RackForge's `docs/WEB_PLUGIN_API.md`.

1. Add program selection to the Rust/WASM UI client using the host API; preserve
   request sequencing and refresh the full parameter snapshot after selection.
   Handle host-originated sound changes, busy states and failures. Do not copy
   Concert Grand's local user-preset storage as a second authoritative library.
2. Implement the responsive sidebar/drawer, knob interaction and page structure.
   Existing controls can ship first; do not expose inactive EQ/tremolo knobs.
3. Add EQ and stereo tremolo, appending parameter IDs after 0-7. Migrate old
   state with neutral EQ and bypassed modulation, preserving previous sound.
   Adapt the UI's fixed eight-parameter snapshot handling and native metadata.
4. Voice and audition the eight musical programs. Test program/state round trips,
   automation, modulation mono behavior, neutral processing and peak headroom.
5. Produce the package through the standard audition workflow. A research-only
   document does not constitute a new plugin release or an audio validation.
