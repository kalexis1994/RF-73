# Damper lift geometry

The [damper seating study](LOADED-DAMPER-SEATING.md) found that the felt
lifts 6.5 mm during a strike, comes back at 0.36 m/s, lands on the tine 27 ms
after key-up and bounces, and that no contact or damping coefficient makes
the tine quiet before it arrives. This experiment changes what sets that
lift and that return: the bridle ratio and slack that couple the hammer to
the damper arm, and the arm spring that throws the felt back, with its
seating force matched.

```text
cargo run --locked --release -p rf-73-lab -- loaded-damper-lift --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Lift, seating, damping onset,
readiness, first-attack and repetition outcomes remain separate from
numerical qualification.

## Frozen matrix

Twenty-four takes cross six cases and two nominal speeds at 128/256 ticks,
all on the original profile with the 4 g hammer, the 0.05 kg key, the sharp
let-off and the 60 ms repetition wait:

| Case | Gravity (m/s²) | Pedestal rate loss (s/m) | Bridle ratio | Bridle slack (mm) | Arm spring (N/m) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control | 0 | 2 | 0.8 | 2 | 200 |
| Settled hammer | 9.81 | 30 | 0.8 | 2 | 200 |
| Bridle ratio 0.5 | 9.81 | 30 | 0.5 | 2 | 200 |
| Bridle slack 4 mm | 9.81 | 30 | 0.8 | 4 | 200 |
| Soft arm, matched seating | 9.81 | 30 | 0.8 | 2 | 100 |
| Ratio 0.5, soft arm | 9.81 | 30 | 0.5 | 2 | 100 |

The settled hammer replays the damper seating study; the gravity-free
control anchors the replay chain. The soft arm halves the spring and moves
its base so that the spring force at the settled rest arm position is
unchanged; that position therefore remains an equilibrium and the rest felt
force matches the settled hammer exactly. A first-order estimate of the base
shift was tried and left the felt force 14% high; the exact match was
adopted before the frozen run.

## Lift observer

The retained damper observer is kept in full. A lift observer adds the rest
felt force, arm offset, arm spring force and felt and bridle compressions,
the maximum felt lift and arm speed during each held gesture, and the
minimum felt force during 50–110 ms after each key-down. Rest values must
agree between resolutions exactly, held lifts within 1% with a 10 µm floor
and held arm speeds within 1% with a 0.01 m/s floor, together with the
retained damper, repetition, launch, key and let-off gates. Flight
identities, release and strike agreement, release-to-impact refinement and
window-end hammer terms are gated; window-end coupling terms are retained
but not gated. First strikes are compared with the control and with the
settled hammer using the 5% impact/speed and 1 ms latency limits, and
readiness keeps its retained limits. No configuration is selected.

## Retained results

All 24 takes and twelve refinement pairs qualify. The control and the
settled hammer replay the damper seating study exactly after removing the
lift record. Every geometry case strikes on both gestures at both speeds and
passes clean repeatability, ten rows in all; the control soft row misses its
repeat and the settled soft row never strikes. No row passes readiness.

### Lift, flight toll and first strike

| First gesture, 256 ticks | Held felt lift at 1.125 / 1.5 (mm) | Bridle work in flight at 1.5 (mJ) | Arm energy in flight (mJ) | First impulse at 1.125 / 1.5 (mNs) |
| --- | ---: | ---: | ---: | ---: |
| Settled hammer | 7.09 / 7.75 | 1.940 | 1.387 | No contact / 5.190309 |
| Bridle ratio 0.5 | 4.30 / 4.76 | 0.735 | 0.477 | 5.149443 / 7.868362 |
| Bridle slack 4 mm | 5.52 / 6.10 | 1.542 | 0.939 | 2.860473 / 6.352182 |
| Soft arm, matched seating | 7.61 / 8.33 | 1.250 | 0.636 | 3.763354 / 6.780271 |
| Ratio 0.5, soft arm | 4.51 / 5.01 | 0.498 | 0.234 | 5.749098 / 8.282059 |

The lift geometry is the largest single lever on the strike found so far.
Lowering the bridle ratio to 0.5 lifts the felt 4.3 instead of 7.1 mm,
cuts the bridle's flight toll from 1.94 to 0.73 mJ, raises the strong strike
by 52% and turns the missing soft strike into 5.15 mNs at 0.81 m/s.
Two millimetres more slack cut the toll by a fifth; a softer arm with the
same seating force lifts more but stores less, cutting it by a third. Every
geometry case therefore fails the first-attack limits against both the
control and the settled hammer, as a change of this size must.

### Felt landing and damping onset

| First key-up, 256 ticks | Felt lands at 1.5 (ms, arm speed m/s) | Settled at 1.125 / 1.5 (ms) | −20 dB at 1.125 / 1.5 (ms) | Felt seated in last 20 ms at 1.125 / 1.5 |
| --- | --- | ---: | ---: | ---: |
| Settled hammer | 26.85, 0.359 | 36.9 / 47.9 | — / 50 | 1.000 / 0.754 |
| Bridle ratio 0.5 | 26.04, 0.187 | 46.4 / 51.0 | 38 / 53 | 0.802 / 0.680 |
| Bridle slack 4 mm | 22.24, 0.346 | 39.8 / 42.8 | 38 / 39 | 1.000 / 0.978 |
| Soft arm, matched seating | 27.31, 0.303 | 39.7 / 56.6 | 29 / 59 | 1.000 / 0.448 |
| Ratio 0.5, soft arm | 26.22, 0.177 | 59.4 / 59.5 | 59 / 60 | 0.415 / 0.374 |

The felt lands 22 to 32 ms after key-up in every case, however far it was
lifted and however soft its spring. The arm is not returning freely under
its spring, whose quarter period is 3.5 ms; it is held up by the bridle
until the hammer has fallen far enough to slacken the strap, so the damping
onset is tied to the hammer's descent from the let-off. Extra slack releases
the arm 4.6 ms earlier and gives the best seating: the strong row's felt is
in contact 97.8% of the last 20 ms and the soft row reaches −40 dB at 56 ms,
the only case to do so. The lower ratio halves the landing speed but the
felt still bounces five to seven times, and combining it with the soft arm
leaves too little seating force to stop eight bounces. The soft arm alone
lands the felt gently after a soft gesture but bounces for 57 ms after a
strong one.

### Readiness

With 4 mm of slack the strong gesture leaves the hammer 33 µm from rest, the
arm 69 µm from rest and the felt seated 97.8% of the time, all within the
readiness limits; only the hammer's residual vibration on the pedestal
contact, 0.023 m/s against 0.01 m/s, still fails. That residual is identical
in every settled case and is unaffected by the damper geometry.

### Numerical evidence

Maximum relative total energy defect is `3.934e-12`, launch momentum defect
`1.309e-11`, launch work defect `3.631e-12`, key energy defect `6.463e-13`
and flight hammer defect `4.188e-14`. Rest quantities agree between
resolutions exactly; maximum held-lift error is 0.001044%, held arm speed
error 0.001324%, felt reseat-time error 0.163 µs, settling-time error
0.732 µs and onset-bin error zero. Maximum velocity refinement RMSE is
0.17628% and impact refinement error 0.14278%.

Verification passes 149 lab unit tests, 116 DSP unit tests, 93 loaded
CLI/receipt tests, strict Clippy and formatting. Unit tests check the
declared geometry per case and that the soft arm reproduces the settled rest
felt force within `1e-6`. Receipt tests verify settings, exact rest matching,
reduced lift under lower ratio and larger slack, control and settled-hammer
replay, damper evidence and first-attack decisions.

The [receipt](../references/loaded-damper-lift-validation.json) is 2540380
bytes, SHA-256
`86a9b2ffaca516ed5d550e731b8bdf524e1a28c988a3364809841748a7d33b73`.
The release cache remains approximately 183 MiB, with no new audio renders.

## Scope and next step

Bridle ratio, slack and arm spring are provisional reductions of the damper
linkage, not measured geometry, and the damping onset is a raw level measure
in 1 ms bins. One key, one let-off, one profile and one repetition wait are
covered.

Twenty-five action blocks have now traced the strike from finger force to
damper seating with exact ledgers, and the damper geometry turns out to
govern both the soft threshold and the damping onset. What none of them has
produced is sound. Next assemble an audible action candidate from the
retained findings, the weighted 4 g hammer on the let-off key with pedestal
loss 30, felt loss 40 and 4 mm bridle slack, render soft, strong and repeated
strikes through the retained pickup and circuit alongside the gravity-free
control, and qualify the renders with the existing energy, refinement and
headroom gates so the action work can be heard before it is calibrated.
