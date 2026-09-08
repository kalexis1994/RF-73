# Onset acceleration and complete key drive

The [terminal drive study](LOADED-DRIVE-RELEASE.md) showed that the end of
prescribed pedestal travel is a material part of the soft-strike threshold,
while its initial onset remained an abrupt step to constant slew. This
experiment adds two onset ramps and a fully eased drive, and divides each
key-down into segments with their own pedestal work and impulse ledgers.

```text
cargo run --locked --release -p rf-73-lab -- loaded-drive-onset --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Impact, lift, first-attack and
repetition outcomes remain separate from numerical qualification.

## Frozen matrix

Thirty-two takes cross two physical profiles, four driver shapes and two
nominal speeds at 128/256 internal ticks per 48 kHz observation frame. The
profiles are the original conditional 70 mm G3 and its pedestal rate-loss
10 s/m control. Return damping remains 0.025 Ns/m. Geometry, tuning,
structural losses, bridle, arm, pickup and circuit coefficients stay fixed.
Nominal speeds are 1.125 and 1.5 m/s. The two-strike schedule is unchanged:
key-down at 30 ms, key-up at 150 ms, second key-down at 210 ms, second key-up
at 330 ms, then 70 ms of observation. All state persists across gestures.
Key-up motions keep the original constant slew.

Let `L` be pedestal travel (10.5 mm), `v` the nominal speed, `f = 0.2` and
`tr = 2fL/v` the ramp duration. With `s = t/tr`:

| Driver | Onset | Stop | Arrival after key-down |
| --- | --- | --- | --- |
| Constant slew | Abrupt step to `v` | Abrupt | `L/v` |
| Smooth onset | `v(3s² - 2s³)` over the first `fL` | Abrupt | `1.2 L/v` |
| Constant-acceleration onset | `v s` over the first `fL` | Abrupt | `1.2 L/v` |
| Full ease | Smooth onset | Retained smooth deceleration over the last `fL` | `1.4 L/v` |

Both onset ramps cover the same first 20% of travel in the same time, so
they share the arrival time of the previous terminal-ease driver. The smooth
onset has continuous velocity and acceleration with peak acceleration
`0.75 v²/(fL)`; the constant-acceleration onset has `0.5 v²/(fL)` with
discontinuities at both ramp ends. The full ease reproduces the previous
terminal trajectory shifted by the onset ramp; a unit test checks this
against the retained curve. Arrival times at 1.125 m/s are 9.333, 11.2, 11.2
and 13.067 ms; at 1.5 m/s they are 7, 8.4, 8.4 and 9.8 ms. Constant slew
keeps its accumulated-slew implementation and replays the previous receipt
exactly.

## Segment ledgers and qualification

Each key-down is divided into onset, cruise, stop and hold segments at the
analytical boundaries. Ticks that straddle a boundary are split
proportionally in time. Each segment retains actuator pedestal work,
pedestal force times hammer displacement, pedestal impulse, peak pedestal
force, contact time, pedestal exits, an interior finite-difference peak
acceleration and the state at its end. Empty segments (no onset ramp for
constant slew, no stop ramp without full ease) retain zeros and no end state.

Per key-down, actuator work minus hammer-displacement work must equal the
change in pedestal contact storage plus pedestal heat within `1e-8` relative.
Segment times must sum to the 120 ms hold within 1 µs, segment work must sum
to the gesture total within `1e-12` relative, and each measured ramp peak
acceleration must agree with the analytical peak within 0.1%. The driver
itself must track the analytical position within `1e-8 m`, keep key-down
speed within `[0, v]` to `1e-8` relative and arrive within 10 µs.

Refinement between 128 and 256 ticks requires segment work and impulse
within 1%. Differences are normalized by the segment magnitude or 0.1% of
the key-down's summed absolute magnitude, whichever is larger, with `1e-10 J`
and `1e-9 Ns` floors. Without that rule a stationary hold segment's boundary
remainder of about `2e-8 J` dominated a purely relative error. End hammer
velocity refines within 1% with a 0.01 m/s floor. Pedestal exit counts are
compared per key-down, since an exit at the abrupt stop lands on either side
of the cruise/hold boundary at different resolutions. All previous
repetition, launch energy, momentum, impact, velocity and per-port event
gates remain active.

First-strike comparison against the same-profile original and between the
two same-arrival onset shapes uses the existing 5% impact/speed and 1 ms
latency limits. Repeatability retains the within-take criteria. No
configuration is automatically selected.

## Retained results

All 32 takes and sixteen refinement pairs qualify numerically. Constant-slew
takes replay the previous receipt exactly after removing the new driver and
segment records. Two 256-tick takes (full ease at 1.125 m/s in both profiles)
have no hammer/tine contact and remain explicit; the other fourteen retain one
contact per held gesture and none during recovery or release. Every take
preserves both felt-lift windows. Nine paired rows meet clean repeatability,
with identical decisions at both resolutions.
Only the constant-slew rows preserve their own first attack; every ramped
driver fails the 1 ms latency limit because arrival is later by definition.

### The abrupt onset throws the hammer off the pedestal

In the original driver the pedestal reaches nominal speed in one tick. The
hammer leaves the pedestal 1.076 ms later at 1.547 m/s, above the 1.125 m/s
pedestal speed, engages the bridle at 32.007 ms, re-enters the pedestal at
36.523 ms at 0.662 m/s and is carried until the pedestal stops at 39.333 ms.
Peak pedestal force during travel is 20.40 N. Under either onset ramp the
hammer stays seated through the whole ramp with zero exits, reaches
1.186 m/s (smooth) or 1.159 m/s (constant acceleration) at the end of the
ramp, and leaves the pedestal only after the bridle engages at 34.237 ms,
re-entering within 0.22 ms. Peak pedestal force falls to 3.42 N and 3.05 N.

| Original profile, 1.125 m/s, 256 ticks | Onset work (mJ) | Peak pedestal force (N) | Pre-impact speed (m/s) | First impulse (mNs) | Second/first impulse |
| --- | ---: | ---: | ---: | ---: | ---: |
| Constant slew | 0 | 20.40 | 0.106164 | 0.237325 | 13.509446 |
| Smooth onset | 2.932046 | 3.42 | 0.550203 | 3.113393 | 1.117125 |
| Constant-acceleration onset | 2.848203 | 3.05 | 0.542726 | 3.060499 | 1.175248 |
| Full ease | 2.932046 | 3.42 | No contact | No contact | Undefined |

The onset-ramped soft strike therefore reaches the tine at 0.55 m/s, the
same region as the two loss controls of the [launch study](LOADED-LAUNCH.md)
(0.526 and 0.532 m/s), while the original 0.106 m/s strike and its 13.5
second/first ratio follow from the pedestal/hammer collision at the abrupt
step. The ramped rows still fail 5% repeatability (11.7% and 17.5% impulse
change), so a ramp alone does not complete soft repetition in this profile.

### The high-pedestal-loss profile already absorbs the onset collision

| Profile pedestal loss 10 | Speed (m/s) | Driver | First impulse (mNs) | Impulse change vs original | Latency change (ms) | Repeatability |
| --- | ---: | --- | ---: | ---: | ---: | --- |
| | 1.125 | Constant slew | 2.984929 | 0 | 0 | Pass |
| | 1.125 | Smooth onset | 2.987981 | +0.102% | +1.866 | Pass |
| | 1.125 | Constant-acceleration onset | 2.989445 | +0.151% | +1.865 | Pass |
| | 1.125 | Full ease | No contact | -100% | Undefined | Fail |
| | 1.5 | Constant slew | 6.754437 | 0 | 0 | Pass |
| | 1.5 | Smooth onset | 6.752506 | -0.029% | +1.400 | Pass |
| | 1.5 | Constant-acceleration onset | 6.750905 | -0.052% | +1.400 | Pass |
| | 1.5 | Full ease | 3.322444 | -50.81% | +2.149 | Pass |

With higher pedestal rate loss the ramped onsets change the first impulse by
less than 0.2% at both speeds and keep within-take repetition. The only
declared first-attack failure is latency, which equals the added ramp time.
This profile's peak pedestal force drops from 28.04 N to 2.74 N at 1.125 m/s,
but the resulting impact is unchanged because its lossy contact already
dissipated the collision in the original driver.

### Acceleration profile matters less than the presence of a ramp

The smooth and constant-acceleration onsets share arrival time and ramp
displacement but differ in peak acceleration by a factor of 1.5 and in jerk
at the ramp ends. Their first strikes agree within the 5% limits in all four
same-arrival rows. The smooth onset delivers 2.9% more onset work in the
original profile, and its hammer overshoots the pedestal speed by 5.4% at the
end of the ramp against 3.0% for constant acceleration, but the impact
differences stay below 2%.

### Terminal smoothing still removes the soft strike

Under any driver with an abrupt stop, the pedestal leaves the hammer exactly
at arrival (41.2 ms for ramped onsets) and the hammer coasts into the tine
1.9 ms later. Full easing keeps the hammer decelerating with the pedestal and
produces no soft contact in either profile, consistent with the previous
terminal study. At 1.5 m/s it strikes with 34.7% and 50.8% less impulse than
the original in the two profiles, yet repeats within limits. The abrupt stop
is what launches the soft strike in this action model.

### Numerical evidence

Maximum analytical tracking error is `3.805e-14 m` and arrival error 0.163
µs. Maximum relative total energy defect is `3.161e-12`, launch momentum
defect `1.530e-11`, launch work defect `2.825e-12`, coupling defect
`1.594e-12`, pedestal port defect `4.497e-13` and pickup exchange defect
`5.314e-19`. Maximum measured ramp acceleration error is 0.001767%.
Maximum velocity refinement RMSE is 0.045171%, impact refinement error
0.042589%, launch impulse-component error 0.011028%, kinetic-work-term error
0.020411%, segment work error 0.22132%, segment impulse error 0.003721% and
segment end-velocity error 0.004518%. The largest event list has twelve
entries.

Verification passes 134 lab unit tests, 79 loaded CLI/receipt tests, strict
Clippy and formatting. Curve tests check displacement derivatives, integrated
travel, endpoint continuity, declared durations, peak accelerations and the
match between the full-ease terminal segment and the retained terminal
curve. Receipt tests verify tracking, boundaries, segment sums, the pedestal
port identity, empty-segment handling, original replay, shared arrival with
the previous terminal ease, first-attack decisions and no-contact handling.

The [receipt](../references/loaded-drive-onset-validation.json) is 3048021
bytes, SHA-256
`8ad965e9c0988a208d475eeb970b53c5d981cc7fe79532dbc214c80e8c054bed`.
The release cache remains approximately 182 MiB, with no new audio renders.

## Scope and next step

This is a prescribed-motion intervention on the pedestal, not a measured key,
finger force or finite-inertia mechanism. Ramps lengthen arrival by
construction, so latency comparisons against the original are expected to
fail. It covers one repetition wait, two nominal speeds and two conditional
profiles. It does not establish recorded-source agreement, arbitrary
gestures, full-keyboard realism or realtime operation.

The evidence points at the drive, not the return regulation, as the origin
of the original soft-strike collision, while the abrupt stop remains the
mechanism that launches the soft strike. Next replace the prescribed pedestal
trajectory with a finite-inertia key/pedestal driven by a prescribed finger
force, so that onset, arrival and release follow from mass, return and
contact laws rather than from a chosen ramp. Requalify first-strike and
repetition evidence against the retained constant-slew and high-pedestal-loss
controls, and keep the driver's supplied work explicit so a contact
adjustment cannot compensate for an unqualified forcing trajectory.
