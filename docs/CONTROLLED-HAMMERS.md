# Controlled single-strike hammer comparison

`compare-modal-hammers` compares three existing hammer laws on the same
nine-coordinate tine/tonebar/support assembly. It implements the second stage
of the [direction review](RESEARCH-DIRECTION-2026-09-06.md): measure what each
candidate changes before fitting its parameters or choosing a production model.

```text
cargo run --locked --release -p rf-73-lab -- compare-modal-hammers --output renders/hammer-study.wav
```

The command writes the memory candidate to `hammer-study.wav`, the elastic
candidate to `hammer-study-elastic.wav`, the rate-dependent candidate to
`hammer-study-rate.wav`, and the complete receipt to `hammer-study.json`.
Existing destinations are checked before simulation and created exclusively.
All three WAVs require all candidates to qualify. A failed qualification retains
the report and writes no WAVs. A filesystem failure during writing is reported;
multi-file output is not a filesystem transaction.

The command accepts the same bounded options as
[`render-memory-modal`](MEMORY-MODAL-AUDIO.md): 0.25..3 seconds, at least 0.1
seconds before and after damper engagement, 0.2..0.8 m/s launch speed, 75 or
120 mm tine length, and explicit fixed gain. Defaults are 1.5 seconds, damper
engagement at 0.9 seconds, 0.4 m/s, 75 mm and gain 0.084. No MIDI pitch or
velocity is inferred.

## Controlled quantities and remaining confounds

All candidates start at zero gap with 4 g total launch mass, equal velocity
and kinetic energy. They use the same beam geometry, six tine modes, moving-root
inertia, support and tonebar parameters, structural losses, strike/pickup
locations and damper event. The program checks identical structural inertia
and guards the declared hammer defaults against silent parameter changes.

| Candidate | Contact / material | Action boundary |
| --- | --- | --- |
| Elastic | One 4 g mass, quadratic compression force, coefficient 4e10 N/m² | Hammer exits at first separation; outgoing kinetic energy enters the escaped ledger |
| Rate-dependent | Same mass/coefficient, projected nonadhesive Hunt-Crossley-type loss, beta 2 s/m | Same exit boundary |
| Memory | 3.8 g core + 0.2 g tip, 1e12 N/m² surface, existing nonlinear equilibrium/Maxwell material with 1 ms relaxation | Both masses and material remain active; no subsequent external impulses |

All coefficients remain provisional. Equal launch energy does not equalize
contact compliance, contact history or output level. These laws have not been
fitted to the same measured target, so this is not a fair contest of achievable
Rhodes fidelity. The action boundaries also differ. In particular, the memory
model can make several contacts during one launch. Its first separation is not
the end of the complete impact sequence. The report retains first and last
observed separated states, total impulse, force-positive episode count and
summed force-positive interval duration. Episodes and durations describe the
accepted numerical intervals, not exact root-located event times.

First-contact observables include elapsed time from launch, impulse, peak
interval-mean force, outgoing hammer center-of-mass velocity, structural stored
energy, structural heat, material heat and remaining/exported hammer energy.
For the simple hammer, outgoing velocity is recovered from launch momentum
minus integrated contact impulse and checked against exported kinetic energy.
It is not a coefficient of restitution relative to the moving tine. The memory
report separately retains its actual mass-weighted velocity and energy/work
ledgers. No full single-mass hammer trajectory is invented from hidden state.

## Numerical qualification and output measurements

The memory paths reuse the qualified offline renderer, including its finer
base grid, tighter caps and whole/2 ms mechanical checks. Single-mass paths
use uniform implicit midpoint at 256 and 512 steps per 48 kHz output frame.
Their contact heat and escaped hammer energy remain explicitly accounted for.
Balance error must remain below 1e-8 of launch energy; observed positive energy
increments, including exported energy, must stay below 1e-10. Heat must be
finite and nondecreasing, force finite and nonnegative, and separation observed.
These checks run every single-mass contact step and every 64x observation after
escape; they do not claim a maximum over unobserved free integration steps.

Each 2 ms single-mass section compares mass-weighted structural velocity at 1%
of launch scale, pickup velocity at 1% relative RMSE, and output-frame mean force
at 2%. First-contact observables also receive a 2% refinement check. A zero
reference requires zero difference. These are comparisons between finite
resolutions, not exact-solution bounds or evidence of physical calibration.

For every model, identical-trajectory 16x/32x pickup observations are compared
with 64x through the same physical Blackman FIR kernel. A second 64x render
checks temporal refinement. The common scalar magnetic pickup, FIR history,
15.75-sample delay and fixed gain match the first WAV experiment. No alignment,
normalization, clipping or candidate-specific gain is applied. Whole-record,
attack, body and release audio errors must each stay below 1%. Output requires
at least 1 dB headroom and non-negligible RMS. Checks use f64; files contain f32.

Cross-model waveform differences deliberately have no pass/fail threshold.
They preserve the named reference and section RMS values. Each output also
reports a 10 ms RMS envelope and four spectral energy bands from a direct DFT
of its first 32 ms, with a periodic Hann window and 31.25 Hz bin spacing.
Bands span 0–1, 1–5, 5–20 and 20–24 kHz, with each shared edge assigned to
the upper band. Their sum is windowed mean square; it is not unwindowed power,
a fitted modal decay, harmonic balance, or a perceptual distance. The common
FIR delay remains inside the attack window.

## Selected receipts and results

Date: 2026-09-06. All nine 1.5-second WAVs pass: 75 mm tine, launch speeds
0.2/0.4/0.8 m/s, three models at each speed, common gain 0.084 and damper at
0.9 seconds. The [tracked summary](../references/controlled-hammers-validation.json)
contains configurations, source/file hashes, both integration reports, contact
observables, audio metrics and independent WAV inspections. The complete
[medium-strike receipt](../references/controlled-hammers-75mm-medium.json)
retains every mechanical section. Other full reports and all audio remain in
the ignored `renders` directory.

For the medium strike, the measured differences are:

| Candidate | First separation | Last observed separation | Total impulse | Peak interval-mean force | Output RMS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Memory | 20.355 µs | 828.964 µs | 0.00261757 N s | 17.680 N | 0.0328109 |
| Elastic | 844.971 µs | Same event; hammer exits | 0.00261018 N s | 8.316 N | 0.0327909 |
| Rate-dependent | 840.983 µs | Same event; hammer exits | 0.00260104 N s | 6.396 N | 0.0326214 |

The memory candidate has 13, 22 and 24 force-positive episodes for soft,
medium and strong launches; both integration paths agree on these counts.
This is internal recontact during a single launch, not repeated key excitation.
Its total impulse is close to the alternatives despite substantially larger
short-interval force peaks. This is a model result to investigate against
observations, not evidence that a physical Rhodes has this contact pattern.

At medium launch, memory versus elastic gives 13.671% relative waveform RMSE
in the first 32 ms, 0.333% in the body and 0.240% after damper engagement.
Memory versus rate-dependent gives 9.713%, 0.642% and 0.533%, respectively.
These raw waveform differences point to the attack as the most distinct region
in this experiment. They do not quantify audibility or fidelity.

Maximum within-model audio refinement RMSE is 2.953e-5; maximum frozen-sampling
RMSE is 1.880e-7. Maximum relative balance residual is 2.807e-11 for the simple
models and 8.005e-11 across the memory energy/work ledgers. The largest WAV peak
is -1.734 dBFS, from the strong elastic strike. Every file is 48 kHz, mono float,
72000 frames, finite, and independently inspected. The three memory WAVs are
byte-identical to the earlier previews after adding contact instrumentation.

All 201 workspace tests, strict Clippy, formatting and release lab compilation
pass. New regressions check spectral power/gain accounting and protection of
all four destinations. A gain-10 negative control retains a failed report and
produces no WAVs. CI now includes a short three-model render; remote CI was
not run. No listening or host test was performed.

Medium-intensity local files:
[Memory](../renders/hammer-comparison-75mm-medium.wav),
[elastic](../renders/hammer-comparison-75mm-medium-elastic.wav),
[rate-dependent](../renders/hammer-comparison-75mm-medium-rate.wav).
Replace `medium` with `soft` or `strong` for the other measured intensities.

## Interpretation and next step

Listen and inspect raw contact/energy differences before adjusting coefficients.
The memory candidate must demonstrate useful behavior against shared reference
observations before its additional state is justified. A subsequent fit must
declare training notes/intensities, parameter bounds, shared gain and held-out
validation. Identifying the resonator and separating processed reference audio
from physical measurements remain the next calibration tasks.

This experiment does not resolve the earlier repeated-excitation failure,
qualify complete action mechanics, select a winner, or update the plugin engine.
No host, audio device or GUI is opened.
