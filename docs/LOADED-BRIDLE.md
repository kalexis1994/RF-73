# Loaded bridle, damper lift and key return

The [soft-strike work study](LOADED-STRIKE-THRESHOLD.md) found that most of the
hammer's received work crosses the bridle port before a weak impact. This block
separates that transfer into bridle storage/heat and arm motion, then checks
whether changes still lift the felt and return after key release.

```text
cargo run --locked --release -p rf-73-lab -- loaded-bridle --output references/loaded-bridle-validation.json
```

## Frozen controls and gesture

Five configurations retain the conditional 70 mm G3 tine, spring target,
contact laws, pickup and circuit from the previous block:

| Case | Changed setting | Baseline |
| --- | --- | --- |
| Baseline | None | Existing conditional profile |
| Bridle slack | 4 mm | 2 mm |
| Bridle ratio | 0.6 | 0.8 |
| Arm stiffness | 100 N/m | 200 N/m |
| Arm damping | 0.25 N s/m | 0.5 N s/m |

Each uses pedestal speeds 1, 1.125 and 1.5 m/s, rendered at 128 and 256 ticks
per 48 kHz observation frame: thirty 400 ms takes. The key moves down from
30 ms, stays held until 150 ms, then returns at the same bounded slew rate.
The pedal remains closed. There are no state resets at release. Every profile
prepares its own stationary rest; changed arm stiffness also changes preload.

Snapshots at 120, 150, 220 and 400 ms retain both work ledgers and contact
counts. The original profile's 120 ms hammer snapshot and first-contact state
must reproduce the previous threshold study exactly. This separates new
release observations from changes to the existing strike trajectory.

## Signed energy transfer

The observer retains the independent hammer ledger and adds three identities:

- Hammer work into the bridle minus work delivered by the bridle to the arm
  equals the change in bridle potential plus bridle heat.
- Arm kinetic/spring energy change equals work received from the bridle and
  pedal minus work delivered to felt contact and arm viscous heat.
- Arm work into felt contact minus work delivered to the structure equals
  the change in felt-contact potential plus felt heat.

Every balance includes the prepared initial energy, including felt preload.
The bridle's arm force is negative in the coordinate convention: power into
the arm is minus bridle force times arm velocity. The contact compression
identity supplies structural felt-port displacement independently. Return heat
and pedal work are also integrated separately and compared with the solver.
The latter is zero in this fixed-pedal study. Port work is signed; energy can
return from the arm/bridle and must not be counted as irrecoverable loss.

## Separate numerical and functional checks

Relative total, hammer and port-energy defects must remain below 1e-8;
exchange and independently integrated arm-heat/pedal-work defects stay below
1e-10. All heat remains monotone and pre-key raw voltage below 1e-9 V.
Coarse/fine hammer-contact counts agree. Positive impacts require less than
1% impulse, peak-force and duration error; no-contact pairs require exact zero.
Hammer, arm and both pickup velocities each require less than 1% RMS error
in 30–150, 150–220 and 220–400 ms, with a 1e-8 m/s reference RMS floor.

Functional observations remain separate from numerical qualification:

- Exactly one hammer contact in the gesture.
- At least 0.1 mm felt clearance and zero felt contact throughout 80–140 ms.
- Throughout 350–400 ms, hammer and arm positions within 0.1 mm of their
  own prepared rest and absolute speeds below 0.01 m/s; felt contact present
  for at least 90% of solver ticks in that window.

These are declared engineering diagnostics, not measured instrument regulation
tolerances. No-contact or failed lift/return is retained even if integration
converges. No candidate is selected or automatically adopted.

## Scope

The study tests one key gesture with a fixed pedal in a provisional reduction.
It does not calibrate linkage geometry, prove half-pedal response or establish
long sustain/repetition behavior. Improving a weak strike by reducing load is
insufficient if felt lift or return fails. Other registers, source timbre,
listening and realtime integration remain open. No WAV matrix is generated.

## Retained outcome

All thirty takes and fifteen refinement pairs qualify numerically. All fifteen
cases lift the felt and restore felt contact, but none meets the complete
functional criterion: the hammer has not settled sufficiently in 350–400 ms.
This is a failed settling diagnostic under declared limits, not divergent
integration or proof of a measured instrument defect. No candidate is adopted.

| Configuration | Hammer contact counts at 1 / 1.125 / 1.5 m/s | Pre-impact hammer speed at 1.125 m/s | Maximum return hammer offset across speeds |
| --- | --- | --- | --- |
| Baseline | 0 / 1 / 1 | 0.106164 m/s | 1.395 mm |
| Slack 4 mm | 1 / 0 / 1 | No contact | 2.703 mm |
| Ratio 0.6 | 0 / 0 / 1 | No contact | 2.585 mm |
| Half arm stiffness | 1 / 1 / 1 | 0.829339 m/s | 1.992 mm |
| Half arm damping | 1 / 1 / 1 | 0.690298 m/s | 1.914 mm |

The slack intervention's contact/non-contact/contact sequence occurs at both
resolutions. More slack does not produce a monotone drive-to-impact curve in
this reduction. Lower arm stiffness or damping enables contact at all three
sampled speeds, but also changes arm motion and the return trajectory.
Reducing the ratio removes the formerly weak strike at 1.125 m/s; a simple
"less load gives a better soft strike" rule does not describe this matrix.

Held felt clearance is at least 3.099926 mm across the full matrix. During the
return check the felt is in contact for 100% of solver ticks in every take.
Arm position error stays below 0.016862 mm and arm speed below 0.002804 m/s,
both within the prescribed arm limits. The hammer alone fails: fine maximum
position errors span 1.389–2.703 mm and maximum speeds 0.0540–0.0733 m/s.
Separating hammer and arm return avoids misdiagnosing this as failure to lower
the felt.

For the baseline at drive 1.125 m/s, the hammer is nearly stationary against
the held pedestal at 150 ms. At 220 ms it is at -9.611470 mm, moving positively
at 0.123132 m/s, with both pedestal and bridle force zero. At 400 ms it is at
-10.605847 mm versus -12 mm rest, moving positively at 0.025428 m/s; those two
ports are still unloaded. These samples expose residual hammer motion after
release. They do not locate every intervening collision or determine a unique
settling-time constant.

## Resolving the earlier bridle work

Immediately before the original 1.125 m/s impact:

| Bridle/arm quantity | Value (mJ) |
| --- | --- |
| Work from hammer into bridle | 9.100461 |
| Work delivered from bridle to arm | 8.371911 |
| Bridle elastic potential | 0.112714 |
| Bridle dissipated heat | 0.615837 |
| Arm kinetic plus spring energy, including initial preload | 5.397579 |
| Arm dissipated heat | 2.975780 |

Most work entering the bridle is delivered to the arm; the bridle's own
stored energy and heat account for the remainder. The arm then holds elastic/
kinetic energy and dissipates energy through its viscous coefficient. Its
initial preload and small signed felt-port work remain in the full ledger,
so stored arm energy must not be summed as though its initial value were zero.

By 400 ms, cumulative work from hammer to bridle has fallen to 6.318012 mJ,
bridle potential is zero and bridle heat is 0.857931 mJ. The arm is close to its
initial energy; arm heat has grown to 5.423636 mJ. This resolves the earlier
large port transfer into storage, dissipation and energy returned through the
coupled system, without identifying an actual instrument's material constants.

At drive 1.125 m/s, halving arm stiffness reduces pre-impact hammer-to-bridle
work to 6.825632 mJ and raises hammer speed to 0.829339 m/s. Halving arm damping
gives 7.912864 mJ and 0.690298 m/s. Both preserve held lift in this test; neither
solves the hammer return criterion. These are diagnostic changes, not selected
voicings or an established improvement in source timbre.

## Verification and next block

Maximum relative total energy defect is 1.741e-12. Independent hammer work
defects stay below 1.262e-12, bridle transfer below 1.391e-12, arm energy below
1.137e-12 and felt transfer below 1.223e-14. Independent arm heat agrees exactly
with the solver in this matrix, and both pedal-work ledgers are exactly zero.
Maximum exchange defect is 7.989e-19. Maximum windowed velocity RMSE is
0.010950%; maximum impact refinement error is 0.042589%.

The original configuration's 120 ms hammer snapshots and pre-impact states
exactly reproduce the preceding threshold study. Tests independently
reconstruct every retained bridle/arm/felt work identity, check functional
decisions from the underlying measurements, and preserve the failed hammer
return and non-monotone slack control. Numerical acceptance is kept separate
from functional failure.

Verification passes 122 lab unit tests, seventeen loaded CLI/receipt tests,
strict Clippy and formatting. The [receipt](../references/loaded-bridle-validation.json)
is 664379 bytes, SHA-256
`045b803673f7a9f45b7bac1632701792dc8fac24a34b2c515a3cff911f0c4365`.
The release cache remains approximately 194 MiB. No additional audio files or
production defaults are introduced.

The [hammer return study](LOADED-HAMMER-RETURN.md) now traces hammer return
and pedestal contact over a longer tail, comparing return damping and contact
loss controls with explicit attack and felt-lift checks.
Qualify repetition from the resulting mechanical state before adopting a
parameter change. The present fixed-pedal test does not establish a half-pedal
model or a complete playable action.
