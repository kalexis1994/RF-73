# Physical model ledger

Status: research prototype 0.1.1. All constants are provisional unless explicitly marked as measured. No real-instrument calibration has been performed.

## Signal path

```text
MIDI velocity -> hammer initial velocity <-> unilateral contact <-> modal resonator
                                                                  ^ damper
                                                                  v
                         output <- FIR decimation <- magnetic pickup per key
```

The intended reference is a Mark I Stage 73, direct output. The instrument, year and regulation remain unselected. A faithful assembly model must account for the tonebar as well as the tine; the current three-mode approximation does not yet identify those contributions separately. [Rhodes service manual, chapter 1](https://www.fenderrhodes.com/org/manual/ch1.html).

## Implemented equations

Each modal coordinate satisfies:

```text
m_i q_i'' + 2 m_i gamma_i q_i' + m_i omega_i^2 q_i = b_i F
hammer_mass x'' = -F
delta = x - sum(b_i q_i)
V(delta) = stiffness * max(delta, 0)^3 / 3
```

During contact, implicit midpoint integrates the linear mechanical coordinates. A discrete gradient of the contact potential provides the common force:

```text
F = (V(delta_new) - V(delta_old)) / (delta_new - delta_old)
delta_new = delta_free - compliance * F
```

The derivative limit is used algebraically when compressions coincide. The force equation is monotone with positive compliance. Forty bounded bisection iterations solve it inside a nonnegative bracket; no unbounded Newton loop is used. This provides an energy-consistent elastic contact with linear modal dissipation. The implementation tests nonincreasing total energy, including hammer and contact energy, across rates, registers and velocities. On separation, the detached hammer's remaining energy leaves the simulated subsystem.

This follows the energy-accounting principle of Falaize and Hélie, but is our reduced implementation, not their complete model. Their publication develops passive discretization of a nonlinear hammer, modal beam and pickup. [JSV 2017](https://www.sciencedirect.com/science/article/pii/S0022460X16306320).

After separation, exact damped-oscillator transition matrices advance each free mode. Matrices are prepared for lifted and applied dampers. Retriggering retains all resonator positions and velocities, then introduces a new hammer strike. It intentionally replaces an unfinished hammer contact rather than simulating the entire key/action mechanism.

Version 0.1.1 subdivides contact ticks when the fastest mode would advance by more than 0.2 radians per midpoint step. Preparation chooses a power of two, bounded to 1–16 microsteps across supported notes and rates. If separation occurs inside a tick, exact free transitions complete only the remaining microsteps. The reported contact force is the average across the base tick, so multiplying it by the base duration preserves impulse. Pickup evaluation and the production decimator remain at 4x; unconstrained free motion still uses its exact base-rate transition. See [Numerical convergence](CONVERGENCE.md) for the observed treble error and correction.

## Provisional constants and simplifications

| Quantity | Initial choice | Evidence status |
| --- | --- | --- |
| Hammer mass at A3 | 4 g | Assumed |
| Modal mass at A3 | 1.5 g per mode | Assumed effective mass |
| Contact stiffness | 4e10 N/m^2 | Assumed, quadratic force law |
| Maximum hammer speed | 0.8 m/s | Assumed |
| Speed mapping | maximum speed * normalized velocity^1.4 | Empirical design choice |
| Modal frequency ratios | 1, 6.267, 17.55 | Ideal uniform cantilever approximation |
| A3 modal T60 | 5 s, 160 ms, 55 ms | Assumed |
| Pickup gap / offset | 1.5 / 0.5 mm | Initial study coordinates, not measured |
| Extra damper decay rate | 55/s | Assumed binary damper |
| Pickup/internal base rate | 4x output rate | Fixed research configuration |
| Contact subdivision | 1–16 microsteps per base tick | Prepared from highest modal frequency; numerical accuracy choice |

Mass and decay scale smoothly with pitch; this is a convenient initial profile, not a Rhodes scale measurement. No stiffness, damping or geometry is taken from Concert Grand's strings or soundboard.

Current omissions: measured tonebar/mount modes, two-polarization coupling, hammer-tip hysteresis, actual neoprene zones, tuning-spring geometry, nonlinear large-deflection behavior, escapement, partial pedal and release-velocity response. Pfeifle's measurements motivate the later contact/polarization/pickup work. [DAFx 2017](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

## Magnetic conversion

The initial smooth flux-linkage surrogate is:

```text
z = (pickup_offset + tip_displacement) / pickup_gap
Phi = 1 / sqrt(1 + z^2)
voltage = -dPhi/dt
```

The analytical gradient is multiplied by tip velocity, with an arbitrary electrical scale factor. A positive validated gap removes singularities. This captures geometry-dependent nonlinear transduction, but is not a measured magnetic-field map. Falaize's demonstrations illustrate the significance of pickup displacement. [Author demonstrations](https://afalaize.github.io/posts/rhodes/).

Conversion occurs separately for every key, before summation. This keeps pickup intermodulation within that key; a nonlinear amplifier across the mix would be a separate circuit. Gabrielli and colleagues specifically analyze attack modes and their pickup intermodulation. Their abstract was consulted; full modal tables remain to be obtained. [JASA 2020](https://iris.univpm.it/handle/11566/286030).

## Antialiasing, output and numerical limits

A 127-tap Blackman-windowed sinc filters the summed 4x-rate signal before decimation. Cutoff is 0.105 cycles/internal sample; group delay is 63 internal samples, or 15.75 output samples. The filter's measured stopband response has unit tests. This does not prove that all nonlinear aliasing is inaudible: aliases generated above the internal Nyquist frequency require a higher-rate comparison and cannot be removed afterward.

Output is mono, duplicated for a stereo host. Gain changes use a 5 ms smoother. There is no limiter, normalizer, amplifier, tremolo, cabinet or reverb. Dense chords can exceed full scale; retain headroom when monitoring. The laboratory writes unclipped float WAV files and reports their peak.

The engine supports 44.1–192 kHz. Profiles are immutable during rendering and range-validated before preparation. Rendering has fixed-size storage and no explicit heap allocation, locks, I/O or logging. Nonfinite mixed output resets the engine to silence and increments a fault counter. Tests assert zero faults so recovery cannot conceal a regression.

## MIDI semantics

73 mechanical keys cover MIDI 28–100. One resonator exists per pitch, shared by MIDI channels, with channel-specific key and sustain ownership. Same-channel repeated Note On restrikes the physical key rather than stacking voices. The last strike channel owns a released tail. CC64 is binary sustain; CC123 releases keys subject to sustain, CC120 kills that channel's unshared sound, and CC121 clears its pedal. Global reset clears FIR history too; channel panic can leave the short FIR tail.

Events are sample-positioned. Equal-time order is parameter events, MIDI 1.0 events, then MIDI 2.0 events, preserving order within each list. Seven-bit-origin wide events recover their original velocity; native wide notes use 16-bit resolution. Release velocity, pitch bend and expression beyond sustain are not implemented yet.

Saved plugin state contains a magic header, schema version and output gain. It represents instrument settings, not a recording of currently vibrating keys. Physical profiles are compiled into this research version and must acquire their own versioned state before becoming user-editable plugin controls.
