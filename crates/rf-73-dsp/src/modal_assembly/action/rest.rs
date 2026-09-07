//! Static convex contact equilibrium, prepared before the first dynamic step.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
/// Residual qualification of the initial static contact solve, before any tick.
pub struct RestPreparation {
    /// Completed bounded Gauss-Seidel sweeps (at most 128).
    pub sweeps: usize,
    /// Worst force or torque row, normalized using terms in that row's units.
    pub max_relative_force_defect: f64,
    /// Worst relative mismatch against F = k * positive(compression)^2.
    pub max_relative_contact_defect: f64,
}
impl<const S: usize, const D: usize> ActionAssembly<S, D> {
    pub(in super::super) fn initialize_rest(&mut self) -> Result<RestPreparation, ModelError> {
        self.initialize_rest_budget(128)
    }
    fn initialize_rest_budget(&mut self, budget: usize) -> Result<RestPreparation, ModelError> {
        // A unique rest preparation requires anchored, positive linear stiffness.
        // Historical constructors still support free/singular configurations.
        if self.stiffness.iter().any(|k| *k <= 0.0) {
            return Err(ModelError(
                "rest preparation requires positive return stiffness",
            ));
        }
        let inverse = numerics::inverse(self.op.k)?;
        let responses: [[f64; D]; CONTACTS] = core::array::from_fn(|j| {
            let mut v = [0.0; D];
            let structural = apply(&inverse, core::array::from_fn(|i| self.ports[j][i]));
            v[..S].copy_from_slice(&structural);
            for i in 0..2 {
                v[S + i] = self.ports[j][S + i] / self.stiffness[i];
            }
            v
        });
        let compliance: [[f64; CONTACTS]; CONTACTS] =
            core::array::from_fn(|i| core::array::from_fn(|j| dot(self.ports[i], responses[j])));
        if compliance
            .iter()
            .enumerate()
            .any(|(i, row)| !row[i].is_finite() || row[i] <= 0.0)
        {
            return Err(ModelError("invalid static contact compliance"));
        }
        let mut candidate = self.state;
        // Only constructors call this method. No time history or external work
        // is replaced by a public live-state reset API.
        let free = candidate.q;
        let gaps = self.compression(candidate);
        let mut forces = [0.0; CONTACTS];
        for sweep in 1..=budget {
            for i in 0..CONTACTS {
                let gap = gaps[i]
                    - (0..CONTACTS)
                        .filter(|&j| j != i)
                        .map(|j| compliance[i][j] * forces[j])
                        .sum::<f64>();
                // F=k*d^2, d=gap-C*F. Rationalized positive root avoids
                // cancellation at vanishing compression; negative gaps stay open.
                let d = if gap > 0.0 {
                    2.0 * gap
                        / (1.0
                            + (1.0 + 4.0 * compliance[i][i] * self.laws[i].stiffness * gap).sqrt())
                } else {
                    0.0
                };
                forces[i] = self.laws[i].stiffness * d * d;
            }
            candidate.q = core::array::from_fn(|i| {
                free[i]
                    - (0..CONTACTS)
                        .map(|j| responses[j][i] * forces[j])
                        .sum::<f64>()
            });
            candidate.f = forces;
            let compression = self.compression(candidate);
            let actual = compression.map(|d| d.max(0.0));
            let mut contact_defect = 0.0_f64;
            for i in 0..CONTACTS {
                let f = self.laws[i].stiffness * actual[i] * actual[i];
                if !f.is_finite() || !forces[i].is_finite() || !compression[i].is_finite() {
                    return Err(ModelError("non-finite static contact force"));
                }
                contact_defect = contact_defect
                    .max((f - forces[i]).abs() / f.abs().max(forces[i].abs()).max(1e-12));
            }
            if contact_defect > 1e-12 {
                continue;
            }
            // Independently evaluate generalized force balance. Row scales use
            // like units, including support torque rows; no force/torque norm mix.
            let mut force_defect = 0.0_f64;
            for i in 0..D {
                let (elastic, scale) = if i < S {
                    let terms =
                        core::array::from_fn::<_, S, _>(|j| self.op.k[i][j] * candidate.q[j]);
                    (
                        terms.iter().sum::<f64>(),
                        terms.iter().map(|x| x.abs()).sum::<f64>(),
                    )
                } else {
                    let base = if i == S {
                        self.profile.hammer_rest_m
                    } else {
                        candidate.pedal
                    };
                    let force = self.stiffness[i - S] * (candidate.q[i] - base);
                    (force, force.abs())
                };
                let contact = (0..CONTACTS)
                    .map(|j| self.ports[j][i] * forces[j])
                    .sum::<f64>();
                let scale = scale
                    + (0..CONTACTS)
                        .map(|j| (self.ports[j][i] * forces[j]).abs())
                        .sum::<f64>();
                if !elastic.is_finite() || !contact.is_finite() || !scale.is_finite() {
                    return Err(ModelError("non-finite static force balance"));
                }
                force_defect = force_defect.max((elastic + contact).abs() / scale.max(1e-12));
            }
            let energy = self.energy(candidate);
            if !energy.is_finite()
                || candidate
                    .q
                    .iter()
                    .chain(&candidate.f)
                    .any(|x| !x.is_finite())
                || !contact_defect.is_finite()
                || !force_defect.is_finite()
                || force_defect > 1e-10
            {
                return Err(ModelError("static rest force qualification failed"));
            }
            self.state = candidate;
            self.initial = energy;
            return Ok(RestPreparation {
                sweeps: sweep,
                max_relative_force_defect: force_defect,
                max_relative_contact_defect: contact_defect,
            });
        }
        Err(ModelError("static rest iteration budget exhausted"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equilibrium_retains_preload_and_failed_preparation_is_atomic() {
        let mut v = ActionAssembly::new(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            FeltDamperProfile::default(),
            ActionProfile::default(),
        )
        .unwrap();
        let before = v.probe();
        assert!(v.initialize_rest_budget(0).is_err());
        assert_eq!(before, v.probe());
        let r = v.initialize_rest().unwrap();
        assert!(r.max_relative_force_defect < 1e-10);
        let a = v.probe();
        assert!(a.initial_energy_j > 0.0 && a.initial_energy_j < before.initial_energy_j);
        assert!(a.contact_force_n[1] > 0.0);
        assert_eq!(a.velocity, [0.0; 11]);
        assert_eq!(a.contact_entries, [0; 4]);
        for _ in 0..100000 {
            v.advance(a.pedestal_position_m, a.pedal_position_m)
                .unwrap();
        }
        let b = v.probe();
        assert!(b.pickup_velocity_m_s.abs() < 1e-10);
        assert!((b.pickup_displacement_xy_m[0] - a.pickup_displacement_xy_m[0]).abs() < 1e-12);
        assert!(b.balance_residual_j.abs() / a.initial_energy_j < 1e-8);
        assert_eq!(b.pedestal_work_j + b.pedal_work_j, 0.0);
    }
}
