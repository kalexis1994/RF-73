# Rate-dependent loss in the modal hammer

`ModalAssemblyProfile::contact_damping_s_m` adds a first material-loss hypothesis
to the nine-coordinate research assembly. Positive values make compression and
unloading follow different force paths and dissipate energy at the hammer tip.
The default is zero, retaining the preceding elastic experiment. The parameter
is bounded to 0..10 s/m; the audit explores 0, 0.5 and 2 s/m. These are illustrative
values, not identified neoprene properties or register-specific hammer families.
The production plugin remains unchanged.

## Physical basis and scope

Pfeifle's [DAFx 2017 Rhodes/Wurlitzer model, section 5.2](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf)
uses a Hunt-Crossley-type power-law contact with a deformation-rate loss term.
The indexed section text was consulted for this change; direct retrieval of that
PDF timed out. Bilbao and Torin's [DAFx 2014 collision paper, sections 2.3–2.4](https://dafx.de/paper-archive/2014/dafx14_stefan_bilbao_numerical_simulation_of_s.pdf)
discusses dissipative contact and energy balance in musical-instrument mechanics.
That full paper was accessible. Neither source supplies identified material
coefficients for the illustrative RF-Rhodes geometry.

Here the continuous target, with compression x and velocity v, is
`F = k max(x,0)^2 max(0,1 + beta v)`. Before nonadhesive projection, the rate
coefficient is `lambda = k beta`, in N s/m³. This is a lumped, rate-dependent
contact approximation. It has no internal relaxation state: it cannot model
stress relaxation at fixed indentation, recovery between strikes, temperature,
aging or material memory beyond the mechanical state. Force/displacement loops
from rate dependence must not be presented as a complete neoprene model.

## Passive discrete contact

The existing midpoint mechanics relate endpoint compression b to the force F:

```text
a = initial compression
d = free endpoint compression
c = combined hammer/assembly compliance
b = d - c F
h = contact step duration
delta = b - a
G(a,b) = discrete gradient of V(x) = k max(x,0)^3 / 3
F = G(a,b) max(0, 1 + beta delta/h)
```

This discrete law and its projection are project implementation choices, not a
verbatim transcription of a published discretization. G is evaluated with the
existing cancellation-resistant algebra. Both factors of the nonnegative force
are nondecreasing in b. Thus the implicit scalar residual is monotone and its
root is bracketed by zero and the force evaluated at d. The refined path uses
up to eight safeguarded Newton evaluations and 48 fallback bisections; the
uniform reference uses 48 bisections. Zero beta calls the original elastic
solver directly. Tick/strike/reset remain bounded and allocation-free.

The nonadhesive projection prevents a negative total contact force on fast
unloading. It is part of this constitutive approximation, not an audio limiter.
Its activation and energy contribution are explicitly reported because it can
change unloading behavior materially.

The independent contact heat increment is:

```text
Dcontact = G beta delta^2 / h          when 1 + beta delta/h >= 0
Dcontact = -G delta                   when the unloading limit is active
```

Both branches are nonnegative. Since `G delta = V(b)-V(a)`, the identity
`F delta = V(b)-V(a)+Dcontact` closes the contact work. Combining it with the
mechanical midpoint work gives:

```text
E_next - E_previous = -Dassembly - Dcontact
residual = E + total_dissipated + escaped_hammer - injected
```

The code computes heat from the constitutive expression, not from a total-energy
residual. `contact_dissipated_energy_j` is a subset of `dissipated_energy_j`;
`contact_limited_heat_j` is a subset of contact heat. Do not add them again when
closing the ledger. `contact_limit_steps` counts microsteps, so its magnitude
depends on integration resolution and is not an event count or contact duration.
Reset clears all counters; restrikes retain mechanical state and accumulated heat.

## Analytic and coupled validation

For an isolated hammer striking a rigid wall without activating the projection,
integration of `m v dv/(1+beta v) = -k x^2 dx` gives a continuous restitution
reference. At zero compression, `u - ln(1+u)` is equal at entry and exit, with
`u=beta v`. The test solves this invariant for the restitution coefficient e.
Numerical impacts at beta 0.5/2 s/m and speeds 0.2/0.8/3 m/s converge from 4 to
128 steps at 44.1 kHz. Fine restitution error is below 1e-5, contact heat agrees
with `1-e^2` of initial energy within 2e-5, and each discrete step closes its
energy ledger. This reference applies to the unprojected rigid-wall test only.

Another test covers 216 scalar combinations, each through both accelerated and
pure-bisection solvers, including zero contact and limited unloading. It checks
nonnegative force/heat and the force-work identity. Six coupled trajectories
with beta 0.5/2/10 s/m at 44.1/192 kHz compare both root solvers through two
strikes and damper changes, checking state error, passive energy, heat subsets,
reset and invalid-parameter rejection.

```text
cargo run --locked --release -p rf-rhodes-lab -- modal-hammer-check --output renders/modal-hammer.json
```

The [tracked report](../references/modal-hammer-validation.json) contains 36
cases and 108 takes: three tine lengths, two sample rates, two velocities and
three beta values, each using refined contact 32/256 and uniform midpoint 1024.
Each take lasts 40 ms and applies the damper halfway through. Reports include
full profile/geometry metadata, physical mass matrices, force/impulse, heat and
separation diagnostics. Coordinate errors use the complete physical inertia;
pickup observation velocity has its own raw relative RMSE. No signal alignment,
gain fit or pickup-voltage conversion is performed. Output paths must be new.

The zero-loss cases reproduce all prior metrics for the 36 matching elastic
takes. Nonzero beta changes the mechanics and produces positive contact heat;
comparison to zero beta measures model sensitivity, not improved realism.

Across all 36 cases, the contact-32 maximum pickup-velocity RMSE against contact
256 is `0.01176%`; uniform midpoint 1024 stays within `0.0009772%`. The maximum
contact-32 relative energy residual is `1.193e-11`. These maxima include the
zero-loss controls and retain the preceding convergence gates.

| beta | Contact heat / injected energy, reference 256 | Pickup velocity RMSE vs elastic reference |
| --- | --- | --- |
| 0 s/m | 0 | 0 |
| 0.5 s/m | 0.2391–4.7762% | 0.2568–332.497% |
| 2 s/m | 0.8911–7.6782% | 0.9955–332.968% |

The large upper sensitivity values include changes in level and temporal motion;
they are not an error estimate or evidence of improved sound. Material damping
can redistribute energy between the rebounding hammer and assembly as well as
turn energy into heat. The nonadhesive limit activates in two reference cases:
the 50 mm tine at velocity 1, beta 2 s/m, at both rates. Its heat contribution is
at most `1.024e-5` of injected energy, or `0.01333%` of total contact heat in these
cases. The audit records it separately rather than assuming projection is absent.

All 141 workspace tests passed; the material test was rerun after adding the
limited-heat telemetry. Strict Clippy, formatting and release WASM compilation
passed. The new audit is included in CI, whose updated remote run was not executed
during this local validation.

## Remaining gates

Identifying real hammer material behavior requires documented force/compression
or impact/rebound measurements at multiple speeds, including unloading and
recovery. A passive result alone does not select beta or establish a material
family. Relaxation memory, hammer shape/contact area and full action remain open.
A separate [material-memory coupon](HAMMER-MEMORY.md) now validates internal
deformation and relaxation under prescribed motion. Its nonadhesive contact
connection is still pending; it does not alter this rate-loss model.
This loss-enabled solver has not been qualified for polyphonic or WASM deadlines;
the preceding elastic timing reports do not establish its cost. No listening
package, audio-device access or Desktop controls accompany this offline milestone.
