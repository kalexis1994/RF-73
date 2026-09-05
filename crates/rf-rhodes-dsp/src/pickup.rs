use crate::{ModelError, Profile, SAMPLE_RATE_MIN};

/// Immutable geometry for the provisional, one-coordinate magnetic transducer.
#[derive(Clone, Copy)]
pub struct MagneticPickup {
    gap_m: f64,
    offset_m: f64,
}

impl MagneticPickup {
    pub fn new(gap_m: f64, offset_m: f64) -> Result<Self, ModelError> {
        Profile {
            pickup_gap_m: gap_m,
            pickup_offset_m: offset_m,
            ..Profile::default()
        }
        .validate(SAMPLE_RATE_MIN)?;
        Ok(Self { gap_m, offset_m })
    }

    pub(crate) fn from_validated_profile(profile: Profile) -> Self {
        Self {
            gap_m: profile.pickup_gap_m,
            offset_m: profile.pickup_offset_m,
        }
    }

    /// Production law: -0.015 d/dt [1 / sqrt(1 + z^2)].
    /// Inputs are finite mechanical state; the engine owns numerical-fault recovery.
    #[inline]
    pub fn voltage(self, displacement_m: f64, velocity_m_s: f64) -> f64 {
        let z = (self.offset_m + displacement_m) / self.gap_m;
        let base = 1.0 + z * z;
        // Preserve the original operation order for production output compatibility.
        0.015 * z * velocity_m_s / (self.gap_m * base * base.sqrt())
    }

    /// Experimental field proxy: -0.015 d/dt [(1 + z^2)^(-3/2)].
    /// Normalized axial field of a single effective pole at fixed axial distance,
    /// used as a flux proxy. No finite pole surface, magnetic loading or second
    /// motion coordinate. This is not the full DAFx 2017 pickup model.
    pub fn research_point_pole_voltage(self, displacement_m: f64, velocity_m_s: f64) -> f64 {
        let z = (self.offset_m + displacement_m) / self.gap_m;
        let base = 1.0 + z * z;
        0.045 * z * velocity_m_s / (self.gap_m * base * base * base.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltage_matches_negative_flux_derivative_for_both_laws() {
        let pickup = MagneticPickup::new(0.0015, 0.0005).unwrap();
        for position in [-0.002, -0.0005, 0.0, 0.001, 0.003] {
            for power in [0.5, 1.5] {
                let flux = |x: f64| 0.015 * (1.0 + ((0.0005 + x) / 0.0015).powi(2)).powf(-power);
                let dx = 1e-8;
                let expected = -(flux(position + dx) - flux(position - dx)) / (2.0 * dx) * 0.3;
                let actual = if power == 0.5 {
                    pickup.voltage(position, 0.3)
                } else {
                    pickup.research_point_pole_voltage(position, 0.3)
                };
                assert!((actual - expected).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn validates_geometry_and_preserves_stationary_and_reflection_symmetry() {
        for (gap, offset) in [
            (0.0, 0.0),
            (f64::NAN, 0.0),
            (0.001, f64::INFINITY),
            (0.001, 0.004),
        ] {
            assert!(MagneticPickup::new(gap, offset).is_err());
        }
        let a = MagneticPickup::new(0.0005, 0.00025).unwrap();
        let b = MagneticPickup::new(0.0005, -0.00025).unwrap();
        assert_eq!(a.voltage(0.0002, 0.0), 0.0);
        assert_eq!(a.research_point_pole_voltage(0.0002, 0.0), 0.0);
        assert_eq!(a.voltage(0.0002, 0.3), b.voltage(-0.0002, -0.3));
        assert_eq!(
            a.research_point_pole_voltage(0.0002, 0.3),
            b.research_point_pole_voltage(-0.0002, -0.3)
        );
    }
}
