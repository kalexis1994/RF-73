# First offline audio from the memory modal assembly

The new nine-coordinate tine/tonebar/support assembly now produces a WAV through
the existing scalar magnetic pickup and an offline FIR. The two-mass hammer and
its material memory remain active through contact and recovery. This is a
selected-strike listening preview, not a new plugin version or a calibrated note.

```text
cargo run --locked --release -p rf-73-lab -- render-memory-modal --output renders/physical-preview.wav
```

The default take is 1.5 seconds at 48 kHz, with a 0.4 m/s launch, 75 mm tine
and damper engagement at 0.9 seconds. `--speed` accepts 0.2..0.8 m/s;
`--length-mm` accepts 75 or 120. Duration is bounded to 0.25..3 seconds with
at least 0.1 seconds before and after damper engagement. The explicit fixed
`--gain` defaults to 0.084. There is no inferred MIDI note or velocity mapping.
The JSON receipt records all active geometry, support, tonebar, losses, hammer
and material parameters. They retain the existing provisional defaults.

## Mechanical and audio protocol

The primary trajectory uses RK4/free integration with 8192 base ticks per output
frame, about 2.543 ns each. Observation boundaries are at 64x output rate.
One initial strike starts with zero gap; there are no later core impulses,
restarts or state resets. The damper changes only the prepared loss operator
at the declared frame, with an exact probe equality check.

Pickup displacement and velocity feed the same production scalar flux-derivative
law. Subsets of that identical trajectory feed 16x, 32x and 64x versions of the
same physical FIR kernel. Its cutoff is `0.42 * output Fs`, support is 31.5
output samples and delay is 15.75 output samples. Histories start at zero.
The output retains the common delay; there is no alignment, normalization,
clipping or flush beyond the stated duration. The 64x output is written as mono
IEEE float WAV after qualification. This is an output anti-alias filter, not
a calibrated Rhodes electrical loading or amplifier model.

A second trajectory uses twice the base resolution (16384 ticks, about 1.272 ns)
and tighter contact/free caps (levels 2/5). It supplies another 64x pickup render.
Observation times, geometry, output filter and gain stay fixed. These controlled
paths provide a finite comparison, not an exact continuous-time reference.

Qualification retains independent energy and port-work limits of 1e-8,
positive mechanical-energy increments below 1e-10, nonnegative material heat
and contact force, monotonic structural heat, and observed contact/separation.
Mass-weighted hammer/structure velocity and pickup velocity retain their 1%
RMSE gates, with 2% for output-frame mean force. Whole-record and every separate
2 ms mechanical section must pass. Zero reference force requires zero difference.

Audio comparisons use a 1% relative RMSE limit separately over the whole file,
the first 32 ms, the subsequent body and damper release. Both frozen sampling
comparisons and the integration comparison must pass. Silent reference audio
remains unqualified. These are engineering preview limits, not perceptual
equivalence thresholds or an absolute aliasing bound.

The WAV requires peak <= -1 dBFS and non-negligible RMS. Excessive gain produces
a retained failed report and no WAV. Both output paths are checked before
simulation and use exclusive file creation. A later filesystem write failure
still returns an error. Comparisons and reported levels use f64 before final
f32 conversion; written files are independently inspected afterward.

## First receipts and local audio

Date: 2026-09-06. All four full-length previews pass. Every file is 72000 frames
with the same 0.9-second hold and fixed output gain. Their level differences
are preserved.

| Tine | Launch speed | Peak dBFS | RMS | Local WAV |
| --- | ---: | ---: | ---: | --- |
| 75 mm | 0.2 m/s | -15.154 | 0.0164221 | [Soft](../renders/memory-modal-preview-75mm-soft.wav) |
| 75 mm | 0.4 m/s | -8.916 | 0.0328109 | [Medium](../renders/memory-modal-preview-75mm-medium.wav) |
| 75 mm | 0.8 m/s | -2.452 | 0.0649562 | [Strong](../renders/memory-modal-preview-75mm-strong.wav) |
| 120 mm | 0.4 m/s | -8.211 | 0.0171660 | [Longer tine](../renders/memory-modal-preview-120mm-medium.wav) |

The [tracked summary](../references/memory-modal-audio-preview-validation.json)
retains configurations, levels, comparison metrics and SHA-256 hashes of every
WAV and its local full report. The [complete medium-strike receipt](../references/memory-modal-audio-75mm-medium.json)
is also tracked. WAVs and other full reports stay in the ignored `renders`
directory and can be regenerated with the command above and listed options.

Maximum relative audio RMSE is 1.875e-7 across sampling comparisons and 1.347e-9
across integration comparisons, including release. The largest relative ledger
residual across both integrations of all four cases is 8.005e-11. Maximum
separate-section kinetic velocity error is 1.318e-9 of launch. The final 10 ms
RMS of each written file is below 6.203e-8; no artificial fade was applied.
These results qualify only the selected single-strike configurations.

An initial 0.25-second pilot passes. A negative control at gain 10 fails
headroom and retains `renders/memory-modal-headroom-rejection.json`, with no WAV.
All 199 workspace tests, strict Clippy, formatting and native release laboratory
compilation pass. New regressions reject silent/nonfinite signals, prevent a
whole-file average from hiding attack error, preserve gain differences,
bound CLI work and verify protection of both existing output paths. All four
WAVs pass the laboratory inspector. CI includes a short preview render;
remote CI was not run in this session.

## What is next

The first stage of the [direction review](RESEARCH-DIRECTION-2026-09-06.md) is
available for listening. No listening or host test was performed here.
The original repetition failure remains open, and the plugin still uses its
previous engine. This does not establish realtime performance, physical tuning,
neoprene parameters, pickup loading or Rhodes realism.

The [controlled hammer comparison](CONTROLLED-HAMMERS.md) now compares the
elastic/rate-dependent hammer with the memory hammer on the same structural
configuration and output chain. It preserves launch energy, gain and geometry
and records contact and audio observables. Shared-data fitting and calibration
against held-out reference notes/intensities follow.
