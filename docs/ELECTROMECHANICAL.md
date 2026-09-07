# Reciprocal spatial pickup and passive electrical load

The offline twenty-coordinate action now drives a spatial flux-linkage model
and a coil/load circuit. Coil current exerts a reaction force on both transverse
tine directions during the same mechanical step. The circuit therefore changes
the trajectory as well as the output voltage. This is an uncalibrated single-coil
reduction; it is not yet integrated into the realtime plugin.

## Physical scope

Falaize and Hélie connect mechanical and electrical subsystems through power
ports and discretize them with an explicit energy balance. That is the basis
for checking our reciprocal connection. Our aperture approximation and numerical
constants are hypotheses, not parameters measured or reproduced from that work.
[Falaize and Hélie, DAFx 2015](https://dafx.de/paper-archive/details/-yZbry8ifMSIfcYcvZTpfA).

`SpatialPickup` averages a smooth radial flux-linkage proxy over a circular pole
aperture. Two Gauss nodes in squared radius and eight azimuths give sixteen
equal area weights. For a node `n`, laboratory displacement `q=(x,y)`, offset
`o`, gap `g` and linkage scale `s`:

```text
u_n(q) = q + o - n
r_n(q) = sqrt(g^2 + dot(u_n(q),u_n(q)))
Phi(q) = sum_n s*g^3 / (16*r_n(q)^3)
```

Both transverse coordinates change the observation. This is a finite-aperture
proxy, not a solved three-dimensional magnetic field or finite-element model.
The gap remains positive and constant in this reduction. A dense independent
128-by-256 disk integration agrees within 0.01% at three selected displacements
with the default geometry. This does not establish quadrature accuracy over all
accepted profile bounds, particularly large pole-radius/gap ratios.

Default provisional parameters:

| Quantity | Value |
| --- | --- |
| Gap | 1.5 mm |
| Transverse offset | (0.5, 0.2) mm |
| Pole radius | 0.5 mm |
| Flux linkage scale | 0.001 Wb-turn |
| Coil inductance | 0.2 H |
| Coil series resistance | 200 ohm |
| Parallel capacitance | 4.7 nF |
| Resistive output load | 10 kohm |

## Discrete reciprocal coupling

A symmetric discrete gradient `G(a,b)` obeys the chain rule
`dot(G,b-a)=Phi(b)-Phi(a)`. It is evaluated without subtracting nearby fluxes:

```text
G(a,b) = sum_n -s*g^3 * (ra^2+ra*rb+rb^2) * (ua+ub)
                    / (16*ra^3*rb^3*(ra+rb))
emf = -dot(G,b-a)/h
F_xy = G * (i_old+i_new)/2
W_mechanical = dot(F_xy,b-a)
W_electrical = h*emf*(i_old+i_new)/2
```

Consequently `W_mechanical + W_electrical = 0` up to the coupling solve
tolerance and floating-point arithmetic. Coincident endpoints give the ordinary
gradient; no epsilon displacement or derivative division is necessary.
Mechanical force is distributed through the same prepared pickup ports used
to observe displacement. It enters before the four contacts are solved jointly.

The electrical states are coil current `i` and output voltage `v`. With load
conductance `Gload`, the constant-L network is

```text
L*di/dt = emf - Rcoil*i - v
C*dv/dt = i - Gload*v
E_electrical = (L*i^2 + C*v^2)/2
```

`None` for load resistance means zero conductance; the capacitor remains
connected. A two-by-two implicit midpoint solve accumulates coil and load heat
from squared midpoint current/voltage, independently of the energy residual.
Its transfer is `V/EMF = 1/[1+(Rcoil+j*w*L)*(Gload+j*w*C)]`.
Tests compare complex gain at 1 kHz and preserve energy in an unforced lossless
LC circuit for 100,000 ticks.

The coupled solver permits sixteen outer fixed-point iterations. Every trial
restarts from the same mechanical/electrical state; the existing inner contact
solver remains bounded at 64 sweeps. Acceptance requires force residual no
larger than `1e-10*max(norm(F_expected),1e-9 N)`. Invalid inputs, inner failure
or outer exhaustion preserve the entire previous state and diagnostics. No
clipping, energy rescaling or partially accepted state repairs a failure.
Bounded iteration is not a convergence guarantee for every permitted profile.

`ElectromechanicalProbe` exposes circuit energy, both electrical heats, input
work, mechanical pickup work, separate circuit/exchange defects and the total
mechanical-plus-electrical balance. Zero flux produces exactly zero current,
voltage and reaction, and reproduces the uncoupled mechanical trajectory.

## Retained qualification

```text
cargo run --locked --release -p rf-73-lab -- electromechanical --output NEW.json
```

The [receipt](../references/electromechanical-validation.json) passes eight
cases and 24 takes: 75 mm at 48 kHz and 120 mm at 96 kHz, each with zero flux,
1 kohm load, 10 kohm load and zero resistive load with tenfold capacitance.
All use the default polarized action, two strikes over 180 ms, pedestal slew
1.5 m/s and closed pedal. Resolutions are 64/128/256 ticks per audio frame.
The optional `--refined` selects 256/512/1024; it was not used for this receipt.

| Worst observed quantity across all cases/takes/windows | Result |
| --- | --- |
| Relative total balance defect | 1.16e-12 |
| Relative mechanical/electrical exchange defect | 3.60e-17 |
| Relative circuit balance defect | 1.19e-13 |
| Stationary-drive total energy growth | 0 |
| Voltage RMS refinement error | 0.0270% |
| Current RMS refinement error | 0.0532% |
| Vertical/horizontal velocity RMS refinement error | 0.0460% / 0.0122% |
| Outer coupling iterations | 3 |

The fixed gates are total/circuit balance below 1e-8, exchange and stationary
energy growth below 1e-10, monotone electrical heat, at least two hammer
contacts, zero-flux silence and nonzero loaded voltage, reaction and coil heat.
Resistive load heat must vanish for zero/open controls and be positive otherwise.
Every loaded case changes the fine mechanical velocity relative to zero flux.
Both lower resolutions must independently agree with the finest within 1% for
voltage, current and each pickup velocity in four fixed time windows.

Voltage is averaged into 4x samples then passed through the common 127-tap FIR.
All resolutions use that chain. This establishes selected temporal convergence,
not a complete aliasing bound. No thresholds or material parameters were retuned
after seeing the results.

Receipt: schema 1, 91847 bytes, SHA-256
`9fe3074c7f14e74e5b9fe984a9767d860226b4d12ba53f31b9c051713ec9bc3d`.

## Offline listening artifact

```text
cargo run --locked --release -p rf-73-lab -- electromechanical-render --output NEW.wav --gain 0.1
cargo run --locked --release -p rf-73-lab -- inspect NEW.wav
```

The renderer produces 1.2 seconds at 48 kHz: one default 75 mm tine and two
physical key gestures, 256 ticks per frame, the same filter and a 10 kohm load.
`--gain` is an explicit fixed FS/V conversion after filtering; it does not
affect the physical solve. It defaults to 1 and accepts 0.000001 through 1.
The float WAV has no normalization or limiter. Existing WAVs and companion
JSON receipts cannot be overwritten. Initial preload relaxation is retained.

The first unity-gain render failed headroom with peak 1.84613 FS while its
energy defect remained below 1.28e-12. Its WAV and failed receipt remain under
`renders/electromechanical-two-strikes.*`. The listening command uses 0.1 FS/V
and a separate output path; this is an explicit output-level correction.

The resulting `renders/electromechanical-two-strikes-listen.wav` passes: 57,600
finite float samples, peak 0.184613168 FS and RMS 0.016934764 FS after WAV
readback. Its physical energy defect is identical to the unity-gain run.
The 230458-byte WAV has SHA-256
`cb5bdf82044b2aa62a547f96f11a7db4236411dd8f773b08a3183865fb97d037`.
The companion JSON SHA-256 is
`590919888d42aa7997f9e5aded244dc9ac7132e08a221243527165c530cfc759`.

## Next block and limits

Static permanent-magnet attraction and magnetic stiffness, position-dependent
inductance, hysteresis, eddy-current identification, full keyboard wiring,
amplifier and loudspeaker are absent. Spatial flux and circuit constants still
need real-instrument identification. Numerical passivity does not certify tone.
The action remains a small-deflection reduction with provisional contact laws.

Next qualify longer gestures, pitch and timbre through this combined output,
then compare measured sources and reduce computational cost before integrating
the engine into the playable plugin. The current high-resolution reference is
offline; no realtime, host or human-listening result is claimed here.
