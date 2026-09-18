# RF-Tines

[Project naming and compatibility](docs/RENAMING.md).

A Rust physical-model electric piano for RackForge.

The first working prototype includes a nonlinear hammer-contact solver, a three-mode resonator per key, a geometry-dependent magnetic pickup, sustain and sample-accurate MIDI. It renders audio offline and compiles to a portable RackForge WASM plugin.

Version 0.2.0 packages the playable instrument with its RF-Tines catalog
artwork, five era-inspired factory programs, responsive PLAY and CONFIG
surfaces, local program saving, portable `.rftines` files and channel-aware
MIDI pitch bend. It also carries the [rename to RF-Tines](docs/RENAMING.md),
which changes the plugin identity and the portable program format without
compatibility shims. The physical profile remains provisional while the
reference fit continues.

The offline [coupled assembly experiment](docs/COUPLED-ASSEMBLY.md) now models a tine, tonebar and compliant common support with reciprocal forces, nonlinear hammer contact and a complete energy ledger. Its parameters remain provisional; high-resolution validation precedes plugin integration.

[Prepared free motion and contact refinement](docs/ASSEMBLY-REFINEMENT.md) now reduce its measured treble integration error while retaining the independent energy ledger. This remains an offline candidate awaiting higher-mode identification and calibration.

[Geometry-derived tine modes](docs/TINE-MODES.md) now prepare six bending modes,
a movable tuning mass, spatial hammer/pickup weights and reciprocal inertia for
root translation and rotation. This structural basis is validated independently
and is used by the coupled solver described below.

The [nine-coordinate assembly](docs/MODAL-ASSEMBLY.md) now connects that structural
basis to nonlinear hammer contact and a spatial damper. Its time-domain energy
and convergence audits pass; it remains an offline experiment pending calibration
and polyphonic performance work.

The [moving felt damper](docs/MOVING-FELT-DAMPER.md) adds a massive elastic arm,
unilateral felt contact and continuous prescribed key/pedal actuation to the
experimental assembly. A refined 24-case release/recontact audit passes with
independent energy and actuator-work checks. It remains outside the playable
plugin, pending material calibration and realtime integration.

The [persistent action and repetition](docs/ACTION-REPETITION.md) experiment
connects a moving pedestal, persistent hammer, reciprocal bridle and felt arm.
Four interfaces are solved jointly with independent drive-work and heat ledgers;
repeated strikes preserve all mechanical state. Its reference matrix passes
16 cases and 48 takes; earlier failed studies remain recorded. This is an
offline action reduction with provisional geometry and materials.

The [two-plane action](docs/TWO-PLANE-ACTION.md) now adds a second transverse
component of the tine, reciprocal anisotropic support/tonebar coupling and
spatial hammer/felt contact normals. Its eight-case audit passes with independent
work ledgers for both planes. The symmetric control reproduces planar motion;
material calibration and large-deflection mechanics remain open.

The [reciprocal pickup and circuit](docs/ELECTROMECHANICAL.md) now connect both
motion components to spatial flux, coil current and a passive electrical load.
Current reaction feeds back into the joint action solve. Eight cases and 24
takes pass energy, load-control and output-refinement checks; a short offline
WAV renderer makes the combined model audible. Field/circuit calibration and
realtime plugin integration remain open.

The [loaded spring-tuning pilot](docs/LOADED-SPRING-TUNING.md) now moves the
tuning mass on a fixed tine while tracking the physical mode in both planes.
Four long-gesture takes pass energy and output-refinement gates; the tuned
audio also passes the frozen G3 pitch target. A before/after WAV pair preserves
changes in nonharmonic structure and output level for listening.

The [loaded source baseline](docs/LOADED-SOURCE-BASELINE.md) now compares the
combined model with all five pinned G3 layers using native-rate, pitch-aware
spectral bands and relative level trajectories. Contact feasibility keeps a
non-striking gesture separate from a soft note; source processing and initial
preload remain explicit limitations before physical parameter calibration.

[Stationary initialization](docs/STATIONARY-REST.md) now prepares the coupled
mechanism in static contact equilibrium while retaining preload energy. Four
cold/rest controls and six source-comparison takes pass voltage, displacement,
energy and refinement checks. The remaining fast sustain decay is explicit;
the source comparison starts from rest using `--at-rest`.

The [loaded loss budget](docs/LOADED-LOSS-BUDGET.md) separates twelve mechanical
and electrical heat channels and compares controlled support, tine, tonebar,
damper and load changes. This provides a physical diagnostic before fitting
the remaining sustain mismatch; the playable plugin is unchanged.

The [conditional sustain calibration](docs/LOADED-LOSS-CALIBRATION.md) selects
first-mode tine and support losses using the training layers, then checks
reserved layers, numerical refinement and other strike speeds. Unqualified
frequency observations and spectral mismatch remain explicit.

The [loaded voicing study](docs/LOADED-VOICING.md) compares seven controlled
hammer-point, contact-stiffness and pickup configurations. It measures impact
force, impulse and duration alongside source spectra and numerical refinement,
with no automatic preset selection.

The [shared-voicing dynamics study](docs/LOADED-DYNAMICS.md) tests one physical
configuration across drive speeds. Training layers choose between two pickup
offsets and two explicit layer-order hypotheses; reserved layers then use fixed
intermediate speeds, with no refit to their spectra.

The [soft-strike threshold study](docs/LOADED-STRIKE-THRESHOLD.md) separates
non-striking motion from impact and tests action/escapement/contact controls.
Independent hammer and contact-port work ledgers identify energy transfer
through the soft-drive region.

The [bridle/damper study](docs/LOADED-BRIDLE.md) follows that work through
linkage storage, arm motion and dissipation, then checks felt lift and key
return under controlled slack, ratio and arm-loading changes.

The [hammer return study](docs/LOADED-HAMMER-RETURN.md) extends that gesture
to 800 ms, traces pedestal contacts and checks free hammer motion against
an independent analytical solution. Return damping and contact loss controls
retain separate attack, felt-lift and settling decisions.

The [two-strike repetition study](docs/LOADED-REPETITION.md) follows the same
mechanical and electrical state through a second key gesture. It separates
action readiness, repeated impact consistency, unwanted contacts and felt
lift at two recovery intervals.

The [launch work and momentum study](docs/LOADED-LAUNCH.md) reconstructs the
hammer's first and repeated pre-impact states from incoming motion, signed
force impulses, contact work, spring storage and heat. Contact sequences
remain visible alongside the unchanged repetition measurements.

The [terminal drive study](docs/LOADED-DRIVE-RELEASE.md) compares the original
pedestal stop with continuous terminal deceleration and a matched-duration
linear control. It verifies the imposed trajectory alongside contact work,
first-strike changes and repetition.

The [drive onset study](docs/LOADED-DRIVE-ONSET.md) adds smooth and
constant-acceleration onset ramps and a fully eased key drive, splitting each
gesture into onset, cruise, stop and hold segments with independent pedestal
work and impulse. It shows that the abrupt onset, not the return regulation,
produces the original soft-strike collision and its repetition anomaly.

The [key inertia study](docs/LOADED-KEY-INERTIA.md) replaces the prescribed
pedestal with a lumped key under a step finger force, inelastic or felt-bed
stops and an exact key energy ledger. It shows that the hammer rides the key
and is launched only by the stop, that a compliant bed removes the soft
strike, and that key mass is a first-order parameter of the action.

The [let-off study](docs/LOADED-LETOFF.md) adds a regulated escapement to
that key, so the hammer is released by geometry before key bottom. A sharp
let-off reproduces the hard-stop launch while freeing the key; a 0.6 mm
roll-off or a 1 mm earlier release removes the soft strike, and the bridle
load consumes most of the hammer's flight energy.

The [flight budget study](docs/LOADED-FLIGHT-BUDGET.md) measures that flight
with hammer, bridle and damper-arm identities and varies hammer mass and the
bridle and arm loads. The flight is a nearly fixed energy toll set by the
damper-arm spring, and the soft threshold is release energy above it.

The [gravity study](docs/LOADED-GRAVITY.md) gives the hammer and damper arm
their weight with an exact potential in every ledger and a weighted rest.
Weight seats the hammer, adds a small mass-proportional flight toll and
leaves the return bounce as the remaining repetition defect.

The [landing study](docs/LOADED-LANDING.md) traces the hammer's landing on
the returned pedestal. A strongly dissipative pedestal contact settles the
hammer within 40 ms of release without changing the strike, and the damper
felt's own bounce becomes the remaining readiness defect.

The [damper seating study](docs/LOADED-DAMPER-SEATING.md) traces the felt's
own landing on the tine and the damping onset it produces. The felt lands
27 ms after key-up at 0.36 m/s and bounces; felt contact loss shortens the
bounce without changing the strike, and the lift geometry that throws the
felt back becomes the next question.

The [playable G3 diagnostic](docs/PLAYABLE-G3-DIAGNOSTIC.md) measures the
plugin's engine against the retained recordings, and the
[pickup harmonics study](docs/PICKUP-HARMONICS.md) shows that its darkness is
the pickup transfer alone: the laboratory's finite-aperture law at a close,
wide pole reproduces the recorded harmonic balance from the engine's own tine
motion. The [Close Aperture path](docs/PICKUP-APERTURE-PATH.md) puts that
law into the playable engine as a fourth level-matched pickup: the loud G3
third harmonic rises to +8.4 dB against the recording's +7.0 dB, the sustain
is unchanged, and the all-keys stress now misses a few deadlines. The
[calibrated sustain](docs/PLAYABLE-SUSTAIN.md) then gives the engine
per-partial losses from the D3, G3 and B3 recordings, 20 s and 2.3 s at A3
for the first and bar partials, and closes the 17 to 21 s fundamental T60
deficit to within 5 s without touching the defaults. The
[bar partial block](docs/PLAYABLE-BAR-PARTIAL.md) then shows that the
recordings' line at six times the fundamental is the pickup's sixth harmonic,
not a bending partial, and that the engine's second partial was over-excited
by 20 to 35 dB; `Profile::calibrated` strikes it fifteen times more softly.
The plugin now exposes the three profiles as a **Profile** parameter beside
the four pickups, switchable while notes ring, so the calibrated mechanics
can be auditioned in RackForge before any default changes. The
[voicing physics](docs/VOICING-PHYSICS.md) block then gives the voice its
own pickup law, a velocity exponent and a level compensation derived from
the pickup law, measured over 72 geometries, so continuous voicing controls
can move the pickup without moving the loudness. The [Sound page](docs/SOUND-PAGE.md)
then makes the plugin an instrument: pickup law, distance and alignment,
hammer hardness, sustain, bell and dynamics as physical parameters with the
compensated pickup, four factory presets, and state schema 4 that maps the
old pickup slots and profiles onto the voicing they were listening to.

The [damper lift study](docs/LOADED-DAMPER-LIFT.md) changes the bridle ratio,
bridle slack and arm spring that set that lift. The linkage geometry turns
out to govern both the soft threshold and the damping onset: a lower ratio
halves the flight toll, and extra slack seats the felt in time for the next
gesture.

**RF-Tines is an independent physical-model electric piano and is not affiliated
with or endorsed by any historical instrument manufacturer.** Reference
recordings inform development and are never included in the plugin. The model
is suitable for beta release and listening evaluation; its physical parameters
remain provisional rather than measurements of one specific instrument.

## Quick start

To build, validate, install and open the current instrument in RackForge Desktop on Windows:

```text
cargo run --locked --release -p rf-tines-lab -- audition
```

The [audition workflow](docs/AUDITION.md) keeps a dedicated test library, retains audio/MIDI preferences and supports repeated builds of the same version.

```text
cargo test --locked --workspace
cargo run --release -p rf-tines-lab -- demo --output renders/demo.wav
cargo run --release -p rf-tines-lab -- render --output renders/a3.wav --trace
cargo run --release -p rf-tines-lab -- inspect renders/demo.wav
cargo run --release -p rf-tines-lab -- analyze renders/a3.wav --output renders/a3-analysis.json --note 57 --sustain-end 1.8
cargo run --release -p rf-tines-lab -- compare renders/a3.wav renders/a3.wav --output renders/self-comparison.json
cargo run --release -p rf-tines-lab -- stress
cargo run --release -p rf-tines-lab -- converge --output renders/convergence.json --note 100 --velocity 0.2
```

Requires Rust 1.98 and a sibling RackForge checkout for its public SDK. See [Development](docs/DEVELOPMENT.md) for Windows linker setup, WASM builds and packaging. Existing audio and report files are never overwritten.

## What is here

- `rf-tines-dsp`: safe Rust DSP with bounded contact integration, 73 fixed key states, per-key pickups and 4x antialias filtering.
- `rf-tines-plugin`: RackForge adapter with MIDI 1.0/2.0, matched A/B, declarative program editing and versioned state.
- `rf-tines-ui`: Rust WebAssembly PLAY panel with A/B controls, host synchronization and day/stage styling.
- `rf-tines-analysis`: offline WAV input, FFT spectra, harmonic and independent partial tracks, qualified decay estimates and aligned comparisons.
- `rf-tines-lab`: Rust WAV renderer, physical CSV traces, measurement commands, JSON reports and timing diagnostics.
- Tests for mechanical passivity, repeated strikes, dampers, MIDI ownership, block invariance, malformed input and file integrity.

The rendering and parameter-automation paths use no allocation, locks or I/O. The offline laboratory does not open an audio device. No samples, reverb, amplifier, compressor or limiter hide the direct model output. The plugin applies documented, fixed pickup level compensation.

## Read next

- [Physical-model direction review](docs/RESEARCH-DIRECTION-2026-09-06.md): literature evidence, current assumptions and the next audible/calibration experiments.
- [Controlled hammer comparison](docs/CONTROLLED-HAMMERS.md): equal-launch elastic, rate-dependent and memory candidates with independently qualified offline audio.
- [Frequency reference preparation](docs/PITCH-REFERENCE.md): verified G3 sources, training-only pitch target and explicit limits before resonator fitting.
- [First physical-assembly WAVs](docs/MEMORY-MODAL-AUDIO.md): four offline previews with sampling, integration and headroom checks.

- [Pickup Lab UI](docs/PICKUP-LAB-UI.md): controls, A/B, saving, transition and gain policy.
- [Physical model ledger](docs/MODEL.md): equations, constants and known approximations.
- [Coupled assembly](docs/COUPLED-ASSEMBLY.md): mechanical reduction, energy balance, analytic tests and convergence limits.
- [Modal performance](docs/MODAL-PERFORMANCE.md): prepared nine-coordinate mechanics, independent work and native block timing.
- [Modal contact solver](docs/MODAL-CONTACT-SOLVER.md): bounded root-search acceleration with bisection and energy references.
- [Dissipative hammer](docs/DISSIPATIVE-HAMMER.md): rate-dependent contact loss, nonadhesive unloading and independent material heat.
- [Hammer material memory](docs/HAMMER-MEMORY.md): internal deformation, relaxation and repeated loading in an isolated material coupon.
- [Memory hammer impacts](docs/MEMORY-HAMMER.md): two inertial masses, nonadhesive surface contact, free recovery and impulse-driven reimpact.
- [Stateful modal hammer](docs/MEMORY-MODAL-COUPLING.md): reciprocal memory-hammer/tine coupling, persistent recovery and independent port work.
- [Stateful modal performance](docs/MEMORY-MODAL-PERFORMANCE.md): reused structural energy, exact diagnostic checks and native kernel timing.
- [Hammer free recovery](docs/MEMORY-FREE-MOTION.md): conservative clearance bounds and error-controlled longer free intervals.
- [Moving modal free recovery](docs/MEMORY-MODAL-FREE.md): moving-port contact exclusion, transactional propagation and coupled validation.
- [Direct material roots](docs/MEMORY-MATERIAL-SOLVE.md): checked quadratic branches inside the stateful contact solver.
- [Adaptive contact](docs/MEMORY-ADAPTIVE-CONTACT.md): continuous compression bounds, step doubling and retained fine references.
- [Contact scheduling](docs/MEMORY-CONTACT-SCHEDULING.md): minimum useful trial lengths and deferred retries with unchanged physical checks.
- [Contact reaction reuse](docs/MEMORY-CONTACT-FORCE-REUSE.md): avoid repeating the material solve at an already evaluated normal force.
- [Contact resolution](docs/MEMORY-CONTACT-RESOLUTION.md): read-only trial sweeps identify which coupled state errors limit the timestep.
- [Fourth-order hammer contact](docs/MEMORY-CONTACT-RK4.md): certified fixed-wall RK4 contact with independent heat, work and impulse integration.
- [Coupled fourth-order contact](docs/MEMORY-MODAL-RK4.md): moving tine/tonebar integration with reciprocal work checks and native timing.
- [Coupled resolution study](docs/MEMORY-MODAL-REFINEMENT.md): separate contact/free interval caps, implicit-reference sensitivity and 32 ms trajectories.
- [Incremental modal midpoint](docs/MODAL-MIDPOINT-INCREMENTS.md): preserve force-free velocity and reduce reference drift without changing the discrete equations.
- [Longer modal tails](docs/MEMORY-MODAL-TAIL.md): 128 ms trajectories with separately gated attack and tail accuracy.
- [Modal stiffness cost](docs/MODAL-STIFFNESS-COST.md): section timing and exact diagonal stiffness products with a dense reference path.
- [Contact trial reuse](docs/MODAL-CONTACT-TRIAL-REUSE.md): share identical starting derivatives and state energies while retaining every acceptance check.
- [Contact damping cost](docs/MODAL-CONTACT-DAMPING-COST.md): exact diagonal damping products with a dense reference and unchanged reciprocal heat.
- [Late reimpact study](docs/MEMORY-MODAL-REIMPACT.md): repeated-excitation accuracy failures despite passing energy/work checks; qualification remains open.
- [Shared late-impact checkpoints](docs/MEMORY-MODAL-CHECKPOINT.md): local collisions pass when all physical history starts equal; full repetition remains unqualified.
- [Shared impulse approach](docs/MEMORY-MODAL-APPROACH.md): pre-contact motion passes; one post-impact recovery still exposes reference-grid sensitivity.
- [Shared post-separation recovery](docs/MEMORY-MODAL-RECOVERY.md): identical-state recovery passes; next integrate a selected-strike offline audio preview.
- [Roadmap](docs/ROADMAP.md): implemented work and next milestones.
- [Measurement protocol](docs/MEASUREMENTS.md): reference recordings and evaluation.
- [Analysis laboratory](docs/ANALYSIS.md): commands, metric definitions and interpretation limits.
- [Partial comparison](docs/PARTIAL-COMPARISON.md): compare corresponding spectral components and qualified decays across recordings.
- [Pickup sweep](docs/PICKUP-SWEEP.md): rank a bounded geometry grid against a reference with one global level correction.
- [Pickup reference set](docs/PICKUP-SET.md): fit several takes with shared gain and evaluate reserved notes or intensities.
- [Reference banks](docs/REFERENCE-BANKS.md): commercial candidates and the acquired five-layer real-recording pilot.
- [Tone comparison](docs/TONE-COMPARISON.md): explicit attack/body windows and harmonic balance without inferred velocity or decay.
- [G3 residual pilot](docs/G3-RESIDUAL-PILOT.md): measured baseline and exploratory pickup differences against the acquired recordings.
- [Isolated pickup transfer](docs/PICKUP-TRANSFER.md): compare two magnetic laws under identical motion and measure internal sampling error.
- [Mechanical pickup pairs](docs/PICKUP-PAIR.md): compare the two laws on the production trajectory and filter, with a complete G3 reference matrix.
- [Pickup convergence](docs/PICKUP-CONVERGENCE.md): separate mechanical and pickup/filter sampling residuals across registers, with finite 128x/256x references.
- [Pickup listening](docs/PICKUP-LISTENING.md): four performances with fixed global RMS matching, sample-peak control and measured full-keyboard headroom.
- [Close Aperture pickup path](docs/PICKUP-APERTURE-PATH.md): the finite-aperture law as a fourth level-matched plugin pickup, measured on the G3 strikes, the nocturne and the all-keys stress.
- [Calibrated sustain](docs/PLAYABLE-SUSTAIN.md): per-partial T60 in the playable profile, derived from fifteen recordings and measured on D3, G3 and B3 renders.
- [Bar partial](docs/PLAYABLE-BAR-PARTIAL.md): the six-times line identified as the pickup's sixth harmonic, the second partial's strike weight and ratio as profile fields, and the impulsive contact measured.
- [Voicing physics](docs/VOICING-PHYSICS.md): pickup law and velocity exponent in the profile, and a reference-motion level compensation measured over gap, offset and law.
- [Sound page](docs/SOUND-PAGE.md): the plugin's physical voicing parameters, presets, surfaces and state schema 4.
- [Numerical convergence](docs/CONVERGENCE.md): the treble-contact correction, experiment and residual errors.
- [Desktop audition](docs/AUDITION.md): build, install and launch each test version.
- [Sources](docs/SOURCES.md): primary research and evidence scope.
- [September research review](docs/RESEARCH-2026-09.md): papers, Rust projects, recording candidates and prioritized experiments.
- [Development](docs/DEVELOPMENT.md): commands, integration and output formats.
- [Validation results](docs/VALIDATION.md): tests, native/WASM timing and remaining limitations.

The new physical assembly now has selected-strike offline WAV previews with checked pickup sampling and output headroom. Next compare the hammer alternatives on that output chain, then calibrate modal behavior. The existing 0.1.2 plugin remains the playable baseline. Calibration still targets A3: compare multiple intensities and decay phases against documented direct recordings, then extend validated parameters across the keyboard. All project code, tools, tests and documentation are in English; executable project code is Rust.
