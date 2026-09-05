use rf_rhodes_dsp::{
    MemoryContactStatus, MemoryContactStep, MemoryFreeStatus, MemoryModalAssembly, ModelError,
};
use serde_json::{Value, json};

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
}
impl Controller {
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
        if self.contact_enabled && remaining >= 2 && voice.probe().hammer.surface_energy_j > 0.0 {
            self.contact_level = self
                .contact_level
                .min(usize::BITS - 1 - remaining.leading_zeros())
                .max(1);
            loop {
                let ticks = 1usize << self.contact_level;
                let attempt = voice.try_contact_step(self.contact_level)?;
                match attempt.status {
                    MemoryContactStatus::Advanced => {
                        self.contacts += 1;
                        self.contact_ticks += ticks;
                        self.largest_contact = self.largest_contact.max(ticks);
                        self.mean_force = attempt.mean_contact_force_n;
                        if attempt.normalized_state_error.unwrap_or(1.0)
                            < MemoryContactStep::STATE_ERROR_LIMIT / 8.0
                        {
                            self.contact_level = (self.contact_level + 1).min(12);
                        }
                        return Ok(ticks);
                    }
                    MemoryContactStatus::BoundaryRequired => self.contact_boundary += 1,
                    MemoryContactStatus::AccuracyRequired => self.contact_accuracy += 1,
                }
                if self.contact_level == 1 {
                    break;
                }
                self.contact_level -= 1;
            }
        }
        if voice.probe().hammer.surface_energy_j == 0.0 {
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
        }
        report
    }
}
