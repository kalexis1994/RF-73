use crate::{PARAMETER_A, PARAMETER_B, PARAMETER_GAIN, PARAMETER_LISTEN_B, PARAMETER_PROFILE};
use rf_73_dsp::{PICKUP_NAMES, PROFILE_NAMES};
use serde::{Deserialize, Serialize};

/// Conservative starting gain for the measured ten-key repeated-strike case.
/// Not a limiter or a guarantee for arbitrary accumulated mechanical energy.
pub const DEFAULT_GAIN: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub gain: f64,
    pub a: u8,
    pub b: u8,
    pub listen_b: bool,
    /// Index into `PROFILE_NAMES`; absent in older program documents.
    #[serde(default)]
    pub profile: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gain: DEFAULT_GAIN,
            a: 0,
            b: 2,
            listen_b: false,
            profile: 0,
        }
    }
}

impl Settings {
    pub fn valid(self) -> bool {
        self.gain.is_finite()
            && (0.0..=2.0).contains(&self.gain)
            && usize::from(self.a) < PICKUP_NAMES.len()
            && usize::from(self.b) < PICKUP_NAMES.len()
            && usize::from(self.profile) < PROFILE_NAMES.len()
    }

    pub fn selected(self) -> usize {
        usize::from(if self.listen_b { self.b } else { self.a })
    }

    pub fn parameter(self, index: u32) -> Option<f64> {
        Some(match index {
            PARAMETER_GAIN => self.gain,
            PARAMETER_A => f64::from(self.a),
            PARAMETER_B => f64::from(self.b),
            PARAMETER_LISTEN_B => f64::from(u8::from(self.listen_b)),
            PARAMETER_PROFILE => f64::from(self.profile),
            _ => return None,
        })
    }

    pub fn with_parameter(mut self, index: u32, value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        match index {
            PARAMETER_GAIN if (0.0..=2.0).contains(&value) => self.gain = value,
            PARAMETER_A | PARAMETER_B
                if value.fract() == 0.0 && (0.0..PICKUP_NAMES.len() as f64).contains(&value) =>
            {
                if index == PARAMETER_A {
                    self.a = value as u8;
                } else {
                    self.b = value as u8;
                }
            }
            PARAMETER_LISTEN_B if [0.0, 1.0].contains(&value) => self.listen_b = value == 1.0,
            PARAMETER_PROFILE
                if value.fract() == 0.0 && (0.0..PROFILE_NAMES.len() as f64).contains(&value) =>
            {
                self.profile = value as u8
            }
            _ => return None,
        }
        Some(self)
    }
}
