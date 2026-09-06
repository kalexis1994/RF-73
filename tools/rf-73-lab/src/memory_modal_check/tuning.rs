//! Spring-position tuning with spatial mode tracking and a retained listening pair.
use super::audio::{self, Options, RATE};
use rf_73_analysis::{AudioClip, pitch_anchor};
use rf_73_dsp::{ModalAssemblyProfile, ModalSpectrum, StructuralMode, TineGeometry, TineModes};
use serde_json::{Value, json};
use std::{error::Error, fs::File, io::Read, path::Path};

pub const HELP: &str = "Coupled modal spring tuning:
  tune-modal-pitch REFERENCE.json --output PATH.wav
Uses a qualified frequency-reference-preparation-v1 receipt for MIDI 55.
Fixed provisional 70 mm tine, 0.1 g point mass; slide its center inward from
59.5 mm toward 35 mm from the root. This is not measured spring geometry.
Tracks a coupled undamped mode by physical displacement-field continuity.
Writes PATH.wav (tuned), PATH-before.wav and PATH.json. Both takes: 2 s,
0.4 m/s memory hammer, damper at 1.85 s, common gain 0.084, no normalization.
Requires mechanical/audio gates on both takes and a qualified tuned pitch.
No keyboard calibration, plugin update, device launch or listening claim.
";
const LENGTH: f64 = 0.070;
const INITIAL_POSITION: f64 = 0.85;

pub(super) struct Selected {
    length: f64,
    mass: f64,
    span: f64,
    taper: f64,
    distributed_fields: usize,
    pub(super) position: f64,
    spectrum: ModalSpectrum,
    index: usize,
    fields: [Vec<f64>; 9],
    basis: TineModes,
    fixed_root: [f64; 6],
}
impl Selected {
    pub(super) fn mode(&self) -> StructuralMode {
        self.spectrum.modes[self.index]
    }
    pub(super) fn row(&self, mac: f64) -> Value {
        json!({"length_m":self.length,"tuning_mass_kg":self.mass,"tuning_span_m":self.span,"tip_diameter_ratio":self.taper,"spring_position_fraction":self.position,
            "spring_center_from_root_mm":1000.0*self.length*self.position,
            "selected_mode_index":self.index,"selected_frequency_hz":self.mode().frequency_hz,
            "shape_mac_from_previous":mac,"fixed_root_fundamental_hz":self.spectrum.fixed_root_fundamental_hz,
            "fixed_root_tine_modes":self.fixed_root.iter().enumerate().map(|(i,f)|json!({
                "ordinal":i+1,"frequency_hz":f,"ratio_to_first":f/self.fixed_root[0]})).collect::<Vec<_>>(),
            "maximum_mass_orthogonality_error":self.spectrum.maximum_mass_orthogonality_error,
            "coupled_modes_by_frequency_rank":self.spectrum.modes.iter().map(|m|json!({"frequency_hz":m.frequency_hz,
                "ratio_to_selected":m.frequency_hz/self.mode().frequency_hz,
                "first_tine_projection":m.first_tine_projection,"hammer_weight":m.hammer_weight,
                "pickup_weight":m.pickup_weight,
                "linear_hammer_to_pickup_velocity_residue_per_kg":m.hammer_weight*m.pickup_weight,
                "relative_eigen_residual":m.relative_eigen_residual})).collect::<Vec<_>>()})
    }
}
fn spectrum(length: f64, position: f64) -> Result<Selected, Box<dyn Error>> {
    spectrum_with_mass(length, TineGeometry::default().tuning_mass_kg, position)
}
pub(super) fn spectrum_with_mass(
    length: f64,
    mass: f64,
    position: f64,
) -> Result<Selected, Box<dyn Error>> {
    spectrum_with_span(length, mass, 0.0, position)
}
pub(super) fn spectrum_with_span(
    length: f64,
    mass: f64,
    span: f64,
    position: f64,
) -> Result<Selected, Box<dyn Error>> {
    spectrum_with_taper(length, mass, span, 1.0, position)
}
pub(super) fn spectrum_with_taper(
    length: f64,
    mass: f64,
    span: f64,
    taper: f64,
    position: f64,
) -> Result<Selected, Box<dyn Error>> {
    let g = TineGeometry {
        length_m: length,
        tuning_mass_kg: mass,
        tuning_span_m: span,
        tip_diameter_ratio: taper,
        tuning_position: position,
        ..TineGeometry::default()
    };
    let p = ModalAssemblyProfile::default();
    let spectrum = ModalSpectrum::prepare(g, p)?;
    let basis = TineModes::prepare(g, 64)?;
    // Reconstruct physical fields: modal coordinates alone are not comparable
    // when their fixed-root basis changes. Use the same positive reference
    // inertia (spring at 0.85 L) for every field, not a frequency proximity test.
    let quadrature = g.beam_mass_quadrature(64)?;
    let distributed_fields = quadrature.len() + 3;
    let mut fields: [Vec<f64>; 9] =
        core::array::from_fn(|_| Vec::with_capacity(distributed_fields + 1));
    let reference_spring = TineGeometry {
        tuning_position: INITIAL_POSITION,
        ..g
    }
    .tuning_mass_quadrature(64)?;
    for (mode, field) in spectrum.modes.iter().zip(&mut fields) {
        let q = mode.shape;
        let displacement = |x: f64| -> Result<f64, Box<dyn Error>> {
            let mut y = q[0] + q[1] * length * x;
            for i in 0..6 {
                y += q[2 + i] * basis.shape(i, x)?;
            }
            Ok(y)
        };
        for &(x, mass) in &quadrature {
            field.push(displacement(x)? * mass.sqrt());
        }
        field.push(q[0] * p.support_mass_kg.sqrt());
        field.push(q[1] * p.support_inertia_kg_m2.sqrt());
        field.push((q[0] + p.tonebar_arm_m * q[1] + q[8]) * p.tonebar_mass_kg.sqrt());
        for &(position, fraction) in &reference_spring {
            field.push(displacement(position)? * (g.tuning_mass_kg * fraction).sqrt());
        }
    }
    let index = (0..9)
        .max_by(|&a, &b| {
            spectrum.modes[a]
                .first_tine_projection
                .total_cmp(&spectrum.modes[b].first_tine_projection)
        })
        .unwrap();
    Ok(Selected {
        length,
        mass,
        span,
        taper,
        distributed_fields,
        position,
        spectrum,
        index,
        fields,
        fixed_root: basis.modes.map(|m| m.frequency_hz),
        basis,
    })
}
fn mac(a: &Selected, b: &Selected, index: usize) -> f64 {
    let x = &a.fields[a.index];
    let y = &b.fields[index];
    let product = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(x, y)| x * y).sum::<f64>();
    product(x, y).powi(2) / (product(x, x) * product(y, y))
}
pub(super) fn follow(a: &Selected, position: f64) -> Result<(Selected, f64), Box<dyn Error>> {
    follow_with_metric(a, position, false)
}
// The wider geometry study must not measure remote positions using the initial
// spring inertia: modes there need not be orthogonal in that obsolete metric.
// Average the two actual physical inertias for each local comparison instead.
fn local_mac(a: &Selected, b: &Selected, index: usize) -> Result<f64, Box<dyn Error>> {
    if a.length != b.length || a.mass != b.mass || a.span != b.span || a.taper != b.taper {
        return Err("local spring metric requires fixed cell geometry and mass".into());
    }
    let product = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(x, y)| x * y).sum::<f64>();
    // Leading entries contain distributed beam, support and tonebar inertia.
    // Discard the initial-position spring samples and integrate locally below.
    let x = &a.fields[a.index][..a.distributed_fields];
    let y = &b.fields[index][..b.distributed_fields];
    let (mut xx, mut yy, mut xy) = (product(x, x), product(y, y), product(x, y));
    let displacement = |s: &Selected, index: usize, position: f64| -> Result<f64, Box<dyn Error>> {
        let q = s.spectrum.modes[index].shape;
        let mut value = q[0] + q[1] * s.length * position;
        for i in 0..6 {
            value += q[i + 2] * s.basis.shape(i, position)?;
        }
        Ok(value)
    };
    for center in [a.position, b.position] {
        let g = TineGeometry {
            length_m: a.length,
            tuning_mass_kg: a.mass,
            tuning_span_m: a.span,
            tip_diameter_ratio: a.taper,
            tuning_position: center,
            ..TineGeometry::default()
        };
        for (position, fraction) in g.tuning_mass_quadrature(64)? {
            let u = displacement(a, a.index, position)?;
            let v = displacement(b, index, position)?;
            xx += 0.5 * a.mass * fraction * u * u;
            yy += 0.5 * a.mass * fraction * v * v;
            xy += 0.5 * a.mass * fraction * u * v;
        }
    }
    Ok(xy * xy / (xx * yy))
}
pub(super) fn follow_local(a: &Selected, position: f64) -> Result<(Selected, f64), Box<dyn Error>> {
    follow_with_metric(a, position, true)
}
fn follow_with_metric(
    a: &Selected,
    position: f64,
    local: bool,
) -> Result<(Selected, f64), Box<dyn Error>> {
    let mut next = spectrum_with_taper(a.length, a.mass, a.span, a.taper, position)?;
    let mut scores = Vec::with_capacity(9);
    for i in 0..9 {
        scores.push((
            i,
            if local {
                local_mac(a, &next, i)?
            } else {
                mac(a, &next, i)
            },
        ));
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    if scores
        .iter()
        .any(|(_, x)| !x.is_finite() || *x < 0.0 || *x > 1.0 + 1e-8)
        || scores[0].1 < 0.98
        || scores[1].1 > 0.05
    {
        return Err("structural branch is ambiguous or spatial continuity fell below 0.98".into());
    }
    next.index = scores[0].0;
    Ok((next, scores[0].1))
}
fn fit(target: f64) -> Result<(Selected, Value), Box<dyn Error>> {
    let mut previous = spectrum(LENGTH, INITIAL_POSITION)?;
    if !target.is_finite() || target <= previous.mode().frequency_hz {
        return Err("target is outside the declared inward spring search".into());
    }
    if previous.mode().first_tine_projection < 0.5 {
        return Err("no dominant initial tine coordinate".into());
    }
    let mut rows = vec![previous.row(1.0)];
    let mut bracket = None;
    for step in 1..=14 {
        let (next, score) = follow(&previous, INITIAL_POSITION - step as f64 * 0.025)?;
        rows.push(next.row(score));
        if next.mode().frequency_hz <= previous.mode().frequency_hz {
            return Err("tracked pitch did not increase as the spring moved inward".into());
        }
        if next.mode().frequency_hz >= target {
            bracket = Some((previous, next));
            break;
        }
        previous = next;
    }
    let (mut low, mut high) =
        bracket.ok_or("target was not bracketed within spring center 35..59.5 mm")?;
    for _ in 0..32 {
        let position = 0.5 * (low.position + high.position);
        let (next, score) = follow(&low, position)?;
        let (from_high, _) = follow(&high, position)?;
        if next.index != from_high.index {
            return Err("bracket endpoints disagree on structural identity".into());
        }
        let error = 1200.0 * (next.mode().frequency_hz / target).log2();
        rows.push(next.row(score));
        if error.abs() < 0.0001 {
            let report = json!({"parameter":"spring center from fixed root","bounds_mm":[35.0,59.5],
                "fixed_tine_length_mm":70.0,"fixed_tuning_mass_g":0.1,"passed":true,"target_hz":target,
                "selected_spring_center_mm":position*LENGTH*1000.0,"selected_spring_position_fraction":position,
                "undamped_error_cents":error,"evaluations":rows,
                "tracking_metric":"Physical displacement fields, exact four-point per-element Gauss integration, fixed reference inertia with spring at 0.85 L; MAC >= 0.98, runner-up <= 0.05",
                "scope":"Provisional fixed blank and point-mass spring, not identified instrument geometry or timbre"});
            return Ok((next, report));
        }
        if error < 0.0 {
            low = next;
        } else {
            high = next;
        }
    }
    Err("bounded structural tuning did not converge".into())
}

pub(super) fn reference(value: &Value) -> Result<f64, Box<dyn Error>> {
    if value["schema_version"] != 1
        || value["experiment"] != "frequency-reference-preparation-v1"
        || value["reference_qualification_passed"] != true
        || value["manifest"]["note"] != 55
    {
        return Err("expected a qualified G3 frequency-reference receipt".into());
    }
    let target = value["training_target_hz"]
        .as_f64()
        .filter(|f| f.is_finite() && (150.0..=250.0).contains(f))
        .ok_or("invalid frozen G3 target")?;
    let takes = value["takes"]
        .as_array()
        .filter(|t| (3..=8).contains(&t.len()))
        .ok_or("invalid reference takes")?;
    let mut training = Vec::new();
    let mut validation = 0;
    for row in takes {
        if row["anchor"]["qualified"] != true
            || row["frozen_target_residual"]["consistent_with_5_cent_pilot_limit"] != true
        {
            return Err("reference contains an unqualified take".into());
        }
        let f = row["anchor"]["frequency_hz"]
            .as_f64()
            .filter(|f| f.is_finite() && *f > 0.0)
            .ok_or("invalid take frequency")?;
        if (1200.0 * (f / target).log2()).abs() > 5.0 {
            return Err("reference consistency claim disagrees with frequencies".into());
        }
        match row["take"]["role"].as_str() {
            Some("training") => training.push(f),
            Some("validation") => validation += 1,
            _ => return Err("unknown reference take role".into()),
        }
    }
    if training.len() < 2 || validation == 0 {
        return Err("reference split is incomplete".into());
    }
    let frozen = (training.iter().map(|f| f.ln()).sum::<f64>() / training.len() as f64).exp();
    if (frozen / target - 1.0).abs() > 1e-12 {
        return Err("target differs from training-only log-frequency mean".into());
    }
    Ok(target)
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 4
        || args[2] != "--output"
        || Path::new(&args[3]).extension().is_none_or(|x| x != "wav")
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[3]);
    let report_path = output.with_extension("json");
    let before_path = output.with_file_name(format!(
        "{}-before.wav",
        output
            .file_stem()
            .ok_or("missing file stem")?
            .to_str()
            .ok_or("non-UTF8 file stem")?
    ));
    for path in [output, report_path.as_path(), before_path.as_path()] {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
    }
    let mut bytes = Vec::new();
    File::open(&args[1])?.take(262145).read_to_end(&mut bytes)?;
    if bytes.len() > 262144 {
        return Err("frequency receipt exceeds 256 KiB".into());
    }
    let source: Value = serde_json::from_slice(&bytes)?;
    let target = reference(&source)?;
    let (selected, fit) = match fit(target) {
        Ok(result) => result,
        Err(error) => {
            crate::analysis::write_report(
                &report_path,
                &json!({"schema_version":1,
                "experiment":"coupled-spring-pitch-v1","passed":false,"stage":"structural tuning",
                "reason":error.to_string(),"frozen_reference":source}),
            )?;
            return Err(error);
        }
    };
    println!(
        "Tracked spring center: {:.6} mm from root, {:.6} Hz",
        selected.position * LENGTH * 1000.0,
        selected.mode().frequency_hz
    );
    let before = spectrum(LENGTH, INITIAL_POSITION)?;
    // Endpoint diagnostic for the previous blank. The inward endpoint is a
    // mathematical endpoint diagnostic, not a service setting.
    let old_inward = spectrum(0.075, 0.0)?;
    let mut options = Options {
        output: before_path.clone(),
        frames: RATE * 2,
        release: 88800,
        speed: 0.4,
        length: LENGTH,
        spring_position: INITIAL_POSITION,
        gain: 0.084,
    };
    let (before_signal, before_numerical) = audio::produce(&options)?;
    options.output = output.to_path_buf();
    options.spring_position = selected.position;
    let (signal, numerical) = audio::produce(&options)?;
    // Gate the exact f32 quantization written to each WAV.
    let observe = |signal: &[f64]| -> Result<_, Box<dyn Error>> {
        let clip = AudioClip::from_samples(
            RATE as u32,
            signal.iter().map(|&x| x as f32 as f64).collect(),
        )?;
        Ok(pitch_anchor(&clip, 55)?)
    };
    let before_anchor = observe(&before_signal)?;
    let anchor = observe(&signal)?;
    let error = anchor.frequency_hz.map(|f| 1200.0 * (f / target).log2());
    let pass = numerical["preview_gates_passed"] == true
        && before_numerical["preview_gates_passed"] == true
        && before_anchor.qualified
        && anchor.qualified
        && error.is_some_and(|e| e.abs() <= 5.0);
    let ratios: Vec<_> = before
        .fixed_root
        .iter()
        .zip(selected.fixed_root)
        .enumerate()
        .map(|(i, (a, b))| {
            let ra = a / before.fixed_root[0];
            let rb = b / selected.fixed_root[0];
            json!({"ordinal":i+1,"before_hz":a,"after_hz":b,"before_ratio":ra,"after_ratio":rb,
            "ratio_change_percent":100.0*(rb/ra-1.0)})
        })
        .collect();
    let report = json!({"schema_version":1,"experiment":"coupled-spring-pitch-v1","passed":pass,
        "frozen_reference_file":args[1],"frozen_reference":source,"structural_fit":fit,
        "old_75mm_blank_inward_endpoint":old_inward.row(1.0),
        "baseline_structure":before.row(1.0),"tuned_structure":selected.row(mac(&before,&selected,selected.index)),
        "fixed_root_tine_ratio_changes":ratios,
        "before":{"file":before_path,"numerical_audio_qualification":before_numerical,"written_f32_pitch_anchor":before_anchor,
            "descriptive_audio":super::hammer_comparison::describe(&before_signal)},
        "after":{"file":output,"numerical_audio_qualification":numerical,"written_f32_pitch_anchor":anchor,
            "descriptive_audio":super::hammer_comparison::describe(&signal)},
        "output_pitch_error_cents":error,"listening_performed":false,
        "scope":"Only spring position varies within the 70 mm listening pair. Length is an explicit provisional coarse choice; other geometry, hammer, root/tonebar, losses and pickup are fixed. Point-mass center is not measured coil geometry. Modal ratios describe mechanical resonances, not every nonlinear pickup partial. Raw attack-band power and envelopes are descriptive, not timbre acceptance gates. The reused processed G3 frequency target does not identify geometry, natural decay or velocity. No keyboard scale, realtime/plugin change or listening claim."});
    crate::analysis::write_report(&report_path, &report)?;
    if !pass {
        return Err(
            "spring pair failed numerical or output-pitch gates; report retained, no WAVs".into(),
        );
    }
    audio::write_wav(&options, &signal)?;
    options.output = before_path;
    audio::write_wav(&options, &before_signal)?;
    println!(
        "Tuned WAV: {} (pitch error {:.6} cents); before: {}",
        output.display(),
        error.unwrap(),
        options.output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_metric_recovers_orthogonality_after_large_spring_motion() {
        for taper in [0.9, 1.0, 1.05] {
            for (position, span) in [
                (0.5, 0.0),
                (0.75, 0.0),
                (0.95, 0.0),
                (0.5, 0.006),
                (0.75, 0.006),
                (0.95, 0.006),
            ] {
                let mut a = spectrum_with_taper(0.070, 0.00012, span, taper, position).unwrap();
                let b = spectrum_with_taper(0.070, 0.00012, span, taper, position).unwrap();
                for i in 0..9 {
                    a.index = i;
                    for j in 0..9 {
                        let score = local_mac(&a, &b, j).unwrap();
                        assert!(
                            (score - f64::from(i == j)).abs() < 1e-10,
                            "position={position}, modes={i}/{j}: {score}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn spatial_tracking_is_order_independent_and_reference_fields_are_orthonormal() {
        let selected = spectrum(LENGTH, INITIAL_POSITION).unwrap();
        for (i, a) in selected.fields.iter().enumerate() {
            for (j, b) in selected.fields.iter().enumerate() {
                let dot: f64 = a.iter().zip(b).map(|(a, b)| a * b).sum();
                assert!(
                    (dot - f64::from(i == j)).abs() < 1e-8,
                    "field Gram ({i},{j})={dot}"
                );
            }
        }
        let mut permuted = spectrum(LENGTH, INITIAL_POSITION).unwrap();
        permuted.spectrum.modes.reverse();
        permuted.fields.reverse();
        assert!((mac(&selected, &permuted, 8 - selected.index) - 1.0).abs() < 1e-12);
        let (_, score) = follow(&selected, 0.825).unwrap();
        assert!(score >= 0.98);
        for target in [150.0, f64::NAN, 250.0] {
            assert!(fit(target).is_err());
        }
    }
    #[test]
    fn frozen_target_moves_spring_inward_and_changes_nonharmonic_ratios() {
        let source: Value = serde_json::from_str(include_str!(
            "../../../../references/g3-pitch-reference-validation.json"
        ))
        .unwrap();
        let target = reference(&source).unwrap();
        let (selected, report) = fit(target).unwrap();
        assert_eq!(selected.length, LENGTH);
        assert!((0.5..INITIAL_POSITION).contains(&selected.position));
        assert!(report["undamped_error_cents"].as_f64().unwrap().abs() < 0.0001);
        let before = spectrum(LENGTH, INITIAL_POSITION).unwrap();
        assert!(
            (selected.fixed_root[1] / selected.fixed_root[0]
                - before.fixed_root[1] / before.fixed_root[0])
                .abs()
                > 0.01
        );
        assert!(spectrum(0.075, 0.0).unwrap().mode().frequency_hz < target);
    }
    #[test]
    fn reference_rejects_unqualified_or_recomputed_targets() {
        let mut r: Value = serde_json::from_str(include_str!(
            "../../../../references/g3-pitch-reference-validation.json"
        ))
        .unwrap();
        let good = reference(&r).unwrap();
        r["training_target_hz"] = json!(good + 0.1);
        assert!(reference(&r).is_err());
        r["training_target_hz"] = json!(good);
        r["reference_qualification_passed"] = json!(false);
        assert!(reference(&r).is_err());
    }
}
