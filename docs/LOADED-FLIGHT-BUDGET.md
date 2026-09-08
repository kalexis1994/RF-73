# Hammer flight budget from let-off to impact

The [let-off study](LOADED-LETOFF.md) made the launch geometric and showed
that the bridle consumes most of the hammer's energy between release and
impact. This experiment measures that flight with an independent budget from
release to impact, decomposes the bridle work into storage, heat and
damper-arm terms, and varies the provisional hammer mass and the
bridle/damper-arm load one factor at a time under the sharp let-off.

```text
cargo run --locked --release -p rf-73-lab -- loaded-flight-budget --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Strike presence, release height,
first-attack and repetition outcomes remain separate from numerical
qualification.

## Frozen matrix

Twenty-four takes cross six cases and two nominal speeds at 128/256 ticks,
all on the original profile with the retained 0.05 kg key, sharp let-off at
the escapement top and 60 ms repetition wait:

| Case | Hammer mass (kg) | Arm damping (Ns/m) | Bridle rate loss (s/m) | Arm mass (kg) |
| --- | ---: | ---: | ---: | ---: |
| Control | 0.004 | 0.5 | 2 | 0.001 |
| Hammer mass ×2 | 0.008 | 0.5 | 2 | 0.001 |
| Hammer mass ×3 | 0.012 | 0.5 | 2 | 0.001 |
| Half arm damping | 0.004 | 0.25 | 2 | 0.001 |
| Half bridle rate loss | 0.004 | 0.5 | 1 | 0.001 |
| Half arm mass | 0.004 | 0.5 | 2 | 0.0005 |

Arm stiffness (200 N/m), bridle ratio, slack and stiffness, return spring and
damping, tuning, pickup and circuit stay fixed. The finger force is unchanged,
so a heavier hammer slows the key more.

## Flight observer

For each key-down an observer integrates hammer-to-bridle work, return-damper
heat, pedestal work and tine-contact work on the hammer, tracks return-spring
potential and hammer kinetic energy, and runs the retained bridle/arm/felt
coupling ledger from its own start. The release snapshot is the state after
the last pedestal exit before the first hammer/tine contact; without a strike
it is the first pedestal exit taken with the pedestal at the let-off top. The
impact snapshot is the state before the first contact tick, the same state
the launch ledger uses. A 30 ms window-end snapshot is always taken. Whether
the release lies within 0.6 mm of the top is retained as functional evidence.

Three identities must close within `1e-8` relative for every budget:

```text
hammer:  KE_release - KE_arrival
         = bridle work + return heat + return potential change
           - pedestal work + tine-contact work
bridle:  bridle work = bridle storage + bridle heat + arm transfer
arm:     arm transfer = arm energy change + arm heat + felt transfer
```

The coupling ledger's own defects must stay below `1e-8`. Refinement between
resolutions requires release and arrival times within 0.1 ms, release and
arrival speeds within 1% with a 0.01 m/s floor, budget and coupling terms
within 1% normalized by the term or 0.1% of the summed magnitude, and
agreeing release, release-height and strike outcomes. All repetition,
launch, key and let-off gates are retained. First strikes are compared with
the control using the 5% impact/speed and 1 ms latency limits. No
configuration is selected. A unit test checks that a wrong hammer mass breaks
the hammer identity while leaving the coupling identities intact.

## Retained results

All 24 takes and twelve refinement pairs qualify. Control takes replay the
let-off receipt exactly after removing the flight record. Eleven 256-tick
takes strike on the first gesture; the half-arm-mass soft row never strikes,
and the control and half-bridle-loss soft rows miss the repeated gesture.
Two paired rows meet clean repeatability. Only the control rows and the
half-bridle-loss strong row preserve the control's first attack.

### The flight is a nearly fixed energy toll

| Case, 256 ticks, first gesture | Release speed (m/s) | Release kinetic (mJ) | Bridle work in flight (mJ) | Bridle share | Impact speed (m/s) | First impulse (mNs) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control, 1.125 | 0.9643 | 1.860 | 1.744 | 93.8% | 0.1002 | 0.209443 |
| Control, 1.5 | 1.3338 | 3.558 | 1.962 | 55.2% | 0.8603 | 5.281303 |
| Hammer ×2, 1.125 | 0.9278 | 3.443 | 1.936 | 56.2% | 0.5928 | 6.439448 |
| Hammer ×2, 1.5 | 1.3103 | 6.868 | 2.148 | 31.3% | 1.0724 | 12.129223 |
| Hammer ×3, 1.125 | 0.8934 | 4.789 | 1.985 | 41.4% | 0.6711 | 9.918229 |
| Hammer ×3, 1.5 | 1.2530 | 9.420 | 2.213 | 23.5% | 1.0869 | 16.477618 |

Across a fivefold range of release energy the bridle takes 1.74–2.21 mJ over
the same 1.58–1.62 mm flight. The toll depends on the displacement, not on
the hammer's energy, so the soft threshold is simply release kinetic energy
above the toll. With the control toll of 1.840 mJ including return heat and
spring storage, the threshold release speed is 0.959 m/s for the 4 g hammer,
against a 0.964 m/s release: the retained soft strike sits 0.5% above its
threshold, which is why every earlier perturbation flipped it. For 8 and
12 g the thresholds fall to 0.714 and 0.590 m/s, and the soft first impulse
rises 31- and 47-fold.

### The toll is the damper-arm spring, not bridle loss

| Control, 1.125 m/s, release to impact | mJ |
| --- | ---: |
| Hammer kinetic at release | 1.860 |
| Bridle storage change | −0.034 |
| Bridle heat | 0.002 |
| Arm energy change (kinetic + spring) | 1.438 |
| Arm heat | 0.339 |
| Return-damper heat | 0.025 |
| Return-spring storage | 0.071 |
| Hammer kinetic at impact | 0.020 |

Lifting the damper arm a further 1.26 mm against its 200 N/m spring from an
already lifted position takes 1.44 mJ; arm damping takes 0.34 mJ. Bridle
storage is returned and bridle heat is negligible. Halving the bridle rate
loss accordingly changes the soft impulse by only 30% and the strong impulse
by 1.1%. Halving arm damping lowers the toll to 1.713 mJ and the soft strike
rises to 1.704878 mNs at 0.3426 m/s, repeating within limits.

### Arm inertia sets the flight dynamics

Halving the arm mass at fixed damping leaves the toll formula unchanged but
changes the arm's transient: arm heat within the window rises from 0.339 to
0.654 mJ, the hammer never reaches the tine and falls back onto the pedestal,
which then does 1.15 mJ of work on it. At 1.5 m/s the strike falls 6.7%.
The damper-arm subsystem therefore matters as a dynamic load, not only as a
static spring.

### Heavier hammers do not return in time

Every heavier-hammer row fails repetition, with second/first ratios of 1.062,
0.819, 0.912 and 1.095. The return spring supplies 0.042 N at full lift; the
model has no gravity, and a 12 g hammer returns three times slower than the
4 g one. In the ×3 soft row the second gesture catches the hammer at
−7.18 mm, bounces it once and launches it by a pedestal collision at
−2.14 mm, 0.64 mm below the let-off, which the receipt retains as a release
away from the let-off. Hammer mass therefore cannot be raised without
revisiting the return law.

### Numerical evidence

Maximum flight hammer defect is `4.650e-14`, bridle defect `1.322e-13`, arm
defect `9.235e-14` and coupling ledger defect `3.675e-14`. Maximum relative
total energy defect is `4.629e-12`, launch momentum defect `1.309e-11`,
launch work defect `4.309e-12` and key energy defect `5.782e-13`. Maximum
velocity refinement RMSE is 0.17628%, impact refinement error 0.14278%,
kinetic-work-term error 0.27320%, release-time error 0.163 µs, release speed
error 0.007540%, arrival speed error 0.46967%, budget term error 0.27320% and
coupling term error 0.36238%.

Verification passes 141 lab unit tests, 85 loaded CLI/receipt tests, strict
Clippy and formatting. Unit tests check identity closure and wrong-mass
detection on a forced release, and a full control take. Receipt tests verify
settings, identities, release positions, strike consistency with the phase
record, control replay and first-attack decisions.

The [receipt](../references/loaded-flight-budget-validation.json) is 2251707
bytes, SHA-256
`e19058328a660af052bcecb0dd9acd130bb2067056095d5ab392df90bc7eb42d`.
The release cache remains approximately 183 MiB, with no new audio renders.

## Scope and next step

These are one-at-a-time interventions on provisional parameters, not
identified values. Only the original profile, one key, one let-off and one
repetition wait are covered. Bridle work is signed transfer and the
decomposition attributes it within the retained coupling ledger.

The soft threshold is now a stated quantity: release kinetic energy against
a displacement toll set by the damper-arm spring and damping. Next give the
hammer and damper arm their gravitational weight and a return law consistent
with a measured Rhodes hammer mass, then retune the arm spring so that the
damper still seats, and requalify soft threshold, first strike and repetition
against the retained control with the flight budget kept explicit.
