# Conditional calibration of loaded G3 sustain

The [loss budget](LOADED-LOSS-BUDGET.md) identified first the tine and then the
support as the largest late dissipative sinks. This experiment tests whether
their coefficients can improve the recorded-source envelope without changing
geometry, pickup, gain or upper-mode losses. It selects a candidate from training
layers and evaluates reserved layers only after selection.

```text
cargo run --locked --release -p rf-73-lab -- calibrate-loaded-loss references/g3-pitch-reference.manifest.json --output references/loaded-loss-calibration-qualified-validation.json --preview renders/loaded-loss-calibrated.wav --qualified-grid
```

## Frozen selection and qualification

All five G3 recordings are verified against the pinned Git blob identities.
The three training layers (1, 3 and 5) alone set the spring target through their
mean log frequency. Source onsets, native sample rates and pitch-aware spectral
measurements are preserved from the preceding baseline protocol.

The candidate grid has two axes:

- First fixed-root tine-mode damping scale: 1/6, 1/3 or 1/2. These correspond
  to the existing basis T60 parameter taking values 30, 15 or 10 seconds.
- Support translation and rotation damping scale: 0.1, 0.5 or 1.0.

The first tine coordinate is changed in both transverse planes. The other five
tine-mode losses, tonebar, felt, action, magnetic field, electrical circuit and
spring position remain fixed. The coupled audible mode does not in general
have the same T60 as an isolated fixed-root tine coordinate. These are model
parameters, not measured material decay times.

Each of nine candidates is rendered from static equilibrium using a 1.5 m/s
pedestal gesture, at 128 ticks per 48 kHz frame for 1.8 seconds. Its score is
pooled RMS error in dB against the three training sources over the two sustain
windows 640–1152 and 1152–1664 ms after onset, each expressed relative to the
256–512 ms body. Every source has equal weight; no source layer is assigned a
physical key velocity. There is no time-dependent gain adjustment.

The minimum eligible score selects the candidate, with fixed grid order breaking exact
ties. The initial strict run withheld selection at the first unqualified corner;
its failed receipt is preserved. The explicit follow-up policy is described below.
The report preserves all nine scores, whether selection lies on a grid edge,
and every candidate within 0.25 dB training RMS of the best. That neighborhood
is a descriptive ambiguity check, not an uncertainty interval.

## Explicit follow-up after an unqualified corner

The initial `references/loaded-loss-calibration-validation.json` records
`loss study pitch unavailable` at the first corner (tine scale 1/6, support
scale 0.1), before any candidate was accepted. No output WAV was written.
This is a failed frequency observation, not proof that time integration failed.

The subsequent `--qualified-grid` run evaluates the same nine candidates and
retains every rejected candidate's complete pitch and energy diagnostics, with
a null training score. Selection is restricted to eligible candidates; no
pitch, energy or convergence threshold changes. Validation layers remain outside
selection. Without the explicit flag, the CLI still withholds selection when
a grid candidate fails. The loss renderer now retains an unavailable pitch as
data in a failed take instead of throwing away the diagnostics.

After selection, the candidate is rerendered at 256 ticks. Reserved layers 2
and 4 then receive their own RMS score; they do not rerank or change the winner.
The fixed candidate is also tested at pedestal speeds 1.125 and 1.75 m/s, each
at 128 and 256 ticks. All fifteen source/gesture comparisons remain visible,
including spectral mismatches that were not part of the fit.

Numerical qualification reuses the loss-budget energy, channel-sum, quiet idle,
contact, finite output and 1% waveform-refinement gates. Envelope improvement
requires at least a 50% RMS reduction versus the unmodified 128-tick baseline
on both training and reserved layers. These two claims remain separate from
spectral agreement and physical calibration, neither of which is implied.

## Retained outcome

The qualified-grid follow-up evaluates all nine points: six are eligible and
three are withheld. All three withheld points use support scale 0.1; a competing
output peak near 225.3–225.5 Hz lies within 20 dB of the main peak in the first
two pitch windows. Their energy diagnostics remain finite and within the
numerical gates. The rejection is due to ambiguous frequency observation.

| First tine-mode scale | Support scale | Training RMS error (dB) |
| --- | --- | --- |
| 1/6 | 0.1 | Withheld |
| 1/6 | 0.5 | 1.265892 |
| 1/6 | 1.0 | **1.226637** |
| 1/3 | 0.1 | Withheld |
| 1/3 | 0.5 | 1.433391 |
| 1/3 | 1.0 | 2.196783 |
| 1/2 | 0.1 | Withheld |
| 1/2 | 0.5 | 2.585758 |
| 1/2 | 1.0 | 3.559617 |

Selection retains the original support losses and changes the first tine basis
T60 from 5 to 30 seconds. This lies on the grid boundary and the existing
parameter-domain limit. Two other candidates are within the declared 0.25 dB
training-score neighborhood, including one with T60 15 s and half support
losses. The data and grid do not establish unique material parameters.

| Source role | Baseline RMS (dB) | Selected fine RMS (dB) |
| --- | --- | --- |
| Training, layers 1/3/5 | 8.051992 | 1.226638 |
| Reserved, layers 2/4 | 8.498277 | 0.986501 |

The training error falls by 84.8% and reserved-layer error by 88.4%, exceeding
the frozen 50% improvement gate for both. The rerendered unmodified baseline
exactly reproduces the earlier loss-budget take, including its timbre and energy
summary. Source profiles, identities and spring fit also reproduce the previous
source-comparison receipt exactly.

All six selected coarse/fine takes qualify across three pedestal speeds. The
worst windowed voltage RMSE is 2.869e-5 (0.002869%); relative total energy defect
is at most 1.567e-12 and structural split defect 4.203e-14.

| Pedestal speed (m/s) | Fine late relative level (dB) | Observed pitch (Hz) |
| --- | --- | --- |
| 1.125 | -4.89727 | 196.380247 |
| 1.5 | -4.23858 | 196.381715 |
| 1.75 | -4.25982 | 196.383159 |

All fifteen source/gesture pairs are within 3 dB in both evaluated sustain
windows. All fifteen still fail the separate spectral comparison. This supports
an improved conditional envelope, not full reference agreement. The next block
should address attack and spectral balance through physical excitation and
pickup controls, preserving the loss ambiguity and avoiding an EQ correction
that could hide it. Release, repetition and other registers need qualification
under any adopted candidate.

The single fine medium WAV reads back as 86,400 finite samples at 48 kHz,
peak 0.213811710 FS and RMS 0.020394423 FS. There is no listening verdict.

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| Initial failed receipt | 174 | `c9b5db0719c5ab784f9b0f960d8f7b0e14dccdcae33adc51bf0cb23810bcd686` |
| Qualified-grid receipt | 423961 | `708fb7e58b16bdfc8def13942e29861aeef32424d380c20f2cd5a707e7d56a81` |
| Medium sustain WAV | 345658 | `a169dc8e719546c8474343c4ce4ba6774a5ffbdfed64445fb07b2fc04710da73` |

Verification passes 114 lab unit tests and nine loaded CLI/receipt tests, strict
Clippy and formatting. Tests independently recompute the role-specific scores,
confirm the selected training minimum and exact prior source preservation, and
retain rejected candidates with null scores. Release cache remains near 194 MiB;
the failed receipt is preserved and only one new WAV is written.

## Reproducibility and scope

The optional output is one medium-strike float WAV, at fixed 0.1 FS/V gain and
with no normalization. It is a held-note sustain excerpt ending at 1.8 seconds,
not a release/repetition demonstration. Candidate WAVs are not accumulated. Grid summaries and
all selected coarse/fine takes are retained in a single JSON receipt; the
prior receipts and recordings are preserved.

The reserved layers belong to the same processed bank and were already exposed
in earlier studies. They are not a blind independent instrument. Unknown EQ,
noise reduction, capture gain and physical strike velocities still constrain
interpretation. A better level trajectory does not establish a unique pair of
material coefficients or a realistic spectrum. Grid limits, selected-mode
assumptions, other registers and action/release calibration remain explicit.
The experiment does not alter production defaults or the playable plugin.
