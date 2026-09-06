# Development roadmap

All application code, analysis tools and tests are Rust. Documentation, identifiers and user-facing strings are English.

## Implemented in 0.1.0

- Independent Cargo workspace and locked toolchain.
- Safe Rust DSP core with bounded implicit hammer contact and free modal decay.
- Provisional nonlinear pickup, 4x processing and FIR decimation.
- 73-key research engine with sustain, channel ownership and retrigger continuity.
- Rust WAV renderer, physical CSV probes, signal reports and native stress runner.
- RackForge SDK adapter with MIDI 1.0/2.0, gain, program and versioned state.
- Native and WASM build targets; development package metadata.
- Numerical, MIDI, block-invariance, input-validation and WAV integrity tests.

## Implemented in 0.1.1

- A reproducible Rust convergence experiment with fixed 4/8/16/32x paths and a 64x reference.
- A common offline filter, equal physical time sampling, contact diagnostics and attack comparisons.
- Contact-only subdivision that reduces the measured soft-treble integration error without raising the continuous pickup processing rate.
- Regression coverage for refined passivity, unchanged A3 behavior, treble accuracy, filter integrity and CLI reports.

## Implemented in 0.1.2

- Three real-time pickup paths sharing one mechanical engine and fixed performance-level compensation.
- Continuous FIR histories and interruptible 20 ms A/B fades, preserving held notes and pedal.
- Rust declarative program editor, host control pages, live preview and complete configuration snapshots.
- A separate Rust WebAssembly PLAY panel with A/B selectors, gain, host synchronization and responsive day/stage styling; browser bindings are generated.
- Bounded custom program editing, schema-1 processor migration and explicit host-version compatibility limits.
- A 0.100x starting gain and documented sample-headroom limits, with no limiter.
- A native stress mode for simultaneous pickup processing and repeated interrupted fades.

See [Pickup Lab UI](PICKUP-LAB-UI.md). Human listening feedback now guides the next modeling experiments; final visual design remains later work.

## Physical-model ambition and next implementation gate

Implemented first gate: the offline [passive common-support assembly](COUPLED-ASSEMBLY.md)
has reciprocal tine/tonebar coupling, nonlinear hammer contact and explicit
stored/dissipated/escaped energy accounting. Analytic mechanics tests and the
24-case, 120-take audit pass. This is an uncalibrated lowest-order reduction.
The subsequent [free-motion refinement](ASSEMBLY-REFINEMENT.md) resolves the
measured 4x treble phase error using a prepared exponential transition and
contact-only subdivision. Its 48-case, 288-take audit passes, including an
independent comparison between two contact resolutions. Measured higher-mode
and support identification, pickup comparison and polyphonic timing remain
the next gates before plugin integration.

The [tine modal preparation](TINE-MODES.md) now derives six fixed-root bending
modes, port weights and moving-root inertia from a uniform beam plus a movable
tuning point mass. Analytical and mesh-convergence checks pass. Its full-matrix
time-domain connection is implemented in the following milestone; identification
of the tonebar/root geometry and measured higher-frequency behavior remains open.
The beam geometry itself remains illustrative.

Implemented next integration gate: the [nine-coordinate modal assembly](MODAL-ASSEMBLY.md)
connects all six tine modes, root translation/rotation and one provisional tonebar
coordinate using the full reciprocal inertia. Spatial hammer/damper ports and
independent energy accounting pass an 84-take audit. The solver remains offline
and needs further performance work, pickup/filter comparison and physical
parameter identification before it can replace the plugin engine. The
[prepared free-motion optimization](MODAL-PERFORMANCE.md) removes repeated basis
changes and measures synthetic 1/8/32/73-voice block workloads. The subsequent
[bounded contact solve](MODAL-CONTACT-SOLVER.md) accelerates the existing scalar
equation with a bracketed Newton method and bisection fallback, while preserving
the uniform reference. Contact bursts and WASM/host deadlines remain open gates.
The [dissipative hammer experiment](DISSIPATIVE-HAMMER.md) now couples a
rate-dependent loss law to all nine coordinates, with explicit nonadhesive
unloading and independent contact heat. Analytic rigid-wall restitution and
refined/uniform comparisons separate material sensitivity from integration error.
The loss coefficient remains provisional and defaults to zero; internal material
relaxation and measured calibration are still missing.
The [material-memory coupon](HAMMER-MEMORY.md) now implements and validates an
internal Maxwell deformation, analytic relaxation and positive material heat
under prescribed loading. The subsequent [two-mass memory hammer](MEMORY-HAMMER.md)
connects that bilateral reaction to core/tip inertia and a nonadhesive fixed
surface, preserving memory through separation, free recovery and impulse-driven
reimpact. The [stateful modal coupling](MEMORY-MODAL-COUPLING.md) now connects
that hammer to all nine moving structural coordinates with separate port-work
checks, persistent recovery and damper transitions. Improving temporal accuracy
per unit cost and identifying material parameters remain gates before plugin integration.
The [stateful diagnostic optimization](MEMORY-MODAL-PERFORMANCE.md) now removes
repeated structural-energy evaluation and measures native kernel cost at the
same fixed resolution. Reducing required integration steps remains open.
The [free-recovery primitive](MEMORY-FREE-MOTION.md) now attempts longer nonlinear
hammer intervals with a whole-interval clearance certificate and local state/work
checks. It is validated against the fixed-wall reference. The
[moving modal integration](MEMORY-MODAL-FREE.md) now adds a full-mass port-speed
bound, prepared structural propagation and a coupled fine-reference audit.
The [direct material root](MEMORY-MATERIAL-SOLVE.md) now avoids iterative inner
solves on checked same-sign deformation branches. The
[adaptive contact experiment](MEMORY-ADAPTIVE-CONTACT.md) adds prepared dyadic
contact steps with continuous compression bounds and coarse/fine state checks.
Its stricter local tolerance passes the retained 12-case protocol after a looser
trial failed trajectory accuracy. Qualifying longer trajectories, wider profiles
and runtime cost remains necessary before realtime integration.
The [contact scheduling experiment](MEMORY-CONTACT-SCHEDULING.md) now avoids
short contact trials and defers retries while continuing every original fine
tick. It retains the strict tolerance and separate previous-controller controls.
The [contact reaction reuse](MEMORY-CONTACT-FORCE-REUSE.md) removes a redundant
material solve at converged normal forces without changing integration steps
or physical tolerances. Contact integration cost remains an open gate.
The [contact resolution diagnostic](MEMORY-CONTACT-RESOLUTION.md) now separates
the estimator's squared-error terms without committing trial motion. Its sampled
state-limited steps mostly have surface-contact or structural contributions and
approximately cubic local error growth. The next integrator experiment should
target coupled temporal accuracy while retaining independent heat/work checks.
The [fourth-order fixed-wall contact experiment](MEMORY-CONTACT-RK4.md) now
integrates the same two-mass/material law with checked RK4 contact intervals,
independent heat/work/impulse quadratures and original fine ticks at boundaries.
Its 24-case comparison passes without relaxing global gates. Native timing and
the full moving modal connection remain the next gates before adoption.
The [coupled RK4 experiment](MEMORY-MODAL-RK4.md) now advances the hammer and
all nine structural coordinates in shared stages, with independent reciprocal
port work and damping quadratures. Its 12-case audit passes; native medians are
3.84–6.33 times faster than the previous economical controller in four profiles.
It still costs 5.6–8.1 seconds per simulated second, and its maximum velocity
difference from the fine reference is 0.2612%. Longer/finer reference validation,
further cost reduction and polyphonic host qualification remain open.

The target remains a sophisticated Rhodes-specific physical model, comparable in development depth to RF Concert Grand. The laboratory UI is a measurement and audition tool, not a declaration that the sound engine is finished. Numerical stability and passing tests do not establish realism.

The current mechanical work builds on the explicit tine/tonebar assembly with mounting compliance. Determine which measured modes belong to which assembly motion; introduce orthogonal motion and coupling where the evidence supports it. Preserve the existing plugin engine as an A/B baseline. Fit frequency, decay and coupling against reference observations, and qualify numerical accuracy and performance before real-time integration.

Subsequent work covers hammer-tip material/history and strike geometry, action/repetition, geometry-based magnetic conversion, continuous dampers and release response. Extend identified behavior across the keyboard and velocities before adding optional electronics or spending time on final visual styling. A complete physical-field simulation of every part is not required for real-time fidelity; every reduction must have a documented assumption and a measurable validity range. The current five processed G3 layers do not identify all these physical parameters.

## Next milestone: a calibrated A3

Implemented research tooling after 0.1.1: independent spectral peak tracking, local/global background estimates, resolution and capacity flags, contiguous association, and qualified per-track decay with explicit rejection reasons. Single-note analysis now uses schema 2. The mechanical profile remains unchanged in 0.1.2; its plugin now offers the three matched pickup paths.

Independent tracking now supports 32/128/512/1024 ms observations with reported sample counts and FFT grids. Long observations use the full requested duration, including at 192 kHz. The default remains 128 ms, and harmonic summaries retain their original window.

The [September research review](RESEARCH-2026-09.md) specifies the next experiment: document reference metadata and run an A3/A4 pilot with held-out intensities using the new independent tracking. A4 provides a second anchor; neither note is calibrated yet.

The measurement foundation is implemented: external WAV input, multi-resolution spectra, bounded harmonic searches, RMS envelopes, qualified decay estimates, and raw/level-matched comparison reports. Known synthetic signals validate these estimators; they do not calibrate the instrument. See [Analysis laboratory](ANALYSIS.md).

`compare-partials` now pairs simultaneous detections across explicit equal-duration regions, preserving raw gain and reporting unmatched/ambiguous observations. Decay differences require qualified fits over identical fully paired intervals. See [Partial comparison](PARTIAL-COMPARISON.md).

`sweep-pickup` now evaluates up to 25 gap/offset combinations against an explicit held-note region, using one global RMS gain and multiple spectral windows. It preserves raw errors, reports near ties and leaves the instrument profile unchanged. Synthetic references verify recovery of a known grid geometry at two intensities; measured calibration and held-out validation remain outstanding. See [Pickup sweep](PICKUP-SWEEP.md).

`fit-pickup-set` now selects shared geometry and gain from multiple fitting takes, then evaluates only that frozen choice on held-out note/velocity pairs. Its strict manifest records provenance and explicit sustain boundaries. The [reference-set workflow](PICKUP-SET.md) is validated synthetically; the [sample-bank review](REFERENCE-BANKS.md) identifies commercial candidates and a small processed real-instrument pilot.

The [G3 residual pilot](G3-RESIDUAL-PILOT.md) now compares all five acquired source layers against three model velocities and two pickup geometries. A closer pickup reduces a strong-probe H3 deficit but selects the minimum supported gap and leaves substantial residuals. `compare-tone` provides explicit attack/body diagnostics without inferred velocities or decay fits; the production profile remains unchanged. The next isolated physical experiment targets pickup transfer shape and excitation scale, including nonlinear aliasing.

The [isolated pickup experiment](PICKUP-TRANSFER.md) compares the current law with a more localized point-pole field proxy under identical periodic motion. It reports raw sensitivity, harmonic balance and internal sampling residuals separately. [Mechanical pickup pairs](PICKUP-PAIR.md) now extend this to the production trajectory and FIR, preserving baseline WAVs exactly. The close-gap strong probe further reduces the G3 third-harmonic deficit but leaves substantial upper-harmonic/body residuals and increased output level. Cross-register convergence and gain/headroom evaluation remain necessary before a sound-profile release.

The [pickup convergence matrix](PICKUP-CONVERGENCE.md) now covers three register anchors, two intensities and two geometries, with treble checks at all output rates and 256x reference confirmations. The tested frozen-trajectory residuals are much smaller than the remaining contact/trajectory differences. Candidate gain/headroom and level-matched listening comparisons are the next release gates; no whole-keyboard or perceptual qualification is claimed.

The [listening study](PICKUP-LISTENING.md) now supplies three versions of one performance with fixed global RMS matching and controlled sample peaks. The full keyboard and repeated chords have been measured at 44.1/192 kHz. Listening artifacts are ready; human assessment remains outstanding. Version 0.1.2 implements a documented starting gain and fixed compensation policy. Large raw stress peaks in both the current and candidate models prevent treating the listening gain as a universal output bound.

1. Obtain dry recordings from a documented instrument at multiple intensities.
2. Evaluate independent tracking across the available observation lengths on those recordings, refine background qualification, and distinguish observed spectral peaks from identified mechanical modes.
3. Extend the current convergence measurements to extreme profiles, retriggers and isolated nonlinear aliasing; establish explicit error budgets.
4. Identify modal frequencies, weights, losses and pickup geometry jointly, keeping a held-out validation set.
5. Add hammer hysteresis and assembly modes where measured residuals justify them.
6. Produce matched-level blind listening pairs and an error report.

## Later milestones

| Stage | Deliverable | Exit criterion |
| --- | --- | --- |
| Registers | Calibrated bass, mid and treble anchors | Unfitted notes interpolate acceptably |
| Full action | Dampers, release dynamics and regulated repetition | Recorded gestures reproduce plausibly |
| Electronics | Documented Stage/Suitcase circuit profiles | Stage-by-stage bypass comparisons |
| Product | Branded package and musical controls | Validated install and saved-session roundtrip |
| Qualification | Native, browser, Android and Pi measurements | Device-specific deadlines and listening criteria |

Initial performance target: keep the plugin below half the block deadline on each target. A 128-frame block at 48 kHz lasts 2.667 ms; 1.333 ms is the provisional plugin budget. A short desktop run does not qualify a stage instrument or establish mobile performance.

The current sound is an audible research result, not a claim of high-fidelity Rhodes reproduction. Hitting numerical tolerances is necessary but does not establish perceptual equivalence.
