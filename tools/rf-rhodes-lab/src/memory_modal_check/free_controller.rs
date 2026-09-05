use rf_rhodes_dsp::{MemoryFreeStatus, MemoryModalAssembly, ModelError};
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
}
impl Controller {
    // One accepted interval, bounded by the next observation/event boundary.
    pub(crate) fn advance(
        &mut self,
        voice: &mut MemoryModalAssembly,
        remaining: usize,
    ) -> Result<usize, ModelError> {
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
        json!({"fixed_ticks":self.fixed,"accepted_free_intervals":self.free,
            "uniform_ticks_replaced":self.replaced,"accuracy_rejections":self.accuracy,
            "uncertified_clearance_rejections":self.clearance,
            "maximum_free_interval_seconds":self.largest as f64*h,
            "uniform_to_accepted_interval_ratio":(self.fixed+self.replaced) as f64/(self.fixed+self.free) as f64})
    }
}
