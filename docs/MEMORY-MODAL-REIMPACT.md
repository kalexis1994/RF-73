# Repeated excitation after material recovery

The earlier 128 ms tail qualification contains no new impulses after 2 ms.
After its early impacts, the hammer retreats freely and the later sections have
zero contact force. That study therefore does not qualify a later impact against
an already moving assembly after tens of milliseconds of material recovery.

`memory-modal-reimpact-check` adds two late core impulses and two additional
damper cycles. It retains the original first 32 ms and never resets the hammer,
tine/tonebar motion, material memory, heat or work ledgers. All DSP equations,
coefficients, controller choices and tolerances are unchanged.

## Protocol

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-modal-reimpact-check --output renders/modal-reimpact.json
```

Four profiles combine 75/120 mm tines with 1/10 ms material relaxation at an
initial speed of 0.8 m/s. Every 128 ms take uses these prescribed events:

| Time | Event |
| --- | --- |
| 0 ms | Initial impact at zero gap |
| 2 ms | Original 0.008 Ns core impulse |
| 4 / 6 ms | Damper on / off |
| 32 ms | Additional 0.008 Ns core impulse |
| 40 / 56 ms | Damper on / off |
| 80 ms | Additional 0.008 Ns core impulse |
| 96 / 112 ms | Damper on / off |

These are externally imposed impulses, not a modeled key, action, backcheck or
return mechanism. The unconstrained retreat between impacts is still a limitation
of this research assembly. Passing this experiment cannot establish realistic
Rhodes repetition or physically calibrated action timing.

Each case compares default coupled RK4, contact-level-2/free-level-8 capped RK4,
and uniform incremental midpoint with 20832 ticks per 48 kHz observation
(approximately 1.000064 ns). Every event is applied before its observation frame;
all adaptive intervals remain bounded by that frame. Event snapshots preserve
material state, and damper switches preserve the complete physical snapshot.

Every take must observe zero surface energy and zero force, followed by positive
contact force, separately within 32–80 and 80–128 ms. A contact already in
progress cannot satisfy this check without observed separation in the same
epoch. The recorded time is the end of the first positive-force interval, not
an interpolated contact-onset root.

The existing independent combined energy, structural/hammer port work,
nonnegative heat/force, free recovery and coordinate-activity checks remain.
All three trajectory pairs must pass both whole-record and separate 0–8, 8–32,
32–80 and 80–128 ms gates: 1% kinetic velocity RMSE / initial launch speed,
1% relative pickup velocity RMSE and 2% relative mean-force RMSE. Zero reference
power remains null and passes only for zero difference. Two-millisecond windows
and peak errors remain diagnostics. Fine midpoint is an approximate reference.

Completed failures retain the JSON report and return a nonzero exit code.
Existing reports are never overwritten. This command does not render audio or
measure realtime performance.

## Regression coverage

Two new tests enforce the exact event schedule and reject a late-reimpact claim
without separation in each epoch. They also preserve the first detected contact
time and reject configurations that truncate the prescribed late protocol.
The existing synthetic section-error test still prevents whole-record averages
from hiding a failing section.

All 191 workspace tests, strict Clippy and formatting pass. The repeated
[original short-protocol control](../references/memory-modal-reimpact-control.json)
contains 24 takes and is byte-identical to the preceding audit (SHA-256
`b6cabce3f2b5a9f12acbe4ed1e4af666f5e4e5728feab7d6a04593b6b0394e60`).
All 24 pair/section comparisons before 32 ms also match the earlier tail study
exactly. CLI help, invalid-option rejection and failed-report overwrite
preservation pass. No DSP or plugin code is changed;
this milestone extends laboratory coverage. No timing, remote CI or GUI/audio
qualification was performed.

## Result: repeated trajectories are not qualified

The retained [first repeated-excitation report](../references/memory-modal-reimpact-validation.json)
has `status: fail`; the command returned a nonzero exit code. All twelve takes
pass their energy/work, heat/force and observed-reimpact checks, but all four
profile comparisons fail trajectory gates. This is a newly exposed limitation
of the existing integration/model combination, not a passing qualification.

Maximum relative combined, hammer and structural residuals are 1.291e-10,
1.274e-10 and 4.308e-12, below the 1e-8 gates. Positive energy increments remain
below 5.083e-16. Energy closure therefore does not certify trajectory accuracy.

For default versus capped RK4, the late-section kinetic velocity RMSE normalized
by the initial launch speed is:

| Tine / relaxation | 32–80 ms | 80–128 ms | Entire pair passes |
| --- | ---: | ---: | --- |
| 75 mm / 1 ms | 0.000706% | 0.000297% | Yes |
| 75 mm / 10 ms | 2.622% | 25.683% | No |
| 120 mm / 1 ms | 0.005971% | 3.576% | No |
| 120 mm / 10 ms | 0.200281% | 37.359% | No |

Default versus the uniform reference fails in all four profiles. In the
75 mm / 10 ms case, the final section reaches 60.204% kinetic velocity RMSE,
69.164% relative pickup velocity RMSE and 132.200% relative mean-force RMSE.
Time-local force error can exceed 100% when narrow impacts no longer line up;
this is not an error in accumulated impulse of the same percentage.

The first late positive-force intervals differ by only about 1.25–2.50 ns
between the two RK4 paths. By the second late impact, the 10 ms relaxation
profiles differ by about 34.50 and 41.88 microseconds. The measurements show
growing sensitivity across repeated excitation, but do not isolate whether
contact integration, free/material propagation, finite-reference error or the
provisional physical parameters dominate. They do not establish chaos, a solver
bug or a physically correct reference path.

The next numerical experiment should start competing integrators from the same
complete preimpact checkpoint and measure one late collision and its rebounds,
varying contact and free resolution independently. That distinguishes inherited
state differences from error generated by the local impact. Tolerances remain
unchanged. This failing exploratory command is not added as a required passing
CI qualification; its detector/schedule tests run in the existing workspace suite.
