# Hammer landing on the returned pedestal

The [gravity study](LOADED-GRAVITY.md) showed that the key catches every
hammer about 32 ms after release, after which the hammer bounces off the
pedestal and is still airborne at the repeat command. This experiment
measures that landing and compares pedestal contact loss and hammer return
damping as ways of settling the hammer before the next gesture.

```text
cargo run --locked --release -p rf-73-lab -- loaded-landing --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Settling, readiness, first-attack
and repetition outcomes remain separate from numerical qualification.

## Frozen matrix

Twenty-four takes cross six cases and two nominal speeds at 128/256 ticks,
all on the original profile with the 4 g hammer, the 0.05 kg key, the sharp
let-off and the 60 ms repetition wait:

| Case | Gravity (m/s²) | Pedestal rate loss (s/m) | Return damping (Ns/m) |
| --- | ---: | ---: | ---: |
| Control | 0 | 2 | 0.025 |
| Gravity 4 g | 9.81 | 2 | 0.025 |
| Gravity, pedestal loss 10 | 9.81 | 10 | 0.025 |
| Gravity, pedestal loss 30 | 9.81 | 30 | 0.025 |
| Gravity, return damping 0.1 | 9.81 | 2 | 0.1 |
| Gravity, pedestal loss 30, return damping 0.1 | 9.81 | 30 | 0.1 |

The interventions ride on the weighted hammer. A first matrix applied them
to the gravity-free control and found that nothing can settle: once a
gravity-free hammer bounces, only the 4 N/m return spring with its 200 ms
period brings it back, so even a restitution of 0.06 left the hammer
0.68 mm off the pedestal at the repeat command. That matrix was discarded
before freezing; the gravity-free control is kept as the replay anchor and
the weighted 4 g case replays the gravity study.

## Landing observer

For each key-up window (150–210 ms and 330–400 ms) the observer retains
pedestal contact entry and exit events with the hammer velocity before and
after each transition, capped at 32 with overflow failing qualification. It
records the first reseat, defined as pedestal contact regained with the
pedestal back at rest, together with the landing speed; the maximum upward
hammer speed within 20 ms of reseating as the rebound speed and their ratio
as restitution; the number of pedestal exits after reseating; the settling
time, defined as the last pedestal entry whose contact persists to the
window end; and the lowest hammer position.

The first window must refine within 0.1 ms in reseat time, 1% in landing and
rebound speeds with a 0.01 m/s floor, 1 ms in settling time, 10 µm in lowest
position and with equal exit counts; the truncated second window needs
agreeing reseat time and landing speed. All repetition, launch, key and
let-off gates are retained. Flight identities, release and strike agreement,
release-to-impact refinement and window-end hammer terms are gated;
window-end coupling terms are retained but not gated. First strikes are
compared with the control using the 5% impact/speed and 1 ms latency limits,
and readiness and repeatability keep their retained limits. No configuration
is selected.

## Retained results

All 24 takes and twelve refinement pairs qualify. The control and the
weighted 4 g case replay the gravity study exactly after removing the
landing record. Every strong row strikes once per gesture and passes clean
repeatability; every weighted soft row misses, as in the gravity study, and
the control soft row misses its repeat. No row passes readiness.

### The landing itself

| First key-up, 256 ticks, 1.5 m/s | Landing speed (m/s) | Rebound (m/s) | Restitution | Exits after reseat | Settled (ms after key-up) | Lowest (mm) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Control | 0.583 | 0.322 | 0.552 | 1 | never in 60 ms | −12.181 |
| Gravity 4 g | 0.660 | 0.340 | 0.515 | 1 | never in 60 ms | −12.193 |
| Gravity, pedestal loss 10 | 0.659 | 0.097 | 0.147 | 2 | never in 60 ms | −12.140 |
| Gravity, pedestal loss 30 | 0.659 | 0.033 | 0.049 | 1 | 39.5 | −12.104 |
| Gravity, return damping 0.1 | 0.572 | 0.313 | 0.547 | 1 | never in 60 ms | −12.179 |
| Gravity, loss 30, damping 0.1 | 0.576 | 0.032 | 0.056 | 1 | 39.8 | −12.099 |

The hammer leaves the descending pedestal 10 ms after key-up, is recaught
1.4 ms later, leaves again at 25 ms as the key slows, and lands on the
resting pedestal at 31.5 to 32.2 ms at 0.57 to 0.66 m/s. With the retained
pedestal loss it rebounds at more than half its landing speed. Raising the
pedestal rate loss to 10 s/m cuts restitution to 0.15 but the hammer still
bounces twice and is 0.42 mm off the pedestal at the repeat command. At
30 s/m restitution falls to 0.05, the hammer settles 39.5 ms after key-up
and sits 12 µm from its prepared rest at the repeat command, still
vibrating on the pedestal contact at 0.023 m/s. Return damping barely
changes the landing: it damps the hammer's flight, not its collision.

### Readiness before the repeat

| 20 ms before the repeat, 256 ticks, 1.5 m/s | Hammer position error (mm) | Hammer speed (m/s) | Felt contact fraction | Arm position error (mm) | Readiness |
| --- | ---: | ---: | ---: | ---: | --- |
| Control | 3.706 | 0.302 | 0.160 | 0.761 | Fail |
| Gravity 4 g | 3.300 | 0.246 | 0.323 | 0.440 | Fail |
| Gravity, pedestal loss 10 | 0.417 | 0.089 | 0.754 | 0.080 | Fail |
| Gravity, pedestal loss 30 | 0.012 | 0.023 | 0.754 | 0.080 | Fail |
| Gravity, return damping 0.1 | 2.852 | 0.199 | 0.627 | 0.124 | Fail |
| Gravity, loss 30, damping 0.1 | 0.012 | 0.022 | 0.768 | 0.084 | Fail |

Pedestal loss 30 brings the hammer within the 0.1 mm position limit but
leaves it above the 0.01 m/s speed limit, and the damper felt is in contact
for only 75% of the window against the 90% limit: the damper arm also
bounces when it re-seats on the tine. Readiness therefore now fails on two
remaining items, the residual hammer vibration on the pedestal contact and
the felt re-seating, neither of which the hammer interventions address.

### First strike and repetition

Pedestal loss changes the strong first impulse by 0.8% and 1.0% and keeps
the release at the let-off; both loss cases preserve the control's first
attack within the 5% limits. Return damping 0.1 adds 0.1 mJ to the flight
toll and cuts the strong impulse by 6.5%, failing the first-attack limits.
Second/first ratios lie between 0.988 and 1.000 in every weighted strong
row, with identical latency to the microsecond, so the repeated strike is
already consistent even before the hammer settles: the second gesture
catches the bouncing hammer and carries it to the same let-off release.

### Numerical evidence

Maximum relative total energy defect is `5.528e-12`, launch momentum defect
`1.309e-11`, launch work defect `5.047e-12`, key energy defect `4.310e-13`
and flight hammer defect `9.617e-14`. Maximum reseat-time error is 0.081 µs,
landing speed error 0.000153%, rebound speed error 0.000056%, settling-time
error 0.081 µs and lowest-position error 0.11 nm. Maximum velocity
refinement RMSE is 0.17628% and impact refinement error 0.14278%.

Verification passes 145 lab unit tests, 116 DSP unit tests, 89 loaded
CLI/receipt tests, strict Clippy and formatting. A unit test drives the
landing window with a synthetic bounce and checks reseat time, landing and
rebound speeds, restitution, exit count, settling time and event capture.
Receipt tests verify settings, event alternation, reseat with the pedestal
at rest, restitution consistency, control and weighted replay of the gravity
study, first-attack decisions and that readiness implies settling.

The [receipt](../references/loaded-landing-validation.json) is 2147835
bytes, SHA-256
`44e45c538c6768fe5798c3ae7c0b6ba15dcf5708a403c80dbf4fbacb6ec04e23`.
The release cache remains approximately 183 MiB, with no new audio renders.

## Scope and next step

The pedestal rate loss is a provisional Hunt–Crossley coefficient, not a
measured pedestal felt or hammer tip, and the 60 ms second window is
truncated at 70 ms. One key, one let-off, one profile and one repetition
wait are covered.

A strongly dissipative pedestal landing settles the hammer within 40 ms of
release without changing the strike. What still fails readiness is the
damper arm: its felt bounces on the tine and is seated only three quarters
of the time when the next gesture arrives. The
[damper seating study](LOADED-DAMPER-SEATING.md) now traces that felt
landing and its damping onset under felt loss, arm damping and arm mass.
