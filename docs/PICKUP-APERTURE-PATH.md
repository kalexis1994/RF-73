# Close Aperture pickup path

Follow-up (2026-09-15): the [independent quadrature diagnostic](APERTURE-QUADRATURE-DIAGNOSTIC.md)
finds substantial differences between this retained 16-node surrogate and a
uniform-disk integral at the selected geometry. The existing fitted sound is
unchanged; its harmonic agreement is not evidence of disk-integration accuracy.

The [pickup harmonics study](PICKUP-HARMONICS.md) found that the laboratory's
finite-aperture flux law at gap 0.5 mm, lateral offset 0.5 mm and pole radius
2 mm turns the playable engine's own tine motion into the recorded harmonic
balance. This block puts that law and geometry into the playable engine as a
fourth selectable pickup path, **Close Aperture**, beside the three retained
paths, matches its level with the frozen listening protocol, and measures the
diagnostic's G3 strikes and the nocturne through it. No default changes: the
plugin still opens on Current against Close Point Pole.

```text
cargo run --locked --release -p rf-tines-lab -- render --output g3.wav --note 55 --velocity 1.0 --sample-rate 44100 --seconds 8 --hold 7.5 --pickup 3
cargo run --locked --release -p rf-tines-lab -- render-midi SONG.mid --output song.wav --normalize --pickup 3
cargo run --locked --release -p rf-tines-lab -- pickup-listening --output renders/pickup-listening
```

## The path

The engine's laboratory keeps one set of production voices and now feeds four
continuously filtered pickup signals: Current, Close Original, Close Point
Pole and Close Aperture. The new signal is the finite-aperture law reduced to
the tine axis. `AxialAperture` uses the same two-radius, eight-azimuth disk
quadrature as the laboratory's `SpatialPickup`; with no vertical offset the
mirrored azimuths contribute identically, so the sixteen nodes merge into ten
terms and the flux slope is the exact analytic derivative of the node flux,
not a table or a fit. A unit test holds it against the two-axis discrete
gradient on the axis to 1e-12 of the largest slope. The voltage is −0.015
times the slope of the unit-normalized flux times the tip velocity, the scale
the production law applies to its own normalized flux.

Within the pole face the node ring makes the flux rise toward the ring, so
near the centre the slope, and the voltage sign, reverse relative to the
production law; outside the face they agree. This is a property of the
16-node proxy, retained as it was fitted, and it is why the geometry on the
grid's edge remains provisional.

The fourth path has its own production decimator and 20 ms crossfade like
the others. The plugin accepts index 3 in both slots, the package metadata
and the PLAY panel list the choice, and saved state carries it unchanged.
`render --pickup I` and `render-midi --pickup I` render the laboratory engine
through path I at the default geometry; path 0 reproduces the raw engine
sample for sample, which a CLI test holds.

## Level match

The [listening protocol](PICKUP-LISTENING.md) was rerun with four tracks: one
RMS ratio per complete 24-second performance at 44.1 kHz, never per note or
velocity. The three retained tracks reproduce their frozen factors exactly,
so the receipt confirms their paths are untouched. The raw aperture track is
the quietest and the only one matched with a gain above unity:

| Track | Raw peak | Raw RMS | RMS gain (frozen factor) |
| --- | ---: | ---: | ---: |
| Current | 0.961816 | 0.055813 | 1 |
| Close Original | 3.329438 | 0.188052 | 0.2967936920096338 |
| Close Point Pole | 6.725860 | 0.409904 | 0.1361600848962658 |
| Close Aperture | 0.415945 | 0.022328 | 2.4996279723549004 |

The 192 kHz measure-only run agrees within 0.002%. After matching, the
aperture track's observed stress peaks sit close to Current's: repeated
ten-key chord 3.15 against 3.04, all 73 keys 18.8 against 17.0, so the
0.100x starting gain policy is unchanged.

## G3 strikes against the recordings

The [playable diagnostic](PLAYABLE-G3-DIAGNOSTIC.md) was repeated through
path 3 with the same renders and comparisons. Levels relative to each
signal's own fundamental in the 96 ms attack window, recording / Close
Aperture / previous engine:

| Harmonic | Layer 1 (loud) | Layer 3 (medium) | Layer 5 (soft) |
| --- | --- | --- | --- |
| 2 | +2.4 / −2.8 / −3.3 | −17.4 / −14.7 / −10.6 | −21.7 / −19.2 / −21.6 |
| 3 | +7.0 / +8.4 / −12.4 | −13.6 / −11.3 / −24.5 | −41.6 / −39.9 / −45.7 |
| 4 | −9.2 / −8.1 / −32.8 | −17.3 / −16.8 / −56.2 | −55.9 / −50.5 / absent |
| 5 | +1.8 / −0.4 / −32.9 | −24.6 / −29.6 / −57.6 | absent |
| 6 | −0.1 / −1.4 / −54.0 | −34.5 / −43.6 / absent | absent |

The loud third harmonic now stands above the fundamental as in the
recording, the fourth through sixth are within 2.3 dB of it, and the largest
loud error is the second harmonic, 5.2 dB low. At the medium dynamic every
harmonic through the fourth is within 2.7 dB and the fifth is 4.9 dB low; at
the soft dynamic the second and third are within 2.5 dB. The engine's
measured balance agrees with the static prediction of the harmonics study
within 0.7 dB at every harmonic and dynamic, so the playable engine
reproduces the static transfer and the receipt test holds the balance
limits.

The rest of the diagnostic is unchanged, as a pickup change should leave it:
the fundamental still decays at 11.3 dB/s against the recording's 2.5 dB/s
at every velocity, and the second bar partial still sits at 1228 Hz and fades
within 130 ms. One consequence is worth recording. The engine's second
harmonic decays at 22 dB/s, twice the fundamental's rate, because a pickup
generates it from the square of the fundamental's amplitude; the recording's
second harmonic decays at 5 dB/s, twice its fundamental's 2.5 dB/s. The
harmonic decay ratio is already right; only the fundamental's loss is wrong,
which confirms the order of work. The [calibrated sustain](PLAYABLE-SUSTAIN.md)
takes that step next.

## Listening

The nocturne rendered through path 3 has more attack bite and body growl at
the loud passages and a rounder, less bell-like soft touch, without any
change in sustain. It peaks at +2.9 dBFS raw against +2.1 dBFS on the
default pickup and is 7% higher in RMS, so the normalized copy remains the
one to audition. It renders at 0.16 of real time against 0.09 on the raw
engine. The rendered files are outside the repository; the receipt
is reproducible with the command above.

## Cost

The fourth path is the first pickup law in the plugin that is not a
closed-form scalar. The 73-key laboratory stress at 48 kHz and 128-frame
blocks, which changes pickup every three blocks, moved from p99 0.993 ms and
worst 2.42 ms with zero deadline misses on three paths to p99 2.21 ms, worst
4.72 ms and 8 misses of 1125 blocks with four, against the 2.667 ms deadline;
before the axial reduction the same run missed every block at p99 3.94 ms.
The raw engine is unchanged at p99 0.73 ms. The laboratory engine is
therefore not qualified for the all-keys stress with the fourth path enabled,
which matters because the plugin runs all four paths continuously so that
switching never bursts a stale filter. Ordinary polyphony is far from this
case, but the reduced real-time engine must not inherit a 16-node pickup as
is; the baked transfer table foreseen for it is the intended answer.

## Scope

One geometry, fixed at the harmonics study's grid-edge optimum, on the
existing engine's mechanics and losses. The level factor is a whole-program
RMS ratio, not loudness matching. The G3 comparison uses processed recordings
with unknown capture gain, and the second harmonic at the loud dynamic is
still 5 dB short. The stress numbers are one desktop's short observation.
Nothing here changes the default pickup, the retained paths, the raw engine
or the laboratory assembly.

Receipts: `references/pickup-aperture-listening-summary.json` with both
listening reports and WAV hashes, and `references/g3-aperture-pickup/` with
the three render receipts, nine comparison reports and a manifest of sizes
and SHA-256 hashes. Receipt tests hold the exact reproduction of the retained
factors, the frozen fourth factor, the manifest hashes and the harmonic
balance limits.
