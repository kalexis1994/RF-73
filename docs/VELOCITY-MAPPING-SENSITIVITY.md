# Ordinal velocity mapping sensitivity

Date: 2026-09-15. **Shared ordinal remapping does not make either frozen model
pass the development gates in the tested grids.** This is a reference-matching
study, not a change to the plugin's MIDI response or a recovered hammer law.

## Question and controls

Earlier studies paired p/mp/mf/f with model velocities 0.25/0.45/0.65/0.85.
The recording does not document actual strike speeds. This experiment allows
each of two frozen families to choose its own strictly increasing four-layer
mapping, shared across every development note:

- Current Calibrated, with its existing discrete pickup and mechanics.
- The selected smooth-register continuous-disk candidate, with geometry and
  physical profile frozen at the preceding study's result.

Both use the same source files, H2–H4 objective, development notes and
constraint-first selection against Calibrated's original fixed pairing.
This gives both families the same matching freedom. It does not independently
validate either model's interpretation of the recorded strikes.

## Search and implementation

The new `--velocity-map` mode first renders the required source/model cases
and retains their spectral costs in memory. It then enumerates layer mappings
without rerendering audio. Full spectra are retained for selected mappings;
compact per-case costs retain the entire measured grid.

- Coarse stage: velocity 0.10 through 1.00, spaced by 0.05. There are **3,876**
  strictly increasing four-layer mappings per family.
- Fine stage: each family's selected layer velocity ±0.05, spaced by 0.01;
  **14,641** ordered mappings per family for these selected neighborhoods.
- Total: **37,034 mapping evaluations**, including overlap between stages.
  The renders cover 1,680 short source/model cases, kept in memory rather than
  written as a WAV matrix.
- Development stays at MIDI 43/47/50/55/59/64/72 × four layers. Reserved notes
  MIDI 45/53/60/67/76/86 are loaded only if either frozen family passes the
  development gates. Neither did, so they remain unused by these fits.

The coarse grid is exhaustive within its stated discrete values. Fine search
is exhaustive only within the selected local neighborhoods. Neither establishes
a global continuous optimum or rules out note-dependent performance dynamics.

## Results

| Frozen family | Selected p / mp / mf / f | Development MSE reduction vs original Calibrated pairing | Largest note/critical-case regression |
| --- | --- | ---: | ---: |
| Calibrated | 0.29 / 0.44 / 0.61 / 0.85 | 4.84% | 2.22% |
| Smooth register | 0.25 / 0.50 / 0.63 / 0.85 | 6.94% | 13.63% |

Calibrated satisfies the no-large-regression limits but misses the required
10% global reduction. The register candidate also misses that global reduction
and regresses G3 by 13.63%, above the allowed 10%.

The register candidate's critical cases improve: B2/mp error falls by 31.70%
and E4/f by 33.41%. Their success still cannot override the failed note/global
constraints. Its B2 mean now improves by 12.55%, showing that ordinal pairing
explains part of the previous B2 mismatch.

Relative to the independently remapped Calibrated baseline, the register
candidate's aggregate advantage is only approximately **2.21%**. This is a
development comparison of a spectral metric, not evidence of an audible gain.
No held-out envelope check, listening conclusion or plugin promotion is claimed.

## Interpretation

The fixed ordinal pairing is a meaningful source of uncertainty, but a shared
four-layer remapping alone does not resolve the residuals under the tested
conditions. These mappings should remain fitting variables when comparing
models; they must not be copied into the plugin as a supposedly measured MIDI
velocity curve. Source gain, actual performance dynamics and release timing
remain undocumented.

Before adding more register parameters, inspect G3 and high-register residuals
under these separately optimized mappings and compare other existing pickup
laws under the same matching freedom. Changing geometry and the assumed source
velocities simultaneously without such controls can overstate improvement.

## Verification and artifacts

An independent Python enumeration over the JSON costs (no Python audio
processing) reproduces all four coarse/fine selections exactly. Rust tests
verify complete ordered enumeration and recover a known optimum with a shared
mapping across synthetic notes. Validation passes: **29 Rust tests, six Python
gate tests**, formatting, and release Clippy with warnings denied.

[Retained receipts](../references/velocity-mapping-2026-09-15/) contain both
grids, selected spectra, decisions, independent replay and provenance. The
retained protocol's count field explicitly refers to the coarse grid; actual
fine-stage counts are in the selection receipts and replay. Provenance records
this metadata clarification and the hashes of the original local protocols.

## Reproduction

From the workspace in PowerShell, using new output directories:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
$runner = 'references/matts-reference-runner/Cargo.toml'
$samples = 'references/audio/matts-fender-rhodes/samples/original'
$seed = 'references/register-pickup-fit-2026-09-15/refined/search.json'
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --velocity-map $samples renders/matts-reference/mapping-new $seed
cargo run --locked --release --manifest-path $runner --bin continuous_pickup_fit -- --velocity-map $samples renders/matts-reference/mapping-fine-new $seed renders/matts-reference/mapping-new
```

Fine refinement requires a previous mapping directory with untouched holdouts.
The plugin and source WAVs are unchanged. No playable version was produced,
so RackForge audition was not invoked.
