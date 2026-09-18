//! Offline undamped modes of the planar or polarized structure.
use super::{
    ModalAssemblyProfile, N, Operators, PolarizationProfile, TineGeometry,
    numerics::{apply, dot, inverse},
};
use crate::ModelError;
use core::f64::consts::TAU;

#[derive(Debug, Clone, Copy)]
pub struct StructuralMode<const D: usize = N> {
    pub frequency_hz: f64,
    /// Generalized coordinates, mass normalized: shape^T M shape = 1.
    pub shape: [f64; D],
    pub hammer_weight: f64,
    pub pickup_weight: f64,
    /// Squared M-inner-product projection onto the first fixed-root tine coordinate.
    /// A participation diagnostic, not a measured modal energy fraction.
    pub first_tine_projection: f64,
    pub relative_eigen_residual: f64,
}

#[derive(Debug, Clone)]
pub struct ModalSpectrum<const D: usize = N> {
    pub modes: [StructuralMode<D>; D],
    pub mass_matrix: [[f64; D]; D],
    pub maximum_mass_orthogonality_error: f64,
    pub fixed_root_fundamental_hz: f64,
}
impl ModalSpectrum {
    /// Uses the same mass, stiffness and spatial ports as the time-domain assembly.
    /// Losses, contact and magnetic conversion do not enter this undamped problem.
    pub fn prepare(
        geometry: TineGeometry,
        profile: ModalAssemblyProfile,
    ) -> Result<Self, ModelError> {
        geometry.validate()?;
        profile.validate()?;
        let op = Operators::prepare(geometry, profile)?;
        Self::from_structure(op.m, op.k, op.hammer, op.pickup)
    }
}
impl ModalSpectrum<18> {
    /// Same two-plane mass, stiffness and vertical pickup as the action.
    /// Excludes contact, damping and magnetic loading; output pitch needs a
    /// separate time-domain qualification. Projection refers to the vertical tine.
    pub fn prepare_polarized(
        geometry: TineGeometry,
        profile: ModalAssemblyProfile,
        polarization: PolarizationProfile,
    ) -> Result<Self, ModelError> {
        profile.validate()?;
        polarization.validate()?;
        let base = Operators::prepare(geometry, profile)?;
        let op = super::polarization::prepare_structure(&base, polarization);
        Self::from_structure(op.m, op.k, op.hammer, op.pickup)
    }
}
impl<const D: usize> ModalSpectrum<D> {
    fn from_structure(
        m: [[f64; D]; D],
        k: [[f64; D]; D],
        hammer: [f64; D],
        pickup: [f64; D],
    ) -> Result<Self, ModelError> {
        let flat = |m: [[f64; D]; D]| m.into_iter().flatten().collect::<Vec<_>>();
        let (values, vectors) = crate::tine::eigen(&flat(k), &flat(m), D, D)?;
        let largest = values.iter().copied().fold(0.0_f64, f64::max);
        let inv = inverse(m)?;
        let mut modes = Vec::with_capacity(D);
        for (value, vector) in values.into_iter().zip(vectors) {
            if !value.is_finite() || value < -largest * 1e-12 {
                return Err(ModelError("invalid structural eigenvalue"));
            }
            let lambda = if value.abs() < largest * 1e-12 {
                0.0
            } else {
                value
            };
            let shape: [f64; D] = vector
                .try_into()
                .map_err(|_| ModelError("incomplete structural eigenvector"))?;
            let mv = apply(&m, shape);
            let norm = dot(shape, mv);
            let kv = apply(&k, shape);
            let force = core::array::from_fn(|i| kv[i] - lambda * mv[i]);
            let error = dot(force, apply(&inv, force));
            let scale = if lambda > 0.0 {
                2.0 * lambda * lambda * norm
            } else {
                largest * largest * norm
            };
            let residual = (error.max(0.0) / scale).sqrt();
            let projection = mv[2] * mv[2] / (m[2][2] * norm);
            if ![norm, residual, projection].iter().all(|x| x.is_finite())
                || norm <= 0.0
                || residual > 1e-8
                || !(0.0..=1.0 + 1e-8).contains(&projection)
            {
                return Err(ModelError("structural eigenmode qualification failed"));
            }
            modes.push(StructuralMode {
                frequency_hz: lambda.sqrt() / TAU,
                shape,
                hammer_weight: dot(hammer, shape),
                pickup_weight: dot(pickup, shape),
                first_tine_projection: projection,
                relative_eigen_residual: residual,
            });
        }
        let mut orthogonality = 0.0_f64;
        for (i, a) in modes.iter().enumerate() {
            for (j, b) in modes.iter().enumerate() {
                orthogonality =
                    orthogonality.max((dot(a.shape, apply(&m, b.shape)) - f64::from(i == j)).abs());
            }
        }
        if orthogonality > 1e-8 {
            return Err(ModelError("structural modes are not mass orthonormal"));
        }
        Ok(Self {
            modes: modes
                .try_into()
                .map_err(|_| ModelError("missing structural modes"))?,
            mass_matrix: m,
            maximum_mass_orthogonality_error: orthogonality,
            fixed_root_fundamental_hz: (k[2][2] / m[2][2]).sqrt() / TAU,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ModalAssembly, ModalIntegration, Vector};
    #[test]
    fn polarized_spectrum_recovers_planar_doublets_and_rotation_invariant_frequencies() {
        use super::*;
        let geometry = TineGeometry::default();
        let profile = ModalAssemblyProfile::default();
        let planar = ModalSpectrum::prepare(geometry, profile).unwrap();
        let pair = ModalSpectrum::<18>::prepare_polarized(
            geometry,
            profile,
            PolarizationProfile::isotropic(),
        )
        .unwrap();
        for (i, mode) in planar.modes.iter().enumerate() {
            for index in [2 * i, 2 * i + 1] {
                assert!((pair.modes[index].frequency_hz / mode.frequency_hz - 1.0).abs() < 1e-8);
            }
        }
        let p = PolarizationProfile::default();
        let a = ModalSpectrum::<18>::prepare_polarized(geometry, profile, p).unwrap();
        let b = ModalSpectrum::<18>::prepare_polarized(
            geometry,
            profile,
            PolarizationProfile {
                boundary_angle_rad: 1.1,
                ..p
            },
        )
        .unwrap();
        for (x, y) in a.modes.iter().zip(&b.modes) {
            assert!((x.frequency_hz / y.frequency_hz - 1.0).abs() < 1e-8);
            assert!(x.relative_eigen_residual < 1e-8);
        }
        assert!(a.maximum_mass_orthogonality_error < 1e-8);
        assert!(
            ModalSpectrum::<18>::prepare_polarized(
                geometry,
                profile,
                PolarizationProfile {
                    boundary_angle_rad: f64::NAN,
                    ..p
                }
            )
            .is_err()
        );
    }
    #[test]
    fn sliding_tuning_mass_changes_coupled_pitch_and_partial_ratios() {
        for length in [0.07, 0.075] {
            let mut rows = Vec::new();
            for position in [0.0, 0.5, 0.85, 1.0] {
                let s = ModalSpectrum::prepare(
                    TineGeometry {
                        length_m: length,
                        tuning_position: position,
                        ..TineGeometry::default()
                    },
                    ModalAssemblyProfile::default(),
                )
                .unwrap();
                let mode = s
                    .modes
                    .iter()
                    .max_by(|a, b| a.first_tine_projection.total_cmp(&b.first_tine_projection))
                    .unwrap();
                eprintln!(
                    "length={length} spring={position} coupled_hz={} fixed_hz={}",
                    mode.frequency_hz, s.fixed_root_fundamental_hz
                );
                rows.push((
                    mode.frequency_hz,
                    s.modes[4].frequency_hz / mode.frequency_hz,
                ));
            }
            assert!(rows.windows(2).all(|w| w[0].0 > w[1].0));
            assert!((rows[0].1 - rows[3].1).abs() > 0.01);
        }
    }
    use super::*;
    #[test]
    fn complete_modes_reconstruct_static_port_compliance_and_expose_root_shift() {
        for length in [0.065, 0.075, 0.12] {
            let g = TineGeometry {
                length_m: length,
                ..TineGeometry::default()
            };
            let p = ModalAssemblyProfile::default();
            let spectrum = ModalSpectrum::prepare(g, p).unwrap();
            let op = Operators::prepare(g, p).unwrap();
            let direct = dot(op.pickup, apply(&inverse(op.k).unwrap(), op.hammer));
            let modal: f64 = spectrum
                .modes
                .iter()
                .map(|m| m.hammer_weight * m.pickup_weight / (TAU * m.frequency_hz).powi(2))
                .sum();
            assert!((modal / direct - 1.0).abs() < 1e-8);
            assert!(spectrum.maximum_mass_orthogonality_error < 1e-10);
            let dominant = spectrum
                .modes
                .iter()
                .max_by(|a, b| a.first_tine_projection.total_cmp(&b.first_tine_projection))
                .unwrap();
            assert!((dominant.frequency_hz - spectrum.fixed_root_fundamental_hz).abs() > 0.1);
        }
    }
    #[test]
    fn eigenmode_tracks_the_independent_time_domain_free_transition() {
        let g = TineGeometry::default();
        let p = ModalAssemblyProfile {
            translation_damping_n_s_m: 0.0,
            rotation_damping_n_m_s_rad: 0.0,
            ..ModalAssemblyProfile::default()
        };
        let mut op = Operators::prepare(g, p).unwrap();
        // The public profile bounds finite tine/bar decay. Remove C only inside
        // this lossless analytic test; production preparation is unchanged.
        op.c = [[[0.0; N]; N]; 2];
        let spectrum = ModalSpectrum::prepare(g, p).unwrap();
        let transition =
            super::super::numerics::Free::prepare(op.m, op.k, op.c[0], 1.0 / 48000.0).unwrap();
        for mode in &spectrum.modes {
            let mut q = mode.shape.map(|x| x * 1e-6);
            let mut v = [0.0; N];
            for _ in 0..480 {
                (q, v, _) = transition.advance(q, v);
            }
            let phase = TAU * mode.frequency_hz * 0.01;
            let error: Vector = core::array::from_fn(|i| q[i] - 1e-6 * mode.shape[i] * phase.cos());
            assert!(dot(error, apply(&op.m, error)).sqrt() / 1e-6 < 1e-7);
        }
        let actual = ModalAssembly::new(
            48000.0,
            g,
            p,
            ModalIntegration::Refined {
                contact_substeps: 32,
            },
        )
        .unwrap();
        assert_eq!(actual.mass_matrix(), spectrum.mass_matrix);
    }
    #[test]
    fn free_support_retains_two_rigid_modes_and_invalid_geometry_is_rejected() {
        let p = ModalAssemblyProfile {
            translation_stiffness_n_m: 0.0,
            rotation_stiffness_n_m_rad: 0.0,
            ..ModalAssemblyProfile::default()
        };
        let spectrum = ModalSpectrum::prepare(TineGeometry::default(), p).unwrap();
        assert_eq!(
            spectrum
                .modes
                .iter()
                .filter(|m| m.frequency_hz == 0.0)
                .count(),
            2
        );
        assert!(
            ModalSpectrum::prepare(
                TineGeometry {
                    length_m: f64::NAN,
                    ..TineGeometry::default()
                },
                p
            )
            .is_err()
        );
    }
}
