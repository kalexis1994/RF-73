# Loaded hammer return and pedestal contacts

The preceding [bridle study](LOADED-BRIDLE.md) restored felt contact but left
the hammer moving outside the declared return limits at 350–400 ms. This
experiment extends the same key gesture through 800 ms and distinguishes
free hammer motion, pedestal contacts and the effect of two loss controls.

```text
cargo run --locked --release -p rf-73-lab -- loaded-hammer-return --output REPORT.json
```

The output must be a new JSON path. Numerical failure retains a report and
returns an error; functional failures remain explicit in a numerically
qualified report. There is no automatic parameter adoption.

## Frozen protocol

Twelve takes cover three configurations and two pedestal speeds, each at
128 and 256 internal ticks per 48 kHz observation frame. The conditional
70 mm G3 profile, fitted tuning-spring position, 30 s first-basis tine decay,
pickup and circuit remain as in the preceding study. The key starts down at
30 ms, returns at 150 ms with the same bounded slew and remains at rest
through 800 ms. Pedal position stays closed. Speeds are 1.125 and 1.5 m/s.

| Case | Return damping (Ns/m) | Pedestal rate loss (s/m) |
| --- | ---: | ---: |
| Baseline | 0.025 | 2 |
| Return damping control | 0.1 | 2 |
| Pedestal loss control | 0.025 | 10 |

The coefficients remain constant throughout each take, including attack.
Hammer mass is 0.004 kg and return stiffness is 4 N/m. These are conditional
model values, not measurements of an instrument's action.

## Independent free-motion check

With hammer/tine, pedestal/hammer and bridle forces all zero, hammer offset
`x = q_hammer - hammer_rest` obeys `m*x'' + c*x' + k*x = 0`. Each of the three
configurations is underdamped. The diagnostic evaluates the continuous
closed-form solution from the start of every free interval lasting at least
1 ms after key release. It compares the terminal position and velocity to
the simulated state using the norm `sqrt((k/m)*x*x + v*v)`, normalized to
the initial state with a `1e-8 m/s` norm floor. Relative error must be below
`1e-6`. The receipt retains predicted and actual states, interval times,
initial/final energy and accumulated return heat. Tests independently
reconstruct the energy norm and check that free energy loss equals heat.

This check spans whole free intervals, so agreement cannot be obtained
merely by restarting an exact solution on every numerical tick. It checks
the existing spring/damper equation; it does not validate that equation
against a real action.

Every pedestal force entry/exit after release retains the states on either
side, all four contact forces/compressions, hammer energy and cumulative
pedestal work/contact heat/return heat. Event and free-interval lists are
bounded at 256 entries each; overflow fails qualification. Event sequences
must agree between resolutions and event times differ by less than 0.1 ms.
The receipt also records the last observed hammer tolerance violation and
the duration of the terminal settled interval. This is a finite observation,
not a prediction that the hammer can never leave those limits again.

## Separate numerical and functional decisions

The shared runner preserves independent hammer, pedestal, contact, bridle,
arm and felt energy identities, monotone heat, quiet initial rest and
reciprocal pickup exchange checks. Four velocity channels must converge
within 1% independently in 30–150, 150–220, 220–400 and 400–800 ms windows.
Impulse, peak force and contact duration must refine within 1%, with exact
paired zeros for a non-striking row. Baseline snapshots at 120, 150, 220 and
400 ms and the pre-impact state replay the earlier bridle receipt.

Functional return uses the last 50 ms: hammer and arm positions within
0.1 mm of prepared rest, speeds below 0.01 m/s, and felt contact for at
least 90% of that window. Held felt clearance remains at least 0.1 mm with
no contact during 80–140 ms. Attack preservation requires exactly one
hammer strike, less than 5% change in impulse, peak, duration and pre-impact
speed relative to the same-speed baseline, and less than 1 ms shift in
first-contact time. All limits are fixed engineering diagnostics, not
calibrated regulation tolerances or claims of perceptual equivalence.

## Retained results

All twelve takes and all six refinement pairs qualify numerically. Every take
has one hammer/tine strike and preserves held felt lift and final felt
contact. None of the six configuration/speed rows satisfies the combined
attack-preservation, lift and return criteria.

The following measurements use 256 ticks per observation frame. Position
and velocity columns are maxima over the entire 750–800 ms window.

| Case | Drive (m/s) | Hammer offset (mm) | Hammer speed (m/s) | Return | Attack preserved |
| --- | ---: | ---: | ---: | --- | --- |
| Baseline | 1.125 | 0.373428 | 0.0146893 | Fail | Yes (self-control) |
| Baseline | 1.5 | 0.371371 | 0.0146274 | Fail | Yes (self-control) |
| Return damping 0.1 | 1.125 | 0.002777 | 0.0000900 | Pass | No |
| Return damping 0.1 | 1.5 | 0.002766 | 0.0000881 | Pass | No |
| Pedestal rate loss 10 | 1.125 | 0.149473 | 0.0060726 | Fail | No |
| Pedestal rate loss 10 | 1.5 | 0.136499 | 0.0060948 | Fail | Yes |

In the return-damping control the last observed hammer tolerance violation
occurs around 417.4 ms, followed by about 382.5 ms within limits. Its final
hammer energy falls to approximately `3e-12 J`, versus `3.1e-7 J` in the
baseline. But its soft-drive pre-impact speed increases from 0.106164 to
0.525539 m/s and impulse from 0.237325 to 2.933865 mNs (12.362 times the
baseline). At the higher drive its impulse instead decreases by 18.58%.
Constant damping therefore cannot be treated as a return-only adjustment.
The prescribed drive also supplies different work: at the soft setting,
12.294 mJ in the baseline versus 14.011 mJ in the damping control. The
independent work balances retain that change; increased impact is not an
energy-creation claim.

Higher pedestal rate loss likewise changes the soft impulse to 12.577 times
baseline, while changing high-drive impulse by only 0.4024%. It lowers late
return speed below its limit but leaves position outside the 0.1 mm limit.
Its soft row spends the last 40.77 ms within hammer limits; that still fails
the required 50 ms window. A single endpoint would hide this failure.

The baseline has eight post-release pedestal entries per take, versus seven
in either intervention. Across all takes, 112 free intervals lasting at least
1 ms are checked. Maximum relative free-state error is `5.378e-11`; the
recorded energy loss in each interval equals return heat within `1e-12 J`.
This strongly supports the numerical integration of the declared free-motion
equation over these trajectories. It does not establish a physically correct
return law or contact regulation for an actual instrument.

Maximum relative total energy defect is `4.001e-12`, independent hammer/port
defect `1.207e-12`, bridle/arm/felt defect `4.175e-12` and exchange defect
`4.056e-19`. Maximum velocity refinement RMSE is 0.008404%, impact refinement
error 0.042590%, and pedestal event-time difference 0.163 microseconds.
All baseline pre-impact states and four historical snapshots exactly replay
the preceding bridle receipt. Functional decisions agree at both resolutions.

Verification passes 124 lab unit tests, nineteen loaded CLI/receipt tests,
strict Clippy and formatting. The
[receipt](../references/loaded-hammer-return-validation.json) is 895795 bytes,
SHA-256 `52579de65a5d83a57de3ade283ce9a1eaeffbbc2b88421f4220d7f139ca1952a`.
The release cache remains approximately 194 MiB.

## Scope and next step

This experiment diagnoses the existing offline action. It introduces no
time-dependent damping switch, additional backcheck, new audio render,
recorded-source fit or production default change. Two speeds cannot establish
velocity monotonicity. No intervention is adopted: one solves settling while
changing attack, and the other leaves a settling failure. Repetition must
be qualified from the actual returned state, including the baseline and loss
controls with their distinct first strikes. The
[two-strike study](LOADED-REPETITION.md) now measures those trajectories at
two recovery intervals. The sophisticated model
also still needs computational reduction for the playable plugin.
