//! Same-trajectory observation reduction; no mode extraction from source audio.
use super::*;
use rf_73_dsp::{ModalProbe, ModalSpectrum};
type Vector = [f64; 9];
pub const HELP: &str = "Reduced-state mechanical loss study:
  reduced-mechanical-loss --output REPORT.json
Same known-state strikes: full state, lowest 1/3/6/9 undamped modes, instantaneous pickup lift.
Retains biased and failed fits; modal coordinates are supplied by simulation, not audio.
";

fn apply(m: &Matrix, v: Vector) -> Vector {
    core::array::from_fn(|i| m[i].iter().zip(v).map(|(a, b)| a * b).sum())
}
#[cfg(test)]
fn dot(a: Vector, b: Vector) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
struct Projection {
    label: String,
    matrix: Matrix,
    pickup_lift: Option<Vector>,
}
impl Projection {
    fn state(&self, p: &ModalProbe) -> (Vector, Vector) {
        if let Some(lift) = self.pickup_lift {
            (
                lift.map(|h| h * p.pickup_displacement_m),
                lift.map(|h| h * p.pickup_velocity_m_s),
            )
        } else {
            (
                apply(&self.matrix, p.position),
                apply(&self.matrix, p.velocity),
            )
        }
    }
}
fn projections(s: &ModalSpectrum) -> Vec<Projection> {
    let mut all = vec![Projection {
        label: "full_state".into(),
        matrix: core::array::from_fn(|i| core::array::from_fn(|j| f64::from(i == j))),
        pickup_lift: None,
    }];
    for count in [1, 3, 6, 9] {
        let mut matrix = [[0.0; 9]; 9];
        for mode in &s.modes[..count] {
            let weighted = apply(&s.mass_matrix, mode.shape);
            for (i, row) in matrix.iter_mut().enumerate() {
                for (entry, weight) in row.iter_mut().zip(weighted) {
                    *entry += mode.shape[i] * weight;
                }
            }
        }
        all.push(Projection {
            label: format!("lowest_{count}_modes"),
            matrix,
            pickup_lift: None,
        });
    }
    let norm = s.modes.iter().map(|m| m.pickup_weight.powi(2)).sum::<f64>();
    let lift = core::array::from_fn(|i| {
        s.modes
            .iter()
            .map(|m| m.shape[i] * m.pickup_weight)
            .sum::<f64>()
            / norm
    });
    all.push(Projection {
        label: "instantaneous_pickup_lift".into(),
        matrix: [[0.0; 9]; 9],
        pickup_lift: Some(lift),
    });
    all
}
struct Accumulator {
    projection: Projection,
    rows: Vec<Row>,
    start_energy: f64,
    a: f64,
    b: f64,
    contact_free: bool,
    error_q: f64,
    error_v: f64,
    total_q: f64,
    total_v: f64,
    reconstructed_energy: f64,
    total_energy: f64,
}
impl Accumulator {
    fn new(projection: Projection) -> Self {
        Self {
            projection,
            rows: Vec::new(),
            start_energy: 0.0,
            a: 0.0,
            b: 0.0,
            contact_free: true,
            error_q: 0.0,
            error_v: 0.0,
            total_q: 0.0,
            total_v: 0.0,
            reconstructed_energy: 0.0,
            total_energy: 0.0,
        }
    }
    fn step(
        &mut self,
        tick: usize,
        tick_rate: usize,
        before: &ModalProbe,
        after: &ModalProbe,
        damped: bool,
        operators: &[Matrix; 4],
    ) {
        let [m, k, c, d] = operators;
        let edges = EDGES.map(|t| (t * tick_rate as f64).round() as usize);
        let index = self.rows.len();
        if index >= 6 || tick < edges[index] {
            return;
        }
        let dt = 1.0 / tick_rate as f64;
        let (q0, v0) = self.projection.state(before);
        let (q1, v1) = self.projection.state(after);
        if tick == edges[index] {
            self.start_energy = energy(q0, v0, m, k);
            self.a = 0.0;
            self.b = 0.0;
            self.contact_free = true;
        }
        self.a += 0.5 * dt * (quadratic(v0, c) + quadratic(v1, c));
        if damped {
            self.b += 0.5 * dt * (quadratic(v0, d) + quadratic(v1, d));
        }
        self.contact_free &= !before.contact_active && !after.contact_active;
        for (p, q, v) in [(before, q0, v0), (after, q1, v1)] {
            self.error_q += 0.5 * dt * quadratic(core::array::from_fn(|i| q[i] - p.position[i]), m);
            self.error_v += 0.5 * dt * quadratic(core::array::from_fn(|i| v[i] - p.velocity[i]), m);
            self.total_q += 0.5 * dt * quadratic(p.position, m);
            self.total_v += 0.5 * dt * quadratic(p.velocity, m);
            self.reconstructed_energy += 0.5 * dt * energy(q, v, m, k);
            self.total_energy += 0.5 * dt * energy(p.position, p.velocity, m, k);
        }
        if tick + 1 == edges[index + 1] {
            self.rows.push(Row {
                start_seconds: EDGES[index],
                end_seconds: EDGES[index + 1],
                energy_drop_j: self.start_energy - energy(q1, v1, m, k),
                structural_integral_j: self.a,
                damper_integral_j: self.b,
                contact_free: self.contact_free,
            });
        }
    }
    fn report(&self, a: f64, b: f64) -> Value {
        let fitting: Vec<_> = [0, 1, 3]
            .map(|i| self.rows[i].clone())
            .into_iter()
            .collect();
        let held: Vec<_> = [2, 4, 5].map(|i| &self.rows[i]).into_iter().collect();
        let all: Vec<_> = self.rows.iter().collect();
        let fit = match super::fit(&fitting) {
            Ok((x, y, pivot)) => {
                let validation = residual(&held, x, y);
                json!({"estimated_structural_scale":x,"estimated_damper_scale":y,"normalized_qr_pivot":pivot,
                    "held_out_relative_energy_rmse":validation,
                    "internally_consistent":x>0.0 && y>0.0 && validation<0.005 && self.rows.iter().all(|r|r.contact_free),
                    "known_scale_recovery":(x/a-1.0).abs()<0.01 && (y/b-1.0).abs()<0.01,
                    "structural_relative_error":(x/a-1.0).abs(),"damper_relative_error":(y/b-1.0).abs()})
            }
            Err(error) => {
                json!({"error":error.to_string(),"internally_consistent":false,"known_scale_recovery":false})
            }
        };
        json!({"observation":self.projection.label,"rows":self.rows,"fit":fit,
            "position_mass_relative_rmse":(self.error_q.max(0.0)/self.total_q).sqrt(),
            "velocity_mass_relative_rmse":(self.error_v.max(0.0)/self.total_v).sqrt(),
            "integrated_reconstructed_energy_ratio":self.reconstructed_energy/self.total_energy,
            "known_scale_energy_relative_rmse":residual(&all,a,b)})
    }
}

fn study() -> Result<Value, Box<dyn Error>> {
    let geometry = TineGeometry::default();
    let profile = ModalAssemblyProfile::default();
    let spectrum = ModalSpectrum::prepare(geometry, profile)?;
    let base = ModalAssembly::new(
        48000.0,
        geometry,
        profile,
        ModalIntegration::Refined {
            contact_substeps: 64,
        },
    )?;
    let c = base.damping_matrix(false);
    let on = base.damping_matrix(true);
    let d = core::array::from_fn(|i| core::array::from_fn(|j| on[i][j] - c[i][j]));
    let operators = [base.mass_matrix(), base.stiffness_matrix(), c, d];
    let mut cases = Vec::new();
    for (a, b) in [(0.5, 1.5), (1.0, 0.5), (1.5, 1.0)] {
        for rate in [48000, 96000] {
            let mut observations: Vec<_> = projections(&spectrum)
                .into_iter()
                .map(Accumulator::new)
                .collect();
            let take = simulate_observed(rate, a, b, |tick, tick_rate, before, after, damped| {
                for observation in &mut observations {
                    observation.step(tick, tick_rate, before, after, damped, &operators);
                }
            })?;
            let reports: Vec<_> = observations.iter().map(|o| o.report(a, b)).collect();
            let exact_rows =
                serde_json::to_value(&take.rows)? == serde_json::to_value(&observations[0].rows)?;
            let controls = exact_rows
                && take.diagnostics["mechanical_checks_passed"] == true
                && [0, 4].iter().all(|i| {
                    reports[*i]["fit"]["known_scale_recovery"] == true
                        && reports[*i]["fit"]["internally_consistent"] == true
                })
                && reports[4]["position_mass_relative_rmse"]
                    .as_f64()
                    .is_some_and(|e| e < 1e-8)
                && reports[4]["velocity_mass_relative_rmse"]
                    .as_f64()
                    .is_some_and(|e| e < 1e-8);
            cases.push(json!({"known_structural_scale":a,"known_damper_scale":b,"diagnostics":take.diagnostics,
                "full_state_rows_match_original":exact_rows,"controls_passed":controls,"observations":reports}));
        }
    }
    Ok(
        json!({"schema_version":1,"experiment":"reduced-state-mechanical-loss-v1",
        "controls_passed":cases.iter().all(|c|c["controls_passed"]==true),"cases":cases,
        "modes":spectrum.modes.iter().enumerate().map(|(i,m)|json!({"index":i,"frequency_hz":m.frequency_hz,"pickup_weight":m.pickup_weight})).collect::<Vec<_>>(),
        "fit_row_indices":[0,1,3],"held_out_row_indices":[2,4,5],
        "protocol":"Frozen before first run. Same six mechanical loss cases, known single elastic-hammer strike and 0.14 s binary damper event, 48/96 kHz. Every observation shares one physical trajectory per case. Identity control; lowest 1/3/6/9 undamped mass-normalized modes P=sum(phi*(M phi)^T); and instantaneous minimum-M-norm pickup lift h*y, with h=sum(phi*pickup_weight)/sum(pickup_weight^2), independently for displacement and velocity. Modal coordinates are exact projections of simulator state, not extracted from audio. All reconstructed states use the same M,K,C0,D endpoint energy and trapezoidal power formulas, fixed six intervals and fit/held-out split. Same two-column QR and thresholds: internal consistency requires positive scales, contact-free rows and held-out residual <0.005; known-scale recovery additionally requires both relative errors <1%. Only identity and nine-mode controls must pass; nine-mode mass-norm reconstruction errors <1e-8. Reduced outcomes, including negative scales, are descriptive and retained without retuning counts or gates.",
        "scope":"Known geometry/operator shapes and event history. Truncated-state energy balance omits energy/power exchange with discarded motion. An internally consistent fit can be biased. Pickup lift is a specified instantaneous reconstruction, not a dynamic observer or proof of impossibility of identification from a time history. Pickup means mechanical displacement/velocity, not magnetic voltage. No source-bank fit, waveform asset, physical-parameter change or realtime integration."}),
    )
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err(HELP.into());
    }
    let output = Path::new(&args[2]);
    if output.extension().is_none_or(|p| p != "json") || output.exists() {
        return Err("output must be a new .json file".into());
    }
    let report = study()?;
    crate::analysis::write_report(output, &report)?;
    if report["controls_passed"] != true {
        return Err("reduced mechanical loss retained failed controls".into());
    }
    println!("Reduced mechanical loss: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spectrum() -> ModalSpectrum {
        ModalSpectrum::prepare(TineGeometry::default(), ModalAssemblyProfile::default()).unwrap()
    }
    #[test]
    fn modal_projection_is_mass_orthogonal_idempotent_and_sign_invariant() {
        let s = spectrum();
        let operators = projections(&s);
        let q = [0.1, 0.02, -0.03, 0.04, 0.01, -0.02, 0.03, -0.01, 0.07];
        let mut flipped = s.clone();
        for mode in &mut flipped.modes {
            mode.shape = mode.shape.map(|x| -x);
            mode.pickup_weight *= -1.0;
        }
        let reversed = projections(&flipped);
        for (index, p) in operators.iter().enumerate().take(5).skip(1) {
            let projected = apply(&p.matrix, q);
            let twice = apply(&p.matrix, projected);
            let error = core::array::from_fn(|i| twice[i] - projected[i]);
            assert!(quadratic(error, &s.mass_matrix) / quadratic(q, &s.mass_matrix) < 1e-20);
            let omitted = core::array::from_fn(|i| q[i] - projected[i]);
            assert!(
                dot(projected, apply(&s.mass_matrix, omitted)).abs() / quadratic(q, &s.mass_matrix)
                    < 1e-10
            );
            assert_eq!(p.matrix, reversed[index].matrix);
        }
        let reconstructed = apply(&operators[4].matrix, q);
        let error = core::array::from_fn(|i| q[i] - reconstructed[i]);
        assert!(quadratic(error, &s.mass_matrix) / quadratic(q, &s.mass_matrix) < 1e-20);
    }
    #[test]
    fn pickup_lift_preserves_its_port_but_cannot_reconstruct_an_instantaneous_null_motion() {
        let s = spectrum();
        let projections = projections(&s);
        let projection = projections.last().unwrap();
        let lift = projection.pickup_lift.unwrap();
        let port: Vector = core::array::from_fn(|i| {
            s.modes
                .iter()
                .map(|mode| apply(&s.mass_matrix, mode.shape)[i] * mode.pickup_weight)
                .sum()
        });
        assert!((dot(port, lift) - 1.0).abs() < 1e-10);
        let null: Vector = core::array::from_fn(|i| {
            s.modes[0].shape[i] * s.modes[1].pickup_weight
                - s.modes[1].shape[i] * s.modes[0].pickup_weight
        });
        assert!(dot(port, null).abs() < 1e-9);
        assert!(quadratic(null, &s.mass_matrix) > 0.0);
        let voice = ModalAssembly::new(
            48000.0,
            TineGeometry::default(),
            ModalAssemblyProfile::default(),
            ModalIntegration::Refined {
                contact_substeps: 64,
            },
        )
        .unwrap();
        let mut p = voice.probe();
        p.position = null;
        p.velocity = null;
        assert_eq!(projection.state(&p), ([0.0; 9], [0.0; 9]));
        p.pickup_displacement_m = 0.003;
        p.pickup_velocity_m_s = 0.02;
        let (q, v) = projection.state(&p);
        assert!((dot(port, q) - 0.003).abs() < 1e-12);
        assert!((dot(port, v) - 0.02).abs() < 1e-12);
    }
}
