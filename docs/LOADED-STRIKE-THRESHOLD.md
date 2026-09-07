# Loaded soft-strike threshold and hammer energy transfer

The [shared-voicing study](LOADED-DYNAMICS.md) exposed a large change in actual
hammer speed for a small drive change near the soft-strike region. This study
tests which action/contact controls move the onset of impact and measures
energy transfer independently of the existing whole-system ledger.

```text
cargo run --locked --release -p rf-73-lab -- loaded-strike-threshold --output references/loaded-strike-threshold-validation.json
```

## Frozen experiment

Five configurations share the preceding conditional 70 mm G3 profile and
mode-tracked spring target of 196.38614697959488 Hz:

| Configuration | Changed parameter | Baseline |
| --- | --- | --- |
| Baseline | None | Conditional sustain/voicing profile |
| Half return stiffness | 2 N/m | 4 N/m |
| Half return damping | 0.0125 N s/m | 0.025 N s/m |
| Escapement 1 mm | 1 mm | 1.5 mm |
| Half hammer contact stiffness | 2e10 N/m² | 4e10 N/m² |

Each configuration uses drive speeds 1, 1.0625, 1.125, 1.1875, 1.3125 and 1.5
m/s, at 128 and 256 ticks per 48 kHz observation frame: sixty takes. Static rest
precedes the same pedestal slew from 30 ms; the key is then held through 120 ms.
The shorter window targets action/impact diagnosis, not sustain or source timbre.
Changing escapement also changes prescribed travel, not just free-flight distance.

No-contact, single-contact and multiple-contact outcomes remain distinct. A
gesture without contact within 120 ms is a valid control; it is not relabeled
as a soft note or evidence of never striking. Only adjacent qualified speed
pairs with different contact presence define observed transition intervals.
There is no interpolated threshold, assumed monotonicity or material fit.

## Independent work accounting

Hammer energy is kinetic energy plus the existing linear return-spring
potential. Its change equals signed pedestal-to-hammer work minus signed work
delivered to hammer/tine contact and bridle, minus viscous return heat. The
observer integrates interval contact forces against hammer displacement, with
the bridle port's actual mechanical ratio.

Two further identities separate transfer from contact storage and dissipation:

- Actuator pedestal work minus work received by the hammer equals the change
  in pedestal-contact potential plus that contact's dissipated heat.
- Hammer work delivered to tine contact minus work received by the structure
  equals the change in hammer-contact potential plus contact heat.

The structure's hammer-port displacement follows independently from hammer
displacement minus the change in contact compression. The quadratic law's
potential is K times positive compression cubed divided by three. Return heat
is integrated separately from midpoint hammer velocity and checked against
the solver. Transfers are signed: returned energy must not be clamped away or
interpreted as a nonnegative share of sound.

These fixed profiles start with zero hammer, hammer-contact and pedestal-contact
energy. The total ledger retains the positive felt/rest preload energy.

The receipt records energy/work immediately before the first contact, at
pedestal departure and hammer entry/exit, and at the window end. Events are
bounded at 64 per take; truncation fails qualification. Multiple departures
remain visible rather than assuming a unique launch event. Peak pre-contact
compression also records how close a non-striking gesture comes to the tine.

## Numerical qualification and scope

At every solver tick, relative total, hammer, pedestal and hammer-contact work
defects must stay below 1e-8 of initial total energy plus absolute drive work.
Exchange and independently integrated return-heat defects must stay below
1e-10. Heat remains monotone and raw pre-key voltage below 1e-9 V.

Coarse/fine hammer-contact counts must agree. Hammer and both pickup velocities
are compared independently at output-frame endpoints, requiring less than 1%
RMS error with a 1e-8 m/s reference RMS floor. Positive impacts also require
less than 1% error in impulse, peak interval force and positive-force duration.
No-contact pairs require exact zero impact. These are selected refinement
checks, not exact-solution bounds or an audio antialiasing test.

The recorded bank does not choose any parameter in this study. No source match,
actual instrument threshold, new material law, listening or realtime result is
claimed. The provisional action is prescribed pedestal motion with a persistent
hammer, not measured pivot/cam geometry. Release/repetition and longer-term
contact behavior remain separate qualification tasks. No WAVs are generated.

## Retained results

All sixty takes and thirty coarse/fine pairs qualify numerically. There are
eight no-contact rows, twenty-one single-contact rows and one two-contact row.
Baseline, half return stiffness, half return damping and half contact stiffness
share the observed no-contact/contact interval from 1.0625 to 1.125 m/s. This
grid does not resolve smaller shifts within that interval.

| Configuration | Contact counts at ascending grid speeds | Fine hammer speed at drive 1.125 m/s |
| --- | --- | --- |
| Baseline | 0, 0, 1, 1, 1, 1 | 0.106164 m/s |
| Half return stiffness | 0, 0, 1, 1, 1, 1 | 0.159116 m/s |
| Half return damping | 0, 0, 2, 1, 1, 1 | 0.145226 m/s |
| Escapement 1 mm | 1, 1, 1, 1, 1, 1 | 0.691065 m/s |
| Half contact stiffness | 0, 0, 1, 1, 1, 1 | 0.106164 m/s |

The shorter escapement strikes throughout the sampled range, so its transition
interval is explicitly empty: no lower non-striking endpoint was observed.
It does not establish a useful soft response. Its first four actual hammer
speeds are 0.681605, 0.702800, 0.691065 and 0.690213 m/s despite increasing drive.
The local decreases occur at both resolutions. Changing travel/flight geometry
has produced a nearly flat, locally non-monotone response in this range.

Halving return damping creates two contacts at 1.125 m/s in both resolutions.
This remains a numerically qualified diagnostic row, not an accepted
single-strike candidate. Its impulse and force-positive duration sum both
episodes. The experiment intentionally classifies recontact rather than
excluding it under the preceding source-fit single-contact protocol.

Halving hammer contact stiffness leaves every pre-contact snapshot exactly
unchanged, including the four striking baseline speeds. At 1.125 m/s it changes
impulse from 0.000237325 to 0.000223782 N s and contact duration from 0.382161 to
0.433350 ms. Contact stiffness affects the pulse after impact; this control
does not explain why the preceding drive failed to reach contact.

## Where the soft-strike energy goes

At drive 1.125 m/s, the original model has the following balance immediately
before its first hammer/tine contact:

| Quantity | Energy/work (mJ) |
| --- | --- |
| Prescribed pedestal actuator work | 12.294441 |
| Pedestal-contact heat | 2.546027 |
| Work received by hammer | 9.748414 |
| Signed work from hammer to bridle | 9.100461 |
| Hammer return heat | 0.337374 |
| Hammer return potential | 0.288037 |
| Hammer kinetic energy | 0.022541 |

About 93.4% of the work received by the hammer has crossed the bridle port at
this instant. This is energy transfer into the coupled bridle/damper mechanism,
not 93.4% dissipated loss. The bridle is still exerting approximately 1.30 N
before impact. Its cumulative exported work later falls to 8.021963 mJ by
120 ms, showing net energy returning through that port after the first impact.
Likewise, cumulative pedestal-to-hammer work falls as the mechanism pushes
back against the held pedestal. Signed accounting preserves this exchange.

The 1 mm escapement raises actuator work to 13.645902 mJ and pre-impact hammer
kinetic energy to 0.955141 mJ at the same drive speed. Its much stronger impact
therefore accompanies a changed mechanical work trajectory. It cannot be
interpreted as a gain-only adjustment or adopted as a calibrated soft-note fix.

The bridle transfer is the clearest next diagnostic target. Compare bridle
slack/ratio and damper-arm loading while preserving continuous state, return
behavior and pedal/key timing. Check whether a change improves feasible soft
impact control without losing damper lift or creating repeated contacts. The
present data do not identify a real instrument's linkage geometry, distinguish
all causes, or justify simply removing the bridle load.

## Verification and artifacts

Maximum relative defects across all ticks are 1.742e-12 for total energy,
1.463e-12 for hammer energy, 1.312e-12 for pedestal transfer, 2.175e-15 for
hammer-contact transfer and 9.708e-18 for independent return heat. Maximum
velocity refinement RMSE is 0.034009%; maximum impact refinement error is
0.042589%. Every no-contact pair has exactly zero impact.

Baseline impulse, force peak, duration, onset and pre-impact speed at 1.125,
1.3125 and 1.5 m/s exactly reproduce the earlier long loaded-dynamics takes.
The new receipt checks reconstruct all three end-of-window work identities,
retain non-striking/recontact controls and preserve the non-monotone escapement
response. The independent hammer test also rejects the wrong contact-port sign.

Verification passes 121 lab unit tests, fifteen loaded CLI/receipt tests,
strict Clippy and formatting. The [receipt](../references/loaded-strike-threshold-validation.json)
is 818295 bytes, SHA-256
`c61391b37e22525450ec838c4cf6758bea6d6e69ccd496c5e4954dd21e76f0c5`.
Release cache remains approximately 194 MiB. No source recordings, earlier
receipts or test audio were removed, and no WAV matrix was added.
