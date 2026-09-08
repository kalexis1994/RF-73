# Escapement let-off before key bottom

The [key inertia study](LOADED-KEY-INERTIA.md) showed that the hammer rides
the key and is launched only when the key hits its stop, and that any
compliant stop removes the soft strike. A real action releases the hammer
through its escapement geometry before the key reaches the bed. This
experiment adds a regulated let-off to the finite-inertia key: the effective
pedestal height becomes a monotone map of key position that stops rising at
the let-off point while the key continues into aftertouch.

```text
cargo run --locked --release -p rf-73-lab -- loaded-letoff --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Impact, lift, first-attack and
repetition outcomes remain separate from numerical qualification.

## Let-off reduction

The key is the retained 0.05 kg lumped key with a 1 N return force and a
step finger force. Its travel now extends 1 mm past the escapement top to an
inelastic bed. The pedestal height seen by the assembly is `map(x)` of the
key position `x`:

| Driver | Let-off top | Roll-off band | Aftertouch | Hammer flight gap |
| --- | ---: | ---: | ---: | ---: |
| Constant slew (retained control) | prescribed | — | — | 1.5 mm |
| Sharp let-off | −1.5 mm | 0 | 1.0 mm | 1.5 mm |
| Rolled let-off | −1.5 mm | 0.6 mm of key travel | 1.0 mm | 1.5 mm |
| Early let-off | −2.5 mm | 0 | 2.0 mm | 2.5 mm |

The sharp map is `min(x, top)`. The rolled map follows the key with unit
slope until `top - 2b/3`, then `start + b(u - u³/3)` with `u` the fraction of
the band, reaching the top with zero slope after `b = 0.6 mm` of key travel;
velocity is continuous at both ends. The map has slope in `[0, 1]`, so the
effective pedestal never moves faster than the key. Unit tests check
monotonicity, continuity, the slope limits and that the map reaches the top.

The reaction on the key is the pedestal force times the average map slope
over the step, obtained together with the acceleration by bisection. The
key-side pedestal work therefore equals pedestal force times pedestal
displacement exactly, and the key energy identity closes to rounding through
the let-off. After let-off the hammer's support stays at the top height while
the key is depressed; on return the support descends once the key rises back
through the let-off point. The finger force is `1 N + m v² / (2 L)` with `L`
the travel to the let-off start, so a free key reaches the nominal speed at
let-off; the early let-off therefore uses a slightly larger force over its
shorter travel.

Generalizing the key simulation to carry this map changed none of the
retained key results: rerunning the key inertia study with the new code
reproduces its receipt byte for byte.

## Qualification

All key gates are retained: the assembly pedestal must match the mapped key
position within `1e-9 m`, the key energy identity must hold within `1e-8`
relative every tick, key speed must stay below 2 m/s, and key-side pedestal
work must differ from the assembly's by less than 0.1% in every window. Both
let-offs must complete with a recorded hammer state, both bed arrivals and
the first landing must exist. Refinement adds let-off begin and complete
times within 0.1 ms, key and hammer speeds at let-off within 1% with a
0.01 m/s floor, and finger work spent after let-off within 1% normalized by
the term or 0.1% of the window magnitude. All repetition, launch and key
refinement gates remain. First strikes are compared against the same-profile
constant slew and against the sharp let-off with the 5% impact/speed and
1 ms latency limits. No configuration is selected.

## Retained results

All 32 takes and sixteen refinement pairs qualify. Constant-slew takes replay
the key inertia receipt exactly, including the 30 ms launch windows. Four
256-tick takes (rolled and early let-off at 1.125 m/s in both profiles)
never strike; two more (sharp let-off at 1.125 m/s) strike once and miss the
repeated gesture. Five paired rows meet clean repeatability. Only the
constant-slew rows preserve their own first attack.

### A sharp let-off reproduces the hard-stop launch and frees the key

With the sharp let-off at the retained top, the hammer trajectory agrees with
the key inertia hard-stop rows within `1.03e-5` relative in every first-strike
quantity: the hammer leaves the pedestal at 49.942 ms at 0.964 m/s and strikes
at 0.100 m/s with 0.209443 mNs at 1.125 m/s, and with 5.281303 mNs at
1.5 m/s. The key, no longer carrying the hammer, accelerates through its
1 mm aftertouch from 0.966 to 1.026 m/s and lands on the bed 1.0 ms after
let-off. The finger spends 4.013 mJ after let-off at 1.125 m/s and 6.357 mJ
at 1.5 m/s, exactly the force times the aftertouch, all of which the bed
destroys. Launch and bed are now separate events, and the bed law can no
longer alter the strike.

### A 0.6 mm roll-off removes the soft strike

| 256 ticks, original profile | Release speed (m/s) | Release time (ms) | Pre-impact speed (m/s) | First impulse (mNs) | Second/first |
| --- | ---: | ---: | ---: | ---: | ---: |
| Sharp let-off, 1.125 | 0.964 | 49.942 | 0.100235 | 0.209443 | No second strike |
| Rolled let-off, 1.125 | 0.915 | 49.602 | No contact | No contact | Undefined |
| Early let-off, 1.125 | 0.978 | 47.939 | No contact | No contact | Undefined |
| Sharp let-off, 1.5 | 1.334 | 44.822 | 0.860323 | 5.281303 | 0.960757 |
| Rolled let-off, 1.5 | 1.315 | 44.518 | 0.834286 | 5.102885 | 0.934133 |
| Early let-off, 1.5 | 1.366 | 43.363 | 0.536714 | 3.021146 | 0.835194 |

Under the rolled map the hammer separates inside the band, 0.47 ms after it
begins, while the pedestal still pushes with 1.47 N; it leaves at 0.915 m/s,
5% below the sharp release, and that is enough to lose the soft strike in
both profiles. At 1.5 m/s the rolled strike stays within 5% of the sharp one
in the original profile but fails repetition. A let-off spread over 0.6 mm
of key travel, about 0.6 ms, already behaves like the compliant stops of the
previous studies. The soft strike in this model needs a release sharper than
that.

### Regulation distance is a first-order parameter

Moving the let-off 1 mm earlier releases the hammer slightly faster but adds
1 mm of flight against the bridle. The soft strike disappears, and the strong
strike falls by 42.8% in impulse and 37.6% in pre-impact speed, with the
second/first ratio dropping to 0.835. The pedestal-loss profile behaves the
same way, so this is a geometric effect rather than a contact-loss effect.

### The bridle consumes the flight

From release to impact the launch ledger attributes the loss almost entirely
to the bridle. For the sharp soft strike the hammer carries 1.860 mJ at
release; the bridle takes 1.744 mJ, the return spring stores 0.071 mJ and
return damping 0.025 mJ, leaving 0.020 mJ at impact after a 3.0 ms flight
across 1.5 mm. The early let-off's 2.5 mm flight costs 2.98 mJ of bridle work
at 1.5 m/s against 1.96 mJ for the sharp release. The provisional bridle and
damper-arm load therefore governs the soft threshold once the launch is
geometric, and its calibration is the next physical question.

### Return and landing

With the deeper travel the light key lands 31.26 ms after release at
0.737 m/s (32.71 ms and 0.715 m/s for the early let-off) instead of the
25.98 ms of the previous study. The recovery windows close their identity
with the landing loss as the only dissipative term.

### Numerical evidence

Maximum key tracking error is exactly zero, key energy defect `6.579e-13`
and pedestal work lag `4.716e-5`. Maximum relative total energy defect is
`5.004e-12`, launch momentum defect `1.345e-11` and launch work defect
`4.275e-12`. Maximum velocity refinement RMSE is 0.41312%, impact refinement
error 0.48747%, kinetic-work-term error 0.86782%, let-off begin-time error
0.081 µs, key speed error at let-off 0.000917%, hammer speed error at
let-off 0.003053% and aftertouch work error 0.012657%. The largest refinement
errors again belong to the marginal sharp soft rows, which replay the key
study.

Verification passes 139 lab unit tests, 83 loaded CLI/receipt tests, strict
Clippy and formatting. Unit tests cover the map properties, exact ratio
accounting under a constant pedestal force, free-key let-off at nominal
speed and the retained key tests. Receipt tests verify map parameters,
let-off ordering before the bed, hammer states at release, aftertouch work,
control replay and first-attack decisions.

The [receipt](../references/loaded-letoff-validation.json) is 2605966 bytes,
SHA-256
`9af66ba9a2f7194593636cd2d550b9069233d1464877eed6fc63e3006b4f7da8`.
The release cache remains approximately 183 MiB, with no new audio renders.

## Scope and next step

The map is a kinematic reduction of the cam's velocity ratio, not a measured
cam, pedestal pad or hammer geometry, and the support after let-off is held
at the top height. One key mass, two forces, two profiles and one repetition
wait cannot establish playing realism. Nominal speeds remain free-key labels.

The launch is now geometric and independent of the key bed, and the soft
threshold is set by the hammer speed at release against the bridle load
during a 1.5 mm flight. Next identify the hammer mass and the bridle and
damper-arm load through the flight ledger: vary hammer mass and bridle
stiffness, slack and arm loading under the sharp let-off, retain the flight
energy budget from release to impact, and requalify first-strike and
repetition against the retained constant-slew and sharp let-off controls.
