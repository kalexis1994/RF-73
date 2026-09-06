# Isolated pickup transfer

`pickup-transfer` compares the production magnetic law and an experimental field proxy with identical analytic motion. It separates transfer shape from contact integration and modal motion. The production instrument still uses its original law.

## Equations and provenance

For gap `g`, lateral offset `o` and tine displacement `x`, define `z=(o+x)/g`. Both laws use the same arbitrary scale `C=0.015`; this is not an independently calibrated voltage gain.

```text
Production:       Phi = C (1+z^2)^(-1/2)
                  V = C z v / [g (1+z^2)^(3/2)]
Point-pole proxy: Phi = C (1+z^2)^(-3/2)
                  V = 3 C z v / [g (1+z^2)^(5/2)]
Both:             V = -dPhi/dt
```

Pfeifle's DAFx 2017 paper, section 5.4, equation 6, gives an effective point-pole axial field proportional to axial distance divided by cubed separation. **Our inference:** holding axial distance `g` fixed and varying lateral separation `o+x` gives a normalized axial field `(1+z^2)^(-3/2)`. Using that field as flux linkage is an additional simplifying assumption. The paper's full pickup model includes spatial integration and two motion coordinates; this proxy does not implement those features. [Pfeifle, paper text](https://www.researchgate.net/publication/319644771_REAL-TIME_PHYSICAL_MODEL_OF_A_WURLITZER_AND_RHODES_ELECTRIC_PIANO), [official proceedings](https://www.dafx.de/paper-archive/2017/papers/DAFx17_paper_79.pdf).

There is no finite pole surface, magnetic loading, measured field map or electrical circuit in this experiment. Geometry values cannot be interpreted as identified instrument dimensions. Equal `C` does not make the two laws equally sensitive to a small displacement; raw RMS and Hn/H1 ratios are reported separately.

## Reproduce

```text
cargo run --locked --release -p rf-73-lab -- pickup-transfer --output renders/transfer-default.json
cargo run --locked --release -p rf-73-lab -- pickup-transfer --output renders/transfer-close.json --gap-mm 0.5 --offset-mm 0.25
cargo run --locked --release -p rf-73-lab -- pickup-transfer --output renders/transfer-treble.json --period-frames 16 --gap-mm 0.5 --offset-mm 0.25 --amplitudes-mm 0.75,3
```

The default sample rate is 44,100 Hz with 225 output frames per cycle: exactly 196 Hz, near G3 but not an exact equal-tempered MIDI note. `--sample-rate` accepts 44,100/48,000/96,000/192,000 Hz; `--period-frames` accepts integers 16..512. Frequency is always sample rate divided by period frames, so changing the rate alone also changes the excitation frequency. Gap/offset use the validated profile limits of 0.5..5 mm and -3..3 mm. `--amplitudes-mm` accepts 1..5 unique finite values in 0.005..3 mm, default 0.05/0.25/0.75. Displacement amplitudes are independent probes, not inferred strike velocities.

Every path samples one full period of `x=A cos(2*pi*f*t)` and its exact derivative at 4/8/16/32/64/128x density. No mechanical solver or resampling interpolation contributes motion error. The command writes a new JSON report and refuses an existing output; it does not write audio or change the plugin.

## Measurements

Complex Fourier peak coefficients use `2/N` scaling, with DC measured separately. The common ideal output band retains harmonics 1 through `floor(0.42*period_frames)` plus DC. Projection removes every component outside this band exactly in the mathematical definition; this is not the production FIR's response.

For each density, the residual against the finite 128x reference is measured with Parseval's identity:

```text
band power = DC^2 + (1/2) sum_k |coefficient_k|^2
NRMSE = sqrt(band power of coefficient differences / reference band power)
```

Complex differences preserve phase. Every tested density reports its NRMSE; the dB field is null below the conservative -160 dB numerical reporting floor. Such values indicate an unresolved residual at this reporting precision, not zero physical aliasing. Always inspect the 64x residual to assess reference convergence. The Fourier recurrence re-anchors phase every 1,024 samples to limit accumulated roundoff.

Reference harmonics include up to 12 raw peak amplitudes and their dB ratios to H1. Ratios are null when H1 or the component is below `1e-10 * reference band RMS`. In particular, a centered pickup driven by this symmetric trajectory has only even harmonics and an absent fundamental. There is no best-law ranking or reference-audio score.

This diagnoses aliasing introduced by internally sampling a smooth periodic transfer in a shared ideal output band. It excludes production decimator stopband leakage, attack transients, multiple mechanical modes, two-plane trajectories and real-time CPU budgets. A small residual here does not qualify the complete instrument's antialiasing. See [validation results](VALIDATION.md) and the preceding [G3 residual pilot](G3-RESIDUAL-PILOT.md).
