//! Bounded, error-controlled free recovery. Contact retains the original solver.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFreeStatus {
    Advanced,
    ContactRequired,
    AccuracyRequired,
}
#[derive(Debug, Clone, Copy)]
pub struct MemoryFreeStep {
    pub status: MemoryFreeStatus,
    /// Difference between one RK4 step and two half steps, without extrapolation.
    pub normalized_state_error: Option<f64>,
    /// Largest half-step or reconstructed endpoint energy/work defect.
    pub relative_energy_defect: Option<f64>,
}

// x, relative velocity, Maxwell extension, heat, material work, force integral.
type State = [f64; 6];
impl MemoryHammer {
    /// Attempt a free interval against the stationary wall used by public tick.
    /// Returns a non-advancing status if contact cannot be excluded or accuracy fails.
    /// The fixed tick interval and its prepared material moments remain unchanged.
    pub fn try_free_step(&mut self, h: f64) -> Result<MemoryFreeStep, ModelError> {
        if !h.is_finite() || !(1e-9..=0.001).contains(&h) {
            return Err(ModelError("free step must be between 1 ns and 1 ms"));
        }
        let before = self.probe();
        // Continuous passive energy bounds each tip velocity by sqrt(2E/mt).
        // This strict travel envelope rules out an interior collision as well as
        // endpoint penetration; endpoint-only tests would miss returning contacts.
        let travel = h * (2.0 * before.mechanical_energy_j / self.p.tip_mass_kg).sqrt();
        let margin = 64.0 * f64::EPSILON * (self.tip.abs() + travel).max(1e-12);
        if self.surface_position != 0.0 || self.tip + travel + margin >= 0.0 {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::ContactRequired,
                normalized_state_error: None,
                relative_energy_defect: None,
            });
        }
        let y = [
            before.material.displacement_m,
            self.vc - self.vt,
            before.material.branch_extension_m,
            0.0,
            0.0,
            0.0,
        ];
        let mu =
            self.p.core_mass_kg * self.p.tip_mass_kg / (self.p.core_mass_kg + self.p.tip_mass_kg);
        let stages = || -> Option<(State, State, State)> {
            let coarse = rk4(y, h, self.p.material, mu)?;
            let half = rk4(y, 0.5 * h, self.p.material, mu)?;
            let fine = rk4(half, 0.5 * h, self.p.material, mu)?;
            Some((coarse, half, fine))
        };
        let Some((coarse, half, fine)) = stages() else {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: None,
                relative_energy_defect: None,
            });
        };
        let energy_scale = (before.initial_energy_j + before.absolute_impulse_work_j).max(1e-30);
        let velocity_scale = (2.0 * energy_scale / mu).sqrt();
        let deformation_scale = (2.0 * energy_scale / self.p.material.memory_stiffness_n_m).sqrt();
        let state_error = ((fine[0] - coarse[0]).abs() / deformation_scale)
            .max((fine[1] - coarse[1]).abs() / velocity_scale)
            .max((fine[2] - coarse[2]).abs() / deformation_scale);
        let mut defect = 0.0_f64;
        let mut passive = true;
        for (a, b) in [(y, half), (half, fine)] {
            let ua = potential(a, self.p.material);
            let ub = potential(b, self.p.material);
            let ea = ua + 0.5 * mu * a[1] * a[1];
            let eb = ub + 0.5 * mu * b[1] * b[1];
            let heat = b[3] - a[3];
            let work = b[4] - a[4];
            defect = defect
                .max((eb + heat - ea).abs() / energy_scale)
                .max((ub + heat - ua - work).abs() / energy_scale);
            passive &= eb <= ea + energy_scale * 1e-14 && heat >= 0.0;
        }
        if !coarse
            .iter()
            .chain(half.iter())
            .chain(fine.iter())
            .all(|x| x.is_finite())
            || !state_error.is_finite()
            || !defect.is_finite()
            || state_error > 1e-10
            || defect > 1e-13
            || !passive
            || [coarse[0], half[0], fine[0]].iter().any(|x| x.abs() > 0.01)
        {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: state_error.is_finite().then_some(state_error),
                relative_energy_defect: defect.is_finite().then_some(defect),
            });
        }
        let mass = self.p.core_mass_kg + self.p.tip_mass_kg;
        let center = (self.p.core_mass_kg * self.core + self.p.tip_mass_kg * self.tip) / mass;
        let velocity = (self.p.core_mass_kg * self.vc + self.p.tip_mass_kg * self.vt) / mass;
        let core = center + h * velocity + self.p.tip_mass_kg / mass * fine[0];
        let tip = center + h * velocity - self.p.core_mass_kg / mass * fine[0];
        let vc = velocity + self.p.tip_mass_kg / mass * fine[1];
        let vt = velocity - self.p.core_mass_kg / mass * fine[1];
        if ![core, tip, vc, vt].iter().all(|x| x.is_finite()) || tip >= 0.0 {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: Some(state_error),
                relative_energy_defect: Some(defect),
            });
        }
        let mut next = self.clone();
        next.material
            .commit_free_motion(core - tip, fine[2], fine[3], fine[4], fine[5] / h)?;
        next.core = core;
        next.tip = tip;
        next.vc = vc;
        next.vt = vt;
        next.force = 0.0;
        let after = next.probe();
        defect = defect
            .max(
                (after.mechanical_energy_j + fine[3] - before.mechanical_energy_j).abs()
                    / energy_scale,
            )
            .max(
                (after.material.stored_energy_j + fine[3]
                    - before.material.stored_energy_j
                    - fine[4])
                    .abs()
                    / energy_scale,
            );
        if !defect.is_finite()
            || defect > 1e-13
            || after.mechanical_energy_j > before.mechanical_energy_j + energy_scale * 1e-14
        {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: Some(state_error),
                relative_energy_defect: defect.is_finite().then_some(defect),
            });
        }
        *self = next;
        // Stationary wall: no contact impulse/work in a certified free interval.
        Ok(MemoryFreeStep {
            status: MemoryFreeStatus::Advanced,
            normalized_state_error: Some(state_error),
            relative_energy_defect: Some(defect),
        })
    }
}
fn potential(y: State, p: HammerMemoryProfile) -> f64 {
    0.5 * p.equilibrium_stiffness_n_m * y[0] * y[0]
        + p.equilibrium_cubic_n_m2 * y[0].abs().powi(3) / 3.0
        + 0.5 * p.memory_stiffness_n_m * y[2] * y[2]
}
fn rhs(y: State, p: HammerMemoryProfile, mu: f64) -> State {
    let force = p.equilibrium_stiffness_n_m * y[0]
        + p.equilibrium_cubic_n_m2 * y[0] * y[0].abs()
        + p.memory_stiffness_n_m * y[2];
    [
        y[1],
        -force / mu,
        y[1] - y[2] / p.relaxation_seconds,
        p.memory_stiffness_n_m * y[2] * y[2] / p.relaxation_seconds,
        force * y[1],
        force,
    ]
}
fn rk4(y: State, h: f64, p: HammerMemoryProfile, mu: f64) -> Option<State> {
    let valid = |y: State| y.iter().all(|v| v.is_finite()) && y[0].abs() <= 0.01;
    let a = rhs(y, p, mu);
    let yb = core::array::from_fn(|i| y[i] + 0.5 * h * a[i]);
    if !valid(yb) {
        return None;
    }
    let b = rhs(yb, p, mu);
    let yc = core::array::from_fn(|i| y[i] + 0.5 * h * b[i]);
    if !valid(yc) {
        return None;
    }
    let c = rhs(yc, p, mu);
    let yd = core::array::from_fn(|i| y[i] + h * c[i]);
    if !valid(yd) {
        return None;
    }
    let d = rhs(yd, p, mu);
    let end = core::array::from_fn(|i| y[i] + h / 6.0 * (a[i] + 2.0 * b[i] + 2.0 * c[i] + d[i]));
    valid(end).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_free_motion_agrees_with_the_analytic_maxwell_relative_mode() {
        let p = MemoryHammerProfile {
            material: HammerMemoryProfile {
                equilibrium_cubic_n_m2: 0.0,
                ..HammerMemoryProfile::default()
            },
            ..MemoryHammerProfile::default()
        };
        let mut v = MemoryHammer::new(1e-8, p, 0.01, 0.0).unwrap();
        v.apply_core_impulse(0.001).unwrap();
        let w0 = 0.001 / p.core_mass_kg;
        let decay = 0.5 / p.material.relaxation_seconds;
        let omega = (p.material.memory_stiffness_n_m
            * (1.0 / p.core_mass_kg + 1.0 / p.tip_mass_kg)
            - decay * decay)
            .sqrt();
        for i in 1..=20000 {
            assert_eq!(
                v.try_free_step(5e-8).unwrap().status,
                MemoryFreeStatus::Advanced
            );
            let t = f64::from(i) * 5e-8;
            let envelope = w0 * (-decay * t).exp();
            let e = envelope * (omega * t).sin() / omega;
            let w = envelope * ((omega * t).cos() + decay / omega * (omega * t).sin());
            let q = v.probe();
            assert!((q.material.branch_extension_m - e).abs() * omega / w0 < 1e-8);
            assert!((q.core_velocity_m_s - q.tip_velocity_m_s - w).abs() / w0 < 1e-8);
        }
    }
    #[test]
    fn rigid_free_translation_accepts_large_steps_and_preserves_the_fixed_tick() {
        let p = MemoryHammerProfile::default();
        let mut v = MemoryHammer::new(1e-8, p, 0.01, 0.2).unwrap();
        let q = v.try_free_step(0.001).unwrap();
        assert_eq!(q.status, MemoryFreeStatus::Advanced);
        assert!((v.probe().core_position_m + 0.0098).abs() < 1e-16);
        assert_eq!(v.h, 1e-8);
        assert!(v.probe().balance_residual_j.abs() < 1e-18);
        v.tick().unwrap();
        assert!((v.probe().core_position_m - (-0.0098 + 0.2e-8)).abs() < 1e-16);
    }
    #[test]
    fn unproven_clearance_and_invalid_intervals_leave_all_state_unchanged() {
        let mut v = MemoryHammer::new(1e-8, MemoryHammerProfile::default(), 1e-6, 0.8).unwrap();
        let before = v.probe();
        assert_eq!(
            v.try_free_step(1e-5).unwrap().status,
            MemoryFreeStatus::ContactRequired
        );
        assert_eq!(before, v.probe());
        for h in [f64::NAN, 0.0, 0.002] {
            assert!(v.try_free_step(h).is_err());
            assert_eq!(before, v.probe());
        }
    }
    #[test]
    fn nonlinear_free_recovery_matches_fine_contact_solver_without_erasing_heat_or_memory() {
        let p = MemoryHammerProfile::default();
        let mut fast = MemoryHammer::new(1e-8, p, 0.01, 0.0).unwrap();
        fast.apply_core_impulse(0.001).unwrap();
        let mut reference = fast.clone();
        let before = fast.probe();
        assert_eq!(
            fast.try_free_step(1e-4).unwrap().status,
            MemoryFreeStatus::AccuracyRequired
        );
        assert_eq!(before, fast.probe());
        let mut accepted = 0;
        for _ in 0..10000 {
            let mut remaining = 8;
            while remaining > 0 {
                let mut ticks = remaining;
                loop {
                    let attempt = fast.try_free_step(ticks as f64 * 1e-8).unwrap();
                    if attempt.status == MemoryFreeStatus::Advanced {
                        accepted += 1;
                        remaining -= ticks;
                        break;
                    }
                    assert!(ticks > 1, "{attempt:?}");
                    ticks /= 2;
                }
            }
            for _ in 0..8 {
                reference.tick().unwrap();
            }
        }
        assert!(accepted < 80000);
        let a = fast.probe();
        let b = reference.probe();
        assert!(
            (a.tip_velocity_m_s - b.tip_velocity_m_s).abs() < 1e-5,
            "{a:?} {b:?}"
        );
        assert!(a.balance_residual_j.abs() < a.absolute_impulse_work_j * 1e-8);
        assert!(a.material.balance_residual_j.abs() < a.absolute_impulse_work_j * 1e-8);
        assert!(a.material.dissipated_energy_j > 0.0);
        assert_eq!(a.surface_impulse_n_s, 0.0);
        assert!(
            (p.core_mass_kg * a.core_velocity_m_s + p.tip_mass_kg * a.tip_velocity_m_s - 0.001)
                .abs()
                < 1e-14
        );
    }
}
