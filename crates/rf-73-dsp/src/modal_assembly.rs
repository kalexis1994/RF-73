//! Nine-coordinate research assembly with geometry-derived tine inertia.
//! Coordinates: root translation, root angle, six tine modal amplitudes, bar deflection.
use crate::{ModelError, SAMPLE_RATE_MAX, SAMPLE_RATE_MIN, TineGeometry, TineModes};
use core::f64::consts::TAU;
mod contact;
mod dissipative_contact;
mod memory_coupling;
pub use memory_coupling::{
    MemoryContactInspection, MemoryContactStatus, MemoryContactStep, MemoryModalAssembly,
    MemoryModalCheckpoint, MemoryModalProbe, MemoryModalRk4Step,
};
mod numerics;
mod spectrum;
use numerics::{Free, Midpoint, apply, dot};
pub use spectrum::{ModalSpectrum, StructuralMode};
const N: usize = 9;
type Vector = [f64; N];
type Matrix = [[f64; N]; N];

#[derive(Debug, Clone, Copy)]
pub struct ModalAssemblyProfile {
    /// Additional block/support inertia. Does not include the tine or tonebar.
    pub support_mass_kg: f64,
    pub support_inertia_kg_m2: f64,
    pub translation_stiffness_n_m: f64,
    pub rotation_stiffness_n_m_rad: f64,
    pub translation_damping_n_s_m: f64,
    pub rotation_damping_n_m_s_rad: f64,
    /// Single effective bending coordinate, still a provisional tonebar reduction.
    pub tonebar_mass_kg: f64,
    pub tonebar_arm_m: f64,
    pub tonebar_frequency_hz: f64,
    pub tonebar_decay_seconds: f64,
    pub tine_decay_seconds: [f64; 6],
    pub damper_position: f64,
    pub damper_n_s_m: f64,
    pub hammer_mass_kg: f64,
    pub contact_stiffness_n_m2: f64,
    /// Hunt-Crossley-type rate loss beta, seconds/meter. Zero preserves elastic contact.
    pub contact_damping_s_m: f64,
    pub maximum_hammer_speed_m_s: f64,
}
impl Default for ModalAssemblyProfile {
    fn default() -> Self {
        Self {
            support_mass_kg: 0.03,
            support_inertia_kg_m2: 1e-5,
            translation_stiffness_n_m: 0.03 * (TAU * 80.0).powi(2),
            rotation_stiffness_n_m_rad: 1e-5 * (TAU * 100.0).powi(2),
            translation_damping_n_s_m: 5.0,
            rotation_damping_n_m_s_rad: 0.002,
            tonebar_mass_kg: 0.015,
            tonebar_arm_m: 0.08,
            tonebar_frequency_hz: 190.0,
            tonebar_decay_seconds: 8.0,
            tine_decay_seconds: [5.0, 0.16, 0.055, 0.035, 0.025, 0.02],
            damper_position: 0.8,
            damper_n_s_m: 0.2,
            hammer_mass_kg: 0.004,
            contact_stiffness_n_m2: 4e10,
            contact_damping_s_m: 0.0,
            maximum_hammer_speed_m_s: 0.8,
        }
    }
}
impl ModalAssemblyProfile {
    pub fn validate(self) -> Result<(), ModelError> {
        for (value, low, high) in [
            (self.support_mass_kg, 0.001, 1.0),
            (self.support_inertia_kg_m2, 1e-8, 0.01),
            (self.translation_stiffness_n_m, 0.0, 1e7),
            (self.rotation_stiffness_n_m_rad, 0.0, 1e5),
            (self.translation_damping_n_s_m, 0.0, 1000.0),
            (self.rotation_damping_n_m_s_rad, 0.0, 10.0),
            (self.tonebar_mass_kg, 0.001, 0.1),
            (self.tonebar_arm_m, -0.3, 0.3),
            (self.tonebar_frequency_hz, 20.0, 5000.0),
            (self.tonebar_decay_seconds, 0.01, 30.0),
            (self.damper_position, 0.0, 1.0),
            (self.damper_n_s_m, 0.0, 1000.0),
            (self.hammer_mass_kg, 0.001, 0.02),
            (self.contact_stiffness_n_m2, 1e8, 1e12),
            (self.contact_damping_s_m, 0.0, 10.0),
            (self.maximum_hammer_speed_m_s, 0.1, 3.0),
        ] {
            bounded(value, low, high)?;
        }
        for value in self.tine_decay_seconds {
            bounded(value, 0.001, 30.0)?;
        }
        Ok(())
    }
}
fn bounded(v: f64, low: f64, high: f64) -> Result<(), ModelError> {
    if !v.is_finite() || !(low..=high).contains(&v) {
        Err(ModelError(
            "modal assembly parameter outside its finite domain",
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ModalIntegration {
    /// Uniform midpoint reference, power of two from 4 to 1024 steps/output frame.
    Midpoint { steps_per_sample: usize },
    /// Four base ticks/output frame; 1..256 power-of-two contact subdivisions.
    Refined { contact_substeps: usize },
}
#[derive(Debug, Clone, Copy)]
pub struct ModalProbe {
    /// Index 1 is radians/radians per second; all other coordinates are meters.
    pub position: Vector,
    pub velocity: Vector,
    pub pickup_displacement_m: f64,
    pub pickup_velocity_m_s: f64,
    pub contact_force_n: f64,
    pub contact_active: bool,
    pub mechanical_energy_j: f64,
    pub dissipated_energy_j: f64,
    /// Contact material heat, already included in dissipated_energy_j.
    pub contact_dissipated_energy_j: f64,
    /// Subset of contact heat from unloading steps with the nonadhesive limit active.
    pub contact_limited_heat_j: f64,
    /// Contact microsteps where the nonadhesive unloading limit was active.
    pub contact_limit_steps: u64,
    pub escaped_hammer_energy_j: f64,
    pub injected_energy_j: f64,
    pub balance_residual_j: f64,
}

#[derive(Clone)]
struct Operators {
    m: Matrix,
    k: Matrix,
    diagonal_stiffness: bool,
    diagonal_damping: [bool; 2],
    c: [Matrix; 2],
    hammer: Vector,
    pickup: Vector,
}
impl Operators {
    fn prepare(g: TineGeometry, p: ModalAssemblyProfile) -> Result<Self, ModelError> {
        let basis = TineModes::prepare(g, 64)?;
        let mut m = [[0.0; N]; N];
        let mut k = [[0.0; N]; N];
        let mut c = [[0.0; N]; N];
        let tine = basis.moving_root_mass_matrix();
        for i in 0..8 {
            for j in 0..8 {
                m[i][j] = tine[i][j];
            }
        }
        m[0][0] += p.support_mass_kg;
        m[1][1] += p.support_inertia_kg_m2;
        let bar = core::array::from_fn(|i| match i {
            0 => 1.0,
            1 => p.tonebar_arm_m,
            8 => 1.0,
            _ => 0.0,
        });
        add_outer(&mut m, bar, p.tonebar_mass_kg);
        k[0][0] = p.translation_stiffness_n_m;
        k[1][1] = p.rotation_stiffness_n_m_rad;
        c[0][0] = p.translation_damping_n_s_m;
        c[1][1] = p.rotation_damping_n_m_s_rad;
        k[8][8] = p.tonebar_mass_kg * (TAU * p.tonebar_frequency_hz).powi(2);
        c[8][8] = 2.0 * p.tonebar_mass_kg * 1000.0_f64.ln() / p.tonebar_decay_seconds;
        for (i, mode) in basis.modes.iter().enumerate() {
            k[i + 2][i + 2] = mode.effective_mass_kg * (TAU * mode.frequency_hz).powi(2);
            c[i + 2][i + 2] =
                2.0 * mode.effective_mass_kg * 1000.0_f64.ln() / p.tine_decay_seconds[i];
        }
        let port = |position: f64| -> Result<Vector, ModelError> {
            let mut b = [0.0; N];
            b[0] = 1.0;
            b[1] = g.length_m * position;
            for i in 0..6 {
                b[i + 2] = basis.shape(i, position)?;
            }
            Ok(b)
        };
        let hammer = port(g.hammer_position)?;
        let pickup = port(g.pickup_position)?;
        let mut damped = c;
        add_outer(&mut damped, port(p.damper_position)?, p.damper_n_s_m);
        numerics::inverse(m)?;
        Ok(Self {
            m,
            diagonal_stiffness: is_diagonal(&k),
            diagonal_damping: [is_diagonal(&c), is_diagonal(&damped)],
            k,
            c: [c, damped],
            hammer,
            pickup,
        })
    }
    fn stiffness_force(&self, q: Vector) -> Vector {
        if self.diagonal_stiffness {
            // Preserve the dense row sum's initial +0, including signed-zero inputs.
            core::array::from_fn(|i| 0.0 + self.k[i][i] * q[i])
        } else {
            apply(&self.k, q)
        }
    }
    fn damping_force(&self, v: Vector, damped: bool) -> Vector {
        let index = usize::from(damped);
        if self.diagonal_damping[index] {
            // Match the dense row's +0 accumulator for finite inputs.
            core::array::from_fn(|i| 0.0 + self.c[index][i][i] * v[i])
        } else {
            apply(&self.c[index], v)
        }
    }
}
fn is_diagonal(matrix: &Matrix) -> bool {
    matrix.iter().enumerate().all(|(i, row)| {
        row.iter()
            .enumerate()
            .all(|(j, value)| i == j || *value == 0.0)
    })
}
fn add_outer(a: &mut Matrix, b: Vector, weight: f64) {
    for i in 0..N {
        for j in 0..N {
            a[i][j] += weight * b[i] * b[j];
        }
    }
}
struct FreeSteps {
    base: [Free; 2],
    remainder: [Free; 2],
}

/// Offline time-domain assembly, no pickup voltage/filter or plugin integration.
/// Construction may allocate for modal preparation. Tick/strike/reset allocate nothing.
pub struct ModalAssembly {
    p: ModalAssemblyProfile,
    op: Operators,
    dt: f64,
    contact_steps: usize,
    steps_per_sample: usize,
    midpoint: [Midpoint; 2],
    free: Option<FreeSteps>,
    q: Vector,
    v: Vector,
    hx: f64,
    hv: f64,
    contact: bool,
    damped: bool,
    force: f64,
    dissipated: f64,
    contact_heat: f64,
    contact_limited_heat: f64,
    contact_limit_steps: u64,
    escaped: f64,
    injected: f64,
}
impl ModalAssembly {
    pub fn new(
        rate: f64,
        g: TineGeometry,
        p: ModalAssemblyProfile,
        integration: ModalIntegration,
    ) -> Result<Self, ModelError> {
        bounded(rate, SAMPLE_RATE_MIN, SAMPLE_RATE_MAX)?;
        g.validate()?;
        p.validate()?;
        let (steps_per_sample, contact_steps, refined) = match integration {
            ModalIntegration::Midpoint { steps_per_sample } => {
                if !(4..=1024).contains(&steps_per_sample) || !steps_per_sample.is_power_of_two() {
                    return Err(ModelError(
                        "modal midpoint steps must be a power of two from 4 to 1024",
                    ));
                }
                (steps_per_sample, 1, false)
            }
            ModalIntegration::Refined { contact_substeps } => {
                if !(1..=256).contains(&contact_substeps) || !contact_substeps.is_power_of_two() {
                    return Err(ModelError(
                        "modal contact steps must be a power of two from 1 to 256",
                    ));
                }
                (4, contact_substeps, true)
            }
        };
        let op = Operators::prepare(g, p)?;
        let dt = 1.0 / (rate * steps_per_sample as f64);
        let h = dt / contact_steps as f64;
        let midpoint = [
            Midpoint::prepare(op.m, op.k, op.c[0], op.hammer, h)?,
            Midpoint::prepare(op.m, op.k, op.c[1], op.hammer, h)?,
        ];
        let free = if refined {
            Some(FreeSteps {
                base: [
                    Free::prepare(op.m, op.k, op.c[0], dt)?,
                    Free::prepare(op.m, op.k, op.c[1], dt)?,
                ],
                remainder: [
                    Free::prepare(op.m, op.k, op.c[0], h)?,
                    Free::prepare(op.m, op.k, op.c[1], h)?,
                ],
            })
        } else {
            None
        };
        Ok(Self {
            p,
            op,
            dt,
            contact_steps,
            steps_per_sample,
            midpoint,
            free,
            q: [0.0; N],
            v: [0.0; N],
            hx: 0.0,
            hv: 0.0,
            contact: false,
            damped: false,
            force: 0.0,
            dissipated: 0.0,
            contact_heat: 0.0,
            contact_limited_heat: 0.0,
            contact_limit_steps: 0,
            escaped: 0.0,
            injected: 0.0,
        })
    }
    pub fn steps_per_sample(&self) -> usize {
        self.steps_per_sample
    }
    /// Research metric/inspection: immutable physical inertia, including radian coordinates.
    pub fn mass_matrix(&self) -> [[f64; 9]; 9] {
        self.op.m
    }
    pub fn strike(&mut self, velocity: f64) -> bool {
        if self.contact || !velocity.is_finite() || velocity <= 0.0 || velocity > 1.0 {
            return false;
        }
        self.hx = dot(self.op.hammer, self.q);
        self.hv = self.p.maximum_hammer_speed_m_s * velocity.powf(1.4);
        self.injected += 0.5 * self.p.hammer_mass_kg * self.hv.powi(2);
        self.contact = true;
        self.damped = false;
        true
    }
    pub fn set_damped(&mut self, damped: bool) {
        self.damped = damped;
    }
    pub fn tick(&mut self) {
        self.tick_with_solver::<true>();
    }
    fn tick_with_solver<const FAST: bool>(&mut self) {
        if self.free.is_none() {
            self.advance_midpoint::<false>(self.dt);
        } else if !self.contact {
            self.advance_free(false);
            self.force = 0.0;
        } else {
            let h = self.dt / self.contact_steps as f64;
            let mut force = 0.0;
            for _ in 0..self.contact_steps {
                if self.contact {
                    self.advance_midpoint::<FAST>(h);
                    force += self.force;
                } else {
                    self.advance_free(true);
                }
            }
            self.force = force / self.contact_steps as f64;
        }
    }
    fn advance_free(&mut self, remainder: bool) {
        let free = self.free.as_ref().expect("prepared modal free transition");
        let steps = if remainder {
            &free.remainder
        } else {
            &free.base
        };
        let (q, v, loss) = steps[usize::from(self.damped)].advance(self.q, self.v);
        self.q = q;
        self.v = v;
        self.dissipated += loss;
    }
    fn advance_midpoint<const FAST: bool>(&mut self, h: f64) {
        let step = &self.midpoint[usize::from(self.damped)];
        let old_v = self.v;
        let fv = step.free_velocity(self.q, self.v);
        let fq: Vector = core::array::from_fn(|i| self.q[i] + 0.5 * h * (self.v[i] + fv[i]));
        self.force = 0.0;
        if self.contact {
            let a = self.hx - dot(self.op.hammer, self.q);
            let dfree = self.hx + h * self.hv - dot(self.op.hammer, fq);
            let compliance = h * h / (2.0 * self.p.hammer_mass_kg)
                + 0.5 * h * dot(self.op.hammer, step.response);
            let step = dissipative_contact::RateContact {
                stiffness: self.p.contact_stiffness_n_m2,
                rate: self.p.contact_damping_s_m / h,
            }
            .advance::<FAST>(a, dfree, compliance);
            self.force = step.force;
            self.contact_heat += step.heat;
            if step.limited {
                self.contact_limited_heat += step.heat;
            }
            self.dissipated += step.heat;
            self.contact_limit_steps = self
                .contact_limit_steps
                .saturating_add(u64::from(step.limited));
            self.hx += h * self.hv - 0.5 * h * h * self.force / self.p.hammer_mass_kg;
            self.hv -= h * self.force / self.p.hammer_mass_kg;
        }
        self.v = core::array::from_fn(|i| fv[i] + step.response[i] * self.force);
        self.q = core::array::from_fn(|i| fq[i] + 0.5 * h * step.response[i] * self.force);
        let mid = core::array::from_fn(|i| 0.5 * (old_v[i] + self.v[i]));
        self.dissipated += h * dot(mid, apply(&self.op.c[usize::from(self.damped)], mid));
        if self.contact
            && self.hx <= dot(self.op.hammer, self.q)
            && self.hv <= dot(self.op.hammer, self.v)
        {
            self.escaped += 0.5 * self.p.hammer_mass_kg * self.hv.powi(2);
            self.contact = false;
            self.hx = 0.0;
            self.hv = 0.0;
        }
    }
    pub fn probe(&self) -> ModalProbe {
        let mut energy =
            0.5 * (dot(self.q, apply(&self.op.k, self.q)) + dot(self.v, apply(&self.op.m, self.v)));
        if self.contact {
            energy += 0.5 * self.p.hammer_mass_kg * self.hv.powi(2)
                + self.p.contact_stiffness_n_m2
                    * (self.hx - dot(self.op.hammer, self.q)).max(0.0).powi(3)
                    / 3.0;
        }
        ModalProbe {
            position: self.q,
            velocity: self.v,
            pickup_displacement_m: dot(self.op.pickup, self.q),
            pickup_velocity_m_s: dot(self.op.pickup, self.v),
            contact_force_n: self.force,
            contact_active: self.contact,
            mechanical_energy_j: energy,
            dissipated_energy_j: self.dissipated,
            contact_dissipated_energy_j: self.contact_heat,
            contact_limited_heat_j: self.contact_limited_heat,
            contact_limit_steps: self.contact_limit_steps,
            escaped_hammer_energy_j: self.escaped,
            injected_energy_j: self.injected,
            balance_residual_j: energy + self.dissipated + self.escaped - self.injected,
        }
    }
    pub fn reset(&mut self) {
        self.q = [0.0; N];
        self.v = [0.0; N];
        self.hx = 0.0;
        self.hv = 0.0;
        self.contact = false;
        self.damped = false;
        self.force = 0.0;
        self.dissipated = 0.0;
        self.contact_heat = 0.0;
        self.contact_limited_heat = 0.0;
        self.contact_limit_steps = 0;
        self.escaped = 0.0;
        self.injected = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_tuning_span_closes_coupled_contact_and_release_energy_budget() {
        let g = TineGeometry {
            length_m: 0.07,
            tuning_position: 0.796,
            tuning_span_m: 0.006,
            ..TineGeometry::default()
        };
        let mut v = ModalAssembly::new(
            48000.0,
            g,
            ModalAssemblyProfile::default(),
            ModalIntegration::Refined {
                contact_substeps: 32,
            },
        )
        .unwrap();
        assert!(v.strike(0.5));
        for i in 0..4800 {
            if i == 2400 {
                v.set_damped(true);
            }
            let before = v.probe();
            v.tick();
            let after = v.probe();
            assert!(after.balance_residual_j.abs() < after.injected_energy_j * 1e-8);
            assert!(
                after.mechanical_energy_j
                    <= before.mechanical_energy_j + after.injected_energy_j * 1e-10
            );
            assert!(
                after.dissipated_energy_j
                    >= before.dissipated_energy_j - after.injected_energy_j * 1e-12
            );
            assert!(after.pickup_velocity_m_s.is_finite());
        }
        assert!(!v.contact && v.escaped > 0.0 && v.dissipated > 0.0);
    }
    #[test]
    fn damping_products_preserve_dense_arithmetic_and_nonzero_coupling() {
        for length in [0.05, 0.075, 0.12] {
            for damper in [0.0, 0.2] {
                let mut op = Operators::prepare(
                    TineGeometry {
                        length_m: length,
                        ..TineGeometry::default()
                    },
                    ModalAssemblyProfile {
                        damper_n_s_m: damper,
                        ..ModalAssemblyProfile::default()
                    },
                )
                .unwrap();
                assert_eq!(op.diagonal_damping, [true, damper == 0.0]);
                for damped in [false, true] {
                    for scale in [0.0, -0.0, 1e-100, -1e-5, 1e100] {
                        let v = core::array::from_fn(|i| scale * (i as f64 - 4.0));
                        assert_eq!(
                            op.damping_force(v, damped).map(f64::to_bits),
                            apply(&op.c[usize::from(damped)], v).map(f64::to_bits)
                        );
                    }
                }
                // Keep even arbitrarily small off-diagonal damping, in either state.
                for damped in [false, true] {
                    let index = usize::from(damped);
                    op.c[index] = [[0.0; N]; N];
                    op.c[index][0][1] = 1e-300;
                    op.c[index][1][0] = 1e-300;
                    op.diagonal_damping[index] = is_diagonal(&op.c[index]);
                    assert!(!op.diagonal_damping[index]);
                    let mut v = [0.0; N];
                    v[1] = 2.0;
                    assert_eq!(op.damping_force(v, damped), apply(&op.c[index], v));
                    assert_eq!(op.damping_force(v, damped)[0], 2e-300);
                }
            }
        }
    }
    #[test]
    fn prepared_stiffness_preserves_dense_products_and_retains_off_diagonal_coupling() {
        for length in [0.05, 0.075, 0.12] {
            let mut op = Operators::prepare(
                TineGeometry {
                    length_m: length,
                    ..TineGeometry::default()
                },
                ModalAssemblyProfile::default(),
            )
            .unwrap();
            assert!(op.diagonal_stiffness);
            for scale in [0.0, -0.0, 1e-100, -1e-5, 1e100] {
                let q = core::array::from_fn(|i| scale * (i as f64 - 4.0));
                assert_eq!(
                    op.stiffness_force(q).map(f64::to_bits),
                    apply(&op.k, q).map(f64::to_bits)
                );
            }
            // Classify after building a coupled matrix, just as preparation does.
            op.k = [[0.0; N]; N];
            op.k[0][1] = 1e-300;
            op.k[1][0] = 1e-300;
            op.diagonal_stiffness = is_diagonal(&op.k);
            assert!(!op.diagonal_stiffness, "no approximate sparsity threshold");
            let mut q = [0.0; N];
            q[1] = 2.0;
            assert_eq!(op.stiffness_force(q), apply(&op.k, q));
            assert_eq!(op.stiffness_force(q)[0], 2e-300);
        }
    }
    fn refined() -> ModalIntegration {
        ModalIntegration::Refined {
            contact_substeps: 32,
        }
    }
    fn voice() -> ModalAssembly {
        ModalAssembly::new(
            44100.0,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            refined(),
        )
        .unwrap()
    }

    #[test]
    fn inertia_counts_each_component_once_and_force_ports_obey_virtual_work() {
        let g = TineGeometry::default();
        let p = ModalAssemblyProfile::default();
        let op = Operators::prepare(g, p).unwrap();
        assert!(
            (op.m[0][0]
                - g.beam_mass_kg()
                - g.tuning_mass_kg
                - p.support_mass_kg
                - p.tonebar_mass_kg)
                .abs()
                < 1e-16
        );
        assert_eq!(op.hammer[1], g.length_m * g.hammer_position);
        assert_eq!(op.pickup[1], g.length_m * g.pickup_position);
        for i in 0..N {
            for j in 0..N {
                assert_eq!(op.m[i][j], op.m[j][i]);
            }
        }
        let velocity = [0.2, -0.03, 0.01, -0.05, 0.003, 0.06, -0.01, 0.001, 0.04];
        let force = 3.0;
        assert!(
            (dot(op.hammer.map(|b| b * force), velocity) - force * dot(op.hammer, velocity)).abs()
                < 1e-14
        );
        assert!(dot(velocity, apply(&op.m, velocity)) > 0.0);
    }

    #[test]
    fn free_rigid_translation_and_rotation_follow_the_continuous_nullspace() {
        let p = ModalAssemblyProfile {
            translation_stiffness_n_m: 0.0,
            rotation_stiffness_n_m_rad: 0.0,
            translation_damping_n_s_m: 0.0,
            rotation_damping_n_m_s_rad: 0.0,
            ..ModalAssemblyProfile::default()
        };
        let mut v = ModalAssembly::new(44100.0, TineGeometry::default(), p, refined()).unwrap();
        v.v[0] = 0.01;
        v.v[1] = 0.03;
        let initial = v.probe().mechanical_energy_j;
        for i in 1..=10000 {
            v.tick();
            assert!((v.q[0] - 0.01 * i as f64 * v.dt).abs() < 1e-11);
            assert!((v.q[1] - 0.03 * i as f64 * v.dt).abs() < 1e-10);
            assert!((v.probe().mechanical_energy_j / initial - 1.0).abs() < 1e-8);
            for value in &v.q[2..] {
                assert!(value.abs() < 1e-10);
            }
        }
    }

    #[test]
    fn hammer_and_tonebar_impulses_have_reciprocal_displacement_responses() {
        let mut a = voice();
        let mut b = voice();
        let bar = core::array::from_fn(|i| if i == 8 { 1.0 } else { 0.0 });
        let inverse = numerics::inverse(a.op.m).unwrap();
        a.v = apply(&inverse, a.op.hammer).map(|v| v * 1e-5);
        b.v = apply(&inverse, bar).map(|v| v * 1e-5);
        let mut response = 0.0_f64;
        for _ in 0..5000 {
            a.tick();
            b.tick();
            response = response.max(a.q[8].abs());
            assert!((dot(bar, a.q) - dot(b.op.hammer, b.q)).abs() < 1e-12);
        }
        assert!(response > 1e-9);
    }

    #[test]
    fn free_transition_composes_and_dissipation_matches_independent_energy() {
        let mut v = voice();
        v.q = [1e-5, 2e-4, 1e-6, -1e-6, 2e-6, -2e-6, 1e-6, 1e-6, 1e-5];
        v.v = [0.01, 0.1, -0.03, 0.01, -0.01, 0.02, -0.005, 0.005, 0.01];
        let half = Free::prepare(v.op.m, v.op.k, v.op.c[0], v.dt / 2.0).unwrap();
        for _ in 0..100 {
            let before = v.probe().mechanical_energy_j;
            let (q1, v1, l1) = half.advance(v.q, v.v);
            let (q2, v2, l2) = half.advance(q1, v1);
            let old_loss = v.dissipated;
            v.tick();
            for i in 0..N {
                assert!((v.q[i] - q2[i]).abs() < 1e-12);
                assert!((v.v[i] - v2[i]).abs() < 1e-9);
            }
            assert!(((v.dissipated - old_loss) - (l1 + l2)).abs() < before * 1e-10);
            assert!((v.probe().mechanical_energy_j + (l1 + l2) - before).abs() < before * 1e-10);
        }
    }

    #[test]
    fn hammer_release_restrike_and_spatial_damper_close_the_energy_budget() {
        let mut v = voice();
        assert!(v.strike(1.0));
        assert!(!v.strike(0.9));
        let mut bar_motion = 0.0_f64;
        let mut rotation = 0.0_f64;
        let mut high_mode = 0.0_f64;
        for i in 0..20000 {
            if i == 5000 || i == 15000 {
                v.set_damped(true);
            }
            if i == 10000 {
                let before = v.q;
                assert!(v.strike(0.5));
                assert_eq!(before, v.q);
            }
            let before = v.probe();
            v.tick();
            let after = v.probe();
            assert!(
                after.mechanical_energy_j
                    <= before.mechanical_energy_j + after.injected_energy_j * 1e-10
            );
            assert!(
                after.balance_residual_j.abs() < after.injected_energy_j * 1e-8,
                "{after:?}"
            );
            assert!(
                after.dissipated_energy_j
                    >= before.dissipated_energy_j - after.injected_energy_j * 1e-12
            );
            bar_motion = bar_motion.max(v.q[8].abs());
            rotation = rotation.max(v.q[1].abs());
            high_mode = high_mode.max(v.q[7].abs());
        }
        assert!(!v.contact);
        assert!(v.escaped > 0.0);
        assert!(v.dissipated > 0.0);
        assert!(bar_motion > 1e-9);
        assert!(rotation > 1e-8);
        assert!(high_mode > 1e-10);
        v.reset();
        assert_eq!(v.probe().mechanical_energy_j, 0.0);
        assert_eq!(v.probe().balance_residual_j, 0.0);
    }

    #[test]
    fn folded_free_motion_matches_normalized_trajectories_and_independent_work() {
        for length in [0.05, 0.075, 0.12] {
            for light_support in [false, true] {
                let p = if light_support {
                    ModalAssemblyProfile {
                        support_mass_kg: 0.001,
                        support_inertia_kg_m2: 1e-8,
                        tonebar_arm_m: -0.08,
                        damper_n_s_m: 2.0,
                        ..ModalAssemblyProfile::default()
                    }
                } else {
                    ModalAssemblyProfile::default()
                };
                let op = Operators::prepare(
                    TineGeometry {
                        length_m: length,
                        ..TineGeometry::default()
                    },
                    p,
                )
                .unwrap();
                for rate in [44100.0, 192000.0] {
                    for c in op.c {
                        for substeps in [1.0, 32.0] {
                            let free = Free::prepare(op.m, op.k, c, 1.0 / (4.0 * rate * substeps))
                                .unwrap();
                            let mut q = [1e-5, 2e-4, 1e-6, -1e-6, 2e-6, -2e-6, 1e-6, 1e-6, 1e-5];
                            let mut v = [0.01, 0.1, -0.03, 0.01, -0.01, 0.02, -0.005, 0.005, 0.01];
                            let (mut qr, mut vr) = (q, v);
                            let initial = 0.5 * (dot(q, apply(&op.k, q)) + dot(v, apply(&op.m, v)));
                            let (mut work, mut reference_work) = (0.0, 0.0);
                            for _ in 0..1000 {
                                let (qn, vn, loss) = free.advance(q, v);
                                let (qrn, vrn, reference_loss) = free.advance_normalized(qr, vr);
                                (q, v, qr, vr) = (qn, vn, qrn, vrn);
                                work += loss;
                                reference_work += reference_loss;
                                let dq = core::array::from_fn(|i| q[i] - qr[i]);
                                let dv = core::array::from_fn(|i| v[i] - vr[i]);
                                let error_energy =
                                    dot(dq, apply(&op.k, dq)) + dot(dv, apply(&op.m, dv));
                                assert!(error_energy.is_finite() && error_energy < initial * 1e-16);
                                assert!(loss.is_finite() && loss >= -initial * 1e-14);
                                assert!((work - reference_work).abs() < initial * 1e-10);
                                let energy =
                                    0.5 * (dot(q, apply(&op.k, q)) + dot(v, apply(&op.m, v)));
                                assert!((energy + work - initial).abs() < initial * 1e-8);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn folded_lossless_motion_conserves_energy_without_manufactured_dissipation() {
        let op =
            Operators::prepare(TineGeometry::default(), ModalAssemblyProfile::default()).unwrap();
        let free = Free::prepare(op.m, op.k, [[0.0; N]; N], 1.0 / 176400.0).unwrap();
        let mut q = [0.0; N];
        let mut v = apply(&numerics::inverse(op.m).unwrap(), op.hammer).map(|x| x * 1e-5);
        let initial = 0.5 * dot(v, apply(&op.m, v));
        for _ in 0..20000 {
            let (qn, vn, loss) = free.advance(q, v);
            (q, v) = (qn, vn);
            assert_eq!(loss, 0.0);
            let energy = 0.5 * (dot(q, apply(&op.k, q)) + dot(v, apply(&op.m, v)));
            assert!((energy / initial - 1.0).abs() < 1e-8);
        }
    }

    #[test]
    fn accelerated_contact_matches_bisection_through_damping_and_restrikes() {
        for rate in [44100.0, 192000.0] {
            for length in [0.05, 0.075, 0.12] {
                for strong in [false, true] {
                    let p = if strong {
                        ModalAssemblyProfile {
                            contact_stiffness_n_m2: 1e12,
                            maximum_hammer_speed_m_s: 3.0,
                            support_mass_kg: 0.001,
                            damper_n_s_m: 10.0,
                            ..ModalAssemblyProfile::default()
                        }
                    } else {
                        ModalAssemblyProfile::default()
                    };
                    let g = TineGeometry {
                        length_m: length,
                        ..TineGeometry::default()
                    };
                    let mut fast = ModalAssembly::new(rate, g, p, refined()).unwrap();
                    let mut reference = ModalAssembly::new(rate, g, p, refined()).unwrap();
                    let (mut force_error, mut force_energy) = (0.0, 0.0);
                    for tick in 0..6000 {
                        if tick % 3000 == 0 {
                            let velocity = if strong { 1.0 } else { 0.1 };
                            assert!(fast.strike(velocity));
                            assert!(reference.strike(velocity));
                        }
                        if tick % 3000 == 1500 {
                            fast.set_damped(true);
                            reference.set_damped(true);
                        }
                        fast.tick();
                        reference.tick_with_solver::<false>();
                        let p = fast.probe();
                        let r = reference.probe();
                        assert_eq!(p.contact_active, r.contact_active);
                        let dq = core::array::from_fn(|i| p.position[i] - r.position[i]);
                        let dv = core::array::from_fn(|i| p.velocity[i] - r.velocity[i]);
                        let error = dot(dq, apply(&fast.op.k, dq)) + dot(dv, apply(&fast.op.m, dv));
                        assert!(error.is_finite() && error < p.injected_energy_j * 1e-16);
                        assert!(p.balance_residual_j.abs() < p.injected_energy_j * 1e-8);
                        assert!(r.balance_residual_j.abs() < r.injected_energy_j * 1e-8);
                        force_error += (p.contact_force_n - r.contact_force_n).powi(2);
                        force_energy += r.contact_force_n.powi(2);
                    }
                    assert!(force_energy > 0.0 && force_error / force_energy < 1e-18);
                    assert!(!fast.probe().contact_active);
                }
            }
        }
    }

    #[test]
    fn material_loss_survives_restrikes_damping_and_bounded_reference_comparison() {
        for beta in [0.5, 2.0, 10.0] {
            for rate in [44100.0, 192000.0] {
                let p = ModalAssemblyProfile {
                    contact_damping_s_m: beta,
                    ..ModalAssemblyProfile::default()
                };
                let mut fast =
                    ModalAssembly::new(rate, TineGeometry::default(), p, refined()).unwrap();
                let mut reference =
                    ModalAssembly::new(rate, TineGeometry::default(), p, refined()).unwrap();
                for tick in 0..12000 {
                    if tick % 6000 == 0 {
                        assert!(fast.strike(1.0));
                        assert!(reference.strike(1.0));
                    }
                    if tick % 6000 == 3000 {
                        fast.set_damped(true);
                        reference.set_damped(true);
                    }
                    let before = fast.probe();
                    fast.tick();
                    reference.tick_with_solver::<false>();
                    let after = fast.probe();
                    let r = reference.probe();
                    assert!(after.contact_force_n >= 0.0);
                    assert!(
                        after.contact_limited_heat_j >= 0.0
                            && after.contact_limited_heat_j <= after.contact_dissipated_energy_j
                    );
                    assert!(
                        after.contact_dissipated_energy_j >= before.contact_dissipated_energy_j
                    );
                    assert!(
                        after.mechanical_energy_j
                            <= before.mechanical_energy_j + after.injected_energy_j * 1e-10
                    );
                    assert!(after.balance_residual_j.abs() < after.injected_energy_j * 1e-8);
                    let dq = core::array::from_fn(|i| after.position[i] - r.position[i]);
                    let dv = core::array::from_fn(|i| after.velocity[i] - r.velocity[i]);
                    assert!(
                        dot(dq, apply(&fast.op.k, dq)) + dot(dv, apply(&fast.op.m, dv))
                            < after.injected_energy_j * 1e-16
                    );
                }
                assert!(!fast.probe().contact_active);
                assert!(fast.probe().contact_dissipated_energy_j > 0.0);
                assert!(
                    fast.probe().contact_dissipated_energy_j < fast.probe().dissipated_energy_j
                );
                fast.reset();
                assert_eq!(fast.probe().contact_dissipated_energy_j, 0.0);
                assert_eq!(fast.probe().contact_limited_heat_j, 0.0);
                assert_eq!(fast.probe().contact_limit_steps, 0);
                assert_eq!(fast.probe().balance_residual_j, 0.0);
            }
        }
        for beta in [-1.0, 10.1, f64::INFINITY, f64::NAN] {
            let p = ModalAssemblyProfile {
                contact_damping_s_m: beta,
                ..ModalAssemblyProfile::default()
            };
            assert!(ModalAssembly::new(44100.0, TineGeometry::default(), p, refined()).is_err());
        }
    }

    #[test]
    fn malformed_inputs_are_rejected_before_stepping() {
        let p = ModalAssemblyProfile::default();
        let g = TineGeometry::default();
        for rate in [f64::NAN, 0.0, 192001.0] {
            assert!(ModalAssembly::new(rate, g, p, refined()).is_err());
        }
        for integration in [
            ModalIntegration::Midpoint {
                steps_per_sample: 0,
            },
            ModalIntegration::Midpoint {
                steps_per_sample: 2048,
            },
            ModalIntegration::Refined {
                contact_substeps: 3,
            },
            ModalIntegration::Refined {
                contact_substeps: 512,
            },
        ] {
            assert!(ModalAssembly::new(44100.0, g, p, integration).is_err());
        }
        for value in [f64::NAN, f64::INFINITY, -1.0] {
            for bad in [
                ModalAssemblyProfile {
                    support_mass_kg: value,
                    ..p
                },
                ModalAssemblyProfile {
                    support_inertia_kg_m2: value,
                    ..p
                },
                ModalAssemblyProfile {
                    tine_decay_seconds: [value; 6],
                    ..p
                },
                ModalAssemblyProfile {
                    damper_position: value,
                    ..p
                },
                ModalAssemblyProfile {
                    tonebar_mass_kg: value,
                    ..p
                },
                ModalAssemblyProfile {
                    contact_stiffness_n_m2: value,
                    ..p
                },
            ] {
                assert!(bad.validate().is_err());
            }
        }
        let mut v = voice();
        for velocity in [0.0, -1.0, 1.1, f64::NAN] {
            assert!(!v.strike(velocity));
        }
        assert_eq!(v.probe().injected_energy_j, 0.0);
    }
}
