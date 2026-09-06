//! Offline modes of a circular Euler-Bernoulli tine with a point or spread tuning mass.
//! No measured Rhodes geometry is implied. See docs/TINE-MODES.md.
use crate::ModelError;
use core::f64::consts::{PI, TAU};

pub const TINE_MODE_COUNT: usize = 6;
const MAX_DOFS: usize = 130;
const GAUSS5: [(f64, f64); 5] = [
    (-0.906179845938664, 0.2369268850561891),
    (-0.5384693101056831, 0.4786286704993665),
    (0.0, 0.5688888888888889),
    (0.5384693101056831, 0.4786286704993665),
    (0.906179845938664, 0.2369268850561891),
];

#[derive(Debug, Clone, Copy)]
pub struct TineGeometry {
    pub length_m: f64,
    pub diameter_m: f64,
    /// Tip/root diameter ratio; diameter varies linearly up to taper_end_fraction.
    /// One preserves the uniform baseline. No measured manufacturing profile is implied.
    pub tip_diameter_ratio: f64,
    /// End of the root transition as a fraction of free length; constant section beyond.
    /// One preserves full-length taper. This is a provisional piecewise-linear profile.
    pub taper_end_fraction: f64,
    pub young_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub tuning_mass_kg: f64,
    /// Uniform co-moving mass span, centered at tuning_position. Zero is a point.
    /// Adds inertia only: no coil elasticity, slip, or cross-sectional rotary inertia.
    pub tuning_span_m: f64,
    /// All positions are fractions of free beam length, measured from the root.
    pub tuning_position: f64,
    pub hammer_position: f64,
    pub pickup_position: f64,
}

impl Default for TineGeometry {
    fn default() -> Self {
        Self {
            length_m: 0.075,
            diameter_m: 0.0015,
            tip_diameter_ratio: 1.0,
            taper_end_fraction: 1.0,
            young_modulus_pa: 2e11,
            density_kg_m3: 7850.0,
            tuning_mass_kg: 0.0001,
            tuning_span_m: 0.0,
            tuning_position: 0.85,
            hammer_position: 0.2,
            pickup_position: 0.98,
        }
    }
}

impl TineGeometry {
    pub fn validate(self) -> Result<(), ModelError> {
        for (value, low, high) in [
            (self.length_m, 0.015, 0.3),
            (self.diameter_m, 0.0003, 0.003),
            (self.tip_diameter_ratio, 0.5, 1.5),
            (self.taper_end_fraction, 0.05, 1.0),
            (self.diameter_m * self.tip_diameter_ratio, 0.0003, 0.003),
            (self.young_modulus_pa, 1e10, 3e11),
            (self.density_kg_m3, 1000.0, 20000.0),
            (self.tuning_mass_kg, 0.0, 0.01),
            (self.tuning_span_m, 0.0, self.length_m),
            (self.tuning_position, 0.0, 1.0),
            (self.hammer_position, 0.0, 1.0),
            (self.pickup_position, 0.0, 1.0),
        ] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err(ModelError(
                    "tine geometry outside its finite SI parameter domain",
                ));
            }
        }
        if self.length_m / (self.diameter_m * self.tip_diameter_ratio.max(1.0)) < 10.0 {
            return Err(ModelError(
                "Euler-Bernoulli tine requires length/diameter >= 10",
            ));
        }
        let half = 0.5 * self.tuning_span_m / self.length_m;
        if self.tuning_position - half < 0.0
            || self.tuning_position + half > 1.0
            || (self.tuning_span_m > 0.0
                && self.tuning_position - half >= self.tuning_position + half)
        {
            return Err(ModelError(
                "tuning mass span must fit on the tine and be numerically resolvable",
            ));
        }
        Ok(())
    }

    /// Offline quadrature: (position fraction, fraction of total tuning mass).
    /// Four Gauss points per intersected Hermite element integrate degree 7
    /// exactly, including products of cubic shapes. At most 4*elements points.
    pub fn tuning_mass_quadrature(self, elements: usize) -> Result<Vec<(f64, f64)>, ModelError> {
        self.validate()?;
        if !(8..=64).contains(&elements) || !elements.is_power_of_two() {
            return Err(ModelError(
                "tuning quadrature needs 8..64 power-of-two elements",
            ));
        }
        if self.tuning_span_m == 0.0 {
            return Ok(vec![(self.tuning_position, 1.0)]);
        }
        let half = 0.5 * self.tuning_span_m / self.length_m;
        let lo = self.tuning_position - half;
        let hi = self.tuning_position + half;
        let mut points = Vec::with_capacity(4 * elements);
        for e in 0..elements {
            let a = lo.max(e as f64 / elements as f64);
            let b = hi.min((e + 1) as f64 / elements as f64);
            if b <= a {
                continue;
            }
            for (node, weight) in [
                (-0.8611363115940526, 0.3478548451374538),
                (-0.3399810435848563, 0.6521451548625461),
                (0.3399810435848563, 0.6521451548625461),
                (0.8611363115940526, 0.3478548451374538),
            ] {
                points.push((
                    a + 0.5 * (b - a) * (1.0 + node),
                    0.5 * weight * (b - a) / (hi - lo),
                ));
            }
        }
        Ok(points)
    }

    fn reference_mass_kg(self) -> f64 {
        self.density_kg_m3 * PI * self.diameter_m.powi(2) * self.length_m / 4.0
    }
    fn relative_mass_moment(self, order: i32) -> f64 {
        let a = self.tip_diameter_ratio - 1.0;
        let full =
            1.0 / (order + 1) as f64 + 2.0 * a / (order + 2) as f64 + a * a / (order + 3) as f64;
        if self.taper_end_fraction == 1.0 || self.tip_diameter_ratio == 1.0 {
            return full;
        }
        let end_power = self.taper_end_fraction.powi(order + 1);
        end_power * full + self.tip_diameter_ratio.powi(2) * (1.0 - end_power) / (order + 1) as f64
    }
    fn section_ratio(self, s: f64) -> f64 {
        1.0 + (self.tip_diameter_ratio - 1.0) * (s / self.taper_end_fraction).min(1.0)
    }
    // Element-local t and Gauss weight. Split the integration at the section corner,
    // even when the finite-element nodes do not coincide with that corner.
    fn section_rule(self, elements: usize, element: usize) -> Vec<(f64, f64)> {
        let cut = (self.taper_end_fraction * elements as f64 - element as f64).clamp(0.0, 1.0);
        let mut result = Vec::with_capacity(10);
        for (a, b) in [(0.0, cut), (cut, 1.0)] {
            if b <= a {
                continue;
            }
            for (node, weight) in GAUSS5 {
                result.push((a + 0.5 * (b - a) * (1.0 + node), weight * (b - a)));
            }
        }
        result
    }
    pub fn beam_mass_kg(self) -> f64 {
        self.reference_mass_kg() * self.relative_mass_moment(0)
    }
    /// Offline samples (fraction of free length, physical mass in kg).
    /// Five points integrate tapered Hermite mass products (degree 8) exactly.
    pub fn beam_mass_quadrature(self, elements: usize) -> Result<Vec<(f64, f64)>, ModelError> {
        self.validate()?;
        if !(8..=64).contains(&elements) || !elements.is_power_of_two() {
            return Err(ModelError(
                "beam quadrature needs 8..64 power-of-two elements",
            ));
        }
        let uniform = [
            (-0.8611363115940526, 0.3478548451374538),
            (-0.3399810435848563, 0.6521451548625461),
            (0.3399810435848563, 0.6521451548625461),
            (0.8611363115940526, 0.3478548451374538),
        ];
        let rule: &[(f64, f64)] = if self.tip_diameter_ratio == 1.0 {
            &uniform
        } else {
            &GAUSS5
        };
        let mut points = Vec::with_capacity(elements * rule.len());
        for e in 0..elements {
            if self.tip_diameter_ratio != 1.0 && self.taper_end_fraction != 1.0 {
                for (t, weight) in self.section_rule(elements, e) {
                    let s = (e as f64 + t) / elements as f64;
                    points.push((
                        s,
                        self.reference_mass_kg() * weight / (2 * elements) as f64
                            * self.section_ratio(s).powi(2),
                    ));
                }
                continue;
            }
            for &(node, weight) in rule {
                let s = (e as f64 + 0.5 * (1.0 + node)) / elements as f64;
                let area = (1.0 + (self.tip_diameter_ratio - 1.0) * s).powi(2);
                points.push((
                    s,
                    self.reference_mass_kg() * weight / (2 * elements) as f64 * area,
                ));
            }
        }
        Ok(points)
    }
    /// Bending rigidity at the root; tapered preparation integrates its local value.
    pub fn bending_rigidity_n_m2(self) -> f64 {
        self.young_modulus_pa * PI * self.diameter_m.powi(4) / 64.0
    }
}

/// Shape normalized to displacement 1 at the free tip; modal coordinate in meters.
#[derive(Debug, Clone, Copy, Default)]
pub struct TineMode {
    pub frequency_hz: f64,
    pub effective_mass_kg: f64,
    pub hammer_weight: f64,
    pub pickup_weight: f64,
    /// Kinetic cross coefficient between modal velocity and root translation.
    pub translation_coupling_kg: f64,
    /// Kinetic cross coefficient between modal velocity and root angular velocity.
    pub rotation_coupling_kg_m: f64,
    pub relative_eigen_residual: f64,
}

/// Six fixed-root bending modes, plus the inertia needed for a moving-root reduction.
/// Construction allocates bounded dense work matrices; shape lookup does not allocate.
pub struct TineModes {
    pub modes: [TineMode; TINE_MODE_COUNT],
    pub total_mass_kg: f64,
    pub first_mass_moment_kg_m: f64,
    pub second_mass_moment_kg_m2: f64,
    pub maximum_mass_orthogonality_error: f64,
    elements: usize,
    shapes: [[f64; MAX_DOFS]; TINE_MODE_COUNT],
}

impl TineModes {
    /// Cubic Hermite elements with consistent mass, clamped displacement/slope at root.
    /// 8..64 elements, power of two. This is offline preparation, not a render method.
    pub fn prepare(geometry: TineGeometry, elements: usize) -> Result<Self, ModelError> {
        geometry.validate()?;
        if !(8..=64).contains(&elements) || !elements.is_power_of_two() {
            return Err(ModelError(
                "tine elements must be a power of two from 8 to 64",
            ));
        }
        let n = 2 * elements;
        let mut k = vec![0.0; n * n];
        let mut m = vec![0.0; n * n];
        let h = 1.0 / elements as f64;
        // Dimensionless s=x/L and nodal coordinates [w, dw/ds].
        let ke = [
            [12.0, 6.0 * h, -12.0, 6.0 * h],
            [6.0 * h, 4.0 * h * h, -6.0 * h, 2.0 * h * h],
            [-12.0, -6.0 * h, 12.0, -6.0 * h],
            [6.0 * h, 2.0 * h * h, -6.0 * h, 4.0 * h * h],
        ];
        let me = [
            [156.0, 22.0 * h, 54.0, -13.0 * h],
            [22.0 * h, 4.0 * h * h, 13.0 * h, -3.0 * h * h],
            [54.0, 13.0 * h, 156.0, -22.0 * h],
            [-13.0 * h, -3.0 * h * h, -22.0 * h, 4.0 * h * h],
        ];
        for element in 0..elements {
            let (mut local_k, mut local_m) = (ke, me);
            if geometry.tip_diameter_ratio != 1.0 {
                local_k = [[0.0; 4]; 4];
                local_m = [[0.0; 4]; 4];
                for (t, weight) in geometry.section_rule(elements, element) {
                    let s = (element as f64 + t) * h;
                    let d = geometry.section_ratio(s);
                    let (_, shape) = interpolation(elements, s);
                    // Curvatures with respect to s, multiplied by h^2.
                    let curvature = [
                        12.0 * t - 6.0,
                        h * (6.0 * t - 4.0),
                        6.0 - 12.0 * t,
                        h * (6.0 * t - 2.0),
                    ];
                    for i in 0..4 {
                        for j in 0..4 {
                            local_k[i][j] += 0.5 * weight * d.powi(4) * curvature[i] * curvature[j];
                            local_m[i][j] += 210.0 * weight * d.powi(2) * shape[i] * shape[j];
                        }
                    }
                }
            }
            for i in 0..4 {
                for j in 0..4 {
                    let (gi, gj) = (2 * element + i, 2 * element + j);
                    if gi >= 2 && gj >= 2 {
                        k[(gi - 2) * n + gj - 2] += local_k[i][j] / h.powi(3);
                        m[(gi - 2) * n + gj - 2] += local_m[i][j] * h / 420.0;
                    }
                }
            }
        }
        let spring_points = geometry.tuning_mass_quadrature(elements)?;
        let mass_ratio = geometry.tuning_mass_kg / geometry.reference_mass_kg();
        // Point evaluation or an integrated finite span, without snapping to nodes.
        for &(position, fraction) in &spring_points {
            let (element, weights) = interpolation(elements, position);
            for i in 0..4 {
                for j in 0..4 {
                    let (gi, gj) = (2 * element + i, 2 * element + j);
                    if gi >= 2 && gj >= 2 {
                        m[(gi - 2) * n + gj - 2] +=
                            (mass_ratio * fraction) * weights[i] * weights[j];
                    }
                }
            }
        }
        let (values, vectors) = eigen(&k, &m, n, TINE_MODE_COUNT)?;
        let mass = geometry.reference_mass_kg();
        let rigidity = geometry.bending_rigidity_n_m2();
        let tuning_x = geometry.tuning_position * geometry.length_m;
        let mut result = Self {
            modes: [TineMode::default(); TINE_MODE_COUNT],
            total_mass_kg: geometry.beam_mass_kg() + geometry.tuning_mass_kg,
            first_mass_moment_kg_m: mass * geometry.length_m * geometry.relative_mass_moment(1)
                + geometry.tuning_mass_kg * tuning_x,
            second_mass_moment_kg_m2: if geometry.tip_diameter_ratio == 1.0 {
                mass * geometry.length_m.powi(2) / 3.0
            } else {
                mass * geometry.length_m.powi(2) * geometry.relative_mass_moment(2)
            } + geometry.tuning_mass_kg
                * (tuning_x.powi(2) + geometry.tuning_span_m.powi(2) / 12.0),
            maximum_mass_orthogonality_error: 0.0,
            elements,
            shapes: [[0.0; MAX_DOFS]; TINE_MODE_COUNT],
        };
        for index in 0..TINE_MODE_COUNT {
            let phi = &vectors[index];
            let tip = phi[n - 2];
            if !tip.is_finite() || tip.abs() < 1e-8 {
                return Err(ModelError("tine tip normalization is ill-conditioned"));
            }
            for (i, &value) in phi.iter().enumerate() {
                result.shapes[index][i + 2] = value / tip;
            }
            let kp = apply(&k, phi, n);
            let mp = apply(&m, phi, n);
            let lambda = inner(phi, &kp) / inner(phi, &mp);
            if !lambda.is_finite() || lambda <= 0.0 || values[index] <= 0.0 {
                return Err(ModelError(
                    "tine eigensolver produced a nonpositive frequency",
                ));
            }
            let residual = kp
                .iter()
                .zip(&mp)
                .map(|(a, b)| (a - lambda * b).powi(2))
                .sum::<f64>()
                .sqrt()
                / (inner(&kp, &kp).sqrt() + lambda * inner(&mp, &mp).sqrt());
            if !residual.is_finite() || residual > 1e-4 {
                return Err(ModelError("tine eigen residual exceeded 1e-4"));
            }
            let effective_mass = mass * inner(phi, &mp) / tip.powi(2);
            let (mut translation, mut rotation) = (0.0, 0.0);
            if geometry.tip_diameter_ratio != 1.0 {
                for (s, weight) in geometry.beam_mass_quadrature(elements)? {
                    let shape = sample(&result.shapes[index], elements, s);
                    translation += weight * shape;
                    rotation += weight * geometry.length_m * s * shape;
                }
            } else {
                let node = (3.0_f64 / 5.0).sqrt();
                for e in 0..elements {
                    for (t, w) in [
                        (0.5 * (1.0 - node), 5.0 / 18.0),
                        (0.5, 4.0 / 9.0),
                        (0.5 * (1.0 + node), 5.0 / 18.0),
                    ] {
                        let s = (e as f64 + t) * h;
                        let shape = sample(&result.shapes[index], elements, s);
                        translation += mass * h * w * shape;
                        rotation += mass * geometry.length_m * h * w * s * shape;
                    }
                }
            }
            for &(position, fraction) in &spring_points {
                let spring_shape = sample(&result.shapes[index], elements, position);
                translation += (geometry.tuning_mass_kg * fraction) * spring_shape;
                rotation += (geometry.tuning_mass_kg * fraction)
                    * (position * geometry.length_m)
                    * spring_shape;
            }
            result.modes[index] = TineMode {
                frequency_hz: (lambda * rigidity / (mass * geometry.length_m.powi(3))).sqrt() / TAU,
                effective_mass_kg: effective_mass,
                hammer_weight: sample(&result.shapes[index], elements, geometry.hammer_position),
                pickup_weight: sample(&result.shapes[index], elements, geometry.pickup_position),
                translation_coupling_kg: translation,
                rotation_coupling_kg_m: rotation,
                relative_eigen_residual: residual,
            };
        }
        for i in 0..TINE_MODE_COUNT {
            for j in 0..i {
                let a = &vectors[i];
                let b = &vectors[j];
                let ma = apply(&m, a, n);
                let mb = apply(&m, b, n);
                let cross = inner(a, &mb).abs() / (inner(a, &ma) * inner(b, &mb)).sqrt();
                result.maximum_mass_orthogonality_error =
                    result.maximum_mass_orthogonality_error.max(cross);
            }
        }
        if result.maximum_mass_orthogonality_error > 1e-8 {
            return Err(ModelError("tine modes failed mass orthogonality"));
        }
        // The exported moving-root reduction must itself have positive kinetic
        // energy, including the numerically integrated modal cross coefficients.
        let reduced = result.moving_root_mass_matrix();
        let mut factor = [[0.0; 8]; 8];
        for i in 0..8 {
            for j in 0..=i {
                let value =
                    reduced[i][j] - (0..j).map(|k| factor[i][k] * factor[j][k]).sum::<f64>();
                factor[i][j] = if i == j {
                    if !value.is_finite() || value <= 0.0 {
                        return Err(ModelError(
                            "tine moving-root inertia is not positive definite",
                        ));
                    }
                    value.sqrt()
                } else {
                    value / factor[j][j]
                };
            }
        }
        Ok(result)
    }

    pub fn shape(&self, mode: usize, position: f64) -> Result<f64, ModelError> {
        if mode >= TINE_MODE_COUNT || !position.is_finite() || !(0.0..=1.0).contains(&position) {
            return Err(ModelError("invalid tine mode or observation position"));
        }
        Ok(sample(&self.shapes[mode], self.elements, position))
    }

    /// Coordinates [root translation, root angle, six tip-normalized modal q].
    /// Reciprocal inertial coupling includes the beam and tuning mass once.
    /// A host assembly must add its other components' inertia, not this mass again.
    pub fn moving_root_mass_matrix(&self) -> [[f64; 8]; 8] {
        let mut mass = [[0.0; 8]; 8];
        mass[0][0] = self.total_mass_kg;
        mass[0][1] = self.first_mass_moment_kg_m;
        mass[1][0] = self.first_mass_moment_kg_m;
        mass[1][1] = self.second_mass_moment_kg_m2;
        for (i, mode) in self.modes.iter().enumerate() {
            mass[i + 2][i + 2] = mode.effective_mass_kg;
            mass[0][i + 2] = mode.translation_coupling_kg;
            mass[i + 2][0] = mode.translation_coupling_kg;
            mass[1][i + 2] = mode.rotation_coupling_kg_m;
            mass[i + 2][1] = mode.rotation_coupling_kg_m;
        }
        mass
    }
}

fn interpolation(elements: usize, s: f64) -> (usize, [f64; 4]) {
    let e = ((s * elements as f64) as usize).min(elements - 1);
    let t = s * elements as f64 - e as f64;
    let h = 1.0 / elements as f64;
    (
        e,
        [
            1.0 - 3.0 * t * t + 2.0 * t * t * t,
            h * (t - 2.0 * t * t + t * t * t),
            3.0 * t * t - 2.0 * t * t * t,
            h * (-t * t + t * t * t),
        ],
    )
}
fn sample(shape: &[f64; MAX_DOFS], elements: usize, s: f64) -> f64 {
    let (e, weights) = interpolation(elements, s);
    weights
        .into_iter()
        .enumerate()
        .map(|(i, w)| w * shape[2 * e + i])
        .sum()
}
fn inner(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn apply(a: &[f64], x: &[f64], n: usize) -> Vec<f64> {
    a.chunks_exact(n).map(|row| inner(row, x)).collect()
}

/// Bounded dense generalized symmetric eigensolve, independent of render solvers.
/// Cholesky mass whitening, cyclic Jacobi, then back-transform the eigenvectors.
pub(crate) fn eigen(
    k: &[f64],
    m: &[f64],
    n: usize,
    count: usize,
) -> Result<(Vec<f64>, Vec<Vec<f64>>), ModelError> {
    if n == 0
        || n > MAX_DOFS
        || count == 0
        || count > n
        || k.len() != n * n
        || m.len() != n * n
        || !k.iter().chain(m).all(|x| x.is_finite())
    {
        return Err(ModelError(
            "invalid generalized eigenproblem dimensions or values",
        ));
    }
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let value = m[i * n + j] - (0..j).map(|s| l[i * n + s] * l[j * n + s]).sum::<f64>();
            l[i * n + j] = if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError("tine mass matrix is not positive definite"));
                }
                value.sqrt()
            } else {
                value / l[j * n + j]
            };
        }
    }
    let mut inv = vec![0.0; n * n];
    for j in 0..n {
        for i in j..n {
            inv[i * n + j] = (f64::from(i == j)
                - (j..i).map(|s| l[i * n + s] * inv[s * n + j]).sum::<f64>())
                / l[i * n + i];
        }
    }
    let mut temp = vec![0.0; n * n];
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            temp[i * n + j] = (0..n).map(|s| inv[i * n + s] * k[s * n + j]).sum();
        }
    }
    for i in 0..n {
        for j in 0..=i {
            let value = (0..n).map(|s| temp[i * n + s] * inv[j * n + s]).sum();
            a[i * n + j] = value;
            a[j * n + i] = value;
        }
    }
    let mut q = vec![0.0; n * n];
    for i in 0..n {
        q[i * n + i] = 1.0;
    }
    let mut converged = false;
    for _ in 0..64 {
        let mut changed = false;
        for p in 0..n {
            for r in (p + 1)..n {
                let off = a[p * n + r];
                if off.abs() <= 1e-14 * (a[p * n + p].abs() * a[r * n + r].abs()).sqrt() {
                    continue;
                }
                changed = true;
                let tau = (a[r * n + r] - a[p * n + p]) / (2.0 * off);
                let t = if tau >= 0.0 { 1.0 } else { -1.0 } / (tau.abs() + tau.hypot(1.0));
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                a[p * n + p] -= t * off;
                a[r * n + r] += t * off;
                a[p * n + r] = 0.0;
                a[r * n + p] = 0.0;
                for i in 0..n {
                    if i != p && i != r {
                        let x = a[i * n + p];
                        let y = a[i * n + r];
                        a[i * n + p] = c * x - s * y;
                        a[p * n + i] = a[i * n + p];
                        a[i * n + r] = s * x + c * y;
                        a[r * n + i] = a[i * n + r];
                    }
                    let x = q[i * n + p];
                    let y = q[i * n + r];
                    q[i * n + p] = c * x - s * y;
                    q[i * n + r] = s * x + c * y;
                }
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(ModelError(
            "tine Jacobi eigensolver did not converge in 64 sweeps",
        ));
    }
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|&i, &j| a[i * n + i].total_cmp(&a[j * n + j]));
    let mut values = Vec::new();
    let mut vectors = Vec::new();
    for &index in order.iter().take(count) {
        values.push(a[index * n + index]);
        vectors.push(
            (0..n)
                .map(|i| (0..n).map(|j| inv[j * n + i] * q[j * n + index]).sum())
                .collect(),
        );
    }
    Ok((values, vectors))
}

#[cfg(test)]
mod tests {
    #[test]
    fn localized_transition_preserves_exact_mass_moments_and_static_flexibility() {
        for end in [0.137, 0.25, 0.5, 1.0] {
            let r = 0.9;
            let g = TineGeometry {
                tip_diameter_ratio: r,
                taper_end_fraction: end,
                tuning_mass_kg: 0.0,
                ..TineGeometry::default()
            };
            let m0 = g.density_kg_m3 * PI * g.diameter_m.powi(2) * g.length_m / 4.0;
            // Frustum on [0,end], cylinder on [end,1], with all moments about the root.
            let exact = [
                m0 * (end * (1.0 + r + r * r) / 3.0 + r * r * (1.0 - end)),
                m0 * g.length_m
                    * (end.powi(2) * (1.0 + 2.0 * r + 3.0 * r * r) / 12.0
                        + r * r * (1.0 - end.powi(2)) / 2.0),
                m0 * g.length_m.powi(2)
                    * (end.powi(3) * (1.0 + 3.0 * r + 6.0 * r * r) / 30.0
                        + r * r * (1.0 - end.powi(3)) / 3.0),
            ];
            for elements in [8, 16, 32, 64] {
                let q = g.beam_mass_quadrature(elements).unwrap();
                assert!(q.len() <= 5 * (elements + 1));
                for order in 0..=2 {
                    let sum: f64 = q
                        .iter()
                        .map(|(s, m)| m * (s * g.length_m).powi(order))
                        .sum();
                    assert!((sum / exact[order as usize] - 1.0).abs() < 2e-14);
                }
            }
            let b = TineModes::prepare(g, 64).unwrap();
            for (value, target) in [
                b.total_mass_kg,
                b.first_mass_moment_kg_m,
                b.second_mass_moment_kg_m2,
            ]
            .into_iter()
            .zip(exact)
            {
                assert!((value / target - 1.0).abs() < 2e-14);
            }
            // Independent composite midpoint integral of static bending flexibility.
            let integral: f64 = (0..20000)
                .map(|i| {
                    let s = (i as f64 + 0.5) / 20000.0;
                    let d = if s < end {
                        1.0 + (r - 1.0) * s / end
                    } else {
                        r
                    };
                    (1.0 - s).powi(2) / d.powi(4) / 20000.0
                })
                .sum();
            let target = integral * g.length_m.powi(3) / g.bending_rigidity_n_m2();
            let modal: f64 = b
                .modes
                .iter()
                .map(|m| 1.0 / (m.effective_mass_kg * (TAU * m.frequency_hz).powi(2)))
                .sum();
            assert!(
                modal < target && (modal / target - 1.0).abs() < 0.001,
                "end {end}: {modal}/{target}"
            );
        }
    }
    #[test]
    fn off_mesh_transition_converges_is_continuous_and_uniform_limit_is_exact() {
        let g = TineGeometry {
            tip_diameter_ratio: 0.9,
            taper_end_fraction: 0.137,
            tuning_span_m: 0.006,
            ..TineGeometry::default()
        };
        let fine = TineModes::prepare(g, 64).unwrap();
        let medium = TineModes::prepare(g, 32).unwrap();
        let coarse = TineModes::prepare(g, 16).unwrap();
        let error = |b: &TineModes| {
            b.modes
                .iter()
                .zip(fine.modes)
                .map(|(a, b)| (a.frequency_hz / b.frequency_hz - 1.0).abs())
                .fold(0.0_f64, f64::max)
        };
        assert!(
            error(&medium) < 0.0003 && error(&medium) < error(&coarse),
            "errors {} / {}",
            error(&medium),
            error(&coarse)
        );
        let left = TineModes::prepare(
            TineGeometry {
                taper_end_fraction: 0.25 - 1e-7,
                ..g
            },
            32,
        )
        .unwrap();
        let right = TineModes::prepare(
            TineGeometry {
                taper_end_fraction: 0.25 + 1e-7,
                ..g
            },
            32,
        )
        .unwrap();
        for (a, b) in left.modes.into_iter().zip(right.modes) {
            assert!((a.frequency_hz / b.frequency_hz - 1.0).abs() < 1e-6);
        }
        let a = TineModes::prepare(TineGeometry::default(), 64).unwrap();
        let b = TineModes::prepare(
            TineGeometry {
                taper_end_fraction: 0.137,
                ..TineGeometry::default()
            },
            64,
        )
        .unwrap();
        for (a, b) in a.modes.into_iter().zip(b.modes) {
            assert_eq!(a.frequency_hz, b.frequency_hz);
            assert_eq!(a.effective_mass_kg, b.effective_mass_kg);
        }
        for end in [0.0, 0.049, 1.001, f64::NAN, f64::INFINITY] {
            assert!(
                TineGeometry {
                    taper_end_fraction: end,
                    ..g
                }
                .validate()
                .is_err()
            );
        }
    }
    #[test]
    fn tapered_frustum_mass_moments_and_tip_compliance_match_analytic_values() {
        for r in [0.5, 0.9, 1.0, 1.5] {
            let g = TineGeometry {
                tip_diameter_ratio: r,
                tuning_mass_kg: 0.0,
                ..TineGeometry::default()
            };
            let b = TineModes::prepare(g, 64).unwrap();
            let m0 = g.density_kg_m3 * PI * g.diameter_m.powi(2) * g.length_m / 4.0;
            let expected = [
                m0 * (1.0 + r + r * r) / 3.0,
                m0 * g.length_m * (1.0 + 2.0 * r + 3.0 * r * r) / 12.0,
                m0 * g.length_m.powi(2) * (1.0 + 3.0 * r + 6.0 * r * r) / 30.0,
            ];
            for (actual, exact) in [
                b.total_mass_kg,
                b.first_mass_moment_kg_m,
                b.second_mass_moment_kg_m2,
            ]
            .into_iter()
            .zip(expected)
            {
                assert!((actual / exact - 1.0).abs() < 2e-14);
            }
            let q = g.beam_mass_quadrature(64).unwrap();
            for order in 0..=2 {
                let actual: f64 = q
                    .iter()
                    .map(|(s, m)| m * (s * g.length_m).powi(order))
                    .sum();
                assert!((actual / expected[order as usize] - 1.0).abs() < 2e-14);
            }
            // Exact static tip flexibility: integral (L-x)^2/[E I(x)] dx = L^3/(3 EI_root r).
            // Six retained modes approach it from below; this checks stiffness independently of mass.
            let exact = g.length_m.powi(3) / (3.0 * g.bending_rigidity_n_m2() * r);
            let modal: f64 = b
                .modes
                .iter()
                .map(|m| 1.0 / (m.effective_mass_kg * (TAU * m.frequency_hz).powi(2)))
                .sum();
            assert!(
                modal < exact && (modal / exact - 1.0).abs() < 0.001,
                "ratio {r}, compliance {modal}/{exact}"
            );
        }
    }

    #[test]
    fn tapered_modes_converge_scale_and_reject_invalid_sections() {
        for r in [0.5, 0.9, 1.05, 1.5] {
            let g = TineGeometry {
                tip_diameter_ratio: r,
                tuning_span_m: 0.006,
                ..TineGeometry::default()
            };
            let fine = TineModes::prepare(g, 64).unwrap();
            let medium = TineModes::prepare(g, 32).unwrap();
            let coarse = TineModes::prepare(g, 16).unwrap();
            let error = |b: &TineModes| {
                b.modes
                    .iter()
                    .zip(fine.modes)
                    .map(|(a, b)| (a.frequency_hz / b.frequency_hz - 1.0).abs())
                    .fold(0.0_f64, f64::max)
            };
            assert!(
                error(&medium) < 0.0002 && error(&medium) < 0.2 * error(&coarse),
                "ratio {r}: {} / {}",
                error(&medium),
                error(&coarse)
            );
            let scaled = TineModes::prepare(
                TineGeometry {
                    length_m: 2.0 * g.length_m,
                    tuning_mass_kg: 2.0 * g.tuning_mass_kg,
                    tuning_span_m: 2.0 * g.tuning_span_m,
                    ..g
                },
                64,
            )
            .unwrap();
            for (a, b) in fine.modes.into_iter().zip(scaled.modes) {
                assert!((b.frequency_hz / a.frequency_hz - 0.25).abs() < 1e-10);
                assert!((b.effective_mass_kg / a.effective_mass_kg - 2.0).abs() < 1e-10);
            }
        }
        for r in [f64::NAN, f64::INFINITY, 0.0, 0.49, 1.51] {
            assert!(
                TineGeometry {
                    tip_diameter_ratio: r,
                    ..TineGeometry::default()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            TineGeometry {
                diameter_m: 0.003,
                tip_diameter_ratio: 1.5,
                ..TineGeometry::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            TineGeometry {
                length_m: 0.02,
                diameter_m: 0.0015,
                tip_diameter_ratio: 1.5,
                ..TineGeometry::default()
            }
            .validate()
            .is_err()
        );
        assert!(TineGeometry::default().beam_mass_quadrature(12).is_err());
    }

    #[test]
    fn finite_span_quadrature_preserves_polynomial_mass_moments_and_bounds() {
        let g = TineGeometry {
            tuning_position: 0.637,
            tuning_span_m: 0.006,
            ..TineGeometry::default()
        };
        let half = g.tuning_span_m / (2.0 * g.length_m);
        for elements in [8, 16, 32, 64] {
            let q = g.tuning_mass_quadrature(elements).unwrap();
            assert!(
                q.len() <= 4 * elements
                    && q.iter()
                        .all(|(s, w)| *w > 0.0 && (*s - g.tuning_position).abs() <= half)
            );
            for k in 0..=7 {
                let exact = ((g.tuning_position + half).powi(k + 1)
                    - (g.tuning_position - half).powi(k + 1))
                    / (2.0 * half * (k + 1) as f64);
                let actual: f64 = q.iter().map(|(s, w)| w * s.powi(k)).sum();
                assert!((actual - exact).abs() < 2e-14);
            }
        }
        for span in [-0.001, f64::NAN, 1e-300, 0.08] {
            assert!(
                TineGeometry {
                    tuning_span_m: span,
                    ..g
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            TineGeometry {
                tuning_position: 0.99,
                ..g
            }
            .validate()
            .is_err()
        );
        assert!(g.tuning_mass_quadrature(12).is_err());
        assert_eq!(
            TineGeometry::default().tuning_mass_quadrature(64).unwrap(),
            vec![(0.85, 1.0)]
        );
    }
    #[test]
    fn full_length_mass_span_matches_uniform_density_and_analytic_root_inertia() {
        let mut g = TineGeometry {
            tuning_position: 0.5,
            tuning_span_m: 0.075,
            ..TineGeometry::default()
        };
        g.tuning_mass_kg = 0.3 * g.beam_mass_kg();
        let a = TineModes::prepare(g, 64).unwrap();
        let b = TineModes::prepare(
            TineGeometry {
                density_kg_m3: 1.3 * g.density_kg_m3,
                tuning_mass_kg: 0.0,
                tuning_span_m: 0.0,
                ..g
            },
            64,
        )
        .unwrap();
        assert!((a.total_mass_kg / b.total_mass_kg - 1.0).abs() < 1e-14);
        assert!((a.first_mass_moment_kg_m / b.first_mass_moment_kg_m - 1.0).abs() < 1e-14);
        assert!((a.second_mass_moment_kg_m2 / b.second_mass_moment_kg_m2 - 1.0).abs() < 1e-14);
        for i in 0..6 {
            assert!((a.modes[i].frequency_hz / b.modes[i].frequency_hz - 1.0).abs() < 1e-6);
            for x in [0.1, 0.37, 0.83, 1.0] {
                assert!((a.shape(i, x).unwrap() - b.shape(i, x).unwrap()).abs() < 1e-5);
            }
        }
    }
    #[test]
    fn shrinking_span_recovers_point_mass_and_finite_span_converges_with_mesh() {
        let g = TineGeometry::default();
        let point = TineModes::prepare(g, 64).unwrap();
        let tiny = TineModes::prepare(
            TineGeometry {
                tuning_span_m: 1e-6,
                ..g
            },
            64,
        )
        .unwrap();
        for (a, b) in point.modes.iter().zip(tiny.modes) {
            assert!((a.frequency_hz / b.frequency_hz - 1.0).abs() < 1e-6);
        }
        let g = TineGeometry {
            tuning_position: 0.793,
            tuning_span_m: 0.006,
            ..g
        };
        let fine = TineModes::prepare(g, 64).unwrap();
        let middle = TineModes::prepare(g, 32).unwrap();
        let coarse = TineModes::prepare(g, 16).unwrap();
        let err = |a: &TineModes| {
            a.modes
                .iter()
                .zip(fine.modes)
                .map(|(a, b)| (a.frequency_hz / b.frequency_hz - 1.0).abs())
                .fold(0.0_f64, f64::max)
        };
        assert!(err(&middle) < err(&coarse) * 0.2);
        assert!(err(&middle) < 1e-4);
        let point = TineModes::prepare(
            TineGeometry {
                tuning_span_m: 0.0,
                ..g
            },
            64,
        )
        .unwrap();
        assert!(
            (fine.second_mass_moment_kg_m2
                - point.second_mass_moment_kg_m2
                - g.tuning_mass_kg * g.tuning_span_m.powi(2) / 12.0)
                .abs()
                < 1e-20
        );
    }
    use super::*;
    const ROOTS: [f64; 6] = [
        1.875104068711961,
        4.694091132974174,
        7.854757438237612,
        10.995540734875467,
        14.13716839104647,
        17.278759532088237,
    ];

    #[test]
    fn unladen_beam_converges_to_analytic_modes_shapes_and_static_compliance() {
        let g = TineGeometry {
            tuning_mass_kg: 0.0,
            ..TineGeometry::default()
        };
        let scale =
            (g.bending_rigidity_n_m2() / (g.beam_mass_kg() * g.length_m.powi(3))).sqrt() / TAU;
        let mut previous = f64::INFINITY;
        for elements in [16, 32, 64] {
            let basis = TineModes::prepare(g, elements).unwrap();
            let mut worst = 0.0_f64;
            for (i, beta) in ROOTS.into_iter().enumerate() {
                let error = (basis.modes[i].frequency_hz / (scale * beta * beta) - 1.0).abs();
                worst = worst.max(error);
                assert_eq!(basis.shape(i, 0.0).unwrap(), 0.0);
                assert!((basis.shape(i, 1.0).unwrap() - 1.0).abs() < 1e-12);
                if elements == 64 {
                    assert!(error < 5e-6, "mode {i}, error {error}");
                    assert!(
                        (basis.modes[i].effective_mass_kg / (g.beam_mass_kg() / 4.0) - 1.0).abs()
                            < 5e-5
                    );
                    let sigma = (beta.cosh() + beta.cos()) / (beta.sinh() + beta.sin());
                    let analytic = |s: f64| {
                        (beta * s).cosh()
                            - (beta * s).cos()
                            - sigma * ((beta * s).sinh() - (beta * s).sin())
                    };
                    for s in [0.13, 0.37, 0.73, 0.98] {
                        assert!(
                            (basis.shape(i, s).unwrap() - analytic(s) / analytic(1.0)).abs() < 5e-5
                        );
                    }
                }
            }
            assert!(
                worst < previous * 0.1,
                "mesh {elements}, {worst}, previous {previous}"
            );
            previous = worst;
        }
        let basis = TineModes::prepare(g, 64).unwrap();
        let compliance: f64 = basis
            .modes
            .iter()
            .map(|m| 1.0 / (m.effective_mass_kg * (TAU * m.frequency_hz).powi(2)))
            .sum();
        let exact = g.length_m.powi(3) / (3.0 * g.bending_rigidity_n_m2());
        assert!(compliance < exact);
        assert!((compliance / exact - 1.0).abs() < 0.0003);
    }

    #[test]
    fn tuning_mass_changes_modes_without_snapping_to_nodes() {
        let g = TineGeometry {
            tuning_mass_kg: 0.0,
            ..TineGeometry::default()
        };
        let plain = TineModes::prepare(g, 32).unwrap();
        let root = TineModes::prepare(
            TineGeometry {
                tuning_mass_kg: g.beam_mass_kg() * 0.5,
                tuning_position: 0.0,
                ..g
            },
            32,
        )
        .unwrap();
        let middle = TineModes::prepare(
            TineGeometry {
                tuning_mass_kg: g.beam_mass_kg() * 0.5,
                tuning_position: 0.5,
                ..g
            },
            32,
        )
        .unwrap();
        let tip = TineModes::prepare(
            TineGeometry {
                tuning_mass_kg: g.beam_mass_kg() * 0.5,
                tuning_position: 1.0,
                ..g
            },
            32,
        )
        .unwrap();
        for i in 0..6 {
            assert_eq!(root.modes[i].frequency_hz, plain.modes[i].frequency_hz);
            assert!(tip.modes[i].frequency_hz < plain.modes[i].frequency_hz);
        }
        assert!(tip.modes[0].frequency_hz < middle.modes[0].frequency_hz);
        let a = TineModes::prepare(
            TineGeometry {
                tuning_position: 0.37,
                ..TineGeometry::default()
            },
            32,
        )
        .unwrap();
        let b = TineModes::prepare(
            TineGeometry {
                tuning_position: 0.370001,
                ..TineGeometry::default()
            },
            32,
        )
        .unwrap();
        assert!(a.modes[0].frequency_hz != b.modes[0].frequency_hz);
        assert!((a.modes[0].frequency_hz / b.modes[0].frequency_hz - 1.0).abs() < 1e-5);
    }

    #[test]
    fn tip_mass_modes_satisfy_the_continuous_dynamic_boundary_condition() {
        for ratio in [0.1, 0.5, 5.0] {
            let mut g = TineGeometry {
                tuning_position: 1.0,
                ..TineGeometry::default()
            };
            g.tuning_mass_kg = ratio * g.beam_mass_kg();
            let basis = TineModes::prepare(g, 64).unwrap();
            for (i, mode) in basis.modes.into_iter().enumerate() {
                let beta =
                    ((TAU * mode.frequency_hz).powi(2) * g.beam_mass_kg() * g.length_m.powi(3)
                        / g.bending_rigidity_n_m2())
                    .powf(0.25);
                // Fixed root; zero tip bending moment; dynamic tip shear from
                // a translational point mass, with no rotary inertia.
                let characteristic =
                    |b: f64| 1.0 / b.cosh() + b.cos() + ratio * b * (b.tanh() * b.cos() - b.sin());
                let (mut low, mut high) = (i as f64 * PI, (i + 1) as f64 * PI);
                let sign = characteristic(low).signum();
                assert!(sign != characteristic(high).signum());
                for _ in 0..80 {
                    let mid = (low + high) / 2.0;
                    if characteristic(mid).signum() == sign {
                        low = mid;
                    } else {
                        high = mid;
                    }
                }
                let exact = (low + high) / 2.0;
                // Check frequency error, rather than an unscaled characteristic
                // residual whose slope grows substantially with mode number.
                assert!(
                    ((beta / exact).powi(2) - 1.0).abs() < 6e-6,
                    "ratio {ratio}, mode {i}, beta {beta}, exact {exact}"
                );
            }
        }
    }

    #[test]
    fn geometry_scaling_and_ports_follow_the_same_shapes() {
        let g = TineGeometry::default();
        let a = TineModes::prepare(g, 16).unwrap();
        let b = TineModes::prepare(
            TineGeometry {
                length_m: g.length_m * 2.0,
                tuning_mass_kg: g.tuning_mass_kg * 2.0,
                ..g
            },
            16,
        )
        .unwrap();
        let c = TineModes::prepare(
            TineGeometry {
                hammer_position: g.pickup_position,
                pickup_position: g.hammer_position,
                ..g
            },
            16,
        )
        .unwrap();
        for i in 0..6 {
            let (a, b, c) = (a.modes[i], b.modes[i], c.modes[i]);
            assert!((b.frequency_hz / a.frequency_hz - 0.25).abs() < 1e-12);
            assert!((b.effective_mass_kg / a.effective_mass_kg - 2.0).abs() < 1e-12);
            assert!((b.rotation_coupling_kg_m / a.rotation_coupling_kg_m - 4.0).abs() < 1e-12);
            assert_eq!(a.frequency_hz, c.frequency_hz);
            assert_eq!(a.hammer_weight, c.pickup_weight);
            assert_eq!(a.pickup_weight, c.hammer_weight);
        }
    }

    #[test]
    fn moving_root_inertia_is_reciprocal_positive_and_contains_point_mass_once() {
        for position in [0.0, 0.37, 1.0] {
            let g = TineGeometry {
                tuning_position: position,
                ..TineGeometry::default()
            };
            let basis = TineModes::prepare(g, 32).unwrap();
            let matrix = basis.moving_root_mass_matrix();
            assert!((matrix[0][0] - g.beam_mass_kg() - g.tuning_mass_kg).abs() < 1e-16);
            let mut l = [[0.0; 8]; 8];
            for i in 0..8 {
                for j in 0..=i {
                    assert_eq!(matrix[i][j], matrix[j][i]);
                    let v = matrix[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
                    if i == j {
                        assert!(v > 0.0, "pivot {i}: {v}");
                        l[i][j] = v.sqrt();
                    } else {
                        l[i][j] = v / l[j][j];
                    }
                }
            }
            assert!(basis.maximum_mass_orthogonality_error < 1e-10);
        }
    }

    #[test]
    fn invalid_geometry_and_mesh_fail_before_preparation() {
        for elements in [0, 4, 9, 128, usize::MAX] {
            assert!(TineModes::prepare(TineGeometry::default(), elements).is_err());
        }
        for value in [f64::NAN, f64::INFINITY, -1.0] {
            let g = TineGeometry::default();
            for bad in [
                TineGeometry {
                    length_m: value,
                    ..g
                },
                TineGeometry {
                    diameter_m: value,
                    ..g
                },
                TineGeometry {
                    young_modulus_pa: value,
                    ..g
                },
                TineGeometry {
                    density_kg_m3: value,
                    ..g
                },
                TineGeometry {
                    tuning_mass_kg: value,
                    ..g
                },
                TineGeometry {
                    tuning_position: value,
                    ..g
                },
                TineGeometry {
                    hammer_position: value,
                    ..g
                },
                TineGeometry {
                    pickup_position: value,
                    ..g
                },
            ] {
                assert!(TineModes::prepare(bad, 16).is_err());
            }
        }
        assert!(
            TineModes::prepare(
                TineGeometry {
                    length_m: 0.015,
                    diameter_m: 0.003,
                    ..TineGeometry::default()
                },
                16
            )
            .is_err()
        );
        let basis = TineModes::prepare(TineGeometry::default(), 8).unwrap();
        assert!(basis.shape(6, 0.5).is_err());
        assert!(basis.shape(0, f64::NAN).is_err());
        assert!(basis.shape(0, 1.1).is_err());
    }
}
