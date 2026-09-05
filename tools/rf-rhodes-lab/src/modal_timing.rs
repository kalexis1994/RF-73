use rf_rhodes_dsp::{ModalAssembly, ModalAssemblyProfile, ModalIntegration, TineGeometry};
use serde_json::json;
use std::{error::Error, hint::black_box, io::Write, path::Path, time::Instant};

pub const HELP: &str = "Modal timing:
  modal-timing --output REPORT.json
Measures native mechanics in 128-frame blocks with 1/8/32/73 simultaneous voices.
Includes contact and energy accounting; excludes preparation, pickup and host.
";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3
        || args[1] != "--output"
        || Path::new(&args[2]).extension().is_none_or(|s| s != "json")
    {
        return Err(HELP.into());
    }
    let mut file = crate::new_file(Path::new(&args[2]))?;
    let mut cases = Vec::new();
    for rate in [48000_usize, 192000] {
        for count in [1, 8, 32, 73] {
            cases.push(measure(rate, count)?);
        }
    }
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"modal-block-timing-v1",
            "profile":if cfg!(debug_assertions){"debug"}else{"release"},
            "os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
            "voice_size_bytes":std::mem::size_of::<ModalAssembly>(),
            "duration_seconds":0.25,"block_frames":128,"measured_runs":5,"warmup_runs":1,
            "contact_substeps":32,"steps_per_sample":4,
            "contact_solver":"up to 8 safeguarded Newton evaluations, then 48 bisections if needed",
            "geometry":"Default TineGeometry, cycling lengths 0.05/0.075/0.12 m by voice index; not a calibrated keyboard",
            "profile_parameters":"ModalAssemblyProfile::default()",
            "gesture":"All voices strike at velocity 1 every 50 ms, damper applied 25 ms after each strike",
            "scope":"Serial native mechanics and independent loss ledger. Preparation, reset, block-boundary probes, pickup, filter, mixing, audio driver and host excluded. Wall-clock block times include event handling and timer overhead. No thread priority or realtime scheduling. Timing is observational, not a CI performance gate or host qualification.",
            "cases":cases}),
    )?;
    writeln!(file)?;
    println!("Modal timing completed: 8 cases. Report: {}", args[2]);
    Ok(())
}

fn measure(rate: usize, count: usize) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut voices = (0..count)
        .map(|i| {
            ModalAssembly::new(
                rate as f64,
                TineGeometry {
                    length_m: [0.05, 0.075, 0.12][i % 3],
                    ..TineGeometry::default()
                },
                ModalAssemblyProfile::default(),
                ModalIntegration::Refined {
                    contact_substeps: 32,
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let frames = rate / 4;
    let period = rate / 20;
    let mut totals = Vec::new();
    let mut ratios = Vec::new();
    let mut max_balance = 0.0_f64;
    for run in 0..6 {
        for voice in &mut voices {
            voice.reset();
        }
        let mut total = 0.0;
        for first in (0..frames).step_by(128) {
            let end = (first + 128).min(frames);
            let start = Instant::now();
            for frame in first..end {
                for voice in &mut voices {
                    if frame % period == 0 && !voice.strike(1.0) {
                        return Err("modal block timing strike rejected".into());
                    }
                    if frame % period == period / 2 {
                        voice.set_damped(true);
                    }
                    for _ in 0..4 {
                        black_box(&mut *voice).tick();
                    }
                }
            }
            let elapsed = start.elapsed().as_secs_f64();
            total += elapsed;
            if run > 0 {
                ratios.push(elapsed / ((end - first) as f64 / rate as f64));
            }
            for voice in &voices {
                let p = black_box(voice.probe());
                let balance = p.balance_residual_j.abs() / p.injected_energy_j;
                if !balance.is_finite()
                    || balance > 1e-8
                    || !p.mechanical_energy_j.is_finite()
                    || p.mechanical_energy_j < 0.0
                    || !p
                        .position
                        .iter()
                        .chain(p.velocity.iter())
                        .all(|x| x.is_finite())
                {
                    return Err("modal block timing energy/state check failed".into());
                }
                max_balance = max_balance.max(balance);
            }
        }
        if run > 0 {
            totals.push(total);
        }
    }
    totals.sort_by(f64::total_cmp);
    ratios.sort_by(f64::total_cmp);
    Ok(json!({"sample_rate_hz":rate,"voices":count,
        "minimum_render_seconds":totals[0],"median_render_seconds":totals[2],"maximum_render_seconds":totals[4],
        "median_render_time_over_duration":totals[2]/0.25,
        "measured_blocks":ratios.len(),"blocks_over_deadline":ratios.iter().filter(|r| **r > 1.0).count(),
        "median_block_deadline_ratio":ratios[ratios.len()/2],
        "p99_block_deadline_ratio":ratios[(ratios.len()*99).div_ceil(100)-1],
        "maximum_block_deadline_ratio":ratios.last(),
        "maximum_block_boundary_relative_energy_residual":max_balance}))
}
