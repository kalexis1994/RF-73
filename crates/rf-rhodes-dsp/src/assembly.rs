//! Offline, lowest-order tine/tonebar/common-support experiment.
//! These lumped coordinates are NOT measured Rhodes mode shapes. See
//! docs/COUPLED-ASSEMBLY.md for the reduction, power balance and validity limits.
use crate::{FIRST_NOTE, LAST_NOTE, ModelError, SAMPLE_RATE_MAX, SAMPLE_RATE_MIN};
use core::f64::consts::TAU;

type Vector = [f64; 3];
type Matrix = [[f64; 3]; 3];

/// Absolute coordinates: tine, tonebar, common support, in that order.
/// Springs/dashpots connect each prong to the support and the support to ground.
/// Values are frozen at construction; no time-varying stiffness energy injection.
#[derive(Debug, Clone, Copy)]
pub struct AssemblyParameters {
    pub masses_kg: Vector,
    pub stiffnesses_n_m: Vector,
    pub damping_n_s_m: Vector,
    /// Additional viscous loss from tine to ground when the binary damper is on.
    pub damper_n_s_m: f64,
    pub hammer_mass_kg: f64,
    /// Elastic contact F = k * max(compression, 0)^2, units N/m^2.
    pub contact_stiffness_n_m2: f64,
    pub maximum_hammer_speed_m_s: f64,
}

impl AssemblyParameters {
    /// An explicit engineering hypothesis, not an identified instrument.
    /// `note` sets the isolated prong frequency, NOT the assembled eigenfrequency.
    pub fn provisional(note: u8) -> Result<Self, ModelError> {
        if !(FIRST_NOTE..=LAST_NOTE).contains(&note) {
            return Err(ModelError("assembly note must be MIDI 28..100"));
        }
        let f = 440.0 * 2.0_f64.powf((f64::from(note) - 69.0) / 12.0);
        let scale = (220.0 / f).clamp(0.15, 4.0);
        let masses = [0.0015 * scale, 0.015 * scale, 0.030];
        let frequencies = [f, f, 80.0];
        let t60 = [5.0 * scale.sqrt(), 8.0 * scale.sqrt(), 0.08];
        Ok(Self {
            masses_kg: masses,
            stiffnesses_n_m: core::array::from_fn(|i| masses[i] * (TAU * frequencies[i]).powi(2)),
            damping_n_s_m: core::array::from_fn(|i| 2.0 * masses[i] * 1000.0_f64.ln() / t60[i]),
            damper_n_s_m: 110.0 * masses[0],
            hammer_mass_kg: 0.004 * scale.sqrt(),
            contact_stiffness_n_m2: 4.0e10,
            maximum_hammer_speed_m_s: 0.8,
        })
    }

    pub fn validate(self) -> Result<(), ModelError> {
        for value in self.masses_kg {
            bounded(value, 1e-4, 1.0, "assembly masses must be 0.0001..1 kg")?;
        }
        for value in self.stiffnesses_n_m {
            bounded(value, 0.0, 1e8, "assembly stiffness must be 0..1e8 N/m")?;
        }
        for value in self.damping_n_s_m.into_iter().chain([self.damper_n_s_m]) {
            bounded(value, 0.0, 1000.0, "assembly damping must be 0..1000 N s/m")?;
        }
        bounded(
            self.hammer_mass_kg,
            0.001,
            0.02,
            "assembly hammer mass must be 0.001..0.02 kg",
        )?;
        bounded(
            self.contact_stiffness_n_m2,
            1e8,
            1e12,
            "assembly contact stiffness must be 1e8..1e12 N/m^2",
        )?;
        bounded(
            self.maximum_hammer_speed_m_s,
            0.1,
            3.0,
            "assembly hammer speed must be 0.1..3 m/s",
        )
    }
}

fn bounded(value: f64, lo: f64, hi: f64, error: &'static str) -> Result<(), ModelError> {
    if value.is_finite() && (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(ModelError(error))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AssemblyProbe {
    pub displacement_m: Vector,
    pub velocity_m_s: Vector,
    pub contact_force_n: f64,
    pub contact_active: bool,
    /// Includes all three coordinates and active hammer/contact potential.
    pub mechanical_energy_j: f64,
    pub dissipated_energy_j: f64,
    /// Kinetic energy carried away by the hammer at separation.
    pub escaped_hammer_energy_j: f64,
    pub injected_energy_j: f64,
    /// E + dissipated + escaped - injected. Measures the numerical power balance.
    pub balance_residual_j: f64,
}

#[derive(Clone, Copy)]
struct Step {
    from_q: Matrix,
    from_v: Matrix,
    force_response: Vector,
}

impl Step {
    fn prepare(p: AssemblyParameters, h: f64, damped: bool) -> Result<Self, ModelError> {
        let k = network(p.stiffnesses_n_m);
        let mut c = network(p.damping_n_s_m);
        if damped {
            c[0][0] += p.damper_n_s_m;
        }
        let mass = |i, j| if i == j { p.masses_kg[i] } else { 0.0 };
        let a = core::array::from_fn(|i| {
            core::array::from_fn(|j| mass(i, j) + 0.5 * h * c[i][j] + 0.25 * h * h * k[i][j])
        });
        let inverse = inverse_spd(a)?;
        let b: Matrix = core::array::from_fn(|i| {
            core::array::from_fn(|j| mass(i, j) - 0.5 * h * c[i][j] - 0.25 * h * h * k[i][j])
        });
        let from_q = core::array::from_fn(|i| {
            core::array::from_fn(|j| -h * (0..3).map(|n| inverse[i][n] * k[n][j]).sum::<f64>())
        });
        let from_v = core::array::from_fn(|i| {
            core::array::from_fn(|j| (0..3).map(|n| inverse[i][n] * b[n][j]).sum())
        });
        Ok(Self {
            from_q,
            from_v,
            force_response: inverse.map(|row| h * row[0]),
        })
    }
}

/// Passive three-coordinate mechanical candidate. No pickup or decimator.
/// Call tick `substeps` times per output frame. Fixed-size, bounded work; no
/// allocations in tick, strike or damper changes. Not yet real-time qualified.
pub struct AssemblyVoice {
    parameters: AssemblyParameters,
    h: f64,
    steps: [Step; 2],
    q: Vector,
    v: Vector,
    hammer_x: f64,
    hammer_v: f64,
    contact: bool,
    damped: bool,
    force: f64,
    dissipated: f64,
    escaped: f64,
    injected: f64,
}

impl AssemblyVoice {
    pub fn new(
        sample_rate: f64,
        substeps: usize,
        parameters: AssemblyParameters,
    ) -> Result<Self, ModelError> {
        bounded(
            sample_rate,
            SAMPLE_RATE_MIN,
            SAMPLE_RATE_MAX,
            "assembly sample rate must be 44100..192000 Hz",
        )?;
        if !(4..=256).contains(&substeps) || !substeps.is_power_of_two() {
            return Err(ModelError(
                "assembly substeps must be a power of two from 4 to 256",
            ));
        }
        parameters.validate()?;
        let h = 1.0 / (sample_rate * substeps as f64);
        let steps = [
            Step::prepare(parameters, h, false)?,
            Step::prepare(parameters, h, true)?,
        ];
        Ok(Self {
            parameters,
            h,
            steps,
            q: [0.0; 3],
            v: [0.0; 3],
            hammer_x: 0.0,
            hammer_v: 0.0,
            contact: false,
            damped: false,
            force: 0.0,
            dissipated: 0.0,
            escaped: 0.0,
            injected: 0.0,
        })
    }

    /// Retains assembly motion. An unfinished strike is rejected, not replaced:
    /// repeated-contact action/escapement is outside this experiment's scope.
    pub fn strike(&mut self, velocity: f64) -> bool {
        if self.contact || !velocity.is_finite() || velocity <= 0.0 || velocity > 1.0 {
            return false;
        }
        self.hammer_x = self.q[0];
        self.hammer_v = self.parameters.maximum_hammer_speed_m_s * velocity.powf(1.4);
        self.injected += 0.5 * self.parameters.hammer_mass_kg * self.hammer_v.powi(2);
        self.contact = true;
        self.damped = false;
        true
    }

    pub fn set_damped(&mut self, damped: bool) {
        self.damped = damped;
    }

    pub fn tick(&mut self) {
        let step = self.steps[usize::from(self.damped)];
        let old_v = self.v;
        let free_v: Vector =
            core::array::from_fn(|i| dot(step.from_v[i], self.v) + dot(step.from_q[i], self.q));
        let free_q: Vector =
            core::array::from_fn(|i| self.q[i] + 0.5 * self.h * (self.v[i] + free_v[i]));
        self.force = 0.0;
        if self.contact {
            let d0 = self.hammer_x - self.q[0];
            let d_free = self.hammer_x + self.h * self.hammer_v - free_q[0];
            let compliance = self.h * self.h / (2.0 * self.parameters.hammer_mass_kg)
                + 0.5 * self.h * step.force_response[0];
            let k = self.parameters.contact_stiffness_n_m2;
            let mut lo = 0.0;
            let mut hi = k * d0.max(d_free).max(0.0).powi(2);
            for _ in 0..48 {
                let force = 0.5 * (lo + hi);
                if force > crate::voice::contact_gradient(k, d0, d_free - compliance * force) {
                    hi = force;
                } else {
                    lo = force;
                }
            }
            self.force = 0.5 * (lo + hi);
            self.hammer_x += self.h * self.hammer_v
                - 0.5 * self.h * self.h * self.force / self.parameters.hammer_mass_kg;
            self.hammer_v -= self.h * self.force / self.parameters.hammer_mass_kg;
        }
        self.v = core::array::from_fn(|i| free_v[i] + step.force_response[i] * self.force);
        self.q = core::array::from_fn(|i| {
            free_q[i] + 0.5 * self.h * step.force_response[i] * self.force
        });
        let mid_v: Vector = core::array::from_fn(|i| 0.5 * (old_v[i] + self.v[i]));
        let relative_v = [mid_v[0] - mid_v[2], mid_v[1] - mid_v[2], mid_v[2]];
        self.dissipated += self.h * dot(self.parameters.damping_n_s_m, relative_v.map(|v| v * v));
        if self.damped {
            self.dissipated += self.h * self.parameters.damper_n_s_m * mid_v[0].powi(2);
        }
        if self.contact && self.hammer_x <= self.q[0] && self.hammer_v <= self.v[0] {
            self.escaped += 0.5 * self.parameters.hammer_mass_kg * self.hammer_v.powi(2);
            self.contact = false;
            self.hammer_x = 0.0;
            self.hammer_v = 0.0;
        }
    }

    pub fn probe(&self) -> AssemblyProbe {
        let relative_q = [self.q[0] - self.q[2], self.q[1] - self.q[2], self.q[2]];
        let mut energy = 0.5
            * (dot(self.parameters.masses_kg, self.v.map(|v| v * v))
                + dot(self.parameters.stiffnesses_n_m, relative_q.map(|q| q * q)));
        if self.contact {
            energy += 0.5 * self.parameters.hammer_mass_kg * self.hammer_v.powi(2)
                + self.parameters.contact_stiffness_n_m2
                    * (self.hammer_x - self.q[0]).max(0.0).powi(3)
                    / 3.0;
        }
        AssemblyProbe {
            displacement_m: self.q,
            velocity_m_s: self.v,
            contact_force_n: self.force,
            contact_active: self.contact,
            mechanical_energy_j: energy,
            dissipated_energy_j: self.dissipated,
            escaped_hammer_energy_j: self.escaped,
            injected_energy_j: self.injected,
            balance_residual_j: energy + self.dissipated + self.escaped - self.injected,
        }
    }

    pub fn reset(&mut self) {
        self.q = [0.0; 3];
        self.v = [0.0; 3];
        self.hammer_x = 0.0;
        self.hammer_v = 0.0;
        self.contact = false;
        self.damped = false;
        self.force = 0.0;
        self.dissipated = 0.0;
        self.escaped = 0.0;
        self.injected = 0.0;
    }
}

fn dot(a: Vector, b: Vector) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn network([t, b, s]: Vector) -> Matrix {
    [[t, 0.0, -t], [0.0, b, -b], [-t, -b, t + b + s]]
}

/// Cholesky at construction only. M > 0 and K,C >= 0 imply A > 0,
/// including disconnected springs and overdamped configurations.
fn inverse_spd(a: Matrix) -> Result<Matrix, ModelError> {
    let mut l = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let value = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            l[i][j] = if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError("assembly factorization failed"));
                }
                value.sqrt()
            } else {
                value / l[j][j]
            };
        }
    }
    let mut inverse = [[0.0; 3]; 3];
    for column in [0, 1, 2] {
        let mut y = [0.0; 3];
        let mut x = [0.0; 3];
        for i in 0..3 {
            y[i] = (f64::from(i == column) - (0..i).map(|k| l[i][k] * y[k]).sum::<f64>()) / l[i][i];
        }
        for i in (0..3).rev() {
            x[i] = (y[i] - ((i + 1)..3).map(|k| l[k][i] * x[k]).sum::<f64>()) / l[i][i];
        }
        for i in 0..3 {
            inverse[i][column] = x[i];
        }
    }
    Ok(inverse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> AssemblyParameters {
        AssemblyParameters::provisional(55).unwrap()
    }

    #[test]
    fn disconnected_prongs_cannot_receive_energy() {
        let mut p = parameters();
        p.stiffnesses_n_m = [0.0; 3];
        p.damping_n_s_m = [0.0; 3];
        let mut voice = AssemblyVoice::new(48000.0, 4, p).unwrap();
        assert!(voice.strike(1.0));
        for _ in 0..10000 {
            voice.tick();
        }
        assert_eq!(voice.q[1..], [0.0, 0.0]);
        assert_eq!(voice.v[1..], [0.0, 0.0]);
        let expected = 2.0 * p.hammer_mass_kg * p.maximum_hammer_speed_m_s
            / (p.hammer_mass_kg + p.masses_kg[0]);
        assert!((voice.v[0] / expected - 1.0).abs() < 1e-10);
        let probe = voice.probe();
        assert!(probe.balance_residual_j.abs() / probe.injected_energy_j < 1e-10);
    }

    #[test]
    fn undamped_fork_matches_analytic_antisymmetric_mode_and_converges() {
        let mut p = parameters();
        p.masses_kg[1] = p.masses_kg[0];
        p.stiffnesses_n_m[1] = p.stiffnesses_n_m[0];
        p.damping_n_s_m = [0.0; 3];
        let omega = (p.stiffnesses_n_m[0] / p.masses_kg[0]).sqrt();
        let mut previous_error = f64::INFINITY;
        for substeps in [4, 8, 16, 32] {
            let mut voice = AssemblyVoice::new(48000.0, substeps, p).unwrap();
            voice.q = [1e-4, -1e-4, 0.0];
            let initial = voice.probe().mechanical_energy_j;
            let mut error = 0.0_f64;
            for frame in 1..=4800 {
                for _ in 0..substeps {
                    voice.tick();
                }
                let expected = 1e-4 * (omega * f64::from(frame) / 48000.0).cos();
                error = error.max((voice.q[0] - expected).abs());
                assert!(voice.q[2].abs() < 1e-15);
                assert!((voice.probe().mechanical_energy_j / initial - 1.0).abs() < 1e-9);
            }
            assert!(
                error < previous_error * 0.27,
                "{substeps}: {error}, previous {previous_error}"
            );
            previous_error = error;
        }
    }

    #[test]
    fn mount_transfers_energy_reciprocally() {
        let mut p = parameters();
        p.damping_n_s_m = [0.0; 3];
        let mut from_tine = AssemblyVoice::new(48000.0, 8, p).unwrap();
        let mut from_bar = AssemblyVoice::new(48000.0, 8, p).unwrap();
        // Equal external impulses, not equal velocities, test mechanical reciprocity.
        from_tine.v[0] = 1e-4 / p.masses_kg[0];
        from_bar.v[1] = 1e-4 / p.masses_kg[1];
        let mut transferred = 0.0_f64;
        for _ in 0..20000 {
            from_tine.tick();
            from_bar.tick();
            transferred = transferred.max(from_tine.q[1].abs());
            assert!((from_tine.q[1] - from_bar.q[0]).abs() < 1e-13);
        }
        assert!(transferred > 1e-8);
    }

    #[test]
    fn coupled_support_modes_match_the_analytic_eigenproblem() {
        let mut p = parameters();
        p.masses_kg[1] = p.masses_kg[0];
        p.stiffnesses_n_m[1] = p.stiffnesses_n_m[0];
        p.damping_n_s_m = [0.0; 3];
        let (m, support_mass) = (p.masses_kg[0], p.masses_kg[2]);
        let (k, support_k) = (p.stiffnesses_n_m[0], p.stiffnesses_n_m[2]);
        // Symmetric prongs reduce to masses 2m,M joined by stiffness 2k,
        // with the support also grounded by ks. Solve the quadratic in omega^2.
        let a = m * support_mass;
        let b = k * support_mass + m * (2.0 * k + support_k);
        let discriminant = (b * b - 4.0 * a * k * support_k).sqrt();
        for lambda in [
            (b - discriminant) / (2.0 * a),
            (b + discriminant) / (2.0 * a),
        ] {
            let mut voice = AssemblyVoice::new(48000.0, 32, p).unwrap();
            let initial_q = [1e-4, 1e-4, 1e-4 * (k - m * lambda) / k];
            voice.q = initial_q;
            for frame in 1..=2400 {
                for _ in 0..32 {
                    voice.tick();
                }
                let phase = (lambda.sqrt() * f64::from(frame) / 48000.0).cos();
                for (actual, initial) in voice.q.into_iter().zip(initial_q) {
                    assert!((actual - initial * phase).abs() < initial.abs() * 1e-5);
                }
            }
        }
    }

    #[test]
    fn contact_release_and_damper_close_the_energy_budget() {
        for note in [FIRST_NOTE, 55, LAST_NOTE] {
            for rate in [44100.0, 192000.0] {
                for velocity in [0.01, 1.0] {
                    let mut voice =
                        AssemblyVoice::new(rate, 4, AssemblyParameters::provisional(note).unwrap())
                            .unwrap();
                    assert!(voice.strike(velocity));
                    let initial = voice.probe().injected_energy_j;
                    let mut previous = initial;
                    for n in 0..20000 {
                        if n == 10000 {
                            voice.set_damped(true);
                        }
                        voice.tick();
                        let probe = voice.probe();
                        assert!(probe.mechanical_energy_j.is_finite());
                        assert!(probe.mechanical_energy_j <= previous + initial * 1e-11);
                        assert!(
                            probe.balance_residual_j.abs() < initial * 1e-8,
                            "{note}, {rate}, {velocity}: {probe:?}"
                        );
                        previous = probe.mechanical_energy_j;
                    }
                    assert!(!voice.contact);
                    assert!(voice.probe().dissipated_energy_j > 0.0);
                    assert!(voice.probe().escaped_hammer_energy_j > 0.0);
                }
            }
        }
    }

    #[test]
    fn parameter_corners_remain_passive_including_overdamping() {
        for high_mass in [false, true] {
            for high_stiffness in [false, true] {
                for high_damping in [false, true] {
                    for hard_contact in [false, true] {
                        let mut p = parameters();
                        p.masses_kg = if high_mass {
                            [1.0, 1e-4, 1.0]
                        } else {
                            [1e-4, 1.0, 1e-4]
                        };
                        p.stiffnesses_n_m = [if high_stiffness { 1e8 } else { 0.0 }; 3];
                        p.damping_n_s_m = [if high_damping { 1000.0 } else { 0.0 }; 3];
                        p.contact_stiffness_n_m2 = if hard_contact { 1e12 } else { 1e8 };
                        p.maximum_hammer_speed_m_s = 3.0;
                        let mut voice = AssemblyVoice::new(44100.0, 4, p).unwrap();
                        assert!(voice.strike(1.0));
                        for _ in 0..10000 {
                            voice.tick();
                            let probe = voice.probe();
                            assert!(
                                probe.balance_residual_j.abs() / probe.injected_energy_j < 1e-7,
                                "{probe:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_inputs_are_rejected_without_mutation_and_restrike_retains_motion() {
        let p = parameters();
        for rate in [f64::NAN, f64::INFINITY, 0.0, 44100.0 - 1.0, 192001.0] {
            assert!(AssemblyVoice::new(rate, 4, p).is_err());
        }
        for steps in [0, 1, 3, 6, 512, usize::MAX] {
            assert!(AssemblyVoice::new(48000.0, steps, p).is_err());
        }
        for value in [f64::NAN, f64::INFINITY, -1.0] {
            for index in 0..3 {
                let mut bad = p;
                bad.masses_kg[index] = value;
                assert!(bad.validate().is_err());
                let mut bad = p;
                bad.stiffnesses_n_m[index] = value;
                assert!(bad.validate().is_err());
                let mut bad = p;
                bad.damping_n_s_m[index] = value;
                assert!(bad.validate().is_err());
            }
            let mut bad = p;
            bad.damper_n_s_m = value;
            assert!(bad.validate().is_err());
            let mut bad = p;
            bad.hammer_mass_kg = value;
            assert!(bad.validate().is_err());
            let mut bad = p;
            bad.contact_stiffness_n_m2 = value;
            assert!(bad.validate().is_err());
            let mut bad = p;
            bad.maximum_hammer_speed_m_s = value;
            assert!(bad.validate().is_err());
        }
        assert!(AssemblyParameters::provisional(27).is_err());
        assert!(AssemblyParameters::provisional(101).is_err());
        let mut voice = AssemblyVoice::new(48000.0, 8, p).unwrap();
        for velocity in [0.0, -1.0, 1.01, f64::NAN, f64::INFINITY] {
            assert!(!voice.strike(velocity));
        }
        assert_eq!(voice.probe().injected_energy_j, 0.0);
        assert!(voice.strike(0.7));
        let injected = voice.injected;
        assert!(!voice.strike(0.9));
        assert_eq!(voice.injected, injected);
        for _ in 0..10000 {
            voice.tick();
        }
        let (q, v) = (voice.q, voice.v);
        assert!(voice.strike(0.9));
        assert_eq!((voice.q, voice.v), (q, v));
        for _ in 0..10000 {
            voice.tick();
        }
        assert!(voice.probe().balance_residual_j.abs() / voice.injected < 1e-9);
        voice.reset();
        assert_eq!(voice.probe().mechanical_energy_j, 0.0);
        assert_eq!(voice.probe().injected_energy_j, 0.0);
        assert_eq!(voice.probe().balance_residual_j, 0.0);
    }
}
