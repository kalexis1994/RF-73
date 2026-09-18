# Aperture quadrature and the strong-hit H2 deficit

Date: 2026-09-15. **The retained 16-node pickup is materially different from
the uniform-disk integral at the Calibrated geometry.** This diagnoses part of
the harmonic behavior; it does not qualify a replacement factory sound.

## Why this check

The [Matt's calibration search](MATTS-CALIBRATION-SEARCH.md) improved average
harmonic error by widening the gap, but substantially worsened E4/f H2.
The [existing aperture documentation](PICKUP-APERTURE-PATH.md) already identifies
the internal node-ring slope reversal as a property of the discrete proxy.
Before fitting more geometry against that behavior, this study checks its
difference from a continuously filled disk with the same flux kernel.

## Independent computation

The current implementation differentiates a sum over two radial and eight
angular nodes. The new offline diagnostic computes the gradient of a uniform
disk by a boundary integral, independently of that node layout. With radius R,
gap g, offset o, displacement q and linkage scale s:

```text
Phi(q) = s g^3 / (pi R^2) integral_disk [g^2 + |q+o-r|^2]^(-3/2) dA
dPhi/dq = -2 s g^3/R * mean_theta [cos(theta) / d(theta)^3]
d(theta)^2 = g^2 + (q+o-R cos(theta))^2 + (R sin(theta))^2
```

The second identity follows from the divergence theorem. Four tests verify:
agreement with an independent area midpoint integral, the small-disk point
limit and reflection symmetry, Fourier gain invariance, and unchanged mechanical
trajectories when switching the voice's pickup law.

Six static displacement checks compare the boundary method with 128×512 and
256×1024 area integrations. The finer area result differs from the boundary
result by at most 3.39e-6 Wb/m at these points. For the 16 sine-motion transfer
cases, refining 128 to 256 boundary nodes changes the waveform by at most
5.92e-16 relative RMS. This is a selected-case numerical check, not a bound
over all permitted geometry or a validation of the physical flux kernel.

## Frozen mechanical trajectory comparison

G3 and E4 are rendered at velocities 0.25/0.45/0.65/0.85. Each pair shares the
same existing voice trajectory, 4× internal sample rate and production
decimator. Only the flux gradient differs. Harmonics are measured against the
same Matt's recordings, with onset alignment and ratios relative to H1.
No gain fit, hammer change or extra distortion is involved.

The reconstructed discrete path reproduces the previous Calibrated H2–H4
ratios within **0.000004 dB**, confirming that the comparison isolates the
pickup observation. Selected strong-hit body-window ratios:

| Note / harmonic | Recording | Current 16 nodes | Uniform disk |
| --- | ---: | ---: | ---: |
| G3 / H2 | -3.49 dB | -13.67 dB | -2.55 dB |
| G3 / H3 | -3.64 dB | +1.64 dB | -17.40 dB |
| E4 / H2 | -7.72 dB | -14.49 dB | -6.67 dB |
| E4 / H3 | -12.36 dB | -11.90 dB | -26.04 dB |

At E4/f attack96, H2 changes from -17.50 to -5.33 dB versus the recording's
-1.53 dB; H3 changes from -7.55 to -23.27 dB versus -5.70 dB. The continuous
disk removes much of this H2 deficit while making H3 too weak. Consequently,
replacing the existing law at unchanged geometry is not a complete improvement.
The current law's strong H3 cannot be interpreted as a numerically converged
uniform-aperture prediction for this geometry.

The sine-motion rows also retain raw waveform differences. Those include
gain and polarity differences and are not perceptual quality scores. The
frozen-trajectory comparison is limited to short harmonic observations;
it does not qualify decay, polyphony, CPU cost, aliasing or GUI/audio playback.

## Artifacts and reproduction

- [Measurement receipt](../references/aperture-diagnostic-2026-09-15.json):
  16 sine-transfer cases, six independent area checks, and 16 source/model
  comparisons (eight performances × two pickup laws).
- [Provenance](../references/aperture-diagnostic-2026-09-15.provenance.json).
- [Rust diagnostic](../tools/rf-tines-lab/examples/aperture_diagnostic.rs).

From the workspace in PowerShell:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
cargo run --locked --release --manifest-path references/matts-reference-runner/Cargo.toml --bin aperture_diagnostic -- renders/matts-reference/aperture-new.json references/audio/matts-fender-rhodes/samples/original
```

The report path must be new. Omit the source directory to run only analytic
transfer and independent integration checks. Validation passes: 18 Rust tests,
six Python gate tests, formatting, and release Clippy with warnings denied.
The runner reuses the existing build directory and writes no WAV matrix.

## Decision and next experiment

Completed follow-up: [continuous-disk calibration](CONTINUOUS-PICKUP-CALIBRATION.md)
implements and numerically checks the offline table, then fits geometry and
velocity response. It improves aggregate error but fails per-note validation.

Keep the existing plugin unchanged. Treat its aperture path as a fitted
discrete surrogate, not a converged disk integral. The next fit should compare
a numerically checked continuous-disk pickup and alternate retained transfer
laws while constraining both H2 and H3. Geometry and ordinal velocity remain
joint fitting variables; improved numerical integration alone does not prove
closer sound to a real Rhodes.

Fresh notes MIDI **40, 47, 62, 69, 79, 88** are reserved for the next validation.
This diagnostic did not load or fit those samples. No installable plugin
version was produced, so the RackForge audition workflow was not invoked.
