# Numerical convergence and treble contact

Version 0.1.1, 2026-09-04. This experiment measures agreement between discretizations of our provisional model. It is independent of calibration against a real Rhodes.

## Finding and correction

At MIDI 100, normalized velocity 0.2 and 48 kHz, fixed 4x integration differed from a fixed 64x reference by 10.540% normalized RMS waveform error over the first 32 ms. Final resonator energy was 21.849% lower. This error was much larger than the A3 error and decreased with finer contact integration.

The highest provisional mode is approximately 46 kHz at MIDI 100. Implicit midpoint is passive but its phase accuracy degrades when a mode traverses a large angle in one step. Passivity alone did not guarantee accurate excitation during contact.

Production now prepares a contact subdivision count:

```text
contact_steps = next_power_of_two(max(1, ceil(highest_mode_omega * base_dt / 0.2)))
```

The count is capped at 16, sufficient for the supported keyboard and sample-rate range with the current mode ratios. Subdivision applies only to contact ticks. Free motion is still advanced by exact transition matrices; after separation inside a tick, precomputed fine free transitions finish the remaining interval. Pickup evaluation and the production FIR remain at 4x. No allocation, root-solver iteration growth, or runtime parameter-dependent unbounded loop is introduced. Each contact microstep retains the 40-iteration scalar force solve.

The 0.2-radian preparation threshold is an engineering choice supported by this regression set, not a perceptual tolerance. Future changes to modal ratios require reassessing the cap and error budget.

## Reproduce

```text
cargo run --locked --release -p rf-rhodes-lab -- converge --output renders/treble-convergence.json --note 100 --velocity 0.2 --sample-rate 48000 --seconds 0.25
```

Defaults are A3, velocity 0.7, 48 kHz and 250 ms. Every take holds the key throughout and uses the default physical profile. Supported output rates are 44.1/48/96/192 kHz, notes 28–100, velocities 0.01–1 and durations 50–1000 ms. Existing reports are never overwritten.

`comparisons` contains fixed 4/8/16/32x voices; `production` contains the actual prepared contact rule with a 4x pickup. Both are compared to a fixed 64x voice, without production subdivision in the reference. This keeps the coarse baseline visible after fixing production.

## Measurement method

All paths sample mechanical displacement and velocity at the same physical output-frame endpoints. Force impulse and positive energy increments are accumulated at each observed base tick. Production force is the average of its contact microstep forces; force maxima therefore depend on the reported base resolution. Separation time is the end of the first observed tick detecting separation, with uncertainty up to one base tick. Final mechanical energy is measured at the requested duration, before filter flushing.

Audio uses the same physical low-pass kernel at each internal rate: a symmetric Blackman-windowed sinc, cutoff `0.42 * output_sample_rate`, length `128 * substeps + 1`, normalized DC gain and 64 output samples of group delay. The experiment renders 64 extra output frames and removes this common delay, preserving the initial attack and the requested end time. Startup history is zero; no artificial silence replaces the final physical samples. This offline filter is longer than the production 127-tap FIR and allocates memory outside the audio plugin.

Reports contain raw and separately level-matched audio errors over the full interval and its first 32 ms, plus unfiltered displacement/velocity errors, energy error, contact impulse and separation. Relative mechanical errors use the reference norm; signed energy/impulse errors are candidate/reference minus one. No lag adjustment is allowed. Spectral distances use the analysis crate's documented windowing and floor; see [Analysis laboratory](ANALYSIS.md).

## Results

The local grid contains MIDI 28/45/57/81/100 at velocities 0.2/0.7/1.0, 48 kHz and 250 ms, plus MIDI 100 at velocities 0.2/1.0 and 44.1/96/192 kHz. All use the same provisional profile.

| Case | Fixed 4x attack NRMSE | Production attack NRMSE | Contact microsteps |
| --- | --- | --- | --- |
| A3, velocity 0.7, 48 kHz | 0.03654% | 0.03654% | 1 |
| MIDI 81, velocity 1.0, 48 kHz | 0.24514% | 0.01437% | 4 |
| MIDI 100, velocity 0.2, 48 kHz | 10.53953% | 0.12480% | 8 |
| MIDI 100, velocity 1.0, 48 kHz | 0.88198% | 0.01030% | 8 |

For the soft-treble case, final energy error falls from −21.849% to −0.274%. At 48 kHz, production contact resolution equals 32x output while the reference is 64x, so a visible residual remains. At 96 and 192 kHz the same soft-treble attack errors are approximately 0.156% and 0.164%; the reference itself is finer in absolute time at those rates. At 44.1 kHz production contact and reference both reach 64x, making that particular comparison much closer. Cross-rate values do not constitute an absolute error bound.

Grid reports are ignored files named `renders/refined-NOTE-VELOCITY-RATE.json`. Earlier fixed-only exploratory reports remain under `renders/convergence-*`. The final command can reproduce both baseline and production in one report.

## Remaining limits

A finite 64x reference is not an analytic solution. The 32x row exposes one reference-refinement residual, but no Richardson bound or formal convergence order is claimed. The common low-pass comparison combines contact error, sampling error and any residual nonlinear aliasing; it does not isolate each mechanism or qualify the shorter production FIR. Filters, numerical agreement and passivity do not demonstrate Rhodes timbral fidelity.

The grid covers isolated default-profile strikes. Dense retriggers, extreme valid profiles, contact threshold boundaries and long-duration behavior still require broader measurement. The additional contact work must be included in host timing; higher precision has a cost even though free vibration remains at the base rate.
