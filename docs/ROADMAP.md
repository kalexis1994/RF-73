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

The [2026-09-06 direction review](RESEARCH-DIRECTION-2026-09-06.md) supports
the coupled modal architecture while prioritizing an offline audible reference,
a controlled comparison of hammer models and measured modal calibration.
The two-mass memory hammer remains a research hypothesis; its additional state
and numerical cost need demonstrated physical or acoustic value before adoption.

The [controlled hammer comparison](CONTROLLED-HAMMERS.md) now renders elastic,
rate-dependent and memory candidates through the same structural and output
configuration. It preserves launch energy and raw gain, qualifies each candidate
independently, and records contact, attack-band and envelope observables.
The comparison remains uncalibrated; fitting against shared training data and
held-out observations is the next step before selecting a hammer model.

The [frequency-reference pilot](PITCH-REFERENCE.md) freezes a 196.386147 Hz G3
target from declared training layers and evaluates separate validation layers.
The broad-band diagnostic measures the provisional 75 mm assembly at
169.570540 Hz. One-parameter tuning of the coupled resonator is next; a single
output frequency does not identify the physical geometry or material.

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
difference from the fine reference is 0.2612%.
The [resolution study](MEMORY-MODAL-REFINEMENT.md) now separately refines contact,
free motion and the implicit reference, with 96 audited takes including four
32 ms profiles. RK4 interval refinements agree much more closely than the
implicit-reference refinements. This points to reference sensitivity as a
substantial contributor to the earlier difference, without establishing an
exact continuous solution. Some late-window differences grow, and the finest
uniform path's structural work residual reaches 7.327e-9 against a 1e-8 gate.
The [incremental midpoint correction](MODAL-MIDPOINT-INCREMENTS.md) now removes
repeated multiplication by a rounded near-identity matrix. Force-free velocity
is preserved exactly in the new dense-inertia test; the finest uniform path's
32 ms structural residual falls to 2.654e-13 with the same discrete equations.
All 276 affected regression takes pass. Trajectory differences remain measurable,
so longer/wider validation, further cost reduction and polyphonic host
qualification remain open.

The [128 ms tail study](MEMORY-MODAL-TAIL.md) now passes twelve trajectories
across four selected strong-strike profiles, with separate attack and tail
accuracy gates. Default RK4/reference velocity RMSE reaches 0.1665% in the
attack and at most 0.02316% in 64–128 ms; some individual profiles still grow
before decaying. Structural reference drift remains controlled, while the
reference hammer-work residual grows to 4.562e-10. Wider gesture/profile
coverage, longer-path native cost and reference-ledger monitoring are next.
The [stiffness cost study](MODAL-STIFFNESS-COST.md) now times those paths by
section and skips exactly zero off-diagonal stiffness products. Same-executable
paired total medians improve by 7.7–11.3% while all three repeated numerical
reports remain byte-identical. The first 8 ms still costs 51.68–55.76 ms for one
assembly. Coupled attack cost, material/free recovery cost, broader physical
coverage and host qualification remain open.

The [contact trial reuse study](MODAL-CONTACT-TRIAL-REUSE.md) removes repeated
initial derivatives and endpoint energies while preserving every acceptance
check. Both repeated trajectory reports are byte-identical. In 24 paired timing
runs, the first 8 ms is faster in all four profiles; total medians improve in
three profiles and remain essentially tied in the fourth. The attack still costs
48.32–58.08 ms per assembly, so further contact-cost work and broader physical
qualification remain necessary before realtime integration.

The [contact damping cost study](MODAL-CONTACT-DAMPING-COST.md) now skips zero
off-diagonal damping products in coupled RK4, keeping the full product for any
coupled damper matrix. Paired total medians improve by 2.36–7.60% with identical
section states and counters. The first 8 ms still costs 45.09–49.52 ms for one
assembly. Broader gesture coverage, physical calibration and substantial further
cost reduction remain necessary before integration with the audible engine.

The [late reimpact study](MEMORY-MODAL-REIMPACT.md) exposes a new numerical
qualification failure: additional 32/80 ms impulses preserve energy/work in
all twelve takes but fail trajectory comparisons in all four profiles. Default
and capped RK4 also diverge in three profiles. Prior tail qualification does not
cover these repeated impacts. Isolating a late collision from an identical full
preimpact checkpoint is now the priority before further performance work or
real-time integration; the root cause remains unresolved.

The [shared-checkpoint study](MEMORY-MODAL-CHECKPOINT.md) now passes 48 local
8 ms continuations from eight identical complete preimpact states. Independent
contact/free caps agree closely; default versus finer midpoint reaches at most
0.019647% section kinetic velocity RMSE. This localizes the earlier failure:
large divergence is not reproduced over these short collisions with equal
history. Full repetition remains unqualified. Move the shared checkpoint back
to the late impulse and isolate intervening free/material recovery next.

The [shared-impulse study](MEMORY-MODAL-APPROACH.md) now passes every force-free
approach prefix and seven of eight complete 48 ms continuations. The second
impulse at 120 mm / 10 ms relaxation still fails kinetic trajectory gates;
even the two uniform midpoint grids diverge after contact. At the final endpoint
the difference is concentrated in the hammer, while pickup/force gates pass.
Next isolate post-separation material recovery from one identical state to
distinguish inherited impact differences from free-propagation error. Full
repetition and reference convergence remain unqualified.

The [shared post-separation study](MEMORY-MODAL-RECOVERY.md) passes all 20
continuations from identical 96 ms states. Default/finest-midpoint section
kinetic RMSE stays below 0.000017105% of launch across 32 ms of recovery.
The earlier divergence is not reproduced by this selected free propagation;
history accumulated before 96 ms remains under investigation. Full repeated
excitation is still unqualified.

Prioritize an offline single-strike audio preview of the new physical assembly
as the next integration milestone. Connect modal pickup motion to magnetic
conversion, qualify oversampling/decimation and explicit gain/headroom, and
render attack, decay and damper release with numerical receipts. This supports
listening before realtime optimization is complete. Keep the existing plugin
as an audible baseline and retain the known repetition failure separately.
Realtime use additionally needs CPU reduction, note lifecycle, bounded
polyphony and host validation; no calendar estimate is established.

The [first physical-assembly WAV milestone](MEMORY-MODAL-AUDIO.md) is now
implemented. Four selected 1.5-second previews pass independent mechanical
ledgers, section-wise refinement, shared-trajectory 16/32/64x pickup sampling
and output headroom checks. These are uncalibrated single strikes with a binary
damper, not tuned keyboard notes or a realtime plugin update. Next compare
hammer alternatives on the same audible chain, followed by modal calibration.

The target remains a sophisticated Rhodes-specific physical model, comparable in development depth to RF Concert Grand. The laboratory UI is a measurement and audition tool, not a declaration that the sound engine is finished. Numerical stability and passing tests do not establish realism.

The current mechanical work builds on the explicit tine/tonebar assembly with mounting compliance. Determine which measured modes belong to which assembly motion; introduce orthogonal motion and coupling where the evidence supports it. Preserve the existing plugin engine as an A/B baseline. Fit frequency, decay and coupling against reference observations, and qualify numerical accuracy and performance before real-time integration.

Subsequent work covers hammer-tip material/history and strike geometry, action/repetition, geometry-based magnetic conversion, continuous dampers and release response. Extend identified behavior across the keyboard and velocities before adding optional electronics or spending time on final visual styling. A complete physical-field simulation of every part is not required for real-time fidelity; every reduction must have a documented assumption and a measurable validity range. The current five processed G3 layers do not identify all these physical parameters.

## Calibration milestone: a calibrated A3

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

## Coupled G3 spring-tuning pilot

[Spring-position tuning](SPRING-TUNING.md) now fits the movable 0.1 g tuning
mass on an explicitly provisional fixed 70 mm blank. A coupled-mode solve
tracks physical fields as the spring moves; the independently rendered output
lands within 0.067 cent of the frozen G3 anchor. Both sides of the listening
pair pass numerical qualification. Fixed-root mode ratios change alongside
pitch, including a 4.51% change in the third/first ratio. Listening and modal
weight/timbre identification remain open. This is an offline single-note
experiment, not keyboard calibration or a realtime plugin change.


The [post-tuning observations](POST-TUNING-MODES.md) now compare the frozen
pair and all five source layers at native rates. Higher model components are
audible but do not align with accepted reference peaks under the declared
windows. A post-hoc 1425 Hz family in layers 3/4/5 is a candidate for further
identification, not a new calibration target. Next examine tine/mass geometry
sensitivity at fixed coupled pitch before fitting modal weights or losses.


The [fixed-pitch geometry study](FIXED-PITCH-GEOMETRY.md) retains nine length/mass
cases: eight retune, one is unreachable. Local physical inertia resolves false
ambiguity in the wider spring search without changing acceptance thresholds.
Solved higher-mode frequencies and port residues vary substantially at the
same G3 fundamental; none of the grid's rank-5 modes reaches the exploratory
1425 Hz family. No new sound is selected. Additional modal/spatial evidence is
needed before adopting geometry, with nonlinear/audio qualification reserved
for a justified candidate. The existing tuned WAV pair remains the baseline.

The [finite spring-span study](FINITE-SPRING-SPAN.md) now integrates uniform
co-moving tuning mass over 0/2/4/6 mm. All four cases regain G3; upper modes and
port residues change, but rank 5 moves away from the exploratory 1425 Hz family.
Mass moments, a uniform-density analytic case, mesh convergence and coupled
contact/release energy are tested. No width or new sound is selected. Next
investigate nonuniform tine geometry with explicit assumptions and independent
checks, while seeking modal/spatial evidence before fitting a physical profile.

The [linear tine-taper study](TINE-TAPER.md) now derives mass and stiffness from
a varying circular section. Four tip/root ratios retune to G3, with analytic
frustum moments, static flexibility, mesh convergence and coupled energy checks.
Full-length narrowing lowers the rank-5 mode away from the exploratory 1425 Hz
family. No profile is selected. Next distinguish localized section transitions
from full-length taper, and require multiple modal/spatial observations before
geometry identification. The uniform structural and audio baselines remain exact.

The [localized transition study](TINE-TRANSITION.md) now separates section ratio
from the extent of its root transition, integrating exactly across off-mesh
corners. Four transition endpoints retune to G3; earlier endpoints lower rank 5
away from the exploratory family. Analytic moments, static flexibility,
convergence and coupled energy checks pass. Next constrain modal identity with
several spectral families and cross-note evidence before selecting more geometry
or fitting losses. No measured section or new sound has been selected.

[Cross-take spectral families](SPECTRAL-FAMILIES.md) now group all independently
detected G3 peaks without model proposals. Four attack-128 components recur;
the 1425/1620 Hz pair has a fundamental-offset relation in layers 3/4/5, so it
must not be assumed to represent two independent mechanical modes. Capped,
harmonic and ambiguous evidence remains explicit. Next test cross-note behavior
and low-frequency background before assigning structural identity or fitting
additional geometry. No physical parameter or audible baseline changed.

The [D3/G3/B3 comparison](REGISTER-FAMILIES.md) now retains all fifteen source
takes, qualified pitch anchors and independent family observations. Neighboring
notes also show fundamental-offset chains. G3 attack families have no matches
under the tested fixed-Hz or constant-ratio rules; some body components have
fixed-frequency correspondences. Neither result identifies modal origin. Next
test prescribed two-mode motion through linearized/nonlinear pickups, including
sideband and sampling checks, before further geometry or damping fitting.

The [controlled pickup mixing study](PICKUP-MIXING.md) now demonstrates
fundamental-spaced sidebands in both existing nonlinear laws, absent from the
linearized and independently transduced controls. Weak-motion coefficients match
an analytic expansion; all eight probes pass full ideal-band sampling refinement.
This establishes a mechanism, not recorded modal identity. Next test independent
modal decay envelopes and predicted sideband decay/phase relations before using
the source families to fit geometry or losses. No DSP or audible baseline changed.

The [phase-resolved decay study](PICKUP-DECAY.md) now recovers the sum of modal
amplitude-decay rates in weak pickup sidebands and quantifies finite-amplitude
departures. Exact envelope velocity passes independent derivative/phase checks;
all eight probes qualify. Phase quadrature is not a recording estimator. Next
validate a temporal envelope estimator against synthetic known-rate mixtures,
nearby components and noise before applying it to the pinned recordings or
fitting mechanical losses. Production DSP and the audible baseline are unchanged.

The [temporal component-envelope estimator](COMPONENT-ENVELOPE.md) now measures
complex amplitude and phase in WAVs with bounded work and explicit rejection
reasons. All 18 synthetic expectations pass across two window lengths. Native-rate,
gain, PCM and an undeclared-neighbor limitation are tested. Next observe a bounded
set of pinned source families with declared intervals/neighbors and retain failed
gates; qualified envelope slopes alone still do not identify mechanical losses.

The [pinned G3 source-envelope pilot](SOURCE-ENVELOPES.md) now retains all five
takes, 15 component slots and 22 measurements. One fundamental has conditional
cross-window agreement; every available higher-family envelope rejects, so all
mixing-rate comparisons are withheld. Upper-family initial detectability is too
short for the present support requirement. Next validate short-transient
estimation with uncertain carriers and neighboring harmonics before revisiting
the sources. No rejection threshold, mechanical loss or audible baseline changed.

The [joint short-envelope estimator](SHORT-ENVELOPE.md) now fits declared carriers
with quadratic local envelopes and rejects ill-conditioned combinations. Synthetic
32/64 ms probes cover strong neighbors, noise, detuning and short support; all
18 final expectations pass. Next freeze short source intervals and nuisance
harmonics, compare both widths and retain failures before adopting physical
losses. Earlier source/Hann gates and the audible baseline are unchanged.

The [short G3 source pilot](SHORT-SOURCE-ENVELOPES.md) now fixes 20..180 ms,
32/64 ms windows and the two nearest prior nuisance peaks. All five takes and
15 slots are retained; four families are missing and two fundamental selections
are unsupported. None of 18 measurements qualifies. Next validate a frequency-
selective observation stage against strong distant components, including its
transient/decay bias, before repeating source inference. No losses were fitted.

The [band-isolated short-envelope study](BAND-ENVELOPE.md) now recovers a weak
target beside a 300-times-stronger distant fundamental. All 30 positive cases
pass, but three 64 ms onset observations evade the individual-window gates;
the original 66-case study remains failed (63 expectations pass). The existing
two-window criterion gives the intended result for these 33 pairs. Next test
paired temporal-event coverage before freezing a filtered source protocol.
No gate was relaxed and no physical parameter or source recording changed.

The [paired temporal-event study](BAND-EVENTS.md) retains 162 pairs across three
sample rates. Both windows accept 53 of 96 event-containing intervals, including
early release rates near 40 /s and late release rates near 8 /s. Pair agreement
does not identify natural sustain; every result states that limit explicitly.
Next use controlled mechanical excitation/damper timing to test loss recovery.
The processed bank remains conditional timbral evidence, not a natural-loss target
without independent state and mode information. No estimator gate or DSP changed.

The [controlled mechanical loss study](MECHANICAL-LOSS.md) now recovers separate
structural and damper multipliers from full-state nine-coordinate trajectories.
All six known-state cases pass held-out energy prediction; omission of damper
power leaves 63..74% relative residual. Independent endpoint quadrature error
decreases under 48/96 kHz refinement. Next reduce observation to modal/pickup
motion under known event history; full-state success does not identify source
losses. New matrix inspection methods leave the physical solver unchanged.

The [reduced-observation loss study](REDUCED-MECHANICAL-LOSS.md) retains 36
observations of the same six mechanical trajectories. Three and six exact modal
projections recover the two scales within the existing criterion; one mode and
an instantaneous pickup lift fail. Reduced-state bias persists under refinement.
Next test dynamic reconstruction from pickup history, including noise and omitted
modes. Oracle modal projection is not audio extraction or a reduced plugin engine.

The [dynamic pickup-state study](DYNAMIC-PICKUP-STATE.md) retains 108 fits from
one scalar mechanical velocity channel, including noise and omitted modes.
All twelve nine-mode noiseless controls pass held-out prediction with supplied
damping. Three-mode inference differs substantially from oracle projection;
even nine modes with 1% noise can have 43.71% error in one weakly observed mode.
Next profile unknown structural/damper scales with training-only nuisance-state
fits, held-out predictions and sensitivity/wrong-operator controls. Known-damping
state recovery is not evidence of unknown-loss or recorded-audio identification.

The [profiled pickup-loss study](PROFILED-PICKUP-LOSS.md) now estimates unknown
structural/damper multipliers with training-only nuisance-state fits. All six
noiseless off-grid controls pass. With 1% training noise, both scale errors stay
below 1%, but three held-out predictions fail. Wrong damper contact position
biases its scale by 46..52%; all six cases are rejected, including four interior
fits. Next quantify smaller operator errors and qualify the magnetic observation
path. This synthetic two-scale result still assumes known geometry and timing.

The [small-position-error study](PICKUP-LOSS-POSITION-ERRORS.md) retains 102
noiseless fits: 100 pass prediction, but 46 of those have biased damper loss.
A -0.75 mm contact-position error can hide a 3.91% loss error behind a 0.0953%
pickup residual. Pickup-position mismatch is absorbed by independent window
states while motion estimates change. Next enforce state continuity through
the known damper event and test whether that reduces position ambiguity, then
qualify magnetic observation. Existing prediction gates cannot certify geometry.

The [continuous-state loss study](CONTINUOUS-PICKUP-LOSS.md) now uses one initial
state propagated through the known damper event. All six matched controls pass;
paired accepted-but-biased fits fall from 46 to 28 across 102 position cases.
The 54 fits within 1% of both true scales remain unchanged: this improves rejection,
not geometric identification. Pickup-position mismatch becomes visible in the
residual but still passes the existing gate. Next qualify the nonlinear magnetic
observation on these trajectories while retaining continuity and geometry limits.

The [magnetic observation study](MAGNETIC-OBSERVATION-LOSS.md) retains 66 cases:
30 positive controls pass, 12 centered zero-slope observations are withheld, and
all 24 full nonlinear voltage fits fail prediction and two-scale recovery under
the rest-linearized observer. Next qualify a nonlinear state fit with known
mechanics/sensor geometry, continuous state and held-out samples before freeing
loss parameters. These direct point samples do not validate antialiasing or a
real-instrument field law; the production equations remain unchanged.
