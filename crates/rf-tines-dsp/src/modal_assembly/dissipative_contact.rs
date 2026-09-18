//! Projected Hunt-Crossley-type contact with an explicit discrete heat balance.
use super::contact::{bisect, gradient_slope, solve};
use crate::voice::contact_gradient;

pub(super) struct ContactStep {
    pub force: f64,
    pub heat: f64,
    pub limited: bool,
}

pub(super) struct RateContact {
    pub stiffness: f64,
    /// beta / h, where beta has units s/m.
    pub rate: f64,
}
impl RateContact {
    pub(super) fn law(&self, a: f64, b: f64) -> (f64, f64) {
        let g = contact_gradient(self.stiffness, a, b);
        let multiplier = 1.0 + self.rate * (b - a);
        if multiplier <= 0.0 {
            return (0.0, 0.0);
        }
        (
            g * multiplier,
            gradient_slope(self.stiffness, a, b, g) * multiplier + self.rate * g,
        )
    }

    pub fn advance<const FAST: bool>(&self, a: f64, free: f64, compliance: f64) -> ContactStep {
        if self.rate == 0.0 {
            return ContactStep {
                force: if FAST {
                    solve(self.stiffness, a, free, compliance)
                } else {
                    bisect(self.stiffness, a, free, compliance)
                },
                heat: 0.0,
                limited: false,
            };
        }
        // Both positive factors in the force law are nondecreasing in b.
        // Consequently [0, law(a,free)] brackets a unique implicit force.
        let mut lo = 0.0;
        let mut hi = self.law(a, free).0;
        let mut force = hi;
        let mut converged = hi == 0.0;
        if FAST && !converged {
            for _ in 0..8 {
                let (value, slope) = self.law(a, free - compliance * force);
                let residual = force - value;
                if residual.abs() <= 8.0 * f64::EPSILON * force.max(value) {
                    converged = true;
                    break;
                }
                if residual > 0.0 {
                    hi = force;
                } else {
                    lo = force;
                }
                let next = force - residual / (1.0 + compliance * slope);
                force = if next > lo && next < hi {
                    next
                } else {
                    0.5 * (lo + hi)
                };
            }
        }
        if !converged {
            for _ in 0..48 {
                force = 0.5 * (lo + hi);
                if force > self.law(a, free - compliance * force).0 {
                    hi = force;
                } else {
                    lo = force;
                }
            }
            force = 0.5 * (lo + hi);
        }
        let b = free - compliance * force;
        let delta = b - a;
        let g = contact_gradient(self.stiffness, a, b);
        let limited = g > 0.0 && 1.0 + self.rate * delta < 0.0;
        // Constitutive dissipation, independently evaluated from compression.
        // This is not a residual correction based on total mechanical energy.
        let heat = if limited {
            -g * delta
        } else {
            g * self.rate * delta * delta
        };
        ContactStep {
            force,
            heat,
            limited,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dissipative_contact_is_nonadhesive_and_closes_work_including_limited_unloading() {
        let mut limited = 0;
        for beta in [0.0, 0.5, 2.0, 10.0] {
            for h in [1e-7, 1e-5] {
                let law = RateContact {
                    stiffness: 4e10,
                    rate: beta / h,
                };
                for a in [-1e-5, 0.0, 1e-5] {
                    for free in [-2e-5, 0.0, 2e-5] {
                        for compliance in [0.0, 1e-14, 1e-10] {
                            let fast = law.advance::<true>(a, free, compliance);
                            let reference = law.advance::<false>(a, free, compliance);
                            let bound = law.law(a, free).0;
                            assert!((fast.force - reference.force).abs() <= 1e-12 * bound);
                            for step in [fast, reference] {
                                assert!(step.force.is_finite() && step.force >= 0.0);
                                assert!(step.heat.is_finite() && step.heat >= 0.0);
                                let b = free - compliance * step.force;
                                let work = step.force * (b - a);
                                let potential =
                                    law.stiffness * (b.max(0.0).powi(3) - a.max(0.0).powi(3)) / 3.0;
                                let scale =
                                    work.abs().max(potential.abs()).max(step.heat).max(1e-30);
                                assert!((work - potential - step.heat).abs() < 1e-10 * scale);
                                if step.limited {
                                    limited += 1;
                                    assert_eq!(step.force, 0.0);
                                }
                                if beta == 0.0 {
                                    assert_eq!(step.heat, 0.0);
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(limited > 0);
    }

    fn rigid_impact(beta: f64, speed: f64, steps: f64) -> (f64, f64) {
        let mass = 0.004;
        let h = 1.0 / (44100.0 * steps);
        let law = RateContact {
            stiffness: 4e10,
            rate: beta / h,
        };
        let (mut x, mut v, mut heat) = (0.0_f64, speed, 0.0);
        let initial = 0.5 * mass * speed * speed;
        for _ in 0..100000 {
            let step = law.advance::<true>(x, x + h * v, h * h / (2.0 * mass));
            x += h * v - h * h * step.force / (2.0 * mass);
            v -= h * step.force / mass;
            heat += step.heat;
            let energy = 0.5 * mass * v * v + law.stiffness * x.max(0.0).powi(3) / 3.0;
            assert!((energy + heat - initial).abs() < initial * 1e-9);
            assert!(!step.limited);
            if x <= 0.0 && v <= 0.0 {
                return (-v / speed, heat / initial);
            }
        }
        panic!("rigid contact did not separate");
    }

    #[test]
    fn isolated_impact_converges_to_continuous_hunt_crossley_restitution() {
        for beta in [0.5, 2.0] {
            for speed in [0.2, 0.8, 3.0] {
                // Integrating m*v*dv/(1+beta*v) = -k*x^2*dx gives
                // u - ln(1+u) equal at entry and exit (x=0).
                let u: f64 = beta * speed;
                let target = u - u.ln_1p();
                let (mut lo, mut hi) = (0.0_f64, 1.0_f64.min(1.0 / u));
                for _ in 0..80 {
                    let e = 0.5 * (lo + hi);
                    let exit = -u * e;
                    if exit - exit.ln_1p() > target {
                        hi = e;
                    } else {
                        lo = e;
                    }
                }
                let expected = 0.5 * (lo + hi);
                let coarse = rigid_impact(beta, speed, 4.0);
                let fine = rigid_impact(beta, speed, 128.0);
                assert!(
                    (fine.0 - expected).abs() < 1e-5,
                    "{beta} {speed}: {fine:?} expected {expected}"
                );
                assert!((fine.0 - expected).abs() < (coarse.0 - expected).abs());
                assert!(fine.0 > 0.0 && fine.0 < 1.0 && fine.1 > 0.0);
                assert!((fine.1 - (1.0 - expected * expected)).abs() < 2e-5);
            }
        }
    }
}
