use crate::{FIRST_NOTE, LAST_NOTE, ModelError, OVERSAMPLE, Profile};
use core::f64::consts::TAU;

const MODES: usize = 3;
const HAMMER_WEIGHTS: [f64; MODES] = [1.0, -0.3, 0.12];
const PICKUP_WEIGHTS: [f64; MODES] = [1.0, 0.8, 0.6];

#[derive(Debug, Clone, Copy, Default)]
pub struct Probe {
    pub displacement_m: f64,
    pub velocity_m_s: f64,
    pub contact_force_n: f64,
    pub mechanical_energy_j: f64,
    pub pickup_signal: f64,
    pub contact_active: bool,
}

#[derive(Clone, Copy)]
struct Mode {
    q: f64,
    v: f64,
    mass: f64,
    omega: f64,
    gamma: f64,
    free: [[f64; 4]; 2],
}

impl Mode {
    fn new(frequency: f64, mass: f64, gamma: f64, dt: f64) -> Self {
        let omega = TAU * frequency;
        let free = [gamma, gamma + 55.0].map(|g| {
            let wd = (omega * omega - g * g).sqrt();
            let (sin, cos) = (wd * dt).sin_cos();
            let envelope = (-g * dt).exp();
            let s = sin / wd;
            [
                envelope * (cos + g * s),
                envelope * s,
                -envelope * omega * omega * s,
                envelope * (cos - g * s),
            ]
        });
        Self {
            q: 0.0,
            v: 0.0,
            mass,
            omega,
            gamma,
            free,
        }
    }

    fn advance_free(&mut self, damped: bool) {
        let [a, b, c, d] = self.free[usize::from(damped)];
        let q = a * self.q + b * self.v;
        self.v = c * self.q + d * self.v;
        self.q = q;
    }

    fn energy(&self) -> f64 {
        0.5 * self.mass * (self.v * self.v + self.omega * self.omega * self.q * self.q)
    }
}

/// Research voice: three provisional cantilever modes, nonlinear elastic
/// hammer contact, and an analytic flux surrogate. No measured tonebar fit yet.
pub struct Voice {
    modes: [Mode; MODES],
    profile: Profile,
    dt: f64,
    hammer_mass: f64,
    hammer_x: f64,
    hammer_v: f64,
    contact: bool,
    damped: bool,
    active: bool,
    force: f64,
    signal: f64,
    pub(crate) last_channel: u8,
}

impl Voice {
    pub fn new(sample_rate: f64, note: u8, profile: Profile) -> Result<Self, ModelError> {
        profile.validate(sample_rate)?;
        if !(FIRST_NOTE..=LAST_NOTE).contains(&note) {
            return Err(ModelError("note must be in the 73-key range, MIDI 28..100"));
        }
        Ok(Self::new_validated(sample_rate, note, profile))
    }

    pub(crate) fn new_validated(sample_rate: f64, note: u8, profile: Profile) -> Self {
        let frequency = 440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0);
        let dt = 1.0 / (sample_rate * OVERSAMPLE as f64);
        let scale = (220.0 / frequency).clamp(0.15, 4.0);
        // Ideal uniform cantilever ratios, not measurements of a Rhodes assembly.
        let ratios = [1.0, 6.267, 17.55];
        let t60 = [
            profile.decay_seconds * scale.sqrt(),
            0.16 * scale.sqrt(),
            0.055 * scale.sqrt(),
        ];
        let modes = core::array::from_fn(|i| {
            Mode::new(
                frequency * ratios[i],
                profile.modal_mass_kg * scale,
                1000.0_f64.ln() / t60[i],
                dt,
            )
        });
        Self {
            modes,
            profile,
            dt,
            hammer_mass: profile.hammer_mass_kg * scale.sqrt(),
            hammer_x: 0.0,
            hammer_v: 0.0,
            contact: false,
            damped: true,
            active: false,
            force: 0.0,
            signal: 0.0,
            last_channel: 0,
        }
    }

    /// Retriggering retains every resonator coordinate and its velocity.
    pub fn strike(&mut self, velocity: f64) -> bool {
        if !velocity.is_finite() || !(0.0..=1.0).contains(&velocity) || velocity == 0.0 {
            return false;
        }
        self.hammer_x = self.contact_position();
        self.hammer_v = self.profile.maximum_hammer_speed_m_s * velocity.powf(1.4);
        self.contact = true;
        self.damped = false;
        self.active = true;
        true
    }

    pub fn set_damped(&mut self, damped: bool) {
        self.damped = damped;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Advance ONE internal sample (4x the output rate).
    pub fn tick(&mut self) -> f64 {
        if !self.active {
            return 0.0;
        }
        self.force = 0.0;
        if self.contact {
            self.advance_contact();
        } else {
            for mode in &mut self.modes {
                mode.advance_free(self.damped);
            }
        }
        let (position, velocity) = self.tip();
        // Smooth, bounded flux linkage surrogate. Gap never reaches zero.
        // Phi = 1 / sqrt(1 + ((offset + position) / gap)^2).
        // Output follows -dPhi/dt, not displacement and not a post-mix clipper.
        let z = (self.profile.pickup_offset_m + position) / self.profile.pickup_gap_m;
        let base = 1.0 + z * z;
        self.signal = 0.015 * z * velocity / (self.profile.pickup_gap_m * base * base.sqrt());
        if !self.contact && self.modes.iter().map(Mode::energy).sum::<f64>() < 1e-18 {
            self.reset();
        }
        self.signal
    }

    fn advance_contact(&mut self) {
        let h = self.dt;
        let delta0 = self.hammer_x - self.contact_position();
        let mut free_q = [0.0; MODES];
        let mut free_v = [0.0; MODES];
        let mut response_v = [0.0; MODES];
        let mut delta_free = self.hammer_x + h * self.hammer_v;
        let mut compliance = h * h / (2.0 * self.hammer_mass);
        for (i, mode) in self.modes.iter().enumerate() {
            let gamma = mode.gamma + if self.damped { 55.0 } else { 0.0 };
            let denominator = 1.0 + h * gamma + 0.25 * h * h * mode.omega * mode.omega;
            free_v[i] = ((1.0 - h * gamma - 0.25 * h * h * mode.omega * mode.omega) * mode.v
                - h * mode.omega * mode.omega * mode.q)
                / denominator;
            free_q[i] = mode.q + 0.5 * h * (mode.v + free_v[i]);
            response_v[i] = h * HAMMER_WEIGHTS[i] / (mode.mass * denominator);
            delta_free -= HAMMER_WEIGHTS[i] * free_q[i];
            compliance += 0.5 * h * HAMMER_WEIGHTS[i] * response_v[i];
        }
        let stiffness = self.profile.contact_stiffness;
        let maximum_delta = delta0.max(delta_free).max(0.0);
        let mut low = 0.0;
        let mut high = stiffness * maximum_delta * maximum_delta;
        // Monotone scalar solve with a proven bracket, bounded work, no Newton
        // divergence. The discrete gradient preserves the contact potential.
        for _ in 0..40 {
            let force = 0.5 * (low + high);
            let delta1 = delta_free - compliance * force;
            if force > contact_gradient(stiffness, delta0, delta1) {
                high = force;
            } else {
                low = force;
            }
        }
        self.force = 0.5 * (low + high);
        for (i, mode) in self.modes.iter_mut().enumerate() {
            mode.v = free_v[i] + response_v[i] * self.force;
            mode.q = free_q[i] + 0.5 * h * response_v[i] * self.force;
        }
        self.hammer_x += h * self.hammer_v - 0.5 * h * h * self.force / self.hammer_mass;
        self.hammer_v -= h * self.force / self.hammer_mass;
        let delta1 = self.hammer_x - self.contact_position();
        let contact_v: f64 = self
            .modes
            .iter()
            .zip(HAMMER_WEIGHTS)
            .map(|(m, b)| m.v * b)
            .sum();
        if delta1 <= 0.0 && self.hammer_v <= contact_v {
            self.contact = false;
            self.hammer_x = 0.0;
            self.hammer_v = 0.0;
        }
    }

    fn contact_position(&self) -> f64 {
        self.modes
            .iter()
            .zip(HAMMER_WEIGHTS)
            .map(|(m, b)| m.q * b)
            .sum()
    }

    fn tip(&self) -> (f64, f64) {
        let mut q = 0.0;
        let mut v = 0.0;
        for (mode, weight) in self.modes.iter().zip(PICKUP_WEIGHTS) {
            q += mode.q * weight;
            v += mode.v * weight;
        }
        (q, v)
    }

    pub fn probe(&self) -> Probe {
        let (displacement_m, velocity_m_s) = self.tip();
        let delta = (self.hammer_x - self.contact_position()).max(0.0);
        let hammer_energy = if self.contact {
            0.5 * self.hammer_mass * self.hammer_v * self.hammer_v
                + self.profile.contact_stiffness * delta * delta * delta / 3.0
        } else {
            0.0
        };
        Probe {
            displacement_m,
            velocity_m_s,
            contact_force_n: self.force,
            mechanical_energy_j: self.modes.iter().map(Mode::energy).sum::<f64>() + hammer_energy,
            pickup_signal: self.signal,
            contact_active: self.contact,
        }
    }

    pub fn reset(&mut self) {
        for mode in &mut self.modes {
            mode.q = 0.0;
            mode.v = 0.0;
        }
        self.hammer_x = 0.0;
        self.hammer_v = 0.0;
        self.contact = false;
        self.active = false;
        self.force = 0.0;
        self.signal = 0.0;
    }
}

/// Difference quotient of V(d) = k * max(d, 0)^3 / 3.
/// Piecewise algebra avoids catastrophic cancellation at nearby compressions.
fn contact_gradient(k: f64, a: f64, b: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        k * (a * a + a * b + b * b) / 3.0
    } else if a <= 0.0 && b <= 0.0 {
        0.0
    } else {
        let positive = a.max(b);
        k * positive * positive * positive / (3.0 * (b - a).abs())
    }
}
