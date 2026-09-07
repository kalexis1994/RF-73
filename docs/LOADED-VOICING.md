# Controlled loaded excitation and pickup sensitivity

The [conditional sustain preset](LOADED-LOSS-CALIBRATION.md) improves relative
decay against the pinned G3 bank while retaining spectral disagreement. This
study separates changes to excitation from changes to magnetic observation.
It is a prescribed sensitivity experiment, not a parameter search.

```text
cargo run --locked --release -p rf-73-lab -- loaded-voicing references/g3-pitch-reference.manifest.json --output references/loaded-voicing-validation.json
```

## Frozen controls

All five source recordings are verified against their pinned blob identities.
The mean log frequency of training layers 1, 3 and 5 sets the mode-tracked spring
position on the 70 mm tine. The first fixed-root tine coordinate retains T60
30 s in both planes; other structural losses and support coefficients retain
their original values. The preceding fit's boundary and ambiguity limitations
still apply. No retuning takes place after an intervention.

| Case | Changed setting | Baseline setting |
| --- | --- | --- |
| Baseline | None | Conditional sustain preset |
| Strike at 10% | 7 mm from root | 14 mm |
| Strike at 30% | 21 mm from root | 14 mm |
| Half contact stiffness | 2e10 N/m² | 4e10 N/m² |
| Double contact stiffness | 8e10 N/m² | 4e10 N/m² |
| Pickup gap | 1 mm | 1.5 mm |
| Vertical pickup offset | 1 mm | 0.5 mm |

The quadratic point-contact law remains unchanged. Moving the contact port
changes its participation in the beam modes; changing its stiffness changes
the force pulse. Neither setting changes the free structural eigenproblem.
Pickup changes use the existing finite-aperture flux law and reciprocal current
reaction. They can therefore affect both voltage and mechanical motion. These
are model interventions, not measured dimensions of the reference instrument.

Every case starts from static equilibrium and uses the same 1.5 m/s pedestal
gesture, beginning at 30 ms and held through 1.8 s. Each is rendered at 128 and
256 solver ticks per 48 kHz frame. Midpoint voltage averaging, the common FIR
decimator and fixed gain 0.1 FS/V remain unchanged. No case-specific gain, EQ,
source velocity assignment or additional WAV matrix is introduced.

## Independent observations and acceptance

The existing independent twelve-channel loss observer checks energy closure,
monotone heat, quiet pre-key output, finite unclipped voltage, exactly one hammer
contact entry and qualified pitch-aware timbre. Four windows require less than
1% coarse/fine voltage RMS difference.

A passive impact observer additionally records integrated hammer force, peak
tick force, total duration of positive force and hammer speed immediately before
the first contact tick. Force is the solver's interval-mean contact force;
duration is tick resolved. Impulse, peak force and duration must each change by
less than 1% under refinement. The observer does not feed back into the model.
Baseline reports with the added impact object removed must exactly reproduce
the previous calibration's medium coarse/fine reports.

The source comparison retains all 35 source/case pairs and all five timbre
windows. Attack RMS uses all three spectral band balances in 0–64 ms after
onset. Any missing attack band withholds that score; weak bands are never
silently discarded to improve it. The pooled training attack score has nine
equally weighted dimensions across three sources and is descriptive only.
No winner is selected. Differences from the model baseline are also retained.

For these training recordings, the arithmetic mean of the three attack band
balances is [-7.619790, -19.112656, -33.412167] dB. Their within-bank spread
imposes a 16.509272 dB lower bound on pooled RMS for any single candidate
spectrum, even an unconstrained one: RMS squared equals that variance plus
the mean squared distance to the three-band mean. This is an algebraic bound
for this metric, not a perceptual threshold. One drive speed cannot represent
all source dynamics, and an average spectrum need not describe any real layer.
Individual layer comparisons are therefore essential before a subsequent
multi-speed study; a pooled score must not select a production voicing here.

The separate 6 dB spectral and 3 dB sustain gates remain unchanged. A known
spectral exceedance is failure even when other bands are unavailable; complete
spectral acceptance needs all fifteen bands. Numerical qualification does not
imply source agreement. Failed case qualifications are retained, and the CLI
returns failure if any case fails; an existing output path is never overwritten.

## Retained outcome

All fourteen takes completed. Six case pairs qualify; the 21 mm strike does
not. It produces three hammer/tine force-positive episodes at both resolutions,
violating the frozen single-contact condition. Its output, energy and impact
refinement still pass. The overall receipt therefore records
`measurement_qualified: false`, with no runtime exception, and the CLI returns
failure. This is a retained recontact observation in the provisional model,
not evidence that the solver diverged or that the real instrument behaves so.
Its impulse and force-positive duration sum all three episodes.

| Case | Qualified | Fine peak force (N) | Fine impulse (N s) | Positive-force duration (ms) | Training attack RMS (dB) |
| --- | --- | --- | --- | --- | --- |
| Baseline | Yes | 23.5560 | 0.00678173 | 0.714762 | 18.735510 |
| Strike 7 mm | Yes | 44.0087 | 0.00613873 | 0.283040 | 30.652462 |
| Strike 21 mm | No: recontact | 24.7801 | 0.00566673 | 0.781494 | 23.567887 |
| Half contact stiffness | Yes | 22.2913 | 0.00679801 | 0.739176 | 18.627497 |
| Double contact stiffness | Yes | 30.2525 | 0.00678947 | 0.703857 | 18.705092 |
| Pickup gap 1 mm | Yes | 23.5560 | 0.00678169 | 0.714762 | 19.339256 |
| Vertical offset 1 mm | Yes | 23.5560 | 0.00678171 | 0.714762 | 17.095162 |

Moving the strike toward the root substantially changes the pulse and spectral
balance. Its three attack bands shift from the baseline by 21.77 dB RMS, while
observed pitch stays near 196.38 Hz. This intervention changes excitation,
not the free structural eigenfrequencies; the invariant spectrum test confirms
that distinction.

Halving contact stiffness extends positive-force duration by about 3.4% and
reduces the highest attack-band balance by 3.54 dB. The impulse changes only
0.24%. Doubling stiffness raises peak force by about 28.4%, but its three-band
attack shift is only 0.226 dB RMS. Peak force alone does not describe spectral
excitation through this coupled mechanism.

The 1 mm vertical pickup offset changes the three attack bands by 6.24 dB RMS
with nearly unchanged contact impulse. It lowers pooled training attack error,
but the layer-specific effects conflict: layer 3 improves from 11.66 to 6.50 dB,
while layer 1 worsens from 12.98 to 18.32 dB. The reserved layer 2 also worsens,
from 4.08 to 5.17 dB. No geometry is selected from that average improvement.
Reducing pickup gap instead raises fixed-gain peak output from 0.214 to 0.281 FS
and worsens the pooled attack score.

All 35 retained source/case pairs fail the separate full spectral gate. All
35 satisfy the two late-level comparisons, including the five observations
from the ineligible recontact case; that does not make that case eligible.
These interventions preserve the improved conditional sustain but do not
resolve full spectral disagreement with the bank.

Across the entire matrix, maximum relative energy defect is 1.161e-12,
structural channel-sum defect 4.203e-14 and exchange defect 3.489e-18. Maximum
windowed voltage RMSE is 8.972e-5 (0.008972%). The worst impact refinement error
is 0.031241%, for duration in the recontact case. No numerical gate was relaxed.
Source profiles, identities, spring fit and both baseline reports exactly
reproduce the preceding calibration after removing the new impact fields.

Verification passes 116 lab unit tests, eleven loaded CLI/receipt tests, strict
Clippy and formatting. Receipt tests independently recompute impact errors and
training attack scores, preserve the rejected control and verify exact baseline
reproduction. The release cache remains about 194 MiB and no WAVs are added.

The [retained receipt](../references/loaded-voicing-validation.json) is 477924
bytes, SHA-256
`2d884be2ec6f7ef10553f6f4411d1a4c3c827578e231bb0d81d7c61bc0ce47aa`.

## Next experiment and scope

The next useful block is a constrained multi-speed excitation/pickup comparison:
preserve each source layer's spectrum, qualify feasible strikes and use a shared
physical voicing across speeds. Any inferred velocity mapping must be declared
as a nuisance assumption, with reserved-layer evaluation and nearby alternatives.
Investigate the spatial recontact before admitting that strike region. Qualify
release/repetition under any adopted voicing before production integration.

The earlier [hammer comparison](CONTROLLED-HAMMERS.md) found attack differences
among elastic, rate-dependent and memory candidates in a different reduction.
It does not establish that added material states explain the present bank
mismatch. Current controls give a shared loaded baseline for deciding whether
that added complexity is warranted.


This is one conditional G3 reduction at one pedestal speed. The bank has unknown
strike velocities, capture gain and documented processing, including EQ and
noise reduction. Differences among source layers cannot identify a unique
hammer material or pickup geometry. Spectral band agreement alone would also
be insufficient to establish perceptual realism.

The study adds an observational tool, not a new material law or a production
preset. Release/repetition under changed voicing, other registers, measured
magnetic fields and realtime integration remain open. No listening verdict or
host test is claimed.
