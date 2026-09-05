//! Provisional Rhodes research engine. Physical plausibility is not calibration.
//! Construction prepares all memory; rendering uses fixed-size state only.
mod filter;
mod model;
mod pickup;
mod voice;

pub use filter::Decimator as ProductionDecimator;
pub use model::{ModelError, Profile, SAMPLE_RATE_MAX, SAMPLE_RATE_MIN};
pub use pickup::MagneticPickup;
pub use voice::{Probe, Voice};

pub const FIRST_NOTE: u8 = 28;
pub const LAST_NOTE: u8 = 100;
pub const KEY_COUNT: usize = (LAST_NOTE - FIRST_NOTE + 1) as usize;
pub const OVERSAMPLE: usize = 4;

/// One shared mechanical key per pitch, with channel-aware key/pedal ownership.
/// Multiple MIDI channels do not create extra copies of the same physical tine.
pub struct Engine {
    voices: [Voice; KEY_COUNT],
    held: [u16; KEY_COUNT],
    sustained: [u16; KEY_COUNT],
    pedals: u16,
    decimator: filter::Decimator,
    gain: f64,
    target_gain: f64,
    gain_step: f64,
    faults: u64,
}

impl Engine {
    pub fn new(sample_rate: f64, profile: Profile) -> Result<Self, ModelError> {
        profile.validate(sample_rate)?;
        let voices = core::array::from_fn(|i| {
            Voice::new_validated(sample_rate, FIRST_NOTE + i as u8, profile)
        });
        Ok(Self {
            voices,
            held: [0; KEY_COUNT],
            sustained: [0; KEY_COUNT],
            pedals: 0,
            decimator: filter::Decimator::new(),
            gain: 0.7,
            target_gain: 0.7,
            gain_step: 1.0 - (-1.0 / (0.005 * sample_rate)).exp(),
            faults: 0,
        })
    }

    pub fn set_gain(&mut self, gain: f64) -> bool {
        if !gain.is_finite() || !(0.0..=2.0).contains(&gain) {
            return false;
        }
        self.target_gain = gain;
        true
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: f64) -> bool {
        let Some(i) = key_index(channel, note) else {
            return false;
        };
        if !velocity.is_finite() || !(0.0..=1.0).contains(&velocity) {
            return false;
        }
        if velocity == 0.0 {
            return self.note_off(channel, note);
        }
        let bit = 1 << channel;
        self.held[i] |= bit;
        self.sustained[i] &= !bit;
        self.voices[i].last_channel = channel;
        self.voices[i].strike(velocity);
        true
    }

    pub fn note_off(&mut self, channel: u8, note: u8) -> bool {
        let Some(i) = key_index(channel, note) else {
            return false;
        };
        let bit = 1 << channel;
        if self.held[i] & bit != 0 && self.pedals & bit != 0 {
            self.sustained[i] |= bit;
        }
        self.held[i] &= !bit;
        self.update_damper(i);
        true
    }

    pub fn control_change(&mut self, channel: u8, controller: u8, value: f64) -> bool {
        if channel >= 16 || !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return false;
        }
        let bit = 1 << channel;
        match controller {
            64 => {
                if value >= 0.5 {
                    self.pedals |= bit;
                    // A late pedal catches a still-vibrating released tine.
                    for i in 0..KEY_COUNT {
                        if self.voices[i].is_active() && self.voices[i].last_channel == channel {
                            self.sustained[i] |= bit;
                            self.update_damper(i);
                        }
                    }
                } else {
                    self.pedals &= !bit;
                    for i in 0..KEY_COUNT {
                        self.sustained[i] &= !bit;
                        self.update_damper(i);
                    }
                }
            }
            123 => {
                for note in FIRST_NOTE..=LAST_NOTE {
                    self.note_off(channel, note);
                }
            }
            120 => {
                for i in 0..KEY_COUNT {
                    let owned = (self.held[i] | self.sustained[i]) & bit != 0;
                    self.held[i] &= !bit;
                    self.sustained[i] &= !bit;
                    // Released tails have no ownership, but still belong to this
                    // channel's last strike unless another channel holds the key.
                    if (owned || self.voices[i].last_channel == channel)
                        && (self.held[i] | self.sustained[i]) == 0
                    {
                        self.voices[i].reset();
                    }
                    self.update_damper(i);
                }
            }
            121 => {
                self.pedals &= !bit;
                for i in 0..KEY_COUNT {
                    self.sustained[i] &= !bit;
                    self.update_damper(i);
                }
            }
            _ => return false,
        }
        true
    }

    fn update_damper(&mut self, i: usize) {
        self.voices[i].set_damped((self.held[i] | self.sustained[i]) == 0);
    }

    /// A mono direct signal, after a 127-tap antialias filter. No limiter,
    /// normalization, reverb or amplifier coloration is hidden here.
    pub fn next_sample(&mut self) -> f32 {
        for _ in 0..OVERSAMPLE {
            let mut sum = 0.0;
            for voice in &mut self.voices {
                sum += voice.tick();
            }
            if !sum.is_finite() {
                self.reset();
                self.faults = self.faults.saturating_add(1);
                return 0.0;
            }
            self.decimator.push(sum);
        }
        self.gain += self.gain_step * (self.target_gain - self.gain);
        (self.decimator.output() * self.gain * 0.12) as f32
    }

    pub fn probe(&self, note: u8) -> Option<Probe> {
        let i = key_index(0, note)?;
        Some(self.voices[i].probe())
    }

    pub fn faults(&self) -> u64 {
        self.faults
    }

    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
        self.held.fill(0);
        self.sustained.fill(0);
        self.pedals = 0;
        self.decimator.clear();
        self.gain = self.target_gain;
    }
}

fn key_index(channel: u8, note: u8) -> Option<usize> {
    (channel < 16 && (FIRST_NOTE..=LAST_NOTE).contains(&note)).then(|| (note - FIRST_NOTE) as usize)
}
