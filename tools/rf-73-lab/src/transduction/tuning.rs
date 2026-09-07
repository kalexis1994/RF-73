//! Spring tuning and output qualification of the complete polarized transducer.
use rf_73_analysis::{AudioClip, ToneComparisonOptions, compare_tone, pitch_anchor};
use rf_73_dsp::{
    ElectromechanicalAssembly, ElectromechanicalProfile, ModalSpectrum, ProductionDecimator,
    TineModes,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Read},
    path::Path,
};

pub const HELP: &str = "Loaded two-plane tuning:
  tune-electromechanical REFERENCE.json --output AUDIO.wav
Qualified frozen G3 reference; fixed 70 mm tine, 0.1 g point mass.
Move spring from 59.5 toward 35 mm from root with physical mode tracking.
Writes tuned AUDIO.wav, AUDIO-before.wav and AUDIO.json. Four 2.5-second
takes at 128/256 ticks, fixed 0.1 FS/V, two key gestures, no normalization.
Rejects existing outputs. Retains failed qualification. Offline only.
";
const RATE: u32 = 48000;
const FRAMES: usize = 120000;
const GAIN: f64 = 0.1;
const INITIAL: f64 = 0.85;

fn profile(position: f64) -> ElectromechanicalProfile {
    let mut p = ElectromechanicalProfile::default();
    p.geometry.length_m = 0.07;
    p.geometry.tuning_position = position;
    p
}
struct Selected {
    position: f64,
    spectrum: ModalSpectrum<18>,
    index: usize,
    fields: Vec<Vec<f64>>,
    fixed_root: [f64; 6],
}
impl Selected {
    fn frequency(&self) -> f64 {
        self.spectrum.modes[self.index].frequency_hz
    }
    fn row(&self, mac: f64) -> Value {
        json!({"spring_center_from_root_mm":70.0*self.position,"spring_position_fraction":self.position,
            "selected_index":self.index,"selected_frequency_hz":self.frequency(),"shape_mac":mac,
            "maximum_mass_orthogonality_error":self.spectrum.maximum_mass_orthogonality_error,
            "fixed_root_frequencies_hz":self.fixed_root,"fixed_root_ratios":self.fixed_root.map(|f|f/self.fixed_root[0]),
            "coupled_modes":self.spectrum.modes.iter().map(|m|json!({"frequency_hz":m.frequency_hz,"ratio_to_selected":m.frequency_hz/self.frequency(),
                "vertical_first_tine_projection":m.first_tine_projection,"hammer_weight":m.hammer_weight,"vertical_pickup_weight":m.pickup_weight,
                "relative_eigen_residual":m.relative_eigen_residual})).collect::<Vec<_>>()})
    }
}
fn selected(position: f64) -> Result<Selected, Box<dyn Error>> {
    let p = profile(position);
    let spectrum = ModalSpectrum::<18>::prepare_polarized(p.geometry, p.assembly, p.polarization)?;
    let basis = TineModes::prepare(p.geometry, 64)?;
    // Compare physical displacement fields, not coordinates in changing modal bases.
    // A common positive metric keeps the reference spring at the initial position.
    let quadrature = p.geometry.beam_mass_quadrature(64)?;
    let mut fields = Vec::new();
    for mode in &spectrum.modes {
        let mut field = Vec::new();
        for plane in 0..2 {
            let q = &mode.shape[9 * plane..9 * plane + 9];
            let displacement = |x: f64| -> Result<f64, Box<dyn Error>> {
                let mut y = q[0] + q[1] * p.geometry.length_m * x;
                for i in 0..6 {
                    y += q[2 + i] * basis.shape(i, x)?;
                }
                Ok(y)
            };
            for &(x, m) in &quadrature {
                field.push(displacement(x)? * m.sqrt());
            }
            field.push(q[0] * p.assembly.support_mass_kg.sqrt());
            field.push(q[1] * p.assembly.support_inertia_kg_m2.sqrt());
            field.push(
                (q[0] + q[1] * p.assembly.tonebar_arm_m + q[8]) * p.assembly.tonebar_mass_kg.sqrt(),
            );
            field.push(displacement(INITIAL)? * p.geometry.tuning_mass_kg.sqrt());
        }
        fields.push(field);
    }
    let index = (0..18)
        .max_by(|&a, &b| {
            spectrum.modes[a]
                .first_tine_projection
                .total_cmp(&spectrum.modes[b].first_tine_projection)
        })
        .unwrap();
    Ok(Selected {
        position,
        spectrum,
        index,
        fields,
        fixed_root: basis.modes.map(|m| m.frequency_hz),
    })
}
fn follow(a: &Selected, position: f64) -> Result<(Selected, f64), Box<dyn Error>> {
    let mut b = selected(position)?;
    let x = &a.fields[a.index];
    let dot = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(x, y)| x * y).sum::<f64>();
    let mut scores: Vec<_> = b
        .fields
        .iter()
        .enumerate()
        .map(|(i, y)| (i, dot(x, y).powi(2) / (dot(x, x) * dot(y, y))))
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    if scores
        .iter()
        .any(|(_, s)| !s.is_finite() || !(0.0..=1.0 + 1e-8).contains(s))
        || scores[0].1 < 0.98
        || scores[1].1 > 0.05
    {
        return Err("ambiguous polarized mode tracking".into());
    }
    b.index = scores[0].0;
    Ok((b, scores[0].1))
}
fn fit(target: f64) -> Result<(Selected, Value), Box<dyn Error>> {
    if !target.is_finite() || !(150.0..=250.0).contains(&target) {
        return Err("target outside frozen G3 pilot range".into());
    }
    let mut low = selected(INITIAL)?;
    let mut rows = vec![low.row(1.0)];
    if low.frequency() >= target {
        return Err("target not above initial branch frequency".into());
    }
    let mut high = None;
    for step in 1..=70 {
        let (next, score) = follow(&low, INITIAL - 0.005 * f64::from(step))?;
        rows.push(next.row(score));
        if next.frequency() <= low.frequency() {
            return Err("spring branch is not increasing inward".into());
        }
        if next.frequency() >= target {
            high = Some(next);
            break;
        }
        low = next;
    }
    let mut high = high.ok_or("target unreachable within spring travel")?;
    for _ in 0..32 {
        let position = 0.5 * (low.position + high.position);
        let (next, score) = follow(&low, position)?;
        let (reverse, _) = follow(&high, position)?;
        if next.index != reverse.index {
            return Err("bracket endpoints disagree on mode identity".into());
        }
        let cents = 1200.0 * (next.frequency() / target).log2();
        rows.push(next.row(score));
        if cents.abs() < 0.0001 {
            return Ok((
                next,
                json!({"target_hz":target,"undamped_error_cents":cents,"evaluations":rows,"bounds_mm":[35.0,59.5],
            "tracking":"Two-plane physical fields with beam quadrature, support/bar inertia and fixed initial spring metric; MAC >= 0.98, runner-up <= 0.05; both bracket directions agree"}),
            ));
        }
        if cents < 0.0 {
            low = next;
        } else {
            high = next;
        }
    }
    Err("polarized spring tuning budget exhausted".into())
}
struct Take {
    samples: Vec<f64>,
    summary: Value,
}
fn take(position: f64, steps: usize) -> Result<Take, Box<dyn Error>> {
    let p = profile(position);
    let h = 1.0 / (f64::from(RATE) * steps as f64);
    let mut model = ElectromechanicalAssembly::new(h, p)?;
    let mut old = model.probe();
    let mut x = p.action.hammer_rest_m;
    let mut filter = ProductionDecimator::new();
    let mut samples = Vec::with_capacity(FRAMES);
    let mut balance = 0.0_f64;
    let mut exchange = 0.0_f64;
    let mut passive = 0.0_f64;
    let mut heat_monotone = true;
    for frame in 0..FRAMES {
        let mut average = 0.0;
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = if (0.03..1.85).contains(&t) || (2.1..2.32).contains(&t) {
                -p.action.escapement_m
            } else {
                p.action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            model.advance(x, p.action.damper_closed_m)?;
            let b = model.probe();
            let scale =
                (b.mechanical.initial_energy_j + b.mechanical.absolute_drive_work_j).max(1e-20);
            balance = balance.max(b.total_balance_residual_j.abs() / scale);
            exchange = exchange.max(b.exchange_residual_j.abs() / scale);
            if x == old.mechanical.pedestal_position_m {
                passive = passive.max(
                    (b.mechanical.mechanical_energy_j + b.electrical_energy_j
                        - old.mechanical.mechanical_energy_j
                        - old.electrical_energy_j)
                        / scale,
                );
            }
            heat_monotone &= b.coil_heat_j >= old.coil_heat_j && b.load_heat_j >= old.load_heat_j;
            average += 0.5 * (old.output_voltage_v + b.output_voltage_v) / (steps / 4) as f64;
            old = b;
            if (sub + 1) % (steps / 4) == 0 {
                filter.push(average);
                average = 0.0;
            }
        }
        samples.push(GAIN * filter.output());
    }
    let peak = samples.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    let rms = (samples.iter().map(|x| x * x).sum::<f64>() / FRAMES as f64).sqrt();
    let passed = balance < 1e-8
        && exchange < 1e-10
        && passive < 1e-10
        && heat_monotone
        && peak > 0.0
        && peak < 1.0
        && old.mechanical.contact_entries[0] >= 2;
    Ok(Take {
        samples,
        summary: json!({"steps_per_frame":steps,"passed":passed,"peak":peak,"rms":rms,"max_relative_total_balance_defect":balance,
        "max_relative_exchange_defect":exchange,"max_stationary_drive_energy_growth":passive,"heat_monotone":heat_monotone,
        "hammer_contact_entries":old.mechanical.contact_entries[0],"maximum_coupling_iterations":old.maximum_iterations,
        "coil_heat_j":old.coil_heat_j,"load_heat_j":old.load_heat_j}),
    })
}
fn compare_resolution(a: &Take, b: &Take) -> Value {
    let windows:Vec<_>=[(0.0,0.25),(0.25,0.762),(0.762,1.274),(1.274,1.786),(1.786,2.5)].into_iter().map(|(lo,hi)| {
        let range=(lo*f64::from(RATE)).round() as usize..(hi*f64::from(RATE)).round() as usize;
        let mut e=0.0;let mut s=0.0;
        for (x,y) in a.samples[range.clone()].iter().zip(&b.samples[range]) {e+=(x-y).powi(2);s+=y*y;}
        let error=(e/s.max(1e-30)).sqrt();
        json!({"start_seconds":lo,"end_seconds":hi,"voltage_relative_rmse":error,"passed":error<0.01})
    }).collect();
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
fn write_wav(path: &Path, take: &Take) -> Result<(), Box<dyn Error>> {
    let mut wav =
        crate::wav::FloatWav::new(BufWriter::new(crate::new_file(path)?), RATE, FRAMES as u32)?;
    for &x in &take.samples {
        wav.sample(x as f32)?;
    }
    wav.finish()?;
    Ok(())
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[2] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    if output.extension().is_none_or(|x| x != "wav") {
        return Err(HELP.into());
    }
    let before = output.with_file_name(format!(
        "{}-before.wav",
        output
            .file_stem()
            .and_then(|x| x.to_str())
            .ok_or("invalid output stem")?
    ));
    let report = output.with_extension("json");
    for p in [output, before.as_path(), report.as_path()] {
        if p.exists() {
            return Err("tuning requires new WAV pair and JSON paths".into());
        }
    }
    let mut bytes = Vec::new();
    File::open(&args[1])?.take(262145).read_to_end(&mut bytes)?;
    if bytes.len() > 262144 {
        return Err("frequency reference exceeds 256 KiB".into());
    }
    let reference: Value = serde_json::from_slice(&bytes)?;
    let target = crate::memory_modal_check::validate_pitch_reference(&reference)?;
    let result = (|| -> Result<Value, Box<dyn Error>> {
        let (tuned, fit) = fit(target)?;
        let baseline = selected(INITIAL)?;
        println!(
            "Selected spring center {:.6} mm; structural frequency {:.9} Hz",
            70.0 * tuned.position,
            tuned.frequency()
        );
        let mut cases = Vec::new();
        for (label, position, path) in [
            ("before", INITIAL, before.as_path()),
            ("tuned", tuned.position, output),
        ] {
            println!("Rendering {label}, 128 ticks");
            let coarse = take(position, 128)?;
            println!("Rendering {label}, 256 ticks");
            let fine = take(position, 256)?;
            let convergence = compare_resolution(&coarse, &fine);
            write_wav(path, &fine)?;
            let clip = AudioClip::open(path, Some(0))?;
            let anchor = pitch_anchor(&clip, 55)?;
            let cents = anchor.frequency_hz.map(|f| 1200.0 * (f / target).log2());
            let passed = coarse.summary["passed"] == true
                && fine.summary["passed"] == true
                && convergence["passed"] == true
                && anchor.qualified
                && (label == "before" || cents.is_some_and(|c| c.abs() < 5.0));
            cases.push(json!({"case":label,"passed":passed,"spring_center_from_root_mm":70.0*position,"takes":[coarse.summary,fine.summary],
                "voltage_convergence":convergence,"pitch_anchor":anchor,"output_error_cents":cents}));
        }
        let a = AudioClip::open(&before, Some(0))?;
        let b = AudioClip::open(output, Some(0))?;
        let tone = compare_tone(
            &a,
            &b,
            ToneComparisonOptions {
                note: 55,
                reference_start_seconds: 0.03,
                candidate_start_seconds: 0.03,
            },
        )?;
        Ok(
            json!({"schema_version":1,"experiment":"loaded-polarized-spring-tuning-v1","passed":cases.iter().all(|c|c["passed"]==true),
            "frozen_reference":reference,"structural_fit":fit,"before_structure":baseline.row(1.0),"tuned_structure":tuned.row(1.0),
            "cases":cases,"tone_comparison":tone,"gain_fs_per_volt":GAIN,"sample_rate":RATE,"frames":FRAMES,
            "protocol":"Frozen before first run. 70 mm circular tine, 0.1 g point mass, spring center 59.5 toward 35 mm; default polarized action and 10k circuit. Undamped mode tuning within 0.0001 cent, no output frequency feedback. Two gestures with 1.5 m/s pedestal slew, key down 0.03-1.85 and 2.1-2.32 s, closed pedal. 128/256 ticks, average to 4x then common FIR, fixed 0.1 FS/V. Require energy defect <1e-8, exchange/growth <1e-10, monotone heat, two strikes and headroom. Per-window voltage refinement <1%; broad three-window pitch anchor must qualify and tuned output must lie within 5 cents of the frozen target. Save raw before/after spectral balance and nonharmonic structural ratios, without a timbre improvement gate.",
            "scope":"Synthetic tuning transfer to loaded polarized output. No measured geometry, real-instrument timbre match, complete aliasing bound or realtime plugin qualification. Structural modal frequencies exclude circuit/contact loading; waveform pitch is checked independently."}),
        )
    })();
    let value = match result {
        Ok(v) => v,
        Err(e) => {
            crate::analysis::write_report(
                &report,
                &json!({"schema_version":1,"experiment":"loaded-polarized-spring-tuning-v1","passed":false,"reason":e.to_string()}),
            )?;
            return Err(e);
        }
    };
    crate::analysis::write_report(&report, &value)?;
    if value["passed"] != true {
        return Err("loaded tuning retained failed qualification".into());
    }
    println!("Loaded polarized tuning passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_reference_replays_the_structural_fit_with_current_validation() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../references/loaded-polarized-spring-tuning-validation.json");
        let r: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let target =
            crate::memory_modal_check::validate_pitch_reference(&r["frozen_reference"]).unwrap();
        let (_, fresh) = fit(target).unwrap();
        let fresh: Value = serde_json::from_slice(&serde_json::to_vec(&fresh).unwrap()).unwrap();
        assert_eq!(fresh, r["structural_fit"]);
    }
    #[test]
    fn spring_fit_tracks_both_directions_and_changes_nonharmonic_structure() {
        let target = 196.386147;
        let a = selected(INITIAL).unwrap();
        let (b, _) = fit(target).unwrap();
        assert!((1200.0 * (b.frequency() / target).log2()).abs() < 0.0001);
        assert!(b.position < a.position);
        let (back, _) = follow(&b, b.position + 0.0001).unwrap();
        let (again, _) = follow(&back, b.position).unwrap();
        assert_eq!(b.index, again.index);
        assert!(
            (a.fixed_root[1] / a.fixed_root[0] - b.fixed_root[1] / b.fixed_root[0]).abs() > 0.001
        );
        assert!(fit(f64::NAN).is_err());
        assert!(fit(150.0).is_err());
    }
}
