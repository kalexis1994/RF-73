# Attack and body tone comparison

`compare-tone` compares short spectral observations without requiring a strike-velocity mapping or declaring an uninterrupted sustain boundary. It is useful for processed sample-bank references whose physical capture metadata is incomplete. It does not estimate decay, align onsets or rank candidates.

```text
cargo run --locked --release -p rf-73-lab -- compare-tone REFERENCE.wav CANDIDATE.wav --note 55 --output renders/tone.json --reference-start 0 --candidate-start 0
```

`--note` and a new `.json` output are required. Inputs retain their samples and must have equal rates; multichannel inputs require `--reference-channel` and/or `--candidate-channel`. The existing WAV limits apply. Anchors default to file start and must be finite, nonnegative positions inside their recordings. Choose corresponding note ages; edited sample starts do not establish the time of physical hammer contact.

Three complete observations are taken relative to each anchor:

| Label | Start | Duration |
| --- | --- | --- |
| `attack_32_ms` | 0 ms | 32 ms |
| `attack_96_ms` | 0 ms | 96 ms |
| `body_350_ms` | 250 ms | 350 ms |

Anchors, offsets and lengths round to samples. A missing complete window rejects the comparison; there is no padding or truncation. Full observations are used at every supported rate, including all 67,200 body samples at 192 kHz. Window names describe intended note phases, not automatic evidence of attack or sustain. The report records exact start frames, sample counts and FFT grids.

The spectrum uses DC removal, symmetric Hann, coherent-gain correction and zero padding. It retains the existing snapshot peak searches: up to 12 harmonic labels below Nyquist and 16 separated prominent peaks. Searches use the supplied note as a frequency hint, an approximately -60 dB relative-peak gate and the existing bounded search width. They are not the independent tracker's noise/ambiguity qualification. A labeled harmonic is a spectral candidate, not an identified mechanical mode; unresolved neighbors or interference can change the selected peak. Zero padding does not improve physical resolution.

For each detected harmonic, the report includes its amplitude relative to that input's detected fundamental, the candidate-minus-reference difference of those ratios, raw amplitude difference and frequency difference in cents. Raw spectra and window RMS remain available. Harmonic balance removes a constant gain from that ratio only; it does not modify audio or provide a matched-level waveform. Relative balance requires both a detected H1 and at least four cycles of the expected fundamental in the observation. Missing peaks or an unusable fundamental produce null ratios/errors, never a synthetic zero-amplitude component.

Compare all available peaks and missing detections as well as ratios. No aggregate score is supplied: an average over only detected harmonics could conceal a missing component. EQ alters relative balance; noise reduction and windowing affect weak components. These metrics locate residuals and do not establish perceptual equivalence or physical parameter identification.

Fixtures verify identity, a constant 6.02 dB gain difference, a known H3/H1 change, missing H3/H1, silence, insufficient cycles, explicit offsets, full high-rate observations and rejected invalid/incomplete regions. See [G3 residual pilot](G3-RESIDUAL-PILOT.md) for the first real-reference experiment.
