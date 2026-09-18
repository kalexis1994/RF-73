//! Prepared exponential transition and independently integrated dissipated work.
//! All matrix exponentiation/quadrature runs at construction, never during audio.
use super::{AssemblyParameters, ModelError, Vector, network};

type Matrix6 = [[f64; 6]; 6];

pub(super) struct FreeStep {
    transition: Matrix6,
    loss: Matrix6,
    scale: [f64; 6],
}

impl FreeStep {
    pub(super) fn prepare(p: AssemblyParameters, h: f64, damped: bool) -> Result<Self, ModelError> {
        let k = network(p.stiffnesses_n_m);
        let mut c = network(p.damping_n_s_m);
        if damped {
            c[0][0] += p.damper_n_s_m;
        }
        let roots = p.masses_kg.map(f64::sqrt);
        let frequency_scale = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| k[i][j].abs() / (roots[i] * roots[j]))
                    .sum::<f64>()
            })
            .fold(1.0, f64::max)
            .sqrt();
        // Mass-normalized coordinates, with a frequency scale to balance q/v.
        // This also supports K=0 and avoids physical SI units dominating the norm.
        let scale =
            core::array::from_fn(|i| roots[i % 3] * if i < 3 { frequency_scale } else { 1.0 });
        let mut generator = [[0.0; 6]; 6];
        for i in 0..3 {
            generator[i][i + 3] = frequency_scale;
            for j in 0..3 {
                generator[i + 3][j] = -k[i][j] / (roots[i] * roots[j] * frequency_scale);
                generator[i + 3][j + 3] = -c[i][j] / (roots[i] * roots[j]);
            }
        }
        let norm = generator
            .iter()
            .map(|r| r.iter().map(|v| v.abs()).sum::<f64>())
            .fold(0.0, f64::max);
        let mut small_h = h;
        let mut squarings = 0;
        while norm * small_h > 1.0 / 32.0 {
            small_h *= 0.5;
            squarings += 1;
            if squarings > 32 {
                return Err(ModelError(
                    "assembly exponential scaling exceeded its bound",
                ));
            }
        }
        let mut transition = exponential(generator, small_h);
        let mut loss = [[0.0; 6]; 6];
        // Three-point Gauss-Legendre on the small interval. Positive weights
        // and sums of dashpot observation outer products give a PSD work form.
        // Scaling bounds the trajectory variation; squaring composes its work.
        let node = (3.0_f64 / 5.0).sqrt();
        for (t, weight) in [
            (0.5 * (1.0 - node), 5.0 / 18.0),
            (0.5, 4.0 / 9.0),
            (0.5 * (1.0 + node), 5.0 / 18.0),
        ] {
            let e = exponential(generator, small_h * t);
            let observers: [[f64; 6]; 4] = core::array::from_fn(|edge| {
                core::array::from_fn(|j| match edge {
                    0 => e[3][j] / roots[0] - e[5][j] / roots[2],
                    1 => e[4][j] / roots[1] - e[5][j] / roots[2],
                    2 => e[5][j] / roots[2],
                    _ => e[3][j] / roots[0],
                })
            });
            let damping = [
                p.damping_n_s_m[0],
                p.damping_n_s_m[1],
                p.damping_n_s_m[2],
                if damped { p.damper_n_s_m } else { 0.0 },
            ];
            for (observer, damping) in observers.into_iter().zip(damping) {
                for i in 0..6 {
                    for j in 0..6 {
                        loss[i][j] += small_h * weight * damping * observer[i] * observer[j];
                    }
                }
            }
        }
        for _ in 0..squarings {
            let propagated = multiply(transpose(transition), multiply(loss, transition));
            for i in 0..6 {
                for j in 0..6 {
                    loss[i][j] += propagated[i][j];
                }
            }
            transition = multiply(transition, transition);
        }
        if transition
            .iter()
            .chain(loss.iter())
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(ModelError("non-finite assembly free transition"));
        }
        Ok(Self {
            transition,
            loss,
            scale,
        })
    }

    pub(super) fn advance(&self, q: Vector, v: Vector) -> (Vector, Vector, f64) {
        let state: [f64; 6] =
            core::array::from_fn(|i| self.scale[i] * if i < 3 { q[i] } else { v[i - 3] });
        let next: [f64; 6] =
            core::array::from_fn(|i| inner(self.transition[i], state) / self.scale[i]);
        let loss = (0..6).map(|i| state[i] * inner(self.loss[i], state)).sum();
        (
            core::array::from_fn(|i| next[i]),
            core::array::from_fn(|i| next[i + 3]),
            loss,
        )
    }
}

fn inner(a: [f64; 6], b: [f64; 6]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn transpose(a: Matrix6) -> Matrix6 {
    core::array::from_fn(|i| core::array::from_fn(|j| a[j][i]))
}
fn multiply(a: Matrix6, b: Matrix6) -> Matrix6 {
    core::array::from_fn(|i| core::array::from_fn(|j| (0..6).map(|k| a[i][k] * b[k][j]).sum()))
}

/// Fixed Taylor degree 18 on ||hG||_infinity <= 1/32, also at quadrature nodes.
/// The scalar norm bound on the omitted terms is far below f64 roundoff.
fn exponential(generator: Matrix6, h: f64) -> Matrix6 {
    let mut sum: Matrix6 = core::array::from_fn(|i| core::array::from_fn(|j| f64::from(i == j)));
    let mut term = sum;
    for n in 1..=18 {
        term = multiply(term, generator);
        for i in 0..6 {
            for j in 0..6 {
                term[i][j] *= h / f64::from(n);
                sum[i][j] += term[i][j];
            }
        }
    }
    sum
}
