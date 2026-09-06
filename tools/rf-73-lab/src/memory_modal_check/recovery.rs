//! Compare post-impact free motion without inherited differences between paths.
use super::*;
use serde_json::Value;

pub const HELP: &str = "Shared post-separation recovery:
  memory-modal-recovery-check --output REPORT.json
Four profiles; reproduce the default 80 ms impulse branch, checkpoint at 96 ms.
Five identical-state continuations to 128 ms: two adaptive paths and three
uniform grids. Every interval must remain force-free; all existing gates apply.
No audio, physical calibration, changed tolerances or realtime timing.
";
const BASE: usize = 16672;
const START: usize = 4608;
const END: usize = 6144;
const PATHS: [(&str, usize, bool, u32); 5] = [
    ("rk4_default", BASE, true, 12),
    ("rk4_free_8", BASE, true, 8),
    ("uniform_8336", 8336, false, 0),
    ("uniform_16672", BASE, false, 0),
    ("uniform_20832", 20832, false, 0),
];
const PAIRS: [(usize, usize); 6] = [(0, 1), (0, 3), (0, 4), (1, 4), (2, 3), (3, 4)];

fn separated(q: MemoryModalProbe) -> bool {
    q.hammer.contact_force_n == 0.0 && q.hammer.surface_energy_j == 0.0
}
#[derive(Default)]
struct Audit {
    residuals: [f64; 3],
    positive: f64,
    invalid: bool,
    contact_intervals: usize,
}
impl Audit {
    fn observe(&mut self, before: MemoryModalProbe, q: MemoryModalProbe) {
        let scale = q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j;
        self.invalid |= !scale.is_finite() || scale <= 0.0;
        for (maximum, value) in self.residuals.iter_mut().zip([
            q.balance_residual_j,
            q.structural_work_residual_j,
            q.hammer.balance_residual_j,
        ]) {
            self.invalid |= !value.is_finite();
            *maximum = maximum.max(value.abs() / scale);
        }
        let growth = (q.mechanical_energy_j - before.mechanical_energy_j) / scale;
        self.invalid |= !growth.is_finite()
            || !q.hammer.contact_force_n.is_finite()
            || q.hammer.contact_force_n < 0.0
            || q.hammer.material.last_step_heat_j < 0.0
            || !q.hammer.material.last_step_heat_j.is_finite()
            || !q.structural_heat_j.is_finite()
            || q.structural_heat_j < before.structural_heat_j;
        self.positive = self.positive.max(growth);
        self.contact_intervals += usize::from(!separated(q));
    }
    fn passed(&self) -> bool {
        !self.invalid && self.residuals.iter().all(|r| *r < 1e-8) && self.positive < 1e-10
    }
    fn report(&self) -> Value {
        json!({"energy_and_work_passed":self.passed(),"maximum_relative_energy_and_port_residuals":self.residuals,
            "maximum_positive_relative_energy_step":self.positive,"nonfree_intervals":self.contact_intervals})
    }
}
fn capture(length: f64, tau: f64) -> Result<(checkpoint::Saved, Value), Box<dyn Error>> {
    // Recreate the same independently restarted second branch as the approach study.
    let source = checkpoint::donor(length, tau, true)?.remove(1);
    let h = 1.0 / (48000.0 * BASE as f64);
    let mut voice = source.checkpoint.restart(h)?;
    if voice.probe() != source.probe {
        return Err("recovery source restart changed state".into());
    }
    voice.prepare_free_steps(12)?;
    voice.prepare_rk4_contact()?;
    let mut controller = Controller::with_rk4_contact();
    let mut audit = Audit::default();
    for frame in source.frame..START {
        checkpoint::event(&mut voice, frame)?;
        let mut remaining = BASE;
        while remaining > 0 {
            let before = voice.probe();
            remaining -= controller.advance(&mut voice, remaining)?;
            audit.observe(before, voice.probe());
        }
    }
    if !audit.passed() || audit.contact_intervals == 0 || !separated(voice.probe()) {
        return Err(
            "recovery donor must close energy/work and finish separated after contact".into(),
        );
    }
    let report = json!({"impulse_checkpoint_state":state(source.probe),"propagation_audit":audit.report(),
        "controller":controller.report(h)});
    Ok((
        checkpoint::Saved {
            frame: START,
            checkpoint: voice.checkpoint(),
            probe: voice.probe(),
            first_contact_seconds: source.first_contact_seconds,
        },
        report,
    ))
}
fn continuation(
    saved: &checkpoint::Saved,
    substeps: usize,
    adaptive: bool,
    free: u32,
) -> Result<Take, Box<dyn Error>> {
    let h = 1.0 / (48000.0 * substeps as f64);
    let mut v = saved.checkpoint.restart(h)?;
    if v.probe() != saved.probe {
        return Err("recovery restart changed full physical state".into());
    }
    let mut controller = if free == 8 {
        Controller::with_rk4_limits(12, 8)?
    } else {
        Controller::with_rk4_contact()
    };
    if adaptive {
        v.prepare_free_steps(12)?;
        v.prepare_rk4_contact()?;
    }
    if v.probe() != saved.probe {
        return Err("recovery preparation changed full physical state".into());
    }
    let mut audit = Audit::default();
    let mut states = Vec::new();
    let mut forces = Vec::new();
    for frame in START..END {
        checkpoint::event(&mut v, frame)?;
        let mut remaining = substeps;
        let mut force = 0.0;
        while remaining > 0 {
            let before = v.probe();
            let consumed = if adaptive {
                controller.advance(&mut v, remaining)?
            } else {
                v.tick()?;
                1
            };
            remaining -= consumed;
            let q = v.probe();
            audit.observe(before, q);
            force += (if adaptive {
                controller.mean_force()
            } else {
                None
            })
            .unwrap_or(q.hammer.contact_force_n)
                * consumed as f64
                / substeps as f64;
        }
        states.push(v.probe());
        forces.push(force);
    }
    let pass = audit.passed() && audit.contact_intervals == 0;
    Ok(Take {
        states,
        forces,
        mass: v.mass_matrix(),
        pass,
        report: json!({"passed":pass,"substeps":substeps,
        "initial_state":state(saved.probe),"final_state":state(v.probe()),"audit":audit.report(),
        "controller":if adaptive {controller.report(h)} else {Value::Null}}),
    })
}
fn component_errors(a: &Take, b: &Take) -> Value {
    let mass = a.mass;
    let mut hammer = 0.0;
    let mut structure = 0.0;
    let mut tip = 0.0;
    let mut core = 0.0;
    for (a, b) in a.states.iter().zip(&b.states) {
        let dc = a.hammer.core_velocity_m_s - b.hammer.core_velocity_m_s;
        let dt = a.hammer.tip_velocity_m_s - b.hammer.tip_velocity_m_s;
        core += dc * dc;
        tip += dt * dt;
        hammer += 0.0038 * dc * dc + 0.0002 * dt * dt;
        let dv: [f64; 9] = core::array::from_fn(|i| a.velocity[i] - b.velocity[i]);
        structure += (0..9)
            .map(|i| dv[i] * (0..9).map(|j| dv[j] * mass[i][j]).sum::<f64>())
            .sum::<f64>();
    }
    let scale = a.states.len() as f64 * 0.004 * 0.8 * 0.8;
    json!({"hammer_kinetic_rmse_over_launch":(hammer/scale).sqrt(),"structural_kinetic_rmse_over_launch":(structure/scale).sqrt(),
        "core_velocity_rmse_m_s":(core/a.states.len() as f64).sqrt(),"tip_velocity_rmse_m_s":(tip/a.states.len() as f64).sqrt()})
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
    for length in [0.075, 0.12] {
        for tau in [0.001, 0.01] {
            let (saved, source) = capture(length, tau)?;
            let mut takes = Vec::new();
            for (_, substeps, adaptive, free) in PATHS {
                takes.push(continuation(&saved, substeps, adaptive, free)?);
            }
            let mut pairs = Vec::new();
            for (a, b) in PAIRS {
                let mut row =
                    checkpoint::comparison_frames(&takes[a], &takes[b], START, END - START)?;
                row["candidate"] = json!(PATHS[a].0);
                row["reference"] = json!(PATHS[b].0);
                row["whole_record_components"] = component_errors(&takes[a], &takes[b]);
                pairs.push(row);
            }
            let passed = pairs.iter().all(|p| p["passed"] == true);
            let reports: Vec<_> = takes
                .into_iter()
                .zip(PATHS)
                .map(|(take, path)| json!({"path":path.0,"report":take.report}))
                .collect();
            cases.push(json!({"tine_length_m":length,"relaxation_seconds":tau,"checkpoint_seconds":0.096,
            "checkpoint_state":state(saved.probe),"source":source,"takes":reports,"comparisons":pairs,"passed":passed}));
            println!("Completed shared recovery: length {length} m, relaxation {tau} s.");
        }
    }
    let passed = cases.iter().all(|c| c["passed"] == true);
    serde_json::to_writer_pretty(
        &mut file,
        &json!({"schema_version":1,"experiment":"memory-modal-recovery-v1",
        "status":if passed {"pass"} else {"fail"},"cases":cases,"start_seconds":0.096,"end_seconds":0.128,
        "gates":{"take_energy_and_work":1e-8,"positive_energy_step":1e-10,"nonfree_intervals":0,
            "whole_and_section_kinetic_velocity_rmse":0.01,"whole_and_section_pickup_velocity_rmse":0.01,"whole_and_section_mean_force_rmse":0.02},
        "protocol":"Reproduce the default branch restarted from the donor checkpoint before the 80 ms impulse. Propagate to 96 ms before that frame's damper event; require observed contact and a separated checkpoint. Restart five paths with exactly equal complete physical state and fresh controllers. Apply the original 96/112 ms damper events once. Compare 32 ms of recovery with default/capped free RK4 and uniform midpoint at 8336/16672/20832 ticks per 48 kHz frame. Every interval must have zero contact force and surface energy. Full energy/work/heat checks and whole-record plus separate 2 ms trajectory gates remain unchanged. Component metrics separate hammer and structural kinetic contributions without removing either from acceptance.",
        "scope":"Selected post-impact recovery from a common approximate donor. Initial ledgers are preserved, not reset. Three uniform grids do not establish an exact solution. Whole-record windows use checkpoint-relative time; absolute sections use donor time. No physical coefficients/tolerances changed, audio, action/backcheck, realism or realtime timing qualification. Failures retain reports and exit nonzero; existing reports are preserved."}),
    )?;
    writeln!(file)?;
    if !passed {
        return Err("shared-recovery trajectory qualification failed; see retained report".into());
    }
    println!("Shared-recovery qualification passed: four checkpoints, 20 continuations.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_metrics_keep_hammer_velocity_error_out_of_structural_motion() {
        let voice = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let make = |q| Take {
            states: vec![q; 96],
            forces: vec![0.0; 96],
            mass: voice.mass_matrix(),
            pass: true,
            report: Value::Null,
        };
        let b = make(voice.probe());
        let mut q = voice.probe();
        q.hammer.tip_velocity_m_s += 0.4;
        let a = make(q);
        let result = component_errors(&a, &b);
        assert!((result["tip_velocity_rmse_m_s"].as_f64().unwrap() - 0.4).abs() < 1e-14);
        assert_eq!(result["core_velocity_rmse_m_s"], 0.0);
        assert_eq!(result["structural_kinetic_rmse_over_launch"], 0.0);
        let expected = (0.0002_f64 * 0.4 * 0.4 / (0.004 * 0.8 * 0.8)).sqrt();
        assert!(
            (result["hammer_kinetic_rmse_over_launch"].as_f64().unwrap() - expected).abs() < 1e-14
        );
    }
    #[test]
    fn recovery_requires_both_zero_force_and_zero_surface_energy() {
        let voice = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let q = voice.probe();
        assert!(separated(q));
        let mut force = q;
        force.hammer.contact_force_n = 1.0;
        assert!(!separated(force));
        let mut compressed = q;
        compressed.hammer.surface_energy_j = 1e-10;
        assert!(!separated(compressed));
        let mut audit = Audit::default();
        audit.observe(q, force);
        assert_eq!(audit.contact_intervals, 1);
        force.hammer.material.last_step_heat_j = f64::NAN;
        audit.observe(q, force);
        assert!(!audit.passed());
    }
}
