# Rhodes model research: September 2026

Research date: 2026-09-04. Baseline: RF-73 0.1.1, revision `f74e102`.

## Decision

The next implementation should extend the Rust laboratory to track inharmonic partials and their decay. Then fit a small, documented reference set before changing the production model. Numerical convergence has improved; acoustic calibration is still missing. This document proposes experiments, not validated replacement parameters.

## Papers and physical documentation

### 1. Gabrielli, Cantarini, Castellini and Squartini, JASA 2020

**The Rhodes electric piano: Analysis and simulation of the inharmonic overtones.** JASA 148(5), 3052–3064. DOI [10.1121/10.0002002](https://doi.org/10.1121/10.0002002). [Institutional record](https://iris.univpm.it/handle/11566/286030).

The abstract reports pickup spectra, perceptual analysis, scanning laser Doppler vibrometry of the assembled fork and its components, and magnetic intermodulation simulations. It also describes the keyboard distribution of important modes.

**Relevance:** our fixed beam ratios cannot establish which assembly resonances dominate a real attack. This is the highest-priority measurement paper to obtain in full. Only the institutional abstract was accessible; no modal tables or numerical results were extracted. The following experiments are our proposals, not reproductions of its results.

### 2. Florian Pfeifle, DAFx 2017

**Real-time Physical Model of a Wurlitzer and Rhodes Electric Piano.** [Conference PDF](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf), [conference mirror](https://dafx17.eca.ed.ac.uk/papers/DAFx17_paper_79.pdf).

Sections 4–5 describe observed nonplanar tine motion, a viscoelastic hammer law, a beam model with two polarizations and tuning-spring mass, and a geometric magnetic pickup model. This gives concrete alternatives to our single displacement coordinate and elastic contact. The Wurlitzer and Rhodes transducers must be distinguished when interpreting the paper.

**Relevance:** compare a two-coordinate pickup and dissipative contact independently. The paper does not validate our constants or prove that every proposed mechanism is perceptually necessary. Relevant physical sections were read; this study did not reproduce the implementation or benchmarks.

### 3. Antoine Falaize and Thomas Hélie, JSV 2017

**Passive simulation of the nonlinear port-Hamiltonian modeling of a Rhodes Piano.** Journal of Sound and Vibration 390, 289–309. DOI [10.1016/j.jsv.2016.11.008](https://doi.org/10.1016/j.jsv.2016.11.008). [Author demonstrations and code links](https://afalaize.github.io/posts/rhodes/).

The publisher abstract and author page describe a power-balanced hammer/beam/pickup system. The author demonstrates a hysteretic hammer, damped beam and pickup/RC response, including position-dependent sound examples and comparison plots against UVI measurements. A Python 2.7 simulation archive is linked.

**Relevance:** preserve energy accounting when adding material losses. Demo distances are example configurations, not universal factory settings. The full manuscript and code archive could not be inspected through their linked endpoints; code licensing, execution and reproduction remain unchecked. The project implementation will remain Rust.

### 4. Rhodes service manual, 1979

[Chapter 4: Dimensional Standards and Adjustments](https://www.fenderrhodes.com/org/manual/ch4.html), original manual transcribed by an independent archive.

The manual documents register-dependent hammer-tip hardness and height, escapement, strike line, and separate pickup alignment and distance adjustments. It notes instrument-date differences and player-dependent regulation.

**Relevance:** calibrate a specific instrument configuration. Smoothly scaling one hammer across all notes is only a provisional approximation. Record hardware key numbering explicitly; manual hammer numbers are not MIDI note numbers. These adjustment instructions do not directly provide elastic stiffness, damping or modal masses.

### 5. Greg Shear and Matthew Wright, NIME 2011; Shear thesis

**The Electromagnetically Sustained Rhodes Piano.** [NIME record and paper](https://www.nime.org/proc/nime2011_shear/index.html), pp. 14–17, DOI 10.5281/zenodo.1178161. [Shear's 2011 UCSB master's thesis](https://www.mat.ucsb.edu/Masters/GregShearMasters2011_12_5.pdf).

The conference abstract describes an augmented physical Rhodes with controlled electromagnetic sustain. It is a source on instrument experiments, not a ready-made synthesis engine. The thesis is also cited by the Phosphor implementation below for measured Q values. That table was not independently verified during this search; do not import those values from another project's comments as measurements.

## Projects worth inspecting

| Project | What was checked | Use for RF-73 |
| --- | --- | --- |
| [Phosphor](https://github.com/joshjetson/phosphor) | Rust Rhodes source introduction, modal constants and parameter definitions at `523f74a0227f7d3604a25ec0f1bc89732d4be11d`; repository metadata reports MIT | Concrete comparison for bounded modal storage, voicing controls and parameter provenance. Its six-resonator design includes a close partner mode and ideal beam overtones. This source inspection does not validate its acoustic accuracy. |
| [OpenWurli](https://github.com/hal0zer0/openwurli) | README, documented signal chain and GPL-3.0 declaration | Rust implementation and circuit/calibration workflow reference. Its Wurlitzer capacitive pickup is a different transducer. Author claims of physical accuracy were not independently tested. |
| [Falaize simulation](https://afalaize.github.io/posts/rhodes/) | Author description and links | Research reference for passive coupling; archive retrieval, licensing and numerical reproduction remain pending. |

Pinned Phosphor source: [rhodes.rs](https://github.com/joshjetson/phosphor/blob/523f74a0227f7d3604a25ec0f1bc89732d4be11d/crates/phosphor-dsp/src/rhodes.rs). No external project code was incorporated or executed.

## Available recordings and their limits

[jRhodes3d by Jeffrey Learman](https://github.com/sfzinstruments/jlearman.jRhodes3d) documents a 1977 Mark I Stage 73, up to five velocity layers and full-length samples. Although recorded from the harp connector, the distributed recordings have EQ and noise reduction; stereo variants add effects. This is a useful listening reference, not an untreated physical calibration set. Sampling is sparse, and layer labels do not measure hammer velocity.

The repository [license](https://github.com/sfzinstruments/jlearman.jRhodes3d/blob/master/LICENSE) declares CC BY-NC 4.0 for sample distribution and CC0 for control files/example clips. No audio was downloaded or bundled. Any future dataset manifest must retain source, processing and license information.

## Gaps in our model

These are conclusions from the local implementation, not claims made by the papers.

| Current approximation | Consequence to investigate | Proposed experiment |
| --- | --- | --- |
| Three modes with ratios 1, 6.267 and 17.55 | Real assembly attack peaks may be misplaced or absent | Track persistent peaks without imposing harmonic or beam ratios |
| One tine displacement coordinate | Cannot represent a two-plane trajectory | Compare scalar and two-coordinate flux models with controlled motion |
| Fixed modal strike/pickup weights | Excitation and observation geometry are conflated with gain | Fit multiple strike intensities with shared geometry and bounded weights |
| Elastic contact potential | No material loss during compression/restitution | Introduce one dissipative contact candidate and audit energy/contact separation |
| Provisional register-dependent losses | Decay could match one note by accident | Estimate per-partial early and late slopes across anchor notes |
| No measured assembly coupling | Cannot attribute beating or multiple decay rates physically | Add a partner mode only if repeatable data supports it |

## Implementation sequence

### A. Reference manifest and inharmonic analysis

Extend `rf-73-analysis` with local peak detection, cross-frame association, noise-floor qualification and per-track decay fits. Preserve harmonic summaries as separate outputs. Report frequency, amplitude, time span, fit interval and uncertainty/rejection reasons. A spectral peak alone is not proof of a mechanical mode: pickup mixing can create additional components.

Use deterministic synthetic cases with a noninteger partial, nearby tones, a short attack-only component and a decaying component entering noise. Report unresolved peaks when window resolution is insufficient; avoid manufacturing precision through zero padding. These tests validate measurement behavior, not Rhodes fidelity.

The reference manifest should record: source URL and revision/hash, permission/license, instrument model/year/key count, MIDI note and tuning, take/repetition, intensity label and how measured, pickup/hammer regulation if known, output tap/loading, processing, sample rate, and note-off/sustain markers. Unknowns remain explicit. Our existing input supports WAV; FLAC resources require a documented conversion before analysis.

### B. Fit a compact set, then hold out notes and intensities

Start with A3 (MIDI 57, 220 Hz) and A4 (69, 440 Hz), then add bass and treble anchors. Proposed acquisition: at least five strike intensities and three repetitions, long held notes plus separate release takes, fixed recording gain and no effects. Human intensity labels are not calibrated velocities.

Fit frequencies and decay first from qualified intervals. Then jointly refine excitation and pickup parameters across intensities, retaining uncertainty where different parameter combinations explain the same audio. Keep one intensity and neighboring notes out of fitting. Preserve absolute level comparisons alongside listening pairs with matched level.

### C. Compare physical alternatives separately

1. Replace provisional modal profiles with measured frequencies, losses and observation weights where evidence is available.
2. Compare pickup flux laws using identical mechanical trajectories. Sweep gap and offset independently and measure harmonic balance, intermodulation and aliasing against a higher-rate reference. Add the second motion coordinate only with a defined mechanical model.
3. Evaluate dissipative hammer contact with bounded iteration count, nonadhesive separation and energy balance. A published continuous-time law does not automatically preserve the passivity of our discrete solver.
4. Add assembly coupling or register-specific hammer families only when held-out residuals justify their parameters.

For an exponential amplitude envelope `A(t) = A0 exp(-t/tau)`, `T60 = ln(1000) tau`. For a lightly damped isolated mode, `Q` is approximately `pi f tau`. These are unit conversions, not a measured Rhodes decay profile. Do not confuse amplitude and energy time constants or assign a single Q to a visibly multistage envelope.

### D. Accept an audible improvement

Require a report with baseline/candidate/reference spectra, attack and decay errors, held-out cases, level-matched blind pairs, convergence results and CPU cost. Set acoustic tolerances after establishing measurement repeatability. Retain current passivity, MIDI and bounded-work requirements. Generate a new test package and run the documented `audition` workflow when an actual candidate version is ready.

## Still needed

- Full JASA modal study and any legitimately available supplementary measurements.
- Independent reading of the Shear thesis Q table and its measurement conditions.
- An untreated, documented recording set; the available sample library does not meet this criterion.
- Reproduction and license inspection of the author simulation before any code reuse.

No paper or project examined establishes perceptual equivalence for RF-73. The immediate deliverable is a better measurement path followed by a controlled physical change.
