# Coupled contact and reference resolution

The [coupled RK4 experiment](MEMORY-MODAL-RK4.md) passed its initial gates but
differed from the uniform implicit reference by up to 0.2612% in the kinetic
velocity metric. This study separates interval sensitivity in the candidate
from timestep sensitivity in that reference. It changes laboratory scheduling
only: the DSP equations, coefficients, tolerances and audible plugin are unchanged.

## Reproduction and protocol

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-rk4-refinement --output renders/modal-refinement.json
```

The command requires a new JSON output path. It audits all twelve original
8 ms profiles: tine lengths 50/75/120 mm, launch speeds 0.2/0.8 m/s and material
relaxation times 1/10 ms. Four strong-strike 75/120 mm profiles continue to
32 ms. These include the original worst velocity-difference case and test both
relaxation times. They are a selected extension, not full parameter coverage.
Every path starts with zero-gap impact, receives a core impulse at 2 ms, and
switches the damper on at 4 ms and off at 6 ms. No new events occur afterward.
Observations and force averages remain at 48 kHz.

Each case has six paths, making 96 independently audited takes:

| Report path | Integration / interval constraint |
| --- | --- |
| `rk4_default` | Existing controller, contact/free levels through 12 |
| `rk4_contact_cap_2` | Contact at most four base ticks; free levels through 12 |
| `rk4_contact_2_free_8` | Contact at most four ticks; free at most 256 ticks |
| `uniform_8336` | Original implicit method, 8336 ticks per observation |
| `uniform_16672` | Previous implicit reference, 16672 ticks per observation |
| `uniform_20832` | Finer implicit reference, 20832 ticks per observation |

All RK4 paths share the original 16672-tick base grid. Diagnostic caps cannot
enlarge an interval or bypass error, energy, clearance or event checks. Rejected
minimum trials retain the original implicit fallback. In the 8 ms matrix,
maximum accepted contact intervals decrease from about 160 ns to 5 ns; free
intervals decrease from about 1.280 microseconds to 0.320 microseconds with the
additional free cap. These are observed maxima, not timing results.

The uniform steps are approximately 2.4992, 1.2496 and 1.000064 ns. The assembly
API's 1 ns minimum prevents halving the old reference step. These adjacent
refinements are unequal; the report does not estimate convergence order or
extrapolate an exact solution. They also share the original implicit algorithm.

## Metrics and gates

For corresponding observations, the full kinetic metric uses both hammer
masses and the complete structural mass matrix, including cross terms:

```text
d_i = mc * delta_v_core^2 + mt * delta_v_tip^2 + delta_v^T * M * delta_v
velocity_rmse = sqrt(sum(d_i) / (observation_count * (mc + mt) * launch_speed^2))
peak_velocity_error = sqrt(max(d_i) / ((mc + mt) * launch_speed^2))
```

Here `mc = 0.0038 kg` and `mt = 0.0002 kg`, matching every take's provisional
profile. Pickup-velocity and observation-averaged force RMSE use the second
path's squared signal sum as denominator. Zero reference power yields a null
relative metric and passes only for exactly zero difference. The tool rejects
incompatible trajectories and non-finite metrics.

Seven pair comparisons isolate contact refinement, additional free refinement,
both caps together, the old candidate/reference comparison, both adjacent
uniform refinements, and the capped candidate against the finest reference.
Every take retains the existing energy and each port-work gate of 1e-8,
positive energy-step gate of 1e-10, nonnegative force/heat checks, free recovery,
reimpact and activation of all nine coordinates. Every pair requires velocity
and pickup RMSE below 1%, and force RMSE below 2%.

Peak velocity differences and nonoverlapping 2 ms kinetic windows are retained
as diagnostics. There is no new peak/window acceptance gate. Whole-record RMSE
can decrease when a long quiet tail is added, so it cannot replace local checks.
The JSON is retained on a completed gate failure, and the CLI exits with an error.

## Retained results

The [96-take report](../references/memory-modal-rk4-refinement.json) passes all
16 cases and 112 pair comparisons. Maximum whole-record kinetic velocity RMSE,
expressed as a percentage of launch speed, is:

| Pair | Twelve 8 ms cases | Four 32 ms cases |
| --- | ---: | ---: |
| Default / contact cap | 0.0001941% | 0.0001207% |
| Contact cap / both caps | 0.0002981% | 0.0001853% |
| Default / both caps | 0.0001040% | 0.00006461% |
| Default / old uniform reference | 0.2612% | 0.1624% |
| Uniform 8336 / 16672 | 0.7693% | 0.4772% |
| Uniform 16672 / 20832 | 0.09511% | 0.05912% |
| Both caps / finest uniform reference | 0.1661% | 0.1032% |

The separate cap effects need not add in magnitude: trajectory differences can
partly cancel. Close agreement between capped candidates alone does not prove
accuracy. However, the much larger reference sensitivity and smaller difference
against its finer version support the inference that the old reference accounts
for a substantial part of the original 0.2612% discrepancy. This study does not
establish an exact continuous solution or eliminate shared boundary-step error.

Across both durations, the capped candidate versus finest reference has maximum
pickup-velocity RMSE 0.02059%, mean-force RMSE 0.03018%, peak kinetic velocity
difference 0.4225% and 2 ms velocity RMSE 0.2612%. The largest observed peak for
any pair is 1.9691%, and the largest 2 ms RMSE is 1.2088%, both between the two
coarser uniform paths. These exceed 1% locally while the existing whole-record
gate passes; they are explicitly not claimed to pass a pointwise 1% criterion.

For the 75 mm / 0.8 m/s / 1 ms case, the default-versus-old-reference maximum
2 ms window is 6–8 ms (0.4108%). Its largest later window is smaller (0.3205%).
For 10 ms material relaxation, both extended lengths instead reach their worst
window at 30–32 ms. At 75 mm the default/reference window RMSE grows from a
maximum 0.01640% in the first 8 ms to 0.07296% later; with both caps versus the
finest reference, it grows from 0.01041% to 0.04631%. The extension therefore
does not establish that all late differences decay.

Across all takes, maximum combined and structural-work residuals reach
7.321e-9 and 7.327e-9 in the finest uniform path at 32 ms. Hammer-work residual
is at most 8.997e-11 and positive energy increment at most 6.777e-16. The
structural ledger's proximity to the 1e-8 gate is an open numerical issue;
smaller reference steps are not automatically a stronger energy check. Its
origin must be investigated before extending this reference much further.
The subsequent [incremental midpoint correction](MODAL-MIDPOINT-INCREMENTS.md)
investigates prepared near-identity matrix roundoff and reruns this protocol.
The report above remains the retained pre-correction evidence.

## Regression and remaining work

The [default-controller control](../references/memory-modal-refinement-control.json)
is byte-identical to the previous twelve-case RK4 audit, SHA256
`64A9DFF8DA333239242C73AF9F7F9D7EDEF1D41431B57F53906DE4A8C4EFC7BE`.
The study's twelve default and old-reference take reports match those existing
reports. Two new unit tests check bounded contact/free advancement across short
remainders and events, plus metric normalization, silent denominators and
localization of a known velocity error. CI runs the new study on both supported
native runners.

The next numerical work is to explain reference-ledger drift and qualify longer
trajectories with independently controlled accuracy, then resume cost reduction.
Physical calibration, action/repetition, pickup voltage from this assembly and
polyphonic host integration remain separate open work. This experiment makes no
new speed, realtime, acoustic-realism or listening claim.
