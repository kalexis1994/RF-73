# Pickup listening and observed headroom

`pickup-listening` prepares three versions of the same 24-second performance and measures full-keyboard/stress peaks. The purpose is to listen to the candidate without its larger raw gain deciding the comparison. It follows the [convergence study](PICKUP-CONVERGENCE.md); it does not select a new plugin profile.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-listening --output renders/pickup-listening
cargo run --locked --release -p rf-73-lab -- pickup-listening --output renders/pickup-listening-192k --sample-rate 192000 --measure-only
```

The output directory must not exist. Defaults are 44.1 kHz, candidate gap 0.5 mm/offset 0.25 mm and a -6 dBFS sample ceiling. `--sample-rate` accepts 44.1/48/96/192 kHz. Candidate geometry retains the profile's validated ranges. `--ceiling-dbfs` accepts -24..-1 dBFS. `--measure-only` computes the complete study and writes only `report.json`, avoiding additional WAV storage.

Outputs use this order:

1. `current.wav`: production transfer at the current default 1.5/0.5 mm geometry.
2. `close-original.wav`: production transfer at the candidate geometry.
3. `close-point-pole.wav`: experimental transfer at that same candidate geometry.

The `close-*` filenames describe the default close geometry; custom geometry is recorded explicitly in the receipt. Current versus close-original reveals the geometry change; close-original versus close-point-pole reveals the transfer-law change. These are labeled exploratory comparisons, not blinded listening-test results. No acquired reference-bank samples are used in the performance.

## Performance and shared mechanics

The first 16 seconds contain notes MIDI 40, 55 and 88, each at velocities 0.2/0.5/0.9. Notes begin at 0.25 seconds with 1.75-second spacing and 1.1-second key holds. At 16.5 seconds a six-note E-minor voicing begins under sustain; three chord notes are struck again at 18 seconds. The pedal lifts at 20 seconds. A final G3 strike at 21 seconds tests key release and late-pedal recapture, with pedal up at 22.5 seconds and a tail through 24 seconds. The exact sorted event list is included in the report.

One set of production voices supplies all three pickup signals. Each signal has its own actual production decimator and the default `filtered * 0.7 * 0.12` output scale. The offline single-channel note/pedal evaluator preserves ringing motion through repeated strikes. A regression fixture compares the current track sample-for-sample against `Engine` through overlapping notes, release, retrigger and late pedal. This helper is not a new real-time plugin engine or a general MIDI implementation.

## Matching method

Let `Ri` and `Pi` be the raw RMS and sample peak of each complete performance. Using the current track as the RMS reference:

```text
relative_gain[i] = Rcurrent / Ri
common_attenuation = min(1, ceiling * (1 - 1e-6) / max_i(Pi * relative_gain[i]))
export[i] = raw[i] * relative_gain[i] * common_attenuation
```

Each track uses one constant gain for the entire program. The common attenuation leaves margin for final f32 rounding; exports are verified against the sample ceiling after conversion. No note, intensity, attack, sustain window or chord receives separate normalization. Relative dynamics and envelopes within each track remain intact. Silence or insufficient matching energy is rejected rather than amplified arbitrarily.

Equal whole-program RMS is not equal perceptual loudness. It also does not make every individual note equally loud across models: those differences are part of the comparison. The ceiling is a sample-peak constraint, not an oversampled true-peak measurement or a guarantee about host effects.

## Observed headroom

The same run measures all 73 isolated notes at velocity 1, held for one second. It also measures a ten-note chord and all 73 keys struck together at 0, 0.3 and 0.6 seconds, retaining mechanical motion between strikes. Both stress observations last one second. The chord is MIDI 40/47/52/55/59/62/64/66/71/74.

The receipt keeps per-note raw RMS/peak, maximum isolated peaks, both repeated-chord results and the per-track gain that would bring the largest observed diagnostic peak to the requested sample ceiling. These diagnostic gains are **not applied** to the listening WAVs or plugin. Applying a dense 73-key stress gain to every listening example would obscure the timbral comparison with unnecessarily low playback levels. Conversely, the listening gain is not a headroom guarantee for those stress inputs.

The observations are bounded examples, not a mathematical limit for arbitrarily long sustains/retriggers or all sample rates and parameter combinations. Output gain remains a release decision; no compressor or limiter is added by the study.

Numerical validation and analysis finish before the directory is reserved with create-new semantics. Existing output directories are rejected. WAVs are finished before the JSON receipt is written; a filesystem failure can leave an incomplete new directory, which is not reported as a successful run.

## Recorded study

Date: 2026-09-04. The complete study ran at 44.1 kHz with audio export and at 192 kHz with `--measure-only`, using the default candidate geometry and -6 dBFS ceiling. Both runs finished with zero numerical faults. This provides 146 isolated-note observations and four repeated-chord observations, with three signal paths in each.

The 44.1 kHz performance levels and export gains are:

| Track | Raw peak | Raw RMS | RMS gain | Final applied gain | Export peak | Export RMS |
| --- | --- | --- | --- | --- | --- | --- |
| Current | 0.961816 | 0.055813 | 1.000000 | 0.507194 | 0.487827 | 0.028308 |
| Close original | 3.329438 | 0.188052 | 0.296794 | 0.150532 | 0.501187 | 0.028308 |
| Close point-pole | 6.725860 | 0.409904 | 0.136160 | 0.069060 | 0.464485 | 0.028308 |

Common attenuation is 0.507194. The point-pole-to-current whole-program RMS factor of 0.136160 includes both the geometry and law changes. Comparing the two close tracks instead isolates the law; their relative matching factor is approximately 0.45877. None of these factors was selected as a new plugin gain. The 192 kHz matching factors are within approximately 0.002% of the 44.1 kHz factors for this program.

Observed raw headroom results, before matching or attenuation:

| Rate | Track | Maximum isolated peak (MIDI note) | Repeated ten-key peak | Repeated 73-key peak |
| --- | --- | --- | --- | --- |
| 44.1 kHz | Current | 0.385237 (60) | 3.040863 | 16.991201 |
| 44.1 kHz | Close original | 1.452072 (89) | 10.021235 | 63.217300 |
| 44.1 kHz | Close point-pole | 3.377928 (93) | 19.385435 | 138.250137 |
| 192 kHz | Current | 0.388961 (89) | 3.044980 | 17.060589 |
| 192 kHz | Close original | 1.467710 (89) | 10.026181 | 63.423981 |
| 192 kHz | Close point-pole | 3.429340 (90) | 19.372740 | 139.716248 |

The raw electrical scale is arbitrary. These large stress peaks are unexported diagnostics, not the amplitude of the listening WAVs. Both the current and candidate models need an explicit gain/headroom policy for dense playing; matching an isolated note or a musical program cannot establish that policy. The highest isolated-note index changes with rate because the sample grid and numerical trajectory also change. No maximum across all possible rates, gestures or parameter values is inferred.

The three 24-second mono float WAVs are under `renders/pickup-listening-20260904-234759/`, totaling 12,700,974 bytes. The 192 kHz directory `renders/pickup-listening-20260904-234759-192k/` contains only its receipt. `references/pickup-listening-summary.json` preserves both complete reports, source-report hashes and SHA-256 hashes of all three WAVs; audio remains ignored by Git.

These artifacts are ready for human comparison. No human listening result or preference is claimed. The plugin remains the current 0.1.1 research profile; the next instrument version still needs a deliberate gain/headroom decision and a RackForge audition.
