# Pickup residual diagnosis

The retained development spectra identify **H2 balance across register and
strike strength** as the first issue to investigate. A uniform harmonic boost
or cut would move important cases in opposite directions. This is a diagnostic
of the existing objective, not a new fit or a physical cause determination.

## Evidence and weighting

The analysis reads five frozen models' JSON receipts, not audio. It reproduces
each stored case score from signed residuals and preserves equal case weight:
each spectral term contributes `error_db^2 / (case_count * terms_in_case)`.
Grouped contributions below add to the original objective, in squared dB.
They are not within-group MSEs. Positive residual means the model harmonic is
too strong relative to its fundamental.

| Frozen model | H2 contribution | H3 contribution | H4 contribution | Total |
| --- | ---: | ---: | ---: | ---: |
| Calibrated, original mapping | 94.583 | 25.336 | 41.682 | 161.601 |
| Calibrated, remapped | 99.610 | 19.517 | 34.654 | 153.781 |
| Smooth register, remapped | 117.881 | 15.501 | 17.007 | 150.389 |
| Selected Production | 128.657 | 51.968 | 27.982 | 208.607 |
| Selected PointPole | 110.863 | 21.916 | 11.147 | 143.926 |

H2 accounts for 58.53% of original Calibrated error, 78.38% of smooth-register
error and 77.03% of PointPole error. The latter two improve H3/H4 in aggregate
while making H2 worse. Their lower totals do not resolve the failed per-note
and critical-case gates documented in the preceding studies.

## Register and dynamics matter

**C5 (MIDI 72) contributes 53.14% of original Calibrated error** despite being
one of seven equally weighted notes. Its reference H2 is much weaker than the
model: original Calibrated residuals are +25.35 to +32.55 dB across both
windows and four layers. C5 still contributes 61.41% of smooth-register error.

The earlier development receipt independently qualifies the C5 fundamental in
all four layers, at 523.456-523.534 Hz. This supports the note assignment; it
does not qualify every weak harmonic or establish recording-chain neutrality.
The analysis retains these pitch receipts and their input hash.

**G3/f (MIDI 55) has the opposite H2 sign.**

| Model | Attack H2 residual, dB | Body H2 residual, dB |
| --- | ---: | ---: |
| Original Calibrated | -8.85 | -10.18 |
| Smooth register | -24.66 | -12.11 |
| PointPole | -25.35 | -14.04 |

For smooth register, G3's H2 contribution rises by 3.345 squared dB against
the baseline; H3/H4 improvements recover only 2.057. This localizes the G3
regression, rather than attributing it generally to the whole spectrum.

## Attack/body behavior

Error is not concentrated only in the attack. Original Calibrated contributes
81.165 squared dB in attack and 80.435 in body. Smooth register contributes
76.113 and 74.276 respectively.

Some low-register cases also have the wrong direction of spectral evolution.
For G2/mp (MIDI 43), reference H2/H1 falls 16.77 dB between the windows; original
Calibrated rises 11.52 dB, smooth register rises 2.78 dB, and PointPole rises
25.01 dB. These are changes in relative harmonic balance, not directly measured
decay constants. H1 evolution, interference and windowing can also contribute.

## Coverage and limits

All five models share **160 of 168** possible development terms. Missing
reference terms remain excluded exactly as in the scorer:

- E4/p: body H4 missing.
- C5/p: attack/body H3 and H4 missing; only H2 contributes to this case.
- C5/mp: attack/body H4 missing.
- C5/mf: body H4 missing.

Equal case weighting gives each C5/p H2 observation three times the objective
weight of a term in a six-term case. This is existing policy, made visible by
the decomposition; no weights or acceptance thresholds were changed. None of
the retained terms is affected by the -60 dB scoring floor. Weak detected
source components still warrant a sensitivity check before optimizing them.

The compared models have different fitted ordinal mappings. These results
describe their frozen fits; they cannot isolate pickup geometry from actual
strike strength, source gain or recording-chain effects. The source's physical
strike velocities are undocumented.

## Next bounded experiment

Before adding a harmonic correction to the plugin:

1. Qualify weak C5 H2 observations against nearby spectral background and
   reasonable window/onset perturbations, using development recordings only.
2. Inspect how a bounded register-dependent geometry or excitation adjustment
   moves C5 and G3/f H2 together, retaining H3/H4 and existing per-note gates.
3. Track the low-register attack/body direction separately; do not interpret
   a static balance improvement as an envelope fix.

Step 1 is now complete: [C5 H2 qualification](C5-H2-QUALIFICATION.md) supports
the body observations in all four layers, flags the soft attacks, and confirms
that excluding two uncertain nominal terms does not make a frozen fit pass.

No held-out data was inspected, no new model parameters were fitted, and no
plugin version was produced. RackForge audition therefore was not run.

## Reproduction and checks

```powershell
$env:PYTHONDONTWRITEBYTECODE = '1'
python -m unittest discover -s tools -p 'test_matts*.py'
python tools/matts_residuals.py renders/matts-reference/residuals-new.json
```

The output path must be new. Ten Python tests pass: six existing gate tests
and four new tests for partial coverage weighting, additive differences,
signed temporal changes, missing candidate peaks and inconsistent inputs.
The unchanged Rust/DSP code retains its preceding 32-test validation; those
tests were not rerun for this receipt-only analysis.

[Machine-readable decomposition](../references/pickup-family-screen-2026-09-15/residual-decomposition.json)
includes input hashes, coverage, group contributions, every signed cell
residual and paired temporal changes. The new receipt has its own provenance
file; the earlier screen's provenance remains an immutable record of that run.
