//! Finite-inertia key/pedestal under a prescribed finger force, coupled to the
//! loaded action through the pedestal contact force.
use super::{bridle, launch, repetition, tuning};
use rf_73_dsp::{ElectromechanicalProbe, ElectromechanicalProfile};
use serde_json::{Value, json};
use std::{cell::RefCell, error::Error, path::Path};

pub(super) const REPEAT: usize = 10080;
pub(super) const WINDOW: usize = 1440; // 30 ms launch observation after each key-down.
pub(super) const SPEED_BOUND: f64 = 2.0;
pub(super) const RETURN_FORCE: f64 = 1.0;
const NAMES: [&str; 4] = [
    "constant_slew",
    "key_hard_stop",
    "key_felt_bed",
    "key_heavy_hard_stop",
];
const TERMS: [&str; 8] = [
    "finger_work_j",
    "negative_return_work_j",
    "negative_pedestal_work_j",
    "negative_bed_stored_change_j",
    "negative_bed_heat_j",
    "negative_end_stop_loss_j",
    "negative_rest_stop_loss_j",
    "kinetic_change_j",
];

#[derive(Clone, Copy)]
pub(super) struct Key {
    mass: f64,
    finger_force: f64,
    bed_depth: f64,
    bed_stiffness: f64,
    bed_damping: f64,
}
// The finger force is chosen so that a free key without pedestal reaction
// would reach the nominal speed exactly at the end of travel.
pub(super) fn key(index: usize, speed: f64, length: f64) -> Option<Key> {
    if index == 0 {
        return None;
    }
    let mass = if index == 3 { 0.1 } else { 0.05 };
    Some(Key {
        mass,
        finger_force: RETURN_FORCE + mass * speed * speed / (2.0 * length),
        bed_depth: if index == 2 { 0.00025 } else { 0.0 },
        bed_stiffness: if index == 2 { 3e10 } else { 0.0 },
        bed_damping: if index == 2 { 300.0 } else { 0.0 },
    })
}
fn potential(k: f64, p: f64) -> f64 {
    k * p.max(0.0).powi(3) / 3.0
}
// Discrete-gradient bed force: its product with the penetration change equals
// the potential change exactly, including partial entry and exit ticks.
fn gradient(k: f64, p0: f64, p1: f64) -> f64 {
    if p0 <= 0.0 && p1 <= 0.0 {
        0.0
    } else if p0 > 0.0 && p1 > 0.0 {
        k * (p0 * p0 + p0 * p1 + p1 * p1) / 3.0
    } else if (p1 - p0).abs() > 0.0 {
        (potential(k, p1) - potential(k, p0)) / (p1 - p0)
    } else {
        k * p0.max(0.0).powi(2)
    }
}
// Effective pedestal height as a function of key position. A sharp let-off
// clamps the pedestal at its top; a rolled let-off decelerates it smoothly over
// a band of key travel with unit slope at entry and zero slope at the top.
#[derive(Clone, Copy)]
pub(super) struct LetOff {
    pub(super) top: f64,
    pub(super) band: f64,
}
impl LetOff {
    pub(super) fn start(&self) -> f64 {
        if self.band > 0.0 {
            self.top - 2.0 * self.band / 3.0
        } else {
            self.top
        }
    }
    pub(super) fn complete(&self) -> f64 {
        self.start() + self.band
    }
    pub(super) fn map(&self, x: f64) -> f64 {
        if self.band <= 0.0 {
            return x.min(self.top);
        }
        let start = self.start();
        if x <= start {
            x
        } else if x >= start + self.band {
            self.top
        } else {
            let u = (x - start) / self.band;
            start + self.band * (u - u * u * u / 3.0)
        }
    }
    pub(super) fn slope(&self, x: f64) -> f64 {
        if x < self.start() {
            1.0
        } else if x >= self.complete() {
            0.0
        } else {
            let u = (x - self.start()) / self.band;
            1.0 - u * u
        }
    }
    // Average slope over a key step: the reaction on the key times the key
    // displacement then equals the pedestal force times the pedestal displacement.
    pub(super) fn ratio(&self, x0: f64, x1: f64) -> f64 {
        if x1 == x0 {
            self.slope(x0)
        } else {
            (self.map(x1) - self.map(x0)) / (x1 - x0)
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Sums {
    seconds: f64,
    x: f64,
    v: f64,
    finger: f64,
    return_work: f64,
    pedestal: f64,
    bed_stored: f64,
    bed_heat: f64,
    end_loss: f64,
    rest_loss: f64,
    assembly_pedestal: f64,
}
impl Sums {
    fn kinetic(&self, mass: f64) -> f64 {
        0.5 * mass * self.v * self.v
    }
    fn terms(&self, from: &Sums, mass: f64) -> [f64; 8] {
        [
            self.finger - from.finger,
            -(self.return_work - from.return_work),
            -(self.pedestal - from.pedestal),
            -(self.bed_stored - from.bed_stored),
            -(self.bed_heat - from.bed_heat),
            -(self.end_loss - from.end_loss),
            -(self.rest_loss - from.rest_loss),
            self.kinetic(mass) - from.kinetic(mass),
        ]
    }
}
struct Gesture {
    start: f64,
    arrival: Option<(f64, f64)>,
    peak_speed: f64,
    end_departures: u32,
    letoff_begin: Option<(f64, f64)>,
    letoff_complete: Option<(f64, f64)>,
    finger_at_letoff: Option<f64>,
    hammer_at_letoff: Value,
}
pub(super) struct Sim {
    travel_end: f64,
    letoff: Option<LetOff>,
    pending_letoff: bool,
    p: ElectromechanicalProfile,
    key: Key,
    sums: Sums,
    pub(super) pedestal_force: f64,
    commanded: bool,
    snapshots: Vec<Sums>,
    gestures: Vec<Gesture>,
    landings: Vec<Option<(f64, f64)>>,
    max_speed: f64,
    max_penetration: f64,
    hard_limit_engaged: bool,
    bed_heat_monotone: bool,
    max_defect: f64,
    tracking: f64,
}
impl Sim {
    pub(super) fn new(p: ElectromechanicalProfile, key: Key) -> Self {
        Self::with_letoff(p, key, -p.action.escapement_m, None)
    }
    pub(super) fn with_letoff(
        p: ElectromechanicalProfile,
        key: Key,
        travel_end: f64,
        letoff: Option<LetOff>,
    ) -> Self {
        Self {
            travel_end,
            letoff,
            pending_letoff: false,
            p,
            key,
            sums: Sums {
                x: p.action.hammer_rest_m,
                ..Sums::default()
            },
            pedestal_force: 0.0,
            commanded: false,
            snapshots: Vec::new(),
            gestures: Vec::new(),
            landings: Vec::new(),
            max_speed: 0.0,
            max_penetration: 0.0,
            hard_limit_engaged: false,
            bed_heat_monotone: true,
            max_defect: 0.0,
            tracking: 0.0,
        }
    }
    fn bed_origin(&self) -> f64 {
        -self.p.action.escapement_m - self.key.bed_depth
    }
    fn bed_force(&self, x: f64, v: f64, h: f64, a: f64) -> (f64, f64, f64) {
        let p0 = x - self.bed_origin();
        let p1 = p0 + h * (v + 0.5 * h * a);
        let g = gradient(self.key.bed_stiffness, p0, p1);
        let vm = v + 0.5 * h * a;
        // Nonadhesive: a dashpot pull larger than the spring push releases the bed.
        ((g + self.key.bed_damping * vm).max(0.0), g, vm)
    }
    // Returns the pedestal position after this tick.
    pub(super) fn step(&mut self, t: f64, h: f64) -> f64 {
        let rest = self.p.action.hammer_rest_m;
        let end = self.travel_end;
        let m = self.key.mass;
        let commanded = repetition::target(self.p, t, REPEAT) != rest;
        if commanded != self.commanded {
            let mut s = self.sums;
            s.seconds = t;
            self.snapshots.push(s);
            if commanded {
                self.gestures.push(Gesture {
                    start: t,
                    arrival: None,
                    peak_speed: 0.0,
                    end_departures: 0,
                    letoff_begin: None,
                    letoff_complete: None,
                    finger_at_letoff: None,
                    hammer_at_letoff: Value::Null,
                });
            } else {
                self.landings.push(None);
            }
            self.commanded = commanded;
        }
        let finger = if commanded {
            self.key.finger_force
        } else {
            0.0
        };
        let f_ped = self.pedestal_force;
        let mut other = finger - RETURN_FORCE - f_ped;
        let (x, v) = (self.sums.x, self.sums.v);
        let mut a = other / m;
        let mut bed = (0.0, 0.0, 0.0);
        let mut ratio = 1.0;
        if let Some(l) = self.letoff {
            // The cam reaction ratio and any bed force both depend on the step, so
            // the acceleration is bracketed and bisected; g is nondecreasing in a.
            let eval = |a: f64| {
                let bed = if self.key.bed_depth > 0.0 {
                    self.bed_force(x, v, h, a)
                } else {
                    (0.0, 0.0, 0.0)
                };
                let r = l.ratio(x, x + h * (v + 0.5 * h * a));
                (m * a - finger + RETURN_FORCE + f_ped * r + bed.0, bed, r)
            };
            let mut hi = (finger - RETURN_FORCE) / m;
            let mut lo = (finger - RETURN_FORCE - f_ped.max(0.0) - eval(hi).1.0) / m;
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if eval(mid).0 > 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
                if hi - lo <= 1e-13 * hi.abs().max(lo.abs()).max(1.0) {
                    break;
                }
            }
            a = 0.5 * (lo + hi);
            let e = eval(a);
            bed = e.1;
            ratio = e.2;
            other = finger - RETURN_FORCE - f_ped * ratio;
        } else if self.key.bed_depth > 0.0 {
            let p0 = x - self.bed_origin();
            let p_free = p0 + h * (v + 0.5 * h * a);
            if p0 > 0.0 || p_free > 0.0 {
                // The bed force increases with acceleration, so g is monotone.
                let g = |a: f64| m * a - other + self.bed_force(x, v, h, a).0;
                let mut hi = other / m;
                let mut lo = (other - self.bed_force(x, v, h, hi).0) / m;
                for _ in 0..200 {
                    let mid = 0.5 * (lo + hi);
                    if g(mid) > 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                    if hi - lo <= 1e-13 * hi.abs().max(lo.abs()).max(1.0) {
                        break;
                    }
                }
                a = 0.5 * (lo + hi);
                bed = self.bed_force(x, v, h, a);
            }
        }
        let held_at_rest = x <= rest && v == 0.0 && other - bed.0 <= 0.0;
        let held_at_end = self.key.bed_depth == 0.0 && x >= end && v == 0.0 && other >= 0.0;
        if held_at_rest || held_at_end {
            a = 0.0;
            bed = (0.0, 0.0, 0.0);
        }
        // Displacement is formed from the same midpoint velocity as the kinetic
        // change, so the identity does not inherit position rounding.
        let mut new_v = v + h * a;
        let mut dx = h * (v + 0.5 * h * a);
        let mut new_x = x + dx;
        let mut end_loss = 0.0;
        let mut rest_loss = 0.0;
        let mut landing = None;
        if new_x > end {
            if self.key.bed_depth > 0.0 {
                self.hard_limit_engaged = true;
            }
            dx = end - x;
            let contact = (v * v + 2.0 * a * dx).max(0.0);
            end_loss = 0.5 * m * contact;
            if let Some(g) = self.gestures.last_mut()
                && g.arrival.is_none()
            {
                g.arrival = Some((t + h - g.start, contact.sqrt()));
            }
            new_x = end;
            new_v = 0.0;
        } else if new_x < rest {
            dx = rest - x;
            let contact = (v * v + 2.0 * a * dx).max(0.0);
            rest_loss = 0.5 * m * contact;
            landing = Some((t + h, contact.sqrt()));
            new_x = rest;
            new_v = 0.0;
        }
        let origin = self.bed_origin();
        let bed_depth = self.key.bed_depth;
        if let Some(g) = self.gestures.last_mut()
            && commanded
        {
            g.peak_speed = g.peak_speed.max(new_v);
            if bed_depth > 0.0 && g.arrival.is_none() && new_x > origin {
                g.arrival = Some((t + h - g.start, new_v));
            }
            if bed_depth == 0.0 && x >= end && new_x < end {
                g.end_departures += 1;
            }
        }
        if let (Some(l), Some(slot)) = (landing, self.landings.last_mut())
            && slot.is_none()
        {
            *slot = Some(l);
        }
        // The discrete-gradient product is the exact potential change; bed heat is
        // c*v_mid^2*h while engaged and the unrecovered potential when released.
        let stored = bed.1 * dx;
        let bed_heat = if bed.0 > 0.0 {
            self.key.bed_damping * bed.2 * dx
        } else {
            -stored
        };
        if bed_heat < 0.0 {
            self.bed_heat_monotone = false;
        }
        let s = &mut self.sums;
        s.finger += finger * dx;
        s.return_work += RETURN_FORCE * dx;
        s.pedestal += f_ped * ratio * dx;
        s.bed_stored += stored;
        s.bed_heat += bed_heat;
        s.end_loss += end_loss;
        s.rest_loss += rest_loss;
        s.x = new_x;
        s.v = new_v;
        self.max_speed = self.max_speed.max(new_v.abs());
        self.max_penetration = self.max_penetration.max(new_x - self.bed_origin());
        let terms = self.sums.terms(&Sums::default(), m);
        let residual = terms[7] - terms[..7].iter().sum::<f64>();
        let scale = terms.iter().map(|x| x.abs()).sum::<f64>().max(1e-20);
        self.max_defect = self.max_defect.max(residual.abs() / scale);
        let Some(l) = self.letoff else {
            return new_x;
        };
        if let Some(g) = self.gestures.last_mut()
            && commanded
        {
            if g.letoff_begin.is_none() && new_x >= l.start() {
                g.letoff_begin = Some((t + h - g.start, new_v));
                g.finger_at_letoff = Some(self.sums.finger);
                self.pending_letoff = true;
            }
            if g.letoff_complete.is_none() && new_x >= l.complete() {
                g.letoff_complete = Some((t + h - g.start, new_v));
            }
        }
        l.map(new_x)
    }
    pub(super) fn observe(&mut self, b: ElectromechanicalProbe) {
        let expected = self.letoff.map_or(self.sums.x, |l| l.map(self.sums.x));
        self.tracking = self
            .tracking
            .max((b.mechanical.pedestal_position_m - expected).abs());
        self.pedestal_force = b.mechanical.contact_force_n[2];
        self.sums.assembly_pedestal = b.mechanical.pedestal_work_j;
        if self.pending_letoff
            && let Some(g) = self.gestures.last_mut()
        {
            g.hammer_at_letoff = json!({"hammer_position_m":b.mechanical.position[18],
                "hammer_velocity_m_s":b.mechanical.velocity[18],"pedestal_force_n":b.mechanical.contact_force_n[2],
                "pedestal_compression_m":b.mechanical.compression_m[2],"arm_velocity_m_s":b.mechanical.velocity[19]});
            self.pending_letoff = false;
        }
    }
    pub(super) fn report(&self, shape: &str, speed: f64, final_seconds: f64) -> Value {
        let m = self.key.mass;
        let mut snapshots = self.snapshots.clone();
        let mut last = self.sums;
        last.seconds = final_seconds;
        snapshots.push(last);
        let window = |from: &Sums, to: &Sums| -> Value {
            let terms = to.terms(from, m);
            let residual = terms[7] - terms[..7].iter().sum::<f64>();
            let scale = terms.iter().map(|x| x.abs()).sum::<f64>().max(1e-20);
            let assembly = to.assembly_pedestal - from.assembly_pedestal;
            json!({"start_seconds":from.seconds,"end_seconds":to.seconds,"terms_j":terms,
                "assembly_pedestal_work_j":assembly,
                "pedestal_work_lag_j":(to.pedestal-from.pedestal)-assembly,
                "relative_defect":residual.abs()/scale,"end_position_m":to.x,"end_velocity_m_s":to.v})
        };
        let mut gestures = Vec::new();
        let mut recoveries = Vec::new();
        let mut lag = 0.0_f64;
        for (i, pair) in snapshots.windows(2).enumerate() {
            let w = window(&pair[0], &pair[1]);
            let scale = w["assembly_pedestal_work_j"].as_f64().unwrap().abs();
            if scale > 0.0 {
                lag = lag.max(w["pedestal_work_lag_j"].as_f64().unwrap().abs() / scale);
            }
            if i % 2 == 0 {
                let g = &self.gestures[i / 2];
                let mut gesture = json!({"start_seconds":g.start,
                    "arrival_seconds_after_key_down":g.arrival.map(|a|a.0),"arrival_speed_m_s":g.arrival.map(|a|a.1),
                    "peak_key_speed_m_s":g.peak_speed,"end_stop_departures":g.end_departures,"window":w});
                if self.letoff.is_some() {
                    gesture["letoff"] = json!({"begin_seconds_after_key_down":g.letoff_begin.map(|b|b.0),
                        "key_speed_at_begin_m_s":g.letoff_begin.map(|b|b.1),
                        "complete_seconds_after_key_down":g.letoff_complete.map(|c|c.0),
                        "key_speed_at_complete_m_s":g.letoff_complete.map(|c|c.1),
                        "hammer_at_begin":g.hammer_at_letoff,
                        "finger_work_after_begin_j":g.finger_at_letoff.map(|f|pair[1].finger-f)});
                }
                gestures.push(gesture);
            } else {
                let l = self.landings[i / 2];
                recoveries.push(
                    json!({"landing_seconds_after_key_up":l.map(|l|l.0-pair[0].seconds),
                    "landing_speed_m_s":l.map(|l|l.1),"window":w}),
                );
            }
        }
        let arrived = self.gestures.len() == 2
            && self.gestures.iter().all(|g| {
                g.arrival.is_some()
                    && (self.letoff.is_none()
                        || (g.letoff_complete.is_some() && !g.hammer_at_letoff.is_null()))
            });
        let landed = self.landings.len() == 2 && self.landings[0].is_some();
        let passed = self.tracking < 1e-9
            && self.max_defect < 1e-8
            && self.max_speed < SPEED_BOUND
            && !self.hard_limit_engaged
            && self.bed_heat_monotone
            && lag < 1e-3
            && arrived
            && landed;
        let mut report = json!({"passed":passed,"shape":shape,"mass_kg":m,"return_force_n":RETURN_FORCE,
            "finger_force_n":self.key.finger_force,"nominal_free_arrival_speed_m_s":speed,
            "bed":(self.key.bed_depth>0.0).then(||json!({"depth_m":self.key.bed_depth,"stiffness_n_m2":self.key.bed_stiffness,
                "damping_n_s_m":self.key.bed_damping,"max_penetration_m":self.max_penetration,"heat_monotone":self.bed_heat_monotone})),
            "speed_bound_m_s":SPEED_BOUND,"max_key_speed_m_s":self.max_speed,"max_tracking_error_m":self.tracking,
            "hard_limit_engaged":self.hard_limit_engaged,"max_relative_energy_defect":self.max_defect,
            "max_relative_pedestal_work_lag":lag,"term_order":TERMS,"gestures":gestures,"recoveries":recoveries});
        if let Some(l) = self.letoff {
            report["travel_end_m"] = json!(self.travel_end);
            report["letoff"] = json!({"top_m":l.top,"band_m":l.band,"start_m":l.start(),"complete_m":l.complete()});
        }
        report
    }
}
fn take(
    p: ElectromechanicalProfile,
    steps: usize,
    speed: f64,
    index: usize,
) -> Result<bridle::Take, Box<dyn Error>> {
    let length = -p.action.escapement_m - p.action.hammer_rest_m;
    let h = 1.0 / (48000.0 * steps as f64);
    let Some(key) = key(index, speed, length) else {
        let mut result = launch::take_driven(
            p,
            steps,
            speed,
            REPEAT,
            WINDOW,
            |t| repetition::target(p, t, REPEAT),
            |_, _, _, _, _| {},
        )?;
        result.report["driver"] = json!({"shape":NAMES[0],"nominal_speed_m_s":speed});
        result.report["key"] = Value::Null;
        return Ok(result);
    };
    let sim = RefCell::new(Sim::new(p, key));
    let mut result = launch::take_driven(
        p,
        steps,
        SPEED_BOUND,
        REPEAT,
        WINDOW,
        |t| sim.borrow_mut().step(t, h),
        |_, _, b, _, _| sim.borrow_mut().observe(b),
    )?;
    let frames = REPEAT + 9120;
    let key_report = sim
        .borrow()
        .report(NAMES[index], speed, frames as f64 / 48000.0);
    result.report["driver"] = json!({"shape":NAMES[index],"nominal_speed_m_s":speed});
    result.report["passed"] =
        json!(result.report["passed"] == true && key_report["passed"] == true);
    result.report["key"] = key_report;
    Ok(result)
}
fn scalar_error(a: &Value, b: &Value, floor: f64) -> f64 {
    let a = a.as_f64().unwrap();
    let b = b.as_f64().unwrap();
    (a - b).abs() / b.abs().max(floor)
}
fn optional_time_error(a: &Value, b: &Value) -> Option<f64> {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        (None, None) => Some(0.0),
        _ => None,
    }
}
fn optional_scalar_error(a: &Value, b: &Value, floor: f64) -> Option<f64> {
    match (a.as_f64(), b.as_f64()) {
        (Some(_), Some(_)) => Some(scalar_error(a, b, floor)),
        (None, None) => Some(0.0),
        _ => None,
    }
}
// Window terms are normalized by the term magnitude or 0.1% of the window's
// summed absolute magnitude, whichever is larger.
fn window_error(a: &Value, b: &Value) -> f64 {
    let ta = a["terms_j"].as_array().unwrap();
    let tb = b["terms_j"].as_array().unwrap();
    let total: f64 = tb.iter().map(|x| x.as_f64().unwrap().abs()).sum();
    ta.iter()
        .zip(tb)
        .map(|(x, y)| {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() / y.abs().max(1e-3 * total).max(1e-10)
        })
        .fold(0.0_f64, f64::max)
}
pub(super) fn key_convergence(a: &Value, b: &Value) -> Value {
    if a["key"].is_null() || b["key"].is_null() {
        return json!({"passed":a["key"].is_null() && b["key"].is_null(),"prescribed":true});
    }
    let mut rows = Vec::new();
    for (i, (ga, gb)) in a["key"]["gestures"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["key"]["gestures"].as_array().unwrap())
        .enumerate()
    {
        let time = optional_time_error(
            &ga["arrival_seconds_after_key_down"],
            &gb["arrival_seconds_after_key_down"],
        );
        let speed = optional_scalar_error(&ga["arrival_speed_m_s"], &gb["arrival_speed_m_s"], 0.01);
        let peak = scalar_error(&ga["peak_key_speed_m_s"], &gb["peak_key_speed_m_s"], 0.01);
        let terms = window_error(&ga["window"], &gb["window"]);
        let held = (ga["window"]["end_position_m"].as_f64().unwrap()
            - gb["window"]["end_position_m"].as_f64().unwrap())
        .abs();
        rows.push(json!({"gesture":i+1,"passed":time.is_some_and(|t|t<0.0001) && speed.is_some_and(|e|e<0.01) && peak<0.01 && terms<0.01 && held<1e-6,
            "arrival_time_error_seconds":time,"relative_arrival_speed_error":speed,"relative_peak_speed_error":peak,
            "max_relative_term_error":terms,"held_position_error_m":held}));
    }
    let mut recoveries = Vec::new();
    for (i, (ra, rb)) in a["key"]["recoveries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["key"]["recoveries"].as_array().unwrap())
        .enumerate()
    {
        let time = optional_time_error(
            &ra["landing_seconds_after_key_up"],
            &rb["landing_seconds_after_key_up"],
        );
        let speed = optional_scalar_error(&ra["landing_speed_m_s"], &rb["landing_speed_m_s"], 0.01);
        let terms = window_error(&ra["window"], &rb["window"]);
        recoveries.push(json!({"recovery":i+1,"passed":time.is_some_and(|t|t<0.0001) && speed.is_some_and(|e|e<0.01) && terms<0.01,
            "landing_time_error_seconds":time,"relative_landing_speed_error":speed,"max_relative_term_error":terms}));
    }
    json!({"passed":rows.len()==2 && recoveries.len()==2 && rows.iter().chain(recoveries.iter()).all(|r|r["passed"]==true),
        "prescribed":false,"gestures":rows,"recoveries":recoveries})
}
pub(super) fn compare_first(candidate: &Value, reference: &Value) -> Value {
    let mut first = candidate["repetition"]["phases"][0].clone();
    first["start_seconds"] = json!(0.03);
    repetition::repeated_attack(&reference["repetition"]["phases"][0], &first)
}
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 || args[1] != "--output" {
        return Err("loaded-key-inertia --output REPORT.json".into());
    }
    let output = Path::new(&args[2]);
    if output.exists() || output.extension().is_none_or(|x| x != "json") {
        return Err("key inertia study requires a new JSON path".into());
    }
    let (position, fit) = tuning::fitted_position(196.38614697959488)?;
    let mut profiles = Vec::new();
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        for (profile_index, name) in [(0, "baseline"), (2, "pedestal_rate_loss_10")] {
            let p = repetition::profile(position, profile_index);
            let mut drivers: Vec<Value> = Vec::new();
            for (index, driver) in NAMES.iter().enumerate() {
                let mut rows = Vec::new();
                for (row, speed) in [1.125, 1.5].iter().enumerate() {
                    println!("Loaded key inertia: {name}, {driver}, {speed} m/s, 128/256 ticks");
                    let a = take(p, 128, *speed, index)?;
                    let b = take(p, 256, *speed, index)?;
                    let c = repetition::convergence(&a, &b, REPEAT);
                    let l = launch::convergence(&a.report, &b.report);
                    let k = key_convergence(&a.report, &b.report);
                    let qualified = a.report["passed"] == true
                        && b.report["passed"] == true
                        && c["passed"] == true
                        && l["passed"] == true
                        && k["passed"] == true;
                    let first = compare_first(
                        &b.report,
                        if index == 0 {
                            &b.report
                        } else {
                            &drivers[0]["rows"][row]["takes"][1]
                        },
                    );
                    let same_key = (index == 3)
                        .then(|| compare_first(&b.report, &drivers[1]["rows"][row]["takes"][1]));
                    println!(
                        "Loaded key inertia: qualified={qualified}, repeatable={}",
                        b.report["repetition"]["two_clean_repeatable_strikes"]
                    );
                    rows.push(json!({"nominal_speed_m_s":speed,"measurement_qualified":qualified,"first_vs_original":first,
                        "first_vs_light_hard_stop":same_key,"takes":[a.report,b.report],"convergence":c,
                        "launch_convergence":l,"key_convergence":k}));
                }
                drivers.push(json!({"name":driver,"rows":rows}));
            }
            profiles.push(
                json!({"name":name,"settings":{"return_damping_n_s_m":p.action.hammer_return_n_s_m,
                "pedestal_rate_loss_s_m":p.action.pedestal_rate_loss_s_m,"hammer_mass_kg":p.assembly.hammer_mass_kg},"drivers":drivers}),
            );
        }
        Ok(())
    })();
    let failure = outcome.err().map(|e| e.to_string());
    let passed = failure.is_none()
        && profiles.len() == 2
        && profiles.iter().all(|p| {
            p["drivers"].as_array().is_some_and(|d| {
                d.len() == 4
                    && d.iter().all(|d| {
                        d["rows"].as_array().is_some_and(|r| {
                            r.len() == 2 && r.iter().all(|r| r["measurement_qualified"] == true)
                        })
                    })
            })
        });
    let report = json!({"schema_version":1,"experiment":"loaded-key-inertia-v1","measurement_qualified":passed,"failure_reason":failure,
        "structural_fit":fit,"profiles":profiles,"key_term_order":TERMS,"physical_calibration_claimed":false,"reference_match_claimed":false,
        "protocol":"Frozen 32-take matrix: original and pedestal-rate-loss-10 profiles; nominal speeds 1.125/1.5 m/s; repeated key-down 60 ms after first key-up; 128/256 ticks. Drivers: retained constant slew; a 0.05 kg key/pedestal with inelastic hard stops at both travel ends; the same key with a felt bed over the last 0.25 mm of travel (cubic potential 3e10 N/m^2 with a discrete-gradient spring force, a 300 Ns/m dashpot and nonadhesive release; bed heat is the bed force work not stored in the potential and must be nonnegative every tick); and a 0.1 kg key with hard stops. A constant 1 N return force acts toward rest. A step finger force of 1 N plus m*v^2/(2*travel) acts during the commanded key-down windows, so a free key would reach the nominal speed exactly at the end of travel. The key integrates one tick ahead of the assembly with the previous tick's pedestal contact force as reaction, midpoint displacement and exact discrete kinetic identity; inelastic stops record the arriving kinetic energy as stop loss. Key gates: assembly pedestal position matches the key within 1e-9 m; independent key energy identity (finger work minus return, pedestal, bed storage, bed heat and stop losses equals kinetic change) within 1e-8 relative at every tick; key speed below the 2 m/s bound; hard travel limit never engaged with a felt bed; key-side pedestal work differs from the assembly's by less than 0.1% of its magnitude in every window; both arrivals and the first landing exist. Launch observation is 30 ms after each key-down. Refinement between resolutions requires arrival and landing times within 0.1 ms, arrival, peak and landing speeds within 1% (0.01 m/s floor), window terms within 1% normalized by the term or 0.1% of the window's summed magnitude, and held positions within 1 um. All repetition and launch gates are retained. First-strike comparison against the same-profile constant slew and between the two hard-stop key masses uses the prior 5% impact/speed and 1 ms latency limits. No candidate selection.",
        "scope":"Lumped key reduction with a prescribed step finger force, not a measured key, finger or lever geometry. Nominal speed labels are free-key arrival speeds; actual arrival follows from the hammer reaction. The felt bed lies inside the retained travel, so its held pedestal position is up to 0.25 mm below the prescribed drivers. The rest stop is inelastic in every key variant. Explicit one-tick force lag is retained and measured. No new contact law inside the assembly, geometry, gain, source fit, audio render or production default. Only the 60 ms repetition wait is covered."});
    crate::analysis::write_report(output, &report)?;
    if !passed {
        return Err("key inertia study retained failed qualification".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_key_reaches_nominal_speed_at_travel_end_and_closes_its_energy_identity() {
        let p = repetition::profile(0.8, 0);
        let length = -p.action.escapement_m - p.action.hammer_rest_m;
        for index in [1, 3] {
            let key = key(index, 1.125, length).unwrap();
            let mut sim = Sim::new(p, key);
            let h = 1e-6;
            let mut arrival = None;
            for i in 0..200_000 {
                let t = i as f64 * h;
                let x = sim.step(t, h);
                if arrival.is_none() && x >= -p.action.escapement_m {
                    arrival = Some(t + h);
                }
            }
            let (time, speed) = sim.gestures[0].arrival.unwrap();
            assert!((speed - 1.125).abs() < 1e-3);
            assert!((time - 2.0 * length / 1.125).abs() < 1e-4);
            assert!((arrival.unwrap() - 0.03 - time).abs() < 2.0 * h);
            assert!(sim.max_defect < 1e-8);
            assert!(!sim.hard_limit_engaged);
            assert!((sim.sums.end_loss - 0.5 * key.mass * 1.125_f64.powi(2)).abs() < 1e-5);
            assert_eq!(sim.snapshots[1].x, -p.action.escapement_m);
            assert_eq!(sim.sums.x, p.action.hammer_rest_m);
            assert_eq!(sim.sums.v, 0.0);
            let landing = sim.landings[0].unwrap();
            assert!(landing.0 > 0.15 && landing.0 < 0.21 && landing.1 > 0.3);
        }
    }
    #[test]
    fn felt_bed_stores_and_dissipates_without_reaching_the_hard_limit() {
        let p = repetition::profile(0.8, 0);
        let length = -p.action.escapement_m - p.action.hammer_rest_m;
        let key = key(2, 1.5, length).unwrap();
        let mut sim = Sim::new(p, key);
        let h = 5e-7;
        for i in 0..400_000 {
            sim.step(i as f64 * h, h);
        }
        assert!(sim.gestures[0].arrival.is_some());
        assert!(sim.max_penetration > 1e-5 && sim.max_penetration < key.bed_depth);
        assert!(!sim.hard_limit_engaged);
        assert!(sim.sums.bed_heat > 0.0 && sim.sums.end_loss == 0.0);
        assert!(sim.bed_heat_monotone);
        assert!(sim.max_defect < 1e-8);
        // Held position sits inside the bed under the finger force.
        assert!(
            sim.snapshots[1].x > sim.bed_origin() && sim.snapshots[1].x < -p.action.escapement_m,
            "held {} origin {} end {} v {} landing {:?} arrival {:?} departures {}",
            sim.snapshots[1].x,
            sim.bed_origin(),
            -p.action.escapement_m,
            sim.snapshots[1].v,
            sim.landings,
            sim.gestures[0].arrival,
            sim.gestures[0].end_departures
        );
        let g = gradient(3e10, 0.0, 1e-5);
        assert!((g * 1e-5 - potential(3e10, 1e-5)).abs() < 1e-20);
        assert!((gradient(3e10, -1e-5, 1e-5) * 2e-5 - potential(3e10, 1e-5)).abs() < 1e-20);
        assert_eq!(gradient(3e10, -1.0, -0.5), 0.0);
    }
    #[test]
    fn key_reacts_to_pedestal_force_and_snapshots_follow_command_edges() {
        let p = repetition::profile(0.8, 0);
        let length = -p.action.escapement_m - p.action.hammer_rest_m;
        let key = key(1, 1.125, length).unwrap();
        let h = 1e-6;
        let mut free = Sim::new(p, key);
        let mut loaded = Sim::new(p, key);
        for i in 0..40_000 {
            let t = i as f64 * h;
            free.step(t, h);
            loaded.pedestal_force = 1.0;
            loaded.step(t, h);
        }
        assert!(loaded.sums.v < free.sums.v && loaded.sums.x < free.sums.x);
        assert!((loaded.sums.pedestal - (loaded.sums.x - p.action.hammer_rest_m)).abs() < 1e-12);
        assert!(loaded.max_defect < 1e-8);
        assert_eq!(free.snapshots.len(), 1);
        assert_eq!(free.snapshots[0].seconds, 0.03);
        assert_eq!(free.snapshots[0].x, p.action.hammer_rest_m);
    }
}
