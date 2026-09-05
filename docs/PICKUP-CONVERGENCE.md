# Pickup convergence across registers

`converge-pickup` evaluates both pickup laws with the production contact rule and independent, denser mechanical integrations. A separate path samples the highest-rate trajectory at production endpoints to distinguish integration differences from pickup/filter sampling differences. It builds on [mechanical pickup pairs](PICKUP-PAIR.md), without changing the plugin.

## Reproduce

```text
cargo run --locked --release -p rf-rhodes-lab -- converge-pickup --output renders/g3-pickup-convergence.json --note 55 --velocity 0.9 --gap-mm 0.5 --offset-mm 0.25
cargo run --locked --release -p rf-rhodes-lab -- converge-pickup --output renders/treble-pickup-convergence.json --note 100 --velocity 0.2 --sample-rate 48000 --gap-mm 0.5 --offset-mm 0.25 --reference-steps 256
```

Defaults are MIDI 55, velocity 0.9, 44.1 kHz, 0.1 seconds, gap 1.5 mm, offset 0.5 mm and a 128x reference. Supported output rates are 44.1/48/96/192 kHz, notes 28..100, velocities 0.01..1 and durations 0.05..0.25 seconds. Geometry uses the validated profile limits. `--reference-steps` accepts 128 or 256. Every take is a single strike held throughout; the command writes a new JSON report and refuses existing files. There is no reference recording or audio export.

## Controlled comparisons

The production row uses `Voice::new` with 4x pickup sampling and prepared contact subdivision. Other independent rows use `Voice::new_for_convergence` at 8/16/32/64x, plus 128x when the reference is 256x. Each independent voice integrates its own contact and free motion. Both magnetic laws see the same position and velocity within each path.

The frozen-trajectory row retains every `reference_steps / 4`-th state from the reference voice, at exactly the 4x endpoints. It changes neither contact integration nor trajectory interpolation: there is no interpolation. Its mechanical comparison fields are null because it does not integrate an independent voice.

The 4x paths use `ProductionDecimator` directly. Denser paths sample the same physical Blackman-windowed sinc kernel with cutoff `0.42 * output_rate`, normalize DC gain and retain a 31.5-output-sample support. Their tap count is `126 * steps / 4 + 1`; group delay is 15.75 output samples at every density. Output is read after each complete internal-step group. Every filter starts with zero history, without delay compensation, gain fitting or alignment. All paths apply the same `filtered * 0.7 * 0.12` in f64; output quantization is excluded.

This differs deliberately from the original [contact convergence command](CONVERGENCE.md), which uses a longer common low-pass kernel and a default-profile 64x reference. Its existing behavior is retained.

Reports include raw NRMSE, raw RMS, signed level change and peak over the full duration, the first `floor(rate * 0.032)` frames, and the remaining frames. NRMSE is `sqrt(sum((candidate-reference)^2) / sum(reference^2))`; a reference energy at or below 1e-30 produces null. Mechanical displacement/velocity errors use common output-frame endpoints. Peak displacement/speed, positive energy increments and first separation are measured at every internal tick. Separation time is quantized to that path's tick interval.

The frozen residual includes nonlinear transfer sampling **and discrete filter-kernel sampling**. Independent-row residuals also include mechanical integration differences. Neither is a pure aliasing bound. A finite dense reference must be checked against its next-lower independent row; close agreement is numerical evidence, not proof of convergence to the true physical instrument. These reports do not qualify release, retriggers, polyphony, CPU deadlines or perceptual realism.

## Register matrix

Date: 2026-09-04. Eighteen 100 ms observations initially use a 128x reference: MIDI 28/55/100 at velocities 0.2/0.9, default and close geometry, at 44.1 kHz; plus close-geometry MIDI 100 at both velocities and 48/96/192 kHz. Close geometry is gap 0.5 mm/offset 0.25 mm; default is 1.5/0.5 mm.

The following table shows the point-pole law at close geometry and 44.1 kHz. All errors are percentages of reference RMS, not dB or perceptual scores.

| MIDI note | Velocity | Production 4x attack error % | Independent 64x attack error % | Frozen trajectory 4x attack error % |
| --- | --- | --- | --- | --- |
| 28 | 0.2 | 0.005136 | 0.000015 | 0.00000040 |
| 28 | 0.9 | 0.043234 | 0.000127 | 0.00000438 |
| 55 | 0.2 | 0.020861 | 0.000061 | 0.00000074 |
| 55 | 0.9 | 0.078071 | 0.000230 | 0.00000832 |
| 100 | 0.2 | 0.036837 | 0.036837 | 0.00002800 |
| 100 | 0.9 | 0.002769 | 0.002769 | 0.00002802 |

The treble rows require care: at 44.1 kHz, production uses 16 contact microsteps per 4x tick, reaching the same contact resolution as independent 64x. Similar production and 64x residuals therefore do not make the 128x reference exact. Five additional reports use 256x: close-geometry soft MIDI 100 at all four rates, and its strong 44.1 kHz strike.

For soft MIDI 100, velocity 0.2, close geometry, against 256x:

| Output rate | Production attack error % | Production full error % | Independent 64x attack error % | Independent 128x attack error % |
| --- | --- | --- | --- | --- |
| 44,100 | 0.046044 | 0.049488 | 0.046044 | 0.009209 |
| 48,000 | 0.163997 | 0.175775 | 0.039048 | 0.007810 |
| 96,000 | 0.166069 | 0.177893 | 0.009769 | 0.001954 |
| 192,000 | 0.166644 | 0.178419 | 0.002444 | 0.000489 |

Independent 128x residuals are approximately one fifth of the independent 64x residuals against 256x in these cases. This supports continued refinement; it is not a formal convergence-order estimate. The strong 44.1 kHz confirmation has 0.003461% production attack error and 0.000692% independent 128x error. Frozen-path attack residuals remain at or below 0.00002803% across these confirmations.

The small frozen residual compared with the production residual indicates that mechanical/contact discretization dominates these tested treble errors. Increasing the continuous pickup rate alone is not justified by this matrix. This does not exclude aliasing outside the tested strikes or finite production-filter stopband leakage.

All 23 reports have finite output and separated hammer contact. The largest positive mechanical energy increment across their independent paths is 4.34e-19 J, within the numerical passivity tolerance. The largest observed point-pole single-note peak is approximately 2.8661 at close geometry, MIDI 100/velocity 0.9/192 kHz. The experiment retains the arbitrary common electrical scale; headroom still needs attention before an audition profile.

The complete numeric matrix and source-report hashes are tracked in `references/pickup-convergence-summary.json`. Full reports are ignored artifacts in `renders/pickup-convergence-20260904-232659/`. The first 18 reports predate additive stride metadata but contain the same numeric method. Reproduce their default 128x cases and the five 256x confirmations with the flags above and fresh output paths.

The production plugin remains at the 0.1.1 research profile and unchanged processing rate. Next work should establish candidate gain/headroom and level-matched listening comparisons while retaining these convergence diagnostics. The current results establish numerical behavior for selected strikes, not realistic instrument timbre or a whole-keyboard error bound.
