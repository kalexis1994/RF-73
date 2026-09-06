# What limits the memory-contact timestep

The contact estimator now has a read-only inspection API and a Rust laboratory
sweep. This provides evidence for the next integrator experiment without
changing the existing physical law, `1e-11` state tolerance or accepted path.

## Inspection contract

`MemoryModalAssembly::inspect_contact_step(level)` evaluates exactly the same
prepared contact trial as `try_contact_step`. It returns the prospective
acceptance result and, when the trial reaches a finite state estimate, seven
squared-error terms plus the energy normalization scale. It never commits the
trial. Invalid levels remain errors; boundary and physical-check rejections
without a usable metric return absent terms rather than zeros.

The terms, in report order, are:

1. Structural velocity error using the full reciprocal mass matrix.
2. Structural displacement error using the full stiffness matrix.
3. Hammer core velocity error weighted by core mass.
4. Hammer tip velocity error weighted by tip mass.
5. Material-memory extension and core/tip position errors weighted by memory stiffness.
6. Material deformation error weighted by the equilibrium tangent.
7. Relative tip/surface gap error weighted by the contact tangent.

These are contributions to a **squared state-error metric**, in energy units.
They are not stored energy, heat, acoustic error or independent physical modes.
In particular, term 5 includes the estimator's core/tip position weights; it
does not measure only Maxwell branch error. Off-diagonal structural mass terms
are retained. The normalized estimate is `sqrt(sum(terms)/(2*scale))`, with the
same summation order and scale as before.

Trial storage is fixed-size, and inspection allocates nothing in DSP. It runs
the full coarse/two-half-step solves and physical checks, so it is intended for
offline diagnostics rather than routine audio metering. The new test verifies
accepted and rejected inspections, invalid levels, stationary boundaries,
damper transitions, error reconstruction, unchanged physical state, and exact
agreement with subsequent committed steps and base ticks.

## Reproduction and scope

```text
cargo run --locked --release -p rf-73-lab -- memory-contact-resolution --output renders/contact-resolution.json
```

The [retained sweep](../references/memory-contact-resolution.json) covers the
same 12 provisional length/speed/relaxation profiles as the modal audit. It uses
the economical controller at the original 16672 ticks per 48 kHz observation,
an impulse at 2 ms, damper engagement at 4 ms and disengagement at 6 ms.

Each profile contributes 44 snapshots: observation-frame starts 0–15, 96–111,
192–195 and 288–291, plus tick 1600 of frames 0, 96, 192 and 288. External events
apply before their snapshots. The extra observations split the controller's
available interval; that can change subdivision compared with the previous
audit. The inspection itself does not advance or change the state. All 12
dyadic levels are tried from each snapshot, spanning about 2.499 ns to 5.118 us.
No trial crosses the next external event.

The sweep verifies state preservation, finite nonnegative metric terms, exact
reconstruction of each available scalar estimate, and trajectory energy balance.
It is a diagnostic, not an independent fine-reference accuracy audit. Global
relative energy residual is at most `1.093e-10` in these trajectories.

## Observations

The 528 snapshots produce 6,336 read-only trials: 288 would be accepted, 1,075
require refinement for accuracy/physical checks and 4,973 fail the continuous
compression certificate. Rejected trials are not committed.

For each snapshot that has a state-error rejection, select the smallest tested
level whose finite error exceeds `1e-11`. This gives 162 observations. The
largest individual squared-error term is distributed as follows:

| Largest metric term | Snapshots |
| --- | ---: |
| Surface contact | 87 |
| Structural kinetic | 52 |
| Structural elastic | 10 |
| Hammer tip kinetic | 12 |
| Equilibrium material | 1 |

The first state-limited intervals range from about 2.499 to 19.994 ns. The
largest term is not necessarily a majority of the metric: its fraction can be
as low as 27.6%. These sampled snapshots are not independent experiments or a
population estimate for the keyboard. A missing state-error rejection at the
other snapshots does not establish that large contact steps are safe there;
many are outside continuous compression.

Of the 162 selected observations, 159 have usable adjacent-level error ratios.
Their `log2(error(h)/error(h/2))` lies between 2.983 and 3.031. Thus the observed
coarse/fine error grows approximately eightfold when the interval doubles.
This is consistent with temporal truncation of the existing second-order
scheme, rather than root-solver roundoff dominating these particular estimates.
It is an inference from local scaling, not an order proof over contact events
or the whole trajectory. The report omits ratios with missing or <=`1e-14`
estimates; that floor is only for the diagnostic ratio, never for acceptance.

The next substantial experiment should improve temporal accuracy through the
coupled surface/structure dynamics. A higher-order contact candidate must retain
independent heat and reciprocal work checks, bounded solves, event boundaries
and comparison with the current fine reference. A wall experiment can establish
its material/contact behavior before coupling all nine structural coordinates.
These observations do not justify softening provisional contact coefficients,
discarding higher modes or relaxing the existing tolerance.

## Regression and limitations

The [strict-controller regression audit](../references/memory-contact-inspection-control.json)
passes all 12 cases/24 takes and remains byte-identical to the pre-inspection
report: SHA256 `D209D20B9F6D8E73581FB1157BF66B55A7F2C1B372A64A20C7EAF156DD2123AB`.
All 173 workspace tests, strict Clippy, formatting and release WASM compilation
pass. CLI help, invalid-argument rejection and existing-report preservation
were checked. Remote CI was not run locally.
The new diagnostic is included in CI. Physical calibration, integration into
the audible plugin, longer trajectories and realtime qualification remain open.
This change makes no runtime improvement claim. No GUI/audio test was performed.
