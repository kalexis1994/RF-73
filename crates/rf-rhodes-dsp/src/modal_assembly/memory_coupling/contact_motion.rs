//! Error-controlled, persistently compressed contact intervals. No extrapolation.
use super::*;
use crate::HammerMemory;
use crate::modal_assembly::numerics::inverse;

struct Level {
    steps: [Midpoint; 2],
    material: HammerMemory,
}
pub(super) struct ContactBank {
    levels: Vec<Level>,
    inverse_relative_mass: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryContactStatus {
    Advanced,
    BoundaryRequired,
    AccuracyRequired,
}
#[derive(Debug, Clone, Copy)]
pub struct MemoryContactStep {
    pub status: MemoryContactStatus,
    pub normalized_state_error: Option<f64>,
    /// Average of the two accepted half-step reactions, not just the last reaction.
    pub mean_contact_force_n: Option<f64>,
}
/// Read-only trial diagnostics. Metric terms are squared state differences in
/// energy units, not stored energy or heat. A rejected physical check has no metric.
#[derive(Debug, Clone, Copy)]
pub struct MemoryContactInspection {
    pub result: MemoryContactStep,
    /// Structural kinetic, structural elastic, core kinetic, tip kinetic,
    /// memory-coordinate, equilibrium-material and surface-contact terms.
    pub error_metric_terms_j: Option<[f64; 7]>,
    /// Initial energy plus absolute impulse work, with the estimator's floor.
    pub energy_scale_j: Option<f64>,
}
struct Trial {
    motion: Option<Motion>,
    inspection: MemoryContactInspection,
}
impl MemoryContactStep {
    pub const STATE_ERROR_LIMIT: f64 = 1e-11;
}
fn rejected(status: MemoryContactStatus) -> Trial {
    Trial {
        motion: None,
        inspection: MemoryContactInspection {
            result: MemoryContactStep {
                status,
                normalized_state_error: None,
                mean_contact_force_n: None,
            },
            error_metric_terms_j: None,
            energy_scale_j: None,
        },
    }
}
impl MemoryModalAssembly {
    /// Prepare dyadic midpoint and material operators without changing physical state.
    /// Attempts require levels 1..max_level, so both half steps remain at least one base tick.
    pub fn prepare_contact_steps(&mut self, max_level: u32) -> Result<(), ModelError> {
        if !(1..=12).contains(&max_level) || self.h * (1u32 << max_level) as f64 > 0.001 {
            return Err(ModelError("contact bank exceeds its bounded time domain"));
        }
        let inv = inverse(self.op.m)?;
        let port_mass = dot(self.op.hammer, apply(&inv, self.op.hammer));
        if !port_mass.is_finite() || port_mass <= 0.0 {
            return Err(ModelError("invalid contact port mass"));
        }
        let mut levels = Vec::with_capacity(max_level as usize + 1);
        for level in 0..=max_level {
            let h = self.h * (1u32 << level) as f64;
            levels.push(Level {
                steps: [
                    Midpoint::prepare(self.op.m, self.op.k, self.op.c[0], self.op.hammer, h)?,
                    Midpoint::prepare(self.op.m, self.op.k, self.op.c[1], self.op.hammer, h)?,
                ],
                material: self.hammer.prepare_interval(h)?,
            });
        }
        self.contact = Some(ContactBank {
            levels,
            inverse_relative_mass: port_mass + self.hammer.inverse_tip_mass(),
        });
        Ok(())
    }
    /// Reserved prepared contact payload, excluding allocator overhead and inline voice state.
    pub fn contact_operator_bytes(&self) -> usize {
        self.contact
            .as_ref()
            .map_or(0, |b| b.levels.capacity() * core::mem::size_of::<Level>())
    }
    /// Attempt two accepted half steps after a whole-interval compression certificate.
    /// Rejections preserve all state. Callers must not cross external events.
    pub fn try_contact_step(&mut self, level: u32) -> Result<MemoryContactStep, ModelError> {
        let trial = self.contact_trial(level)?;
        if let Some(motion) = trial.motion {
            self.commit_motion(motion);
        }
        Ok(trial.inspection.result)
    }
    /// Inspect the same prepared trial and acceptance checks without committing it.
    /// This diagnostic does the full trial work; it is not a cheap control probe.
    pub fn inspect_contact_step(&self, level: u32) -> Result<MemoryContactInspection, ModelError> {
        Ok(self.contact_trial(level)?.inspection)
    }
    fn contact_trial(&self, level: u32) -> Result<Trial, ModelError> {
        let bank = self
            .contact
            .as_ref()
            .ok_or(ModelError("contact operators have not been prepared"))?;
        if level == 0 || level as usize >= bank.levels.len() {
            return Err(ModelError("contact level was not prepared"));
        }
        let h = self.h * (1u32 << level) as f64;
        let before = self.probe();
        let surface = dot(self.op.hammer, self.q);
        let gap = before.hammer.tip_position_m - surface;
        // Combined passive energy bounds relative velocity, even while energy crosses the port.
        let travel = h * (2.0 * before.mechanical_energy_j * bank.inverse_relative_mass).sqrt();
        let margin = 64.0
            * f64::EPSILON
            * (before.hammer.tip_position_m.abs() + surface.abs() + travel).max(1e-12);
        if !travel.is_finite() || gap - travel - margin <= 0.0 {
            return Ok(rejected(MemoryContactStatus::BoundaryRequired));
        }
        let start = self.motion();
        let scale =
            (before.hammer.initial_energy_j + before.hammer.absolute_impulse_work_j).max(1e-30);
        let trial = |m: &Motion, l: usize| {
            let p = &bank.levels[l];
            advance_motion::<true>(
                m,
                &self.op,
                &p.steps[usize::from(self.damped)],
                self.h * (1usize << l) as f64,
                self.damped,
                Some(&p.material),
            )
        };
        let Ok((coarse, cp)) = trial(&start, level as usize) else {
            return Ok(rejected(MemoryContactStatus::AccuracyRequired));
        };
        let Ok((half, hp)) = trial(&start, level as usize - 1) else {
            return Ok(rejected(MemoryContactStatus::AccuracyRequired));
        };
        let Ok((mut fine, fp)) = trial(&half, level as usize - 1) else {
            return Ok(rejected(MemoryContactStatus::AccuracyRequired));
        };
        for (a, b) in [(before, cp), (before, hp), (hp, fp)] {
            let heat = b.structural_heat_j - a.structural_heat_j
                + b.hammer.material.dissipated_energy_j
                - a.hammer.material.dissipated_energy_j;
            if b.hammer.surface_energy_j <= 0.0
                || b.hammer.contact_force_n <= 0.0
                || b.structural_heat_j < a.structural_heat_j
                || b.hammer.material.dissipated_energy_j < a.hammer.material.dissipated_energy_j
                || b.mechanical_energy_j > a.mechanical_energy_j + scale * 1e-14
                || (b.mechanical_energy_j + heat - a.mechanical_energy_j).abs() > scale * 1e-12
                || (b.structural_work_residual_j - a.structural_work_residual_j).abs()
                    > scale * 1e-12
                || (b.hammer.balance_residual_j - a.hammer.balance_residual_j).abs() > scale * 1e-12
            {
                return Ok(rejected(MemoryContactStatus::AccuracyRequired));
            }
        }
        let terms = phase_error_terms(&self.op, &coarse, &fine, cp.hammer, fp.hammer);
        let error = normalized_error(terms, scale);
        if !error.is_finite() || error > MemoryContactStep::STATE_ERROR_LIMIT {
            let mut trial = rejected(MemoryContactStatus::AccuracyRequired);
            if error.is_finite() {
                trial.inspection.result.normalized_state_error = Some(error);
                trial.inspection.error_metric_terms_j = Some(terms);
                trial.inspection.energy_scale_j = Some(scale);
            }
            return Ok(trial);
        }
        // Restore base preparation only; physical memory and all ledgers stay at the fine endpoint.
        fine.hammer.use_interval(self.h, &bank.levels[0].material);
        Ok(Trial {
            motion: Some(fine),
            inspection: MemoryContactInspection {
                result: MemoryContactStep {
                    status: MemoryContactStatus::Advanced,
                    normalized_state_error: Some(error),
                    mean_contact_force_n: Some(
                        0.5 * (hp.hammer.contact_force_n + fp.hammer.contact_force_n),
                    ),
                },
                error_metric_terms_j: Some(terms),
                energy_scale_j: Some(scale),
            },
        })
    }
}
fn phase_error_terms(
    op: &Operators,
    a: &Motion,
    b: &Motion,
    qa: MemoryHammerProbe,
    qb: MemoryHammerProbe,
) -> [f64; 7] {
    let p = a.hammer.profile();
    let dv = core::array::from_fn(|i| a.v[i] - b.v[i]);
    let dq = core::array::from_fn(|i| a.q[i] - b.q[i]);
    let dc = qa.core_position_m - qb.core_position_m;
    let dt = qa.tip_position_m - qb.tip_position_m;
    let de = qa.material.branch_extension_m - qb.material.branch_extension_m;
    let dx = qa.material.displacement_m - qb.material.displacement_m;
    let gap_a = qa.tip_position_m - dot(op.hammer, a.q);
    let gap_b = qb.tip_position_m - dot(op.hammer, b.q);
    [
        dot(dv, apply(&op.m, dv)),
        dot(dq, apply(&op.k, dq)),
        p.core_mass_kg * (qa.core_velocity_m_s - qb.core_velocity_m_s).powi(2),
        p.tip_mass_kg * (qa.tip_velocity_m_s - qb.tip_velocity_m_s).powi(2),
        p.material.memory_stiffness_n_m * (dc * dc + dt * dt + de * de),
        (p.material.equilibrium_stiffness_n_m
            + 2.0
                * p.material.equilibrium_cubic_n_m2
                * qa.material
                    .displacement_m
                    .abs()
                    .max(qb.material.displacement_m.abs()))
            * dx
            * dx,
        2.0 * p.surface_stiffness_n_m2 * gap_a.max(gap_b).max(0.0) * (gap_a - gap_b).powi(2),
    ]
}
fn normalized_error(terms: [f64; 7], scale: f64) -> f64 {
    // Preserve the original left-associated metric arithmetic exactly.
    let metric = terms[0] + terms[1] + terms[2] + terms[3] + terms[4] + terms[5] + terms[6];
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
    fn read_only_trials_preserve_state_and_match_commit_decisions() {
        let mut inspected = voice();
        let mut control = voice();
        assert!(inspected.inspect_contact_step(1).is_err());
        inspected.prepare_contact_steps(8).unwrap();
        control.prepare_contact_steps(8).unwrap();
        let initial = inspected.probe();
        let boundary = inspected.inspect_contact_step(8).unwrap();
        assert_eq!(
            boundary.result.status,
            MemoryContactStatus::BoundaryRequired
        );
        assert!(boundary.error_metric_terms_j.is_none());
        assert!(boundary.energy_scale_j.is_none());
        assert!(inspected.inspect_contact_step(0).is_err());
        assert_eq!(initial, inspected.probe());
        for _ in 0..2000 {
            assert_eq!(inspected.tick().unwrap(), control.tick().unwrap());
        }
        for damped in [false, true] {
            inspected.set_damped(damped);
            control.set_damped(damped);
            let before = inspected.probe();
            let mut accepted = false;
            let mut rejected = false;
            for level in 1..=8 {
                let trial = inspected.inspect_contact_step(level).unwrap();
                assert_eq!(before, inspected.probe());
                if let Some(terms) = trial.error_metric_terms_j {
                    assert!(terms.iter().all(|x| x.is_finite() && *x >= 0.0));
                    assert_eq!(
                        Some(normalized_error(terms, trial.energy_scale_j.unwrap())),
                        trial.result.normalized_state_error
                    );
                }
                accepted |= trial.result.status == MemoryContactStatus::Advanced;
                rejected |= trial.result.status == MemoryContactStatus::AccuracyRequired;
            }
            assert!(accepted && rejected);
            let trial = inspected.inspect_contact_step(2).unwrap().result;
            let committed = inspected.try_contact_step(2).unwrap();
            assert_eq!(trial.status, committed.status);
            assert_eq!(
                trial.normalized_state_error,
                committed.normalized_state_error
            );
            assert_eq!(trial.mean_contact_force_n, committed.mean_contact_force_n);
            control.try_contact_step(2).unwrap();
            assert_eq!(inspected.probe(), control.probe());
            // Inspection/acceptance must leave the base-step preparation usable.
            assert_eq!(inspected.tick().unwrap(), control.tick().unwrap());
        }
    }
    #[test]
    fn preparation_and_boundary_rejections_preserve_fixed_ticks_and_all_state() {
        let mut v = voice();
        let mut reference = voice();
        let before = v.probe();
        assert!(v.try_contact_step(1).is_err());
        v.prepare_contact_steps(8).unwrap();
        assert_eq!(before, v.probe());
        assert!(v.prepare_contact_steps(0).is_err());
        assert!(v.prepare_contact_steps(13).is_err());
        assert!(v.try_contact_step(0).is_err());
        assert!(v.try_contact_step(u32::MAX).is_err());
        assert_eq!(
            v.try_contact_step(8).unwrap().status,
            MemoryContactStatus::BoundaryRequired
        );
        assert_eq!(before, v.probe());
        for _ in 0..2000 {
            assert_eq!(v.tick().unwrap(), reference.tick().unwrap());
        }
    }
    #[test]
    fn accuracy_rejection_is_atomic_and_accepted_contact_matches_two_half_ticks() {
        let mut v = voice();
        v.prepare_contact_steps(8).unwrap();
        for _ in 0..2000 {
            v.tick().unwrap();
        }
        let before = v.probe();
        assert_eq!(
            v.try_contact_step(7).unwrap().status,
            MemoryContactStatus::AccuracyRequired
        );
        assert_eq!(before, v.probe());
        let mut reference = MemoryModalAssembly::new(
            2e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        let preparation = reference.hammer.prepare_interval(2e-9).unwrap();
        reference.commit_motion(v.motion());
        reference.hammer.use_interval(2e-9, &preparation);
        let first = reference.tick().unwrap();
        let second = reference.tick().unwrap();
        let accepted = v.try_contact_step(2).unwrap();
        assert_eq!(accepted.status, MemoryContactStatus::Advanced);
        assert_eq!(v.probe(), second);
        assert_eq!(
            accepted.mean_contact_force_n,
            Some(0.5 * (first.hammer.contact_force_n + second.hammer.contact_force_n))
        );
        // After acceptance, the ordinary tick must use the original 1 ns preparation.
        let base = reference.hammer.prepare_interval(1e-9).unwrap();
        reference.h = 1e-9;
        reference.steps = [
            Midpoint::prepare(
                reference.op.m,
                reference.op.k,
                reference.op.c[0],
                reference.op.hammer,
                1e-9,
            )
            .unwrap(),
            Midpoint::prepare(
                reference.op.m,
                reference.op.k,
                reference.op.c[1],
                reference.op.hammer,
                1e-9,
            )
            .unwrap(),
        ];
        reference.hammer.use_interval(1e-9, &base);
        assert_eq!(v.tick().unwrap(), reference.tick().unwrap());
    }
}
