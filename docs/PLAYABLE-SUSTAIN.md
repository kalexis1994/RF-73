# Calibrated sustain for the playable engine

The [playable G3 diagnostic](PLAYABLE-G3-DIAGNOSTIC.md) found the plugin
engine's fundamental decaying four times faster than the recordings and its
second bar partial gone within 130 ms. This block gives the playable engine
per-partial losses, derives the first and bar partial T60 from the retained
D3, G3 and B3 recordings, and measures the engine against those recordings
with the new sustain and the [Close Aperture pickup](PICKUP-APERTURE-PATH.md).
The default profile is unchanged; the calibrated sustain is a named profile
and a renderer option.

```text
cargo run --locked --release -p rf-73-lab -- render --output g3.wav --note 55 --velocity 0.6 --sample-rate 44100 --seconds 8 --hold 7.5 --pickup 3 --sustain calibrated
cargo run --locked --release -p rf-73-lab -- render-midi SONG.mid --output song.wav --normalize --pickup 3 --sustain calibrated
cargo run --locked --release -p rf-73-lab -- compare-partials LAYER.wav LAYER.wav --output decay.json --seconds 4 --reference-start 0.5 --candidate-start 0.5 --partial-window-ms 512
```

## Per-partial losses

`Profile` carried one decay time, the first partial's T60 at A3, while the
second and third partials had fixed 160 ms and 55 ms constants in the voice.
The profile now carries all three, `decay_seconds`,
`bar_partial_decay_seconds` and `third_partial_decay_seconds`, under the
existing sqrt(220 Hz / f) pitch scaling, with defaults equal to the old
constants so every retained render is unchanged. `Profile::calibrated_sustain`
sets the first partial to 20 s and the bar partial to 2.3 s at A3 and leaves
the third partial, the mechanics and the pickup alone. The laboratory engine
accepts a profile through `Engine::new_laboratory_with`, provided it keeps the
pickup geometry the level factors were frozen on.

## What the recordings decay at

Each of the fifteen retained recordings, five layers each of D3, G3 and B3,
was tracked against itself over 0.5 to 4.5 s after onset in 512 ms windows.
The fundamental track's late slope gives T60 = 60 / |slope|, divided by
sqrt(220 / f0) to refer it to A3:

| Note | Fundamental late slope, layers 1 to 5 (dB/s) | T60 (s) | A3 anchor (s) |
| --- | --- | --- | --- |
| D3 | −2.3, −2.5, −2.6, −2.7, −2.5 | 22 to 26 | 18 to 21 |
| G3 | −2.9, −2.7, −2.6, untracked, −2.3 | 21 to 26 | 20 to 25 |
| B3 | −3.8, untracked, −3.5, −3.3, −3.1 | 16 to 19 | 17 to 21 |

The geometric mean anchor is 19.9 s, rounded to 20 s. The sqrt law fits the
three notes within about 8%; D3 and G3 decay alike while B3 decays faster than
the law predicts, so a steeper pitch dependence remains possible with more
notes. The line at 6.01 times the fundamental, read here as the bar partial
and identified afterwards as the pickup's sixth harmonic by the
[bar partial block](PLAYABLE-BAR-PARTIAL.md), is tracked through the
sustain window in six recordings, D3 layers 1 to 4 and G3 layers 1 and 2,
with late slopes of −18 to −23 dB/s, T60 2.6 to 3.3 s, and an A3 anchor whose
geometric mean is 2.5 s; in the first second it decays faster, near
−20 dB/s at D3 and G3 and −36 dB/s at B3, so 2.3 s is retained as the single
exponential. The second and third harmonics decay at two and three times the
fundamental's rate in every recording, as a pickup-generated harmonic must,
and need no loss of their own.

## The engine against the recordings

D3, G3 and B3 were rendered at velocities 1.0, 0.6 and 0.25 with the
calibrated sustain through the aperture pickup and compared with layers 1, 3
and 5. Fundamental late slopes in the 0.5 to 4.5 s window, engine / recording,
and the matched T60 difference where the tracker qualified the pair:

| Note | Velocity 1.0 | Velocity 0.6 | Velocity 0.25 |
| --- | --- | --- | --- |
| D3 | −0.6 / −2.3, unqualified | −2.2 / −2.6, unqualified | −2.4 / −2.5, +1.1 s |
| G3 | −2.1 / −2.9, saturated | −2.7 / −2.6, +0.9 s | −2.8 / −2.3, −4.7 s |
| B3 | −2.9 / −3.8, unqualified | −3.1 / −3.5, +3.2 s | −3.2 / −3.1, +0.3 s |

The diagnostic's deficits of 17 to 21 s are gone; the qualified differences
lie between −4.7 and +3.2 s, within the spread of the recordings' own layers.
At the loud dynamic the engine's fundamental, like the recordings', rises
during the first second before it decays, so those early slopes are shallow
on both sides. The engine's second harmonic now decays at 5 dB/s at G3
against the recording's 5 dB/s. The bar partial, still at the uniform 6.27
ratio, decays at 20 to 27 dB/s across the three notes instead of vanishing,
but it sits 25 dB below the recording's 6.01 line at the medium dynamic:
its tuning and excitation are the next block, not its loss. That block found
the 6.01 line to be the pickup's sixth harmonic and the engine's partial to be
too strong, not too weak; see [PLAYABLE-BAR-PARTIAL.md](PLAYABLE-BAR-PARTIAL.md).

The default pickup with the calibrated sustain gives the same fundamental
decay, so the two changes are independent as intended.

## Listening

The nocturne with calibrated sustain and the aperture pickup finally holds
its chords: the left hand's arpeggios ring under the melody instead of
evaporating, and repeated notes land on a still-sounding tine. The longer
sustain accumulates: the raw render peaks at +6.9 dBFS against +2.9 dBFS
with the aperture pickup alone, and RMS rises 58%, so the normalized copy is
the one to audition and the gain policy has to be revisited before any
default change. Rendering runs at 0.22 of real time.

## Scope

Three notes from one processed bank with unknown capture gain, one
exponential per partial, and a pitch law checked at three points only. The
third partial is unmeasured and keeps its 55 ms constant. The bar partial's
level and 6.0 ratio are untouched. The level factors of the laboratory
pickups were frozen on the default sustain; longer sustain raises every path
alike, so the ratios stand but the absolute level policy does not. Nothing
here changes the plugin's profile or defaults.

Receipts under `references/playable-sustain/`: the fifteen recording
tracking reports, ten render receipts, nine tone comparisons and ten sustain
comparisons with a manifest of sizes and SHA-256 hashes. A receipt test
re-derives both anchors from the recording reports and holds the profile
constants within 15% of them, the engine's fundamental slopes, the qualified
T60 differences and the bar partial's decay.
