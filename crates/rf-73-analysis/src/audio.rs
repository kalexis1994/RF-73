use serde::Serialize;
use std::{
    fmt,
    io::{Read, Seek},
    path::Path,
};

const MAX_SECONDS: usize = 60;
const MAX_FRAMES: usize = 12_000_000;

#[derive(Debug)]
pub struct AudioError(pub String);
impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AudioError {}
impl From<hound::Error> for AudioError {
    fn from(error: hound::Error) -> Self {
        Self(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioMetadata {
    pub sample_rate: u32,
    pub source_channels: u16,
    pub selected_channel: u16,
    pub source_bits_per_sample: u16,
    pub source_format: &'static str,
    pub frames: usize,
}

/// A single explicitly selected channel. Samples retain original gain and DC.
pub struct AudioClip {
    pub(crate) samples: Vec<f64>,
    pub(crate) metadata: AudioMetadata,
}

impl AudioClip {
    pub fn open(path: impl AsRef<Path>, channel: Option<u16>) -> Result<Self, AudioError> {
        Self::decode(hound::WavReader::open(path)?, channel)
    }

    pub fn read(reader: impl Read + Seek, channel: Option<u16>) -> Result<Self, AudioError> {
        Self::decode(hound::WavReader::new(reader)?, channel)
    }

    fn decode<R: Read>(
        mut reader: hound::WavReader<R>,
        channel: Option<u16>,
    ) -> Result<Self, AudioError> {
        let spec = reader.spec();
        if spec.channels == 0 || spec.channels > 8 {
            return Err(AudioError("WAV must contain 1..8 channels".into()));
        }
        let frames = reader.duration() as usize;
        validate_shape(spec.sample_rate, frames)?;
        let selected = match (channel, spec.channels) {
            (Some(c), n) if c < n => c,
            (None, 1) => 0,
            (None, _) => {
                return Err(AudioError(
                    "multichannel WAV requires an explicit zero-based channel".into(),
                ));
            }
            _ => {
                return Err(AudioError(
                    "selected channel is outside the WAV channel count".into(),
                ));
            }
        };
        if reader.len() as usize != frames * spec.channels as usize {
            return Err(AudioError("WAV ends in a partial frame".into()));
        }
        let mut samples = Vec::with_capacity(frames);
        match spec.sample_format {
            hound::SampleFormat::Float => {
                for (i, value) in reader.samples::<f32>().enumerate() {
                    let value = value? as f64;
                    validate_sample(value)?;
                    if i % spec.channels as usize == selected as usize {
                        samples.push(value);
                    }
                }
            }
            hound::SampleFormat::Int => {
                if ![8, 16, 24, 32].contains(&spec.bits_per_sample) {
                    return Err(AudioError(
                        "supported PCM widths are 8, 16, 24 and 32 bits".into(),
                    ));
                }
                let scale = (1_u64 << (spec.bits_per_sample - 1)) as f64;
                for (i, value) in reader.samples::<i32>().enumerate() {
                    let value = value? as f64 / scale;
                    if i % spec.channels as usize == selected as usize {
                        samples.push(value);
                    }
                }
            }
        }
        if samples.len() != frames {
            return Err(AudioError("truncated WAV sample data".into()));
        }
        Ok(Self {
            samples,
            metadata: AudioMetadata {
                sample_rate: spec.sample_rate,
                source_channels: spec.channels,
                selected_channel: selected,
                source_bits_per_sample: spec.bits_per_sample,
                source_format: match spec.sample_format {
                    hound::SampleFormat::Int => "pcm",
                    _ => "ieee_float",
                },
                frames,
            },
        })
    }

    pub fn from_samples(sample_rate: u32, samples: Vec<f64>) -> Result<Self, AudioError> {
        validate_shape(sample_rate, samples.len())?;
        for &sample in &samples {
            validate_sample(sample)?;
        }
        let frames = samples.len();
        Ok(Self {
            samples,
            metadata: AudioMetadata {
                sample_rate,
                source_channels: 1,
                selected_channel: 0,
                source_bits_per_sample: 64,
                source_format: "memory_f64",
                frames,
            },
        })
    }

    pub fn samples(&self) -> &[f64] {
        &self.samples
    }
    pub fn metadata(&self) -> &AudioMetadata {
        &self.metadata
    }
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / self.metadata.sample_rate as f64
    }
}

fn validate_shape(rate: u32, frames: usize) -> Result<(), AudioError> {
    if !(8_000..=192_000).contains(&rate) {
        return Err(AudioError("sample rate must be 8000..192000 Hz".into()));
    }
    if frames == 0 || frames > MAX_FRAMES || frames > MAX_SECONDS * rate as usize {
        return Err(AudioError(
            "analysis requires nonempty audio of at most 60 seconds / 12 million frames".into(),
        ));
    }
    Ok(())
}

fn validate_sample(value: f64) -> Result<(), AudioError> {
    if !value.is_finite() || value.abs() > 1e6 {
        return Err(AudioError(
            "nonfinite or excessive audio sample (absolute limit 1e6)".into(),
        ));
    }
    Ok(())
}
