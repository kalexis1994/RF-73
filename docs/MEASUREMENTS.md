# Measurement protocol

## Reference provenance

Record instrument model, year when known, restoration, regulation, modifications and capture point. Keep direct instrument output, preamp output and cabinet microphones separate. Desired source: mono 24-bit/96 kHz WAV, fixed gain, no clipping, normalization, compression, modulation or reverb. Record input impedance and interface settings.

The following values are our proposed protocol, not published standards.

## Initial session

Begin with A3 (MIDI 57): eight intensities and five repetitions at each intensity. Capture the complete decay down to the noise floor. Add releases at 0.2, 1 and 3 seconds, pedal gestures and fast repetitions. Then sample bass, midrange and treble anchors, including neighboring keys at construction changes.

A mechanically played Rhodes does not inherently report MIDI velocity. Measure hammer speed if possible; otherwise record intensity labels as approximate. Do not assign invented meters-per-second values to human pp/ff labels.

If a technician can document pickup movement, repeat takes with small known changes. Unknown geometry may be fitted as effective parameters, with that uncertainty stated explicitly.

## Evaluation

| Characteristic | Measurement |
| --- | --- |
| Tuning | Sustained frequency and drift in cents |
| Attack | Time waveform and spectra at multiple window lengths |
| Body | Harmonic and inharmonic components separately |
| Decay | Per-component slopes and changing slopes |
| Dynamics | Level and color versus intensity, without per-note normalization |
| Release | Damper envelope and transient |
| Aliasing | Aligned comparison to a higher-rate reference render |
| Musical response | Blind listening to phrases, chords, repeated keys and pedal |

Split fitting and validation by notes and intensities. Use one reference gain to evaluate dynamics, and a separate level-matched comparison for timbre. Account for variance between real repeated takes.

Provisional targets: sustained tuning within 3 cents, dominant component levels within 3 dB, and measurable decay times within 15%. Revise these after acquiring the dataset. They are not proof of perceptual equivalence.

## Current laboratory output

`render --trace` exports the latest internal state at each output frame: displacement, velocity, contact force, mechanical energy, raw pickup signal and filtered output. This output-rate CSV can miss a substep peak; tests run directly at the internal rate to check energy. JSON reports store input settings, rates, peak, RMS and numerical faults. Demo reports describe a fixed multi-event sequence, not a single-velocity reference take.

The `stress` command includes 73 simultaneous strikes and periodic retriggers under pedal in 128-frame blocks. It reports wall-clock worst block, p99 and deadline misses after warmup. It includes OS scheduling effects and is a short diagnostic rather than a qualification soak.

`analyze` reads external PCM/float WAV and reports tuning candidates, attack/body spectra, harmonic tracks, independent spectral tracks and RMS envelopes. Independent tracks carry background margins, resolution/capacity flags and per-track decay rejection reasons. `--sustain-end` explicitly bounds natural-decay fitting; no boundary means no decay estimate. `compare` reports both the original level difference and a separate RMS-matched error. The [analysis specification](ANALYSIS.md) defines the windows, normalization and limitations. Neither command estimates a calibrated hammer velocity or proves timbral fidelity.

## Without an instrument

Published models permit development, but calibration stays provisional. Obtain documented direct recordings from a player or studio. Public processed music can guide listening; it cannot uniquely identify mechanical parameters. Reference audio need not ship inside the plugin. Record source and permission for each dataset before incorporating it.
