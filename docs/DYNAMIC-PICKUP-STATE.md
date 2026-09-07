# Mechanical state from pickup history

The full nine-mode observer reconstructs motion from a single mechanical pickup
velocity history in all twelve noiseless controls. This requires known geometry,
observation gain **and damping**. It demonstrates initial-state recovery under
those assumptions, not identification of unknown losses from recorded audio.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-state --output renders/dynamic-pickup-state.json
```

The [receipt](../references/dynamic-pickup-state-validation.json) contains 108
observations from six physical trajectories, experiment `dynamic-pickup-state-v1`,
schema 1, in 205598 bytes. SHA-256:
`a3be60601a0acda19db55324969e41cf3eb7a87f37b8702418ba6dd26c8ebb3a`.
Outputs must be new JSON files; failed fits are retained. Failed required controls
make the command return an error after writing the report. No WAVs are generated.

## Fixed protocol

The [mechanical loss study](MECHANICAL-LOSS.md) supplies the same three loss-scale
pairs at 48/96 kHz, default untuned 75 mm geometry, elastic single-mass hammer,
strike and 0.14 s damper event. Every variant uses the same trajectory at its
rate and scales. All mechanical ticks in each observation interval must be free
of hammer contact, including ticks between observation samples.

Only scalar pickup velocity is observed, at output sample rate. Neither pickup
displacement nor a numerical derivative enters the inverse. There are two
independent batch fits:

| Damper | Fit initial state | Predict without refitting |
|---|---|---|
| Off | [0.02, 0.06) s | [0.06, 0.10) s |
| On | [0.14, 0.18) s | [0.18, 0.22) s |

Each fit and prediction has 1920 samples at 48 kHz or 3840 at 96 kHz. The second
fit starts at the known damper event; this experiment does not infer its timing
or predict the transition using the first fit's state.

For mass-normalized undamped modes `Phi`, coordinates are `x=[Omega*z, zdot]`,
where `q=Phi*z`. The generator is

```text
G = [ 0        Omega       ]
    [ -Omega  -Phi^T C Phi ]
y = [ 0       pickup_weights^T ] x
```

All off-diagonal damping couplings are retained. Lowest 3/6/9-mode models use
the corresponding projected operators. Discarded modes are present in the
physical observations, so truncated models have real model mismatch rather
than observations artificially cleaned of omitted motion.

An independent laboratory matrix exponential propagates the model. Scalar
observation templates `H exp(G*t)` supply the initial-state least-squares fit,
using normalized columns and two-pass reorthogonalized QR. Pivots below `1e-8`
reject a fit; the reported minimum pivot is not a condition number. The inverse
accepts only templates and training samples. Mechanical state is reserved for
evaluation, and held-out samples never select or update the inferred state.

Training noise levels are 0, 0.001 and 0.01 times the clean training signal RMS.
A fixed uniform pseudorandom LCG sequence is centered and normalized to unit
sample RMS, shared between mode counts and levels at each sample count.
Noise is added only to training; prediction is evaluated against the clean
held-out synthetic signal. This is one deterministic perturbation per length,
not a Monte Carlo confidence interval, colored-noise model or sensor noise floor.

Only noiseless nine-mode controls have pass requirements: held-out pickup
relative RMSE below `1e-6`, full-state energy-norm relative RMSE below `1e-5`, and
the original mechanical ledger checks. Counts, windows, noise and thresholds
were fixed before the first run. All reduced/noisy results remain descriptive.

## Results

All 12 required controls pass; all 108 inverse fits return finite results. The
table gives maxima across the twelve rate/scale/damper observations per row.
Relative errors are dimensionless fractions, not percentages.

| Modes | Training noise / clean RMS | Held-out pickup relative RMSE | Held-out full-state energy-norm relative RMSE |
|---:|---:|---:|---:|
| 3 | 0 | 0.448780 | 0.196012 |
| 3 | 0.001 | 0.448781 | 0.195925 |
| 3 | 0.01 | 0.448787 | 0.195170 |
| 6 | 0 | 0.001141 | 0.002043 |
| 6 | 0.001 | 0.001382 | 0.002575 |
| 6 | 0.01 | 0.010041 | 0.009173 |
| 9 | 0 | 4.158e-11 | 4.070e-11 |
| 9 | 0.001 | 0.001019 | 0.000941 |
| 9 | 0.01 | 0.010188 | 0.009401 |

The state error is `sqrt(sum ||x_est-x_true||^2 / sum ||x_true||^2)` over held-out
samples. It weights displacement by modal frequency and includes velocity,
equivalent to an energy norm of the error. It is not the error of the scalar
total energy. The receipt also reports retained-state and per-mode errors;
omitted modes have a zero estimate and therefore relative error 1.

The noiseless three-mode estimate has up to 9.78% error even within its retained
state. It cannot be substituted for the three exact oracle coordinates that
passed the [reduced loss experiment](REDUCED-MECHANICAL-LOSS.md). Six modes are
much closer in this particular geometry, with measurable truncation error.

Small aggregate error does not ensure accurate recovery of every mode. With
nine modes and 1% training noise, full-state error stays below 0.941%, but the
second structural mode (about 80.69 Hz, zero-based index 1) reaches 43.71%
relative state error in one held-out damper-on case. Its pickup weight is much
smaller than those of several other modes. Normalizing QR columns does not
remove that physical sensitivity. A favorable aggregate fit must not authorize
independent modal-loss calibration without sensitivity analysis.

## Verification and next step

Unit tests compare the exponential against an analytic damped oscillator and
time refinement, reconstruct initially invisible displacement from analytic
velocity observations, and reject missing rank and malformed/non-finite inputs.
They also check deterministic zero-mean/unit-RMS noise. CLI coverage checks all
108 variants, twelve controls, held-out metrics, and output preservation.
The laboratory release suite has 110 passing tests (80 unit and 30 CLI).

Next remove the supplied-loss assumption: profile candidate structural/damper
scales while fitting nuisance initial states using training samples only, and
retain held-out predictions and parameter sensitivity. Include wrong-operator
and noise controls before interpreting fitted losses. Magnetic conversion,
unknown capture gain and processed source recordings remain separate stages.

No physical solver, production preset or plugin engine changed. This work adds
an offline inverse-model validation tool; it does not claim a new audible version
or a host/listening test.

The subsequent [profiled pickup-loss study](PROFILED-PICKUP-LOSS.md) removes the
supplied-scale assumption while retaining known operator shapes. Six noiseless
off-grid controls recover both global losses; noise and incorrect damper position
remain explicit controls. It does not yet identify losses from magnetic voltage.
