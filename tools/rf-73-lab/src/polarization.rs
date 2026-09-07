//! Two-plane action cycles with independent work ledgers for each transverse plane.
use rf_73_dsp::{
    ActionProfile, FeltDamperProfile, ModalAssemblyProfile, PolarizationProfile,
    PolarizedActionAssembly, PolarizedActionProbe, TineGeometry,
};
use serde_json::{Value, json};
use std::{error::Error, path::Path};

pub const HELP: &str = "Two-plane action audit:
  polarized-action --output REPORT.json [--refined]
Eight cases: symmetry controls, reciprocal boundary coupling and oblique contact.
";
const DURATION: f64 = 0.18;
const NAMES: [&str; 4] = [
    "isotropic",
    "aligned_anisotropy",
    "rotated_boundary",
    "oblique_contacts",
];

fn profile(case: usize) -> PolarizationProfile {
    match case {
        0 => PolarizationProfile::isotropic(),
        1 => PolarizationProfile {
            boundary_angle_rad: 0.0,
            ..PolarizationProfile::default()
        },
        2 => PolarizationProfile::default(),
        _ => PolarizationProfile {
            hammer_angle_rad: 0.08,
            felt_angle_rad: 0.04,
            ..PolarizationProfile::isotropic()
        },
    }
}
struct Take {
    samples: Vec<[f64; 4]>,
    summary: Value,
}
fn plane_energy(
    p: &PolarizedActionProbe,
    m: &[[f64; 18]; 18],
    k: &[[f64; 18]; 18],
    plane: usize,
) -> f64 {
    let start = 9 * plane;
    let mut energy = 0.0;
    for i in start..start + 9 {
        for j in start..start + 9 {
            energy += 0.5
                * (p.velocity[i] * m[i][j] * p.velocity[j]
                    + p.position[i] * k[i][j] * p.position[j]);
        }
    }
    energy
}
fn take(length: f64, rate: u32, steps: usize, case: usize) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (f64::from(rate) * steps as f64);
    let ap = ActionProfile::default();
    let mp = ModalAssemblyProfile::default();
    let fp = FeltDamperProfile::default();
    let pp = profile(case);
    let mut v = PolarizedActionAssembly::new_polarized(
        h,
        TineGeometry {
            length_m: length,
            ..TineGeometry::default()
        },
        mp,
        fp,
        ap,
        pp,
    )?;
    let m = v.structural_mass_matrix();
    let k = v.structural_stiffness_matrix();
    let c = v.structural_damping_matrix();
    let bh = v.structural_hammer_port();
    let bd = v.structural_damper_port();
    let stiffness = [
        mp.contact_stiffness_n_m2,
        fp.felt_stiffness_n_m2,
        ap.pedestal_stiffness_n_m2,
        ap.bridle_stiffness_n_m2,
    ];
    let mut previous = v.probe();
    let initial_planes = core::array::from_fn::<_, 2, _>(|p| plane_energy(&previous, &m, &k, p));
    let (mut plane_heat, mut plane_input, mut plane_coupling) = ([0.0; 2], [0.0; 2], [0.0; 2]);
    let mut max_plane_defect = [0.0_f64; 2];
    let mut max_balance = 0.0_f64;
    let mut max_contact_work = 0.0_f64;
    let mut max_continuity = 0.0_f64;
    let mut passive_growth = 0.0_f64;
    let mut peaks = [0.0_f64; 2];
    let mut strike_peaks = [0.0_f64; 2];
    let mut all_heat_monotone = true;
    let mut nonnegative = true;
    let frames = (DURATION * f64::from(rate)).round() as usize;
    let mut samples = Vec::with_capacity(frames);
    let mut x = ap.hammer_rest_m;
    let mut xy_moments = [0.0; 5];
    let mut xy_count = 0.0;
    for frame in 0..frames {
        for sub in 0..steps {
            let t = (frame * steps + sub) as f64 * h;
            let target = if (0.01..0.045).contains(&t) || (0.095..0.13).contains(&t) {
                -ap.escapement_m
            } else {
                ap.hammer_rest_m
            };
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            let a = previous;
            v.advance(x, ap.damper_closed_m)?;
            let b = v.probe();
            previous = b;
            let scale = (b.initial_energy_j + b.absolute_drive_work_j).max(1e-20);
            max_balance = max_balance.max(b.balance_residual_j.abs() / scale);
            let vm: [f64; 18] = core::array::from_fn(|i| 0.5 * (a.velocity[i] + b.velocity[i]));
            let qm: [f64; 18] = core::array::from_fn(|i| 0.5 * (a.position[i] + b.position[i]));
            for plane in 0..2 {
                for i in 0..9 {
                    let j = plane * 9 + i;
                    let other = (1 - plane) * 9 + i;
                    plane_heat[plane] += h * c[j][j] * vm[j] * vm[j];
                    plane_input[plane] +=
                        h * vm[j] * (bh[j] * b.contact_force_n[0] + bd[j] * b.contact_force_n[1]);
                    // This model's cross-plane matrices connect corresponding boundary coordinates.
                    plane_coupling[plane] -=
                        h * vm[j] * (k[j][other] * qm[other] + c[j][other] * vm[other]);
                }
                peaks[plane] = peaks[plane].max(b.pickup_displacement_xy_m[plane].abs());
            }
            for (j, stiffness) in stiffness.iter().enumerate() {
                let du = stiffness
                    * (b.compression_m[j].max(0.0).powi(3) - a.compression_m[j].max(0.0).powi(3))
                    / 3.0;
                let heat = b.contact_heat_j[j] - a.contact_heat_j[j];
                max_contact_work = max_contact_work.max(
                    (b.contact_force_n[j] * (b.compression_m[j] - a.compression_m[j]) - du - heat)
                        .abs()
                        / scale,
                );
                all_heat_monotone &= heat >= 0.0;
                nonnegative &= b.contact_force_n[j] >= 0.0;
            }
            all_heat_monotone &= b.structural_heat_j >= a.structural_heat_j
                && b.arm_heat_j >= a.arm_heat_j
                && b.hammer_return_heat_j >= a.hammer_return_heat_j;
            for j in 0..20 {
                max_continuity = max_continuity.max(
                    (b.position[j] - a.position[j] - 0.5 * h * (a.velocity[j] + b.velocity[j]))
                        .abs(),
                );
            }
            if x == a.pedestal_position_m {
                passive_growth =
                    passive_growth.max((b.mechanical_energy_j - a.mechanical_energy_j) / scale);
            }
            let window = usize::from(t >= 0.09);
            strike_peaks[window] = strike_peaks[window].max(b.contact_force_n[0]);
        }
        let b = previous;
        let scale = (b.initial_energy_j + b.absolute_drive_work_j).max(1e-20);
        for plane in 0..2 {
            let defect = plane_energy(&b, &m, &k, plane) + plane_heat[plane]
                - initial_planes[plane]
                - plane_input[plane]
                - plane_coupling[plane];
            max_plane_defect[plane] = max_plane_defect[plane].max(defect.abs() / scale);
        }
        samples.push([
            b.pickup_velocity_xy_m_s[0],
            b.pickup_velocity_xy_m_s[1],
            b.position[18],
            b.position[19],
        ]);
        if frame >= (0.03 * f64::from(rate)).round() as usize {
            let [x, y] = b.pickup_displacement_xy_m;
            for (sum, value) in xy_moments.iter_mut().zip([x, y, x * x, y * y, x * y]) {
                *sum += value;
            }
            xy_count += 1.0;
        }
    }
    let b = previous;
    let var_x = xy_moments[2] - xy_moments[0].powi(2) / xy_count;
    let var_y = xy_moments[3] - xy_moments[1].powi(2) / xy_count;
    let covariance = xy_moments[4] - xy_moments[0] * xy_moments[1] / xy_count;
    let orbit_rank = if var_x > 0.0 && var_y > 0.0 {
        (1.0 - covariance * covariance / (var_x * var_y))
            .clamp(0.0, 1.0)
            .sqrt()
    } else {
        0.0
    };
    let behavior = strike_peaks.iter().all(|p| *p > 0.0)
        && if case < 2 {
            peaks[1] == 0.0
        } else {
            peaks[1] > 1e-9 && orbit_rank > 1e-3
        };
    let passed = max_balance < 1e-8
        && max_contact_work < 1e-9
        && max_plane_defect.iter().all(|d| *d < 1e-8)
        && max_continuity < 1e-14
        && passive_growth < 1e-10
        && nonnegative
        && all_heat_monotone
        && behavior;
    Ok(Take {
        samples,
        summary: json!({"steps_per_frame":steps,"passed":passed,"behavior_passed":behavior,
        "max_relative_balance_defect":max_balance,"max_relative_contact_work_defect":max_contact_work,
        "max_relative_plane_work_defect":max_plane_defect,"max_kinematic_defect":max_continuity,
        "max_stationary_drive_relative_energy_growth":passive_growth,"heat_monotone":all_heat_monotone,"nonnegative_force":nonnegative,
        "peak_displacement_xy_m":peaks,"orbit_covariance_rank":orbit_rank,"strike_window_peak_n":strike_peaks,
        "plane_diagonal_heat_j":plane_heat,"plane_contact_work_j":plane_input,"plane_coupling_work_j":plane_coupling,
        "final_plane_diagonal_energy_j":[plane_energy(&b,&m,&k,0),plane_energy(&b,&m,&k,1)],
        "initial_energy_j":b.initial_energy_j,"final_mechanical_energy_j":b.mechanical_energy_j,
        "pedestal_work_j":b.pedestal_work_j,"pedal_work_j":b.pedal_work_j,"absolute_drive_work_j":b.absolute_drive_work_j,
        "contact_heat_j":b.contact_heat_j,"structural_heat_j":b.structural_heat_j,"arm_heat_j":b.arm_heat_j,
        "hammer_return_heat_j":b.hammer_return_heat_j,"contact_entries":b.contact_entries,
        "maximum_solver_sweeps":b.maximum_solver_sweeps,"final_position":b.position,"final_velocity":b.velocity}),
    })
}
fn compare(a: &Take, b: &Take, rate: u32) -> Value {
    let windows:Vec<_>=[(0.0,0.045),(0.045,0.095),(0.095,0.14),(0.14,DURATION)].into_iter().map(|(lo,hi)| {
        let start=(lo*f64::from(rate)).round() as usize;let end=(hi*f64::from(rate)).round() as usize;
        let mut error=[0.0;4];let mut signal=[0.0;2];
        for (x,y) in a.samples[start..end].iter().zip(&b.samples[start..end]) {
            for j in 0..4 {error[j]+=(x[j]-y[j]).powi(2);}
            for j in 0..2 {signal[j]+=y[j]*y[j];}
        }
        let relative:[f64;2]=core::array::from_fn(|j|(error[j]/signal[j].max(1e-30)).sqrt());
        let positions:[f64;2]=core::array::from_fn(|j|(error[j+2]/(end-start) as f64).sqrt());
        json!({"start_seconds":lo,"end_seconds":hi,"velocity_xy_relative_rmse":relative,"hammer_arm_rmse_m":positions,
            "passed":relative.iter().all(|e|*e<0.01)&&positions.iter().all(|e|*e<1e-5)})
    }).collect();
    json!({"passed":windows.iter().all(|w|w["passed"]==true),"windows":windows})
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if !matches!(args.len(), 3 | 4)
        || args[1] != "--output"
        || (args.len() == 4 && args[3] != "--refined")
    {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|x| x != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let steps = if args.len() == 4 {
        [256, 512, 1024]
    } else {
        [64, 128, 256]
    };
    let mut cases = Vec::new();
    for (length, rate) in [(0.075, 48000), (0.12, 96000)] {
        for (case, name) in NAMES.iter().enumerate() {
            let a = take(length, rate, steps[0], case)?;
            let b = take(length, rate, steps[1], case)?;
            let c = take(length, rate, steps[2], case)?;
            let first = compare(&a, &c, rate);
            let second = compare(&b, &c, rate);
            let passed = [&a, &b, &c].iter().all(|t| t.summary["passed"] == true)
                && first["passed"] == true
                && second["passed"] == true;
            let p = profile(case);
            println!("Polarized action: {length} m, {rate} Hz, {name}, passed={passed}");
            cases.push(json!({"length_m":length,"sample_rate":rate,"case":name,"passed":passed,
            "polarization":{"boundary_angle_rad":p.boundary_angle_rad,"transverse_support_stiffness_ratio":p.transverse_support_stiffness_ratio,
                "transverse_rotation_stiffness_ratio":p.transverse_rotation_stiffness_ratio,"transverse_tonebar_frequency_ratio":p.transverse_tonebar_frequency_ratio,
                "transverse_boundary_damping_ratio":p.transverse_boundary_damping_ratio,"hammer_angle_rad":p.hammer_angle_rad,"felt_angle_rad":p.felt_angle_rad},
            "takes":[a.summary,b.summary,c.summary],"coarse_vs_fine":first,"medium_vs_fine":second}));
        }
    }
    let report = json!({"schema_version":1,"experiment":"polarized-action-v1","passed":cases.iter().all(|c|c["passed"]==true),"steps_per_frame":steps,"cases":cases,
        "protocol":"Frozen before first run. Eight cases, 24 takes: 75 mm at 48 kHz and 120 mm at 96 kHz (selected paired regimes, not a full factorial grid), each with isotropic, aligned anisotropic, rotated anisotropic and oblique-contact configurations. Repeat the same 180 ms action trajectory at three fixed resolutions. Pedestal speed 1.5 m/s, key down 10-45 and 95-130 ms; pedal closed. Circular tine and tuning inertia unchanged across axes, boundary stiffness/damping derived by reciprocal rotation; no synthetic detune, lateral seed impulse or state resets. All assembly/action/felt parameters use current library defaults; per-case polarization parameters are recorded. Require both strikes, zero lateral motion in both symmetry controls and >1 nm lateral peak with covariance rank >0.001 in coupled/oblique cases. Per-tick total energy defect <1e-8, each contact work defect <1e-9, stationary-drive energy growth <1e-10, midpoint identity <1e-14, monotone heat and nonnegative force; each plane's independent work defect <1e-8 at every output frame. Both lower resolutions versus the highest must have <1% velocity RMSE separately in both physical axes and <10 micrometer hammer/arm position RMSE in each of four windows. Failed studies are retained without changing gates.",
        "scope":"Offline small-deflection two-plane action reduction. Boundary anisotropy and contact angles are hypotheses, not measured geometry or material constants. Does not add geometric large-deflection nonlinearities, torsion, shear/rotary inertia, 3D spring eccentricity, magnetic conversion, audio or host qualification."});
    crate::analysis::write_report(output, &report)?;
    if report["passed"] != true {
        return Err("polarized action retained failed qualification".into());
    }
    Ok(())
}
