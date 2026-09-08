# Damper felt re-seating after key-up

The [landing study](LOADED-LANDING.md) settled the hammer on the returned
pedestal with a strongly dissipative pedestal contact and left two readiness
defects: a residual hammer vibration and a damper felt that was seated only
three quarters of the time before the next gesture. This experiment measures
the felt's own landing on the tine and the damping it produces, and compares
felt contact loss, damper-arm damping and damper-arm mass with the hammer
already settled.

```text
cargo run --locked --release -p rf-73-lab -- loaded-damper-seating --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Seating, damping onset, readiness,
first-attack and repetition outcomes remain separate from numerical
qualification.

## Frozen matrix

Twenty-four takes cross six cases and two nominal speeds at 128/256 ticks,
all on the original profile with the 4 g hammer, the 0.05 kg key, the sharp
let-off and the 60 ms repetition wait:

| Case | Gravity (m/s²) | Pedestal rate loss (s/m) | Felt rate loss (s/m) | Arm damping (Ns/m) | Arm mass (kg) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control | 0 | 2 | 5 | 0.5 | 0.001 |
| Settled hammer | 9.81 | 30 | 5 | 0.5 | 0.001 |
| Felt rate loss 15 | 9.81 | 30 | 15 | 0.5 | 0.001 |
| Felt rate loss 40 | 9.81 | 30 | 40 | 0.5 | 0.001 |
| Arm damping 2 | 9.81 | 30 | 5 | 2 | 0.001 |
| Arm mass 2 g | 9.81 | 30 | 5 | 0.5 | 0.002 |

The settled hammer is the landing study's pedestal-loss-30 case and replays
it; the gravity-free control anchors the replay chain. Arm stiffness
(200 N/m), felt stiffness, bridle, key, tuning, pickup and circuit stay
fixed.

## Damper observer

For each key-up window (150–210 ms and 330–400 ms) the observer retains
felt/tine contact entry and exit events with the arm velocity before and
after each transition, capped at 32 with overflow failing qualification. It
records the first felt reseat after key-up, the felt exits after reseating,
the settling time as the last felt entry whose contact persists to the
window end, the felt contact fraction over the window and the maximum felt
lift. Damping onset is measured on the raw output voltage: its mean square
in 1 ms bins after key-up is expressed in decibels relative to the
millisecond before key-up, and the first bins at or below −20 dB and −40 dB
are retained. For rows without a strike the reference is residual noise
and the onset is undefined; the retained bins then rise, because the felt's
own landing excites the tine.

Refinement between resolutions requires felt reseat times within 0.1 ms,
settling times within 1 ms, onset times within one bin, contact fractions
within 0.01 and equal exit counts. All repetition, launch, key and let-off
gates are retained; flight identities, release and strike agreement,
release-to-impact refinement and window-end hammer terms are gated, and
window-end coupling terms are retained but not gated. First strikes are
compared with the control and with the settled hammer using the 5%
impact/speed and 1 ms latency limits, and readiness keeps its retained
limits. No configuration is selected.

## Retained results

All 24 takes and twelve refinement pairs qualify. The control and the
settled hammer replay the landing study exactly after removing the damper
record. Every strong row except arm damping 2 strikes once per gesture and
passes clean repeatability; arm damping 2 never strikes. No row passes
readiness.

### The felt lands early and bounces

| First key-up, 256 ticks, 1.5 m/s | Felt lands (ms after key-up) | Arm speed at landing (m/s) | Bounces | Settled (ms) | Contact fraction over 60 ms | Felt seated in last 20 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control | 26.85 | 0.376 | 6 | 57.9 | 0.116 | 0.160 |
| Settled hammer | 26.85 | 0.359 | 5 | 47.9 | 0.302 | 0.754 |
| Felt rate loss 15 | 26.85 | 0.359 | 5 | 44.1 | 0.331 | 0.883 |
| Felt rate loss 40 | 26.85 | 0.360 | 9 | 43.6 | 0.325 | 0.890 |
| Arm damping 2 | 51.14 | 0.015 | 0 | 51.1 | 0.148 | 0.443 |
| Arm mass 2 g | 26.89 | 0.393 | 4 | 50.1 | 0.267 | 0.693 |

The felt lifts 6.5 mm during the strike and comes back under its 200 N/m
spring as the bridle slackens, landing on the tine 26.9 ms after key-up at
0.36 m/s, before the key itself has landed. It then bounces for 17 to
31 ms. Felt rate loss shortens the settling from 47.9 to 43.6 ms and raises
the seated fraction before the repeat from 0.754 to 0.890, just under the
0.9 readiness limit, without changing the strong strike by more than 0.001%.
Damping the arm to 2 Ns/m removes the bounce but the arm then creeps back
in 51 ms at 0.015 m/s, and its drag also destroys the strike. Doubling the
arm mass lands harder, bounces longer, raises the strong strike by 9% and
lets the soft strike through at 1.131394 mNs.

### Damping onset

| First key-up, 256 ticks, 1.5 m/s | −20 dB reached (ms after key-up) | Level at 30 / 40 / 50 / 59 ms (dB) |
| --- | ---: | --- |
| Control | 58 | −4.5 / −10.0 / −17.3 / −18.7 |
| Settled hammer | 50 | −5.9 / −9.0 / −21.2 / −31.8 |
| Felt rate loss 15 | 43 | −5.4 / −14.3 / −30.2 / −29.1 |
| Felt rate loss 40 | 42 | −4.3 / −16.1 / −27.0 / −28.6 |
| Arm mass 2 g | 49 | −5.9 / −19.3 / −26.1 / −34.5 |

Nothing damps the tine before the felt lands at 27 ms, and the bouncing
felt takes 15 to 30 ms more to bring the output down 20 dB; −40 dB is not
reached within 60 ms in any case. Felt rate loss brings the −20 dB point
forward from 50 to 42 ms. The second window, 60 ms after a repeated strike,
reaches −20 dB at 8 ms because the felt is already in contact when the
second key is released.

### First strike and readiness

Felt rate loss leaves the strong first impulse within 0.001% of the settled
hammer and within the control's limits; both felt cases pass the first-attack
comparison against the settled hammer. Second/first ratios lie between 0.993
and 0.996 in the striking weighted rows. Readiness still fails everywhere:
the hammer's residual vibration on the pedestal contact stays at 0.023 m/s
against the 0.01 m/s limit in every settled case, and the felt reaches at
most 89% seating against 90%.

### Numerical evidence

Maximum relative total energy defect is `3.934e-12`, launch momentum defect
`1.309e-11`, launch work defect `3.631e-12`, key energy defect `4.255e-13`
and flight hammer defect `1.942e-13`. Maximum felt reseat-time error is
0.163 µs, settling-time error 0.326 µs, onset-bin error zero and contact
fraction error `1.98e-5`. Maximum velocity refinement RMSE is 0.23162% and
impact refinement error 0.14278%.

Verification passes 147 lab unit tests, 116 DSP unit tests, 91 loaded
CLI/receipt tests, strict Clippy and formatting. A unit test drives the
window with a synthetic decaying output and a bouncing felt and checks
reseat, bounce count, settling, contact fraction, lift, reference level and
both onsets. Receipt tests verify settings, event alternation, onset
consistency with the retained level bins, control and settled-hammer replay,
first-attack decisions and that readiness implies a seated felt.

The [receipt](../references/loaded-damper-seating-validation.json) is
2208397 bytes, SHA-256
`9817786106b70899ceda8394a7574f8e9534a15dc979c7eddeebf619daabebc4`.
The release cache remains approximately 183 MiB, with no new audio renders.

## Scope and next step

Felt rate loss, arm damping and arm mass are provisional coefficients, not a
measured damper felt or arm, and the damping onset is a raw level measure in
1 ms bins rather than a perceptual damping time. One key, one let-off, one
profile and one repetition wait are covered.

The felt's landing is set by its 6.5 mm lift and the 200 N/m arm spring that
throws it back at 0.36 m/s, and no contact or damping coefficient can make
the tine quiet before it arrives. Next model the damper lift geometry: the
bridle ratio and slack set how far the felt lifts, and a softer arm spring
with the same seating force sets how fast it returns. Compare smaller lifts
and matched seating forces under the settled hammer, and requalify damping
onset, readiness and the first strike against the retained control with the
felt landing evidence kept explicit.
