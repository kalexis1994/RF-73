# Cross-note spectral hypotheses: D3, G3 and B3

The neighboring-note pilot finds recurrent groups separated by each note's
fundamental, while simple fixed-frequency and constant-ratio predictions do
not connect the G3 attack families to the other notes under the declared
recurrence rule. These observations motivate a controlled pickup-mixing
experiment before assigning individual spectral peaks to mechanical modes.

## Selection and source identity

Before inspecting the new audio, the [manifest](../references/register-families.manifest.json)
fixed D3 (MIDI 50) and B3 (MIDI 59), the nearest independently recorded notes
on either side of G3 in the pinned mono bank, and retained all five layers.
These are neighboring sampled anchors, not adjacent semitones. G3 had already
been inspected; the new notes are descriptive probes of existing hypotheses,
not a new instrument or a blind calibration set.

Ten new WAVs total 13955116 bytes. The [acquisition inventory](../references/jrhodes-neighbors.inventory.json)
records source URLs, Git blob IDs, byte lengths and SHA-256 hashes. All bytes
match author revision `a886e6cebf074c995a10634f82ebe4fdb90f5ca6`. Audio and the
upstream LICENSE/README are retained under ignored
`references/audio/jrhodes-neighbors-a886e6c/`; samples are not bundled in RF-73.
[Author's pinned repository](https://github.com/jlearman/jRhodes3d-wav/tree/a886e6cebf074c995a10634f82ebe4fdb90f5ca6).

The source is the same 1977 Mark I Stage 73 with harp-output recording, EQ and
noise reduction described in [Reference banks](REFERENCE-BANKS.md). Capture gain,
strike speeds, precise note-off and untreated signals remain unknown. Layer
numbers are playback mappings, not calibrated hammer velocities.

## Reproduction and qualification

```text
cargo run --locked --release -p rf-73-lab -- observe-register references/register-families.manifest.json --output renders/register-families.json
```

Output must be a new JSON file. The strict schema permits 2..5 distinct notes
and 3..8 distinct pinned takes per note. Duplicate notes, IDs or content,
unknown fields, oversized manifests and a missing reference note are rejected.
Git verifies the exact bytes that the Rust reader decodes. All paths in this
manifest are relative to the repository working directory.

Each take first uses the existing qualified three-window pitch-anchor method:
three 512 ms observations starting at 250 ms. A failed anchor is retained and
withholds family inference for that entire note, rather than dropping the take.
The report is written with the failure state and the command exits unsuccessfully
if any note has an unqualified pitch anchor. Byte/format errors fail before a
completed report is published.

All 15 anchors qualified here. Their per-note geometric means are descriptive
scales for comparison, not replacement tuning targets:

| Note | Mean fundamental (Hz) | Takes |
| --- | --- | --- |
| D3 | 147.367864 | 5 |
| G3 | 196.380284 | 5 |
| B3 | 247.269382 | 5 |

The unchanged [family method](SPECTRAL-FAMILIES.md) then observes native-rate
32/128 ms attack windows and a 512 ms body window from 250 ms. Every peak,
pitch observation, recurrence flag, ambiguity and capacity limit is retained in
the [1027206-byte report](../references/register-families-validation.json).
These are file offsets, not inferred mechanical onset or guaranteed sustain.

For each recurrent nonharmonic G3 family, two separate hypotheses predict the
other note's frequency: fixed Hz, and G3 family frequency multiplied by the
ratio of the per-note mean fundamentals. Family centers use only reliable,
unambiguous, uncapped nonharmonic members. Correspondence tolerances add both
reciprocal-window durations, scaling the reference duration's frequency width
for the ratio hypothesis. These are observation tolerances, not confidence
intervals; pitch-anchor uncertainty is not statistically propagated.

Multiple candidate matches and candidates shared by multiple G3 families are
ambiguous. A missing correspondence means no recurrent candidate passed these
rules, not that the instrument lacks a mode. Nonuniform tine geometry need
not preserve modal ratios across notes, so the proportional hypothesis is a
diagnostic simplification, not a physical law or a mode-identification test.

## Findings

The 128 ms attack windows contain the following recurrent nonharmonic groups
(rounded descriptive locations, not fitted resonances):

| Note | Candidate groups (Hz) | Fundamental-offset relations |
| --- | --- | --- |
| D3 | 2695, 2841, 3136, 3283, 5521 | The first four support a chain with offsets of 1, 2, 3 or 4 fundamentals |
| G3 | 886, 1425, 1620, 7108 | 1425 to 1620 supports one fundamental |
| B3 | 375, 622, 1578, 1823, 2070 | 375 to 622, plus the 1578/1823/2070 group, support fundamental offsets |

D3's 2695-to-2841 Hz relation has residuals +0.813/-0.255/+0.087 Hz after
subtracting each take's fundamental in layers 3/4/5. Other D3 chain relations
also lie within the same declared 15.624 Hz difference tolerance. The report
retains the exact supporting takes and residuals for every relation. A chain
is compatible with nonlinear mixing but does not establish its physical origin,
carrier, causal direction, or number of underlying mechanical modes.

None of the four G3 attack-128 families finds a recurrent counterpart in D3 or
B3 under either simple cross-note rule. This does not justify fitting the
tine geometry harder to a single G3 peak. Source processing, detector limits,
real inharmonic geometry and incorrect mode assignment remain alternatives.

In the body window, reliable centers near 90.791/99.977 Hz in G3 correspond to
90.796/100.135 Hz in B3 under the fixed-Hz rule. A G3 component at 386.890 Hz
corresponds to D3 at 386.689 Hz under that rule, while its constant-ratio prediction
matches B3 at 485.818 Hz. The G3 near-100 Hz family also has a separate approximate
ratio match at B3's 123.304 Hz. These competing descriptions do not select an
origin. Fixed-frequency background is worth investigating, but is not proven
electrical hum; the same assembly or unrelated coincidences are also possible.

No attack-32 group passes isolated recurrence. D3 and G3 layer-2 attack-128
windows are capped; their layer-1/2 body windows are capped. B3's layer-1 body
window is capped. All remain explicit and do not count toward reliable recurrence.

All 127 affected analysis/laboratory release tests, strict workspace Clippy
and formatting pass. Three new unit regressions and one CLI regression bring
the workspace total to 242; unchanged DSP/plugin/UI suites were not rerun.
Tests cover separate fixed/scaled predictions, absent and shared matches,
withheld unqualified groups, duplicate notes/content, missing references,
output preservation and altered source-byte rejection. The five G3 pitch anchors
and all 239 previously accepted peaks across its 15 windows remain exactly equal.
No audio-device, listening, remote-CI or physical-identity qualification is claimed.

## Next physical experiment

Use prescribed two-mode mechanical trajectories through the existing nonlinear
pickup, with a linearized pickup as control, to separate parent frequencies
from generated sidebands. Keep motion, gain and sampling fixed while checking
sum/difference components and numerical aliasing. That test can qualify a
mechanism capable of producing the observed patterns without claiming it has
identified their actual source. Geometry and damping fitting should wait for
better modal evidence. No DSP parameter, plugin version or audible baseline is
changed by this reference-only study.
