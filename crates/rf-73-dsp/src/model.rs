use core::fmt;

pub const SAMPLE_RATE_MIN: f64 = 44_100.0;
pub const SAMPLE_RATE_MAX: f64 = 192_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelError(pub &'static str);

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ModelError {}

/// SI-valued research constants. Defaults are design assumptions, NOT measured
/// Rhodes dimensions. A profile is immutable for the lifetime of a prepared engine.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub hammer_mass_kg: f64,
    pub modal_mass_kg: f64,
    /// F = stiffness * compression^2; units N/m^2.
    pub contact_stiffness: f64,
    pub maximum_hammer_speed_m_s: f64,
    pub pickup_gap_m: f64,
    pub pickup_offset_m: f64,
    pub decay_seconds: f64,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            hammer_mass_kg: 0.004,
            modal_mass_kg: 0.0015,
            contact_stiffness: 4.0e10,
            maximum_hammer_speed_m_s: 0.8,
            pickup_gap_m: 0.0015,
            pickup_offset_m: 0.0005,
            decay_seconds: 5.0,
        }
    }
}

impl Profile {
    pub fn validate(self, sample_rate: f64) -> Result<(), ModelError> {
        if !sample_rate.is_finite() || !(SAMPLE_RATE_MIN..=SAMPLE_RATE_MAX).contains(&sample_rate) {
            return Err(ModelError(
                "sample rate must be finite and between 44100 and 192000 Hz",
            ));
        }
        for (value, minimum, maximum, error) in [
            (
                self.hammer_mass_kg,
                0.001,
                0.02,
                "hammer mass outside 0.001..0.02 kg",
            ),
            (
                self.modal_mass_kg,
                0.0001,
                0.01,
                "modal mass outside 0.0001..0.01 kg",
            ),
            (
                self.contact_stiffness,
                1.0e8,
                1.0e12,
                "contact stiffness outside 1e8..1e12 N/m^2",
            ),
            (
                self.maximum_hammer_speed_m_s,
                0.1,
                3.0,
                "hammer speed outside 0.1..3 m/s",
            ),
            (
                self.pickup_gap_m,
                0.0005,
                0.005,
                "pickup gap outside 0.0005..0.005 m",
            ),
            (
                self.pickup_offset_m,
                -0.003,
                0.003,
                "pickup offset outside -0.003..0.003 m",
            ),
            (
                self.decay_seconds,
                0.25,
                20.0,
                "decay outside 0.25..20 seconds",
            ),
        ] {
            if !value.is_finite() || !(minimum..=maximum).contains(&value) {
                return Err(ModelError(error));
            }
        }
        Ok(())
    }
}
