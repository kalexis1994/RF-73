//! Independent contact/free caps and implicit-reference resolution study.
use super::*;
use serde_json::Value;

pub const HELP: &str = "Coupled RK4 resolution study:
  memory-modal-rk4-refinement --output REPORT.json
Twelve 8 ms profiles and four strong-strike 32 ms profiles; six paths each.
Separately caps contact and free intervals, and refines the implicit reference.
No physical coefficient or acceptance tolerance changes, audio or calibration.
";
const LABELS: [&str; 6] = [
    "rk4_default",
    "rk4_contact_cap_2",
    "rk4_contact_2_free_8",
    "uniform_8336",
    "uniform_16672",
    "uniform_20832",
];
const PAIRS: [(usize, usize); 7] = [(0, 1), (1, 2), (0, 2), (0, 4), (3, 4), (4, 5), (2, 5)];

fn kinetic(a: MemoryModalProbe, b: MemoryModalProbe, mass: [[f64; 9]; 9]) -> f64 {
    let dv: [f64; 9] = core::array::from_fn(|i| a.velocity[i] - b.velocity[i]);
    0.0038 * (a.hammer.core_velocity_m_s - b.hammer.core_velocity_m_s).powi(2)
        + 0.0002 * (a.hammer.tip_velocity_m_s - b.hammer.tip_velocity_m_s).powi(2)
        + (0..9)
            .map(|i| dv[i] * (0..9).map(|j| mass[i][j] * dv[j]).sum::<f64>())
            .sum::<f64>()
}
pub(super) fn compare(a: &Take, b: &Take, speed: f64) -> Result<Value, Box<dyn Error>> {
    if a.states.is_empty()
        || a.states.len() != b.states.len()
        || a.forces.len() != a.states.len()
        || b.forces.len() != b.states.len()
        || a.mass != b.mass
        || !speed.is_finite()
        || speed <= 0.0
    {
        return Err("incompatible refinement trajectories".into());
    }
    let energies: Vec<_> = a
        .states
        .iter()
        .zip(&b.states)
        .map(|(qa, qb)| kinetic(*qa, *qb, a.mass))
        .collect();
    if !energies.iter().all(|x| x.is_finite() && *x >= 0.0) {
        return Err("invalid refinement kinetic metric".into());
    }
    let scale = 0.004 * speed * speed;
    let velocity = (energies.iter().sum::<f64>() / (energies.len() as f64 * scale)).sqrt();
    let peak = (energies.iter().copied().fold(0.0_f64, f64::max) / scale).sqrt();
    let pickup_difference = a
        .states
        .iter()
        .zip(&b.states)
        .map(|(a, b)| (a.pickup_velocity_m_s - b.pickup_velocity_m_s).powi(2))
        .sum::<f64>();
    let pickup_power = b
        .states
        .iter()
        .map(|q| q.pickup_velocity_m_s.powi(2))
        .sum::<f64>();
    let force_difference = a
        .forces
        .iter()
        .zip(&b.forces)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>();
    let force_power = b.forces.iter().map(|x| x * x).sum::<f64>();
    if ![
        scale,
        pickup_difference,
        pickup_power,
        force_difference,
        force_power,
    ]
    .iter()
    .all(|x| x.is_finite() && *x >= 0.0)
        || scale == 0.0
    {
        return Err("non-finite refinement comparison or normalization".into());
    }
    let relative = |error: f64, power: f64| {
        if power > 0.0 {
            (error / power)
                .sqrt()
                .is_finite()
                .then_some((error / power).sqrt())
        } else {
            None
        }
    };
    let pickup = relative(pickup_difference, pickup_power);
    let force = relative(force_difference, force_power);
    let passed = a.pass
        && b.pass
        && velocity.is_finite()
        && velocity < 0.01
        && peak.is_finite()
        && pickup.map_or(pickup_difference == 0.0, |x| x < 0.01)
        && force.map_or(force_difference == 0.0, |x| x < 0.02);
    let windows:Vec<_>=energies.chunks(96).enumerate().map(|(i,window)|json!({
        "start_seconds":(i*96) as f64/48000.0,"end_seconds":(i*96+window.len()) as f64/48000.0,
        "kinetic_velocity_rmse_over_launch_speed":(window.iter().sum::<f64>()/(window.len() as f64*scale)).sqrt(),
        "peak_kinetic_velocity_error_over_launch_speed":(window.iter().copied().fold(0.0_f64,f64::max)/scale).sqrt()})).collect();
    Ok(
        json!({"passed":passed,"kinetic_velocity_rmse_over_launch_speed":velocity,
        "peak_kinetic_velocity_error_over_launch_speed":peak,"pickup_velocity_relative_rmse":pickup,
        "output_mean_force_relative_rmse":force,"pickup_reference_squared_sum":pickup_power,
        "force_reference_squared_sum":force_power,"two_ms_windows":windows}),
    )
}

fn study_case(length: f64, speed: f64, tau: f64, frames: usize) -> Result<Value, Box<dyn Error>> {
    let specs = [
        (16672, ContactMode::Rk4, None),
        (16672, ContactMode::Rk4, Some((2, 12))),
        (16672, ContactMode::Rk4, Some((2, 8))),
        (8336, ContactMode::None, None),
        (16672, ContactMode::None, None),
        (20832, ContactMode::None, None),
    ];
    let mut takes = Vec::new();
    for (substeps, mode, limits) in specs {
        takes.push(take_configured(
            length,
            speed,
            tau,
            substeps,
            mode == ContactMode::Rk4,
            mode,
            TakeConfig {
                frames,
                rk4_limits: limits,
                ..TakeConfig::default()
            },
        )?);
    }
    let mut comparisons = Vec::new();
    for (candidate, reference) in PAIRS {
        let mut row = compare(&takes[candidate], &takes[reference], speed)?;
        row["candidate"] = json!(LABELS[candidate]);
        row["reference"] = json!(LABELS[reference]);
        comparisons.push(row);
    }
    let passed = comparisons.iter().all(|x| x["passed"] == true);
    let reports: Vec<_> = takes
        .into_iter()
        .zip(LABELS)
        .map(|(take, label)| json!({"path":label,"report":take.report}))
        .collect();
    Ok(
        json!({"tine_length_m":length,"launch_speed_m_s":speed,"relaxation_seconds":tau,
        "duration_seconds":frames as f64/48000.0,"passed":passed,"takes":reports,"comparisons":comparisons}),
    )
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|x| x != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    for length in [0.05, 0.075, 0.12] {
        for speed in [0.2, 0.8] {
            for tau in [0.001, 0.01] {
                cases.push(study_case(length, speed, tau, 384)?);
                println!(
                    "Completed 8 ms refinement: length {length} m, speed {speed} m/s, tau {tau} s."
                );
            }
        }
    }
    for length in [0.075, 0.12] {
        for tau in [0.001, 0.01] {
            cases.push(study_case(length, 0.8, tau, 1536)?);
            println!("Completed 32 ms refinement: length {length} m, speed 0.8 m/s, tau {tau} s.");
        }
    }
    let passed = cases.iter().all(|x| x["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-rk4-refinement-v1",
        "status":if passed {"pass"} else {"fail"},"cases":cases,
        "gates":{"each_take_energy_and_port_work":1e-8,"each_take_positive_energy_step":1e-10,
            "pair_kinetic_velocity_rmse":0.01,"pair_pickup_velocity_rmse":0.01,"pair_mean_force_rmse":0.02},
        "protocol":"Same initial impact, core impulse at 2 ms, damper on at 4 ms and off at 6 ms. Twelve original 8 ms profiles; four strong 75/120 mm profiles continue to 32 ms without new events. Six paths per case: original RK4; contact capped at level 2; contact 2 and free 8; uniform implicit 8336, 16672, 20832 ticks per 48 kHz frame. Caps preserve physical coefficients, acceptance tolerances and external-event boundaries. All paths retain global energy/work checks. Seven pair comparisons include separate contact/free refinements and implicit-reference refinement; 2 ms kinetic windows and peak errors expose localized differences.",
        "scope":"Numerical qualification, not acoustic calibration or an exact continuous reference. Uniform finest step is approximately 1.000064 ns; the API does not permit halving the old 1.25 ns reference. Adjacent implicit refinements are therefore unequal. A smaller timestep or pair difference alone is not a global accuracy proof. Zero reference power produces a null relative metric, accepted only for exact zero difference. Window/peak metrics are diagnostic, with no added peak gate. No audio, GUI, pickup voltage, realtime qualification or timing claims."}),
    )?;
    writeln!(file)?;
    if !passed {
        return Err("modal RK4 refinement failed; see retained report".into());
    }
    println!(
        "Modal RK4 refinement passed: 16 cases, 96 takes. Report: {}",
        args[2]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic(n: usize) -> Take {
        let v = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        Take {
            states: vec![v.probe(); n],
            forces: vec![0.0; n],
            mass: v.mass_matrix(),
            pass: true,
            report: json!({}),
        }
    }
    #[test]
    fn comparison_keeps_silent_metrics_undefined_and_localizes_velocity_error() {
        let mut a = synthetic(192);
        let b = synthetic(192);
        let same = compare(&a, &b, 0.8).unwrap();
        assert_eq!(same["passed"], true);
        assert!(same["pickup_velocity_relative_rmse"].is_null());
        a.states[100].hammer.tip_velocity_m_s += 0.01;
        let result = compare(&a, &b, 0.8).unwrap();
        assert_eq!(
            result["two_ms_windows"][0]["kinetic_velocity_rmse_over_launch_speed"],
            0.0
        );
        let expected = (0.0002_f64 * 0.01 * 0.01 / (0.004 * 0.8 * 0.8)).sqrt();
        assert!(
            (result["peak_kinetic_velocity_error_over_launch_speed"]
                .as_f64()
                .unwrap()
                - expected)
                .abs()
                < 1e-14
        );
        assert!(
            (result["kinetic_velocity_rmse_over_launch_speed"]
                .as_f64()
                .unwrap()
                - expected / (192.0_f64).sqrt())
            .abs()
                < 1e-14
        );
        a.forces[0] = 1.0;
        assert_eq!(compare(&a, &b, 0.8).unwrap()["passed"], false);
        assert!(compare(&synthetic(1), &b, 0.8).is_err());
    }
}
