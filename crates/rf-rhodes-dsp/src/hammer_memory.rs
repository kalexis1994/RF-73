//! Displacement-controlled material coupon. This bilateral law is not contact.
use crate::ModelError;

#[derive(Debug, Clone, Copy)]
pub struct HammerMemoryProfile {
    pub equilibrium_stiffness_n_m: f64,
    pub equilibrium_cubic_n_m2: f64,
    pub memory_stiffness_n_m: f64,
    pub relaxation_seconds: f64,
}
impl Default for HammerMemoryProfile {
    fn default() -> Self {
        Self {
            equilibrium_stiffness_n_m: 0.0,
            equilibrium_cubic_n_m2: 4e10,
            memory_stiffness_n_m: 2e5,
            relaxation_seconds: 0.001,
        }
    }
}
impl HammerMemoryProfile {
    pub fn validate(self) -> Result<(), ModelError> {
        for (v, lo, hi) in [
            (self.equilibrium_stiffness_n_m, 0.0, 1e8),
            (self.equilibrium_cubic_n_m2, 0.0, 1e12),
            (self.memory_stiffness_n_m, 1.0, 1e8),
            (self.relaxation_seconds, 1e-6, 1.0),
        ] {
            if !v.is_finite() || !(lo..=hi).contains(&v) {
                return Err(ModelError(
                    "memory material parameter outside its finite domain",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HammerMemoryProbe {
    pub displacement_m: f64,
    pub viscous_deformation_m: f64,
    pub branch_extension_m: f64,
    /// Instantaneous endpoint reaction; may be negative in this bilateral coupon.
    pub force_n: f64,
    /// Reaction averaged over the last material path (linear for advance_to).
    pub mean_force_n: f64,
    pub stored_energy_j: f64,
    pub dissipated_energy_j: f64,
    pub external_work_j: f64,
    pub absolute_work_j: f64,
    pub last_step_heat_j: f64,
    pub balance_residual_j: f64,
}

/// Cubic/linear equilibrium elasticity in parallel with one Maxwell branch.
/// advance_to prescribes a linear displacement ramp over a fixed timestep.
/// All state, force, work and heat are analytic for that ramp (up to f64 rounding).
#[derive(Clone)]
pub struct HammerMemory {
    p: HammerMemoryProfile,
    decay: f64,
    mean_old: f64,
    mean_delta: f64,
    heat_scale: f64,
    heat_old: f64,
    heat_shift: f64,
    heat_delta: f64,
    x: f64,
    extension: f64,
    heat: f64,
    work: f64,
    absolute_work: f64,
    last_heat: f64,
    mean_force: f64,
}
impl HammerMemory {
    pub fn new(step_seconds: f64, p: HammerMemoryProfile) -> Result<Self, ModelError> {
        p.validate()?;
        if !step_seconds.is_finite() || !(1e-9..=0.1).contains(&step_seconds) {
            return Err(ModelError(
                "memory material step must be between 1 ns and 100 ms",
            ));
        }
        let r = step_seconds / p.relaxation_seconds;
        let u = -(-r).exp_m1();
        let a = u / r;
        let a2 = -(-2.0 * r).exp_m1() / (2.0 * r);
        let b2 = 0.5 * a * a;
        let (b, c2) = if r < 0.1 {
            // Taylor moments avoid cancellation of O(r) terms leaving O(r^3).
            let mut b = 0.0;
            let mut c = 0.0;
            let (mut power, mut factorial, mut two) = (1.0, 2.0, 4.0);
            for n in 0..14 {
                b += power / factorial;
                c += power * (two - 2.0) / (factorial * f64::from(n + 3));
                power *= -r;
                factorial *= f64::from(n + 3);
                two *= 2.0;
            }
            (b, c)
        } else {
            ((1.0 - a) / r, (r - u - 0.5 * u * u) / (r * r * r))
        };
        let schur = c2 - b2 * b2 / a2;
        if !schur.is_finite() || schur <= 0.0 {
            return Err(ModelError("memory heat moment factorization failed"));
        }
        Ok(Self {
            p,
            decay: (-r).exp(),
            mean_old: a,
            mean_delta: b,
            heat_scale: p.memory_stiffness_n_m * r,
            heat_old: a2,
            heat_shift: b2 / a2,
            heat_delta: schur,
            x: 0.0,
            extension: 0.0,
            heat: 0.0,
            work: 0.0,
            absolute_work: 0.0,
            last_heat: 0.0,
            mean_force: 0.0,
        })
    }

    pub fn advance_to(&mut self, displacement_m: f64) -> Result<HammerMemoryProbe, ModelError> {
        if !displacement_m.is_finite() || displacement_m.abs() > 0.01 {
            return Err(ModelError(
                "memory coupon displacement must be within +/-10 mm",
            ));
        }
        let dx = displacement_m - self.x;
        let extension = self.decay * self.extension + self.mean_old * dx;
        let mean_force = self.response_at(displacement_m).0;
        // Integrated eta*z_dot^2 in a positive quadratic form, not E/work subtraction.
        let step_heat = self.heat_scale
            * (self.heat_old * (self.extension + self.heat_shift * dx).powi(2)
                + self.heat_delta * dx * dx);
        let heat = self.heat + step_heat;
        let step_work = mean_force * dx;
        let work = self.work + step_work;
        let absolute_work = self.absolute_work + step_work.abs();
        if ![extension, mean_force, step_heat, heat, work, absolute_work]
            .iter()
            .all(|x| x.is_finite())
        {
            return Err(ModelError("non-finite memory coupon update"));
        }
        self.x = displacement_m;
        self.extension = extension;
        self.mean_force = mean_force;
        self.heat = heat;
        self.work = work;
        self.absolute_work = absolute_work;
        self.last_heat = step_heat;
        Ok(self.probe())
    }

    /// Trial ramp reaction and endpoint tangent for internal coupling solvers.
    /// Only the committed update enforces the coupon displacement domain.
    pub(crate) fn response_at(&self, end: f64) -> (f64, f64) {
        let gradient = cubic_gradient(self.x, end);
        let slope = if self.x >= 0.0 && end >= 0.0 {
            (self.x + 2.0 * end) / 3.0
        } else if self.x <= 0.0 && end <= 0.0 {
            -(self.x + 2.0 * end) / 3.0
        } else {
            (end * end.abs() - gradient) / (end - self.x)
        };
        (
            0.5 * self.p.equilibrium_stiffness_n_m * (self.x + end)
                + self.p.equilibrium_cubic_n_m2 * gradient
                + self.p.memory_stiffness_n_m
                    * (self.mean_old * self.extension + self.mean_delta * (end - self.x)),
            0.5 * self.p.equilibrium_stiffness_n_m
                + self.p.equilibrium_cubic_n_m2 * slope
                + self.p.memory_stiffness_n_m * self.mean_delta,
        )
    }

    /// Internal free-motion quadrature; not a prescribed linear ramp.
    pub(crate) fn commit_free_motion(
        &mut self,
        x: f64,
        extension: f64,
        heat: f64,
        work: f64,
        mean_force: f64,
    ) -> Result<(), ModelError> {
        if ![x, extension, heat, work, mean_force]
            .iter()
            .all(|v| v.is_finite())
            || x.abs() > 0.01
            || heat < 0.0
        {
            return Err(ModelError("invalid free material update"));
        }
        let total_heat = self.heat + heat;
        let total_work = self.work + work;
        let absolute = self.absolute_work + work.abs();
        if ![total_heat, total_work, absolute]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(ModelError("non-finite free material ledger"));
        }
        self.x = x;
        self.extension = extension;
        self.heat = total_heat;
        self.work = total_work;
        self.absolute_work = absolute;
        self.mean_force = mean_force;
        self.last_heat = heat;
        Ok(())
    }

    pub fn probe(&self) -> HammerMemoryProbe {
        let energy = 0.5 * self.p.equilibrium_stiffness_n_m * self.x * self.x
            + self.p.equilibrium_cubic_n_m2 * self.x.abs().powi(3) / 3.0
            + 0.5 * self.p.memory_stiffness_n_m * self.extension * self.extension;
        HammerMemoryProbe {
            displacement_m: self.x,
            viscous_deformation_m: self.x - self.extension,
            branch_extension_m: self.extension,
            force_n: self.p.equilibrium_stiffness_n_m * self.x
                + self.p.equilibrium_cubic_n_m2 * self.x * self.x.abs()
                + self.p.memory_stiffness_n_m * self.extension,
            mean_force_n: self.mean_force,
            stored_energy_j: energy,
            dissipated_energy_j: self.heat,
            external_work_j: self.work,
            absolute_work_j: self.absolute_work,
            last_step_heat_j: self.last_heat,
            balance_residual_j: energy + self.heat - self.work,
        }
    }
    /// Administrative reset of specimen and ledgers, not a physical recovery step.
    pub fn reset(&mut self) {
        self.x = 0.0;
        self.extension = 0.0;
        self.heat = 0.0;
        self.work = 0.0;
        self.absolute_work = 0.0;
        self.last_heat = 0.0;
        self.mean_force = 0.0;
    }
}

fn cubic_gradient(a: f64, b: f64) -> f64 {
    let sign = if a >= 0.0 && b >= 0.0 {
        1.0
    } else if a <= 0.0 && b <= 0.0 {
        -1.0
    } else {
        (b.abs() - a.abs()) / (b - a)
    };
    sign * (a * a + a.abs() * b.abs() + b * b) / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_composes_and_independent_heat_closes_energy_across_time_scales() {
        for (h, tau) in [
            (1e-9, 1.0),
            (1e-8, 0.01),
            (0.00009999, 0.001),
            (0.00010001, 0.001),
            (0.001, 0.001),
            (0.05, 1e-6),
        ] {
            let p = HammerMemoryProfile {
                relaxation_seconds: tau,
                equilibrium_stiffness_n_m: 1e5,
                ..HammerMemoryProfile::default()
            };
            let mut full = HammerMemory::new(h, p).unwrap();
            // The minimum supported step cannot be halved; use two full steps
            // against one double step to retain the same linear path.
            let mut twice = HammerMemory::new(2.0 * h, p).unwrap();
            for x in [1e-5, -2e-5, 4e-5, 0.0, 0.0, -1e-5] {
                let start = full.x;
                full.advance_to((start + x) * 0.5).unwrap();
                let f = full.advance_to(x).unwrap();
                let t = twice.advance_to(x).unwrap();
                let scale = f.absolute_work_j.max(1e-20);
                assert!((f.branch_extension_m - t.branch_extension_m).abs() < 1e-14);
                assert!((f.external_work_j - t.external_work_j).abs() < scale * 1e-10);
                assert!((f.dissipated_energy_j - t.dissipated_energy_j).abs() < scale * 1e-10);
                assert!(f.balance_residual_j.abs() < scale * 1e-10);
                assert!(t.balance_residual_j.abs() < scale * 1e-10);
                assert!(f.last_step_heat_j >= 0.0 && t.last_step_heat_j >= 0.0);
            }
        }
    }

    #[test]
    fn fixed_deformation_relaxes_exponentially_and_rest_preserves_history() {
        let p = HammerMemoryProfile::default();
        let h = p.relaxation_seconds / 100.0;
        let mut v = HammerMemory::new(h, p).unwrap();
        let first = v.advance_to(1e-5).unwrap();
        for i in 1..=1000 {
            let q = v.advance_to(1e-5).unwrap();
            let expected =
                first.branch_extension_m * (-f64::from(i) * h / p.relaxation_seconds).exp();
            assert!((q.branch_extension_m - expected).abs() < 1e-17);
            assert!(q.last_step_heat_j >= 0.0);
        }
        let released = v.advance_to(0.0).unwrap();
        assert!(released.force_n < 0.0); // Clamped specimen reaction, not an adhesive contact.
        for i in 1..=1000 {
            let q = v.advance_to(0.0).unwrap();
            let expected = released.force_n * (-f64::from(i) * h / p.relaxation_seconds).exp();
            assert!((q.force_n - expected).abs() < 1e-11);
            assert!(q.balance_residual_j.abs() < q.absolute_work_j * 1e-10);
        }
        let recovered = v.advance_to(1e-5).unwrap();
        assert!((recovered.force_n - first.force_n).abs() < 1e-3);
    }

    #[test]
    fn sinusoidal_cycle_heat_matches_continuous_maxwell_loss() {
        let p = HammerMemoryProfile::default();
        let frequency = 100.0;
        let amplitude: f64 = 1e-5;
        let rate = 48000;
        let period = rate / 100;
        let mut v = HammerMemory::new(1.0 / f64::from(rate), p).unwrap();
        let mut start_heat = 0.0;
        for i in 1..=20 * period {
            if i == 19 * period + 1 {
                start_heat = v.probe().dissipated_energy_j;
            }
            let x = amplitude * (core::f64::consts::TAU * f64::from(i) / f64::from(period)).sin();
            v.advance_to(x).unwrap();
        }
        let w = core::f64::consts::TAU * frequency * p.relaxation_seconds;
        let expected =
            core::f64::consts::PI * p.memory_stiffness_n_m * amplitude.powi(2) * w / (1.0 + w * w);
        assert!(((v.probe().dissipated_energy_j - start_heat) / expected - 1.0).abs() < 1e-4);
    }

    #[test]
    fn parameter_corners_and_signed_travel_remain_passive() {
        for linear in [0.0, 1e8] {
            for cubic in [0.0, 1e12] {
                for memory in [1.0, 1e8] {
                    for tau in [1e-6, 1.0] {
                        for h in [1e-9, 0.1] {
                            let p = HammerMemoryProfile {
                                equilibrium_stiffness_n_m: linear,
                                equilibrium_cubic_n_m2: cubic,
                                memory_stiffness_n_m: memory,
                                relaxation_seconds: tau,
                            };
                            let mut v = HammerMemory::new(h, p).unwrap();
                            for x in [0.01, 0.01, -0.01, -0.01, 0.0, 0.0] {
                                let q = v.advance_to(x).unwrap();
                                assert!(q.stored_energy_j.is_finite() && q.force_n.is_finite());
                                assert!(q.stored_energy_j >= 0.0 && q.last_step_heat_j >= 0.0);
                                assert!(q.balance_residual_j.abs() < q.absolute_work_j * 1e-10);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_updates_are_atomic_and_reset_clears_memory() {
        let p = HammerMemoryProfile::default();
        for h in [0.0, 1.0, f64::NAN, f64::INFINITY] {
            assert!(HammerMemory::new(h, p).is_err());
        }
        for bad in [
            HammerMemoryProfile {
                relaxation_seconds: 0.0,
                ..p
            },
            HammerMemoryProfile {
                memory_stiffness_n_m: -1.0,
                ..p
            },
            HammerMemoryProfile {
                equilibrium_cubic_n_m2: f64::NAN,
                ..p
            },
        ] {
            assert!(HammerMemory::new(1e-5, bad).is_err());
        }
        let mut v = HammerMemory::new(1e-5, p).unwrap();
        v.advance_to(1e-5).unwrap();
        let before = v.probe();
        for x in [0.0101, -0.0101, f64::NAN, f64::INFINITY] {
            assert!(v.advance_to(x).is_err());
            assert_eq!(v.probe(), before);
        }
        v.reset();
        assert_eq!(v.probe(), HammerMemory::new(1e-5, p).unwrap().probe());
    }
}
