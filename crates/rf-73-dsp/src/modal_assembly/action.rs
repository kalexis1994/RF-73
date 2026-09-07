//! Persistent hammer, moving felt and reciprocal tension-only bridle reduction.
//! Offline experiment: prescribed pedestal motion, not a measured pivot/cam model.
use super::dissipative_contact::RateContact;
use super::*;

#[cfg(test)]
const D: usize = N + 2;
#[cfg(test)]
const H: usize = N;
#[cfg(test)]
const Z: usize = N + 1;
const CONTACTS: usize = 4;
type Forces = [f64; CONTACTS];

/// Hammer-tip-equivalent distances; key dip is not identified with these distances.
#[derive(Clone, Copy, Debug)]
pub struct ActionProfile {
    pub hammer_rest_m: f64,
    pub escapement_m: f64,
    pub hammer_return_n_m: f64,
    pub hammer_return_n_s_m: f64,
    pub pedestal_stiffness_n_m2: f64,
    pub pedestal_rate_loss_s_m: f64,
    pub bridle_ratio: f64,
    pub bridle_slack_m: f64,
    pub bridle_stiffness_n_m2: f64,
    pub bridle_rate_loss_s_m: f64,
    pub damper_closed_m: f64,
    pub pedal_travel_m: f64,
}
impl Default for ActionProfile {
    fn default() -> Self {
        Self {
            hammer_rest_m: -0.012,
            escapement_m: 0.0015,
            hammer_return_n_m: 4.0,
            hammer_return_n_s_m: 0.025,
            pedestal_stiffness_n_m2: 2e8,
            pedestal_rate_loss_s_m: 2.0,
            bridle_ratio: 0.8,
            bridle_slack_m: 0.002,
            bridle_stiffness_n_m2: 2e7,
            bridle_rate_loss_s_m: 2.0,
            damper_closed_m: 0.0002,
            pedal_travel_m: 0.01,
        }
    }
}
impl ActionProfile {
    pub fn validate(self) -> Result<(), ModelError> {
        for (v, lo, hi) in [
            (self.hammer_rest_m, -0.04, -0.005),
            (self.escapement_m, 0.0005, 0.01),
            (self.hammer_return_n_m, 0.0, 100.0),
            (self.hammer_return_n_s_m, 0.0, 2.0),
            (self.pedestal_stiffness_n_m2, 1e5, 1e10),
            (self.pedestal_rate_loss_s_m, 0.0, 100.0),
            (self.bridle_ratio, 0.1, 2.0),
            (self.bridle_slack_m, 0.0, 0.02),
            (self.bridle_stiffness_n_m2, 1e4, 1e10),
            (self.bridle_rate_loss_s_m, 0.0, 100.0),
            (self.damper_closed_m, -0.001, 0.001),
            (self.pedal_travel_m, 0.0001, 0.015),
        ] {
            bounded(v, lo, hi)?;
        }
        if self.escapement_m >= -self.hammer_rest_m {
            return Err(ModelError("escapement must leave positive pedestal travel"));
        }
        Ok(())
    }
    pub fn pedestal_position_m(self, key: f64) -> Result<f64, ModelError> {
        self.validate()?;
        bounded(key, 0.0, 1.0)?;
        Ok(
            (self.hammer_rest_m + (-self.escapement_m - self.hammer_rest_m) * key)
                .clamp(self.hammer_rest_m, -self.escapement_m),
        )
    }
    pub fn pedal_position_m(self, pedal: f64) -> Result<f64, ModelError> {
        self.validate()?;
        bounded(pedal, 0.0, 1.0)?;
        Ok(self.damper_closed_m - self.pedal_travel_m * pedal)
    }
}

/// Contact arrays are ordered: hammer/tine, felt/tine, pedestal/hammer, bridle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionProbe<const D: usize = 11> {
    /// Structural coordinates followed by hammer and arm. The planar structure
    /// has nine coordinates; the polarized structure has vertical/horizontal
    /// blocks of nine. Root angles (index 1 in each block) are in radians.
    pub position: [f64; D],
    pub velocity: [f64; D],
    pub pickup_velocity_m_s: f64,
    /// Vertical and horizontal laboratory axes; horizontal is zero in the planar model.
    pub pickup_displacement_xy_m: [f64; 2],
    pub pickup_velocity_xy_m_s: [f64; 2],
    pub pedestal_position_m: f64,
    pub pedal_position_m: f64,
    pub compression_m: Forces,
    pub contact_force_n: Forces,
    pub contact_heat_j: Forces,
    pub contact_entries: [u64; CONTACTS],
    pub limited_unloading_steps: [u64; CONTACTS],
    pub simultaneous_hammer_felt_steps: u64,
    pub solver_sweeps: usize,
    pub maximum_solver_sweeps: usize,
    pub mechanical_energy_j: f64,
    pub initial_energy_j: f64,
    pub structural_heat_j: f64,
    pub hammer_return_heat_j: f64,
    pub arm_heat_j: f64,
    pub pickup_force_work_j: f64,
    pub pedestal_work_j: f64,
    pub pedal_work_j: f64,
    pub absolute_drive_work_j: f64,
    pub balance_residual_j: f64,
}

#[derive(Clone, Copy)]
pub(super) struct State<const D: usize> {
    q: [f64; D],
    v: [f64; D],
    pedestal: f64,
    pedal: f64,
    f: Forces,
    heat: Forces,
    entries: [u64; CONTACTS],
    limited: [u64; CONTACTS],
    simultaneous: u64,
    sweeps: usize,
    max_sweeps: usize,
    structure_heat: f64,
    hammer_heat: f64,
    arm_heat: f64,
    work: [f64; 2],
    absolute_work: f64,
    pickup_work: f64,
}

/// Fixed-size, bounded, transactional stepping. No strike-time state reset or
/// velocity assignment. The caller supplies continuous mechanical drive positions.
pub struct ActionAssembly<const S: usize = 9, const D: usize = 11> {
    op: ActionStructure<S>,
    step: Midpoint<S>,
    profile: ActionProfile,
    mass: [f64; 2],
    stiffness: [f64; 2],
    damping: [f64; 2],
    inverse_a: [f64; 2],
    ports: [[f64; D]; CONTACTS],
    responses: [[f64; D]; CONTACTS],
    pickup_response: [[f64; S]; 2],
    compliance: [[f64; CONTACTS]; CONTACTS],
    laws: [RateContact; CONTACTS],
    h: f64,
    state: State<D>,
    initial: f64,
}

// Exact scalar coordinate solves of the joint monotone contact system. Each
// sweep uses all cross-compliances; acceptance checks all simultaneous residuals.
// A failure never publishes an approximate contact state.
fn solve_joint(
    laws: &[RateContact; CONTACTS],
    a: Forces,
    free: Forces,
    c: &[[f64; CONTACTS]; CONTACTS],
    mut force: Forces,
    budget: usize,
) -> Result<(Forces, usize), ModelError> {
    for sweep in 1..=budget {
        for i in 0..CONTACTS {
            let coupled = free[i]
                - (0..CONTACTS)
                    .filter(|&j| j != i)
                    .map(|j| c[i][j] * force[j])
                    .sum::<f64>();
            force[i] = laws[i].advance::<true>(a[i], coupled, c[i][i]).force;
        }
        let b: Forces = core::array::from_fn(|i| free[i] - dot(c[i], force));
        let mut converged = true;
        let scale = force.iter().copied().fold(1e-8_f64, f64::max);
        for i in 0..CONTACTS {
            let (value, slope) = laws[i].law(a[i], b[i]);
            if !value.is_finite() || !force[i].is_finite() || force[i] < 0.0 {
                return Err(ModelError("non-finite joint action contact"));
            }
            // The rate law amplifies endpoint rounding by its local slope.
            // Different dot-product association cannot resolve below this floor.
            let endpoint_scale = free[i].abs()
                + (0..CONTACTS)
                    .map(|j| (c[i][j] * force[j]).abs())
                    .sum::<f64>();
            let tolerance =
                32.0 * f64::EPSILON * scale + 8.0 * f64::EPSILON * slope * endpoint_scale;
            converged &= (force[i] - value).abs() <= tolerance;
        }
        if converged {
            return Ok((force, sweep));
        }
    }
    Err(ModelError(
        "joint action contact iteration budget exhausted",
    ))
}

impl ActionAssembly<9, 11> {
    pub fn new(
        h: f64,
        geometry: TineGeometry,
        assembly: ModalAssemblyProfile,
        felt: FeltDamperProfile,
        profile: ActionProfile,
    ) -> Result<Self, ModelError> {
        bounded(h, 1e-9, 1e-4)?;
        assembly.validate()?;
        felt.validate()?;
        profile.validate()?;
        let op = Operators::prepare(geometry, assembly)?;
        Self::prepare(h, ActionStructure::from_planar(op), assembly, felt, profile)
    }
}

pub(super) struct ActionStructure<const S: usize> {
    pub m: [[f64; S]; S],
    pub k: [[f64; S]; S],
    pub c: [[[f64; S]; S]; 1],
    pub hammer: [f64; S],
    pub damper: [f64; S],
    pub pickup: [f64; S],
    pub pickup_cross: [f64; S],
}
impl ActionStructure<9> {
    fn from_planar(op: Operators) -> Self {
        Self {
            m: op.m,
            k: op.k,
            c: [op.c[0]],
            hammer: op.hammer,
            damper: op.damper,
            pickup: op.pickup,
            pickup_cross: [0.0; 9],
        }
    }
}
impl<const S: usize, const D: usize> ActionAssembly<S, D> {
    pub(super) fn prepare(
        h: f64,
        op: ActionStructure<S>,
        assembly: ModalAssemblyProfile,
        felt: FeltDamperProfile,
        profile: ActionProfile,
    ) -> Result<Self, ModelError> {
        if D != S + 2 {
            return Err(ModelError("invalid action coordinate dimensions"));
        }
        bounded(h, 1e-9, 1e-4)?;
        assembly.validate()?;
        felt.validate()?;
        profile.validate()?;
        let step = Midpoint::prepare(op.m, op.k, op.c[0], op.hammer, h)?;
        let ds = Midpoint::prepare(op.m, op.k, op.c[0], op.damper, h)?;
        let pickup_response = [
            Midpoint::prepare(op.m, op.k, op.c[0], op.pickup, h)?.response,
            Midpoint::prepare(op.m, op.k, op.c[0], op.pickup_cross, h)?.response,
        ];
        let mass = [assembly.hammer_mass_kg, felt.arm_mass_kg];
        let stiffness = [profile.hammer_return_n_m, felt.arm_stiffness_n_m];
        let damping = [profile.hammer_return_n_s_m, felt.arm_damping_n_s_m];
        let inverse_a: [f64; 2] = core::array::from_fn(|i| {
            1.0 / (mass[i] + h * damping[i] / 2.0 + h * h * stiffness[i] / 4.0)
        });
        let mut ports = [[0.0; D]; CONTACTS];
        ports[0][..S].copy_from_slice(&op.hammer.map(|x| -x));
        ports[1][..S].copy_from_slice(&op.damper.map(|x| -x));
        ports[0][S] = 1.0;
        ports[1][S + 1] = 1.0;
        ports[2][S] = -1.0;
        ports[3][S] = profile.bridle_ratio;
        ports[3][S + 1] = 1.0;
        let mut responses = [[0.0; D]; CONTACTS];
        responses[0][..S].copy_from_slice(&step.response);
        responses[1][..S].copy_from_slice(&ds.response);
        for j in 0..CONTACTS {
            for i in 0..2 {
                responses[j][S + i] = -h * inverse_a[i] * ports[j][S + i];
            }
        }
        let compliance = core::array::from_fn(|i| {
            core::array::from_fn(|j| -0.5 * h * dot(ports[i], responses[j]))
        });
        let parameters = [
            (
                assembly.contact_stiffness_n_m2,
                assembly.contact_damping_s_m,
            ),
            (felt.felt_stiffness_n_m2, felt.felt_rate_loss_s_m),
            (
                profile.pedestal_stiffness_n_m2,
                profile.pedestal_rate_loss_s_m,
            ),
            (profile.bridle_stiffness_n_m2, profile.bridle_rate_loss_s_m),
        ];
        let laws = parameters.map(|(stiffness, beta)| RateContact {
            stiffness,
            rate: beta / h,
        });
        let mut q = [0.0; D];
        q[S] = profile.hammer_rest_m;
        q[S + 1] = profile.damper_closed_m;
        let state = State {
            q,
            v: [0.0; D],
            pedestal: profile.hammer_rest_m,
            pedal: profile.damper_closed_m,
            f: [0.0; CONTACTS],
            heat: [0.0; CONTACTS],
            entries: [0; CONTACTS],
            limited: [0; CONTACTS],
            simultaneous: 0,
            sweeps: 0,
            max_sweeps: 0,
            structure_heat: 0.0,
            hammer_heat: 0.0,
            arm_heat: 0.0,
            work: [0.0; 2],
            absolute_work: 0.0,
            pickup_work: 0.0,
        };
        let mut result = Self {
            op,
            step,
            profile,
            mass,
            stiffness,
            damping,
            inverse_a,
            ports,
            responses,
            pickup_response,
            compliance,
            laws,
            h,
            state,
            initial: 0.0,
        };
        result.initial = result.energy(state);
        Ok(result)
    }
    fn compression(&self, s: State<D>) -> Forces {
        let mut d = self.ports.map(|port| dot(port, s.q));
        d[2] += s.pedestal;
        d[3] -= self.profile.bridle_ratio * self.profile.hammer_rest_m
            + self.profile.damper_closed_m
            + self.profile.bridle_slack_m;
        d
    }
    fn energy(&self, s: State<D>) -> f64 {
        let q = core::array::from_fn(|i| s.q[i]);
        let v = core::array::from_fn(|i| s.v[i]);
        0.5 * (dot(q, apply(&self.op.k, q)) + dot(v, apply(&self.op.m, v)))
            + 0.5 * self.mass[0] * s.v[S].powi(2)
            + 0.5 * self.mass[1] * s.v[S + 1].powi(2)
            + 0.5 * self.stiffness[0] * (s.q[S] - self.profile.hammer_rest_m).powi(2)
            + 0.5 * self.stiffness[1] * (s.q[S + 1] - s.pedal).powi(2)
            + self
                .compression(s)
                .iter()
                .zip(&self.laws)
                .map(|(d, law)| law.stiffness * d.max(0.0).powi(3) / 3.0)
                .sum::<f64>()
    }
    pub fn structural_mass_matrix(&self) -> [[f64; S]; S] {
        self.op.m
    }
    pub fn structural_stiffness_matrix(&self) -> [[f64; S]; S] {
        self.op.k
    }
    pub fn structural_damping_matrix(&self) -> [[f64; S]; S] {
        self.op.c[0]
    }
    pub fn structural_hammer_port(&self) -> [f64; S] {
        self.op.hammer
    }
    pub fn structural_damper_port(&self) -> [f64; S] {
        self.op.damper
    }
    pub fn advance(&mut self, pedestal_m: f64, pedal_m: f64) -> Result<(), ModelError> {
        self.advance_with_budget(pedestal_m, pedal_m, 64)
    }
    fn advance_with_budget(
        &mut self,
        pedestal: f64,
        pedal: f64,
        budget: usize,
    ) -> Result<(), ModelError> {
        self.advance_forced(pedestal, pedal, budget, [0.0; 2])
    }
    pub(super) fn checkpoint(&self) -> State<D> {
        self.state
    }
    pub(super) fn restore(&mut self, state: State<D>) {
        self.state = state;
    }
    pub(super) fn advance_with_pickup_force(
        &mut self,
        pedestal: f64,
        pedal: f64,
        force: [f64; 2],
    ) -> Result<(), ModelError> {
        self.advance_forced(pedestal, pedal, 64, force)
    }
    fn advance_forced(
        &mut self,
        pedestal: f64,
        pedal: f64,
        budget: usize,
        pickup_force: [f64; 2],
    ) -> Result<(), ModelError> {
        for f in pickup_force {
            bounded(f, -100.0, 100.0)?;
        }
        bounded(
            pedestal,
            self.profile.hammer_rest_m,
            -self.profile.escapement_m,
        )?;
        bounded(
            pedal,
            self.profile.damper_closed_m - self.profile.pedal_travel_m,
            self.profile.damper_closed_m,
        )?;
        let a = self.state;
        let h = self.h;
        for (next, old) in [(pedestal, a.pedestal), (pedal, a.pedal)] {
            if (next - old).abs() > 2.0 * h + 8.0 * f64::EPSILON * next.abs().max(old.abs()) {
                return Err(ModelError("action drive exceeds 2 m/s"));
            }
        }
        let mut b = a;
        b.pedestal = pedestal;
        b.pedal = pedal;
        let vf = self.step.free_velocity(
            core::array::from_fn(|i| a.q[i]),
            core::array::from_fn(|i| a.v[i]),
        );
        b.v[..S].copy_from_slice(&vf);
        if pickup_force != [0.0; 2] {
            for (i, velocity) in b.v[..S].iter_mut().enumerate() {
                *velocity += self.pickup_response[0][i] * pickup_force[0]
                    + self.pickup_response[1][i] * pickup_force[1];
            }
        }
        let bases = [self.profile.hammer_rest_m, 0.5 * (a.pedal + pedal)];
        let base_velocity = [0.0, (pedal - a.pedal) / h];
        for i in 0..2 {
            let j = S + i;
            b.v[j] = a.v[j]
                + self.inverse_a[i]
                    * (-h * self.stiffness[i] * (a.q[j] - bases[i])
                        - h * self.damping[i] * (a.v[j] - base_velocity[i])
                        - 0.5 * h * h * self.stiffness[i] * a.v[j]);
        }
        b.q = core::array::from_fn(|i| a.q[i] + 0.5 * h * (a.v[i] + b.v[i]));
        let compression_a = self.compression(a);
        let (force, sweeps) = solve_joint(
            &self.laws,
            compression_a,
            self.compression(b),
            &self.compliance,
            a.f,
            budget,
        )?;
        for i in 0..D {
            let dv = (0..CONTACTS)
                .map(|j| self.responses[j][i] * force[j])
                .sum::<f64>();
            b.v[i] += dv;
            b.q[i] += 0.5 * h * dv;
        }
        let compression_b = self.compression(b);
        if pickup_force != [0.0; 2] {
            let qa = core::array::from_fn(|i| a.q[i]);
            let qb = core::array::from_fn(|i| b.q[i]);
            b.pickup_work += pickup_force[0] * (dot(self.op.pickup, qb) - dot(self.op.pickup, qa))
                + pickup_force[1] * (dot(self.op.pickup_cross, qb) - dot(self.op.pickup_cross, qa));
        }
        for i in 0..CONTACTS {
            // Re-evaluate the material heat from actual endpoints, independently
            // of the joint force residual and global energy bookkeeping.
            let material = self.laws[i].advance::<true>(compression_a[i], compression_b[i], 0.0);
            b.heat[i] += material.heat;
            b.limited[i] = b.limited[i].saturating_add(u64::from(material.limited));
            b.entries[i] = b.entries[i].saturating_add(u64::from(force[i] > 0.0 && a.f[i] == 0.0));
        }
        b.f = force;
        b.sweeps = sweeps;
        b.max_sweeps = b.max_sweeps.max(sweeps);
        b.simultaneous = b
            .simultaneous
            .saturating_add(u64::from(force[0] > 0.0 && force[1] > 0.0));
        let vm: [f64; D] = core::array::from_fn(|i| 0.5 * (a.v[i] + b.v[i]));
        let sv = core::array::from_fn(|i| vm[i]);
        b.structure_heat += h * dot(sv, apply(&self.op.c[0], sv));
        b.hammer_heat += h * self.damping[0] * vm[S].powi(2);
        b.arm_heat += h * self.damping[1] * (vm[S + 1] - base_velocity[1]).powi(2);
        let work = [
            force[2] * (pedestal - a.pedestal),
            -(self.stiffness[1] * (0.5 * (a.q[S + 1] + b.q[S + 1]) - bases[1])
                + self.damping[1] * (vm[S + 1] - base_velocity[1]))
                * (pedal - a.pedal),
        ];
        for (i, w) in work.into_iter().enumerate() {
            b.work[i] += w;
            b.absolute_work += w.abs();
        }
        if b.q
            .iter()
            .chain(&b.v)
            .chain(&b.heat)
            .chain(&b.work)
            .chain(&[
                b.structure_heat,
                b.hammer_heat,
                b.arm_heat,
                b.absolute_work,
                b.pickup_work,
                self.energy(b),
            ])
            .any(|x| !x.is_finite())
            || b.heat.iter().zip(a.heat).any(|(&next, old)| next < old)
        {
            return Err(ModelError("non-finite or nonpassive action state"));
        }
        self.state = b;
        Ok(())
    }
    pub fn probe(&self) -> ActionProbe<D> {
        let s = self.state;
        let energy = self.energy(s);
        ActionProbe {
            position: s.q,
            velocity: s.v,
            pickup_velocity_m_s: dot(self.op.pickup, core::array::from_fn(|i| s.v[i])),
            pickup_displacement_xy_m: [
                dot(self.op.pickup, core::array::from_fn(|i| s.q[i])),
                dot(self.op.pickup_cross, core::array::from_fn(|i| s.q[i])),
            ],
            pickup_velocity_xy_m_s: [
                dot(self.op.pickup, core::array::from_fn(|i| s.v[i])),
                dot(self.op.pickup_cross, core::array::from_fn(|i| s.v[i])),
            ],
            pedestal_position_m: s.pedestal,
            pedal_position_m: s.pedal,
            compression_m: self.compression(s),
            contact_force_n: s.f,
            contact_heat_j: s.heat,
            contact_entries: s.entries,
            limited_unloading_steps: s.limited,
            simultaneous_hammer_felt_steps: s.simultaneous,
            solver_sweeps: s.sweeps,
            maximum_solver_sweeps: s.max_sweeps,
            mechanical_energy_j: energy,
            initial_energy_j: self.initial,
            structural_heat_j: s.structure_heat,
            hammer_return_heat_j: s.hammer_heat,
            arm_heat_j: s.arm_heat,
            pickup_force_work_j: s.pickup_work,
            pedestal_work_j: s.work[0],
            pedal_work_j: s.work[1],
            absolute_drive_work_j: s.absolute_work,
            balance_residual_j: energy
                + s.heat.iter().sum::<f64>()
                + s.structure_heat
                + s.hammer_heat
                + s.arm_heat
                - self.initial
                - s.work.iter().sum::<f64>()
                - s.pickup_work,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(h: f64) -> ActionAssembly {
        ActionAssembly::new(
            h,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            FeltDamperProfile::default(),
            ActionProfile::default(),
        )
        .unwrap()
    }
    #[test]
    fn isolated_hammer_return_matches_analytic_motion_and_static_rest_is_exact() {
        let h = 1e-6;
        let profile = ActionProfile {
            damper_closed_m: -0.001,
            bridle_slack_m: 0.02,
            ..ActionProfile::default()
        };
        let mut v = ActionAssembly::new(
            h,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            FeltDamperProfile::default(),
            profile,
        )
        .unwrap();
        for _ in 0..1000 {
            v.advance(profile.hammer_rest_m, profile.damper_closed_m)
                .unwrap();
        }
        let rest = v.probe();
        assert_eq!(rest.mechanical_energy_j, 0.0);
        assert_eq!(rest.velocity, [0.0; D]);
        assert_eq!(rest.contact_heat_j, [0.0; 4]);
        v.state.q[H] += 0.001;
        v.initial = v.energy(v.state);
        let gamma = profile.hammer_return_n_s_m / (2.0 * v.mass[0]);
        let omega = (profile.hammer_return_n_m / v.mass[0] - gamma * gamma).sqrt();
        for _ in 0..20000 {
            v.advance(profile.hammer_rest_m, profile.damper_closed_m)
                .unwrap();
        }
        let t = 0.02;
        let expected =
            0.001 * (-gamma * t).exp() * ((omega * t).cos() + gamma / omega * (omega * t).sin());
        assert!((v.probe().position[H] - profile.hammer_rest_m - expected).abs() < 1e-12);
        assert_eq!(v.probe().contact_entries, [0; 4]);
        assert!(v.probe().balance_residual_j.abs() < 1e-15);
    }
    #[test]
    fn action_domains_reject_bad_profiles_and_mapping_preserves_endpoints() {
        let p = ActionProfile::default();
        assert_eq!(p.pedestal_position_m(0.0).unwrap(), p.hammer_rest_m);
        assert_eq!(p.pedestal_position_m(1.0).unwrap(), -p.escapement_m);
        assert!(p.pedestal_position_m(f64::NAN).is_err());
        assert!(p.pedal_position_m(-0.1).is_err());
        assert!(
            ActionProfile {
                escapement_m: 0.01,
                hammer_rest_m: -0.005,
                ..p
            }
            .validate()
            .is_err()
        );
        assert!(
            ActionProfile {
                bridle_ratio: f64::INFINITY,
                ..p
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn joint_contacts_match_symmetric_scalar_solution_and_permutation() {
        let laws = core::array::from_fn(|_| RateContact {
            stiffness: 2e8,
            rate: 2e6,
        });
        let a = [1e-5; 4];
        let free = [2e-5; 4];
        let c =
            core::array::from_fn(|i| core::array::from_fn(|j| if i == j { 1e-8 } else { -1e-9 }));
        let (f, _) = solve_joint(&laws, a, free, &c, [0.0; 4], 64).unwrap();
        let exact = laws[0].advance::<false>(a[0], free[0], 7e-9).force;
        for value in f {
            assert!((value - exact).abs() < 1e-12);
        }
        let free = [2e-5, -1e-5, 1e-5, 5e-5];
        let a = [1e-5, -2e-5, 2e-5, 0.0];
        let (f, _) = solve_joint(&laws, a, free, &c, [0.0; 4], 64).unwrap();
        let perm = [3, 0, 2, 1];
        let cp = core::array::from_fn(|i| core::array::from_fn(|j| c[perm[i]][perm[j]]));
        let (g, _) = solve_joint(
            &laws,
            perm.map(|i| a[i]),
            perm.map(|i| free[i]),
            &cp,
            [0.0; 4],
            64,
        )
        .unwrap();
        for i in 0..4 {
            assert!((g[i] - f[perm[i]]).abs() < 1e-12);
        }
        assert!(solve_joint(&laws, a, free, &c, [0.0; 4], 0).is_err());
    }
    #[test]
    fn invalid_drives_and_exhausted_solver_leave_all_state_unchanged() {
        let mut v = voice(1e-6);
        let a = v.probe();
        for (x, r) in [
            (f64::NAN, a.pedal_position_m),
            (-0.0015, a.pedal_position_m),
            (a.pedestal_position_m, f64::INFINITY),
            (a.pedestal_position_m, -0.02),
        ] {
            assert!(v.advance(x, r).is_err());
            assert_eq!(a, v.probe());
        }
        assert!(
            v.advance_with_budget(a.pedestal_position_m, a.pedal_position_m, 0)
                .is_err()
        );
        assert_eq!(a, v.probe());
        v.advance(a.pedestal_position_m, a.pedal_position_m)
            .unwrap();
        assert_ne!(a, v.probe());
    }
    #[test]
    fn reciprocal_bridle_and_simultaneous_impacts_close_all_contact_work() {
        let h = 1e-6;
        let mut v = voice(h);
        // Deliberately misregulated initial geometry: both tine contacts active.
        // This is a stress fixture, not a playable preset or an impulse API.
        v.profile.bridle_slack_m = 0.009;
        v.state.q[H] = 2e-5;
        v.state.v[H] = 0.4;
        v.state.q[Z] = 1e-4;
        v.initial = v.energy(v.state);
        let initial = v.initial;
        let checkpoint = v.probe();
        assert!(
            v.advance_with_budget(
                checkpoint.pedestal_position_m,
                checkpoint.pedal_position_m,
                1
            )
            .is_err()
        );
        assert_eq!(checkpoint, v.probe());
        let mut seen = [false; 4];
        for _ in 0..15000 {
            let a = v.probe();
            v.advance(a.pedestal_position_m, a.pedal_position_m)
                .unwrap();
            let b = v.probe();
            assert!(b.balance_residual_j.abs() / initial < 1e-9);
            assert!(b.mechanical_energy_j <= a.mechanical_energy_j + 1e-12 * initial);
            for (j, touched) in seen.iter_mut().enumerate() {
                *touched |= b.contact_force_n[j] > 0.0;
                let du = v.laws[j].stiffness
                    * (b.compression_m[j].max(0.0).powi(3) - a.compression_m[j].max(0.0).powi(3))
                    / 3.0;
                let heat = b.contact_heat_j[j] - a.contact_heat_j[j];
                assert!(heat >= 0.0);
                assert!(
                    (b.contact_force_n[j] * (b.compression_m[j] - a.compression_m[j]) - du - heat)
                        .abs()
                        / initial
                        < 1e-10
                );
            }
        }
        assert!(seen[0] && seen[1] && seen[3]);
        assert!(v.probe().simultaneous_hammer_felt_steps > 0);
        // The tension port acts on both masses, with the same geometric ratio.
        assert_eq!(v.ports[3][H], v.profile.bridle_ratio);
        assert_eq!(v.ports[3][Z], 1.0);
        for i in 0..4 {
            for j in 0..4 {
                assert!((v.compliance[i][j] - v.compliance[j][i]).abs() < 1e-20);
            }
        }
    }
    #[test]
    fn repeated_key_motion_strikes_without_resetting_the_mechanics() {
        let h = 1e-6;
        let mut v = voice(h);
        let p = ActionProfile::default();
        let mut x = p.hammer_rest_m;
        let mut peaks = [0.0_f64; 2];
        for i in 0..180000 {
            let pressed = (10000..45000).contains(&i) || (95000..130000).contains(&i);
            let target = p.pedestal_position_m(f64::from(pressed)).unwrap();
            x += (target - x).clamp(-1.5 * h, 1.5 * h);
            let a = v.probe();
            v.advance(x, p.damper_closed_m).unwrap();
            let b = v.probe();
            let scale = (b.initial_energy_j + b.absolute_drive_work_j).max(1e-20);
            assert!(b.balance_residual_j.abs() / scale < 1e-8);
            for j in 0..D {
                assert!(
                    (b.position[j] - a.position[j] - 0.5 * h * (a.velocity[j] + b.velocity[j]))
                        .abs()
                        < 1e-15
                );
            }
            peaks[usize::from(i >= 90000)] =
                peaks[usize::from(i >= 90000)].max(b.contact_force_n[0]);
        }
        assert!(peaks[0] > 0.0 && peaks[1] > 0.0, "{peaks:?}");
        assert!(v.probe().contact_entries[0] >= 2);
        assert!(v.probe().contact_heat_j[3] > 0.0);
    }
}
