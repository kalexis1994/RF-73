# G3 source-envelope pilot

Only the layer-3 fundamental passes both temporal window lengths in this fixed
pilot. The approximately 1425/1620 Hz families fail envelope qualification in
every available take, so their decay-rate relation is withheld. The data do not
yet justify fitting mechanical losses or claiming that pickup mixing explains
the recorded pair.

## Frozen selection and reproduction

```text
cargo run --locked --release -p rf-73-lab -- observe-source-envelopes references/source-envelope.manifest.json --output renders/source-envelopes.json
```

The [manifest](../references/source-envelope.manifest.json) was fixed before
temporal measurements. It selects G3 from the existing
[register evidence](REGISTER-FAMILIES.md), whose exact Git blob is
`73274fe7d48e9ac7cd8f5041f9910d1392656ba4`. Git verifies the same evidence bytes
that Rust deserializes, then verifies each WAV's pinned bytes before decoding.
All paths are relative to the repository working directory. The strict manifest
is limited to 64 KiB, evidence to 2 MB, and a note group to 3..8 distinct takes.
Missing/mismatched IDs, duplicate content, invalid selection bounds, changed
bytes or unsupported evidence versions fail before publishing a report.

All five original mono G3 recordings are retained at 44.1 kHz. They come from
the pinned jRhodes3d bank described in [Reference banks](REFERENCE-BANKS.md),
with EQ/noise reduction and unknown recording gain, strike speed and note-off.
Previously inspected recordings are reused; this is not a blind validation or
an independently recorded instrument. Samples are not bundled in RF-73.

Each take has three component slots:

- Fundamental: its previously qualified per-take pitch anchor.
- Lower family: a unique previously accepted attack-128 peak in 1410..1440 Hz.
- Upper family: a unique previously accepted attack-128 peak in 1605..1635 Hz.

No family mean replaces a missing take-specific peak. Missing, ambiguous and
capacity-limited evidence is retained. Every other accepted attack peak is a
declared neighbor; a unique attack peak within 8 Hz of the fundamental anchor
is associated with that anchor and excluded as self. Ambiguity/capacity flags
remain disqualifying. A prior peak list does not prove isolation throughout
the later observation interval.

The interval is fixed at file offsets 0.1..1.5 seconds. Each available component
uses the unchanged [envelope estimator](COMPONENT-ENVELOPE.md) at 128/256 ms,
with quarter-window hops. These bounds are neither aligned mechanical onset
nor guaranteed natural sustain. There is no carrier refinement, interval search,
threshold relaxation or averaging of rejected fits.

To report a conditional rate, prior selection must be reliable and both window
measurements must qualify. Their rates must agree within
`max(0.2 /s, 10% of the larger absolute rate)`. The reported mean is descriptive,
not a statistical uncertainty estimate. The weak-mixing rate residual
`upper - lower - fundamental` is only reported if all three conditional rates
exist; no causal acceptance threshold is attached to it.

## Results

The [417084-byte receipt](../references/source-envelope-validation.json),
experiment `pinned-source-envelopes-v1`, retains 15 component slots, four missing
selections and all 22 temporal measurements. Four individual measurements pass;
only one component passes both window lengths and the selection checks.

| Take | Fundamental 128 ms | Fundamental 256 ms | Conditional rate /s | Higher families |
|---|---|---|---:|---|
| G3 layer 1 | Reject | Reject | withheld | Neither has a prior accepted peak |
| G3 layer 2 | Reject | Reject | withheld | Missing; prior window also capacity-limited |
| G3 layer 3 | Pass | Pass | 0.329510 | Both reject at both lengths |
| G3 layer 4 | Pass | Reject | withheld | Both reject at both lengths |
| G3 layer 5 | Pass | Reject | withheld | Both reject at both lengths |

The layer-3 fundamental rates are 0.329950 and 0.329070 /s. This is a conditional
output-component observation, not a measured tine-energy loss coefficient.
Layer 1's fundamental initially grows and has inconsistent early/late slopes.
Layer 2 does not reach the minimum 3 dB fitted drop. Layers 4 and 5 pass at
128 ms but have only 2.959 and 2.640 dB fitted drop at 256 ms; the existing
3 dB threshold is retained, including the near miss.

All twelve higher-family measurements fail the local-margin, amplitude-fit,
early/late-slope and phase-stability checks. Nine also fail carrier offset.
The large provisional rates in their reports are **rejected diagnostics** and
must not be used to tune damping. Adjusting carrier frequency alone would not
remove the other failures.

The pre-existing upper-minus-lower-minus-fundamental frequency residuals remain
approximately -0.589, -3.057 and -2.265 Hz in layers 3/4/5. Their proximity under
the earlier attack-window tolerance does not establish the decay or phase
relationship. All five mixing-rate comparisons are withheld.

## Detectability diagnostic

After retaining the full-interval results, inspect the initial contiguous run
of points meeting the existing 18 dB guard-margin and -180 dBFS floor. The table
gives the span between first and last such centers, not a physical lifetime or
a newly selected fit interval. No estimator is rerun on these shorter regions.

| Family / take | Initial supported span, 128 ms | Initial supported span, 256 ms |
|---|---:|---:|
| Lower / layer 3 | No initial passing point | 0.384 s |
| Upper / layer 3 | No initial passing point | 0.320 s |
| Lower / layer 4 | 0.480 s | 0.512 s |
| Upper / layer 4 | 0.160 s | 0.128 s |
| Lower / layer 5 | 0.544 s | 0.640 s |
| Upper / layer 5 | 0.064 s | One initial point; zero span |

The upper family's initial supported run is shorter than the current 0.4 s fit
requirement in every measured take/window. Window length changes both temporal
support and guard frequencies; these differences are not evidence of different
physical lifetimes. Interference, processing and loss of local detectability
remain possible explanations for the rejected measurements.

## Validation and next work

Tests cover missing/ambiguous/capped selection, unique fundamental association,
strict manifests and source IDs/content, qualified cross-window rate agreement,
withholding failed windows, source/evidence byte protection and CLI output
preservation. A synthetic CLI fixture retains missing higher components rather
than inserting frequencies. All source recordings and existing audible baselines
remain unchanged; no listening result or physical fit is claimed.

Next validate an estimator for short transients against synthetic mixtures with
strong neighboring harmonics, uncertain carrier frequency and limited signal
support. Simply shortening Hann windows increases overlap risk; simply trimming
until a fit passes biases selection. Preserve this full-interval pilot and test
the new method independently before revisiting the source family or adopting
mechanical loss parameters.
