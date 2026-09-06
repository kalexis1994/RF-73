# Fit a pickup reference set

`fit-pickup-set` fits one pickup geometry and one gain across several takes, then evaluates that frozen choice on reserved notes or intensities. This extends the [single-take sweep](PICKUP-SWEEP.md) without changing the instrument profile. All rendering and scoring runs offline in Rust; only a JSON report is written.

## Synthetic recovery example

The checked-in [example manifest](../references/pickup-set.synthetic.json) describes generated audio, not a measured Rhodes. From the workspace, generate its three files and run:

```text
cargo run --locked --release -p rf-73-lab -- render --output renders/pickup-set-demo/soft.wav --note 57 --velocity 0.3 --seconds 1.5 --hold 1.4 --gap-mm 2 --offset-mm 0.75
cargo run --locked --release -p rf-73-lab -- render --output renders/pickup-set-demo/loud.wav --note 57 --velocity 0.7 --seconds 1.5 --hold 1.4 --gap-mm 2 --offset-mm 0.75
cargo run --locked --release -p rf-73-lab -- render --output renders/pickup-set-demo/held.wav --note 57 --velocity 0.5 --seconds 1.5 --hold 1.4 --gap-mm 2 --offset-mm 0.75
cargo run --locked --release -p rf-73-lab -- fit-pickup-set references/pickup-set.synthetic.json --output renders/pickup-set-demo/result.json
```

Use fresh filenames for another run and update a copy of the manifest accordingly. Paths inside a manifest resolve against its own directory, independently of the shell's current directory. Existing output files are never overwritten.

## Manifest contract

Schema 1 rejects unknown fields and limits JSON input to 64 KiB. Required set-level strings are `source`, `source_revision`, `license`, `instrument`, `processing` and `capture_gain`. Supply source URL/revision and recording permission where applicable. State unknown details explicitly. These fields document provenance; the tool does not independently verify them or compute source-file hashes. Preserve the source recordings and their acquisition records.

Use one instrument configuration, recording chain and fixed capture gain throughout a set. Per-file normalization or changes in recording gain invalidate a shared-gain dynamics comparison. The tool cannot infer or verify that condition from the audio. Document instrument year, pickup/hammer regulation, output tap and loading in `instrument`, `source` or the referenced acquisition record. `gaps_mm` and `offsets_mm` each contain 1..5 unique values within 0.5..5 and -3..3 mm respectively.

There must be 3..8 takes, at least two distinct fitting note/velocity pairs and at least one validation pair absent from fitting. Declare each take with:

| Field | Meaning |
| --- | --- |
| `id` | Nonempty unique take identifier |
| `file` | WAV path, absolute or relative to manifest directory |
| `role` | `fit` or `validation` |
| `note` | MIDI 28..100 |
| `velocity` | Explicit normalized model strike, 0.01..1 |
| `velocity_basis` | How that model input was chosen; human intensity labels are not measured hammer velocities |
| `reference_start_seconds` | Nonnegative position in the recording |
| `model_start_seconds` | Position after the generated strike, 0..2 seconds |
| `seconds` | Region duration, 0.128..4 seconds |
| `sustain_end_seconds` | End of the known held-note interval in recording time |
| `channel` | Zero-based selected channel; null/omitted only for mono |

The complete selected region must lie before the sustain boundary, and the boundary must lie inside the recording. Starts/duration round to samples; the rounded region is checked too. Every file must use the same supported rate: 44100, 48000, 96000 or 192000 Hz. WAV reader limits remain 60 seconds and 12 million frames per input. Silence, missing files, duplicate IDs and reuse of a canonical file path are rejected before fitting. Renamed copies/hard links are not detected as duplicate content: keeping independent takes is the dataset author's responsibility. Repeated fitting pairs are allowed; each repetition receives its own weight.

No automatic alignment, resampling, pedal or release fitting occurs. Candidates are held notes starting from a fresh default engine. Keeping validation pairs out of the manifest's fitting split prevents exact pair reuse; it does not establish that human intensities are distinct or prohibit manually tuning a model using prior validation results. Predeclare splits and preserve them.

## Selection and validation

For each grid geometry, render only fitting takes and calculate:

```text
gain = sqrt(sum(mean(reference_take^2)) / sum(mean(candidate_take^2)))
```

The sums include fitting takes only. This matches pooled energy with equal take weighting regardless of duration; louder takes contribute more energy to the gain estimate. It is an explicitly chosen normalization rule, not a gain optimized for the spectral objective.

Apply this one gain across all windows of all fitting takes. Each take uses the single-take sweep's 128 ms magnitude spectra, 64 ms hops and 80 dB reference-relative floor. The fit objective is the RMS of the per-take objectives, so each take contributes equally to selection. Inspect per-window coverage and residuals as well as the scalar score. A failed fitting take rejects the whole candidate; candidates cannot benefit from dropping a difficult take.

Rank geometries using fitting objectives alone. Exact ties retain grid order; the report includes candidates within 0.01 dB of the best. This is a reporting band, not a confidence interval or audibility threshold. Render validation takes only for the winning geometry and apply the already fitted gain unchanged. Validation results cannot change the winner, gain or ranking.

Schema-1 reports include the complete manifest, resolved input paths/metadata and actual frame regions, fixed mechanical parameters, every candidate's fitting metrics or rejection reason, selected index, near ties and individual validation results. `validation_objective_db` is present only when all validation takes succeed. `evaluation_complete` means computation completed; it is not an acoustic acceptance decision. An incomplete fit/validation writes a diagnostic report and exits unsuccessfully. Invalid input fails before creating a report.

Shared score fields distinguish `applied_candidate_gain` and `applied_gain_normalized_rmse` from the legacy diagnostics `candidate_gain_to_match_reference` and `level_matched_normalized_rmse`, which describe hypothetical independent per-take RMS matching. Those hypothetical values never enter set selection or validation. Window `level_matched_spectral_error_db` and `objective_db` always use the actual applied shared gain. The single-take command now includes the explicit applied-gain fields too; its existing ranking is unchanged.

## Limits

Synthetic recovery and deliberately mismatched validation fixtures check the workflow; they do not establish Rhodes fidelity. Geometry is provisional, the grid is coarse and magnitude-spectrum error is not a listening score. Hammer velocity, modal weights and pickup response can trade off. Do not reinterpret effective fitted millimeters as measured regulation.

For real calibration, acquire documented dry takes at fixed gain, estimate tuning and qualified modal decay first, choose fitting/validation intensities in advance and examine raw dynamics, partial residuals and listening pairs. Repeatedly revising the model using a reserved set makes that set part of fitting; acquire fresh held-out data for the next acceptance test.
