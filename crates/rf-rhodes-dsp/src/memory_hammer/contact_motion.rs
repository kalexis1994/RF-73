//! Fourth-order trial integration confined to continuously compressed fixed-wall contact.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryHammerContactStatus {
    Advanced,
    BoundaryRequired,
    AccuracyRequired,
}
#[derive(Debug, Clone, Copy)]
pub struct MemoryHammerContactStep {
    pub status: MemoryHammerContactStatus,
    pub normalized_state_error: Option<f64>,
    pub relative_energy_defect: Option<f64>,
    /// Integrated normal impulse divided by the whole accepted interval.
    pub mean_contact_force_n: Option<f64>,
}
impl MemoryHammerContactStep {
    pub const STATE_ERROR_LIMIT: f64 = 1e-11;
    pub const ENERGY_DEFECT_LIMIT: f64 = 1e-13;
}
fn rejected(status: MemoryHammerContactStatus) -> MemoryHammerContactStep {
    MemoryHammerContactStep {
        status,
        normalized_state_error: None,
        relative_energy_defect: None,
        mean_contact_force_n: None,
    }
}

// Core/tip position, core/tip velocity, Maxwell extension, material heat/work,
// material force integral, normal impulse, and work into the surface potential.
type State = [f64; 10];
impl MemoryHammer {
    /// Attempt a compressed interval against the stationary wall at zero.
    /// Requires a continuous compression certificate and a checked RK4 trial.
    /// Rejection preserves all state; accepted motion keeps the base tick preparation.
    /// The caller must stop at external events. No modal moving-port integration.
    pub fn try_contact_step(&mut self, h: f64) -> Result<MemoryHammerContactStep, ModelError> {
        if !h.is_finite() || !(1e-9..=0.001).contains(&h) {
            return Err(ModelError("contact step must be between 1 ns and 1 ms"));
        }
        let before = self.probe();
        let travel = h * (2.0 * before.mechanical_energy_j / self.p.tip_mass_kg).sqrt();
        let margin = 64.0 * f64::EPSILON * (self.tip.abs() + travel).max(1e-12);
        // Passive continuous energy bounds tip speed even during internal energy transfer.
        if self.surface_position != 0.0 || !travel.is_finite() || self.tip - travel - margin <= 0.0
        {
            return Ok(rejected(MemoryHammerContactStatus::BoundaryRequired));
        }
        let y = [
            self.core,
            self.tip,
            self.vc,
            self.vt,
            before.material.branch_extension_m,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        let stages = || -> Option<(State, State, State)> {
            let coarse = rk4(y, h, self.p)?;
            let half = rk4(y, 0.5 * h, self.p)?;
            let fine = rk4(half, 0.5 * h, self.p)?;
            Some((coarse, half, fine))
        };
        let Some((coarse, half, fine)) = stages() else {
            return Ok(rejected(MemoryHammerContactStatus::AccuracyRequired));
        };
        let scale = (before.initial_energy_j + before.absolute_impulse_work_j).max(1e-30);
        let error = state_error(coarse, fine, self.p, scale);
        let mut defect = 0.0_f64;
        let mut passive = true;
        for (a, b) in [(y, coarse), (y, half), (half, fine)] {
            let (ea, ua, sa) = energies(a, self.p);
            let (eb, ub, sb) = energies(b, self.p);
            let heat = b[5] - a[5];
            defect = defect
                .max((eb + heat - ea).abs() / scale)
                .max((ub + heat - ua - (b[6] - a[6])).abs() / scale)
                .max((sb - sa - (b[9] - a[9])).abs() / scale);
            passive &= heat >= 0.0 && eb <= ea + scale * 1e-14 && b[8] >= a[8];
        }
        let mut result = rejected(MemoryHammerContactStatus::AccuracyRequired);
        result.normalized_state_error = error.is_finite().then_some(error);
        result.relative_energy_defect = defect.is_finite().then_some(defect);
        if !error.is_finite()
            || error > MemoryHammerContactStep::STATE_ERROR_LIMIT
            || !defect.is_finite()
            || defect > MemoryHammerContactStep::ENERGY_DEFECT_LIMIT
            || !passive
        {
            return Ok(result);
        }
        let mut next = self.clone();
        next.material.commit_integrated_motion(
            fine[0] - fine[1],
            fine[4],
            fine[5],
            fine[6],
            fine[7] / h,
        )?;
        next.core = fine[0];
        next.tip = fine[1];
        next.vc = fine[2];
        next.vt = fine[3];
        next.force = fine[8] / h;
        next.impulse += fine[8];
        let after = next.probe();
        let momentum = self.p.core_mass_kg * (next.vc - self.vc)
            + self.p.tip_mass_kg * (next.vt - self.vt)
            + fine[8];
        let momentum_scale = (2.0 * scale * (self.p.core_mass_kg + self.p.tip_mass_kg)).sqrt();
        defect = defect
            .max((after.mechanical_energy_j + fine[5] - before.mechanical_energy_j).abs() / scale)
            .max(
                (after.material.stored_energy_j + fine[5]
                    - before.material.stored_energy_j
                    - fine[6])
                    .abs()
                    / scale,
            )
            .max((after.surface_energy_j - before.surface_energy_j - fine[9]).abs() / scale);
        result.relative_energy_defect = defect.is_finite().then_some(defect);
        if !defect.is_finite()
            || defect > MemoryHammerContactStep::ENERGY_DEFECT_LIMIT
            || !next.impulse.is_finite()
            || !next.force.is_finite()
            || !momentum.is_finite()
            || momentum.abs() > momentum_scale * 1e-13
            || after.mechanical_energy_j > before.mechanical_energy_j + scale * 1e-14
        {
            return Ok(result);
        }
        *self = next;
        result.status = MemoryHammerContactStatus::Advanced;
        result.mean_contact_force_n = Some(self.force);
        Ok(result)
    }
}
fn state_error(a: State, b: State, p: MemoryHammerProfile, scale: f64) -> f64 {
    let dc = a[0] - b[0];
    let dt = a[1] - b[1];
    let de = a[4] - b[4];
    let xa = a[0] - a[1];
    let xb = b[0] - b[1];
    let metric = p.core_mass_kg * (a[2] - b[2]).powi(2)
        + p.tip_mass_kg * (a[3] - b[3]).powi(2)
        + p.material.memory_stiffness_n_m * (dc * dc + dt * dt + de * de)
        + (p.material.equilibrium_stiffness_n_m
            + 2.0 * p.material.equilibrium_cubic_n_m2 * xa.abs().max(xb.abs()))
            * (xa - xb).powi(2)
        + 2.0 * p.surface_stiffness_n_m2 * a[1].max(b[1]) * dt * dt;
    (metric / (2.0 * scale)).sqrt()
}
fn energies(y: State, p: MemoryHammerProfile) -> (f64, f64, f64) {
    let x = y[0] - y[1];
    let material = 0.5 * p.material.equilibrium_stiffness_n_m * x * x
        + p.material.equilibrium_cubic_n_m2 * x.abs().powi(3) / 3.0
        + 0.5 * p.material.memory_stiffness_n_m * y[4] * y[4];
    let surface = p.surface_stiffness_n_m2 * y[1].powi(3) / 3.0;
    (
        material + surface + 0.5 * (p.core_mass_kg * y[2] * y[2] + p.tip_mass_kg * y[3] * y[3]),
        material,
        surface,
    )
}
fn rhs(y: State, p: MemoryHammerProfile) -> State {
    let x = y[0] - y[1];
    let w = y[2] - y[3];
    let force = p.material.equilibrium_stiffness_n_m * x
        + p.material.equilibrium_cubic_n_m2 * x * x.abs()
        + p.material.memory_stiffness_n_m * y[4];
    let normal = p.surface_stiffness_n_m2 * y[1] * y[1];
    [
        y[2],
        y[3],
        -force / p.core_mass_kg,
        (force - normal) / p.tip_mass_kg,
        w - y[4] / p.material.relaxation_seconds,
        p.material.memory_stiffness_n_m * y[4] * y[4] / p.material.relaxation_seconds,
        force * w,
        force,
        normal,
        normal * y[3],
    ]
}
fn rk4(y: State, h: f64, p: MemoryHammerProfile) -> Option<State> {
    let valid =
        |s: State| s.iter().all(|x| x.is_finite()) && s[1] > 0.0 && (s[0] - s[1]).abs() <= 0.01;
    if !valid(y) {
        return None;
    }
    let a = rhs(y, p);
    let yb = core::array::from_fn(|i| y[i] + 0.5 * h * a[i]);
    if !valid(yb) {
        return None;
    }
    let b = rhs(yb, p);
    let yc = core::array::from_fn(|i| y[i] + 0.5 * h * b[i]);
    if !valid(yc) {
        return None;
    }
    let c = rhs(yc, p);
    let yd = core::array::from_fn(|i| y[i] + h * c[i]);
    if !valid(yd) {
        return None;
    }
    let d = rhs(yd, p);
    let end = core::array::from_fn(|i| y[i] + h / 6.0 * (a[i] + 2.0 * b[i] + 2.0 * c[i] + d[i]));
    valid(end).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compressed_rk4_trajectory_converges_at_fourth_order() {
        let p = MemoryHammerProfile::default();
        let initial = [8e-6, 5e-6, 0.4, 0.2, 2e-6, 0.0, 0.0, 0.0, 0.0, 0.0];
        let duration = 2e-6;
        let evolve = |steps: usize| {
            let mut y = initial;
            for _ in 0..steps {
                y = rk4(y, duration / steps as f64, p).unwrap();
            }
            y
        };
        let reference = evolve(256);
        let scale = energies(initial, p).0;
        let errors = [4, 8, 16].map(|n| state_error(evolve(n), reference, p, scale));
        assert!(
            errors[1] < errors[0] * 0.08 && errors[2] < errors[1] * 0.08,
            "{errors:?}"
        );
        assert!(
            errors[2] > 1e-14,
            "comparison must remain above roundoff: {errors:?}"
        );
    }
    #[test]
    fn compressed_trials_are_atomic_and_preserve_fixed_step_preparation() {
        let mut v = MemoryHammer::new(1e-9, MemoryHammerProfile::default(), 0.0, 0.8).unwrap();
        let before = v.probe();
        assert_eq!(
            v.try_contact_step(1e-9).unwrap().status,
            MemoryHammerContactStatus::BoundaryRequired
        );
        for h in [f64::NAN, 0.0, 0.002] {
            assert!(v.try_contact_step(h).is_err());
        }
        assert_eq!(before, v.probe());
        for _ in 0..4000 {
            v.tick().unwrap();
        }
        let before = v.probe();
        assert_eq!(
            v.try_contact_step(4e-7).unwrap().status,
            MemoryHammerContactStatus::AccuracyRequired
        );
        assert_eq!(before, v.probe());
        assert_eq!(
            v.try_contact_step(0.001).unwrap().status,
            MemoryHammerContactStatus::BoundaryRequired
        );
        assert_eq!(before, v.probe());
        let result = v.try_contact_step(1e-8).unwrap();
        assert_eq!(result.status, MemoryHammerContactStatus::Advanced);
        assert_eq!(result.mean_contact_force_n, Some(v.probe().contact_force_n));
        assert_eq!(v.h, 1e-9);
        let mut re_prepared = v.clone();
        let preparation = re_prepared.prepare_interval(1e-9).unwrap();
        re_prepared.use_interval(1e-9, &preparation);
        assert_eq!(v.tick().unwrap(), re_prepared.tick().unwrap());
    }
    #[test]
    fn selected_mass_stiffness_and_relaxation_corners_remain_passive() {
        let mut accepted = 0;
        for core in [0.001, 0.02] {
            for tip in [1e-5, 0.001] {
                for stiffness in [1e10, 1e13] {
                    for tau in [1e-6, 1.0] {
                        let p = MemoryHammerProfile {
                            core_mass_kg: core,
                            tip_mass_kg: tip,
                            surface_stiffness_n_m2: stiffness,
                            material: HammerMemoryProfile {
                                relaxation_seconds: tau,
                                ..HammerMemoryProfile::default()
                            },
                        };
                        let mut v = MemoryHammer::new(1e-9, p, 0.0, 0.8).unwrap();
                        for _ in 0..600 {
                            let before = v.probe();
                            let result = v.try_contact_step(1e-7).unwrap();
                            if result.status == MemoryHammerContactStatus::Advanced {
                                accepted += 1;
                            } else {
                                assert_eq!(v.probe(), before);
                                for _ in 0..100 {
                                    v.tick().unwrap();
                                }
                            }
                            let q = v.probe();
                            assert!(
                                q.mechanical_energy_j
                                    <= before.mechanical_energy_j + q.initial_energy_j * 1e-10
                            );
                            assert!(q.balance_residual_j.abs() < q.initial_energy_j * 1e-8);
                            assert!(
                                q.material.balance_residual_j.abs() < q.initial_energy_j * 1e-8
                            );
                            assert!(q.contact_force_n >= 0.0 && q.material.last_step_heat_j >= 0.0);
                        }
                    }
                }
            }
        }
        assert!(accepted > 0);
    }
    #[test]
    fn mixed_contact_steps_match_the_implicit_reference_and_close_work_and_momentum() {
        let p = MemoryHammerProfile::default();
        let mut fast = MemoryHammer::new(1e-8, p, 0.0, 0.8).unwrap();
        let mut reference = MemoryHammer::new(1e-9, p, 0.0, 0.8).unwrap();
        let mut accepted = 0;
        for frame in 0..20000 {
            if frame == 8000 {
                fast.apply_core_impulse(0.008).unwrap();
                reference.apply_core_impulse(0.008).unwrap();
            }
            let mut remaining = 8;
            while remaining > 0 {
                let mut ticks = remaining;
                loop {
                    let result = fast.try_contact_step(ticks as f64 * 1e-8).unwrap();
                    if result.status == MemoryHammerContactStatus::Advanced {
                        accepted += usize::from(ticks > 1);
                        remaining -= ticks;
                        break;
                    }
                    if ticks == 1 {
                        fast.tick().unwrap();
                        remaining -= 1;
                        break;
                    }
                    ticks /= 2;
                }
            }
            for _ in 0..80 {
                reference.tick().unwrap();
            }
            let a = fast.probe();
            let b = reference.probe();
            let scale = a.initial_energy_j + a.absolute_impulse_work_j;
            assert!(a.balance_residual_j.abs() < scale * 1e-8);
            assert!(a.material.balance_residual_j.abs() < scale * 1e-8);
            assert!(a.material.last_step_heat_j >= 0.0 && a.contact_force_n >= 0.0);
            assert!(
                (a.tip_velocity_m_s - b.tip_velocity_m_s).abs() < 1e-4,
                "frame {frame}: candidate {}, reference {}",
                a.tip_velocity_m_s,
                b.tip_velocity_m_s
            );
            let momentum = p.core_mass_kg * a.core_velocity_m_s
                + p.tip_mass_kg * a.tip_velocity_m_s
                + a.surface_impulse_n_s
                - a.applied_impulse_n_s
                - 0.004 * 0.8;
            assert!(momentum.abs() < 1e-12);
        }
        assert!(accepted > 100, "accepted multi-tick intervals: {accepted}");
    }
}
