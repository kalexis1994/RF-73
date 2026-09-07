# Terminal pedestal deceleration

The [launch study](LOADED-LAUNCH.md) showed pedestal departure and re-entry
before the original soft strike, with the final departure close to the abrupt
end of constant-slew travel. This experiment changes the prescribed terminal
trajectory and retains the existing contact laws and work accounting.

```text
cargo run --locked --release -p rf-73-lab -- loaded-drive-release --output REPORT.json
```

The command requires a new JSON output path. A failed numerical qualification
retains its evidence and returns an error. Impact, lift and repetition
outcomes remain separate from numerical qualification.

## Frozen matrix

Twenty-four takes cross two physical profiles, three driver shapes and two
nominal speeds at 128/256 internal ticks per 48 kHz observation frame. The
profiles are the original conditional 70 mm G3 and its pedestal rate-loss
10 s/m control. Return damping remains 0.025 Ns/m. Other geometry, tuning,
structural losses, bridle, arm, pickup and circuit coefficients stay fixed.
Nominal speeds are 1.125 and 1.5 m/s.

The original two-strike schedule is preserved: first key-down at 30 ms,
key-up at 150 ms, second key-down at 210 ms, second key-up at 330 ms, then
70 ms of release observation. The second gesture therefore follows the
first key-up by 60 ms. All state persists across the gestures. Both key-up
motions use the original constant slew; only key-down travel changes.

Let `L` be pedestal travel, `v` the nominal speed and `f = 0.2`.

| Driver | Motion | Arrival after key-down |
| --- | --- | --- |
| Constant slew | Original speed `v` and abrupt stop | `L/v` |
| Terminal ease | Speed `v` through 80% of travel, then decelerate | `1.2*L/v` |
| Matched-duration linear | Constant speed `v/1.2` and abrupt stop | `1.2*L/v` |

For the eased motion, deceleration begins at `tc = (1-f)*L/v` and lasts
`tr = 2*f*L/v`. With `s = (t-tc)/tr`, the terminal trajectory is

```text
position     = (1-f)*L + v*tr*(s - s^3 + 0.5*s^4)
velocity     = v*(1 - 3*s^2 + 2*s^3)
acceleration = (v/tr)*(-6*s + 6*s^2)
```

It reaches exactly the original travel endpoint with zero terminal velocity
and acceleration. Velocity and acceleration are continuous at the cruise/ramp
and ramp/hold boundaries. Maximum terminal deceleration magnitude is
`0.75*v^2/(f*L)`. **Initial onset is still abrupt.** This is not a fully
smoothed gesture or a measured finite-inertia key mechanism.

The eased shape keeps the accumulated constant-slew implementation before
the terminal region, preserving the original first-launch prefix exactly.
Its longer duration is an explicit consequence of retaining both travel
and maximum speed. The matched-duration control supplies a comparison at
the same arrival time, but its lower initial speed also changes the launch;
it does not isolate all kinematic factors independently.

## Qualification and comparison

The actual pedestal trajectory is compared against the analytical position
through each held gesture. Maximum tracking error must be below `1e-8 m`;
sample-interval key-down speed must remain nonnegative and at most nominal
speed, allowing `1e-8` relative tolerance for floating-point differences.
Both measured arrival times must be within 10 microseconds of their predicted
times. Linear drivers report no finite terminal acceleration value because
their stop is discontinuous.

All repetition and launch energy, momentum, heat, exchange, force-impulse,
impact, velocity and per-port event refinement checks remain active. Each
constant-slew control can be replayed against the prior launch receipt.
Functional repetition compares second versus first within each take using
the existing 5% impact/speed and 1 ms latency limits, plus contact separation
and held felt lift. A separate comparison uses the same-profile original
first strike as its reference, keeping first-attack changes explicit.

Missing strikes are retained; they are not treated as a zero-error fit or
successful repetition. Where a launch has no first contact, the retained
peak velocity covers its 20 ms observation window and does not imply that
an impact occurred. No configuration is automatically selected.

## Retained results

All 24 takes and twelve refinement pairs qualify numerically, including six
takes with no hammer/tine strike. The other eighteen takes retain exactly
one contact during each held gesture and none during recovery or release.
Every take preserves both felt-lift windows. Four paired rows satisfy clean
repeatability, all in the higher-pedestal-loss profile. No modified driver
preserves its same-profile original first attack within all declared limits.

The following first-strike impulses use 256 ticks. Nominal speed labels are
kept across drivers; the matched-duration control actually travels at 0.9375
or 1.25 m/s respectively.

| Profile | Driver | First impulse at nominal 1.125 m/s (mNs) | First impulse at nominal 1.5 m/s (mNs) | Repeatability |
| --- | --- | ---: | ---: | --- |
| Baseline | Constant slew | 0.237325 | 6.781728 | Neither speed |
| Baseline | Terminal ease | No contact | 5.736096 | Neither speed |
| Baseline | Matched-duration linear | 0.559276 | 3.218439 | Neither speed |
| Pedestal loss 10 | Constant slew | 2.984929 | 6.754437 | Both speeds |
| Pedestal loss 10 | Terminal ease | No contact | 3.314431 | Strong only |
| Pedestal loss 10 | Matched-duration linear | No contact | 4.431467 | Strong only |

Terminal easing removes both soft strikes in both profiles. These outcomes
are retained as missing impacts with null repeatability ratios, not as two
matching successful strikes. In the original profile the matched-duration
linear control still strikes at that nominal speed, so the extra arrival
time alone does not describe the change. Its actual initial speed also
differs; this is evidence of trajectory sensitivity, not a one-factor causal
decomposition of the entire motion.

For the strong higher-pedestal-loss row, easing preserves within-take
repeatability (second/first impulse difference 0.3762%) while reducing the
first impulse from 6.754437 to 3.314431 mNs. The matched-duration control
also repeats (difference 0.2987%) but gives 4.431467 mNs on its first strike.
Good repeated-note consistency therefore remains compatible with a large
change in first attack. Neither modified trajectory is adopted as a default.

Original-driver takes reproduce the preceding launch receipt exactly after
removing the new driver measurements. The eased first-launch force-event
prefix before terminal deceleration also replays the original exactly.
The disappearance of the soft strike occurs despite an unchanged early
launch history, making the late prescribed motion a material part of the
conditional threshold behavior.

At nominal 1.125 m/s, predicted travel duration changes from 9.333333 to
11.2 ms, with 3.733333 ms of eased deceleration and peak terminal
deceleration 452.009 m/s². At 1.5 m/s it changes from 7 to 8.4 ms, with
2.8 ms of deceleration and a peak of 803.571 m/s². These are derived
kinematic values, not measurements of a player's key action.

Maximum position tracking error is `3.805e-14 m`, and maximum arrival-time
error is 0.163 microseconds. Maximum relative total energy defect is
`2.507e-12`, launch momentum defect `3.254e-13` and launch work defect
`1.699e-12`. Maximum velocity refinement RMSE is 0.040978%, impact
refinement error 0.042590%, launch impulse-component error 0.010942% and
kinetic-work-term error 0.017553%.

Verification passes 129 lab unit tests, 25 loaded CLI/receipt tests, strict
Clippy and formatting. Curve tests independently check displacement
derivatives, integrated velocity, endpoint continuity and speed bounds.
Receipt tests verify measured travel, original replay, the unchanged eased
prefix, first-attack decisions and explicit no-contact handling.

The [receipt](../references/loaded-drive-release-validation.json) is 1914026
bytes, SHA-256
`69721438a97065e8f1947a21b55ee08ad241e21e94e15b30069a6c2fa72b68d1`.
The release cache remains approximately 195 MiB, with no new audio renders.

## Scope and next step

This is a prescribed-motion intervention with the driver's supplied work
retained. It does not add a new contact law, change escapement geometry or
model finite key inertia. It covers only one repetition wait, two nominal
speeds and two conditional profiles. It does not establish recorded-source
agreement, arbitrary gestures, full-keyboard realism or realtime operation.

The new trajectory is retained as a diagnostic control. Terminal smoothing
alone does not resolve soft-strike behavior and makes the previously successful
soft repetition disappear. Next qualify onset acceleration and the complete
key/hammer drive, then use that evidence to develop the key/action coupling
and launch regulation. Keep driver work and first-strike/repetition checks
explicit so a contact adjustment cannot silently compensate for a particular
unqualified forcing trajectory.
