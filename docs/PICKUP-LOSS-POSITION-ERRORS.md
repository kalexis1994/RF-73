# Small position errors in pickup loss inference

Of 102 noiseless fits, 100 meet the existing prediction criterion, but 46 of
those estimate the damper loss with more than 1% relative error. The earlier
large-position negative control did not establish that smaller geometry errors
would be detected. A small signal residual cannot certify physical loss values.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-loss-geometry --output renders/pickup-loss-position-errors.json
```

The [receipt](../references/pickup-loss-position-errors-validation.json) contains
all 102 fits, experiment `pickup-loss-position-errors-v1`, schema 1, 1014475 bytes.
SHA-256: `6dce481f5674cc122dc0ba5c170d7cb7e4796c83f0b10f6b43cde1fd8e42a690`.
The command protects existing files and retains failed controls before returning
an error. No waveform is produced.

## Fixed experiment

The six trajectories, true scale pairs, nine-mode inverse, sequential loss
searches, state fitting, windows and acceptance criteria are unchanged from
[profiled pickup loss](PROFILED-PICKUP-LOSS.md). Ground-truth scales are
`(0.63,1.37)`, `(1.13,0.57)`, `(1.47,0.91)`, each at 48/96 kHz. Each physical
trajectory supplies every assumed-operator variant at its rate and scales.

Positions are measured **along the tine from its root**, normalized by the
default 75 mm length. Actual damper contact is at 0.8; the mechanical pickup
observation point is at 0.98. These are not magnetic gap, lateral alignment or
the tuning spring's position. The 17 variants per trajectory were fixed before
the first run:

- One matched operator control.
- Damper-only errors of +/-0.001, +/-0.005 and +/-0.01 of tine length.
- Pickup-only errors at the same six offsets.
- Four combinations of damper and pickup errors of +/-0.005.

The absolute offsets correspond to 0.075, 0.375 and 0.75 mm. Only the assumed
operators change; the observed trajectory stays fixed. Pickup offsets alter
the observation weights. Damper offsets alter its spatial damping matrix.
Every variant checks unchanged mass, stiffness, structural damping, undamped
frequencies and mode shapes, so state errors use a shared mechanical basis.

Training is noiseless to isolate operator bias. The fit uses [0.02,0.06) s
with the damper off, then [0.14,0.18) s with the damper on. The latter uses the
structural scale found by the former. Subsequent [0.06,0.10) and [0.18,0.22)
intervals remain unused until prediction. Each window has its own 18-coordinate
initial-state fit. True scales and held-out samples never enter the inverse.

Each scalar search still performs 51 evaluations. To bound receipt size, this
study retains the 17 coarse nodes, evaluation count, selected parameter and
objective, final bracket and boundary flag, omitting 34 local-refinement rows.
All variants retain their fitted states, per-mode errors and sensitivity metrics.

## Fixed criteria and controls

Prediction consistency retains the prior requirements: both held-out pickup
relative RMSEs below 0.005, interior scales, minimum local sensitivity singular
value above `1e-8` and singular ratio above `1e-4`. Independent scoring compares
each recovered scale with ground truth using the existing 1% criterion.
`prediction_consistent_but_biased` explicitly marks fits that pass prediction
while failing that ground-truth parameter check. This label does not enter
search, selection or acceptance thresholds.

Only the six matched controls must recover scales within 0.1% and predict both
held-out windows below `1e-5`, alongside the original mechanical checks.
All 102 operator invariants must pass. Perturbed results remain descriptive;
no parameter gates or tolerances were adjusted to improve this grid's outcome.

## Results

All six matched controls and 102 operator-invariant checks pass. All fits
complete. The matched parameters and all window metrics are identical to the
preceding receipt. In total, 54 fits recover both scales within 1%, 100 pass
prediction consistency, and 46 pass prediction while retaining biased losses.

Maxima below cover both offset signs, all three scale pairs and both rates.
Relative errors are dimensionless fractions, not percentages.

| Error family | Absolute offset (mm) | Fits | Prediction consistent | Consistent but biased | Maximum damper scale error | Maximum held-out pickup error |
|---|---:|---:|---:|---:|---:|---:|
| Matched | 0 | 6 | 6 | 0 | 4.043e-9 | 6.024e-9 |
| Damper only | 0.075 | 12 | 12 | 0 | 0.003814 | 0.000664 |
| Damper only | 0.375 | 12 | 12 | 12 | 0.019280 | 0.004541 |
| Damper only | 0.750 | 12 | 10 | 10 | 0.039105 | 0.012027 |
| Pickup only | 0.075 | 12 | 12 | 0 | 4.043e-9 | 6.024e-9 |
| Pickup only | 0.375 | 12 | 12 | 0 | 4.043e-9 | 6.024e-9 |
| Pickup only | 0.750 | 12 | 12 | 0 | 4.043e-9 | 6.024e-9 |
| Combined | 0.375 each | 24 | 24 | 24 | 0.019280 | 0.004541 |

Structural loss relative error remains below `4.686e-9` across the grid. Damper
position affects only the on operator, leaving the off-window structural fit
unchanged. In the worst accepted parameter case, true scales `(1.13,0.57)` at
48 kHz and assumed damper offset -0.75 mm produce 3.91% damper-scale error with
only 0.0953% held-out pickup error. Tight-looking predictions can therefore
conceal material loss bias even without noise.

Pickup-only errors leave the fitted scales unchanged at this search resolution,
yet the full-state energy-norm error reaches 0.1511%, 0.7616% and 1.5396% for
offsets of 0.075, 0.375 and 0.75 mm respectively. Correct losses and a nearly
exact predicted signal do not imply correct motion or pickup placement.

This behavior is consistent with the linear observation model: changing an
observable scalar output port leaves the mechanical evolution's poles unchanged,
while free initial states can change each component's amplitude and phase. The
two windows are allowed independent states, so this experiment imposes no
cross-event mechanical-state constraint that could resolve that ambiguity.
It does not show that pickup placement is physically irrelevant.

These grid results are not manufacturing tolerances or calibration confidence
intervals. They cover one untuned geometry, prescribed events, noiseless linear
observations and finite position offsets. The earlier 7.5 mm damper-position
error was rejected in all six cases; that result cannot generalize to these
smaller offsets or to unsampled values.

## Verification and next step

The laboratory release suite passes 116 tests (84 unit, 32 CLI), plus strict
laboratory Clippy and formatting checks. New tests isolate intended operator
changes, distinguish prediction from physical correctness, retain all variants
and compact profiles, and protect existing output files.

Next test whether enforcing one continuous mechanical state through the known
damper event reduces the observation-position ambiguity. The present inverse
refits states independently before and after that event. Keep geometry treated
as uncertain rather than tightening a signal threshold to hide these failures.
Then qualify the existing [magnetic transfer](PICKUP-TRANSFER.md) on the same
mechanical trajectories before attempting recorded-voltage loss inference.

No production equations, loss presets, plugin engine or audio assets changed.
The command runs offline and does not launch the host.
