//! Two transverse components of one circular tine and one common support/tonebar.
//! Boundary anisotropy is a provisional linear reduction, not measured regulation.
use super::action::ActionStructure;
use super::*;

pub type PolarizedActionAssembly = ActionAssembly<18, 20>;
pub type PolarizedActionProbe = action::ActionProbe<20>;

/// The second axis changes support and tonebar properties only. Circular tine
/// bending stiffness and co-moving tuning mass are identical in both directions.
#[derive(Clone, Copy, Debug)]
pub struct PolarizationProfile {
    /// Rotation of the principal boundary axes relative to laboratory vertical/horizontal.
    pub boundary_angle_rad: f64,
    pub transverse_support_stiffness_ratio: f64,
    pub transverse_rotation_stiffness_ratio: f64,
    pub transverse_tonebar_frequency_ratio: f64,
    pub transverse_boundary_damping_ratio: f64,
    /// Collinear motion/contact-normal axes in laboratory coordinates. Each
    /// reduced mass moves along its own normal; no tangential friction is modeled.
    pub hammer_angle_rad: f64,
    pub felt_angle_rad: f64,
}
impl Default for PolarizationProfile {
    fn default() -> Self {
        Self {
            boundary_angle_rad: 0.3,
            transverse_support_stiffness_ratio: 1.2,
            transverse_rotation_stiffness_ratio: 1.15,
            transverse_tonebar_frequency_ratio: 1.08,
            transverse_boundary_damping_ratio: 1.1,
            hammer_angle_rad: 0.0,
            felt_angle_rad: 0.0,
        }
    }
}
impl PolarizationProfile {
    pub fn isotropic() -> Self {
        Self {
            boundary_angle_rad: 0.0,
            transverse_support_stiffness_ratio: 1.0,
            transverse_rotation_stiffness_ratio: 1.0,
            transverse_tonebar_frequency_ratio: 1.0,
            transverse_boundary_damping_ratio: 1.0,
            hammer_angle_rad: 0.0,
            felt_angle_rad: 0.0,
        }
    }
    pub fn validate(self) -> Result<(), ModelError> {
        for (v, lo, hi) in [
            (self.boundary_angle_rad, -TAU, TAU),
            (self.hammer_angle_rad, -TAU, TAU),
            (self.felt_angle_rad, -TAU, TAU),
            (self.transverse_support_stiffness_ratio, 0.1, 10.0),
            (self.transverse_rotation_stiffness_ratio, 0.1, 10.0),
            (self.transverse_tonebar_frequency_ratio, 0.25, 4.0),
            (self.transverse_boundary_damping_ratio, 0.0, 10.0),
        ] {
            bounded(v, lo, hi)?;
        }
        Ok(())
    }
}

impl ActionAssembly<18, 20> {
    pub fn new_polarized(
        dt: f64,
        geometry: TineGeometry,
        assembly: ModalAssemblyProfile,
        felt: FeltDamperProfile,
        action: ActionProfile,
        polarization: PolarizationProfile,
    ) -> Result<Self, ModelError> {
        assembly.validate()?;
        polarization.validate()?;
        let base = Operators::prepare(geometry, assembly)?;
        Self::prepare(
            dt,
            prepare_structure(&base, polarization),
            assembly,
            felt,
            action,
        )
    }
}

pub(super) fn prepare_structure(base: &Operators, p: PolarizationProfile) -> ActionStructure<18> {
    let mut m = [[0.0; 18]; 18];
    let mut k = m;
    let mut c = m;
    for plane in 0..2 {
        for i in 0..9 {
            for j in 0..9 {
                m[plane * 9 + i][plane * 9 + j] = base.m[i][j];
                k[plane * 9 + i][plane * 9 + j] = base.k[i][j];
                c[plane * 9 + i][plane * 9 + j] = base.c[0][i][j];
            }
        }
    }
    let (sn, cs) = p.boundary_angle_rad.sin_cos();
    // Rotate each principal boundary energy, rather than adding an arbitrary
    // one-way cross force. Equality of the two eigenvalues gives exact zero coupling.
    for (i, ratio) in [
        (0, p.transverse_support_stiffness_ratio),
        (1, p.transverse_rotation_stiffness_ratio),
        (8, p.transverse_tonebar_frequency_ratio.powi(2)),
    ] {
        rotate_pair(&mut k, i, base.k[i][i], base.k[i][i] * ratio, sn, cs);
        rotate_pair(
            &mut c,
            i,
            base.c[0][i][i],
            base.c[0][i][i] * p.transverse_boundary_damping_ratio,
            sn,
            cs,
        );
    }
    let port = |base_port: Vector, angle: f64| {
        let (sn, cs) = angle.sin_cos();
        core::array::from_fn(|i| base_port[i % 9] * if i < 9 { cs } else { sn })
    };
    let mut pickup = [0.0; 18];
    let mut pickup_cross = [0.0; 18];
    pickup[..9].copy_from_slice(&base.pickup);
    pickup_cross[9..].copy_from_slice(&base.pickup);
    ActionStructure {
        m,
        k,
        c: [c],
        hammer: port(base.hammer, p.hammer_angle_rad),
        damper: port(base.damper, p.felt_angle_rad),
        pickup,
        pickup_cross,
    }
}
fn rotate_pair(matrix: &mut [[f64; 18]; 18], i: usize, a: f64, b: f64, sn: f64, cs: f64) {
    let difference = b - a;
    matrix[i][i] = a + difference * sn * sn;
    matrix[i + 9][i + 9] = b - difference * sn * sn;
    matrix[i][i + 9] = -difference * sn * cs;
    matrix[i + 9][i] = matrix[i][i + 9];
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make(p: PolarizationProfile) -> PolarizedActionAssembly {
        PolarizedActionAssembly::new_polarized(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            FeltDamperProfile::default(),
            ActionProfile::default(),
            p,
        )
        .unwrap()
    }
    #[test]
    fn isotropic_and_aligned_boundaries_cannot_create_lateral_motion() {
        for profile in [
            PolarizationProfile {
                boundary_angle_rad: 0.7,
                ..PolarizationProfile::isotropic()
            },
            PolarizationProfile {
                boundary_angle_rad: 0.0,
                ..PolarizationProfile::default()
            },
        ] {
            let mut pair = make(profile);
            let mut planar = ActionAssembly::new(
                1e-6,
                TineGeometry::default(),
                ModalAssemblyProfile::default(),
                FeltDamperProfile::default(),
                ActionProfile::default(),
            )
            .unwrap();
            let action = ActionProfile::default();
            let mut x = action.hammer_rest_m;
            for i in 0..60000 {
                let target = if (10000..35000).contains(&i) {
                    -action.escapement_m
                } else {
                    action.hammer_rest_m
                };
                x += (target - x).clamp(-1.5e-6, 1.5e-6);
                planar.advance(x, action.damper_closed_m).unwrap();
                pair.advance(x, action.damper_closed_m).unwrap();
                let a = planar.probe();
                let b = pair.probe();
                assert_eq!(&b.position[9..18], &[0.0; 9]);
                assert_eq!(&b.velocity[9..18], &[0.0; 9]);
                for j in 0..9 {
                    assert!((a.position[j] - b.position[j]).abs() < 1e-13);
                    assert!((a.velocity[j] - b.velocity[j]).abs() < 1e-9);
                }
                assert!((a.position[9] - b.position[18]).abs() < 1e-13);
                assert!((a.position[10] - b.position[19]).abs() < 1e-13);
                assert!((a.mechanical_energy_j - b.mechanical_energy_j).abs() < 1e-13);
            }
        }
    }
    #[test]
    fn rotation_of_the_entire_physical_configuration_rotates_the_trajectory() {
        let p = PolarizationProfile::default();
        let angle: f64 = 0.4;
        let mut a = make(p);
        let mut b = make(PolarizationProfile {
            boundary_angle_rad: p.boundary_angle_rad + angle,
            hammer_angle_rad: angle,
            felt_angle_rad: angle,
            ..p
        });
        let action = ActionProfile::default();
        let mut x = action.hammer_rest_m;
        let (sn, cs) = angle.sin_cos();
        let mut horizontal_peak = 0.0_f64;
        for i in 0..60000 {
            let target = if (10000..35000).contains(&i) {
                -action.escapement_m
            } else {
                action.hammer_rest_m
            };
            x += (target - x).clamp(-1.5e-6, 1.5e-6);
            a.advance(x, action.damper_closed_m).unwrap();
            b.advance(x, action.damper_closed_m).unwrap();
            let a = a.probe();
            let b = b.probe();
            horizontal_peak = horizontal_peak.max(a.pickup_displacement_xy_m[1].abs());
            for j in 0..9 {
                assert!(
                    (cs * a.position[j] - sn * a.position[j + 9] - b.position[j]).abs() < 1e-11
                );
                assert!(
                    (sn * a.position[j] + cs * a.position[j + 9] - b.position[j + 9]).abs() < 1e-11
                );
            }
            assert!((a.mechanical_energy_j - b.mechanical_energy_j).abs() < 1e-11);
        }
        assert!(horizontal_peak > 1e-8);
    }
    #[test]
    fn rotated_boundary_energies_match_their_principal_form_and_remain_reciprocal() {
        let p = PolarizationProfile::default();
        let base =
            Operators::prepare(TineGeometry::default(), ModalAssemblyProfile::default()).unwrap();
        let op = prepare_structure(&base, p);
        numerics::inverse(op.m).unwrap();
        numerics::inverse(op.k).unwrap();
        numerics::inverse(op.c[0]).unwrap();
        for i in 0..18 {
            for j in 0..18 {
                assert_eq!(op.m[i][j], op.m[j][i]);
                assert_eq!(op.k[i][j], op.k[j][i]);
                assert_eq!(op.c[0][i][j], op.c[0][j][i]);
            }
        }
        let (sn, cs) = p.boundary_angle_rad.sin_cos();
        for i in [0, 1, 8] {
            let ratio = if i == 0 {
                p.transverse_support_stiffness_ratio
            } else if i == 1 {
                p.transverse_rotation_stiffness_ratio
            } else {
                p.transverse_tonebar_frequency_ratio.powi(2)
            };
            let (x, y) = (0.002, -0.003);
            let expected =
                base.k[i][i] * ((cs * x + sn * y).powi(2) + ratio * (-sn * x + cs * y).powi(2));
            let observed =
                op.k[i][i] * x * x + 2.0 * op.k[i][i + 9] * x * y + op.k[i + 9][i + 9] * y * y;
            assert!((expected - observed).abs() < expected * 1e-14);
        }
        // An isotropic transverse pair has m*(vx^2+vy^2)/2, not two times this energy.
        let x: Vector = core::array::from_fn(|i| (i + 1) as f64 * 1e-4);
        let rotated: [f64; 18] = core::array::from_fn(|i| x[i % 9] * if i < 9 { cs } else { sn });
        assert!((dot(rotated, apply(&op.m, rotated)) - dot(x, apply(&base.m, x))).abs() < 1e-20);
    }
    #[test]
    fn invalid_polarization_and_drive_inputs_preserve_the_state() {
        for p in [
            PolarizationProfile {
                boundary_angle_rad: f64::NAN,
                ..PolarizationProfile::default()
            },
            PolarizationProfile {
                transverse_support_stiffness_ratio: -1.0,
                ..PolarizationProfile::default()
            },
        ] {
            assert!(p.validate().is_err());
        }
        let mut v = make(PolarizationProfile::default());
        let before = v.probe();
        assert!(v.advance(f64::INFINITY, before.pedal_position_m).is_err());
        assert_eq!(before, v.probe());
        assert!(v.advance(-0.0015, before.pedal_position_m).is_err());
        assert_eq!(before, v.probe());
    }

    #[test]
    fn selected_boundary_corners_remain_passive_through_moving_contact() {
        for p in [
            PolarizationProfile {
                boundary_angle_rad: 0.7,
                transverse_support_stiffness_ratio: 10.0,
                transverse_rotation_stiffness_ratio: 0.1,
                transverse_tonebar_frequency_ratio: 4.0,
                transverse_boundary_damping_ratio: 0.0,
                hammer_angle_rad: 0.2,
                felt_angle_rad: -0.1,
            },
            PolarizationProfile {
                boundary_angle_rad: 1.1,
                transverse_support_stiffness_ratio: 0.1,
                transverse_rotation_stiffness_ratio: 10.0,
                transverse_tonebar_frequency_ratio: 0.25,
                transverse_boundary_damping_ratio: 10.0,
                hammer_angle_rad: -0.2,
                felt_angle_rad: 0.1,
            },
        ] {
            let mut v = make(p);
            let action = ActionProfile::default();
            let mut x = action.hammer_rest_m;
            let mut r = action.damper_closed_m;
            for i in 0..40000 {
                let target = if (1000..20000).contains(&i) {
                    -action.escapement_m
                } else {
                    action.hammer_rest_m
                };
                x += (target - x).clamp(-1.5e-6, 1.5e-6);
                let pedal_target = action.damper_closed_m
                    - if (5000..27000).contains(&i) {
                        action.pedal_travel_m
                    } else {
                        0.0
                    };
                r += (pedal_target - r).clamp(-1e-6, 1e-6);
                let a = v.probe();
                v.advance(x, r).unwrap();
                let b = v.probe();
                let scale = (b.initial_energy_j + b.absolute_drive_work_j).max(1e-20);
                assert!(b.balance_residual_j.abs() < 1e-8 * scale);
                assert!(b.structural_heat_j >= a.structural_heat_j);
                assert!(b.contact_force_n.iter().all(|f| *f >= 0.0));
                if x == a.pedestal_position_m && r == a.pedal_position_m {
                    assert!(b.mechanical_energy_j <= a.mechanical_energy_j + 1e-10 * scale);
                }
            }
        }
    }
}
