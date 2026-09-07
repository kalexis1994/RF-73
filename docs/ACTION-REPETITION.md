# Persistent action and repetition

This offline Rust block joins the nine-coordinate tine/support/tonebar assembly,
a persistent hammer and a moving felt arm. It simulates a complete key-driven
strike, release and repeat without replacing hammer velocity or resetting any
mechanical coordinate. It does not change the playable 0.1.2 plugin.

## Physical interpretation

The service manual defines escapement as the hammer-tip/tine gap with the key
depressed, and describes the bridle pulling the damper arm down as the hammer
rises. The reduction uses that topology: a moving unilateral pedestal drives
the hammer, while a tension-only bridle pulls on both hammer and damper.
[Service manual, chapter 4](https://www.fenderrhodes.com/service-manuals/1979/ch4.html).

The hammer can leave the pedestal and reach the tine through inertia. No
software strike trigger or disengagement switch removes stored contact energy.
Pedestal motion is expressed in hammer-tip-equivalent meters, not physical
key-front dip. The nominal 1.5 mm escapement is one provisional setting, not a
register-dependent factory regulation. A 4 N/m spring and 0.025 Ns/m dashpot
reduce hammer return; they do not identify measured gravity, pivot friction,
pedestal contour, rotational inertia or a complete action linkage.

The pedal translates the base of the damper spring. This is an explicit work
port, not yet the actual release-bar contact geometry. Key actuation reaches
the felt through the reciprocal bridle; it no longer prescribes felt lift
directly from `max(key,pedal)`. Strap ratio, slack, compliance and loss remain
unmeasured. The nominal bridle ratio is 0.8, slack 2 mm; a deliberately slack
20 mm configuration is a stress case and must not be treated as a preset.

## Coupled equations

Let `q` contain the nine structural coordinates, `x` the hammer tip and `z`
the felt arm. The structural mass, stiffness, damping and spatial ports are
unchanged from the modal assembly. With pedestal position `d`, pedal base `r`,
hammer rest `x0`, closed arm reference `z0`, bridle ratio `rho` and slack `s`:

```text
delta_h = x - Bh*q
delta_f = z - Bf*q
delta_p = d - x
delta_b = rho*(x-x0) + z-z0-s

M*q'' + C*q' + K*q = Bh*Fh + Bf*Ff
mh*x'' + ch*x' + kh*(x-x0) = -Fh + Fp - rho*Fb
mz*z'' + cz*(z'-r') + kz*(z-r) = -Ff - Fb
```

All four interfaces use `U(delta)=k*positive(delta)^3/3` and the projected
rate-dependent discrete-gradient force already used for the felt. Positive
bridle extension produces tension only. Hammer/tine contact retains the
assembly's elastic default; this block does not merge the separate viscoelastic
hammer-memory experiment.

For compression endpoints `a,b`, potential divided difference `G`, and tick `h`:

```text
F = G(a,b) * max(0, 1 + beta*(b-a)/h)
F*(b-a) = U(b)-U(a) + heat
```

On the positive branch, `heat=G*beta*(b-a)^2/h`; on projected unloading,
`heat=-G*(b-a)`. That projection is a constitutive hypothesis. Its activation
counts are recorded instead of being hidden as numerical damping.

## Joint integration and bounded failure

Implicit midpoint reduces the linear motion to free endpoints plus four force
responses. If `D` is the compression Jacobian and
`A=M+h*C/2+h^2*K/4` for all eleven coordinates, the contact compliance is
`W=h^2*D*A^-1*D^T/2`. Thus `b=b_free-W*F`; cross-compliances include the common
tine ports and the hammer/arm reactions through the bridle.

The solver performs at most 64 sweeps of exact bounded scalar solves, retaining
all off-diagonal terms. This is an iterative solve of one joint system, not
four independently accepted contact updates. Acceptance checks every force
against its law evaluated at the same final compression vector. The residual
tolerance includes the analytically propagated floating-point endpoint error
through the local material slope. This matters when `beta/h` is large; a
force-only tolerance below that floor falsely rejects converged states.

Preparation factors fixed matrices; each tick uses fixed-size arrays and has
bounded iteration counts. Invalid parameters, nonfinite states, input speeds
above 2 m/s or exhausted solves return errors without changing positions,
velocities, forces, drives, counters or energy ledgers. Bounded execution is
not a 73-voice realtime performance claim. Coarse or extreme inputs can fail
closed rather than being silently accepted.

## Independent work accounting

Stored energy includes structural and mass kinetic energy, both return springs
and all four contact potentials. Dissipation separately accumulates structural,
hammer-return and arm heat, plus each contact's constitutive heat. Input work is:

```text
pedestal_work = Fp * delta_d
pedal_work = -[kz*(z_mid-r_mid) + cz*(vz_mid-vr)] * delta_r
residual = E + all_heat - initial_E - pedestal_work - pedal_work
```

Bridle forces are internal and do not add another actuator work term. There is
no residual correction, impulse injection, energy normalization or ledger
restart at a repeat. Initial felt preload stores energy and causes a small
startup relaxation; that is accounted for and is not a played strike.

## Reproducible validation

```text
cargo run --locked --release -p rf-73-lab -- action-cycle --output NEW.json
cargo run --locked --release -p rf-73-lab -- action-cycle --output NEW-FAST-REFINED.json --fast-drive --refined
cargo run --locked --release -p rf-73-lab -- action-cycle --output NEW-REFERENCE.json --fast-drive --reference
```

Output creation is exclusive. Both commands cover 75/120 mm tines, 48/96 kHz
and four 180 ms gestures: ordinary repeat, pedal-held repeat, partial release
and slack-bridle simultaneous-contact stress. Pedestal targets rise at 10 and
95 ms and fall at 45 and 130 ms. Partial release retains 70% depression between
strikes; no repeat strike is required for that case. Pedal hold spans 5–150 ms.
Drive positions are continuous, with a fixed speed limit per study.

Frozen gates at every tick:

- Energy residual below `1e-8` relative to initial energy plus absolute input work.
- Each contact's work identity defect below `1e-9` on the same energy scale.
- Stationary-drive energy growth below `1e-10`, with monotone heat and nonnegative force.
- Position/velocity midpoint identity error below `1e-14` in coordinate units.
- A first strike in every gesture; a repeat except for partial release; simultaneous
  hammer/felt contact in the deliberately slack configuration.
- Pickup-velocity relative RMSE below 1%, and hammer/arm position RMSE below
  10 micrometers, in **each** of 0–45, 45–95, 95–140 and 140–180 ms. Both lower
  resolutions are compared with the highest, not just their final states.

The original 1 m/s, 32/64/128-tick matrix is retained in
`references/persistent-action-cycle-validation.json`: 7/16 cases pass. All 48
energy audits pass, but ordinary repeats fail strike coverage under the
provisional return/bridle load. The largest pickup comparison error is 7.118%
in the short-tine slack-bridle case. This is useful negative evidence: an
energy-consistent mechanism is not automatically a responsive or numerically
resolved instrument.

The follow-up uses 1.5 m/s and 128/256/512 ticks with unchanged materials and
gates. Speed and resolution both change, so it must not be described as a pure
refinement of the original matrix. Its report records the selected parameters.
That matrix passes all per-take energy and behavior gates but only 11/16 case
comparisons: the largest pickup discrepancy is 5.883%, and the largest hammer
position discrepancy is 31.091 micrometers. It is preserved in
`references/persistent-action-cycle-fast-refined-validation.json`.

The `--reference --fast-drive` study holds that physical experiment fixed and
uses 512/1024/2048 ticks. Its overlapping 512-tick summaries are checked against
the entire previous fine take, including states, ledgers and event counters.
This study remains a high-resolution offline reference, not a production-rate
solution or a claim that this oversampling factor is required by the instrument.

The [reference receipt](../references/persistent-action-cycle-reference-validation.json)
passes **16/16 cases and 48/48 takes**, with exact overlapping 512-tick summaries.

| Maximum across the reference matrix | Observed |
| --- | --- |
| Relative energy residual | 8.861e-12 |
| Relative individual contact-work defect | 1.747e-14 |
| Stationary-drive energy growth | 0 |
| Pickup velocity RMSE, 512 vs 2048 | 0.2555% |
| Pickup velocity RMSE, 1024 vs 2048 | 0.0499% |
| Hammer position RMSE, 512 vs 2048 | 1.933 micrometers |
| Arm position RMSE, 512 vs 2048 | 0.421 micrometers |
| Joint solver sweeps per tick | 2 |

There are 1,702,189 simultaneous hammer/felt ticks across the 48 takes. Counts
depend on resolution; they demonstrate exercised overlap, not independent
physical impacts. Maximum average forces are approximately 56.56 N at the
hammer, 3.66 N at the felt, 42.27 N at the pedestal and 6.17 N in the bridle.
Those are properties of the provisional experiment, not measured instrument
forces or recommended regulation targets.

Retained JSON receipts (bytes and SHA-256):

- Original: 163685 bytes,
  `795b055aad74eab330f0e02b63d52d35b6ef2490fb6fc6633a5c857388003e9d`.
- Fast refined: 164733 bytes,
  `26665751f1bf4622b77afe39ec18d7309c45c756db401b5f3e2776b32cf19b45`.
- Reference: 165175 bytes,
  `c6583f6095fef4280e78f8a967d8880ef11a76304f4a34d017399e0dba1628c2`.

Unit tests independently cover symmetric multi-contact solutions, permutation
invariance, isolated analytic hammer return, exact unforced rest, simultaneous
contact work, reciprocal compliance, invalid-input/solver-failure rollback,
profile bounds and continuous repeat cycles.

## Remaining work

The subsequent [two-plane extension](TWO-PLANE-ACTION.md) now adds the orthogonal
tine/support/tonebar component and spatial contact/observation ports while
reusing this action solver. The planar validation above remains historical
evidence and a live regression target.

This establishes a coupled offline action reduction. It does not establish
measured touch response, authentic velocity calibration, full key/pivot/bridle
geometry, faithful half-pedal mechanics, hammer material memory, stereo motion,
magnetic backreaction or production MIDI integration. No audible artifact or
host test is claimed for this block. Further physics should retain these
work and continuity checks; real-instrument calibration and listening remain
separate requirements before release.
