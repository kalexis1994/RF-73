# Loaded launch work and momentum

The [repetition study](LOADED-REPETITION.md) found large first/second attack
differences near the original soft-strike threshold. Two loss controls repeat
consistently but also alter the first attack. This study measures the incoming
hammer state and the work and force impulses accumulated during launch.

```text
cargo run --locked --release -p rf-73-lab -- loaded-launch --output REPORT.json
```

The command requires a new JSON path, retains numerical failures in the
report and returns an error for failed qualification. It does not select
parameters or change the physical model.

## Protocol and state preservation

The complete 24-take repetition matrix is replayed: conditional 70 mm G3;
baseline, return damping 0.1 Ns/m and pedestal rate loss 10 s/m; drive speeds
1.125/1.5 m/s; second key-down 60/300 ms after first key-up; 128/256 internal
ticks per 48 kHz observation frame. All driver timing, tuning, coefficients,
initial preparation, contact and circuit state evolution remain unchanged.

An observer follows the first 20 ms of each key-down. Each observer starts
its accounting from the existing incoming state. Only diagnostic integrals
start at zero; the physical state and cumulative solver ledgers continue
through both strikes. Full repetition reports are retained with additional
launch evidence, allowing an exact replay check against the previous receipt.

## Independent balances

For each integration interval, the observer integrates five signed forces
acting on the hammer using the solver's mean contact forces and midpoint
position and velocity:

| Component | Signed hammer force |
| --- | --- |
| Pedestal | `+F_pedestal` |
| Bridle | `-bridle_ratio * F_bridle` |
| Tine contact | `-F_hammer_contact` |
| Return spring | `-k_return * (q_mid - hammer_rest)` |
| Return damper | `-c_return * v_mid` |

Their time integrals plus incoming momentum must reproduce current `m*v`.
The maximum absolute residual is normalized by incoming momentum magnitude
plus accumulated absolute impulse, with a `1e-12 Ns` floor. This preserves
the signs of return motion and avoids division by nearly zero net momentum.
The relative defect must remain below `1e-8`.

Independent hammer, pedestal and tine-contact work ledgers also start from
the incoming state. Their storage terms retain preload and initial hammer
energy; cumulative solver work and contact heat are differenced against
that state. The six additive terms

```text
incoming kinetic energy
+ pedestal work on the hammer
- hammer work into the bridle
- hammer work into tine contact
- return-damper heat
+ incoming return-spring potential - current return-spring potential
= current hammer kinetic energy
```

are retained at initialization, immediately before the first contact and at
20 ms. The existing relative work-defect gates remain below `1e-8`. Tests
independently reconstruct both balances and verify that using the wrong mass
fails the momentum observer on a moving incoming state.

Each launch also records its peak pre-impact hammer velocity and time, plus
force entry/exit events at the hammer/tine, pedestal and bridle ports. Events
retain positions, velocities, compression, mean force and cumulative impulse
and kinetic-work terms. At most 64 events are retained per launch; overflow
fails qualification.

## Refinement and interpretation

All previous repetition energy, exchange, velocity and impact checks remain
active. Additional checks compare the pre-contact and 20 ms force impulses
component by component within 1%, using a `1e-9 Ns` denominator floor, and
each kinetic-work term within 1%, using a `1e-10 J` floor. Peak pre-impact
velocity must refine within 1%. Contact event sequences must match within
each port, with times agreeing within 0.1 ms. Per-port comparison avoids
mistaking the ordering of simultaneous joint-contact events for physics.

The report subtracts the first pre-impact ledger from the second. Differences
in the six energy terms reconstruct the kinetic-energy difference. The
incoming-velocity difference plus differences in force impulses divided by
mass reconstruct the pre-impact velocity difference. These are accounting
identities along the observed trajectories. They do not independently vary
hammer, tine, arm or circuit state and therefore do not isolate their causal
effects from one another.

## Retained results

All 24 takes and twelve refinement pairs qualify, covering 48 observed
launches and 448 force transitions. The largest event list contains twelve
entries, below the 64-event cap. Removing the added launch records reproduces
every preceding repetition take exactly, including states, work, impact,
velocity convergence and functional decisions.

### The soft first strike loses speed after its initial launch

At 1.125 m/s pedestal slew, the original hammer reaches 1.548136 m/s at
30.987386 ms but has only 0.106164 m/s immediately before impact at
42.315511 ms. It first leaves the pedestal at approximately 31.076 ms,
engages the bridle at 32.007 ms, contacts the pedestal again at 36.523 ms
and leaves it at 39.333 ms. The low impact speed follows substantial
earlier motion and subsequent coupled contact evolution.

| Soft first strike | Peak hammer speed (m/s) | Pre-impact speed (m/s) | Pedestal re-entry (ms) | Pedestal work (mJ) | Pre-impact kinetic energy (mJ) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 1.548136 | 0.106164 | 36.523 | 9.748414 | 0.022541 |
| Return damping 0.1 | 1.538946 | 0.525539 | 35.917 | 11.446115 | 0.552382 |
| Pedestal rate loss 10 | 1.223938 | 0.532006 | 33.068 | 10.151808 | 0.566060 |

All three soft first strikes leave the pedestal again at about 39.333 ms,
when its commanded upward travel ends. The controls change re-entry and
the preceding transfer even though the key gesture is unchanged. Higher
return damping leaves nearly the same peak speed but changes later impact
speed strongly; higher pedestal loss gives a lower peak yet a larger impact
speed. Peak launch speed alone is therefore a poor substitute for impact
speed in this matrix.

The baseline pre-impact energy is the small remainder
`9.748414 - 9.100461 - 0.337374 - 0.288037 = 0.022541 mJ`, with rounding:
pedestal work minus bridle work, return heat and increased spring storage.
Bridle work is signed transfer, not entirely dissipated heat. In the
return-damping control, increased pedestal work of 1.697701 mJ exceeds
the additional return heat of 1.033086 mJ and additional bridle transfer of
0.134802 mJ. Its larger impact energy remains consistent with passivity and
the work supplied by the prescribed driver.

### Incoming motion does not explain the repeated strike by itself

For the baseline soft strike repeated after 60 ms, the increase in pre-impact
kinetic energy is 0.613032 mJ. Its additive second-minus-first terms are:

| Contribution | Difference (mJ) |
| --- | ---: |
| Incoming kinetic energy | +0.044370 |
| Pedestal work | +0.701499 |
| Negative bridle work | -0.170019 |
| Negative tine-contact work before impact | 0 |
| Negative return heat | +0.034025 |
| Released return-spring potential | +0.003157 |

Incoming velocity is +0.148946 m/s, while pre-impact velocity increases by
0.457562 m/s. The force-impulse differences account for the remaining
0.308616 m/s. The largest positive momentum contribution is a less negative
bridle impulse, despite greater energy transfer into the bridle: impulse
integrates force over time, while work weights force by displacement.
These statements concern different balances and are not contradictory.

After the 300 ms wait, incoming velocity is instead -0.040112 m/s, but
pre-impact velocity still increases by +0.225368 m/s. The increased
pre-impact kinetic energy is 0.197286 mJ, with only 0.003218 mJ from incoming
kinetic energy. The changed contact-work history must remain part of the
explanation; applying a simple correction for incoming hammer velocity
would miss it. These ledgers do not isolate the other coupled states as
independent causes.

### Numerical evidence

Maximum relative momentum defect is `2.447e-13` and independent work defect
`1.089e-12`. Maximum component-wise impulse refinement error is 0.001950%,
kinetic-work refinement error 0.001738%, peak-velocity relative error
`7.204e-9` and per-port event-time difference 0.245 microseconds.

Verification passes 127 lab unit tests, 23 loaded CLI/receipt tests, strict
Clippy and formatting. Tests reconstruct signed kinetic and momentum sums,
incoming pedestal storage, contact timing and second-minus-first identities,
and preserve the entire historical repetition result.

The [receipt](../references/loaded-launch-validation.json) is 1967153 bytes,
SHA-256 `416648f1977712f75beb73f92172634fe3211106967e8eb1c65bc837ce4ba01a`.
The release cache remains approximately 195 MiB. No new WAVs are generated.

## Scope and next step

No state reset, time-switched damping, additional force law, source fit,
audio render or production default is introduced. The observations diagnose
the declared conditional model; they do not validate actual instrument
regulation or establish perceptual realism. A change to launch geometry or
regulation requires its own first-strike and repetition qualification.

The [terminal drive study](LOADED-DRIVE-RELEASE.md) now compares a smooth
terminal stop with the original and a matched-duration linear trajectory,
keeping the prescribed driver's work explicit. Preserve successful repetition
controls and their
first-attack tradeoffs; do not treat a velocity correction or a gain change
as a physical solution. The present abrupt end of constant-slew travel also
needs to be distinguished from an intrinsic action response before adopting
a new regulation or contact law.
