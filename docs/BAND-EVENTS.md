# Temporal limits of paired band-envelope measurements

Agreement between the 32 and 64 ms measurements does not establish natural
sustain. The fixed temporal study accepts 53 of 96 pairs with an event in the
requested interval, including prescribed release and loss changes. These rates
remain conditional observations and must not become natural mechanical losses.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- study-band-events --output renders/band-events.json
```

The [receipt](../references/band-events-validation.json) uses experiment
`paired-band-event-coverage-v1`, schema 1. It contains 162 pairs (324 individual
measurements), with compact fit, margin, qualification and support diagnostics.
Full coefficient arrays and WAVs are omitted to bound storage; waveforms are
reproducible from the recorded equations. Outputs must be new JSON files.

This is a coverage study, not an expectation that every event will be detected.
Only steady/absent controls have pass requirements. The command preserves all
event outcomes and fails after writing the report if those controls fail.
No filter, estimator gate, interval or event time was selected after observing
the results. The earlier 63/66 [single-window study](BAND-ENVELOPE.md) remains
unchanged, including its three failed onset expectations.

## Prescribed signals

The 0.25 s clips use 44.1, 48 and 96 kHz. Every rate has a steady and an absent
target control, then four events at each of these requested times:

```text
0, 0.008, 0.020, 0.028, 0.040, 0.060, 0.100,
0.140, 0.164, 0.176, 0.180, 0.188, 0.204 seconds
```

Event index is `ceil(time*sample_rate)` and actual event time is that index
divided by sample rate. Onset is zero before its index, then the prescribed
absolute-time 8 /s exponential. The other events keep amplitude continuous:

```text
A(t) = 0.002 * exp(-8*min(t, event) - rate_after*max(t-event, 0))
```

The release proxy switches to 40 /s, loss increase to 12 /s, and loss decrease
to 4 /s. This release is a controlled envelope change, not a validated physical
damper simulation. Target carrier/phase are 1620 Hz and 0.73 rad. Interference
matches the earlier full mixture: 196.35/1568/392.7/589.05 Hz, amplitudes
0.6/0.02/0.05/0.015, phases 0.1/-0.4/0.9/-0.7, rates 0.33/1/0.66/0.99 /s.
All cases add uniform noise amplitude 0.00001, LCG64 seed `0x73ba11`.

Every measurement uses the same declared 1620/1568 Hz carriers, centered FIR,
0.02..0.18 s crop, 32/64 ms windows and 8 ms hop. Both windows must qualify;
their rates must agree within `max(0.5 /s, 15% of the larger absolute rate)`.
The descriptive mean is withheld on any failure. Pairing adds no new onset
detector or mechanical inference.

## Observed coverage

All six controls behave as intended. Of 156 event pairs, 96 place the event in
the half-open measurement interval, including its start boundary. The remaining
events are before filter support, in a filter halo or after filter support.
These regions come from synthesis and exact sample indices, not audio inference.

| Event inside measurement interval | Accepted pairs | Total pairs |
|---|---:|---:|
| Onset | 3 | 24 |
| Release proxy, 8 to 40 /s | 15 | 24 |
| Loss increase, 8 to 12 /s | 19 | 24 |
| Loss decrease, 8 to 4 /s | 16 | 24 |

These counts describe this grid, not a population detection probability.
For a 48 kHz release proxy:

| Event time | Paired outcome | Conditional rate /s |
|---|---|---:|
| 0.020 s | Accepted | 40.051857 |
| 0.040 s | Accepted | 40.103816 |
| 0.100 s | Rejected by both windows | Withheld |
| 0.164 s | Accepted | 7.985943 |
| 0.176 s | Accepted | 8.011724 |

An early release can produce a plausible post-release rate near 40 /s; a late
release can leave a plausible rate near the earlier 8 /s. Neither establishes
that 8 /s is the instrument's natural loss. The middle event illustrates why
agreement of provisional rates alone is also insufficient: both fits are near
23.99 /s at 0.10 s, but both windows reject.

Sample rounding and window support matter. The 44.1 kHz loss-increase/decrease
cases at 0.14 s pass, while their 48/96 kHz counterparts reject. The report keeps
actual hop/window durations, first/last centers and the last window's exclusive
sample end. It also records the used source end including the FIR halo. The
filter's whole-crop support is a conservative bound; not every filtered sample
enters a complete regression window.

## What remains identifiable

Events after the used support cannot affect an observation. A unit test verifies
that a 0.204 s rate change has exactly the same source samples over the complete
filter support as the steady control. A delayed onset there is likewise identical
to the absent control over that support. No decision based on those samples can
recover the differing later history.

Every case therefore reports `natural_sustain_status: not_identified_by_measurement`,
even when `paired_qualified` is true. The known synthetic event does not get fed
back into the pair decision. Qualification means agreement under a local signal
model; it is not a key-release annotation, displacement-mode identity, or proof
of free mechanical decay. No inferred source damping is written anywhere.

The next useful physical-model step is a controlled mechanical observation with
known excitation and damper timing, checking what losses the estimator can
recover when those states are observable. The processed reference bank can still
provide conditional spectral/timbral comparisons, but unknown note-off and modal
origin prevent using these short slopes alone to calibrate natural losses.
Further threshold tightening cannot resolve histories outside observed support.

Production DSP, pickup law, spring geometry, presets and source WAVs are unchanged.
No host launch, listening result or packaged release is claimed.

The subsequent [controlled mechanical study](MECHANICAL-LOSS.md) separates
structural and damper power using known full-state trajectories and predicts
held-out energy drops. Its successful two-scale recovery does not remove the
audio observation and event-history limits established here.
