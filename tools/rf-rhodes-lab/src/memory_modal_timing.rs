use rf_rhodes_dsp::{MemoryHammerProfile, MemoryModalAssembly, ModalAssemblyProfile, TineGeometry};
use serde_json::json;
use std::{error::Error, hint::black_box, io::Write, path::Path, time::Instant};

pub const HELP: &str = "Stateful modal kernel timing:
  memory-modal-timing --output REPORT.json
  memory-modal-free-timing --output REPORT.json
Measures native mechanics with returned diagnostics, excluding preparation.
Free timing compares adaptive and uniform paths at the same fine base step.
Four provisional profiles, three repetitions each; no audio or realtime qualification.
";
pub fn run_free(args: &[String]) -> Result<(), Box<dyn Error>> {
    use crate::memory_modal_check::free_controller::Controller;
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    let h = 1.0 / (48000.0 * 16672.0);
    for length in [0.05, 0.12] {
        for speed in [0.2, 0.8] {
            let mut runs = Vec::new();
            let mut uniform_times = Vec::new();
            let mut adaptive_times = Vec::new();
            for repetition in 0..3 {
                // Alternate order to expose, rather than consistently favor, warm-up effects.
                for adaptive in if repetition % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let prep = Instant::now();
                    let mut v = MemoryModalAssembly::new(
                        h,
                        TineGeometry {
                            length_m: length,
                            ..TineGeometry::default()
                        },
                        ModalAssemblyProfile::default(),
                        MemoryHammerProfile::default(),
                        0.0,
                        speed,
                    )?;
                    if adaptive {
                        v.prepare_free_steps(12)?;
                    }
                    let preparation_seconds = prep.elapsed().as_secs_f64();
                    let mut controller = Controller::default();
                    let start = Instant::now();
                    for frame in 0..384 {
                        if frame == 96 {
                            v.apply_core_impulse(2.5 * 0.004 * speed)?;
                        }
                        if frame == 192 {
                            v.set_damped(true);
                        }
                        if frame == 288 {
                            v.set_damped(false);
                        }
                        if adaptive {
                            let mut remaining = 16672;
                            while remaining > 0 {
                                remaining -= controller.advance(&mut v, remaining)?;
                                black_box(v.probe());
                            }
                        } else {
                            for _ in 0..16672 {
                                black_box(v.tick()?);
                            }
                        }
                    }
                    let seconds = start.elapsed().as_secs_f64();
                    let q = v.probe();
                    let scale = q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j;
                    let residual = q.balance_residual_j.abs() / scale;
                    if !residual.is_finite()
                        || residual > 1e-8
                        || !seconds.is_finite()
                        || seconds <= 0.0
                    {
                        return Err("invalid timing run energy or clock".into());
                    }
                    if adaptive {
                        adaptive_times.push(seconds);
                    } else {
                        uniform_times.push(seconds);
                    }
                    runs.push(json!({"repetition":repetition,"adaptive":adaptive,"elapsed_seconds":seconds,
                        "preparation_seconds":preparation_seconds,"free_operator_reserved_bytes":v.free_operator_bytes(),
                        "final_relative_energy_residual":residual,"final_position":q.position,"final_velocity":q.velocity,
                        "final_core_velocity_m_s":q.hammer.core_velocity_m_s,"final_tip_velocity_m_s":q.hammer.tip_velocity_m_s,
                        "controller":if adaptive {controller.report(h)} else {serde_json::Value::Null}}));
                }
            }
            uniform_times.sort_by(f64::total_cmp);
            adaptive_times.sort_by(f64::total_cmp);
            cases.push(json!({"tine_length_m":length,"launch_speed_m_s":speed,
                "uniform_median_seconds":uniform_times[1],"adaptive_median_seconds":adaptive_times[1],
                "median_speedup":uniform_times[1]/adaptive_times[1],
                "adaptive_median_seconds_per_simulated_second":adaptive_times[1]/0.008,"runs":runs}));
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-free-native-timing-v1",
        "status":"pass","step_seconds":h,"uniform_steps_per_take":384*16672,"simulated_seconds":0.008,
        "voice_inline_bytes":std::mem::size_of::<MemoryModalAssembly>(),
        "scope":"Same fine base step and 8 ms event protocol for both paths. Three paired repetitions per provisional profile with alternating order. Preparation is measured separately; execution consumes returned probes and includes adaptive control and rejections. No per-step audit inside timing; final energy checked and separate trajectory audit required. Heap payload excludes allocator overhead. No pickup, mixing, host, WASM or realtime qualification. Timing has no machine-dependent pass threshold.","cases":cases}),
    )?;
    writeln!(file)?;
    println!("Stateful modal free timing completed: {}", args[2]);
    Ok(())
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
    for length in [0.05, 0.12] {
        for speed in [0.2, 0.8] {
            let mut runs = Vec::new();
            let mut times = Vec::new();
            for _ in 0..3 {
                let mut v = MemoryModalAssembly::new(
                    1.0 / (48000.0 * 8336.0),
                    TineGeometry {
                        length_m: length,
                        ..TineGeometry::default()
                    },
                    ModalAssemblyProfile::default(),
                    MemoryHammerProfile::default(),
                    0.0,
                    speed,
                )?;
                let start = Instant::now();
                for frame in 0..384 {
                    if frame == 96 {
                        v.apply_core_impulse(2.5 * 0.004 * speed)?;
                    }
                    if frame == 192 {
                        v.set_damped(true);
                    }
                    if frame == 288 {
                        v.set_damped(false);
                    }
                    for _ in 0..8336 {
                        black_box(v.tick()?);
                    }
                }
                let seconds = start.elapsed().as_secs_f64();
                let q = v.probe();
                let residual = q.balance_residual_j.abs()
                    / (q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j);
                if !residual.is_finite()
                    || residual > 1e-8
                    || !seconds.is_finite()
                    || seconds <= 0.0
                {
                    return Err("invalid timing run energy or clock".into());
                }
                times.push(seconds);
                runs.push(json!({"elapsed_seconds":seconds,"final_relative_energy_residual":residual,
                    "final_position":q.position,"final_velocity":q.velocity,
                    "final_core_velocity_m_s":q.hammer.core_velocity_m_s,"final_tip_velocity_m_s":q.hammer.tip_velocity_m_s}));
            }
            times.sort_by(f64::total_cmp);
            cases.push(json!({"tine_length_m":length,"launch_speed_m_s":speed,
                "median_elapsed_seconds":times[1],"median_seconds_per_simulated_second":times[1]/0.008,
                "runs":runs}));
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-native-kernel-timing-v1",
        "status":"pass","scope":"Single voice fixed-step mechanics with returned probes consumed by black_box. Constructor/matrix preparation and final audit are outside the timed region. No pickup voltage, mixing, host, WASM or realtime qualification. Three sequential repetitions are observations, not statistical confidence intervals.",
        "step_seconds":1.0/(48000.0*8336.0),"steps_per_take":384*8336,"simulated_seconds":0.008,
        "voice_bytes":std::mem::size_of::<MemoryModalAssembly>(),"cases":cases}),
    )?;
    writeln!(file)?;
    println!(
        "Stateful modal kernel timing passed: four cases, twelve runs. Report: {}",
        args[2]
    );
    Ok(())
}
