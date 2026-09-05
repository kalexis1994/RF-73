# Error-controlled hammer free recovery

`MemoryHammer::try_free_step` attempts a longer free interval while retaining
the nonlinear material's deformation, viscous memory, work and heat. A conservative
travel bound must exclude wall contact throughout the interval. The method then
checks fourth-order Runge-Kutta step doubling and independent energy/work defects
before committing any state.

This milestone validates the free-motion primitive against a stationary wall.
The subsequent [moving modal experiment](MEMORY-MODAL-FREE.md) connects it to
`MemoryModalAssembly` using its own moving-surface certificate. Neither experiment
is integrated into the audible plugin. The existing implicit contact solver and
its prepared fixed interval are unchanged.

## Continuous free equations

Let x=c-t, w=vc-vt, e=x-z, and mu=mc mt/(mc+mt). During certified separation:

```text
F = k0 x + kc x |x| + k1 e
x_dot = w
w_dot = -F/mu
e_dot = w-e/tau
heat_dot = k1 e^2/tau
material_work_dot = F w
force_integral_dot = F
```

Center-of-mass velocity stays constant; its position advances linearly. Core
and tip positions/velocities are reconstructed from that translation and the
relative state. Heat and material work are separate quadrature states. No heat
is manufactured from an energy residual, and no memory is reset on separation.

The RK4 weights for heat are positive and its sampled power is nonnegative.
This does not make RK4 unconditionally passive or exactly energy preserving.
Local defects are explicitly checked, and the resulting accumulated global
errors are measured against the retained implicit reference.

## Contact exclusion over the whole interval

All stored potentials are nonnegative. With no external impulse during the
interval, the continuous passive model bounds tip speed by sqrt(2 E0/mt), where
E0 is current total mechanical energy. The stationary wall is at zero. A free
interval h is considered only when:

```text
tip_position + h sqrt(2 E0/mt) + roundoff_margin < 0
```

The margin is 64 machine epsilons times the larger of 1e-12 m and the sum of
absolute tip position and the travel bound. This envelope excludes an interior
collision, including one followed by separation before the endpoint. Merely
checking endpoint clearance would not provide that guarantee. The bound is
conservative: `ContactRequired` means contact cannot be excluded, not that it
has necessarily happened. A nonzero stored surface coordinate is rejected by
this fixed-wall API. The modal wrapper uses its own
[moving-tine certificate](MEMORY-MODAL-FREE.md) and an internal certified endpoint.

## Accuracy checks and atomic commit

One full RK4 step is compared with two half steps. The two half steps are the
candidate; there is no Richardson extrapolation. An attempt has at most twelve
right-hand-side evaluations. Each stage and endpoint must remain finite and
within the material's +/-10 mm deformation domain.

The normalized maximum difference in x, w and e must not exceed 1e-10. Velocity
scale is sqrt(2 S/mu), deformation scale sqrt(2 S/k1), and S is initial energy
plus accumulated absolute external impulse work, floored at 1e-30 J. Each half
step independently checks:

```text
relative_energy = mu w^2/2 + U(x,e)
relative_energy_change + heat
material_energy_change + heat - material_work
```

Both defects must be within 1e-13 S, and positive mechanical-energy changes
within 1e-14 S. The reconstructed full endpoint is checked again with its actual
positions and velocities before committing. These local tests are error controls,
not rigorous global truncation-error bounds or unconditional stability claims.

`Advanced` commits all state and ledgers. `ContactRequired` and `AccuracyRequired`
leave the entire hammer unchanged. Invalid requested intervals return an error;
accepted input bounds are 1 ns..1 ms, with acceptance still subject to the checks.
Callers can shorten a rejected interval or use the original fixed implicit tick.
The fixed timestep and analytic material ramp coefficients are never retimed.
The primitive allocates nothing and performs no I/O or open-ended iteration.

The material probe's mean force becomes the quadrature average over the accepted
free path. Its absolute-work ledger accumulates the absolute net work per accepted
interval, just as it does per prescribed ramp; it is resolution dependent and is
not a second energy source. The audit normalizes global errors using external
impulse work, not that material absolute-work ledger.

## Validation and reproducibility

Four added unit tests cover large-step rigid translation followed by the unchanged
fixed tick; invalid intervals and conservative clearance rejection without mutation;
nonlinear free recovery against a fine implicit trajectory with retained heat,
memory and momentum; and the analytic damped relative mode of a free linear
Maxwell hammer. The latter checks relative velocity and branch extension over
20,000 free intervals against the continuous solution.

```text
cargo run --locked --release -p rf-rhodes-lab -- memory-free-check --output renders/memory-free.json
```

The audit retains the original 24 fixed-wall cases: rates 44.1/192 kHz,
relaxation times 0.1/1/10 ms, speeds 0.2/0.8 m/s and tip masses 0.1/0.5 g at
4 g total mass. A core impulse equal to 2.5 times initial momentum at 4 ms drives
reimpact; the run ends at 16 ms. Times round up to output-frame boundaries as
in the original audit. Candidate contact and uniform reference share the same
approximately 1.25 ns fixed tick.

The offline controller tries dyadic free intervals of up to 4096 fixed ticks.
It decreases the level on rejection and increases it when both local errors
are below one sixty-fourth of their limits. Intervals never cross an output or
impulse boundary. A refinement search has at most thirteen attempts; inability
to accept the smallest free interval falls back to one implicit tick. No-contact
heat, rejection counts, replaced ticks and maximum accepted interval are reported.

Global energy/material-work tolerance is 1e-8, momentum tolerance 1e-10, normalized
velocity and cumulative impulse tolerances 1%, and frame-mean force RMSE tolerance
2%. Each take must demonstrate free recovery and post-impulse contact. The report
refuses overwriting existing files. Interval reduction is not a measured speedup:
an accepted free interval uses two RK4 half steps plus its full-step error estimate,
and rejected trials add work.

The [tracked audit](../references/memory-free-validation.json) passes all 24
cases. Candidate global energy and material-work residuals are at most
`1.656e-10` and `1.007e-10`; normalized momentum error is at most `7.292e-14`.
Velocity RMSE divided by launch speed is at most `3.631e-6` (0.0003631%), mean
force RMSE `3.779e-7`, and normalized cumulative impulse error `2.298e-7`.
Accepted integration intervals are reduced by 30.15x–62.68x, with free intervals
up to 5.119 microseconds. Each count includes both accepted free intervals and
remaining fine contact ticks; rejected trials are reported separately. All 24
uniform reference rows retain the original fixed-wall audit's exact diagnostics.

All 161 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CI includes this audit; its remote workflow was not executed locally.

## Subsequent integration

The [moving modal experiment](MEMORY-MODAL-FREE.md) combines this primitive with
prepared structural propagation and audits work, contact transitions, reimpact
and damper changes against the coupled reference. Native/WASM realtime
qualification and physical calibration remain open. This fixed-wall result alone
does not qualify the combination. No new listening package is produced.
