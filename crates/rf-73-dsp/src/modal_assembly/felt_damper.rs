//! Moving leaf-spring reduction and unilateral felt contact on the full modal port.
//! Offline research: material constants and the prescribed drive are not calibrated.
use super::dissipative_contact::RateContact;
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct FeltDamperProfile {
    pub arm_mass_kg: f64,
    pub arm_stiffness_n_m: f64,
    pub arm_damping_n_s_m: f64,
    /// Quadratic force coefficient: U = k * positive_compression^3 / 3.
    pub felt_stiffness_n_m2: f64,
    pub felt_rate_loss_s_m: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drive_is_continuous_shared_by_key_and_pedal_and_transactional() {
        let mut d = DamperDrive::new(0.0002, 0.01, 1.0).unwrap();
        let old = d.position_m();
        assert_eq!(d.advance(0.0, 1.0, 0.5, 0.001).unwrap(), old);
        assert_eq!(d.advance(1.0, 0.0, 0.5, 0.001).unwrap(), old);
        let next = d.advance(0.0, 0.5, 0.5, 0.001).unwrap();
        assert!((next - old - 0.0005).abs() < 1e-15);
        assert!(d.advance(f64::NAN, 0.0, 0.5, 0.001).is_err());
        assert_eq!(d.position_m(), next);
        for _ in 0..100 {
            d.advance(0.0, 0.5, 0.5, 0.001).unwrap();
        }
        assert!((d.position_m() - (0.0002 - 0.005)).abs() < 1e-15);
        let mut drive = DamperDrive::new(0.0002, 0.01, 1.0).unwrap();
        let mut voice = FeltDamperAssembly::new(
            1e-9,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            FeltDamperProfile::default(),
            drive.position_m(),
        )
        .unwrap();
        // A valid maximum-speed command must survive absolute-position rounding.
        let r = drive.advance(0.0, 0.0, 2.0, 1e-9).unwrap();
        voice.advance(r).unwrap();
        assert!(voice.advance(r + 3e-9).is_err());
    }
    #[test]
    fn isolated_arm_matches_analytic_damped_motion_without_touching_tine() {
        let p = FeltDamperProfile::default();
        let mut v = FeltDamperAssembly::new(
            1e-6,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            p,
            -0.002,
        )
        .unwrap();
        v.state.z += 1e-4;
        v.initial = v.energy(v.state);
        let gamma = p.arm_damping_n_s_m / (2.0 * p.arm_mass_kg);
        let w = (p.arm_stiffness_n_m / p.arm_mass_kg - gamma * gamma).sqrt();
        for _ in 0..20000 {
            v.advance(-0.002).unwrap();
        }
        let t = 0.02;
        let exact = 1e-4 * (-gamma * t).exp() * ((w * t).cos() + gamma / w * (w * t).sin());
        let probe = v.probe();
        assert!((probe.arm_position_m + 0.002 - exact).abs() < 1e-11);
        assert_eq!(probe.position, [0.0; 9]);
        assert_eq!(probe.contact_force_n, 0.0);
        assert!(probe.balance_residual_j.abs() < 1e-14);
    }
    #[test]
    fn moving_felt_closes_each_work_port_and_rejects_invalid_steps_without_mutation() {
        let p = FeltDamperProfile::default();
        let h = 1e-6;
        let mut drive = DamperDrive::new(0.0002, 0.0022, 1.0).unwrap();
        let mut v = FeltDamperAssembly::new(
            h,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            p,
            drive.position_m(),
        )
        .unwrap();
        v.apply_hammer_port_impulse(1e-5).unwrap();
        let mut max_force = 0.0_f64;
        for i in 0..60000 {
            let lift = if (18000..32000).contains(&i) {
                1.0
            } else {
                0.0
            };
            let r = drive.advance(lift, 0.0, 0.5, h).unwrap();
            let before = v.probe();
            v.advance(r).unwrap();
            let after = v.probe();
            max_force = max_force.max(after.contact_force_n);
            let dx = dot(
                v.op.damper,
                core::array::from_fn(|j| after.position[j] - before.position[j]),
            );
            let work = after.contact_force_n * dx;
            let structural = after.structural_energy_j - before.structural_energy_j
                + after.structural_heat_j
                - before.structural_heat_j;
            assert!((structural - work).abs() < 1e-12);
            let du = p.felt_stiffness_n_m2
                * (after.compression_m.max(0.0).powi(3) - before.compression_m.max(0.0).powi(3))
                / 3.0;
            let contact_work =
                after.contact_force_n * (after.arm_position_m - before.arm_position_m - dx);
            assert!((contact_work - du - (after.felt_heat_j - before.felt_heat_j)).abs() < 1e-12);
            assert!(after.balance_residual_j.abs() < 1e-10);
            assert!(
                after.felt_heat_j >= before.felt_heat_j && after.arm_heat_j >= before.arm_heat_j
            );
        }
        assert!(max_force > 0.0 && v.probe().felt_heat_j > 0.0 && v.probe().contact_entries >= 2);
        let before = v.probe();
        for r in [f64::NAN, 0.004, -0.03] {
            assert!(v.advance(r).is_err());
        }
        assert!(v.apply_hammer_port_impulse(f64::INFINITY).is_err());
        assert_eq!(v.probe().position, before.position);
        assert_eq!(v.probe().arm_position_m, before.arm_position_m);
        assert_eq!(v.probe().balance_residual_j, before.balance_residual_j);
    }
    #[test]
    fn handoff_preserves_structure_and_rejects_live_hammer() {
        let mut a = ModalAssembly::new(
            48000.0,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            ModalIntegration::Refined {
                contact_substeps: 32,
            },
        )
        .unwrap();
        a.strike(0.5);
        assert!(
            FeltDamperAssembly::from_released(1e-6, &a, FeltDamperProfile::default(), -0.002)
                .is_err()
        );
        for _ in 0..4000 {
            a.tick();
        }
        let d = FeltDamperAssembly::from_released(1e-6, &a, FeltDamperProfile::default(), -0.002)
            .unwrap();
        assert_eq!(d.probe().position, a.probe().position);
        assert_eq!(d.probe().velocity, a.probe().velocity);
        a.set_damped(true);
        assert!(
            FeltDamperAssembly::from_released(1e-6, &a, FeltDamperProfile::default(), -0.002)
                .is_err()
        );
    }
}
impl Default for FeltDamperProfile {
    fn default() -> Self {
        Self {
            arm_mass_kg: 0.001,
            arm_stiffness_n_m: 200.0,
            arm_damping_n_s_m: 0.5,
            felt_stiffness_n_m2: 2e7,
            felt_rate_loss_s_m: 5.0,
        }
    }
}
impl FeltDamperProfile {
    pub fn validate(self) -> Result<(), ModelError> {
        for (v, lo, hi) in [
            (self.arm_mass_kg, 1e-5, 0.02),
            (self.arm_stiffness_n_m, 1.0, 1e5),
            (self.arm_damping_n_s_m, 0.0, 100.0),
            (self.felt_stiffness_n_m2, 1e4, 1e10),
            (self.felt_rate_loss_s_m, 0.0, 100.0),
        ] {
            bounded(v, lo, hi)?;
        }
        Ok(())
    }
}

/// A prescribed drive, not a simulation of the complete bridle/key linkage.
/// Lift 0 closes the damper; lift 1 retracts it. Key and pedal share max ownership.
#[derive(Clone, Copy, Debug)]
pub struct DamperDrive {
    closed: f64,
    travel: f64,
    position: f64,
}
impl DamperDrive {
    pub fn new(closed_m: f64, travel_m: f64, initial_lift: f64) -> Result<Self, ModelError> {
        bounded(closed_m, -0.005, 0.005)?;
        bounded(travel_m, 0.0001, 0.015)?;
        bounded(initial_lift, 0.0, 1.0)?;
        Ok(Self {
            closed: closed_m,
            travel: travel_m,
            position: closed_m - travel_m * initial_lift,
        })
    }
    pub fn position_m(&self) -> f64 {
        self.position
    }
    pub fn advance(
        &mut self,
        key_lift: f64,
        pedal_lift: f64,
        speed_m_s: f64,
        dt: f64,
    ) -> Result<f64, ModelError> {
        bounded(key_lift, 0.0, 1.0)?;
        bounded(pedal_lift, 0.0, 1.0)?;
        bounded(speed_m_s, 0.0, 2.0)?;
        bounded(dt, 1e-9, 0.01)?;
        let target = self.closed - self.travel * key_lift.max(pedal_lift);
        self.position += (target - self.position).clamp(-speed_m_s * dt, speed_m_s * dt);
        Ok(self.position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FeltDamperProbe {
    pub position: [f64; 9],
    pub velocity: [f64; 9],
    pub pickup_displacement_m: f64,
    pub pickup_velocity_m_s: f64,
    pub arm_position_m: f64,
    pub arm_velocity_m_s: f64,
    pub drive_position_m: f64,
    pub compression_m: f64,
    /// Average nonnegative force during the last tick, not an endpoint force.
    pub contact_force_n: f64,
    pub structural_energy_j: f64,
    pub mechanical_energy_j: f64,
    pub felt_heat_j: f64,
    pub arm_heat_j: f64,
    pub structural_heat_j: f64,
    pub actuator_work_j: f64,
    pub actuator_absolute_work_j: f64,
    pub injected_impulse_energy_j: f64,
    pub initial_energy_j: f64,
    pub balance_residual_j: f64,
    pub contact_entries: u64,
    pub limited_unloading_steps: u64,
}

#[derive(Clone, Copy)]
struct State {
    q: Vector,
    v: Vector,
    z: f64,
    zv: f64,
    r: f64,
    force: f64,
    felt_heat: f64,
    arm_heat: f64,
    structural_heat: f64,
    work: f64,
    absolute_work: f64,
    injected: f64,
    entries: u64,
    limited: u64,
}

/// Nine structural coordinates plus one moving damper mass. Preparation may
/// allocate; advance and impulse use fixed-size storage and commit only valid results.
pub struct FeltDamperAssembly {
    op: Operators,
    step: Midpoint,
    impulse_response: Vector,
    p: FeltDamperProfile,
    h: f64,
    arm_a: f64,
    state: State,
    initial: f64,
}
impl FeltDamperAssembly {
    pub fn new(
        dt: f64,
        g: TineGeometry,
        assembly: ModalAssemblyProfile,
        p: FeltDamperProfile,
        drive_m: f64,
    ) -> Result<Self, ModelError> {
        assembly.validate()?;
        Self::prepare(
            dt,
            Operators::prepare(g, assembly)?,
            p,
            drive_m,
            [0.0; N],
            [0.0; N],
        )
    }
    /// Transfer ringing structure after hammer separation, preserving every modal coordinate.
    /// The prior energy ledger ends at this handoff; current stored energy starts a new ledger.
    pub fn from_released(
        dt: f64,
        assembly: &ModalAssembly,
        p: FeltDamperProfile,
        drive_m: f64,
    ) -> Result<Self, ModelError> {
        if assembly.contact || assembly.damped {
            return Err(ModelError(
                "damper handoff requires separated hammer and lifted legacy damper",
            ));
        }
        Self::prepare(dt, assembly.op.clone(), p, drive_m, assembly.q, assembly.v)
    }
    fn prepare(
        h: f64,
        op: Operators,
        p: FeltDamperProfile,
        r: f64,
        q: Vector,
        v: Vector,
    ) -> Result<Self, ModelError> {
        bounded(h, 1e-9, 1e-4)?;
        bounded(r, -0.02, 0.005)?;
        p.validate()?;
        let step = Midpoint::prepare(op.m, op.k, op.c[0], op.damper, h)?;
        let impulse_response = apply(&numerics::inverse(op.m)?, op.hammer);
        let arm_a =
            p.arm_mass_kg + 0.5 * h * p.arm_damping_n_s_m + 0.25 * h * h * p.arm_stiffness_n_m;
        let state = State {
            q,
            v,
            z: r,
            zv: 0.0,
            r,
            force: 0.0,
            felt_heat: 0.0,
            arm_heat: 0.0,
            structural_heat: 0.0,
            work: 0.0,
            absolute_work: 0.0,
            injected: 0.0,
            entries: 0,
            limited: 0,
        };
        let mut result = Self {
            op,
            step,
            impulse_response,
            p,
            h,
            arm_a,
            state,
            initial: 0.0,
        };
        result.initial = result.energy(state);
        if !result.initial.is_finite() {
            return Err(ModelError("non-finite initial damper energy"));
        }
        Ok(result)
    }
    fn structural_energy(&self, s: State) -> f64 {
        0.5 * (dot(s.q, apply(&self.op.k, s.q)) + dot(s.v, apply(&self.op.m, s.v)))
    }
    fn energy(&self, s: State) -> f64 {
        self.structural_energy(s)
            + 0.5 * self.p.arm_mass_kg * s.zv * s.zv
            + 0.5 * self.p.arm_stiffness_n_m * (s.z - s.r).powi(2)
            + self.p.felt_stiffness_n_m2 * (s.z - dot(self.op.damper, s.q)).max(0.0).powi(3) / 3.0
    }
    pub fn apply_hammer_port_impulse(&mut self, impulse_n_s: f64) -> Result<(), ModelError> {
        bounded(impulse_n_s, -0.01, 0.01)?;
        let mut next = self.state;
        next.v = core::array::from_fn(|i| next.v[i] + impulse_n_s * self.impulse_response[i]);
        let mid = core::array::from_fn(|i| 0.5 * (next.v[i] + self.state.v[i]));
        next.injected += impulse_n_s * dot(self.op.hammer, mid);
        if !self.energy(next).is_finite() || !next.injected.is_finite() {
            return Err(ModelError("invalid damper impulse state"));
        }
        self.state = next;
        Ok(())
    }
    pub fn advance(&mut self, next_drive_m: f64) -> Result<(), ModelError> {
        bounded(next_drive_m, -0.02, 0.005)?;
        let a = self.state;
        let h = self.h;
        let dr = next_drive_m - a.r;
        // Subtracting absolute positions can add a few ulps at tiny valid h.
        let position_roundoff = 8.0 * f64::EPSILON * next_drive_m.abs().max(a.r.abs());
        if dr.abs() > 2.0 * h + position_roundoff {
            return Err(ModelError("damper drive exceeds 2 m/s"));
        }
        let vr = dr / h;
        let rm = 0.5 * (a.r + next_drive_m);
        let k = self.p.arm_stiffness_n_m;
        let c = self.p.arm_damping_n_s_m;
        let zv_free = a.zv
            + (-h * k * (a.z - rm) - h * c * (a.zv - vr) - 0.5 * h * h * k * a.zv) / self.arm_a;
        let z_free = a.z + 0.5 * h * (a.zv + zv_free);
        let vf = self.step.free_velocity(a.q, a.v);
        let qf = core::array::from_fn(|i| a.q[i] + 0.5 * h * (a.v[i] + vf[i]));
        let compression = a.z - dot(self.op.damper, a.q);
        let compliance = 0.5 * h * (h / self.arm_a + dot(self.op.damper, self.step.response));
        let contact = RateContact {
            stiffness: self.p.felt_stiffness_n_m2,
            rate: self.p.felt_rate_loss_s_m / h,
        }
        .advance::<true>(compression, z_free - dot(self.op.damper, qf), compliance);
        let f = contact.force;
        let mut next = a;
        next.v = core::array::from_fn(|i| vf[i] + self.step.response[i] * f);
        next.q = core::array::from_fn(|i| qf[i] + 0.5 * h * self.step.response[i] * f);
        next.zv = zv_free - h / self.arm_a * f;
        next.z = z_free - 0.5 * h * h / self.arm_a * f;
        next.r = next_drive_m;
        next.force = f;
        let vm = core::array::from_fn(|i| 0.5 * (a.v[i] + next.v[i]));
        let vzm = 0.5 * (a.zv + next.zv);
        let spring_force = k * (0.5 * (a.z + next.z) - rm);
        let work = -(spring_force + c * (vzm - vr)) * dr;
        next.work += work;
        next.absolute_work += work.abs();
        next.felt_heat += contact.heat;
        next.arm_heat += h * c * (vzm - vr).powi(2);
        next.structural_heat += h * dot(vm, apply(&self.op.c[0], vm));
        next.entries = next
            .entries
            .saturating_add(u64::from(f > 0.0 && a.force == 0.0));
        next.limited = next.limited.saturating_add(u64::from(contact.limited));
        if next.q.iter().chain(&next.v).any(|x| !x.is_finite())
            || [
                next.z,
                next.zv,
                f,
                next.work,
                next.absolute_work,
                next.felt_heat,
                next.arm_heat,
                next.structural_heat,
                self.energy(next),
            ]
            .iter()
            .any(|x| !x.is_finite())
            || f < 0.0
            || contact.heat < 0.0
        {
            return Err(ModelError("non-finite or nonpassive damper step"));
        }
        self.state = next;
        Ok(())
    }
    pub fn probe(&self) -> FeltDamperProbe {
        let s = self.state;
        let energy = self.energy(s);
        FeltDamperProbe {
            position: s.q,
            velocity: s.v,
            pickup_displacement_m: dot(self.op.pickup, s.q),
            pickup_velocity_m_s: dot(self.op.pickup, s.v),
            arm_position_m: s.z,
            arm_velocity_m_s: s.zv,
            drive_position_m: s.r,
            compression_m: s.z - dot(self.op.damper, s.q),
            contact_force_n: s.force,
            structural_energy_j: self.structural_energy(s),
            mechanical_energy_j: energy,
            felt_heat_j: s.felt_heat,
            arm_heat_j: s.arm_heat,
            structural_heat_j: s.structural_heat,
            actuator_work_j: s.work,
            actuator_absolute_work_j: s.absolute_work,
            injected_impulse_energy_j: s.injected,
            initial_energy_j: self.initial,
            balance_residual_j: energy + s.felt_heat + s.arm_heat + s.structural_heat
                - self.initial
                - s.injected
                - s.work,
            contact_entries: s.entries,
            limited_unloading_steps: s.limited,
        }
    }
}
