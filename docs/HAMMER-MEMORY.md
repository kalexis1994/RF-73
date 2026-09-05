# Material memory and relaxation coupon

`HammerMemory` is a displacement-controlled material experiment with one internal
viscous deformation. Unlike the preceding instantaneous rate-loss law, its force
depends on loading history and relaxes while deformation is held fixed. It is
not yet connected to `ModalAssembly`, the hammer's mass or the plugin.

## Mechanical hypothesis

The Maxwell spring/dashpot branch and parallel equilibrium elasticity follow
the mechanical-network approach described in David Roylance's
[Engineering Viscoelasticity, MIT](https://ocw.mit.edu/courses/3-11-mechanics-of-materials-fall-1999/d173f95347f59d59dd9b7387ab4303e7_MIT3_11F99_visco.pdf).
The linear standard-solid limit is recovered by removing our cubic equilibrium
term. The cubic extension, chosen coefficients and ramp integration below are
project choices; the source does not identify Rhodes hammer parameters.

Let x be prescribed total deformation, z the viscous deformation and e=x-z the
Maxwell spring extension:

```text
U(x,e) = k0 x^2/2 + kc |x|^3/3 + k1 e^2/2
F = k0 x + kc x |x| + k1 e
z_dot = e/tau
eta = k1 tau
heat_power = eta z_dot^2 = k1 e^2/tau >= 0
U_dot = F x_dot - heat_power
```

| Parameter | Provisional default | Accepted range |
| --- | --- | --- |
| k0, linear equilibrium stiffness | 0 N/m | 0..1e8 N/m |
| kc, cubic-potential coefficient | 4e10 N/m² | 0..1e12 N/m² |
| k1, memory spring stiffness | 2e5 N/m | 1..1e8 N/m |
| tau, relaxation time | 1 ms | 1 microsecond..1 s |
| Fixed integration interval | Supplied by caller | 1 ns..100 ms |
| Prescribed deformation | Supplied each step | -10..10 mm |

The ranges define a numerical experiment, not a validated material domain. One
relaxation time cannot describe a general material relaxation spectrum. These
parameters have not been fitted to neoprene, felt, temperature or aging data.

This coupon is bilateral. Negative reaction during unloading is valid for a
specimen held by a test machine; it must not be applied directly as an attractive
hammer/tine contact force. Holding x=0 lets internal strain relax under a clamp.
It is not an unrestrained, zero-force recovery simulation. Reset discards the
specimen state and ledgers administratively; it is not a physical recovery event.

## Exact ramp state, work and independent heat

`advance_to(x1)` prescribes a linear path from x0 to x1 over the prepared interval
h. For r=h/tau, u=1-exp(-r), delta=x1-x0:

```text
a = u/r
b = (1-a)/r
e1 = exp(-r) e0 + a delta
mean_e = a e0 + b delta
mean_F = discrete_gradient(Uequilibrium, x0, x1) + k1 mean_e
external_work_increment = mean_F delta
```

The signed cubic discrete gradient handles coincident states and zero crossings
without subtraction of nearly equal cubic potentials. Mean force is the reaction
averaged over the ramp; the probe also supplies instantaneous endpoint force.
These are different observables during motion and relaxation.

Heat is integrated independently from `k1 e(t)^2/tau`. Its exact quadratic
moments are:

```text
A = (1-exp(-2r))/(2r)
B = a^2/2
C = (r-u-u^2/2)/r^3
D = k1 r [ A (e0 + (B/A) delta)^2 + (C-B^2/A) delta^2 ]
```

The positive form prevents cancellation from producing negative heat when the
old extension and imposed movement oppose each other. Preparation uses `expm1`
and 14-term Taylor moments for r<0.1, and rejects a nonpositive heat factor.
No negative-heat clipping or total-energy residual correction is used. The state,
work and heat formulas are analytic for the chosen piecewise-linear path, up to
floating-point rounding. Exponentials and moment construction occur only in
preparation; advancing the coupon uses fixed-size arithmetic with no allocation,
locks or I/O. Invalid updates leave the existing state unchanged.

The independent ledger is `stored_energy + dissipated_energy - external_work`.
Work is signed; `absolute_work_j` accumulates absolute per-step work for the
audit's residual denominator. It is not another source of supplied energy.

## Validation and repeatability

Five unit tests cover:

- Composition of identical linear ramps at two resolutions over widely separated
  h/tau ratios, including the small-r moment transition, signed deformation,
  both equilibrium terms and independently integrated heat.
- Exponential force relaxation at fixed deformation and recovery of the clamped
  zero-deformation reaction, preserving the internal state between loadings.
- Sinusoidal cycle heat against the continuous Maxwell loss
  `pi k1 amplitude^2 omega tau / (1+(omega tau)^2)`. The 48 kHz piecewise-linear
  approximation agrees within 1e-4 relative error after settling.
- All 32 combinations of minimum/maximum coefficients, relaxation time and
  timestep, with full signed travel, finite states and passive work accounting.
- Invalid-input rejection without mutation and explicit reset of memory/ledgers.

```text
cargo run --locked --release -p rf-rhodes-lab -- hammer-memory-check --output renders/hammer-memory.json
```

The [tracked audit](../references/hammer-memory-validation.json) has 36 cases and
72 takes: rates 44.1/192 kHz, relaxation times 0.1/1/10 ms, amplitudes 5/20/50
micrometers and clamped rest intervals 0.2/20 ms. The imposed path loads over
0.25 ms, holds for 2 ms, unloads over 0.25 ms, rests, then reloads over 0.25 ms.
Each duration rounds up to an output frame. One and two substeps traverse the
same linear path at exactly matched observation times. Each report records
stage-end forces, deformation, memory, energy, work and actual durations.
The reported mean force belongs to the last internal ramp, not the whole stage;
the resolution comparison uses instantaneous endpoint forces.

Maximum relative energy residual is `1.574e-14`; normalized hold/recovery
extension error is `2.257e-14`; endpoint-force RMSE between resolutions is
`1.061e-14`. The force at the end of the second loading is 73.31–99.97% of the
first after short rest, versus 98.66–100% after long rest. Those ratios show
history dependence within this provisional model, not measured Rhodes behavior.
Output paths must be new, and the audit does not open an audio device.

## Coupling gate

The next step is a nonadhesive contact connection that accounts for internal
stored energy during separation and recovery, with a physically consistent
traction boundary. Clipping the coupon's signed force or erasing z at separation
would not preserve the model above. That coupling needs independent force/work
and collision tests before it can replace the rate-loss experiment. A spectrum
of relaxation times and material measurements are later identification gates.
The current audible engine and package version 0.1.2 remain unchanged.
