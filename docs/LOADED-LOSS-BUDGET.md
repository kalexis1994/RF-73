# Loaded mechanism loss budget

The [comparison from stationary rest](STATIONARY-REST.md) still decays more
quickly than the processed G3 bank. This study separates physical heat channels
and applies controlled loss changes to identify which mechanisms matter within
the current reduction. It does not fit parameters to a recording.

## Frozen experiment

```text
cargo run --locked --release -p rf-73-lab -- loaded-loss-budget --output references/loaded-loss-budget-validation.json
```

The target remains 196.38614697959488 Hz, frozen from the qualified training
layers. The same mode-tracked spring position on the 70 mm tine is used in every
case. A 1.5 m/s pedestal gesture begins at 30 ms and holds the key through 1.8 s.
The static rest state is identical across all interventions; there is no
post-intervention retuning, gain matching, EQ, envelope correction or state reset.

| Case | Changed parameters |
| --- | --- |
| Baseline | None |
| Half support loss | Translation and rotation viscous coefficients multiplied by 0.5 |
| Half tine loss | All six tine T60 parameters doubled, halving their damping coefficients |
| Half tonebar loss | Tonebar T60 doubled |
| Half damper loss | Felt rate-loss coefficient and arm viscous coefficient multiplied by 0.5 |
| 100k load | Load resistance increased from 10k to 100k ohms |

Every case runs at 128 and 256 internal ticks per 48 kHz output frame. The
midpoint circuit voltage is averaged to 4x and passed through the common FIR
with fixed 0.1 FS/V gain. Four output windows must agree within 1% relative RMS
error. Quiet idle, a single strike, unclipped finite output, qualified timbre
observations and the independent energy gates are required. The whole waveform
matrix is held only in bounded per-case memory; no additional WAV files are saved.

The final timbre window ends before 1.8 s, and the existing three-window pitch
observer has full support. The baseline's fine-resolution timbre must reproduce
the previous 1.5 m/s source-comparison receipt exactly, establishing that the
new observers did not change the waveform prefix.

## Independent structural observer

The coupled model now exposes a read-only copy of its prepared structural
damping matrix. Dynamic stepping is unchanged. The laboratory integrates

```text
Q_g += h * v_mid^T C_g v_mid
```

for support translation, support rotation, tine bending and tonebar bending.
Each group contains both transverse directions and their rotated cross term.
Dropping that cross term would misattribute anisotropic boundary dissipation.
The observer checks symmetry, finite coefficients, positive semidefinite pair
blocks and the assumed pair sparsity before rendering. A future operator with
cross-group damping is rejected instead of silently assigning its heat.

The four observed heats must sum to the solver's independent aggregate
structural ledger within 1e-10 relative to initial energy plus absolute drive
work. Tests compare the sparse observer against the full dense quadratic on
multiple velocity vectors and reject unsupported coupling and invalid matrices.

The report retains twelve channels: the four structural groups, hammer return,
damper arm, four material contacts, coil resistance and electrical load. For
each of five intervals (0–30, 30–300, 300–600, 600–1200 and 1200–1800 ms), it
records heat, heat fractions, stored energy at both ends, actuator work, felt
contact occupancy and the independent balance

```text
E_end - E_start + sum(Q_channel) - W_drive = residual
```

Total and per-window relative energy defects must remain below 1e-8; reciprocal
exchange error must remain below 1e-10. Fractions are withheld when total heat
is below 1e-18 J. This avoids assigning meaningful percentages to numerical idle
noise. Dissipated heat must remain monotone. Failed numerical qualification or
an interrupted case due to a model error retains a failed report with completed
cases and a reason.

## Retained results

All six cases and twelve takes pass. Worst relative total energy defect is
1.170e-12, structural split defect 6.222e-14 and reciprocal exchange defect
7.948e-19. The largest windowed 128/256-tick voltage RMSE is 6.377e-5
(0.006377%), below the fixed 1% gate. All static states are identical and the
baseline's fine timbre profile exactly reproduces the earlier source receipt.

The final timbre window is 1.152–1.664 s after the observed hammer/FIR onset;
levels are relative to the same case's 0.256–0.512 s body window. Positive
changes below mean less late attenuation, not extra absolute output gain.

| Case | Late relative level (dB) | Change from baseline (dB) |
| --- | --- | --- |
| Baseline | -13.94274 | 0 |
| Half support loss | -12.55380 | +1.38894 |
| Half tine loss | -8.03351 | +5.90922 |
| Half tonebar loss | -13.85795 | +0.08479 |
| Half damper loss | -13.68702 | +0.25571 |
| 100k load | -13.73360 | +0.20914 |

The baseline's last energy window, at file time 1.2–1.8 s, dissipates
3.780537e-6 J with zero actuator work. Its heat shares are:

| Channel group | Share of late dissipated heat |
| --- | --- |
| Tine bending | 78.5891% |
| Support translation | 17.1514% |
| Support rotation | 1.5210% |
| Tonebar bending | 1.1197% |
| Coil plus load | 1.6187% |
| Action returns and material contacts | 0% |

There are no felt-contact ticks after 300 ms in any case. Action/contact heat
increments are also zero during those later windows. The damper intervention
therefore changes the transient excitation; its small later level effect is
not evidence of a dragging felt during sustain. In the baseline attack window
(30–300 ms), arm, pedestal and bridle losses are substantial, illustrating why
whole-gesture heat shares would answer a different question.

The half-tine intervention is the strongest tested output sensitivity and the
independent heat ledger also identifies tine damping as the largest late sink.
The next physical calibration should address tine losses first, then support
losses, retaining these controls. Doubling all six modal T60 values is a
diagnostic intervention, not an accepted material profile. It still leaves
the late trajectory below the bank's approximately -5.58 to -2.43 dB range;
spectral, velocity and register calibration remain necessary.

Stored energy in this report includes elastic potential under the held key.
It is not an isolated vibrating-mode energy envelope and should not be fitted
directly to a material decay time.

Receipt: `references/loaded-loss-budget-validation.json`, 352718 bytes,
SHA-256 `8b316a3f1a47555db48f5c7ce7ed5507afe037411d096699f230b8f0225edfcb`.
Verification passes 129 DSP tests, 112 lab unit tests and seven focused loaded
CLI/receipt checks, including exact baseline timbre preservation. Strict Clippy
and formatting pass. Release cache stays near 194 MiB with incremental builds
disabled; no new WAV, source-bank copy or plugin package is created.

## Interpretation limits

Heat fractions describe the actual trajectory in a given case. Changing losses
also changes the attack, modal participation and future motion, so differences
between interventions are not additive causal percentages. The damper control
changes two dissipative parameters together. The load control changes the
electrical transfer as well as energy loss. Neither is a unique parameter fit.

The processed bank remains useful as an output reference, but its unknown EQ
and noise reduction prevent treating relative voltage decay as a direct material
T60 measurement. The study covers one pitch and one prescribed key speed. It
does not certify a calibrated keyboard, soft-action response, static magnetic
bias, gravity or real-time polyphonic performance.
