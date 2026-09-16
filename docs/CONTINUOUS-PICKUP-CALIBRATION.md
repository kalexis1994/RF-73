# Continuous-disk pickup calibration against Matt's Rhodes

Date: 2026-09-15. **The continuous-disk candidate is not promoted.** It improves
aggregate harmonic error but introduces a large B2/mp H2 deficit, and still
regresses E4 in the development set. The playable plugin remains unchanged.

## Implemented block

The new offline `continuous_pickup_fit` runner fits the boundary-integrated
uniform-disk pickup from the [quadrature diagnostic](APERTURE-QUADRATURE-DIAGNOSTIC.md).
It uses an 8,193-entry linear slope table over ±10 mm to avoid evaluating 128
boundary nodes at every internal sample. An out-of-domain trajectory is an
error; it is never clamped into a valid signal. The existing voice mechanics
and 4× production filter remain identical.

The old `matts_fit` scorer now accepts a rendering function, so both experiments
share the same onset, spectral, missing-data and envelope measurements. Across
all 24 repeated baseline cases, harmonic MSE is exactly unchanged after this
refactor. No audio files or plugin presets were modified.

## Protocol

- Development: MIDI 43/50/55/59/64/72, all four layers (24 cases). These notes
  had already been examined during earlier blocks.
- Held-out: MIDI 40/47/62/69/79/88, all four layers (24 cases), reserved in the
  preceding diagnostic and loaded only after the winner was frozen.
- Search: three fixed starting points, followed by three coordinate rounds
  with logarithmic steps 0.4/0.2/0.1: **33 evaluated parameter sets**.
- Variables: gap, lateral offset, disk radius, maximum hammer speed and velocity
  exponent. Decay and remaining mechanical parameters stay fixed.
- Objective: existing mean per-case H2–H4 squared dB balance error in attack96
  and body350 windows, relative to H1, with a -60 dB floor. This is not a
  perceptual quality score. The source's physical strike speeds remain unknown.
- Acceptance: the prior aggregate, per-held-out-note, coverage and envelope
  gates, plus a numerical table/refinement check before held-out evaluation.

## Selected parameters and measured outcome

| Parameter | Continuous candidate |
| --- | ---: |
| Gap | 0.818731 mm |
| Lateral offset | 0.750000 mm |
| Pole radius | 0.670320 mm |
| Maximum hammer speed | 1.4 m/s |
| Velocity exponent | 1.4 |

These are fitted model parameters, not measured dimensions or hammer speeds.

| Split | Current Calibrated MSE (dB²) | Continuous candidate | Reduction |
| --- | ---: | ---: | ---: |
| Development | 172.0744 | 114.5544 | 33.43% |
| Held-out | 194.4837 | 133.3186 | 31.45% |

All 48 cases have measurable objective terms. This indicates case coverage,
not that every H2–H4 term was available in every window.

| Held-out note | Current MSE | Candidate MSE |
| --- | ---: | ---: |
| E2, MIDI 40 | 92.83 | 45.64 |
| B2, MIDI 47 | 98.76 | **148.11** |
| D4, MIDI 62 | 85.24 | 58.86 |
| A4, MIDI 69 | 203.68 | 176.13 |
| G5, MIDI 79 | 243.77 | 194.68 |
| E6, MIDI 88 | 442.62 | 176.49 |

B2 regresses by approximately 50%, exceeding the allowed 10%. Its mp layer
accounts for most of the loss: body H2 is -10.39 dB in the recording, -14.70 dB
in Calibrated, and **-39.96 dB** in the continuous candidate. Thus an even-harmonic
deficit can also arise in the continuous law; the old 16-node approximation is
not the only source of such behavior.

The development objective also hides a regression in E4: its mean error rises
from 74.03 to 141.24 dB², despite improvement in the overall development mean.
This is a reason to constrain development note/layer regressions during the
next selection, rather than repeatedly favoring large aggregate gains.

Ten held-out cases support qualified source/baseline/candidate envelopes. Their
mean absolute slope error increases by 0.0272 dB/s, below the 0.5 dB/s ceiling;
no new envelope failure appears among previously eligible cases. Passing these
limited checks does not establish full-keyboard natural decay or note-off timing.

## Numerical qualification

At the selected geometry, 2,001 off-grid displacement probes compared with a
256-node direct boundary integral yield maximum table error / peak slope of
4.068e-6 (gate: 1e-4). The 128-to-256-node refinement error / peak slope is
2.014e-15 (gate: 1e-6).

A separate frozen-winner check renders soft and strong strikes at MIDI
40/55/64/88 using both the table and direct 256-node integration. For these
eight cases, the largest waveform relative RMS error is **2.283e-6**, and the
largest measured H2–H4 balance difference is **0.000609 dB**. Waveform errors use
no alignment or gain fitting; harmonic comparisons use the usual onset windows.
These differences are far below the observed source mismatches. Numerical
qualification is case-specific and is not a realtime CPU or aliasing guarantee.

## Reproduction

From the RF-73 workspace in PowerShell; output directories must be new:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$env:PYTHONDONTWRITEBYTECODE = '1'
$runner = 'references/matts-reference-runner/Cargo.toml'
$samples = 'references/audio/matts-fender-rhodes/samples/original'
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- $samples renders/matts-reference/continuous-new
python tools/matts_fit_gate.py renders/matts-reference/continuous-new renders/matts-reference/continuous-new/decision.json
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- $samples renders/matts-reference/continuous-check-new renders/matts-reference/continuous-new/search.json
```

The optional frozen-search argument performs only direct-versus-table rendering
checks; it does not rerun or tune the optimizer. The numerical checks are
additional evidence, separate from the existing Python promotion decision.

[Retained receipts](../references/continuous-pickup-fit-2026-09-15/) include the
protocol, all trial parameters, split observations, gate decision, numerical
rendering check and provenance. All synthesis is in memory; no redundant WAV
matrix is generated. Validation: 20 Rust tests and six Python tests pass;
release Clippy passes with warnings denied.

## Next block

Completed follow-up: [constraint-first calibration](ROBUST-PICKUP-CALIBRATION.md)
adds development-note and critical-layer constraints and runs 147 bounded
evaluations. No candidate passes; the new held-out split remains unused.

Keep this as an offline research path. Constrain regressions within individual
development notes and layers, especially E4/f and B2/mp, before accepting the
next aggregate optimum. Do not use the now-inspected validation notes as fresh
holdouts again. Reserve a new split before any subsequent tuning. No playable
version was produced, so the standard RackForge audition was not run.
