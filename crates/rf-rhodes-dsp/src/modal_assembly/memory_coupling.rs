//! Reciprocal connection of the stateful hammer to all nine structural coordinates.
use super::{Matrix, Midpoint, ModalAssemblyProfile, N, Operators, Vector, apply, dot};
use crate::{MemoryHammer, MemoryHammerProbe, MemoryHammerProfile, ModelError, TineGeometry};
mod free_motion;
use free_motion::FreeBank;
mod contact_motion;
use contact_motion::ContactBank;
pub use contact_motion::{MemoryContactInspection, MemoryContactStatus, MemoryContactStep};
mod rk4_contact;
pub use rk4_contact::MemoryModalRk4Step;
use rk4_contact::RkContact;

#[derive(Clone)]
struct Motion {
    q: Vector,
    v: Vector,
    hammer: MemoryHammer,
    heat: f64,
    structural_energy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryModalProbe {
    pub position: [f64; 9],
    pub velocity: [f64; 9],
    pub pickup_displacement_m: f64,
    pub pickup_velocity_m_s: f64,
    pub hammer: MemoryHammerProbe,
    pub structural_energy_j: f64,
    pub structural_heat_j: f64,
    pub mechanical_energy_j: f64,
    pub balance_residual_j: f64,
    /// Structural energy plus independently integrated damping minus hammer port work.
    pub structural_work_residual_j: f64,
}

/// Offline midpoint model with optional certified free and adaptive contact intervals.
/// No pickup voltage, action or plugin integration. Construction prepares fixed matrices.
/// Each tick advances one fixed interval, not an audio frame; longer steps need separate preparation.
pub struct MemoryModalAssembly {
    op: Operators,
    steps: [Midpoint; 2],
    h: f64,
    q: Vector,
    v: Vector,
    hammer: MemoryHammer,
    damped: bool,
    heat: f64,
    structural_energy: f64,
    free: Option<FreeBank>,
    contact: Option<ContactBank>,
    rk_contact: Option<Box<RkContact>>,
}
impl MemoryModalAssembly {
    /// Only structural/damper fields of `structure` are used. Legacy scalar-hammer
    /// mass, stiffness, rate loss and speed fields do not control this experiment.
    pub fn new(
        h: f64,
        geometry: TineGeometry,
        structure: ModalAssemblyProfile,
        hammer: MemoryHammerProfile,
        gap_m: f64,
        speed_m_s: f64,
    ) -> Result<Self, ModelError> {
        structure.validate()?;
        let hammer = MemoryHammer::new(h, hammer, gap_m, speed_m_s)?;
        let op = Operators::prepare(geometry, structure)?;
        let steps = [
            Midpoint::prepare(op.m, op.k, op.c[0], op.hammer, h)?,
            Midpoint::prepare(op.m, op.k, op.c[1], op.hammer, h)?,
        ];
        Ok(Self {
            op,
            steps,
            h,
            q: [0.0; N],
            v: [0.0; N],
            hammer,
            damped: false,
            heat: 0.0,
            structural_energy: 0.0,
            free: None,
            contact: None,
            rk_contact: None,
        })
    }
    pub fn set_damped(&mut self, damped: bool) {
        self.damped = damped;
    }
    pub fn apply_core_impulse(&mut self, impulse_n_s: f64) -> Result<(), ModelError> {
        // An impulse changes only the hammer, so structural energy remains valid.
        self.hammer.apply_core_impulse(impulse_n_s)
    }
    pub fn mass_matrix(&self) -> [[f64; 9]; 9] {
        self.op.m
    }
    /// Offline diagnostic: force the original dense stiffness products for paired timing.
    /// Only the arithmetic path changes; no state, coefficients or prepared steps change.
    pub fn use_dense_stiffness_reference(&mut self) {
        self.op.diagonal_stiffness = false;
    }
    pub fn tick(&mut self) -> Result<MemoryModalProbe, ModelError> {
        self.advance::<true>()
    }
    fn advance<const FAST: bool>(&mut self) -> Result<MemoryModalProbe, ModelError> {
        let (motion, probe) = advance_motion::<FAST>(
            &self.motion(),
            &self.op,
            &self.steps[usize::from(self.damped)],
            self.h,
            self.damped,
            None,
        )?;
        self.commit_motion(motion);
        Ok(probe)
    }
    fn motion(&self) -> Motion {
        Motion {
            q: self.q,
            v: self.v,
            hammer: self.hammer.clone(),
            heat: self.heat,
            structural_energy: self.structural_energy,
        }
    }
    fn commit_motion(&mut self, motion: Motion) {
        self.q = motion.q;
        self.v = motion.v;
        self.hammer = motion.hammer;
        self.heat = motion.heat;
        self.structural_energy = motion.structural_energy;
    }
    /// Current snapshot using the structural energy of the last committed step.
    pub fn probe(&self) -> MemoryModalProbe {
        make_probe(
            &self.op,
            self.q,
            self.v,
            self.heat,
            self.hammer.probe(),
            self.structural_energy,
        )
    }
}
fn advance_motion<const FAST: bool>(
    motion: &Motion,
    op: &Operators,
    step: &Midpoint,
    h: f64,
    damped: bool,
    prepared: Option<&crate::HammerMemory>,
) -> Result<(Motion, MemoryModalProbe), ModelError> {
    let fv = step.free_velocity(motion.q, motion.v);
    let fq: Vector = core::array::from_fn(|i| motion.q[i] + 0.5 * h * (motion.v[i] + fv[i]));
    let compliance = 0.5 * h * dot(op.hammer, step.response);
    let mut hammer = motion.hammer.clone();
    if let Some(prepared) = prepared {
        hammer.use_interval(h, prepared);
    }
    let hp = hammer.advance_against::<FAST>(dot(op.hammer, fq), compliance)?;
    let v: Vector = core::array::from_fn(|i| fv[i] + step.response[i] * hp.contact_force_n);
    let q: Vector =
        core::array::from_fn(|i| fq[i] + 0.5 * h * step.response[i] * hp.contact_force_n);
    let mid = core::array::from_fn(|i| 0.5 * (motion.v[i] + v[i]));
    let loss = h * dot(mid, apply(&op.c[usize::from(damped)], mid));
    let heat = motion.heat + loss;
    let structure = mechanical(op, q, v);
    let energy = structure + hp.mechanical_energy_j;
    if !q.iter().chain(v.iter()).all(|x| x.is_finite())
        || !heat.is_finite()
        || !energy.is_finite()
        || loss < 0.0
    {
        return Err(ModelError("invalid coupled memory hammer step"));
    }
    Ok((
        Motion {
            q,
            v,
            heat,
            hammer,
            structural_energy: structure,
        },
        make_probe(op, q, v, heat, hp, structure),
    ))
}
fn make_probe(
    op: &Operators,
    q: Vector,
    v: Vector,
    heat: f64,
    hammer: MemoryHammerProbe,
    structure: f64,
) -> MemoryModalProbe {
    let energy = structure + hammer.mechanical_energy_j;
    MemoryModalProbe {
        position: q,
        velocity: v,
        pickup_displacement_m: dot(op.pickup, q),
        pickup_velocity_m_s: dot(op.pickup, v),
        hammer,
        structural_energy_j: structure,
        structural_heat_j: heat,
        mechanical_energy_j: energy,
        balance_residual_j: energy + heat + hammer.material.dissipated_energy_j
            - hammer.initial_energy_j
            - hammer.external_work_j,
        structural_work_residual_j: structure + heat - hammer.surface_work_j,
    }
}
fn mechanical(op: &Operators, q: Vector, v: Vector) -> f64 {
    0.5 * (dot(q, op.stiffness_force(q)) + dot(v, apply(&op.m, v)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_diagnostics_follow_committed_motion_and_signed_impulse_work() {
        let mut v = voice(2e-6);
        for i in 0..10000 {
            if i == 1500 {
                v.apply_core_impulse(0.008).unwrap();
            }
            if i == 3000 {
                let before = v.probe();
                let impulse = -0.5 * 0.0038 * before.hammer.core_velocity_m_s;
                v.apply_core_impulse(impulse).unwrap();
                let after = v.probe();
                assert!(after.hammer.external_work_j < before.hammer.external_work_j);
                assert_eq!(before.structural_energy_j, after.structural_energy_j);
                assert_eq!(before.position, after.position);
                assert_eq!(before.hammer.material, after.hammer.material);
            }
            if i == 4500 {
                v.set_damped(true);
            }
            if i == 7000 {
                v.set_damped(false);
            }
            let fresh = || {
                make_probe(
                    &v.op,
                    v.q,
                    v.v,
                    v.heat,
                    v.hammer.probe(),
                    mechanical(&v.op, v.q, v.v),
                )
            };
            assert_eq!(v.probe(), fresh());
            let returned = v.tick().unwrap();
            assert_eq!(returned, v.probe());
            let scale = returned.hammer.initial_energy_j + returned.hammer.absolute_impulse_work_j;
            assert!(returned.balance_residual_j.abs() < scale * 1e-8);
        }
        let before = v.probe();
        assert!(v.apply_core_impulse(f64::NAN).is_err());
        assert_eq!(v.probe(), before);
    }
    fn voice(h: f64) -> MemoryModalAssembly {
        MemoryModalAssembly::new(
            h,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap()
    }
    #[test]
    fn moving_tine_reacts_and_closes_both_work_ledgers_through_reimpact_and_damping() {
        let mut v = voice(1e-7);
        let mut free_heat = 0.0;
        let mut reimpact = false;
        let mut peaks = [0.0_f64; 9];
        for i in 0..100000 {
            if i == 30000 {
                v.apply_core_impulse(0.008).unwrap();
            }
            if i == 60000 {
                v.set_damped(true);
            }
            let before = v.probe();
            let q = v.tick().unwrap();
            let scale = q.hammer.initial_energy_j + q.hammer.absolute_impulse_work_j;
            assert!(q.balance_residual_j.abs() < scale * 1e-8, "{q:?}");
            assert!(q.structural_work_residual_j.abs() < scale * 1e-8);
            assert!(q.hammer.balance_residual_j.abs() < scale * 1e-8);
            assert!(q.mechanical_energy_j <= before.mechanical_energy_j + scale * 1e-10);
            assert!(q.hammer.contact_force_n >= 0.0);
            if q.hammer.contact_force_n == 0.0 && before.hammer.contact_force_n == 0.0 {
                free_heat += q.hammer.material.last_step_heat_j;
            }
            reimpact |= i > 30000 && q.hammer.contact_force_n > 0.0;
            for (peak, x) in peaks.iter_mut().zip(q.position) {
                *peak = peak.max(x.abs());
            }
        }
        assert!(free_heat > 0.0 && reimpact);
        assert!(peaks.iter().all(|x| *x > 1e-15), "{peaks:?}");
        assert!(v.probe().structural_heat_j > 0.0);
    }
    #[test]
    fn coupled_newton_matches_pure_bisection_and_invalid_impulses_preserve_all_state() {
        let mut fast = voice(2e-6);
        let mut reference = voice(2e-6);
        for _ in 0..1000 {
            let a = fast.tick().unwrap();
            let b = reference.advance::<false>().unwrap();
            assert!((a.pickup_velocity_m_s - b.pickup_velocity_m_s).abs() < 1e-8);
            assert!((a.hammer.tip_velocity_m_s - b.hammer.tip_velocity_m_s).abs() < 1e-8);
        }
        let before = fast.probe();
        assert!(fast.apply_core_impulse(f64::NAN).is_err());
        assert_eq!(before, fast.probe());
        let soft = MemoryHammerProfile {
            core_mass_kg: 0.001,
            material: crate::HammerMemoryProfile {
                equilibrium_cubic_n_m2: 0.0,
                memory_stiffness_n_m: 1.0,
                ..crate::HammerMemoryProfile::default()
            },
            ..MemoryHammerProfile::default()
        };
        let mut rejected = MemoryModalAssembly::new(
            0.001,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            soft,
            0.01,
            0.0,
        )
        .unwrap();
        rejected.apply_core_impulse(0.02).unwrap();
        let before = rejected.probe();
        assert!(rejected.tick().is_err());
        assert_eq!(before, rejected.probe());
    }
    #[test]
    fn ungrounded_translation_preserves_total_momentum_across_contact() {
        let profile = ModalAssemblyProfile {
            translation_stiffness_n_m: 0.0,
            translation_damping_n_s_m: 0.0,
            ..ModalAssemblyProfile::default()
        };
        let mut v = MemoryModalAssembly::new(
            1e-6,
            TineGeometry::default(),
            profile,
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let mass = v.mass_matrix();
        for _ in 0..3000 {
            let q = v.tick().unwrap();
            let momentum = dot(mass[0], q.velocity)
                + 0.0038 * q.hammer.core_velocity_m_s
                + 0.0002 * q.hammer.tip_velocity_m_s;
            assert!((momentum - 0.004 * 0.8).abs() < 1e-12, "{momentum}");
        }
    }
    #[test]
    fn no_contact_keeps_stationary_structure_and_damper_switch_adds_no_energy() {
        let mut v = MemoryModalAssembly::new(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.01,
            0.2,
        )
        .unwrap();
        for _ in 0..1000 {
            v.tick().unwrap();
        }
        let before = v.probe();
        assert_eq!(before.position, [0.0; 9]);
        assert_eq!(before.velocity, [0.0; 9]);
        assert_eq!(before.hammer.surface_work_j, 0.0);
        v.set_damped(true);
        assert_eq!(before, v.probe());
    }
}
