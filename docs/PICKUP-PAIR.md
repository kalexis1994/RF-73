# Pickup laws on the same mechanical trajectory

`render-pickup-pair` runs one production voice and sends each internal-rate displacement/velocity pair through the production pickup law and the experimental point-pole proxy. Both outputs use the actual production decimator, initial filter state, sampling phase and default gain. This extends the [periodic transfer experiment](PICKUP-TRANSFER.md) to contact, modal attack, decay and release.

The alternative remains an offline experiment. The plugin uses the original production transfer and profile.

## Reproduce

```text
cargo run --locked --release -p rf-rhodes-lab -- render-pickup-pair --output renders/g3-pair.wav --note 55 --velocity 0.9 --sample-rate 44100 --seconds 3 --hold 2.8 --gap-mm 0.5 --offset-mm 0.25
cargo run --locked --release -p rf-rhodes-lab -- compare-tone references/audio/jrhodes-g3-a886e6c/A_055__G3_1.wav renders/g3-pair-point-pole.wav --note 55 --output renders/g3-pair-reference.json
```

The first command writes:

- `g3-pair.wav`: the production signal, sample-identical to a regular single-note render with these parameters.
- `g3-pair-point-pole.wav`: the experimental signal at the same raw gain, without clipping or normalization.
- `g3-pair-pickup-pair.json`: mechanical diagnostics, both raw levels and three attack/body tone comparisons, with production as reference.

All normal render options except `--trace` apply. Duration is restricted to 0.65..10 seconds; key hold must be at least 0.6 seconds and shorter than duration, keeping the comparison windows before release. Supported rates, note range, velocity and pickup geometry retain the renderer's validation. Invalid input, failed numerical checks or an existing output prevents writing a new pair. Concurrent creation or an I/O failure can leave a partial set; no existing file is overwritten. A JSON receipt is written only after both WAVs finish successfully.

`ProductionDecimator` exposes the same Rust FIR implementation used by the engine: 127 Blackman-windowed taps, cutoff 0.42 times output rate, group delay 63 internal samples. The pair pushes four internal samples before each output and keeps the delay. There is no interpolated trajectory, duplicate mechanical simulation or substitute filter. Both signals use `filtered * 0.7 * 0.12` followed by the same f32 conversion as the default engine.

The report records maximum displacement and speed at every internal sample, first separation time, contact impulse and final mechanical energy. Peak forces are averages over a production tick when contact subdivision is active. The suggested whole-clip RMS matching gain is informational and is not applied. It includes the full rendered tail; it is not a fitted capture gain or a loudness calibration.

## G3 reference experiment

Date: 2026-09-04. Six three-second pairs use MIDI 55 at 44.1 kHz, release at 2.8 seconds, model velocities 0.2/0.5/0.9 and two geometries: default 1.5/0.5 mm and exploratory close 0.5/0.25 mm. Every production WAV has exactly the same SHA-256 hash as the corresponding [preceding pilot](G3-RESIDUAL-PILOT.md). Thirty reference comparisons cover all five acquired layers against all six point-pole renders.

All twelve WAVs contain finite output with zero reported faults. The two geometries produce exactly equal mechanical diagnostics at each velocity, as expected from this one-way pickup model. Peak tine displacement is 0.09387/0.33669/0.76515 mm and peak speed is 0.13946/0.50586/1.15394 m/s. These are actual model trajectories, much smaller than the earlier 52 m/s synthetic aliasing stress case.

Point-pole H3/H1 and its change relative to the production law are:

| Geometry | Velocity | 96 ms attack dB | Change dB | 250..600 ms body dB | Change dB |
| --- | --- | --- | --- | --- | --- |
| Default | 0.2 | -47.33 | +3.72 | -55.66 | +3.72 |
| Default | 0.5 | -25.12 | +3.79 | -33.47 | +3.74 |
| Default | 0.9 | -10.81 | +4.08 | -19.19 | +3.86 |
| Close | 0.2 | -32.27 | +2.61 | -40.69 | +2.54 |
| Close | 0.5 | -8.96 | +3.53 | -17.95 | +2.98 |
| Close | 0.9 | +5.50 | +4.98 | -2.72 | +4.07 |

Against the strongest source layer, the close/0.9 H3 deficit decreases from 6.49 to 1.51 dB in the 96 ms attack and from 9.71 to 5.65 dB in the body. This is a controlled transfer-law effect for the chosen model strike, not evidence that source and model hammer speeds match. The broader spectrum still has substantial residuals:

| Component | Point-pole minus strongest reference, 96 ms dB | Body dB |
| --- | --- | --- |
| H2/H1 | +2.48 | -1.37 |
| H3/H1 | -1.51 | -5.65 |
| H4/H1 | -1.78 | -14.02 |
| H5/H1 | -1.33 | -5.70 |
| H6/H1 | -11.21 | -16.12 |

The point-pole output is also louder at the common arbitrary electrical scale. Default-geometry peaks are 0.09089/0.33769/0.79771; close-geometry peaks are 0.30118/1.08239/2.41225. Close medium/loud float WAVs exceed unity and remain unclipped. The close/0.9 whole-clip RMS matching multiplier is 0.47451; applying this for a listening comparison would not change Hn/H1. No gain or geometry from this experiment was promoted to the plugin.

Full numeric coverage, including unavailable harmonics as null, is tracked in `references/g3-pickup-pair-summary.json`: six paired observations and thirty reference comparisons, with WAV hashes. Ignored raw WAVs and full JSON reports are under `renders/pickup-pair-20260904-231302/`. Repeat the commands above for model velocities 0.2/0.5/0.9, the two geometries and reference suffixes 1..5 to reproduce the matrix.

## Interpretation and next decision

The point-pole proxy is worth further investigation because the third-harmonic effect survives the complete production trajectory and filter. It does not resolve the remaining higher-harmonic and body discrepancies. The source bank has baked-in processing, unknown capture gain and no physical strike calibration; all five layers have been inspected, so this is not held-out validation. The close gap is still at the supported profile minimum.

These pairs include actual FIR filtering but do not compare against a high-rate mechanical or magnetic reference. They cannot establish an absolute aliasing bound. Before a sound-profile release, evaluate the candidate across registers and against a converged path, quantify gain/headroom, and compare level-matched listening examples. Keep the two-dimensional magnetic geometry, attack modes and reference processing as competing explanations for the remaining residuals.
