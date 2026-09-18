//! Prepared structural propagation and a whole-interval moving-port certificate.
use super::*;
use crate::modal_assembly::numerics::{Free, inverse};
use crate::{MemoryFreeStatus, MemoryFreeStep};

pub(super) struct FreeBank {
    levels: Vec<[Free; 2]>,
    inverse_port_mass: f64,
    inverse_tip_mass: f64,
}
impl MemoryModalAssembly {
    /// Reserved free-operator heap payload; excludes allocator overhead and inline voice state.
    pub fn free_operator_bytes(&self) -> usize {
        self.free.as_ref().map_or(0, |b| {
            b.levels.capacity() * core::mem::size_of::<[Free; 2]>()
        })
    }
    /// Allocate/prepare dyadic free operators without changing the physical state.
    /// Level L advances 2^L original ticks; max_level must be 0..12 and h_L <=1 ms.
    pub fn prepare_free_steps(&mut self, max_level: u32) -> Result<(), ModelError> {
        if max_level > 12 || self.h * (1u32 << max_level) as f64 > 0.001 {
            return Err(ModelError(
                "free bank level exceeds its bounded time domain",
            ));
        }
        let inv = inverse(self.op.m)?;
        let inverse_port_mass = dot(self.op.hammer, apply(&inv, self.op.hammer));
        if !inverse_port_mass.is_finite() || inverse_port_mass <= 0.0 {
            return Err(ModelError("invalid moving-port inverse mass"));
        }
        let mut levels = Vec::with_capacity(max_level as usize + 1);
        for level in 0..=max_level {
            let h = self.h * (1u32 << level) as f64;
            levels.push([
                Free::prepare(self.op.m, self.op.k, self.op.c[0], h)?,
                Free::prepare(self.op.m, self.op.k, self.op.c[1], h)?,
            ]);
        }
        self.free = Some(FreeBank {
            levels,
            inverse_port_mass,
            inverse_tip_mass: self.hammer.inverse_tip_mass(),
        });
        Ok(())
    }
    /// Attempt one prepared free interval. Every rejection preserves the whole assembly.
    /// This does not cross caller events; callers select levels that fit their event grid.
    pub fn try_free_step(&mut self, level: u32) -> Result<MemoryFreeStep, ModelError> {
        let bank = self
            .free
            .as_ref()
            .ok_or(ModelError("free operators have not been prepared"))?;
        let operators = bank
            .levels
            .get(level as usize)
            .ok_or(ModelError("free level was not prepared"))?;
        let h = self.h * (1u32 << level) as f64;
        let before = self.probe();
        let surface = dot(self.op.hammer, self.q);
        let gap = before.hammer.tip_position_m - surface;
        // Until first contact, each disconnected passive subsystem bounds its own
        // speed. Cauchy-Schwarz in the full structural mass metric gives the port
        // bound. Their sum excludes an interior meeting, not just endpoint overlap.
        let speed = (2.0 * before.hammer.mechanical_energy_j * bank.inverse_tip_mass).sqrt()
            + (2.0 * before.structural_energy_j * bank.inverse_port_mass).sqrt();
        let margin = 64.0
            * f64::EPSILON
            * (before.hammer.tip_position_m.abs() + surface.abs() + h * speed).max(1e-12);
        if !speed.is_finite() || gap + h * speed + margin >= 0.0 {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::ContactRequired,
                normalized_state_error: None,
                relative_energy_defect: None,
            });
        }
        let (q, v, loss) = operators[usize::from(self.damped)].advance(self.q, self.v);
        let structure = mechanical(&self.op, q, v);
        let scale =
            (before.hammer.initial_energy_j + before.hammer.absolute_impulse_work_j).max(1e-30);
        let defect = (structure + loss - self.structural_energy).abs() / scale;
        if !q.iter().chain(v.iter()).all(|x| x.is_finite())
            || !loss.is_finite()
            || loss < 0.0
            || !defect.is_finite()
            || defect > 1e-12
            || structure > self.structural_energy + scale * 1e-14
            || !(self.heat + loss).is_finite()
        {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: None,
                relative_energy_defect: defect.is_finite().then_some(defect),
            });
        }
        let mut hammer = self.hammer.clone();
        let mut attempt = hammer.advance_certified_free(h, dot(self.op.hammer, q))?;
        if attempt.status != MemoryFreeStatus::Advanced {
            return Ok(attempt);
        }
        let hp = hammer.probe();
        let global_defect =
            (structure + hp.mechanical_energy_j + loss + hp.material.last_step_heat_j
                - before.mechanical_energy_j)
                .abs()
                / scale;
        if !global_defect.is_finite() || global_defect > 2e-12 {
            return Ok(MemoryFreeStep {
                status: MemoryFreeStatus::AccuracyRequired,
                normalized_state_error: attempt.normalized_state_error,
                relative_energy_defect: global_defect.is_finite().then_some(global_defect),
            });
        }
        self.q = q;
        self.v = v;
        self.heat += loss;
        self.structural_energy = structure;
        self.hammer = hammer;
        attempt.relative_energy_defect = Some(
            attempt
                .relative_energy_defect
                .unwrap_or(0.0)
                .max(defect)
                .max(global_defect),
        );
        Ok(attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn voice() -> MemoryModalAssembly {
        MemoryModalAssembly::new(
            1e-8,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap()
    }
    #[test]
    fn preparation_and_rejected_free_steps_preserve_all_physical_state() {
        let mut v = voice();
        let before = v.probe();
        assert!(v.try_free_step(0).is_err());
        v.prepare_free_steps(6).unwrap();
        assert_eq!(before, v.probe());
        assert_eq!(
            v.try_free_step(6).unwrap().status,
            MemoryFreeStatus::ContactRequired
        );
        assert_eq!(before, v.probe());
        assert!(v.prepare_free_steps(13).is_err());
        assert!(v.try_free_step(7).is_err());
        assert_eq!(before, v.probe());
    }
    #[test]
    fn moving_surface_speed_bound_rejects_unsafe_intervals_even_with_a_stationary_tip() {
        let mut v = MemoryModalAssembly::new(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            1e-6,
            0.0,
        )
        .unwrap();
        v.v[0] = -1.0;
        v.structural_energy = mechanical(&v.op, v.q, v.v);
        v.prepare_free_steps(3).unwrap();
        let before = v.probe();
        assert_eq!(
            v.try_free_step(0).unwrap().status,
            MemoryFreeStatus::ContactRequired
        );
        assert_eq!(before, v.probe());
    }
    #[test]
    fn material_accuracy_rejection_does_not_commit_prepared_structural_motion() {
        let mut v = MemoryModalAssembly::new(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.01,
            0.0,
        )
        .unwrap();
        v.v[0] = 0.01;
        v.structural_energy = mechanical(&v.op, v.q, v.v);
        v.apply_core_impulse(0.004).unwrap();
        v.prepare_free_steps(9).unwrap();
        let before = v.probe();
        assert_eq!(
            v.try_free_step(9).unwrap().status,
            MemoryFreeStatus::AccuracyRequired
        );
        assert_eq!(before, v.probe());
    }
    #[test]
    fn mixed_contact_and_free_intervals_match_uniform_motion_and_preserve_port_work() {
        let mut fast = voice();
        let mut reference = voice();
        fast.prepare_free_steps(3).unwrap();
        let mut accepted = 0;
        for frame in 0..10000 {
            if frame == 2500 {
                fast.apply_core_impulse(0.008).unwrap();
                reference.apply_core_impulse(0.008).unwrap();
            }
            if frame == 5000 {
                fast.set_damped(true);
                reference.set_damped(true);
            }
            if frame == 7500 {
                fast.set_damped(false);
                reference.set_damped(false);
            }
            if fast.try_free_step(3).unwrap().status == MemoryFreeStatus::Advanced {
                accepted += 1;
            } else {
                for _ in 0..8 {
                    fast.tick().unwrap();
                }
            }
            for _ in 0..8 {
                reference.tick().unwrap();
            }
            let a = fast.probe();
            let b = reference.probe();
            let scale = a.hammer.initial_energy_j + a.hammer.absolute_impulse_work_j;
            assert!(a.balance_residual_j.abs() < scale * 1e-8, "{a:?}");
            assert!(a.structural_work_residual_j.abs() < scale * 1e-8);
            assert!((a.pickup_velocity_m_s - b.pickup_velocity_m_s).abs() < 1e-3);
        }
        assert!(accepted > 0);
    }
}
