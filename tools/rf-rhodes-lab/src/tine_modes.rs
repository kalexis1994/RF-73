//! Geometry-derived modes and mesh convergence. No audio or plugin mutations.
use rf_rhodes_dsp::{TineGeometry, TineModes};
use serde_json::{Value, json};
use std::{error::Error, io::Write, path::Path};

pub const HELP: &str = "Tine structural preparation:
  tine-modes --output REPORT.json
Audits six Euler-Bernoulli modes at 16/32/64 elements for 12 tuning-mass cases.
Reports SI modal parameters, moving-root inertia, mesh convergence and mode shapes.
Geometry is illustrative, not a measured Rhodes scale; no audio device is opened.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|e| e != "json")
    {
        return Err(HELP.into());
    }
    let mut output = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let mut pass = true;
    for ratio in [0.0, 0.1, 0.5] {
        for position in [0.0, 0.37, 0.85, 1.0] {
            let mut g = TineGeometry {
                tuning_position: position,
                ..TineGeometry::default()
            };
            g.tuning_mass_kg = ratio * g.beam_mass_kg();
            let fine = TineModes::prepare(g, 64)?;
            let mut meshes = Vec::new();
            for elements in [16, 32, 64] {
                let basis = TineModes::prepare(g, elements)?;
                let rows:Vec<_>=basis.modes.iter().zip(fine.modes).enumerate().map(|(i,(mode,reference))| {
                    let f_error=(mode.frequency_hz/reference.frequency_hz-1.0).abs();
                    let hammer_error=(mode.hammer_weight-reference.hammer_weight).abs();
                    let pickup_error=(mode.pickup_weight-reference.pickup_weight).abs();
                    if elements==32 { pass &= f_error<0.002 && hammer_error<0.005 && pickup_error<0.005; }
                    pass &= mode.relative_eigen_residual<1e-4 && basis.maximum_mass_orthogonality_error<1e-8;
                    json!({"mode":i+1,"frequency_hz":mode.frequency_hz,"effective_mass_kg":mode.effective_mass_kg,
                        "hammer_weight":mode.hammer_weight,"pickup_weight":mode.pickup_weight,
                        "translation_coupling_kg":mode.translation_coupling_kg,"rotation_coupling_kg_m":mode.rotation_coupling_kg_m,
                        "relative_eigen_residual":mode.relative_eigen_residual,"frequency_relative_error_vs_64":f_error,
                        "hammer_weight_absolute_error_vs_64":hammer_error,"pickup_weight_absolute_error_vs_64":pickup_error})
                }).collect();
                meshes.push(json!({"elements":elements,"maximum_mass_orthogonality_error":basis.maximum_mass_orthogonality_error,"modes":rows}));
            }
            let shapes: Vec<_> = (0..6)
                .map(|mode| {
                    (0..=64)
                        .map(|i| fine.shape(mode, i as f64 / 64.0))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<_, _>>()?;
            cases.push(
                json!({"geometry":geometry(g),"tuning_mass_over_beam_mass":ratio,"meshes":meshes,
                "moving_root_mass_matrix":fine.moving_root_mass_matrix(),
                "shape_sample_grid":"s = index / 64, index 0..64; one tip-normalized row per mode",
                "shape_samples":shapes}),
            );
        }
    }
    let report = json!({"schema_version":1,"experiment":"uniform-tine-with-moving-point-mass-v1",
        "status":if pass {"pass"}else{"fail"},"calibrated":false,"plugin_integrated":false,
        "model":"Euler-Bernoulli cubic Hermite finite elements, consistent mass, fixed root, translational tuning point mass",
        "gates":{"frequency_relative_difference_32_vs_64":0.002,"port_weight_absolute_difference_32_vs_64":0.005,
            "relative_eigen_residual":1e-4,"mass_orthogonality":1e-8},
        "scope":"Finite mesh agreement, not an absolute error bound or acoustic calibration. Root inertia is a reduction for future coupling; these are fixed-root modes, not assembled instrument eigenmodes. No damping or magnetic transfer is fitted.",
        "cases":cases});
    serde_json::to_writer_pretty(&mut output, &report)?;
    writeln!(output)?;
    if !pass {
        return Err("tine modal audit failed; see report".into());
    }
    println!(
        "Tine modal audit passed: 12 cases, 36 mesh results, six modes each. Report: {}",
        args[2]
    );
    Ok(())
}

fn geometry(g: TineGeometry) -> Value {
    json!({"length_m":g.length_m,"diameter_m":g.diameter_m,"young_modulus_pa":g.young_modulus_pa,
        "density_kg_m3":g.density_kg_m3,"tuning_mass_kg":g.tuning_mass_kg,"tuning_position":g.tuning_position,
        "hammer_position":g.hammer_position,"pickup_position":g.pickup_position,
        "beam_mass_kg":g.beam_mass_kg(),"bending_rigidity_n_m2":g.bending_rigidity_n_m2()})
}
