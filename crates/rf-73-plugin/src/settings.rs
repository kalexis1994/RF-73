use crate::{
    PARAMETER_ALIGNMENT, PARAMETER_BELL, PARAMETER_DISTANCE, PARAMETER_DYNAMICS, PARAMETER_GAIN,
    PARAMETER_HARDNESS, PARAMETER_LAW, PARAMETER_SUSTAIN,
};
use rf_73_dsp::{PickupLaw, Profile};
use serde::{Deserialize, Serialize};

/// Conservative starting gain for the measured ten-key repeated-strike case.
/// Not a limiter or a guarantee for arbitrary accumulated mechanical energy.
pub const DEFAULT_GAIN: f64 = 0.1;
/// Parameter order: gain, law, distance, alignment, hardness, sustain, bell, dynamics.
pub const PARAMETERS: usize = 8;
pub const LAW_NAMES: [&str; 3] = ["Production", "Aperture", "Register Aperture"];

/// The Sound page in physical and normalized terms. Every field maps to a
/// `Profile` through `profile`; defaults are the retained 0.1.2 engine.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub gain: f64,
    /// Index into `LAW_NAMES`.
    pub law: u8,
    /// Pickup distance, the gap, 0.5..=3 mm.
    pub distance_mm: f64,
    /// Tine alignment, the lateral offset, -1..=1.5 mm.
    pub alignment_mm: f64,
    /// Hammer hardness 0..=1: contact stiffness 4e10 * 25^(2h - 1) N/m^2.
    pub hardness: f64,
    /// Sustain 0..=1: first partial T60 at A3 of 5 * 16^s seconds, the bar
    /// partial 0.16 * 14.375^(2s) capped at 10 s.
    pub sustain: f64,
    /// Bell 0..=1: second partial strike weight -0.3 * b^2.
    pub bell: f64,
    /// Dynamics 0..=1: velocity exponent 1.4 * 2^(2d - 1).
    pub dynamics: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gain: DEFAULT_GAIN,
            law: 0,
            distance_mm: 1.5,
            alignment_mm: 0.5,
            hardness: 0.5,
            sustain: 0.0,
            bell: 1.0,
            dynamics: 0.5,
        }
    }
}

/// Factory presets: id, name, description, settings.
pub fn presets() -> [(&'static str, &'static str, &'static str, Settings); 5] {
    let default = Settings::default();
    [
        (
            "research-direct",
            "Original",
            "The 0.1.2 engine: production pickup at 1.5 / 0.5 mm, original losses. No limiter.",
            default,
        ),
        (
            "close-original",
            "Close Original",
            "Production pickup at 0.5 / 0.25 mm; level compensated. Original losses.",
            Settings {
                distance_mm: 0.5,
                alignment_mm: 0.25,
                ..default
            },
        ),
        (
            "close-aperture",
            "Close Aperture",
            "Finite-aperture pickup at 0.5 / 0.5 mm with a 2 mm pole; original losses.",
            Settings {
                law: 1,
                distance_mm: 0.5,
                alignment_mm: 0.5,
                ..default
            },
        ),
        (
            "calibrated",
            "Calibrated",
            "Aperture pickup at 0.5 / 0.5 mm, recording-derived sustain, soft second partial.",
            Settings {
                law: 1,
                distance_mm: 0.5,
                alignment_mm: 0.5,
                sustain: 0.5,
                bell: 0.2582,
                ..default
            },
        ),
        (
            "calibrated-register",
            "Calibrated Register",
            "Calibrated with the validated upper-register pickup geometry; normal MIDI dynamics.",
            Settings {
                law: 2,
                distance_mm: 0.5,
                alignment_mm: 0.5,
                sustain: 0.5,
                bell: 0.2582,
                ..default
            },
        ),
    ]
}

fn unit(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl Settings {
    pub fn valid(self) -> bool {
        self.gain.is_finite()
            && (0.0..=2.0).contains(&self.gain)
            && usize::from(self.law) < LAW_NAMES.len()
            && self.distance_mm.is_finite()
            && (0.5..=3.0).contains(&self.distance_mm)
            && self.alignment_mm.is_finite()
            && (-1.0..=1.5).contains(&self.alignment_mm)
            && unit(self.hardness)
            && unit(self.sustain)
            && unit(self.bell)
            && unit(self.dynamics)
    }

    /// The mechanical and pickup profile these settings describe.
    pub fn profile(self) -> Profile {
        Profile {
            pickup_law: match self.law {
                1 => PickupLaw::Aperture,
                2 => PickupLaw::RegisterAperture,
                _ => PickupLaw::Production,
            },
            pickup_gap_m: self.distance_mm * 1e-3,
            pickup_offset_m: self.alignment_mm * 1e-3,
            contact_stiffness: 4.0e10 * 25.0_f64.powf(2.0 * self.hardness - 1.0),
            decay_seconds: 5.0 * 16.0_f64.powf(self.sustain),
            bar_partial_decay_seconds: (0.16 * 14.375_f64.powf(2.0 * self.sustain)).min(10.0),
            bar_partial_strike_weight: -0.3 * self.bell * self.bell,
            velocity_exponent: 1.4 * 2.0_f64.powf(2.0 * self.dynamics - 1.0),
            ..Profile::default()
        }
    }

    pub fn parameter(self, index: u32) -> Option<f64> {
        Some(match index {
            PARAMETER_GAIN => self.gain,
            PARAMETER_LAW => f64::from(self.law),
            PARAMETER_DISTANCE => self.distance_mm,
            PARAMETER_ALIGNMENT => self.alignment_mm,
            PARAMETER_HARDNESS => self.hardness,
            PARAMETER_SUSTAIN => self.sustain,
            PARAMETER_BELL => self.bell,
            PARAMETER_DYNAMICS => self.dynamics,
            _ => return None,
        })
    }

    pub fn with_parameter(mut self, index: u32, value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        match index {
            PARAMETER_GAIN => self.gain = value,
            PARAMETER_LAW
                if value.fract() == 0.0 && (0.0..LAW_NAMES.len() as f64).contains(&value) =>
            {
                self.law = value as u8
            }
            PARAMETER_DISTANCE => self.distance_mm = value,
            PARAMETER_ALIGNMENT => self.alignment_mm = value,
            PARAMETER_HARDNESS => self.hardness = value,
            PARAMETER_SUSTAIN => self.sustain = value,
            PARAMETER_BELL => self.bell = value,
            PARAMETER_DYNAMICS => self.dynamics = value,
            _ => return None,
        }
        self.valid().then_some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_map_to_the_retained_profile_and_presets_validate() {
        let default = Settings::default().profile();
        let retained = Profile::default();
        assert_eq!(default.pickup_law, retained.pickup_law);
        assert_eq!(default.pickup_gap_m, retained.pickup_gap_m);
        assert_eq!(default.pickup_offset_m, retained.pickup_offset_m);
        assert_eq!(default.contact_stiffness, retained.contact_stiffness);
        assert_eq!(default.decay_seconds, retained.decay_seconds);
        assert_eq!(
            default.bar_partial_decay_seconds,
            retained.bar_partial_decay_seconds
        );
        assert_eq!(
            default.bar_partial_strike_weight,
            retained.bar_partial_strike_weight
        );
        assert_eq!(default.velocity_exponent, retained.velocity_exponent);
        for (id, _, _, settings) in presets() {
            assert!(settings.valid(), "{id}");
            settings.profile().validate(48_000.0).unwrap();
        }
        let calibrated = presets()[3].3.profile();
        let reference = Profile::calibrated();
        assert!((calibrated.decay_seconds - reference.decay_seconds).abs() < 1e-9);
        assert!(
            (calibrated.bar_partial_decay_seconds - reference.bar_partial_decay_seconds).abs()
                < 1e-9
        );
        assert!(
            (calibrated.bar_partial_strike_weight - reference.bar_partial_strike_weight).abs()
                < 1e-4
        );
        assert_eq!(calibrated.pickup_law, PickupLaw::Aperture);
        // Every parameter extreme keeps the profile inside its validated ranges.
        for index in 0..PARAMETERS as u32 {
            for value in [0.0, 1.0] {
                if let Some(settings) = Settings::default().with_parameter(index, value) {
                    settings.profile().validate(48_000.0).unwrap();
                }
            }
        }
        for (index, value) in [(2, 3.0), (2, 0.5), (3, -1.0), (3, 1.5), (4, 1.0), (5, 1.0)] {
            Settings::default()
                .with_parameter(index, value)
                .unwrap()
                .profile()
                .validate(48_000.0)
                .unwrap();
        }
    }
}
