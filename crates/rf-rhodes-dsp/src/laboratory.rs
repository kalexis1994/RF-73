use crate::{MagneticPickup, ProductionDecimator};

pub const PICKUP_NAMES: [&str; 3] = ["Current", "Close Original", "Close Point Pole"];
/// One RMS ratio per complete reference performance, never per note or velocity.
/// See references/pickup-listening-summary.json (44.1 kHz). Frozen at every rate.
pub const PICKUP_LEVEL_MATCH: [f64; 3] = [1.0, 0.2967936920096338, 0.1361600848962658];

pub(crate) struct Laboratory {
    pub pickup: MagneticPickup,
    filters: [ProductionDecimator; 2],
    weights: [f64; 3],
    start: [f64; 3],
    selected: usize,
    position: u32,
    duration: u32,
}

impl Laboratory {
    pub fn new(rate: f64) -> Self {
        Self {
            pickup: MagneticPickup::new(0.0005, 0.00025).expect("validated research geometry"),
            filters: core::array::from_fn(|_| ProductionDecimator::new()),
            weights: [1.0, 0.0, 0.0],
            start: [1.0, 0.0, 0.0],
            selected: 0,
            position: (rate * 0.020).ceil() as u32,
            duration: (rate * 0.020).ceil() as u32,
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= 3 {
            return false;
        }
        if index != self.selected {
            self.start = self.weights;
            self.selected = index;
            self.position = 0;
        }
        true
    }

    pub fn push(&mut self, signals: [f64; 2]) {
        for (filter, signal) in self.filters.iter_mut().zip(signals) {
            filter.push(signal);
        }
    }

    pub fn mix(&mut self, current: f64) -> f64 {
        if self.position < self.duration {
            self.position += 1;
            let t = f64::from(self.position) / f64::from(self.duration);
            for (i, weight) in self.weights.iter_mut().enumerate() {
                *weight = self.start[i] * (1.0 - t) + if i == self.selected { t } else { 0.0 };
            }
        }
        [current, self.filters[0].output(), self.filters[1].output()]
            .into_iter()
            .zip(self.weights)
            .zip(PICKUP_LEVEL_MATCH)
            .map(|((signal, weight), gain)| signal * weight * gain)
            .sum()
    }

    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.clear();
        }
        self.weights = core::array::from_fn(|i| if i == self.selected { 1.0 } else { 0.0 });
        self.start = self.weights;
        self.position = self.duration;
    }
}
