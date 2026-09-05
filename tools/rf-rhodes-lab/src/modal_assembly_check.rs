use rf_rhodes_dsp::{ModalAssembly, ModalAssemblyProfile, ModalIntegration, TineGeometry};
use serde_json::json;
use std::{error::Error, hint::black_box, io::Write, path::Path, time::Instant};
pub const HELP: &str = "Multimode assembly:
  modal-assembly-check --output REPORT.json
Audits nine-coordinate dynamics for three tine lengths, two velocities and two rates.
Compares contact subdivisions and independent uniform-midpoint references over 40 ms.
No audio device is opened. Profiles remain uncalibrated; no plugin integration.
";
pub const HAMMER_HELP: &str = "Dissipative modal hammer:
  modal-hammer-check --output REPORT.json
Compares beta=0/0.5/2 s/m contact loss across 12 length/rate/velocity cases.
Reports independent contact heat and refined/uniform convergence; no audio device.
";
struct Take {
    report: serde_json::Value,
    q: Vec<[f64; 9]>,
    v: Vec<[f64; 9]>,
    pickup: Vec<f64>,
    mass: [[f64; 9]; 9],
    energy_pass: bool,
}
fn take(
    g: TineGeometry,
    rate: u32,
    velocity: f64,
    integration: ModalIntegration,
    profile: ModalAssemblyProfile,
) -> Result<Take, Box<dyn Error>> {
    let mut voice = ModalAssembly::new(f64::from(rate), g, profile, integration)?;
    let mass = voice.mass_matrix();
    let steps = voice.steps_per_sample();
    let dt = 1.0 / (f64::from(rate) * steps as f64);
    let frames = rate as usize / 25;
    if !voice.strike(velocity) {
        return Err("modal audit strike rejected".into());
    }
    let initial = voice.probe().injected_energy_j;
    let (mut balance, mut positive, mut impulse, mut peak_force) =
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let mut separation = None;
    let mut previous = initial;
    let (mut q, mut v, mut pickup) = (
        Vec::with_capacity(frames),
        Vec::with_capacity(frames),
        Vec::with_capacity(frames),
    );
    let mut peaks = [0.0_f64; 9];
    for frame in 0..frames {
        if frame == frames / 2 {
            voice.set_damped(true);
        }
        for tick in 0..steps {
            voice.tick();
            let p = voice.probe();
            if !p.mechanical_energy_j.is_finite()
                || !p.balance_residual_j.is_finite()
                || !p.contact_dissipated_energy_j.is_finite()
                || p.contact_dissipated_energy_j < 0.0
                || !p.contact_limited_heat_j.is_finite()
                || p.contact_limited_heat_j < 0.0
                || p.contact_limited_heat_j > p.contact_dissipated_energy_j
                || !p.contact_force_n.is_finite()
                || p.contact_force_n < 0.0
            {
                return Err("non-finite modal audit state".into());
            }
            balance = balance.max(p.balance_residual_j.abs() / initial);
            positive = positive.max((p.mechanical_energy_j - previous) / initial);
            previous = p.mechanical_energy_j;
            impulse += p.contact_force_n * dt;
            peak_force = peak_force.max(p.contact_force_n);
            if !p.contact_active && separation.is_none() {
                separation = Some((frame * steps + tick + 1) as f64 * dt);
            }
            for (peak, x) in peaks.iter_mut().zip(p.position) {
                *peak = peak.max(x.abs());
            }
        }
        let p = voice.probe();
        q.push(p.position);
        v.push(p.velocity);
        pickup.push(p.pickup_velocity_m_s);
    }
    let p = voice.probe();
    let method = match integration {
        ModalIntegration::Midpoint { steps_per_sample } => {
            json!({"kind":"midpoint","steps_per_sample":steps_per_sample})
        }
        ModalIntegration::Refined { contact_substeps } => {
            json!({"kind":"refined","steps_per_sample":4,"contact_substeps":contact_substeps})
        }
    };
    Ok(Take {
        report: json!({"integration":method,"maximum_balance_relative_residual":balance,
        "maximum_positive_energy_step_relative":positive,"injected_energy_j":initial,
        "final_energy_j":p.mechanical_energy_j,"dissipated_energy_j":p.dissipated_energy_j,
        "contact_dissipated_energy_j":p.contact_dissipated_energy_j,"contact_limit_steps":p.contact_limit_steps,
        "contact_limited_heat_j":p.contact_limited_heat_j,
        "escaped_hammer_energy_j":p.escaped_hammer_energy_j,"separation_seconds":separation,
        "contact_impulse_n_s":impulse,"peak_tick_mean_force_n":peak_force,"peak_absolute_coordinates":peaks}),
        q,
        v,
        pickup,
        mass,
        energy_pass: balance < 1e-8 && positive < 1e-10 && separation.is_some(),
    })
}
fn quadratic(x: [f64; 9], m: [[f64; 9]; 9]) -> f64 {
    (0..9)
        .map(|i| x[i] * (0..9).map(|j| m[i][j] * x[j]).sum::<f64>())
        .sum()
}
fn error(a: &[[f64; 9]], b: &[[f64; 9]], m: [[f64; 9]; 9]) -> f64 {
    let mut diff = 0.0;
    let mut reference = 0.0;
    for (a, b) in a.iter().zip(b) {
        diff += quadratic(core::array::from_fn(|i| a[i] - b[i]), m);
        reference += quadratic(*b, m);
    }
    (diff / reference).sqrt()
}
fn scalar_error(a: &[f64], b: &[f64]) -> f64 {
    (a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum::<f64>()
        / b.iter().map(|v| v * v).sum::<f64>())
    .sqrt()
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for length in [0.05, 0.075, 0.12] {
        for rate in [44100, 192000] {
            for velocity in [0.1, 1.0] {
                let g = TineGeometry {
                    length_m: length,
                    ..TineGeometry::default()
                };
                let reference = take(
                    g,
                    rate,
                    velocity,
                    ModalIntegration::Refined {
                        contact_substeps: 256,
                    },
                    ModalAssemblyProfile::default(),
                )?;
                pass &= reference.energy_pass;
                let mut rows = Vec::new();
                for integration in [
                    ModalIntegration::Refined {
                        contact_substeps: 1,
                    },
                    ModalIntegration::Refined {
                        contact_substeps: 32,
                    },
                    ModalIntegration::Refined {
                        contact_substeps: 64,
                    },
                    ModalIntegration::Refined {
                        contact_substeps: 128,
                    },
                    ModalIntegration::Midpoint {
                        steps_per_sample: 512,
                    },
                    ModalIntegration::Midpoint {
                        steps_per_sample: 1024,
                    },
                ] {
                    let mut candidate = take(
                        g,
                        rate,
                        velocity,
                        integration,
                        ModalAssemblyProfile::default(),
                    )?;
                    let qerror = error(&candidate.q, &reference.q, reference.mass);
                    let verror = error(&candidate.v, &reference.v, reference.mass);
                    let pickup_error = scalar_error(&candidate.pickup, &reference.pickup);
                    candidate.report["displacement_relative_rmse_vs_refined_256"] = json!(qerror);
                    candidate.report["velocity_relative_rmse_vs_refined_256"] = json!(verror);
                    candidate.report["pickup_velocity_relative_rmse_vs_refined_256"] =
                        json!(pickup_error);
                    pass &= candidate.energy_pass
                        && qerror.is_finite()
                        && verror.is_finite()
                        && pickup_error.is_finite();
                    if matches!(
                        integration,
                        ModalIntegration::Refined {
                            contact_substeps: 32
                        }
                    ) {
                        pass &= qerror < 0.001 && verror < 0.001 && pickup_error < 0.001;
                    }
                    if matches!(
                        integration,
                        ModalIntegration::Midpoint {
                            steps_per_sample: 1024
                        }
                    ) {
                        pass &= qerror < 0.005 && verror < 0.005 && pickup_error < 0.005;
                    }
                    rows.push(candidate.report);
                }
                rows.push(reference.report);
                cases.push(json!({"tine_length_m":length,"sample_rate_hz":rate,"velocity":velocity,"mass_matrix":reference.mass,"takes":rows}));
            }
        }
    }
    let p = ModalAssemblyProfile::default();
    let g = TineGeometry::default();
    let report = json!({"schema_version":1,"experiment":"nine-coordinate-modal-assembly-v1","status":if pass{"pass"}else{"fail"},
        "calibrated":false,"plugin_integrated":false,"duration_seconds":0.04,"damper_frame":"floor(output_frames/2)",
        "contact_solver":{"refined":"up to 8 safeguarded Newton evaluations, then 48 bisections if needed","midpoint":"original 48 bisections"},
        "coordinates":["root_m","root_rad","tine_1_m","tine_2_m","tine_3_m","tine_4_m","tine_5_m","tine_6_m","relative_tonebar_m"],
        "tine_defaults":geometry_metadata(g),"profile":profile_metadata(p),
        "gates":{"balance_relative":1e-8,"positive_energy_step_relative":1e-10,"refined_32_rmse":0.001,"midpoint_1024_rmse":0.005},
        "scope":"Finite-reference mechanical metrics; physical inertia weights mixed translation/rotation coordinates. Refined probes observe base ticks, not each contact microstep. No magnetic conversion or antialias filtering; no audio/host qualification.",
        "cases":cases,"native_timing":timing()?});
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    if !pass {
        return Err("modal assembly audit failed; see report".into());
    }
    println!(
        "Modal assembly audit passed: 12 cases, 84 takes. Report: {}",
        args[2]
    );
    Ok(())
}
pub fn run_hammer(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HAMMER_HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for length in [0.05, 0.075, 0.12] {
        for rate in [44100, 192000] {
            for velocity in [0.1, 1.0] {
                let g = TineGeometry {
                    length_m: length,
                    ..TineGeometry::default()
                };
                let mut baseline = Vec::new();
                for beta in [0.0, 0.5, 2.0] {
                    let p = ModalAssemblyProfile {
                        contact_damping_s_m: beta,
                        ..ModalAssemblyProfile::default()
                    };
                    let mut reference = take(
                        g,
                        rate,
                        velocity,
                        ModalIntegration::Refined {
                            contact_substeps: 256,
                        },
                        p,
                    )?;
                    if beta == 0.0 {
                        baseline.clone_from(&reference.pickup);
                    }
                    let change = scalar_error(&reference.pickup, &baseline);
                    let mut rows = Vec::new();
                    for integration in [
                        ModalIntegration::Refined {
                            contact_substeps: 32,
                        },
                        ModalIntegration::Midpoint {
                            steps_per_sample: 1024,
                        },
                    ] {
                        let mut candidate = take(g, rate, velocity, integration, p)?;
                        let qerror = error(&candidate.q, &reference.q, reference.mass);
                        let verror = error(&candidate.v, &reference.v, reference.mass);
                        let perror = scalar_error(&candidate.pickup, &reference.pickup);
                        let tolerance = if matches!(integration, ModalIntegration::Refined { .. }) {
                            0.001
                        } else {
                            0.005
                        };
                        pass &= candidate.energy_pass
                            && qerror.is_finite()
                            && verror.is_finite()
                            && perror.is_finite()
                            && qerror < tolerance
                            && verror < tolerance
                            && perror < tolerance;
                        candidate.report["displacement_relative_rmse_vs_refined_256"] =
                            json!(qerror);
                        candidate.report["velocity_relative_rmse_vs_refined_256"] = json!(verror);
                        candidate.report["pickup_velocity_relative_rmse_vs_refined_256"] =
                            json!(perror);
                        rows.push(candidate.report);
                    }
                    pass &= reference.energy_pass && change.is_finite();
                    let heat = reference.report["contact_dissipated_energy_j"]
                        .as_f64()
                        .ok_or("missing contact heat")?;
                    pass &= if beta == 0.0 {
                        heat == 0.0 && change == 0.0
                    } else {
                        heat > 0.0 && change > 0.0
                    };
                    reference.report["pickup_velocity_relative_rmse_vs_elastic_256"] =
                        json!(change);
                    rows.push(reference.report);
                    cases.push(
                        json!({"tine_length_m":length,"sample_rate_hz":rate,"velocity":velocity,
                        "contact_damping_s_m":beta,"mass_matrix":reference.mass,"takes":rows}),
                    );
                }
            }
        }
    }
    let report = json!({"schema_version":1,"experiment":"modal-dissipative-hammer-v1","status":if pass {"pass"} else {"fail"},
        "calibrated":false,"plugin_integrated":false,"duration_seconds":0.04,"damper_frame":"floor(output_frames/2)",
        "profile_defaults":profile_metadata(ModalAssemblyProfile::default()),"tine_defaults":geometry_metadata(TineGeometry::default()),
        "varying_parameters":"Each case overrides tine length and contact_damping_s_m; geometry and material values are illustrative.",
        "coordinates":["root_m","root_rad","tine_1_m","tine_2_m","tine_3_m","tine_4_m","tine_5_m","tine_6_m","relative_tonebar_m"],
        "contact_law":"G(a,b) * max(0,1 + beta*(b-a)/h), G is the cubic potential discrete gradient",
        "heat_law":"G*beta*(b-a)^2/h; if nonadhesive limit active, -G*(b-a). Contact heat is included in total dissipated energy.",
        "solver":"Refined: 8 safeguarded Newton evaluations plus 48 bisections if needed. Uniform: 48 bisections.",
        "gates":{"balance_relative":1e-8,"positive_energy_step_relative":1e-10,"refined_32_rmse":0.001,"midpoint_1024_rmse":0.005},
        "scope":"Illustrative rate-dependent loss. No internal relaxation memory, neoprene calibration, pickup voltage or realtime qualification. Base-tick probes do not observe every refined contact microstep.",
        "cases":cases});
    serde_json::to_writer_pretty(&mut file, &report)?;
    writeln!(file)?;
    if !pass {
        return Err("dissipative modal hammer audit failed; see report".into());
    }
    println!(
        "Dissipative modal hammer audit passed: 36 cases, 108 takes. Report: {}",
        args[2]
    );
    Ok(())
}

fn geometry_metadata(g: TineGeometry) -> serde_json::Value {
    json!({"elements":64,"diameter_m":g.diameter_m,"young_modulus_pa":g.young_modulus_pa,"density_kg_m3":g.density_kg_m3,
        "tuning_mass_kg":g.tuning_mass_kg,"tuning_position":g.tuning_position,"hammer_position":g.hammer_position,"pickup_position":g.pickup_position})
}
fn profile_metadata(p: ModalAssemblyProfile) -> serde_json::Value {
    json!({"support_mass_kg":p.support_mass_kg,"support_inertia_kg_m2":p.support_inertia_kg_m2,"translation_stiffness_n_m":p.translation_stiffness_n_m,
        "rotation_stiffness_n_m_rad":p.rotation_stiffness_n_m_rad,"translation_damping_n_s_m":p.translation_damping_n_s_m,"rotation_damping_n_m_s_rad":p.rotation_damping_n_m_s_rad,
        "tonebar_mass_kg":p.tonebar_mass_kg,"tonebar_arm_m":p.tonebar_arm_m,"tonebar_frequency_hz":p.tonebar_frequency_hz,"tonebar_decay_seconds":p.tonebar_decay_seconds,
        "tine_decay_seconds":p.tine_decay_seconds,"damper_position":p.damper_position,"damper_n_s_m":p.damper_n_s_m,"hammer_mass_kg":p.hammer_mass_kg,
        "contact_stiffness_n_m2":p.contact_stiffness_n_m2,"contact_damping_s_m":p.contact_damping_s_m,"maximum_hammer_speed_m_s":p.maximum_hammer_speed_m_s})
}

fn timing() -> Result<serde_json::Value, Box<dyn Error>> {
    let mut runs = Vec::new();
    for _ in 0..5 {
        let mut v = ModalAssembly::new(
            48000.0,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            ModalIntegration::Refined {
                contact_substeps: 32,
            },
        )?;
        let start = Instant::now();
        for frame in 0..12000 {
            if frame % 2400 == 0 && !v.strike(1.0) {
                return Err("modal timing restrike rejected".into());
            }
            if frame % 2400 == 1200 {
                v.set_damped(true);
            }
            for _ in 0..4 {
                black_box(&mut v).tick();
            }
        }
        runs.push(start.elapsed().as_secs_f64());
        if !black_box(v.probe()).mechanical_energy_j.is_finite() {
            return Err("modal timing non-finite energy".into());
        }
    }
    runs.sort_by(f64::total_cmp);
    Ok(
        json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"sample_rate_hz":48000,"duration_seconds":0.25,"strikes":5,"runs":5,
        "preparation_timed":false,"median_seconds":runs[2],"minimum_seconds":runs[0],"maximum_seconds":runs[4],
        "scope":"One native nine-coordinate voice, no pickup or host; energy ledger included; not a polyphonic deadline qualification."}),
    )
}
