# Physical-model direction review

Research date: 2026-09-06. Reviewed RF-Rhodes revision `4b08629` and its
retained laboratory results. This is an engineering assessment, not an acoustic
validation or an implementation change.

## Decision

Keep the coupled modal resonator, reciprocal force ports, independent energy
accounting and Rust implementation. Change the immediate emphasis: establish an
audible, measurable path through the new assembly and compare physical model
alternatives before further expanding or optimizing the two-mass hammer.

The architecture is consistent with published Rhodes modelling. Its current
parameters and reductions are not yet demonstrated to represent a real Rhodes.
The principal project risk is spending substantial effort accurately integrating
an uncalibrated internal mechanism while the new resonator still has no evaluated
magnetic audio output. More coordinates and smaller timesteps do not by themselves
establish greater fidelity.

## Evidence and access quality

Publication years below come from the documents or publication records, not
search-engine crawl dates. This targeted review includes direct Rhodes research,
general numerical methods relevant to it, a service manual and implementation
comparisons. It is not an exhaustive systematic review. No external code was run,
no recordings were acquired, and no audio or desktop application was opened.

### Direct modelling and measurement research

**Pfeifle, DAFx 2017 — Real-time Physical Model of a Wurlitzer and Rhodes
Electric Piano.** Full conference PDF retrieved; sections 4–8 inspected. It
reports measured nonplanar tine motion and combines a beam with tuning mass,
two transverse polarizations, nonlinear effects, rate-dependent hammer contact
and geometric magnetic conversion. Its contact law is of Hunt–Crossley type.
The reported realtime implementation uses FPGA hardware; it does not establish
CPU or WASM performance for our solver. The author explicitly retains pickup
geometry simplifications. This supports our physical decomposition and motivates
testing a second polarization, but supplies no validation of our two-mass
Maxwell hammer. [Conference paper](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

**Falaize and Hélie, JSV 2017, 390:289–309.** The published description uses
passive interconnections between a nonlinear hammer, modal Euler–Bernoulli beam
and nonlinear pickup. The author demonstration includes electrical filtering and
comparisons with UVI recordings. This is strong methodological support for
energy-aware modal reduction. Access in this review: publisher abstract and
section descriptions plus author page; the linked full manuscript was blocked.
Do not describe our implementation as a reproduction of its discretization or
adopt demonstration settings as measured factory constants.
[Publication, DOI 10.1016/j.jsv.2016.11.008](https://www.sciencedirect.com/science/article/pii/S0022460X16306320),
[author demonstration](https://afalaize.github.io/posts/rhodes/).

**Gabrielli, Cantarini, Castellini and Squartini, JASA 2020, 148:3052–3064 —
The Rhodes electric piano: Analysis and simulation of the inharmonic overtones.**
The institutional abstract describes pickup spectra, perceptual relevance,
scanning laser vibrometry of the complete fork and separate components, magnetic
intermodulation, and important modes across the keyboard. This makes it the
priority source for mode identification. Only the abstract was verified; no
modal tables, listening scores or numerical fit were reproduced. In particular,
an electrical spectral peak cannot automatically be assigned to another
mechanical mode. [Institutional record and abstract](https://iris.univpm.it/handle/11566/286030),
[DOI 10.1121/10.0002002](https://doi.org/10.1121/10.0002002).

**Sønderbo, Aalborg master's thesis, 2024 — Real-time physical model of the
Rhodes Electric Piano.** Full PDF retrieved; model, implementation and evaluation
chapters inspected. The author implements FDTD mechanics, Hunt–Crossley contact,
a damper and MPE control. The discussion identifies tuning errors, overly
persistent overtones and a nonphysical pickup producing an excessively harsh
result. The practical lesson is to validate tuning, losses and pickup output
together. This is a master's project with preliminary evaluation, not evidence
of perceptual equivalence or a source of validated RF-Rhodes constants. The
repository landing page labels its own abstract AI-generated; this review relies
on the thesis itself. [Full thesis, especially chapters 3, 6 and 7](https://projekter.aau.dk/projekter/files/719175589/Master_Thesis_Real_time_physical_model_of_the_Rhodes_Electric_Piano_Tobias_Sonderbo.pdf).

**Shear, UCSB master's thesis, 2011 — The Electromagnetically Sustained Rhodes
Piano.** Full PDF retrieved; chapters 2 and the Q-related discussion inspected.
The original instrument is identified as a 1974 Mark I Stage 88. Table 2.1
provides example original-instrument Q values; chapter 5 distinguishes modified
assemblies. The amplitude-decay definition gives `Q = pi*f*tau`. This closes
the earlier review's source-reading gap, but these examples are not a complete
per-mode decay map or universal Rhodes specification. The separation of
mechanical and pickup spectra is particularly useful for our measurement plan.
[Thesis, sections 2.1–2.2 and 5.1](https://mat.ucsb.edu/Masters/GregShearMasters2011_12_5.pdf).

**Prokop, Student EEICT 2024 — The Innovation of Oscillators in Rhodes pianos.**
The institutional record and indexed opening text describe measured Mk7
assemblies, CAD/FE simulation and fabricated geometry/material alternatives.
Useful as a complementary example of simulation checked against hardware;
not a validated synthesis engine or a Mark I calibration table. Full local PDF
retrieval timed out, so detailed numerical results were not assessed.
[Institutional record](https://dspace.vut.cz/items/2cc2fca5-c64c-481f-b123-945c03f79764).

### Numerical methods and instrument documentation

**Bilbao, Torin and Chatziioannou, 2014 — Numerical Modeling of Collisions in
Musical Instruments.** The abstract explains collision potentials and
energy-conserving/dissipating discretization, including the distinction between
deformable contact and a penalty approximation to rigid contact. This supports
retaining our passivity and penetration checks. It does not imply that any
penalty stiffness or energy-conserving timestep produces an accurate trajectory.
Access: primary arXiv abstract, not a fresh full-paper reproduction.
[Primary record](https://arxiv.org/abs/1405.2589).

**Muller and Hélie, DAFx 2017 — Trajectory Anti-aliasing on Guaranteed-passive
Simulation of Nonlinear Physical Systems.** Full PDF retrieved; objectives,
trajectory reconstruction and filtering sections inspected. It explicitly
separates accurate dynamics, power balance and an observation operator that
reduces aliasing. Its demonstration is a nonlinear LC oscillator, not our
hammer. For RF-Rhodes this motivates separate qualification of integration,
magnetic conversion and output filtering. A passive mechanical solver alone
does not provide an alias-free WAV. [Conference paper](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_52.pdf).

**Rhodes service manual, 1979, chapters 2 and 4.** The original manual, available
through an independent archive, documents the modular action, bridle and damper
mechanism, and adjustments including escapement, strike line, hammer-tip
families and pickup position. Choose a specific instrument generation and
regulation before identifying a full keyboard. The research core-impulse
schedule is not a playable action model. Use Rhodes mechanism terminology and
geometry rather than importing an acoustic grand's repetition mechanism.
[Modular action](https://www.fenderrhodes.com/service-manuals/1979/ch2.html),
[adjustment standards](https://www.fenderrhodes.com/service-manuals/1979/ch4.html).

### Open projects: implementation evidence, not acoustic authority

The pinned [Phosphor Rhodes source](https://github.com/joshjetson/phosphor/blob/523f74a0227f7d3604a25ec0f1bc89732d4be11d/crates/phosphor-dsp/src/rhodes.rs)
uses six bounded resonators, ideal cantilever ratios and a partner near the
fundamental. It is a useful Rust cost/organization comparison. Its source
comments and synthetic tests do not establish measured assembly frequencies or
justify copying its partner-mode assignment into our physical matrix.

[OpenWurli](https://github.com/hal0zer0/openwurli) is useful for inspecting an
end-to-end Rust instrument and circuit-validation workflow. The README's fidelity
claims were not independently tested. Its Wurlitzer target has different
resonator/transducer physics, so it is not a Rhodes pickup reference. The
repository declares GPL-3.0; no source was incorporated.

## Audit of the current implementation

These findings are from the local source and retained RF-Rhodes reports. They
are engineering judgments, distinct from the publications' results.

| Area | Assessment | Required evidence or next action |
| --- | --- | --- |
| Six geometry-derived tine modes and tuning mass | Defensible reduced physical model; already stronger than arbitrary harmonic ratios | Check assembled eigenfrequencies, mode shapes and spatial truncation; numerical FE convergence is not experimental validation |
| Root translation/rotation and coupled inertia | Keep reciprocal coupling and consistent force/observation ports | Identify support and tonebar parameters; one effective tonebar coordinate remains an approximation |
| One-plane linear bending | Useful baseline with a known physical limitation | Compare a second polarization and assess shear/rotary/large-deflection effects only over relevant geometry and amplitude ranges |
| Two-mass hammer and Maxwell memory | Plausible research hypothesis with unmeasured internal dynamics | Compare against the existing rate-dependent single-mass contact model before further complexity |
| Passive integration and independent heat/work | Valuable numerical safeguards | Retain them alongside trajectory convergence and sound-output checks |
| Binary spatial dashpot damper | Adequate laboratory switch, incomplete release mechanism | Add a measured or constrained contact/lift law when developing pedal and release behavior |
| Magnetic conversion | Existing scalar pickup paths are useful controls; new mechanics are not connected yet | Qualify geometry sensitivity, intermodulation, output sampling and loading on the new trajectories |
| Calibration and perceptual evidence | Main gap | Fit several notes/intensities and test held-out cases; perform matched-level listening with the same processing path |

The current [tine assumptions](TINE-MODES.md) explicitly exclude shear, rotary
inertia, finite spring width, taper and a second polarization. A uniform beam's
high modes cannot be treated as measured Rhodes partials merely because the FE
eigenproblem converges. Conversely, this review supplies no evidence that a
complete three-dimensional FE simulation is required inside the audio callback.
Spatial preparation and bounded modal runtime remain a sensible division.

### The hammer is the highest-priority hypothesis to challenge

The [two-mass design](MEMORY-HAMMER.md) assigns 3.8 g to the core and 0.2 g
to the tip, links them with the [memory material](HAMMER-MEMORY.md), and adds a
separate stiff contact potential. None of the Rhodes sources inspected here
identifies this mass split, the `2e5 N/m` memory spring, the 1/10 ms relaxation
times or the `1e12 N/m²` surface coefficient.

A local dimensional estimate illustrates why this matters. The reduced mass is
`mu = mc*mt/(mc+mt) = 0.00019 kg`. At zero deformation, treating the Maxwell
state as frozen gives an internal frequency scale
`sqrt(k1/mu)/(2*pi) = 5163.67 Hz`, before nonlinear stiffening or surface contact.
This is a calculation from our assumptions, not a measured Rhodes resonance or
an exact frequency of the full relaxing nonlinear system. It shows that adding
material memory through a separate tip inertia introduces consequential dynamics
that must earn their place through evidence.

The earlier [rate-dependent contact](DISSIPATIVE-HAMMER.md) already provides an
alternative grounded in the contact family used by Pfeifle. It is also
uncalibrated and lacks retained relaxation. Comparing the alternatives does not
require discarding the memory implementation or weakening its tests.

### Numerical status does not yet select the physical model

The [shared recovery](MEMORY-MODAL-RECOVERY.md) passes 20 continuations. The
[shared approach](MEMORY-MODAL-APPROACH.md) still fails one full case, and the
[original repetition study](MEMORY-MODAL-REIMPACT.md) remains failed. Recovery
agreement after restarting equal state at 96 ms does not validate earlier
divergent history. Energy agreement alone has already proved insufficient in
our own experiments.

The extremely fine uniform grids are diagnostic references. They are not an
established physical bandwidth requirement or a viable production sample rate.
Repeated refinements can also accumulate floating-point error; agreement with
one finest supported grid is not an absolute solution. For the difficult case,
compare timestep trends and arithmetic precision in a reduced reproducer,
alongside accepted-step/event histories, before attributing the cause to
material physics. This is a proposed diagnostic, not a diagnosis of roundoff.

Likewise, preserve the combined hammer/structure trajectory test, but add audio
observables as a distinct qualification. A large internal hammer discrepancy
with a small pickup-velocity difference neither proves audible failure nor
justifies accepting an unresolved numerical model.

## Recommended next experiments

### 1. Produce an offline physical audio reference

Render selected single strikes from the new assembly with attack, decay and
damper release. Feed its spatial displacement and velocity to a documented
magnetic law, then to a defined electrical/output filter and WAV writer. Record
raw gain, peak, RMS, model configuration, faults and numerical receipt. Label
the result an exploratory preview until calibrated.

Use a finer observation/oversampling reference and compare the final audio;
do not assume the old engine's 4x decimator qualification transfers to the new
six-mode trajectory. Freeze mechanical trajectories when comparing pickup laws
so contact changes cannot masquerade as transducer improvements. Qualify contact
motion separately. The existing 0.1.2 plugin remains a useful audible baseline.

### 2. Run a controlled hammer comparison

Compare elastic, rate-dependent and memory-contact candidates on the same
structural profile, strike location, pickup and output chain. Start with equal
total launch mass and kinetic energy. Then fit each candidate using the same
training observations and declared parameter bounds; identical uncalibrated
coefficients across different laws would not be a fair fidelity comparison.

Measure contact duration/impulse, restitution, structural energy transfer,
attack spectrum and decay. Test several launch intensities and controlled
inter-strike intervals. Reserve at least one intensity and repetition interval
for validation. Retain memory in the production candidate only if it explains
repeatable held-out behavior or supplies a justified physical requirement at
acceptable cost. Until then, preserve it as a research branch of the model.

### 3. Identify the resonator before expanding it

Fix instrument model/year and recording tap. Begin with A3/A4 and add bass and
treble anchors. Fit fundamental and inharmonic frequencies, early/late losses,
and shared spatial excitation/observation geometry across intensities. Use a
single declared gain rather than an independent gain for each note section.
Retain uncertainty when hammer velocity, pickup gap and gain are confounded.

The acquired G3 library is processed and intensity labels are not hammer-speed
measurements, as already documented in [the earlier review](RESEARCH-2026-09.md)
and the reference inventory. It can support comparison, but cannot uniquely
identify neoprene properties or absolute mechanical scale. Obtaining an untreated
documented set and the JASA modal measurements remains a separate acquisition
task; no author was contacted in this review.

Compare residuals before/after adding a second polarization or more tonebar
detail. Select added physics based on reproducible frequency, decay or spatial
pickup discrepancies, not on coordinate count. Action, damper lift/release and
electrical loading then need their own observable tests for a playable instrument.

## Decision gates and limits

Proceed with an audible research preview and the hammer comparison. Keep
repeated-excitation qualification open; do not promote the current memory model
as a calibrated, realtime Rhodes engine. Avoid further broad micro-optimization
until its useful operating regime and contribution to the output are established.

Acceptance needs three separate results: numerical correctness for the chosen
equations, agreement with instrument observations, and usable runtime behavior.
Set acoustic thresholds using reference repeatability and listening outcomes;
do not invent a universal percentage of realism from test counts.

The full Cantarini 2023 thesis was located through indexed institutional text,
including a Rhodes chapter, but retrieval failed due to size/access restrictions.
No data or conclusions from its unavailable chapter were imported. The JASA
full text and Falaize manuscript also remain access gaps. Recent 2024 work was
found; the targeted search does not establish that no later relevant work exists.

For reproducibility, full PDFs read locally were held in temporary research
storage and are not redistributed in this repository. SHA-256:

- Pfeifle 2017: `fd287f6f5e751f62b0c98a5df42a58de8599902d4e0ea164d9149f6591894301`.
- Sønderbo 2024: `5b8da82aa823cb8e07654b2601717a50ac446f9369fcacfc65a48c86c797f638`.

This review changes documentation only. No DSP parameters, numerical gates,
recorded qualification status, plugin version or executable behavior changed.
