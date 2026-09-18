//! Bounded scalar solve for the existing discrete-gradient contact law.
use crate::voice::contact_gradient;

/// The uniform-midpoint reference retains the original 48-step bisection.
pub(super) fn bisect(k: f64, a: f64, free: f64, compliance: f64) -> f64 {
    bisect_bracket(
        k,
        a,
        free,
        compliance,
        0.0,
        k * a.max(free).max(0.0).powi(2),
    )
}

fn bisect_bracket(k: f64, a: f64, free: f64, compliance: f64, mut lo: f64, mut hi: f64) -> f64 {
    for _ in 0..48 {
        let force = 0.5 * (lo + hi);
        if force > contact_gradient(k, a, free - compliance * force) {
            hi = force;
        } else {
            lo = force;
        }
    }
    0.5 * (lo + hi)
}

pub(super) fn solve(k: f64, a: f64, free: f64, compliance: f64) -> f64 {
    solve_bounded::<8>(k, a, free, compliance)
}

fn solve_bounded<const NEWTON_STEPS: usize>(k: f64, a: f64, free: f64, compliance: f64) -> f64 {
    // G is nondecreasing in its second argument. F - G(a, free - c F)
    // has derivative >= 1 for c >= 0. Therefore [0, G(a, free)] brackets
    // the unique nonnegative root, including compression/release crossings.
    let mut lo = 0.0;
    let mut hi = contact_gradient(k, a, free);
    if hi == 0.0 {
        return 0.0;
    }
    let mut force = hi;
    for _ in 0..NEWTON_STEPS {
        let b = free - compliance * force;
        let value = contact_gradient(k, a, b);
        let residual = force - value;
        if residual.abs() <= 8.0 * f64::EPSILON * force.max(value) {
            return force;
        }
        if residual > 0.0 {
            hi = force;
        } else {
            lo = force;
        }
        let slope = gradient_slope(k, a, b, value);
        let candidate = force - residual / (1.0 + compliance * slope);
        force = if candidate > lo && candidate < hi {
            candidate
        } else {
            0.5 * (lo + hi)
        };
    }
    // Bounded fallback also handles stalled or out-of-bracket Newton proposals.
    // Its interval is never wider than the original reference's interval.
    bisect_bracket(k, a, free, compliance, lo, hi)
}

pub(super) fn gradient_slope(k: f64, a: f64, b: f64, value: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        k * (a + 2.0 * b) / 3.0
    } else if a <= 0.0 && b <= 0.0 {
        0.0
    } else if a > 0.0 {
        value / (a - b)
    } else {
        let ratio = b / (b - a);
        k * ratio * ratio * (2.0 * b - 3.0 * a) / 3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(k: f64, a: f64, free: f64, c: f64) -> f64 {
        let (mut lo, mut hi) = (0.0, k * a.max(free).max(0.0).powi(2));
        for _ in 0..100 {
            let f = 0.5 * (lo + hi);
            if f > contact_gradient(k, a, free - c * f) {
                hi = f;
            } else {
                lo = f;
            }
        }
        0.5 * (lo + hi)
    }

    #[test]
    fn bounded_root_and_forced_fallback_match_long_bisection_across_contact_branches() {
        let displacements = [-1e-3, -1e-5, -1e-9, 0.0, 1e-9, 1e-5, 1e-3];
        for k in [1e8, 4e10, 1e12] {
            for c in [0.0, 1e-18, 1e-14, 1e-10, 1e-6, 1e-2] {
                for a in displacements {
                    for free in displacements {
                        let expected = reference(k, a, free, c);
                        let bound = k * a.max(free).max(0.0).powi(2);
                        for force in [solve(k, a, free, c), solve_bounded::<0>(k, a, free, c)] {
                            // G(a,a) and k*a*a can differ by a final rounding bit.
                            assert!(
                                force.is_finite()
                                    && force >= 0.0
                                    && force <= bound * (1.0 + 4.0 * f64::EPSILON),
                                "k={k} c={c} a={a} free={free}: force={force} bound={bound}"
                            );
                            assert!(
                                (force - expected).abs() <= 16.0 * f64::EPSILON * bound,
                                "k={k} c={c} a={a} free={free}: {force} vs {expected}"
                            );
                            if a <= 0.0 && free <= 0.0 {
                                assert_eq!(force, 0.0);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn root_preserves_contact_potential_work_through_compression_and_separation() {
        let k = 4e10;
        for a in [-1e-5, 0.0, 1e-5] {
            for free in [-2e-5, -1e-9, 0.0, 1e-9, 2e-5] {
                for c in [1e-14, 1e-10, 1e-6] {
                    let force = solve(k, a, free, c);
                    let b = free - c * force;
                    let work = force * (b - a);
                    let potential = k * (b.max(0.0).powi(3) - a.max(0.0).powi(3)) / 3.0;
                    let scale = k * a.max(free).max(0.0).powi(3);
                    assert!((work - potential).abs() <= 1e-12 * scale);
                }
            }
        }
    }
}
