//! Fixed-size, mass-whitened linear mechanics. Preparation only may factor matrices.
use super::{Matrix, N, Vector};
use crate::ModelError;
const S: usize = 2 * N;
type StateMatrix = [[f64; S]; S];

pub(super) fn dot<const D: usize>(a: [f64; D], b: [f64; D]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
pub(super) fn apply<const D: usize>(a: &[[f64; D]; D], x: [f64; D]) -> [f64; D] {
    a.map(|r| dot(r, x))
}
fn transpose<const D: usize>(a: [[f64; D]; D]) -> [[f64; D]; D] {
    core::array::from_fn(|i| core::array::from_fn(|j| a[j][i]))
}
fn multiply<const D: usize>(a: [[f64; D]; D], b: [[f64; D]; D]) -> [[f64; D]; D] {
    core::array::from_fn(|i| core::array::from_fn(|j| (0..D).map(|k| a[i][k] * b[k][j]).sum()))
}
fn factor(a: Matrix) -> Result<Matrix, ModelError> {
    let mut l = [[0.0; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let value = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            l[i][j] = if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError("modal assembly factorization failed"));
                }
                value.sqrt()
            } else {
                value / l[j][j]
            };
        }
    }
    Ok(l)
}
#[allow(
    clippy::needless_range_loop,
    reason = "Triangular substitution reads completed inverse rows while writing the current row"
)]
fn inverse_lower(l: Matrix) -> Matrix {
    let mut inverse = [[0.0; N]; N];
    for (i, row) in l.iter().enumerate() {
        for j in 0..=i {
            inverse[i][j] =
                (f64::from(i == j) - (j..i).map(|k| row[k] * inverse[k][j]).sum::<f64>()) / row[i];
        }
    }
    inverse
}
pub(super) fn inverse(a: Matrix) -> Result<Matrix, ModelError> {
    let inv = inverse_lower(factor(a)?);
    Ok(multiply(transpose(inv), inv))
}

pub(super) struct Midpoint {
    from_q: Matrix,
    from_v: Matrix,
    pub response: Vector,
}
impl Midpoint {
    pub fn prepare(m: Matrix, k: Matrix, c: Matrix, b: Vector, h: f64) -> Result<Self, ModelError> {
        let a = core::array::from_fn(|i| {
            core::array::from_fn(|j| m[i][j] + 0.5 * h * c[i][j] + 0.25 * h * h * k[i][j])
        });
        let inv = inverse(a)?;
        let r = core::array::from_fn(|i| {
            core::array::from_fn(|j| m[i][j] - 0.5 * h * c[i][j] - 0.25 * h * h * k[i][j])
        });
        let ik = multiply(inv, k);
        Ok(Self {
            from_q: ik.map(|r| r.map(|v| -h * v)),
            from_v: multiply(inv, r),
            response: apply(&inv, b).map(|v| h * v),
        })
    }
    pub fn free_velocity(&self, q: Vector, v: Vector) -> Vector {
        let a = apply(&self.from_q, q);
        let b = apply(&self.from_v, v);
        core::array::from_fn(|i| a[i] + b[i])
    }
}

pub(super) struct Free {
    transition: StateMatrix,
    loss: StateMatrix,
    to_normal: Matrix,
    from_normal: Matrix,
    scale: f64,
}
impl Free {
    pub fn prepare(m: Matrix, k: Matrix, c: Matrix, h: f64) -> Result<Self, ModelError> {
        let l = factor(m)?;
        let inv = inverse_lower(l);
        let kn = multiply(inv, multiply(k, transpose(inv)));
        let cn = multiply(inv, multiply(c, transpose(inv)));
        let scale = kn
            .iter()
            .map(|r| r.iter().map(|x| x.abs()).sum::<f64>())
            .fold(1.0, f64::max)
            .sqrt();
        let mut g = [[0.0; S]; S];
        let mut w = [[0.0; S]; S];
        for i in 0..N {
            g[i][i + N] = scale;
            for j in 0..N {
                g[i + N][j] = -kn[i][j] / scale;
                g[i + N][j + N] = -cn[i][j];
                w[i + N][j + N] = cn[i][j];
            }
        }
        let norm = g
            .iter()
            .map(|r| r.iter().map(|x| x.abs()).sum::<f64>())
            .fold(0.0, f64::max);
        let mut small = h;
        let mut squares = 0;
        while norm * small > 1.0 / 32.0 {
            small *= 0.5;
            squares += 1;
            if squares > 32 {
                return Err(ModelError("modal free transition scaling exceeded 32"));
            }
        }
        let mut e = exp(g, small);
        let mut loss = [[0.0; S]; S];
        let node = (3.0_f64 / 5.0).sqrt();
        for (t, weight) in [
            (0.5 * (1.0 - node), 5.0 / 18.0),
            (0.5, 4.0 / 9.0),
            (0.5 * (1.0 + node), 5.0 / 18.0),
        ] {
            let sample = exp(g, small * t);
            let power = multiply(transpose(sample), multiply(w, sample));
            for i in 0..S {
                for j in 0..S {
                    loss[i][j] += small * weight * power[i][j];
                }
            }
        }
        for _ in 0..squares {
            let propagated = multiply(transpose(e), multiply(loss, e));
            for i in 0..S {
                for j in 0..S {
                    loss[i][j] += propagated[i][j];
                }
            }
            e = multiply(e, e);
        }
        if e.iter()
            .chain(loss.iter())
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(ModelError("non-finite modal free transition"));
        }
        Ok(Self {
            transition: e,
            loss,
            to_normal: transpose(l),
            from_normal: transpose(inv),
            scale,
        })
    }
    pub fn advance(&self, q: Vector, v: Vector) -> (Vector, Vector, f64) {
        let qn = apply(&self.to_normal, q);
        let vn = apply(&self.to_normal, v);
        let state = core::array::from_fn(|i| if i < N { self.scale * qn[i] } else { vn[i - N] });
        let next = apply(&self.transition, state);
        let q = apply(
            &self.from_normal,
            core::array::from_fn(|i| next[i] / self.scale),
        );
        let v = apply(&self.from_normal, core::array::from_fn(|i| next[i + N]));
        (q, v, dot(state, apply(&self.loss, state)))
    }
}
fn exp(g: StateMatrix, h: f64) -> StateMatrix {
    let mut sum = core::array::from_fn(|i| core::array::from_fn(|j| f64::from(i == j)));
    let mut term = sum;
    for n in 1..=18 {
        term = multiply(term, g);
        for i in 0..S {
            for j in 0..S {
                term[i][j] *= h / f64::from(n);
                sum[i][j] += term[i][j];
            }
        }
    }
    sum
}
