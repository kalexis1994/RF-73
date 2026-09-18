//! Two-mass material-memory hammer with a fixed-wall API and internal moving port.
use crate::voice::contact_gradient;
use crate::{HammerMemory, HammerMemoryProbe, HammerMemoryProfile, ModelError};
mod contact_motion;
mod free_motion;
pub use contact_motion::{MemoryHammerContactStatus, MemoryHammerContactStep};
pub use free_motion::{MemoryFreeStatus, MemoryFreeStep};

#[derive(Debug, Clone, Copy)]
pub struct MemoryHammerProfile {
    pub core_mass_kg: f64,
    pub tip_mass_kg: f64,
    pub surface_stiffness_n_m2: f64,
    pub material: HammerMemoryProfile,
}
impl Default for MemoryHammerProfile {
    fn default() -> Self {
        Self {
            core_mass_kg: 0.0038,
            tip_mass_kg: 0.0002,
            surface_stiffness_n_m2: 1e12,
            material: HammerMemoryProfile::default(),
        }
    }
}
impl MemoryHammerProfile {
    pub fn validate(self) -> Result<(), ModelError> {
        self.material.validate()?;
        for (v, lo, hi) in [
            (self.core_mass_kg, 0.001, 0.02),
            (self.tip_mass_kg, 1e-5, 0.001),
            (self.surface_stiffness_n_m2, 1e10, 1e13),
        ] {
            if !v.is_finite() || !(lo..=hi).contains(&v) {
                return Err(ModelError(
                    "memory hammer parameter outside its finite domain",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryHammerProbe {
    pub core_position_m: f64,
    pub tip_position_m: f64,
    pub core_velocity_m_s: f64,
    pub tip_velocity_m_s: f64,
    pub material: HammerMemoryProbe,
    pub contact_force_n: f64,
    pub surface_energy_j: f64,
    pub mechanical_energy_j: f64,
    pub initial_energy_j: f64,
    pub external_work_j: f64,
    pub absolute_impulse_work_j: f64,
    pub surface_impulse_n_s: f64,
    /// Work delivered to the moving surface; zero for the fixed-wall experiment.
    pub surface_work_j: f64,
    pub applied_impulse_n_s: f64,
    pub balance_residual_j: f64,
}

/// Offline impact model. Both masses and material remain alive after separation.
/// Public tick uses a stationary wall at zero; the modal wrapper uses an internal
/// reciprocal moving port. No action, pickup voltage or plugin integration.
#[derive(Clone)]
pub struct MemoryHammer {
    p: MemoryHammerProfile,
    h: f64,
    core: f64,
    tip: f64,
    vc: f64,
    vt: f64,
    material: HammerMemory,
    force: f64,
    initial: f64,
    work: f64,
    absolute_work: f64,
    impulse: f64,
    applied_impulse: f64,
    surface_position: f64,
    surface_work: f64,
}
impl MemoryHammer {
    pub(crate) fn prepare_interval(&self, h: f64) -> Result<HammerMemory, ModelError> {
        HammerMemory::new(h, self.p.material)
    }
    pub(crate) fn use_interval(&mut self, h: f64, prepared: &HammerMemory) {
        self.h = h;
        self.material.use_preparation(prepared);
    }
    pub(crate) fn profile(&self) -> MemoryHammerProfile {
        self.p
    }
    pub(crate) fn inverse_tip_mass(&self) -> f64 {
        1.0 / self.p.tip_mass_kg
    }
    pub fn new(
        h: f64,
        p: MemoryHammerProfile,
        gap_m: f64,
        speed_m_s: f64,
    ) -> Result<Self, ModelError> {
        p.validate()?;
        if !h.is_finite()
            || !(1e-9..=0.001).contains(&h)
            || !gap_m.is_finite()
            || !(0.0..=0.01).contains(&gap_m)
            || !speed_m_s.is_finite()
            || !(0.0..=3.0).contains(&speed_m_s)
        {
            return Err(ModelError(
                "invalid memory hammer step, gap or launch speed",
            ));
        }
        Ok(Self {
            p,
            h,
            core: -gap_m,
            tip: -gap_m,
            vc: speed_m_s,
            vt: speed_m_s,
            material: HammerMemory::new(h, p.material)?,
            force: 0.0,
            initial: 0.5 * (p.core_mass_kg + p.tip_mass_kg) * speed_m_s * speed_m_s,
            work: 0.0,
            absolute_work: 0.0,
            impulse: 0.0,
            applied_impulse: 0.0,
            surface_position: 0.0,
            surface_work: 0.0,
        })
    }

    /// Ideal externally applied core impulse. Material and positions are untouched.
    pub fn apply_core_impulse(&mut self, impulse_n_s: f64) -> Result<(), ModelError> {
        if !impulse_n_s.is_finite() || impulse_n_s.abs() > 0.02 {
            return Err(ModelError("core impulse must be within +/-0.02 Ns"));
        }
        let dv = impulse_n_s / self.p.core_mass_kg;
        let work = impulse_n_s * (self.vc + 0.5 * dv);
        let vc = self.vc + dv;
        let total = self.work + work;
        let absolute = self.absolute_work + work.abs();
        let applied = self.applied_impulse + impulse_n_s;
        if ![vc, total, absolute, applied].iter().all(|v| v.is_finite()) {
            return Err(ModelError("non-finite core impulse"));
        }
        self.vc = vc;
        self.work = total;
        self.absolute_work = absolute;
        self.applied_impulse = applied;
        Ok(())
    }

    pub fn tick(&mut self) -> Result<MemoryHammerProbe, ModelError> {
        self.advance::<true>()
    }

    fn advance<const FAST: bool>(&mut self) -> Result<MemoryHammerProbe, ModelError> {
        self.advance_against::<FAST>(0.0, 0.0)
    }

    /// Internal reciprocal port: surface endpoint = free_position + compliance*N.
    /// The caller must apply the same N to its prepared mechanical response.
    pub(crate) fn advance_against<const FAST: bool>(
        &mut self,
        surface_free: f64,
        compliance: f64,
    ) -> Result<MemoryHammerProbe, ModelError> {
        if !surface_free.is_finite() || !compliance.is_finite() || compliance < 0.0 {
            return Err(ModelError("invalid moving surface response"));
        }
        let ac = self.h * self.h / (2.0 * self.p.core_mass_kg);
        let at = self.h * self.h / (2.0 * self.p.tip_mass_kg);
        let free_core = self.core + self.h * self.vc;
        let free_tip = self.tip + self.h * self.vt;
        let zero_force = self.material.response_at(0.0).0;
        let material_force = |normal: f64| {
            let rhs = free_core - free_tip + at * normal;
            let r0 = (ac + at) * zero_force - rhs;
            let direct = if FAST {
                self.material.same_sign_response_root(ac + at, rhs, r0)
            } else {
                None
            };
            let x = direct.unwrap_or_else(|| {
                root::<FAST>(0.0_f64.min(-r0), 0.0_f64.max(-r0), |x| {
                    let (force, slope) = self.material.response_at(x);
                    (x + (ac + at) * force - rhs, 1.0 + (ac + at) * slope)
                })
            });
            let (force, slope) = self.material.response_at(x);
            (force, at * slope / (1.0 + (ac + at) * slope))
        };
        let open_force = material_force(0.0).0;
        let old_gap = self.tip - self.surface_position;
        let open_tip = free_tip + at * open_force - surface_free;
        let upper = contact_gradient(self.p.surface_stiffness_n_m2, old_gap, open_tip);
        // Keep the material reaction from the last outer residual evaluation.
        // The open branch is already solved even when the normal bracket is empty.
        let mut evaluated_normal = 0.0;
        let mut evaluated_force = open_force;
        let normal = root::<FAST>(0.0, upper, |normal| {
            let (force, derivative) = material_force(normal);
            evaluated_normal = normal;
            evaluated_force = force;
            let gap = free_tip + at * (force - normal) - surface_free - compliance * normal;
            let reaction = contact_gradient(self.p.surface_stiffness_n_m2, old_gap, gap);
            let slope = contact_slope(self.p.surface_stiffness_n_m2, old_gap, gap, reaction);
            (
                normal - reaction,
                1.0 + slope * (at * (1.0 - derivative) + compliance),
            )
        });
        // The bounded bisection fallback may return a new, unevaluated midpoint.
        // Reuse only an exact endpoint match; never a nearby Newton candidate.
        let force = if normal == evaluated_normal {
            evaluated_force
        } else {
            material_force(normal).0
        };
        let core = free_core - ac * force;
        let tip = free_tip + at * (force - normal);
        let vc = self.vc - self.h * force / self.p.core_mass_kg;
        let vt = self.vt + self.h * (force - normal) / self.p.tip_mass_kg;
        let impulse = self.impulse + self.h * normal;
        let surface = surface_free + compliance * normal;
        let surface_work = self.surface_work + normal * (surface - self.surface_position);
        if ![core, tip, vc, vt, normal, impulse, surface, surface_work]
            .iter()
            .all(|v| v.is_finite())
            || normal < 0.0
        {
            return Err(ModelError("non-finite memory impact step"));
        }
        // Fixed-size transactional copy; a rejected material update preserves all state.
        let mut material = self.material.clone();
        material.advance_to(core - tip)?;
        self.core = core;
        self.tip = tip;
        self.vc = vc;
        self.vt = vt;
        self.material = material;
        self.force = normal;
        self.impulse = impulse;
        self.surface_position = surface;
        self.surface_work = surface_work;
        Ok(self.probe())
    }

    pub fn probe(&self) -> MemoryHammerProbe {
        let material = self.material.probe();
        let surface = self.p.surface_stiffness_n_m2
            * (self.tip - self.surface_position).max(0.0).powi(3)
            / 3.0;
        let energy = 0.5
            * (self.p.core_mass_kg * self.vc * self.vc + self.p.tip_mass_kg * self.vt * self.vt)
            + material.stored_energy_j
            + surface;
        MemoryHammerProbe {
            core_position_m: self.core,
            tip_position_m: self.tip,
            core_velocity_m_s: self.vc,
            tip_velocity_m_s: self.vt,
            material,
            contact_force_n: self.force,
            surface_energy_j: surface,
            mechanical_energy_j: energy,
            initial_energy_j: self.initial,
            external_work_j: self.work,
            absolute_impulse_work_j: self.absolute_work,
            surface_impulse_n_s: self.impulse,
            applied_impulse_n_s: self.applied_impulse,
            surface_work_j: self.surface_work,
            balance_residual_j: energy + material.dissipated_energy_j - self.initial - self.work
                + self.surface_work,
        }
    }
}

// Monotone residuals with positive tangent and a proven finite bracket.
fn root<const FAST: bool>(
    mut lo: f64,
    mut hi: f64,
    mut evaluate: impl FnMut(f64) -> (f64, f64),
) -> f64 {
    if lo == hi {
        return lo;
    }
    let mut x = 0.5 * (lo + hi);
    if FAST {
        for _ in 0..12 {
            let (r, slope) = evaluate(x);
            if r == 0.0 {
                return x;
            }
            if r > 0.0 {
                hi = x;
            } else {
                lo = x;
            }
            let next = x - r / slope;
            if next == x {
                return x;
            }
            x = if next > lo && next < hi {
                next
            } else {
                0.5 * (lo + hi)
            };
        }
    }
    for _ in 0..64 {
        x = 0.5 * (lo + hi);
        let r = evaluate(x).0;
        if r == 0.0 || x == lo || x == hi {
            return x;
        }
        if r > 0.0 {
            hi = x;
        } else {
            lo = x;
        }
    }
    0.5 * (lo + hi)
}
fn contact_slope(k: f64, a: f64, b: f64, g: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        k * (a + 2.0 * b) / 3.0
    } else if a <= 0.0 && b <= 0.0 {
        0.0
    } else if a > 0.0 {
        g / (a - b)
    } else {
        let r = b / (b - a);
        k * r * r * (2.0 * b - 3.0 * a) / 3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_maxwell_hammer_converges_to_the_analytic_damped_relative_mode() {
        let p = MemoryHammerProfile {
            material: HammerMemoryProfile {
                equilibrium_cubic_n_m2: 0.0,
                ..HammerMemoryProfile::default()
            },
            ..MemoryHammerProfile::default()
        };
        let total_mass = p.core_mass_kg + p.tip_mass_kg;
        let impulse = 0.001;
        let w0 = impulse / p.core_mass_kg;
        let center_velocity = impulse / total_mass;
        let decay = 0.5 / p.material.relaxation_seconds;
        let frequency = (p.material.memory_stiffness_n_m
            * (1.0 / p.core_mass_kg + 1.0 / p.tip_mass_kg)
            - decay * decay)
            .sqrt();
        let mut errors = [0.0_f64; 2];
        for (error, steps) in errors.iter_mut().zip([1000, 4000]) {
            let h = 0.001 / f64::from(steps);
            let mut v = MemoryHammer::new(h, p, 0.01, 0.0).unwrap();
            v.apply_core_impulse(impulse).unwrap();
            for i in 1..=steps {
                let q = v.tick().unwrap();
                let t = f64::from(i) * h;
                let envelope = w0 * (-decay * t).exp();
                let e = envelope * (frequency * t).sin() / frequency;
                let w =
                    envelope * ((frequency * t).cos() + decay / frequency * (frequency * t).sin());
                let core_velocity = center_velocity + p.tip_mass_kg / total_mass * w;
                let tip_velocity = center_velocity - p.core_mass_kg / total_mass * w;
                *error = error
                    .max((q.core_velocity_m_s - core_velocity).abs() / w0)
                    .max((q.tip_velocity_m_s - tip_velocity).abs() / w0)
                    .max((q.material.branch_extension_m - e).abs() * frequency / w0);
                assert_eq!(q.contact_force_n, 0.0);
                assert!(
                    (p.core_mass_kg * q.core_position_m + p.tip_mass_kg * q.tip_position_m
                        - total_mass * (-0.01 + center_velocity * t))
                        .abs()
                        < 1e-16
                );
            }
        }
        assert!(
            errors[1] < 1e-3 && errors[1] < 0.07 * errors[0],
            "{errors:?}"
        );
    }
    #[test]
    fn invalid_setup_and_excessive_material_travel_are_rejected_atomically() {
        let p = MemoryHammerProfile::default();
        for (h, gap, speed) in [
            (f64::NAN, 0.0, 0.8),
            (0.002, 0.0, 0.8),
            (1e-5, -0.001, 0.8),
            (1e-5, 0.0, 3.1),
        ] {
            assert!(MemoryHammer::new(h, p, gap, speed).is_err());
        }
        for bad in [f64::NAN, 0.0, 0.002] {
            assert!(
                MemoryHammer::new(
                    1e-5,
                    MemoryHammerProfile {
                        tip_mass_kg: bad,
                        ..p
                    },
                    0.0,
                    0.8
                )
                .is_err()
            );
        }
        let soft = MemoryHammerProfile {
            core_mass_kg: 0.001,
            material: HammerMemoryProfile {
                equilibrium_cubic_n_m2: 0.0,
                memory_stiffness_n_m: 1.0,
                ..p.material
            },
            ..p
        };
        let mut v = MemoryHammer::new(0.001, soft, 0.01, 0.0).unwrap();
        v.apply_core_impulse(0.02).unwrap();
        let before = v.probe();
        assert!(v.tick().is_err());
        assert_eq!(before, v.probe());
    }
    #[test]
    fn selected_mass_contact_and_relaxation_corners_remain_passive() {
        for core in [0.001, 0.02] {
            for tip in [1e-5, 0.001] {
                for surface in [1e10, 1e13] {
                    for tau in [1e-6, 1.0] {
                        let p = MemoryHammerProfile {
                            core_mass_kg: core,
                            tip_mass_kg: tip,
                            surface_stiffness_n_m2: surface,
                            material: HammerMemoryProfile {
                                relaxation_seconds: tau,
                                ..HammerMemoryProfile::default()
                            },
                        };
                        let mut v = MemoryHammer::new(1.0 / 176400.0, p, 0.0, 3.0).unwrap();
                        for _ in 0..1200 {
                            let before = v.probe();
                            let q = v.tick().unwrap();
                            assert!(q.contact_force_n >= 0.0);
                            assert!(q.material.last_step_heat_j >= 0.0);
                            assert!(
                                q.mechanical_energy_j
                                    <= before.mechanical_energy_j + q.initial_energy_j * 1e-10
                            );
                            assert!(
                                q.balance_residual_j.abs() < q.initial_energy_j * 1e-8,
                                "{p:?}: {q:?}"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn rigid_flight_has_no_internal_or_surface_force() {
        let p = MemoryHammerProfile::default();
        let mut v = MemoryHammer::new(1e-5, p, 0.01, 0.5).unwrap();
        for i in 1..=100 {
            let q = v.tick().unwrap();
            assert_eq!(q.contact_force_n, 0.0);
            assert_eq!(q.material.stored_energy_j, 0.0);
            assert!((q.core_position_m - (-0.01 + f64::from(i) * 1e-5 * 0.5)).abs() < 1e-15);
            assert_eq!(q.core_velocity_m_s, 0.5);
            assert_eq!(q.tip_velocity_m_s, 0.5);
            assert_eq!(q.balance_residual_j, 0.0);
        }
    }
    #[test]
    fn coupled_contact_agrees_with_nested_bisection_and_conserves_momentum_balance() {
        let p = MemoryHammerProfile::default();
        let mut fast = MemoryHammer::new(1.0 / 176400.0, p, 0.0, 0.8).unwrap();
        let mut reference = MemoryHammer::new(1.0 / 176400.0, p, 0.0, 0.8).unwrap();
        let mut seen = false;
        for _ in 0..1000 {
            let q = fast.tick().unwrap();
            let r = reference.advance::<false>().unwrap();
            seen |= q.contact_force_n > 0.0;
            assert!((q.core_position_m - r.core_position_m).abs() < 1e-12);
            assert!((q.tip_velocity_m_s - r.tip_velocity_m_s).abs() < 1e-9);
            assert!(
                q.balance_residual_j.abs() < q.initial_energy_j * 1e-8,
                "{q:?}"
            );
            let momentum = p.core_mass_kg * q.core_velocity_m_s
                + p.tip_mass_kg * q.tip_velocity_m_s
                + q.surface_impulse_n_s;
            assert!((momentum - (p.core_mass_kg + p.tip_mass_kg) * 0.8).abs() < 1e-12);
        }
        assert!(seen);
        assert!(fast.probe().tip_position_m < 0.0);
        assert!(fast.probe().material.dissipated_energy_j > 0.0);
    }
    #[test]
    fn separation_retains_memory_and_core_impulses_account_for_external_work() {
        let p = MemoryHammerProfile::default();
        let mut v = MemoryHammer::new(1.0 / 176400.0, p, 0.0, 0.8).unwrap();
        let mut contact_runs = 0;
        let mut contacting = false;
        let mut free_recovery_heat = 0.0;
        for i in 0..4000 {
            if i == 1000 {
                let before = v.probe();
                v.apply_core_impulse(0.006).unwrap();
                let after = v.probe();
                assert_eq!(before.material, after.material);
                assert!(
                    (after.mechanical_energy_j
                        - before.mechanical_energy_j
                        - (after.external_work_j - before.external_work_j))
                        .abs()
                        < 1e-15
                );
            }
            let before = v.probe();
            let q = v.tick().unwrap();
            let now = q.contact_force_n > 0.0;
            if now && !contacting {
                contact_runs += 1;
            }
            if !now && !contacting {
                free_recovery_heat +=
                    q.material.dissipated_energy_j - before.material.dissipated_energy_j;
            }
            contacting = now;
            assert!(q.contact_force_n >= 0.0);
            assert!(q.material.dissipated_energy_j >= before.material.dissipated_energy_j);
            assert!(
                q.mechanical_energy_j
                    <= before.mechanical_energy_j
                        + (q.initial_energy_j + q.absolute_impulse_work_j) * 1e-10
            );
            assert!(
                q.balance_residual_j.abs()
                    < (q.initial_energy_j + q.absolute_impulse_work_j) * 1e-8,
                "{q:?}"
            );
        }
        assert!(contact_runs >= 2);
        assert!(free_recovery_heat > 0.0);
        let before = v.probe();
        for impulse in [f64::NAN, f64::INFINITY, 0.021] {
            assert!(v.apply_core_impulse(impulse).is_err());
            assert_eq!(v.probe(), before);
        }
    }
}
