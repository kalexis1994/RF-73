# Shared loaded voicing across drive speeds

The [single-speed voicing study](LOADED-VOICING.md) found conflicting changes
between source layers. Its pooled score has a large within-bank variance floor
because one spectrum cannot describe all layers. This experiment asks whether
one physical voicing can describe different attacks when the drive speed varies.

```text
cargo run --locked --release -p rf-73-lab -- loaded-dynamics references/g3-pitch-reference.manifest.json --output references/loaded-dynamics-validation.json
```

## Frozen training and reserved protocol

Two shared physical settings are prescribed: the preceding baseline and the
1 mm vertical pickup offset. Both use the same 70 mm tine, training-only
mode-tracked tuning spring, conditional first-basis T60 of 30 s, original
support losses, 14 mm hammer point, quadratic contact coefficient and remaining
pickup/circuit settings. No parameter changes between speeds of one voicing.

The drive grid is [1.125, 1.3125, 1.5, 1.625, 1.75] m/s. Training layers 1, 3
and 5 use either [1.125, 1.5, 1.75] or its reverse. Both ordinal hypotheses are
tested because layer labels and distributed levels are not measured physical
strike velocities. Endpoints and interpolation are imposed nuisance assumptions;
no continuous velocity curve or per-layer gain is fitted.

Each shared voicing first renders all three training speeds at 128 and 256
ticks per 48 kHz output frame: twelve training takes. Every take starts at static
rest, uses the existing 30 ms delayed pedestal gesture and remains held through
1.8 s. All original energy, quiet, single-contact, pitch, timbre, headroom and
1% voltage/impact refinement gates apply. An unqualified speed makes its entire
shared voicing ineligible. Missing attack bands withhold the hypothesis score.

Four hypotheses combine the two voicings with the two ordinal directions. The
minimum pooled RMS across nine attack-band dB errors from training layers alone
selects the experimental candidate. Fixed hypothesis order breaks exact ties.
All four scores remain visible. The CLI requires the exact five layer identities
and training/reserved roles, independently of manifest row order.

The selected voicing and direction are then frozen. Only then are layers 2 and
4 evaluated with two new coarse/fine pairs, using intermediate speeds 1.3125 and
1.625 m/s, reversed if the selected direction requires it. Each intermediate
speed is the arithmetic mean of its two neighboring training anchors. Reserved
spectra do not choose these speeds, refit the pickup or rerank a failed winner.
This gives sixteen takes when a training candidate is eligible.

The report retains five mapped source comparisons, all training hypotheses,
both resolutions of every take and all source/take cross-comparisons. The
original five-window 6 dB spectral and two-window 3 dB sustain gates remain
separate from numerical qualification and attack RMS. Lower training RMS alone
does not establish spectral agreement or justify changing the production preset.

## Retained training outcome

All six training coarse/fine pairs qualify. Training selects the original
pickup offset with decreasing speed by layer: 1.75, 1.5 and 1.125 m/s for
layers 1, 3 and 5. Its attack RMS is 18.869982 dB. The 1 mm offset that lowered
the preceding single-speed pooled score does not win when required to share
one voicing across the prescribed drive range.

This selection is the minimum of four imposed hypotheses, not evidence of
improvement over all controls. The original pickup at a single 1.5 m/s had
training RMS 18.735510 dB; the selected dynamic mapping is slightly worse on
that score. Its purpose is to test a coherent dynamic response and reserved
intermediate predictions, rather than optimize a separate spectrum for each
recording. Endpoint speeds and interpolation remain unmeasured assumptions.

| Shared voicing | Layer-to-speed direction | Training attack RMS (dB) |
| --- | --- | --- |
| Original offset | Increasing | 23.847735 |
| Original offset | Decreasing | **18.869982** |
| 1 mm vertical offset | Increasing | 22.568631 |
| 1 mm vertical offset | Decreasing | 19.702149 |

Both reserved coarse/fine pairs qualify. Their combined attack RMS is 12.418104
dB, versus 14.108503 dB for the preceding original-offset single-speed control.
That partial improvement does not resolve source agreement: all five mapped
pairs fail the full spectral gate, while all five pass both sustain windows.

| Source layer | Role | Assigned drive (m/s) | Attack RMS (dB) | Worst available spectral difference (dB) |
| --- | --- | --- | --- | --- |
| 1 | Training | 1.75 | 9.445955 | 51.64125 |
| 2 | Reserved | 1.625 | 5.855533 | 33.36767 |
| 3 | Training | 1.5 | 11.658103 | 12.85013 |
| 4 | Reserved | 1.3125 | 16.556913 | 26.52627 |
| 5 | Training | 1.125 | 29.036034 | 45.92509 |

All forty cross-comparisons also fail spectral agreement; 39 pass the sustain
comparison. These diagnostic cross-pairs do not enter selection or reserved
evaluation. Available-band differences retain missing dimensions rather than
silently treating weak observations as zero error.

## Action response exposed by the common voicing

The selected original-offset model has the following fine-resolution response:

| Pedestal speed (m/s) | Pre-contact hammer speed (m/s) | Contact impulse (N s) | Positive-force duration (ms) | Fixed-gain peak (FS) |
| --- | --- | --- | --- | --- |
| 1.125 | 0.106164 | 0.000237325 | 0.382161 | 0.010898 |
| 1.3125 | 0.719104 | 0.004318687 | 0.726481 | 0.133641 |
| 1.5 | 1.078643 | 0.006781728 | 0.714762 | 0.213812 |
| 1.625 | 1.269913 | 0.008052123 | 0.719320 | 0.258734 |
| 1.75 | 1.438573 | 0.009167603 | 0.718913 | 0.298505 |

The 16.7% increase from the lowest to the next drive speed multiplies actual
hammer speed by about 6.77 and impulse by 18.2. The low-speed gesture is close
to the previously observed non-striking region. Its attack band balances are
[-30.82, 0.41, -29.07] dB, while source layer 5 has approximately
[-20.38, -45.51, -46.71] dB. The second reported upper-band balance
(4–12 times the anchored pitch) therefore differs by 45.93 dB.
This is not a uniform level mismatch that a fixed gain could remove.

The 1 mm pickup offset leaves the low-speed hammer speed and impulse nearly
unchanged and does not remove that large second-band discrepancy. These
controls locate a concrete issue for further diagnosis: the action-to-impact
response near the soft-strike threshold and the resulting modal excitation.
They do not prove whether the imposed drive curve, action parameters, point
contact law or source processing is responsible. Further pickup adjustment
alone has not solved it in this grid.

## Verification and retained artifact

All sixteen takes and eight refinement pairs qualify. Maximum relative total
energy defect is 1.567e-12, structural split defect 4.203e-14 and exchange
defect 2.202e-18. Maximum windowed voltage RMSE is 0.002869%; maximum impact
refinement error is 0.042589%, in the duration of both lowest-speed voicings.
No gate was relaxed. Numerical qualification remains separate from the failed
spectral comparison.

Both medium voicings reproduce the preceding voicing receipt exactly. All
three original-offset speeds reproduce the prior conditional sustain renders
after removing the newly observed impact fields. Source identities, measured
profiles and spring fit also match the earlier receipt exactly.

Verification passes 119 lab unit tests, thirteen loaded CLI/receipt tests,
strict Clippy and formatting. Tests independently recompute all training
scores, the minimum, reserved interpolation and reserved RMS; reject missing
dimensions; and check that reserved spectra cannot change the training score.
The receipt is 512707 bytes, SHA-256
`a13e4558979e2f283a3db997018f2dcaf648afaf3b7207dea7e3acc41af72951`:
[loaded-dynamics-validation.json](../references/loaded-dynamics-validation.json).
The release cache remains about 194 MiB and no WAVs are added.

## Scope and next decisions

The physical drive speed is the prescribed pedestal slew rate. The actual
hammer speed immediately before impact is separately observed, not equated
with that drive or with MIDI velocity. Impact force, impulse and duration help
distinguish a changed action trajectory from a changed pickup transfer.

This is one processed G3 source bank with prior exposure, unknown capture gain
and EQ/noise reduction. The reserved layers are same-bank checks, not blind
data from another instrument. Two ordinal directions do not cover arbitrary
velocity mappings. The small grid, conditional loss ambiguity and provisional
point-contact/flux laws remain explicit. No WAV matrix is written, no host is
opened and no listening verdict is claimed.

Next map the soft-strike threshold with observed hammer launch/contact speed,
actuator work and contact impulse. Use controlled action/escapement and contact
interventions to distinguish drive mapping from modal excitation before adding
material states or adopting a velocity curve. Preserve non-striking gestures
as such. Spatial recontact, release/repetition, other registers and realtime
implementation still require qualification before adopting a shared voicing
in the playable plugin.
