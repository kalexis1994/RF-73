# Finite-inertia key under a prescribed finger force

The [drive onset study](LOADED-DRIVE-ONSET.md) showed that the prescribed
pedestal ramp, not the return regulation, produced the original soft-strike
collision, and that the abrupt stop at the end of travel launched the soft
strike. This experiment replaces the prescribed pedestal trajectory with a
lumped key of finite inertia driven by a step finger force, so that onset,
arrival and release follow from mass, return force and contact laws.

```text
cargo run --locked --release -p rf-73-lab -- loaded-key-inertia --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Impact, lift, first-attack and
repetition outcomes remain separate from numerical qualification.

## Key reduction

The key and pedestal are one mass `m` referred to the pedestal coordinate
`x`, moving from the hammer rest position to the escapement limit over the
retained 10.5 mm travel. A constant 1 N return force acts toward rest. A step
finger force acts during the commanded key-down windows (30–150 ms and
210–330 ms). The pedestal contact force computed by the assembly acts back on
the key with opposite sign. The finger force is

```text
F_finger = 1 N + m v² / (2 L)
```

so that a free key without hammer reaction would reach the nominal speed `v`
exactly at the end of travel. Nominal speed labels therefore describe the
finger force, not the measured arrival.

| Driver | Mass (kg) | End stop | Finger force at 1.125 / 1.5 m/s (N) |
| --- | ---: | --- | ---: |
| Constant slew (retained control) | prescribed | prescribed | — |
| Light key, hard stop | 0.05 | inelastic | 4.013 / 6.357 |
| Light key, felt bed | 0.05 | cubic felt over the last 0.25 mm | 4.013 / 6.357 |
| Heavy key, hard stop | 0.1 | inelastic | 7.027 / 11.714 |

The rest stop is inelastic in every key variant: the arriving kinetic energy
is recorded as a discrete stop loss, and the key remains held while the net
force pushes into the stop. The felt bed has potential `k p³/3` with
`k = 3e10 N/m²`, a 300 Ns/m dashpot and nonadhesive release when the dashpot
pull would exceed the spring push. Its spring force is the discrete gradient
of the potential, so spring work equals the potential change exactly,
including partial entry and exit ticks. Bed heat is the bed force work not
stored in the potential; it equals `c v² h` while engaged and the unrecovered
potential on release, and must be nonnegative every tick. An earlier
Hunt–Crossley rate loss was abandoned before the frozen run because its
velocity-proportional dissipation left the key chattering on the bed for
more than 100 ms.

The key advances one tick ahead of the assembly using the previous tick's
pedestal force, with displacement formed from the same midpoint velocity as
the kinetic change. Its energy identity

```text
finger work - return work - pedestal work - bed storage - bed heat
  - end-stop loss - rest-stop loss = kinetic change
```

therefore closes to rounding, and the pedestal work seen by the key is
compared against the assembly's actuator ledger to expose the one-tick lag.

## Qualification

Per take the assembly pedestal position must match the key within `1e-9 m`,
the key energy identity must hold within `1e-8` relative at every tick, key
speed must stay below the 2 m/s slew bound, the hard travel limit must never
engage under a felt bed, bed heat must be monotone, the key-side pedestal
work must differ from the assembly's by less than 0.1% of its magnitude in
every window, and both arrivals and the first landing must exist. Launch
observation is extended to 30 ms after each key-down.

Refinement between 128 and 256 ticks requires arrival and landing times
within 0.1 ms, arrival, peak and landing speeds within 1% with a 0.01 m/s
floor, window terms within 1% normalized by the term or 0.1% of the window's
summed magnitude, and held positions within 1 µm. All repetition and launch
gates are retained. First-strike comparison against the same-profile
constant slew and between the two hard-stop key masses uses the 5%
impact/speed and 1 ms latency limits. No configuration is selected.

## Retained results

All 32 takes and sixteen refinement pairs qualify. Constant-slew takes replay
the previous receipt exactly apart from the longer launch window, whose 20 ms
event prefix and pre-contact ledgers are identical. Two 256-tick takes (felt
bed at 1.125 m/s in both profiles) have no contact in either gesture; two more
(light hard stop at 1.125 m/s in both profiles) strike once and then miss the
repeated gesture. Eight paired rows meet clean repeatability. Only the
constant-slew rows preserve their own first attack.

### The hammer rides the key and is launched by the stop

With the light key at nominal 1.125 m/s the hammer never leaves the pedestal
during travel. The bridle engages at 39.791 ms with the hammer at 0.563 m/s,
and the pedestal releases the hammer at 49.942 ms, the instant the key hits
its stop, at 0.964 m/s. The hammer then coasts through the escapement gap and
strikes at 52.947 ms with 0.100 m/s. The retained constant slew released its
hammer at 39.333 ms at 0.970 m/s and struck at 0.106 m/s. Two very different
drives with the same hammer speed at pedestal release give the same soft
strike: in this action model the soft strike is set by the hammer speed at
the moment the pedestal stops.

| Light key, hard stop, 256 ticks | Nominal 1.125 m/s | Nominal 1.5 m/s |
| --- | ---: | ---: |
| Arrival after key-down (ms) | 19.942 | 14.822 |
| Arrival speed (m/s) | 0.966 | 1.344 |
| Finger work (mJ) | 42.141 | 66.750 |
| Return work (mJ) | 10.500 | 10.500 |
| Pedestal work into the action (mJ) | 8.235 | 10.904 |
| End-stop loss (mJ) | 23.406 | 45.346 |
| Pre-impact hammer speed (m/s) | 0.100235 | 0.860323 |
| First impulse (mNs) | 0.209443 | 5.281303 |
| Second/first impulse | No second strike | 1.013584 |

The hammer reaction slows the key to 86% and 90% of its nominal free arrival
speed. Less than a fifth of the finger work reaches the action; the largest
share is destroyed in the inelastic stop. The repeated soft gesture arrives
at 0.960 m/s instead of 0.966 m/s, and that 0.6% difference is enough to
lose the strike entirely, so the soft threshold remains marginal.

### The pedestal-loss control no longer helps

In the high-pedestal-loss profile the light hard-stop key gives 0.136936 mNs
at nominal 1.125 m/s against 2.984929 mNs under constant slew, and again
misses the second strike. That control's earlier benefit came from absorbing
the onset collision. Without a collision there is nothing for it to absorb,
and its remaining effect is a slightly lower launch.

### A 0.2 ms felt bed removes the soft strike

The felt bed reaches 87 µm penetration at 1.125 m/s and 113 µm at 1.5 m/s,
stops the key in about 0.2 ms and dissipates 21.7 and 42.5 mJ, comparable to
the inelastic loss. Yet the soft strike disappears in both profiles: the
hammer decelerates with the key instead of being released. At 1.5 m/s it
strikes with 4.710370 mNs, 10.8% below the hard stop, and repeats with a
second/first ratio of 1.000000 in the original profile. Even a very stiff
compliant stop is not abrupt enough to launch the soft strike.

### A heavier key launches harder

Doubling the key mass while keeping the nominal free arrival speed reduces
the hammer's share of the reaction. Arrival speeds rise to 1.044 and
1.420 m/s, the soft strike becomes 1.896984 mNs at 0.372 m/s and repeats with
ratio 0.978706, while the strong strike falls to 0.946537 and fails the 5%
limit. Light and heavy keys differ by more than 5% in every row, so key
inertia is a first-order parameter of this action, not a detail.

### Return and landing

After release the light key lands on its rest stop 25.98 ms later at
0.715 m/s and the heavy key after 35.78 ms at 0.517 m/s. During return the
hammer, still above the pedestal, delivers 2.29 mJ into the key. The
recovery windows close their energy identity with the landing loss as the
only dissipative term.

### Numerical evidence

Maximum key tracking error is exactly zero, key energy defect `2.282e-12`,
pedestal work lag `2.809e-5` and key speed 1.4196 m/s. Maximum relative total
energy defect is `5.000e-12`, launch momentum defect `1.309e-11` and launch
work defect `4.110e-12`. Maximum velocity refinement RMSE is 0.41312%, impact
refinement error 0.48747%, launch impulse-component error 0.48747%,
kinetic-work-term error 0.86782%, key arrival-time error 0.081 µs, arrival
speed error 0.049084%, window term error 0.019299% and held-position error
`5.443e-16 m`. The largest refinement errors belong to the marginal light-key
soft rows, where the strike is close to disappearing.

Verification passes 137 lab unit tests, 81 loaded CLI/receipt tests, strict
Clippy and formatting. Unit tests check that a free key reaches its nominal
speed at the end of travel with an exact energy identity, that the felt bed
stores and dissipates without reaching the hard limit, that a pedestal force
slows the key, and that the discrete-gradient force reproduces the potential
across entry ticks. Receipt tests verify key gates, term signs, held
positions, landing evidence, control replay and first-attack decisions.

The [receipt](../references/loaded-key-inertia-validation.json) is 2666300
bytes, SHA-256
`860b73920390a540a3329ed3fe765811373bf3d4140280af0b133b9229c7d8d3`.
The release cache remains approximately 182 MiB, with no new audio renders.

## Scope and next step

This is a lumped key with a step finger force, not a measured key, finger or
lever geometry. Nominal speeds are free-key labels; the felt bed's held
position is up to 0.25 mm below the prescribed drivers; the rest stop is
inelastic; and the one-tick force lag is retained and measured rather than
removed. Two masses, two forces, two profiles and one repetition wait cannot
establish playing realism.

The soft strike in this action model is a coast after an abrupt stop of the
pedestal, and it disappears under any compliant stop. A real Rhodes action
launches the hammer through the escapement geometry before the key reaches
its bed, and the key bed is felt. Next model the escapement release, in which
the pedestal cam rolls off the hammer at a regulated let-off point before key
bottom, and requalify first-strike and repetition against the retained
constant-slew and light-key controls with the key energy ledger kept
explicit.
