use std::io::{self, Read, Write};

/// Mono IEEE-float RIFF/WAVE, including WAVEFORMATEX and fact chunks.
pub struct FloatWav<W: Write> {
    writer: W,
    remaining: u32,
}

impl<W: Write> FloatWav<W> {
    pub fn new(mut writer: W, rate: u32, frames: u32) -> io::Result<Self> {
        let data = frames
            .checked_mul(4)
            .ok_or_else(|| invalid("WAV data too large"))?;
        let riff = data
            .checked_add(50)
            .ok_or_else(|| invalid("RIFF size overflow"))?;
        let byte_rate = rate
            .checked_mul(4)
            .ok_or_else(|| invalid("sample rate overflow"))?;
        if rate == 0 {
            return Err(invalid("sample rate must be positive"));
        }
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff.to_le_bytes())?;
        writer.write_all(b"WAVEfmt ")?;
        writer.write_all(&18_u32.to_le_bytes())?;
        for value in [3_u16, 1] {
            writer.write_all(&value.to_le_bytes())?;
        }
        writer.write_all(&rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        for value in [4_u16, 32, 0] {
            writer.write_all(&value.to_le_bytes())?;
        }
        writer.write_all(b"fact")?;
        writer.write_all(&4_u32.to_le_bytes())?;
        writer.write_all(&frames.to_le_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&data.to_le_bytes())?;
        Ok(Self {
            writer,
            remaining: frames,
        })
    }
    pub fn sample(&mut self, value: f32) -> io::Result<()> {
        if self.remaining == 0 || !value.is_finite() {
            return Err(invalid("extra or non-finite WAV sample"));
        }
        self.writer.write_all(&value.to_le_bytes())?;
        self.remaining -= 1;
        Ok(())
    }
    pub fn finish(mut self) -> io::Result<()> {
        if self.remaining != 0 {
            return Err(invalid("WAV has fewer frames than declared"));
        }
        self.writer.flush()
    }
}

/// Strict verifier for this lab's layout; deliberately not a general WAV parser.
pub fn inspect(mut reader: impl Read) -> io::Result<String> {
    let mut h = [0; 58];
    reader.read_exact(&mut h)?;
    let u32_at =
        |offset| u32::from_le_bytes(h[offset..offset + 4].try_into().expect("header field"));
    if &h[..4] != b"RIFF"
        || &h[8..16] != b"WAVEfmt "
        || u32_at(16) != 18
        || h[20..24] != [3, 0, 1, 0]
        || h[32..38] != [4, 0, 32, 0, 0, 0]
        || &h[38..42] != b"fact"
        || u32_at(42) != 4
        || &h[50..54] != b"data"
    {
        return Err(invalid("not an RF-Rhodes mono float WAV"));
    }
    let rate = u32_at(24);
    let frames = u32_at(46);
    let data = frames
        .checked_mul(4)
        .ok_or_else(|| invalid("frame count overflow"))?;
    if rate == 0
        || rate.checked_mul(4) != Some(u32_at(28))
        || u32_at(54) != data
        || data.checked_add(50) != Some(u32_at(4))
    {
        return Err(invalid("inconsistent WAV header"));
    }
    let mut peak = 0.0_f64;
    let mut squares = 0.0;
    for _ in 0..frames {
        let mut bytes = [0; 4];
        reader.read_exact(&mut bytes)?;
        let value = f32::from_le_bytes(bytes) as f64;
        if !value.is_finite() {
            return Err(invalid("non-finite sample"));
        }
        peak = peak.max(value.abs());
        squares += value * value;
    }
    if reader.read(&mut [0])? != 0 {
        return Err(invalid("unexpected bytes after WAV data"));
    }
    let rms = if frames == 0 {
        0.0
    } else {
        (squares / frames as f64).sqrt()
    };
    Ok(format!(
        "{{\"sample_rate\":{rate},\"frames\":{frames},\"peak\":{peak:.9},\"rms\":{rms:.9},\"finite\":true}}"
    ))
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_corruption_detection() {
        let mut bytes = Vec::new();
        let mut writer = FloatWav::new(&mut bytes, 48_000, 3).unwrap();
        for value in [0.0, 0.5, -1.25] {
            writer.sample(value).unwrap();
        }
        writer.finish().unwrap();
        assert_eq!(bytes.len(), 70);
        assert!(inspect(bytes.as_slice()).unwrap().contains("1.250000000"));
        assert!(inspect(&bytes[..69]).is_err());
        bytes.extend([0]);
        assert!(inspect(bytes.as_slice()).is_err());
    }
    #[test]
    fn rejects_incomplete_nonfinite_and_extra_samples() {
        let mut writer = FloatWav::new(Vec::new(), 48_000, 1).unwrap();
        assert!(writer.sample(f32::NAN).is_err());
        writer.sample(0.0).unwrap();
        assert!(writer.sample(0.0).is_err());
        writer.finish().unwrap();
        assert!(
            FloatWav::new(Vec::new(), 48_000, 1)
                .unwrap()
                .finish()
                .is_err()
        );
    }
}
