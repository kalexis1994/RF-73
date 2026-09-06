# Frequency-selective short-envelope observation

A centered FIR analysis filter now separates a weak short target from a distant
fundamental with 300 times its amplitude in controlled mixtures. All 30 prescribed
positive observations pass unchanged short-envelope gates and the declared error
bounds. However, only 63 of 66 individual-window expectations pass: an onset
inside the interval is accepted at 64 ms at all three sample rates. The original
failed expectations remain in the receipt and the validation command exits with
failure. This is not a fully qualified single-window source estimator.

## Reproduce and inspect

```text
cargo run --locked --release -p rf-73-lab -- validate-band-envelope --output renders/band-envelope.json
```

The command writes a new JSON report before reporting failed expectations.
The [retained receipt](../references/band-envelope-validation.json), experiment
`band-isolated-short-envelope-validation-v1`, is 630544 bytes. It includes every
filtered complex envelope and an unfiltered control summary. No WAVs are emitted.
The API `measure_band_envelope` lives only in the offline analysis crate; no
production DSP, source-pilot command or plugin behavior changes.

## Filter and support

The design follows the windowed-impulse-response method described by Julius O.
Smith in [Window Method for FIR Filter Design](https://dsprelated.com/freebooks/sasp/Window_Method_FIR_Filter.html),
using the [classic Blackman coefficients](https://www.dsprelated.com/freebooks/sasp/Classic_Blackman.html).
With sample rate `Fs`, target `fc`, cutoff half-bandwidth `B=300 Hz`, and integer
half-support `M=round(0.016*Fs)`, the centered kernel is:

```text
g[n] = sin(2*pi*B*n/Fs)/(pi*n), with g[0] = 2*B/Fs
w[n] = 0.42 + 0.5*cos(pi*n/M) + 0.08*cos(2*pi*n/M)
h[n] = 2*g[n]*cos(2*pi*fc*n/Fs)*w[n], -M <= n <= M
```

Normalize by the real stationary response at `fc`. Apply centered convolution
once, without forward/backward filtering. Tap symmetry removes a causal time
delay, but the filter is noncausal and may anticipate or smooth transients.
Unit stationary gain does not imply unit gain or zero phase for a decaying tone.

The requested interval is cropped with `ceil(start*Fs)` and `floor(end*Fs)`.
Every output sample requires all real source samples from `-M` through `+M`;
insufficient data on either side fails. There is no zero padding or inferred
onset. The report records the exact source and filtered sample bounds, kernel
length, work count and stationary response at every declared carrier.

The unchanged short estimator runs on the filtered crop. Its reported times and
demodulated complex coefficients are transformed back to original-file
coordinates; the crop must not reset the phase reference. Raw full-scale samples
anywhere in the filter support reject the result even when the filtered output
is small. Filtering colors noise: the regression margin remains descriptive,
not a confidence interval.

The fixed filter requires `fc > 600 Hz`, `fc+600 < 0.45*Fs`, and at most two
distinct nuisance carriers within 200 Hz of the target. It retains the short
estimator's interval/window/hop limits and additionally bounds convolution to
100 million multiply-accumulates before allocation. It is not a fundamental
envelope estimator and does not automatically select or replace source carriers.

## Frozen challenge

Eleven 0.25 s probes at 44.1, 48 and 96 kHz use the same 0.02..0.18 s interval,
32/64 ms windows and 8 ms hop. Equations and bounds were written before the
first run. All clips are generated in Rust in memory.

| Component | Frequency Hz | Amplitude | Phase rad | Amplitude decay /s |
|---|---:|---:|---:|---:|
| Target | 1620 | 0.002 | 0.73 | 8 |
| Distant fundamental | 196.35 | 0.6 | 0.1 | 0.33 |
| Neighbor | 1568 | 0.02 | -0.4 | 1 |
| Second harmonic | 392.7 | 0.05 | 0.9 | 0.66 |
| Third harmonic | 589.05 | 0.015 | -0.7 | 0.99 |

Positive cases are clean, distant fundamental only, full interference, a +2 Hz
target offset, and uniform noise of amplitude 0.00001. Negative cases are a
continuous rate switch from 4 to 18 /s at 0.09 s, stationary target, absent
target, near-coincident declared neighbor at 1620.5 Hz, target onset at 0.04 s,
and noise amplitude 0.03. Noise uses LCG64 seed `0x73ba11`. The onset waveform
is zero before 0.04 s and follows the prescribed absolute-time exponential
afterward; it is intentionally not exponential over the whole interval.

Every case declares target and neighbor even when the neighbor is absent.
Positive cases require rate error below 0.1 /s, carrier error below 0.02 Hz,
maximum amplitude bias below 0.05 dB and maximum phase bias below 0.01 rad
against the true unfiltered target. Negative expectations remain as originally
declared, including rejection of each individual onset observation.

## Findings and limits

All 30 positive observations pass. Across them, maximum errors are:

| Metric | Observed maximum |
|---|---:|
| Amplitude-decay rate | 0.007376 /s |
| Carrier offset | 0.001091 Hz |
| Point amplitude bias | 0.021395 dB |
| Point phase bias | 0.001776 rad |

Only the six clean unfiltered positive controls qualify; the other 24 reject.
This supports the filter's usefulness for the specified distant-interference
challenge, not arbitrary source spectra or noise populations.

All three 32 ms onset observations reject on amplitude residual, inconsistent
slopes and unstable phase. At 64 ms they pass, with fitted rates around
7.997..8.009 /s, despite the onset within the declared interval. Window centers
and filter smoothing can conceal a temporal event. A plausible fitted rate is
therefore not evidence of uninterrupted natural sustain.

Applying the already-declared short source pilot's two-window rule descriptively
(both qualified, rates within `max(0.5 /s, 15%)`) gives the intended outcome for
all 33 pairs: 15 positive pairs qualify, 18 negative pairs reject. The onset
pairs reject because their 32 ms member fails. This comparison does not erase
the three original failed expectations or prove onset rejection at other times.
The CLI continues to fail the original 66-case study; the regression test checks
that these limitations are retained explicitly.

## Numerical verification and next step

Tests check kernel symmetry, stationary target normalization, neighboring gain,
and distant-frequency attenuation at all three native rates. A separate exact
exponential convolution identity supplies the complex gain and expected waveform
for checking filtered coefficients and absolute phase against an independently
constructed reference, including a non-integer requested start time. Source
halo clipping, unavailable support, invalid carriers and excessive work reject.

The initial phase unit test incorrectly compared the approximate polynomial
regression directly with exact exponential phase at a `1e-5` rad tolerance;
it differed by about `1.04e-4` rad at 44.1 kHz. The corrected oracle applies the
same regression to the independently derived exact filtered waveform and checks
agreement within `1e-8`. The separate predeclared study bias limit remains
0.01 rad. No FIR coefficient, waveform or acceptance gate was changed.

Next challenge the paired decision with a predeclared range of onset/release
positions and rate changes, retaining cases whose history is unidentifiable.
Only then freeze a new source protocol with explicit filter support and carrier
coverage. The original failed G3 broadband pilot stays unchanged; no mechanical
loss, spring geometry, pickup parameter, source WAV or audible baseline is fitted
or changed in this stage.

The subsequent [paired temporal-event coverage study](BAND-EVENTS.md) now shows
that agreement also accepts some in-interval events: 53 of 96 tested pairs.
The earlier 33-pair outcome does not generalize to all attack/release timings.
Natural mechanical sustain remains unidentified, including for qualified pairs.
