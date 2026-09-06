use rf_rhodes_dsp::{
    MemoryContactStatus, MemoryContactStep, MemoryFreeStatus, MemoryModalAssembly, ModelError,
};
use serde_json::{Value, json};
const ECONOMICAL_MINIMUM_LEVEL: u32 = 3;
const CONTACT_RETRY_TICKS: usize = 64;

#[derive(Default)]
pub(crate) struct Controller {
    level: u32,
    fixed: usize,
    free: usize,
    replaced: usize,
    accuracy: usize,
    clearance: usize,
    largest: usize,
    contact_enabled: bool,
    contact_level: u32,
    contacts: usize,
    contact_ticks: usize,
    contact_accuracy: usize,
    contact_boundary: usize,
    largest_contact: usize,
    mean_force: Option<f64>,
    economical: bool,
    retry_ticks: usize,
    deferred_ticks: usize,
    rk4: bool,
    maximum_contact_error: f64,
    maximum_contact_defect: f64,
}
impl Controller {
    pub(crate) fn with_rk4_contact() -> Self {
        Self {
            rk4: true,
            contact_enabled: true,
            ..Self::default()
        }
    }
    pub(crate) fn with_economical_contact() -> Self {
        Self {
            economical: true,
            contact_level: ECONOMICAL_MINIMUM_LEVEL,
            ..Self::with_contact()
        }
    }
    pub(crate) fn with_contact() -> Self {
        Self {
            contact_enabled: true,
            contact_level: 1,
            ..Self::default()
        }
    }
    pub(crate) fn mean_force(&self) -> Option<f64> {
        self.mean_force
    }
    // One accepted interval, bounded by the next observation/event boundary.
    pub(crate) fn advance(
        &mut self,
        voice: &mut MemoryModalAssembly,
        remaining: usize,
    ) -> Result<usize, ModelError> {
        self.mean_force = None;
        let surface_energy = voice.probe().hammer.surface_energy_j;
        let contacting = surface_energy > 0.0;
        let minimum_level = if self.rk4 {
            0
        } else if self.economical {
            ECONOMICAL_MINIMUM_LEVEL
        } else {
            1
        };
        if self.rk4 && !contacting {
            self.contact_level = 0;
        }
        if self.economical && !contacting {
            self.retry_ticks = 0;
            self.contact_level = minimum_level;
        }
        if self.economical && contacting && self.retry_ticks > 0 {
            voice.tick()?;
            self.retry_ticks -= 1;
            self.deferred_ticks += 1;
            self.fixed += 1;
            self.level = 0;
            return Ok(1);
        }
        if self.contact_enabled && remaining >= 1usize << minimum_level && contacting {
            self.contact_level = self
                .contact_level
                .min(usize::BITS - 1 - remaining.leading_zeros())
                .max(minimum_level);
            loop {
                let ticks = 1usize << self.contact_level;
                let (attempt, energy_growth) = if self.rk4 {
                    let r = voice.try_rk4_contact_step(self.contact_level)?;
                    if r.contact.status == MemoryContactStatus::Advanced {
                        self.maximum_contact_error = self
                            .maximum_contact_error
                            .max(r.contact.normalized_state_error.unwrap_or(0.0));
                        self.maximum_contact_defect = self
                            .maximum_contact_defect
                            .max(r.relative_energy_defect.unwrap_or(0.0));
                    }
                    (
                        r.contact,
                        r.relative_energy_defect.is_some_and(|d| d < 1e-13 / 64.0),
                    )
                } else {
                    (voice.try_contact_step(self.contact_level)?, true)
                };
                match attempt.status {
                    MemoryContactStatus::Advanced => {
                        self.contacts += 1;
                        self.contact_ticks += ticks;
                        self.largest_contact = self.largest_contact.max(ticks);
                        self.mean_force = attempt.mean_contact_force_n;
                        if attempt.normalized_state_error.unwrap_or(1.0)
                            < MemoryContactStep::STATE_ERROR_LIMIT
                                / if self.rk4 { 64.0 } else { 8.0 }
                            && energy_growth
                        {
                            self.contact_level = (self.contact_level + 1).min(12);
                        }
                        return Ok(ticks);
                    }
                    MemoryContactStatus::BoundaryRequired => self.contact_boundary += 1,
                    MemoryContactStatus::AccuracyRequired => self.contact_accuracy += 1,
                }
                if self.contact_level == minimum_level {
                    if self.economical {
                        self.retry_ticks = CONTACT_RETRY_TICKS;
                    }
                    break;
                }
                self.contact_level -= 1;
            }
        }
        if surface_energy == 0.0 {
            self.level = self.level.min(usize::BITS - 1 - remaining.leading_zeros());
            loop {
                let ticks = 1usize << self.level;
                let attempt = voice.try_free_step(self.level)?;
                match attempt.status {
                    MemoryFreeStatus::Advanced => {
                        self.free += 1;
                        self.replaced += ticks;
                        self.largest = self.largest.max(ticks);
                        if attempt.normalized_state_error.unwrap_or(1.0) < 1e-10 / 64.0
                            && attempt.relative_energy_defect.unwrap_or(1.0) < 1e-13 / 64.0
                        {
                            self.level = (self.level + 1).min(12);
                        }
                        return Ok(ticks);
                    }
                    MemoryFreeStatus::ContactRequired => self.clearance += 1,
                    MemoryFreeStatus::AccuracyRequired => self.accuracy += 1,
                }
                if self.level == 0 {
                    break;
                }
                self.level -= 1;
            }
        }
        voice.tick()?;
        self.fixed += 1;
        self.level = 0;
        Ok(1)
    }
    pub(crate) fn report(&self, h: f64) -> Value {
        let mut report = json!({"fixed_ticks":self.fixed,"accepted_free_intervals":self.free,
            "uniform_ticks_replaced":self.replaced,"accuracy_rejections":self.accuracy,
            "uncertified_clearance_rejections":self.clearance,
            "maximum_free_interval_seconds":self.largest as f64*h,
            "uniform_to_accepted_interval_ratio":(self.fixed+self.replaced+self.contact_ticks) as f64/(self.fixed+self.free+self.contacts) as f64});
        if self.contact_enabled {
            report["contact"] = json!({"state_error_limit":MemoryContactStep::STATE_ERROR_LIMIT,"accepted_intervals":self.contacts,"uniform_ticks_replaced":self.contact_ticks,
                "accuracy_rejections":self.contact_accuracy,"boundary_rejections":self.contact_boundary,
                "maximum_interval_seconds":self.largest_contact as f64*h,
                "accepted_implicit_half_steps":2*self.contacts});
            if self.economical {
                report["contact"]["minimum_trial_ticks"] =
                    json!(1usize << ECONOMICAL_MINIMUM_LEVEL);
                report["contact"]["retry_delay_ticks"] = json!(CONTACT_RETRY_TICKS);
                report["contact"]["deferred_fixed_ticks"] = json!(self.deferred_ticks);
            }
            if self.rk4 {
                report["contact"]
                    .as_object_mut()
                    .unwrap()
                    .remove("accepted_implicit_half_steps");
                report["contact"]["integrator"] = json!("coupled-rk4-step-doubling");
                report["contact"]["accepted_rk4_half_steps"] = json!(2 * self.contacts);
                report["contact"]["maximum_accepted_state_error"] =
                    json!(self.maximum_contact_error);
                report["contact"]["maximum_accepted_energy_defect"] =
                    json!(self.maximum_contact_defect);
                report["contact"]["local_energy_defect_limit"] = json!(1e-13);
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rf_rhodes_dsp::{MemoryHammerProfile, ModalAssemblyProfile, TineGeometry};
    fn compressed_voice() -> MemoryModalAssembly {
        let mut v = MemoryModalAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            MemoryHammerProfile::default(),
            0.0,
            0.8,
        )
        .unwrap();
        v.prepare_free_steps(12).unwrap();
        v.prepare_contact_steps(12).unwrap();
        for _ in 0..2000 {
            v.tick().unwrap();
        }
        assert!(v.probe().hammer.surface_energy_j > 0.0);
        v
    }
    #[test]
    fn short_frame_remainders_use_original_ticks_without_contact_trials() {
        let mut v = compressed_voice();
        let mut reference = compressed_voice();
        let mut c = Controller::with_economical_contact();
        for remaining in (1..8).rev() {
            assert_eq!(c.advance(&mut v, remaining).unwrap(), 1);
            assert_eq!(v.probe(), reference.tick().unwrap());
        }
        assert_eq!(c.contacts, 0);
        assert_eq!(c.contact_accuracy, 0);
        assert_eq!(c.contact_boundary, 0);
        assert_eq!(c.fixed, 7);
    }
    #[test]
    fn deferred_retries_still_integrate_impulses_damping_and_every_tick() {
        let mut v = compressed_voice();
        let mut reference = compressed_voice();
        let mut c = Controller::with_economical_contact();
        c.retry_ticks = 3;
        v.apply_core_impulse(0.001).unwrap();
        reference.apply_core_impulse(0.001).unwrap();
        v.set_damped(true);
        reference.set_damped(true);
        for remaining in [10, 2, 1] {
            assert_eq!(c.advance(&mut v, remaining).unwrap(), 1);
            assert_eq!(v.probe(), reference.tick().unwrap());
        }
        assert_eq!(c.retry_ticks, 0);
        assert_eq!(c.deferred_ticks, 3);
        assert_eq!(c.fixed, 3);
        assert!(c.mean_force().is_none());
    }
}
