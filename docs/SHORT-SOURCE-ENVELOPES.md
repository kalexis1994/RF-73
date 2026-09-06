# Short envelopes on pinned G3 recordings

The frozen short-source pilot produces no qualified cross-window decay rate.
All twelve available higher-family measurements fail margin, amplitude and phase
stability checks. This is a limitation of this observation protocol, not a
measurement of the instrument's mechanical damping.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- observe-short-source-envelopes references/short-source-envelope.manifest.json --output renders/short-source-envelope.json
```

The [manifest](../references/short-source-envelope.manifest.json) was written
before the first source run. The [receipt](../references/short-source-envelope-validation.json)
uses experiment `pinned-short-source-envelopes-v1`, schema 1. The command accepts
the same strict selection fields as the long pilot, but requires an interval
length of 0.128..0.3 s. Outputs must be new JSON files. Missing components and
failed gates are successful observations; invalid inputs fail before publication.

All five G3 takes come from the same pinned jRhodes3d-wav revision and
[register-family evidence](../references/register-families-validation.json) as
the [long source pilot](SOURCE-ENVELOPES.md). Evidence blob:
`73274fe7d48e9ac7cd8f5041f9910d1392656ba4`. The command verifies the exact receipt
and WAV bytes before decoding. These are processed mono harp-output recordings;
capture gain, strike speed and note-off remain unknown. Native 44.1 kHz rate and
recording gain are retained. No new audio files are generated.

## Frozen protocol

- Observe file offsets 0.02..0.18 s, without onset alignment or trimming.
- Use the qualified prior fundamental anchor and unique prior attack-128 peaks
  in the existing 1410..1440 Hz and 1605..1635 Hz family bands.
- Exclude the associated target peak. Choose the two other prior peaks nearest
  in absolute frequency, breaking ties by prior index. No amplitude ranking,
  harmonic snapping or carrier refinement. These peaks are nuisance candidates,
  not established harmonics or physical modes.
- Keep all omitted indices, missing families, ambiguous/capped flags and takes.
  Carriers must support both window widths; unsupported/duplicate selections
  are retained without substituting more convenient peaks.
- Apply the unchanged [short estimator](SHORT-ENVELOPE.md) at 32 and 64 ms with
  8 ms hop. Both measurements and prior selection must qualify, and their rates
  must agree within `max(0.5 /s, 15% of the larger absolute rate)` to report a
  descriptive mean. No mixing-rate or mechanical-loss inference is made.

The mean-rate rule is a protocol consistency check, not a confidence interval.
Integer windows are 1411 and 2822 samples (31.9955 and 63.9909 ms); hop is 353
samples (8.00454 ms). The resulting 16 and 12 overlapping observations are
correlated. The short estimator's original synthetic cases still pass; no
threshold, equation or carrier choice was changed after seeing these recordings.

## Results

Five takes produce 15 component slots: four missing families, two unsupported
fundamental selections, and 18 measured envelopes. All 18 reject. Twelve are
higher-family measurements and six are fundamental measurements. No conditional
cross-window mean is published.

| Take | Fundamental | Lower family | Upper family |
|---|---|---|---|
| Layer 1 | Nearest nuisance at 75.42 Hz cannot support 32 ms | Missing | Missing |
| Layer 2 | Both widths reject insufficient decay; prior list also capacity limited | Missing | Missing |
| Layer 3 | Both widths reject decay support and slope consistency | Both reject | Both reject |
| Layer 4 | Both widths reject decay support and slope consistency | Both reject | Both reject |
| Layer 5 | Nearest nuisance at 108.24 Hz cannot support 32 ms | Both reject | Both reject |

For the higher families, every measurement rejects on regression margin,
insufficient decay, amplitude residual, early/late slopes and phase residual.
Eleven also reject on carrier offset. Minimum per-envelope regression margins
range from -36.98 to -4.67 dB, below the 18 dB requirement. Amplitude residuals
range from 1.13 to 6.00 dB RMS, above 0.5 dB. All fits complete the QR check;
the smallest relative pivot is about 0.00560, above the `1e-5` cutoff. Numerical
solvability therefore does not imply that the target component has been isolated.

The prior lists contain strong low-frequency components outside the three
selected carriers. For example, layer 5 has a -3.73 dBFS fundamental and a
-53.62 dBFS lower-family attack peak in the prior receipt. Those amplitudes
describe the earlier attack analysis, not a new fitted source envelope. A
three-carrier broadband regression leaves other components in its residual;
short unweighted projections can also leak those components into the target.
This is a plausible contributor to the observed failure, not an isolated causal
measurement. Large reported carrier offsets belong to rejected fits and must
not be interpreted as physical detuning or used to retune the tine.

The earlier long pilot's conditional layer-3 fundamental rate remains intact.
Its longer support and different interval answer a different question; rejection
on the early short interval does not invalidate it. Neither pilot identifies
higher-mode mechanical losses.

## Next modeling step

Validate a frequency-selective observation stage against controlled mixtures
with a much stronger distant fundamental, a close neighbor and a weak short
target. Quantify transient/decay and phase bias before revisiting these files.
Keep this unsuccessful broadband pilot as the baseline. Do not enlarge the
carrier count, discard source peaks or relax thresholds merely to obtain a
source rate. Any new method needs its own declared synthetic challenge and
source protocol before it can inform physical loss calibration.

Production DSP, spring geometry, pickup law, presets and the audible baseline
are unchanged. This stage includes no playback or host test.
