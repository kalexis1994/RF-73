# G3 attack and body residual pilot

Date: 2026-09-04. Production DSP baseline: `d075c05`, research 0.1.1. Reference: five processed mono G3 recordings from Jeffrey Learman's jRhodes3d, author revision `a886e6cebf074c995a10634f82ebe4fdb90f5ca6`. Provenance, hashes, license and processing are in the [reference-bank review](REFERENCE-BANKS.md) and [acquisition inventory](../references/jrhodes-g3.inventory.json).

## Experiment

Render MIDI 55 at 44.1 kHz with normalized model velocities 0.2, 0.5 and 0.9. These are independent model probes, not estimates of the reference hammer speeds. Each is 3 seconds long, released at 2.8 seconds. Compare every probe against every source layer, preserving original pitch and raw levels. File-start anchors are zero; edited reference files do not establish exact physical contact timing. The 96 ms and body observations reduce reliance on the first few samples, but do not solve alignment or recording-chain uncertainty.

The baseline profile uses a 1.5 mm gap and 0.5 mm offset. A separate exploratory sweep compares the strongest reference layer against velocity 0.9 over file times 0..0.6 seconds, with gaps 0.5/0.75/1/1.5/2 mm and offsets 0/0.25/0.5/0.75/1 mm. All 25 candidates are accepted. The lowest windowed spectral error is 7.3568 dB at a 0.5 mm gap and 0.25 mm offset, compared with 8.7878 dB for the baseline geometry. This selects the minimum supported gap and is not an interior optimum or a physical measurement.

Render that exploratory geometry at the same three velocities, then repeat the full comparison matrix. There are 30 compact `compare-tone` reports, each with 32/96 ms attack and 250..600 ms body observations. The [tracked numeric summary](../references/g3-residual-summary.json) preserves all reference and probe ratios, centroids and observation lengths. All six model renders report zero numerical faults. The close-gap 0.9 render has peak 1.1132 in unclipped float audio; any future audition package needs headroom and separate nonlinear aliasing/convergence checks for that geometry.

## Strong-layer residuals

Ratios below are dB relative to each signal's fundamental, measured in the fixed file-relative observations. Positive H3 means the third harmonic is stronger than H1.

| Signal | Attack H2/H1 | Attack H3/H1 | Body H3/H1 |
| --- | --- | --- | --- |
| Reference layer 1 | +2.38 | +7.01 | +2.92 |
| Baseline, velocity 0.9 | -4.94 | -14.89 | -23.05 |
| Close gap, velocity 0.9 | +3.79 | +0.52 | -6.79 |

The baseline third harmonic is 21.90 dB below the reference ratio in the 96 ms attack observation and 25.97 dB below it in the body. Moving the pickup reduces those deficits to 6.49 and 9.71 dB respectively. H2 gets substantially closer at the same time. These are residuals for these particular probes, not errors at measured equivalent hammer velocities.

The broader spectrum matters too: the reference has prominent higher harmonics and additional attack components. No missing harmonic was filled with zero for comparison. The existing broad-band sweep objective includes quiet frequency bins, so its 1.4310 dB improvement must not be read as an equivalent perceptual improvement or proof that the attack is now reproduced.

## Across intensities

| Signal | Attack H3/H1 | Body H3/H1 |
| --- | --- | --- |
| Reference layer 1, strongest | +7.01 | +2.92 |
| Reference layer 2 | -2.50 | -7.30 |
| Reference layer 3 | -13.56 | -18.37 |
| Reference layer 4 | -28.61 | -31.22 |
| Reference layer 5, softest | -41.61 | -41.66 |
| Baseline, velocity 0.2 | -51.05 | -59.37 |
| Baseline, velocity 0.5 | -28.91 | -37.21 |
| Close gap, velocity 0.2 | -34.88 | -43.23 |
| Close gap, velocity 0.5 | -12.50 | -20.93 |

The geometry change shifts the entire velocity response, not just the loud endpoint. A middle/lower model input can resemble a reference ratio while disagreeing in other harmonics, and the source recording gains and strike speeds remain unknown. This matrix does not assign reference layers to model velocities and is not held-out validation: the strongest layer selected the exploratory geometry and all layers were inspected.

## Implementation decision

Retain the current production profile. The first controlled physical follow-up should investigate pickup transfer shape and its excitation scale, using identical mechanical trajectories to separate transducer behavior from hammer/modal changes. Measure harmonic balance and aliasing across displacement amplitude and offset before accepting a closer gap as a new preset. The present experiment establishes sensitivity and a remaining odd-harmonic deficit for the tested strong probe; it does not prove whether the residual originates in the magnetic field law, trajectory, excitation or baked-in EQ.

The sweep tools now report `selection_limits`, marking grid edges, supported-profile bounds and fixed axes. This makes the observed boundary minimum visible in future automated experiments rather than treating it as a confidently located optimum.

## Reproduction

Use fresh output paths. The following reproduces the strong baseline case after the reference WAVs are acquired:

```text
cargo run --locked --release -p rf-rhodes-lab -- render --output renders/g3-baseline.wav --note 55 --velocity 0.9 --sample-rate 44100 --seconds 3 --hold 2.8
cargo run --locked --release -p rf-rhodes-lab -- compare-tone references/audio/jrhodes-g3-a886e6c/A_055__G3_1.wav renders/g3-baseline.wav --note 55 --output renders/g3-tone.json
cargo run --locked --release -p rf-rhodes-lab -- sweep-pickup references/audio/jrhodes-g3-a886e6c/A_055__G3_1.wav --output renders/g3-sweep.json --note 55 --velocity 0.9 --seconds 0.6 --gaps-mm 0.5,0.75,1,1.5,2 --offsets-mm 0,0.25,0.5,0.75,1
```

Render with `--gap-mm 0.5 --offset-mm 0.25` for the exploratory geometry. Repeat model velocities 0.2/0.5/0.9 and source suffixes 1..5 for the full matrix. Generated WAVs, original analyses, the sweep and compact tone reports are under ignored `renders/g3-residual-20260904-224021/`. The original sweep predates the additive `selection_limits` field; its newly generated `loud-pickup-sweep-limits.json` companion verifies that field with otherwise identical results.
