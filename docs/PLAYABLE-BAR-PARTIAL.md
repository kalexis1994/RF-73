# The second partial and the line at six times the fundamental

The [playable diagnostic](PLAYABLE-G3-DIAGNOSTIC.md) read the recordings'
line at 6.0 times the fundamental as the tine's second bending partial and
scheduled the engine's 6.27 partial to be retuned to it. This block set out
to do that, measured first, and found the opposite problem: the line is the
pickup's sixth harmonic, and the engine's second partial is not too brief
but too strong. The result is a profile field for the second partial's
strike weight, a field for its ratio, a calibrated profile that strikes the
partial fifteen times more softly, and a correction to two earlier
readings. The default profile is unchanged.

```text
cargo run --locked --release -p rf-73-lab -- render --output g3.wav --note 55 --velocity 0.6 --sample-rate 44100 --seconds 3 --hold 2.5 --pickup 3 --sustain calibrated --bar-strike -0.02
cargo run --locked --release -p rf-73-lab -- render --output g3-trace.wav --note 55 --velocity 0.25 --sample-rate 192000 --seconds 0.05 --hold 0.04 --trace --contact-stiffness 4e8
cargo run --locked --release -p rf-73-lab -- compare-partials LAYER.wav g3.wav --output attack.json --seconds 1 --reference-start 0.02 --candidate-start 0.02 --partial-window-ms 128
```

## Three arguments that the line is a harmonic

The line sits at 6.007 to 6.027 times each note's fundamental at D3, G3 and
B3, within 0.5% of the sixth harmonic at all three, while a bending partial's
ratio depends on the tuning weight's position and would not track the
harmonic that closely across notes. Its late decay is 18 to 23 dB/s where the
fundamental's is 2.3 to 2.9 dB/s, six to eight times faster, as a
pickup-generated sixth harmonic must decay; the second and third harmonics
decay at two and three times the fundamental's rate in the same recordings.
And its level relative to the fundamental swings from −5 dB at the loud layer
to −55 dB or absence at the soft layer, a fifty-decibel range for a
twenty-decibel change in the fundamental, which a harmonic proportional to the
sixth power of the tine's swing produces and a struck mode does not. The
engine's own sixth harmonic through the aperture pickup lands within 2.5 to
11 dB of the recording's line at the loud dynamic without any second partial
at that frequency.

## The engine's second partial is over-excited

Levels of the engine's 6.27 partial relative to its fundamental in the first
second, against the recordings' line at 6.0 as the upper bound of any
bending partial there:

| Note, velocity | Recording line at 6.0 | Engine partial, weight −0.3 (default) | −0.1 | −0.05 | −0.02 |
| --- | ---: | ---: | ---: | ---: | ---: |
| D3, 0.6 | −31.5 | −28.1 | −39.1 | −45.2 | −53.2 |
| D3, 0.25 | −54.8 | −22.5 | −32.0 | −38.1 | −46.0 |
| G3, 0.6 | −38.6 | −26.0 | −36.0 | −42.0 | −50.0 |
| G3, 0.25 | absent | −24.0 | −33.6 | −39.6 | −47.6 |
| B3, 0.6 | −56.9 | −26.3 | −36.0 | −42.1 | −50.0 |
| B3, 0.25 | absent | −25.5 | −35.0 | −41.1 | −49.0 |

With the default weight the partial sits at −22 to −28 dB at every note and
dynamic, 4 to 35 dB above the recordings' line and nearly independent of
velocity, because the engine's contact is impulsive: it lasts 0.09 to
0.34 ms across velocities and notes, shorter than half the partial's period
at every strike. Softening the contact does not change this. A hundredfold
lower contact stiffness lengthens the soft-strike contact to 1.0 ms and the
loud one to 0.5 ms and lowers the soft partial by only 5 to 9 dB; at four
hundredfold the soft contact lasts 4 ms and the level barely moves further,
so the partial is excited by the release as much as by the pulse. The lever
that works is the partial's participation at the strike point,
`bar_partial_strike_weight`, formerly the constant −0.3 in the voice. The
level falls in proportion to the weight, 20 dB per decade across the sweep,
and −0.02 puts the partial 44 to 55 dB below the fundamental at every note
and dynamic: under the recordings' medium line at D3 and G3, 7 dB above it
at B3, 9 dB above the soft D3 line, and where the recordings show nothing
at the soft dynamic the partial sits at −46 to −49 dB.

`Profile::calibrated` is the calibrated sustain with that weight. The ratio
stays at the uniform 6.267 because no measurement supports 6.0: the
recordings' line is the harmonic, and a partial this weak is not resolvable
in them. `bar_partial_ratio` remains a validated profile field and a renderer
option so the retune can be made when evidence appears. Both `render` and
`render-midi` take `--bar-ratio`, `--bar-strike` and `--contact-stiffness`.

## Two corrections

The diagnostic's third finding, a sustained bar partial at 6.0 that the
engine lacked, is withdrawn: the recording's sustained line is the pickup's
sixth harmonic, which the engine now produces through the aperture pickup
and which will last as long as its fundamental does. The
[calibrated sustain](PLAYABLE-SUSTAIN.md) derived a 2.3 s anchor for the
second partial from that same line; the number is a fit to the harmonic's
decay, not to a partial's, and with the partial 44 dB down it is inaudible
either way. It stays in the profile as an unmeasured constant, marked as such
in both documents.

## Listening

The nocturne with the calibrated profile and the aperture pickup loses a
faint metallic edge on the soft repeated notes that the over-excited partial
gave, and its loud attacks keep their bite from the harmonic series. It peaks
at +7.5 dBFS raw with the same RMS as the calibrated sustain alone, so the
gain policy is unchanged and still open.

## Scope

Three notes from one processed bank with unknown capture gain, a
one-second attack window with 128 ms frames, and an inference about the
line's nature from ratio, decay and level rather than from a separated
measurement. The engine's contact law is left as it is; its impulsive
contact is a known limitation the laboratory's hammer studies address. The
weight is a strike-point participation with no measurement of its own; the
recordings only bound it from above. Nothing here changes the plugin's
profile, its defaults or the level factors.

Receipt: `references/playable-bar-partial/summary.json` with the 36-case
sweep, the contact-duration table, the recordings' line ratios and the
hashes of the 18 retained render receipts and attack comparisons for the
chosen weight. A receipt test holds the hashes, the harmonic's ratio at all
three notes, the monotone fall of the partial with the weight, its level with
the chosen weight, the engine's sixth harmonic against the recording at the
loud dynamic and the contact-duration table.
