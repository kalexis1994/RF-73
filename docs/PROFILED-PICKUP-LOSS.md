# Unknown loss scales from mechanical pickup history

The laboratory now recovers separate structural and damper loss scales without
supplying their true values to the inverse. All six matched noiseless cases
pass. The input is still synthetic mechanical pickup velocity with known
geometry and operator shapes; this is not calibration from magnetic voltage or
the processed reference bank.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-loss --output renders/profiled-pickup-loss.json
```

The [receipt](../references/profiled-pickup-loss-validation.json) contains six
trajectories and 18 fit variants, experiment `profiled-pickup-loss-v1`, schema 1,
in 318045 bytes. SHA-256:
`830c2d80380c7e1e539aa2549f0122faeb443d2ea6b37044c0674895b1ab73c1`.
It retains both search histories, fitted states, held-out errors, sensitivity,
biased results and boundary flags. Existing output files are protected. Failed
required controls are written before the command returns an error.

## Protocol fixed before the first run

The [known-damping observer](DYNAMIC-PICKUP-STATE.md) supplies the propagator and
nuisance-state fit. The nine-coordinate physical assembly, untuned 75 mm tine,
elastic hammer, strike and binary damper event remain those of the
[mechanical-loss experiment](MECHANICAL-LOSS.md). Actual scale pairs are now
`(0.63,1.37)`, `(1.13,0.57)` and `(1.47,0.91)`, each at 48/96 kHz. These values
are deliberately absent from the coarse search grid. Every variant shares its
physical trajectory with the others at that rate and scale pair.

Only scalar pickup velocity enters the inverse. All mechanical ticks within
each window must be free of hammer contact, including ticks between samples.

| Estimate | Training samples | Subsequent unused prediction |
|---|---|---|
| Structural scale, damper off | [0.02,0.06) s | [0.06,0.10) s |
| Damper scale, damper on | [0.14,0.18) s | [0.18,0.22) s |

For each candidate `alpha` or `beta`, the observer uses
`C=alpha*C0 + damper_on*beta*D`. All projected damping couplings remain present.
It refits the 18 initial-state coordinates using training samples and the
existing normalized, reorthogonalized QR solver. The objective is the sum of
squared training residuals divided by the observed training signal's squared
norm. A silent or rank-deficient history cannot produce a successful fit.

The structural scale is estimated from the off window first. It is then frozen
while estimating damper loss from the on window. The two windows have independent
initial-state fits. This is sequential conditional estimation, not a joint
maximum-likelihood fit; uncertainty in the first estimate can affect the second.

Each scalar search evaluates 17 fixed nodes from 0.25 to 2.0, then performs 32
golden-section refinements inside the best coarse bracket. All 51 evaluations
are retained, including endpoints. Parameters within `1e-5` of a search bound
are flagged. A finite coarse scan does not establish a unique global optimum.
No true scale, previous-case solution, clean signal or held-out sample enters
the recovery function. Ground truth only generates observations and scores fits.

The three variants are:

- Matched operators, noiseless training; these are the required controls.
- Matched operators, deterministic training noise at 1% of clean training RMS.
- Noiseless training, assumed damper contact position 0.7 instead of the true
  0.8 of tine length. Only the damper operator is incorrect.

Noise uses the observer study's centered, unit-RMS LCG sequence. It is one
perturbation per training length, not a Monte Carlo uncertainty estimate. The
held-out synthetic signal stays clean for evaluation.

## Sensitivity and acceptance

The local sensitivity diagnostic perturbs each fitted scale by +/-1% and refits
the nuisance states at every point. The resulting residual derivatives from
both training windows form a two-column matrix. Each window is normalized by
its observed signal norm and the pair by `sqrt(2)`. Singular values describe
local sensitivity to approximately logarithmic parameter changes. This includes
cross-sensitivity to structural loss in the damper-on window, but does not
reoptimize the two scales jointly. Bound-adjacent diagnostics can evaluate
perturbations just outside the search interval.

Prediction consistency requires both clean held-out pickup relative RMSEs below
0.005, interior parameters, minimum sensitivity singular value above `1e-8`,
and minimum/maximum singular ratio above `1e-4`. It is distinct from the scored
known-scale recovery criterion of less than 1% error in both parameters.
Required noiseless controls use stricter limits: parameter errors below 0.1%,
both held-out pickup errors below `1e-5`, and the original mechanical checks.
No gates were adjusted after observing results.

Sensitivity is not a confidence interval or proof of global identifiability.
In particular, a model with an incorrect contact position can still be locally
sensitive to its own loss parameters.

## Results

All 18 fits complete. Maxima below cover the six rate/scale cases per variant;
errors are dimensionless fractions, not percentages.

| Variant | Maximum structural relative error | Maximum damper relative error | Maximum held-out pickup relative RMSE | Prediction consistency | Both scales within 1% |
|---|---:|---:|---:|---:|---:|
| Matched, noiseless | 4.686e-9 | 4.043e-9 | 6.024e-9 | 6/6 | 6/6 |
| Matched, 1% training noise | 0.002415 | 0.000529 | 0.009475 | 3/6 | 6/6 |
| Wrong damper position | 4.686e-9 | 0.516374 | 0.189363 | 0/6 | 0/6 |

The six noiseless required controls pass with off-grid values. The same search
resolution selects identical values at both sample rates in these controls;
their remaining scale error is mainly search resolution, not evidence of exact
real-instrument parameter knowledge.

With 1% training noise, parameter errors remain below 0.242% for structure and
0.0529% for the damper. Three cases nevertheless exceed the 0.5% held-out signal
criterion. Accurate global multipliers and accurate future pickup prediction
are separate properties when the fitted state is noisy.

Moving the assumed damper contact from 0.8 to 0.7 biases its fitted loss by
45.99..51.64%. For actual damper scale 1.37, the search reaches its upper bound
of 2.0. The other four mismatched fits have interior parameters but still fail
held-out prediction, with pickup errors around 1.14% or 1.93%. All wrong-position
fits remain locally resolved; sensitivity alone would not expose that bias.
These rejected controls do not prove that every smaller geometry error will
be detectable.

## Verification and next step

The laboratory release suite passes 113 tests (82 unit, 31 CLI). New unit tests
cover off-grid search, retained endpoint minima, invalid objectives and the
absence of damper information in a damper-off history. CLI checks retain all
variants, both 51-point search histories, six required controls and output
protection. Strict laboratory Clippy and formatting checks also pass.

Next measure how smaller errors in contact position and pickup observation
affect parameter bias and the acceptance criteria. The large displacement used
here is an initial negative control, not a tolerance guarantee. Then qualify
the magnetic observation path before applying this inverse to source recordings.
Mass, stiffness, damping shapes and event timing remain supplied assumptions;
only two global viscous multipliers are estimated.

This adds an offline identification study. No physical equations, production
presets, plugin engine or audio assets changed, and no host was launched.
