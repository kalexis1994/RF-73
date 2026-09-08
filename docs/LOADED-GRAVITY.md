# Gravitational weight on the hammer and damper arm

The [flight budget study](LOADED-FLIGHT-BUDGET.md) showed that heavier
hammers clear the flight toll easily but fail repetition because the
gravity-free return cannot bring them back, and that the damper arm's spring
sets that toll. This experiment gives the hammer and the damper arm their
weight, keeps the return law explicit, and requalifies rest, first strike,
flight and repetition under the sharp let-off.

```text
cargo run --locked --release -p rf-73-lab -- loaded-gravity --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Strike presence, seating, return,
first-attack and repetition outcomes remain separate from numerical
qualification.

## Weight in the assembly

The action profile gains a gravitational acceleration, zero by default.
Weight acts as a constant force along the hammer and damper-arm coordinates,
whose positive direction points toward the tine. Its potential is part of the
assembly's mechanical energy, referenced to the prepared rest positions so
that the initial energy remains a positive measure of stored energy; the
laboratory hammer, launch, flight and arm ledgers carry the same potential.
Rest preparation lets both linear springs sag under weight before solving the
contacts, and its force rows include the weight. With zero gravity every
added term is exactly zero: rerunning the flight budget study with the new
code reproduces its receipt byte for byte.

An earlier version referenced the potential to the nominal rest positions.
With the hammer sagged into the pedestal that made the initial energy
vanish, and every relative defect in the laboratory blew up while the
absolute residuals stayed at rounding level. The reference was moved before
the frozen run.

## Frozen matrix

Twenty-four takes cross six cases and two nominal speeds at 128/256 ticks,
all on the original profile with the retained 0.05 kg key, sharp let-off and
60 ms repetition wait:

| Case | Gravity (m/s²) | Hammer mass (kg) | Hammer weight (N) | Return spring (N/m) | Arm spring base |
| --- | ---: | ---: | ---: | ---: | --- |
| Control | 0 | 0.004 | 0 | 4 | retained |
| Gravity 4 g | 9.81 | 0.004 | 0.039 | 4 | retained |
| Gravity 8 g | 9.81 | 0.008 | 0.078 | 4 | retained |
| Gravity 12 g | 9.81 | 0.012 | 0.118 | 4 | retained |
| Gravity 12 g, arm reseated | 9.81 | 0.012 | 0.118 | 4 | raised by the arm's static sag |
| Gravity 12 g, weight return | 9.81 | 0.012 | 0.118 | 2 | retained |

The arm weighs 0.0098 N. Raising the arm spring base by that weight over the
200 N/m stiffness (49 µm) restores the gravity-free felt seating force and,
by construction, leaves the arm dynamics unchanged; the case isolates arm
gravity from hammer gravity. Halving the return spring makes the weight 85%
of the return force at full lift. A 0.1 N/m spring was tried first, but its
118 mm unloaded sag made the rest solve lose its `1e-12` contact tolerance
to cancellation, so the frozen run keeps 2 N/m.

## Qualification

The rest state retains hammer sag, pedestal force, arm offset and felt
force. The minimum felt force while the key is up, the hammer reseating time
on the returned pedestal, the lowest hammer position after each release and
the hammer state at the repeat command are retained. Rest values must agree
between resolutions exactly, minimum felt force within 1%, reseating times
within 1 ms and lowest positions within 10 µm. All repetition, launch, key
and let-off gates are retained. The flight identities close with the
gravitational potential; release and strike agreement, release-to-impact
refinement and window-end hammer terms are gated. Window-end coupling terms
after a chattering re-landing are retained but not gated: in the 8 g soft
row a term of a few microjoules differed by 3% between resolutions while
every release-to-impact term agreed within 0.07%. First strikes are compared
with the control and, for the two 12 g variants, with the plain 12 g case,
using the 5% impact/speed and 1 ms latency limits. No configuration is
selected.

## Retained results

All 24 takes and twelve refinement pairs qualify. Control takes replay the
flight budget receipt exactly after removing the gravity record. Eleven
256-tick takes strike on the first gesture; the 4 g weighted soft row never
strikes. Three paired rows meet clean repeatability. Only the control rows
and the 4 g strong row preserve the control's first attack; both 12 g
variants stay within 5% of the plain 12 g case.

### Weight seats the hammer and lightens the felt

| Rest, 256 ticks | Hammer sag (µm) | Pedestal force (N) | Arm offset (µm) | Felt force (N) |
| --- | ---: | ---: | ---: | ---: |
| Control | 0 | 0 | −116.9 | 0.0234 |
| Gravity 4 g | −14.0 | 0.0392 | −134.8 | 0.0172 |
| Gravity 8 g | −19.8 | 0.0784 | −134.8 | 0.0172 |
| Gravity 12 g | −24.3 | 0.1176 | −134.8 | 0.0172 |
| Gravity 12 g, arm reseated | −24.3 | 0.1176 | −165.9 | 0.0234 |

The pedestal carries the hammer weight less the sagged return spring to nine
digits, so the key holds a seated hammer at rest. Arm weight lowers the felt
seating force by 26%; raising the spring base restores it exactly. The
minimum felt force while the key is up is zero in every case, gravity or
not: the felt leaves the tine during the hammer's return and re-seats later,
which the retained readiness diagnostic already reports.

### Weight adds a small, mass-proportional flight toll

| First gesture, 256 ticks | Release speed (m/s) | Release kinetic (mJ) | Gravity share of flight (mJ) | Flight toll (mJ) | Threshold release (m/s) | First impulse at 1.125 / 1.5 (mNs) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control, 4 g | 0.964 | 1.860 | 0 | 1.840 | 0.959 | 0.209443 / 5.281303 |
| Gravity 4 g | 0.957 | 1.831 | 0.062 | — | — | No contact / 5.138331 |
| Gravity 8 g | 0.913 | 3.334 | 0.125 | 2.133 | 0.730 | 5.852290 / 11.763460 |
| Gravity 12 g | 0.879 | 4.640 | 0.187 | 2.246 | 0.612 | 9.227289 / 16.113810 |
| Gravity 12 g, weight return | 0.881 | 4.662 | 0.187 | 2.213 | 0.607 | 9.351949 / 16.188686 |

Lifting the hammer 1.6 mm against its weight costs 0.062 to 0.187 mJ, a
tenth of the damper-arm toll. For the 4 g hammer that is enough: the retained
soft strike sat 0.5% above its threshold and disappears. For 8 and 12 g the
threshold release speeds remain far below the actual releases, so the soft
strikes survive at 5.85 and 9.23 mNs. The heavier hammer also slows the key,
so release speeds fall from 0.964 to 0.879 m/s. Halving the return spring
changes the first impulse by 1.4%.

### Weight does not cure the return bounce

| Return after the first key-up, 256 ticks | Hammer reseats on pedestal (ms) | Lowest position (mm) | Hammer at repeat command |
| --- | ---: | ---: | --- |
| Control | 32.15 | −12.181 | −9.48 mm, −0.163 m/s, airborne |
| Gravity 4 g | 31.54 | −12.193 | −10.02 mm, −0.189 m/s, airborne |
| Gravity 8 g | 32.42 | −12.241 | −9.06 mm, −0.152 m/s, airborne |
| Gravity 12 g | 32.97 | −12.278 | −8.46 mm, −0.116 m/s, airborne |
| Gravity 12 g, weight return | 33.21 | −12.275 | −8.39 mm, −0.105 m/s, airborne |

In every case the key lands 31 to 32 ms after release and catches the
hammer within a millisecond, so reseating is set by the key, not by the
hammer's weight. The hammer then bounces off the pedestal and is still
airborne 2 to 3.6 mm above rest at the repeat command. The bounce, not the
return speed, is what repetition inherits. With weight the 12 g second
gesture is released at the let-off top instead of by a collision below it,
but its second/first ratios of 1.114 and 0.876 still fail the 5% limit,
while the 4 g and 8 g strong rows now repeat within 0.03% and 0.12%.

### Numerical evidence

Maximum relative total energy defect is `3.934e-12`, launch momentum defect
`1.309e-11`, launch work defect `3.631e-12`, key energy defect `5.234e-13`,
coupling ledger defect `5.222e-14`, flight hammer defect `7.648e-14`, bridle
defect `1.330e-13` and arm defect `6.270e-14`. Rest quantities agree between
resolutions exactly; maximum reseating-time error is 0.163 µs and lowest
position error 1.3 nm. Maximum velocity refinement RMSE is 0.17628%, impact
refinement error 0.14278%, kinetic-work-term error 0.27320% and
release-to-impact budget term error 0.28660%.

Verification passes 143 lab unit tests, 116 DSP unit tests, 87 loaded
CLI/receipt tests, strict Clippy and formatting. A DSP test checks that a
weighted 12 g hammer prepares a seated rest whose pedestal force equals the
weight less the sagged spring, that the balance stays exact through a
prescribed stroke, and that negative gravity is rejected. Laboratory tests
check the declared case settings and the weighted rest, and receipt tests
verify settings, rest values, flight identities with the gravitational term,
control replay and first-attack decisions.

The [receipt](../references/loaded-gravity-validation.json) is 2324897
bytes, SHA-256
`71a9ab02fa4ce67e5317500d1d5773bc872ebb6363324873c3114e3361d9f1fa`.
The release cache remains approximately
183 MiB, with no new audio renders.

## Scope and next step

Weight is a constant force along the reduced coordinates, not a measured
pivot geometry or lever ratio, and hammer masses remain provisional. One
key, one let-off, one profile and one repetition wait are covered.

Return timing is now set by the key, and the remaining repetition defect is
the hammer's bounce on the returned pedestal: a nearly elastic pedestal
contact with 0.025 Ns/m of return damping and no check. Next model the
hammer landing, comparing pedestal contact loss, return damping and a
back-check that captures the hammer after its first bounce, and requalify
readiness, second-strike consistency and the first strike against the
retained control with the flight budget kept explicit.
