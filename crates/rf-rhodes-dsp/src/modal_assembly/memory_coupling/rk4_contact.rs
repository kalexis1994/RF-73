//! Coupled RK4 compressed contact with independent reciprocal work quadratures.
use super::*;
use crate::modal_assembly::numerics::inverse;

pub(super) struct RkContact {
    inverse_mass: Matrix,
    inverse_relative_mass: f64,
}
#[derive(Debug, Clone, Copy)]
pub struct MemoryModalRk4Step {
    pub contact: MemoryContactStep,
    pub relative_energy_defect: Option<f64>,
}
impl MemoryModalRk4Step {
    pub const ENERGY_DEFECT_LIMIT: f64 = 1e-13;
}
fn rejected(status: MemoryContactStatus) -> MemoryModalRk4Step {
    MemoryModalRk4Step {
        contact: MemoryContactStep {
            status,
            normalized_state_error: None,
            mean_contact_force_n: None,
        },
        relative_energy_defect: None,
    }
}
#[derive(Clone, Copy)]
struct State {
    q: Vector,
    v: Vector,
    // Same ten trajectory coordinates/quadratures as the fixed-wall experiment.
    hammer: [f64; 10],
    heat: f64,
    work: f64,
}
impl MemoryModalAssembly {
    /// Prepare the full inverse structural mass without changing physical state.
    pub fn prepare_rk4_contact(&mut self) -> Result<(), ModelError> {
        let inv = inverse(self.op.m)?;
        let relative =
            dot(self.op.hammer, apply(&inv, self.op.hammer)) + self.hammer.inverse_tip_mass();
        if !inv.iter().flatten().all(|x| x.is_finite()) || !relative.is_finite() || relative <= 0.0
        {
            return Err(ModelError("invalid RK4 contact inverse mass"));
        }
        self.rk_contact = Some(Box::new(RkContact {
            inverse_mass: inv,
            inverse_relative_mass: relative,
        }));
        Ok(())
    }
    /// Prepared payload only, excluding the allocator and inline optional pointer.
    pub fn rk4_contact_operator_bytes(&self) -> usize {
        self.rk_contact
            .as_ref()
            .map_or(0, |_| core::mem::size_of::<RkContact>())
    }
    /// Attempt 2^level base ticks, levels 0..12 and at most 1 ms.
    /// No extrapolation, allocation or state change on rejection; callers bound events.
    pub fn try_rk4_contact_step(&mut self, level: u32) -> Result<MemoryModalRk4Step, ModelError> {
        let bank = self
            .rk_contact
            .as_ref()
            .ok_or(ModelError("RK4 contact has not been prepared"))?;
        if level > 12 {
            return Err(ModelError("RK4 contact level exceeds its bounded domain"));
        }
        let h = self.h * (1u32 << level) as f64;
        if h > 0.001 {
            return Err(ModelError("RK4 contact interval exceeds 1 ms"));
        }
        let before = self.probe();
        let hp = before.hammer;
        let surface = dot(self.op.hammer, self.q);
        let gap = hp.tip_position_m - surface;
        let travel = h * (2.0 * before.mechanical_energy_j * bank.inverse_relative_mass).sqrt();
        let margin =
            64.0 * f64::EPSILON * (hp.tip_position_m.abs() + surface.abs() + travel).max(1e-12);
        if !travel.is_finite() || gap - travel - margin <= 0.0 {
            return Ok(rejected(MemoryContactStatus::BoundaryRequired));
        }
        let y = State {
            q: self.q,
            v: self.v,
            hammer: [
                hp.core_position_m,
                hp.tip_position_m,
                hp.core_velocity_m_s,
                hp.tip_velocity_m_s,
                hp.material.branch_extension_m,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
            heat: 0.0,
            work: 0.0,
        };
        let p = self.hammer.profile();
        let c = &self.op.c[usize::from(self.damped)];
        let trial = || -> Option<(State, State, State)> {
            let coarse = rk4(y, h, p, &self.op, c, bank)?;
            let half = rk4(y, 0.5 * h, p, &self.op, c, bank)?;
            let fine = rk4(half, 0.5 * h, p, &self.op, c, bank)?;
            Some((coarse, half, fine))
        };
        let Some((coarse, half, fine)) = trial() else {
            return Ok(rejected(MemoryContactStatus::AccuracyRequired));
        };
        let scale = (hp.initial_energy_j + hp.absolute_impulse_work_j).max(1e-30);
        let error = state_error(coarse, fine, p, &self.op, scale);
        let mut defect = 0.0_f64;
        let mut passive = true;
        for (a, b) in [(y, coarse), (y, half), (half, fine)] {
            let ea = energies(a, p, &self.op);
            let eb = energies(b, p, &self.op);
            let heat = b.hammer[5] - a.hammer[5];
            let structural_heat = b.heat - a.heat;
            let port = b.work - a.work;
            let checks = [
                (eb[0] + heat + structural_heat - ea[0]) / scale,
                (eb[1] + heat - ea[1] - (b.hammer[6] - a.hammer[6])) / scale,
                (eb[2] - ea[2] - (b.hammer[9] - a.hammer[9])) / scale,
                (eb[3] + structural_heat - ea[3] - port) / scale,
                ((eb[0] - eb[3]) + heat - (ea[0] - ea[3]) + port) / scale,
            ];
            if !checks
                .iter()
                .chain(ea.iter())
                .chain(eb.iter())
                .all(|x| x.is_finite())
            {
                return Ok(rejected(MemoryContactStatus::AccuracyRequired));
            }
            for check in checks {
                defect = defect.max(check.abs());
            }
            passive &= heat >= 0.0
                && structural_heat >= 0.0
                && b.hammer[8] >= a.hammer[8]
                && eb[0] <= ea[0] + scale * 1e-14;
        }
        let mut result = rejected(MemoryContactStatus::AccuracyRequired);
        result.contact.normalized_state_error = error.is_finite().then_some(error);
        result.relative_energy_defect = Some(defect);
        if !error.is_finite()
            || error > MemoryContactStep::STATE_ERROR_LIMIT
            || defect > MemoryModalRk4Step::ENERGY_DEFECT_LIMIT
            || !passive
        {
            return Ok(result);
        }
        let hammer = self.hammer.with_contact_trajectory(
            fine.hammer,
            h,
            dot(self.op.hammer, fine.q),
            fine.work,
        )?;
        let motion = Motion {
            q: fine.q,
            v: fine.v,
            hammer,
            heat: self.heat + fine.heat,
            structural_energy: mechanical(&self.op, fine.q, fine.v),
        };
        let after = make_probe(
            &self.op,
            motion.q,
            motion.v,
            motion.heat,
            motion.hammer.probe(),
            motion.structural_energy,
        );
        let checks = [
            (after.balance_residual_j - before.balance_residual_j) / scale,
            (after.structural_work_residual_j - before.structural_work_residual_j) / scale,
            (after.hammer.balance_residual_j - before.hammer.balance_residual_j) / scale,
            (after.hammer.material.balance_residual_j - before.hammer.material.balance_residual_j)
                / scale,
        ];
        if !checks.iter().all(|x| x.is_finite()) {
            return Ok(result);
        }
        for check in checks {
            defect = defect.max(check.abs());
        }
        result.relative_energy_defect = Some(defect);
        if defect > MemoryModalRk4Step::ENERGY_DEFECT_LIMIT
            || !motion.heat.is_finite()
            || after.mechanical_energy_j > before.mechanical_energy_j + scale * 1e-14
        {
            return Ok(result);
        }
        self.commit_motion(motion);
        result.contact.status = MemoryContactStatus::Advanced;
        result.contact.mean_contact_force_n = Some(fine.hammer[8] / h);
        Ok(result)
    }
}
// Total, material, surface potential, and structural mechanical energies.
fn energies(y: State, p: MemoryHammerProfile, op: &Operators) -> [f64; 4] {
    let (hammer, material, surface) =
        MemoryHammer::contact_energies(y.hammer, p, dot(op.hammer, y.q));
    let structural = mechanical(op, y.q, y.v);
    [hammer + structural, material, surface, structural]
}
fn rhs(y: State, p: MemoryHammerProfile, op: &Operators, c: &Matrix, bank: &RkContact) -> State {
    let velocity = dot(op.hammer, y.v);
    let hammer = MemoryHammer::contact_rhs(y.hammer, p, dot(op.hammer, y.q), velocity);
    let elastic = op.stiffness_force(y.q);
    let damping = apply(c, y.v);
    let force = core::array::from_fn(|i| op.hammer[i] * hammer[8] - elastic[i] - damping[i]);
    State {
        q: y.v,
        v: apply(&bank.inverse_mass, force),
        hammer,
        heat: dot(y.v, damping),
        work: hammer[8] * velocity,
    }
}
fn add(y: State, d: State, h: f64) -> State {
    State {
        q: core::array::from_fn(|i| y.q[i] + h * d.q[i]),
        v: core::array::from_fn(|i| y.v[i] + h * d.v[i]),
        hammer: core::array::from_fn(|i| y.hammer[i] + h * d.hammer[i]),
        heat: y.heat + h * d.heat,
        work: y.work + h * d.work,
    }
}
fn rk4(
    y: State,
    h: f64,
    p: MemoryHammerProfile,
    op: &Operators,
    c: &Matrix,
    bank: &RkContact,
) -> Option<State> {
    let valid = |s: State| {
        s.q.iter()
            .chain(s.v.iter())
            .chain(s.hammer.iter())
            .chain([s.heat, s.work].iter())
            .all(|x| x.is_finite())
            && s.hammer[1] > dot(op.hammer, s.q)
            && (s.hammer[0] - s.hammer[1]).abs() <= 0.01
    };
    if !valid(y) {
        return None;
    }
    let a = rhs(y, p, op, c, bank);
    let yb = add(y, a, 0.5 * h);
    if !valid(yb) {
        return None;
    }
    let b = rhs(yb, p, op, c, bank);
    let yc = add(y, b, 0.5 * h);
    if !valid(yc) {
        return None;
    }
    let cc = rhs(yc, p, op, c, bank);
    let yd = add(y, cc, h);
    if !valid(yd) {
        return None;
    }
    let d = rhs(yd, p, op, c, bank);
    let sum = |a: f64, b: f64, c: f64, d: f64| a + 2.0 * b + 2.0 * c + d;
    let delta = State {
        q: core::array::from_fn(|i| sum(a.q[i], b.q[i], cc.q[i], d.q[i])),
        v: core::array::from_fn(|i| sum(a.v[i], b.v[i], cc.v[i], d.v[i])),
        hammer: core::array::from_fn(|i| sum(a.hammer[i], b.hammer[i], cc.hammer[i], d.hammer[i])),
        heat: sum(a.heat, b.heat, cc.heat, d.heat),
        work: sum(a.work, b.work, cc.work, d.work),
    };
    let end = add(y, delta, h / 6.0);
    valid(end).then_some(end)
}
fn state_error(a: State, b: State, p: MemoryHammerProfile, op: &Operators, scale: f64) -> f64 {
    let dq = core::array::from_fn(|i| a.q[i] - b.q[i]);
    let dv = core::array::from_fn(|i| a.v[i] - b.v[i]);
    let dc = a.hammer[0] - b.hammer[0];
    let dt = a.hammer[1] - b.hammer[1];
    let de = a.hammer[4] - b.hammer[4];
    let xa = a.hammer[0] - a.hammer[1];
    let xb = b.hammer[0] - b.hammer[1];
    let ga = a.hammer[1] - dot(op.hammer, a.q);
    let gb = b.hammer[1] - dot(op.hammer, b.q);
    let metric = dot(dv, apply(&op.m, dv))
        + dot(dq, op.stiffness_force(dq))
        + p.core_mass_kg * (a.hammer[2] - b.hammer[2]).powi(2)
        + p.tip_mass_kg * (a.hammer[3] - b.hammer[3]).powi(2)
        + p.material.memory_stiffness_n_m * (dc * dc + dt * dt + de * de)
        + (p.material.equilibrium_stiffness_n_m
            + 2.0 * p.material.equilibrium_cubic_n_m2 * xa.abs().max(xb.abs()))
            * (xa - xb).powi(2)
        + 2.0 * p.surface_stiffness_n_m2 * ga.max(gb) * (ga - gb).powi(2);
    (metric / (2.0 * scale)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn voice() -> MemoryModalAssembly {
        MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap()
    }
    #[test]
    fn preparation_rejection_and_acceptance_preserve_reciprocal_state_and_base_ticks() {
        let mut v = voice();
        let mut reference = voice();
        let before = v.probe();
        assert!(v.try_rk4_contact_step(0).is_err());
        v.prepare_rk4_contact().unwrap();
        assert_eq!(before, v.probe());
        assert!(v.try_rk4_contact_step(13).is_err());
        assert_eq!(
            v.try_rk4_contact_step(0).unwrap().contact.status,
            MemoryContactStatus::BoundaryRequired
        );
        assert_eq!(before, v.probe());
        for _ in 0..2000 {
            assert_eq!(v.tick().unwrap(), reference.tick().unwrap());
        }
        let before = v.probe();
        assert_ne!(
            v.try_rk4_contact_step(8).unwrap().contact.status,
            MemoryContactStatus::Advanced
        );
        assert_eq!(before, v.probe());
        for damped in [false, true] {
            v.set_damped(damped);
            reference.set_damped(damped);
            let r = v.try_rk4_contact_step(3).unwrap();
            assert_eq!(r.contact.status, MemoryContactStatus::Advanced, "{r:?}");
            for _ in 0..8 {
                reference.tick().unwrap();
            }
            let a = v.probe();
            let b = reference.probe();
            assert!((a.hammer.tip_velocity_m_s - b.hammer.tip_velocity_m_s).abs() < 1e-6);
            let scale = a.hammer.initial_energy_j;
            assert!(a.balance_residual_j.abs() < scale * 1e-10);
            assert!(a.structural_work_residual_j.abs() < scale * 1e-10);
            assert!(a.hammer.balance_residual_j.abs() < scale * 1e-10);
            assert_eq!(
                r.contact.mean_contact_force_n,
                Some(a.hammer.contact_force_n)
            );
        }
        reference.commit_motion(v.motion());
        assert_eq!(v.tick().unwrap(), reference.tick().unwrap());
    }
    #[test]
    fn ungrounded_rk4_contact_preserves_total_momentum_through_an_impulse() {
        let mut v = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile {
                translation_stiffness_n_m: 0.0,
                translation_damping_n_s_m: 0.0,
                ..ModalAssemblyProfile::default()
            },
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        v.prepare_rk4_contact().unwrap();
        let mut expected = 0.004 * 0.8;
        let mut accepted = 0;
        for frame in 0..4000 {
            if frame == 2000 {
                v.apply_core_impulse(0.001).unwrap();
                expected += 0.001;
            }
            let r = v.try_rk4_contact_step(3).unwrap();
            if r.contact.status == MemoryContactStatus::Advanced {
                accepted += 1;
            } else {
                for _ in 0..8 {
                    v.tick().unwrap();
                }
            }
            let q = v.probe();
            let momentum = dot(v.op.m[0], q.velocity)
                + 0.0038 * q.hammer.core_velocity_m_s
                + 0.0002 * q.hammer.tip_velocity_m_s;
            assert!(
                (momentum - expected).abs() < 1e-12,
                "{momentum} vs {expected}"
            );
        }
        assert!(accepted > 100);
    }
    #[test]
    fn coupled_compressed_motion_converges_at_fourth_order() {
        let mut v = voice();
        v.prepare_rk4_contact().unwrap();
        for _ in 0..2000 {
            v.tick().unwrap();
        }
        let hp = v.probe().hammer;
        let y = State {
            q: v.q,
            v: v.v,
            hammer: [
                hp.core_position_m,
                hp.tip_position_m,
                hp.core_velocity_m_s,
                hp.tip_velocity_m_s,
                hp.material.branch_extension_m,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
            heat: 0.0,
            work: 0.0,
        };
        let p = v.hammer.profile();
        let evolve = |steps: usize| {
            let mut state = y;
            for _ in 0..steps {
                state = rk4(
                    state,
                    256e-9 / steps as f64,
                    p,
                    &v.op,
                    &v.op.c[0],
                    v.rk_contact.as_ref().unwrap(),
                )
                .unwrap();
            }
            state
        };
        let reference = evolve(256);
        let errors =
            [4, 8, 16].map(|n| state_error(evolve(n), reference, p, &v.op, hp.initial_energy_j));
        assert!(
            errors[1] < errors[0] * 0.08 && errors[2] < errors[1] * 0.08,
            "{errors:?}"
        );
        assert!(errors[2] > 1e-14, "roundoff floor: {errors:?}");
    }
}
