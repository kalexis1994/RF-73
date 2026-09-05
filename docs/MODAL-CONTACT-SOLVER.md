# Bounded acceleration of modal hammer contact

The refined nine-coordinate assembly now solves the existing scalar contact
equation with safeguarded Newton iterations and a bounded bisection fallback.
This changes the numerical root search, not the hammer potential, beam modes,
integration step, force ports or physical profile. The uniform-midpoint
reference retains the original 48-step bisection. The audible plugin is unchanged.

## Equation and bracket

For compression `a` at the start of a step, free endpoint compression `d` and
positive combined hammer/assembly compliance `c`, the contact force satisfies:

```text
V(x) = k max(x, 0)^3 / 3
G(a,b) = (V(b) - V(a)) / (b - a)
b = d - c F
R(F) = F - G(a, d - c F) = 0
```

The implementation retains the existing cancellation-resistant piecewise
expression for `G`, including coincident compressions. Its derivative with
respect to `b` is nonnegative:

| Branch | dG/db |
| --- | --- |
| a >= 0, b >= 0 | k (a + 2b) / 3 |
| a <= 0, b <= 0 | 0 |
| a > 0, b < 0 | G(a,b) / (a - b) |
| a < 0, b > 0 | k [b / (b - a)]² (2b - 3a) / 3 |

Thus `R'(F) = 1 + c dG/db >= 1`, and `[0, G(a,d)]` brackets the unique
nonnegative root in exact arithmetic. Zero contact returns immediately.
The derivatives above follow by differentiating the existing cubic potential;
no new material law is inferred from them.

The solver starts at the upper bound and attempts at most eight Newton updates.
Each evaluated residual first tightens the bracket. A proposed force outside
the bracket is replaced by its midpoint. Early completion requires a residual
no larger than `8 epsilon max(F, G)`, where epsilon is f64 machine precision.
If those attempts do not converge, 48 bisections refine the remaining bracket.
The fallback interval is no wider than the original bisection interval in exact
arithmetic. Including the initial bracket evaluation, at most 57 gradient
evaluations are needed, versus 48 for the original method. The fallback makes
the worst scalar path slightly more expensive; acceleration is measured for
the tested trajectories, not guaranteed for every profile. Total work is bounded;
there is no open-ended convergence loop,
allocation, energy normalization or force clipping.

The same returned force advances the hammer and all assembly coordinates. The
independent viscous-work ledger, separation energy transfer and force averaging
within base ticks retain their previous definitions.

## Validation

Three additional tests cover:

- 882 scalar cases across compression, separation, crossing and zero-contact
  branches, with stiffness from 1e8 to 1e12 N/m² and compliance from zero to
  1e-2 m/N. Both normal acceleration and a deliberately forced fallback are
  compared with 100-step bisection. Force-bound checks allow floating-point
  rounding; the scalar force tolerance is relative to the original bracket.
- Contact force work versus the cubic potential change through compression
  and separation, with no endpoint-energy correction.
- Twelve paired assembly trajectories at 44.1/192 kHz and 50/75/120 mm tine
  lengths, with soft default strikes and an additional stiff, high-speed/light-
  support profile. Both voices use the same refined integration and free
  operators; only one uses the accelerated root search. Two strikes and two
  damper applications are checked over 6,000 base ticks per trajectory. Tests
  compare the energy norm of state differences, force histories, contact flags
  and each voice's independently closed energy ledger.

The standard 84-take audit also compares refined contact resolutions against
the unchanged uniform-midpoint reference. These finite checks do not establish
whole-domain stability, physical parameter identification or perceptual realism.

## Timing protocol

The [existing block benchmark](MODAL-PERFORMANCE.md) is unchanged: 1/8/32/73
serial voices at 48/192 kHz, synchronized strikes, 32 contact subdivisions,
one warmup and five measured 250 ms runs. Preparation and block-boundary probes
are outside the timer; contact events and independent dissipated work are inside.

The [before report](../references/modal-timing-before-contact.json) was generated
with the retained release executable from `fe4cf96`, before rebuilding the
laboratory for this change. The [after report](../references/modal-timing-after-contact.json)
uses the accelerated solver. These are sequential observations on the same
Windows x86-64 computer with Rust 1.98.0; machine load affects the results. Neither
pickup/filter costs nor host/WASM execution are measured.

| Rate | Voices | Before median / 250 ms | After median / 250 ms | Speed ratio | Before p99 / deadline | After p99 / deadline | After late blocks |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 48 kHz | 1 | 16.615 ms | 8.802 ms | 1.89x | 0.750 | 0.139 | 0 / 470 |
| 48 kHz | 8 | 142.117 ms | 71.374 ms | 1.99x | 6.514 | 1.183 | 25 / 470 |
| 48 kHz | 32 | 563.984 ms | 293.187 ms | 1.92x | 24.449 | 5.421 | 183 / 470 |
| 48 kHz | 73 | 1323.499 ms | 659.475 ms | 2.01x | 54.119 | 10.431 | 470 / 470 |
| 192 kHz | 1 | 69.166 ms | 34.837 ms | 1.99x | 10.908 | 1.741 | 25 / 1875 |
| 192 kHz | 8 | 588.113 ms | 289.104 ms | 2.03x | 87.200 | 14.189 | 347 / 1875 |
| 192 kHz | 32 | 2165.707 ms | 1159.122 ms | 1.87x | 307.994 | 53.596 | 1875 / 1875 |
| 192 kHz | 73 | 4887.946 ms | 2872.053 ms | 1.70x | 672.306 | 125.460 | 1875 / 1875 |

The voice object remains 21,712 bytes. The worst block-boundary energy residual
is `1.467e-11` of cumulative injected energy. Contact bursts shrink substantially,
but eight voices at 48 kHz still miss 25 block deadlines. Seventy-three voices
remain over budget at both rates. Average render speed does not qualify a host.
The [new mechanical audit](../references/modal-assembly-contact-validation.json)
records refined/uniform solver identities and the separate accuracy gates.

## Remaining work

The contact root search is only part of the cost: every active contact still
advances nine mechanical coordinates at 32 subdivisions. Further performance
changes must preserve the independent reference and energy checks. Geometry,
material memory, spatial pickup conversion and calibration remain separate
physical-model milestones. This offline change does not create a listening
package or open an audio device or Desktop controls.
