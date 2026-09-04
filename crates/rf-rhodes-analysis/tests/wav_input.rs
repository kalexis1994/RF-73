use rf_rhodes_analysis::AudioClip;
use std::io::Cursor;

fn pcm(bits: u16, channels: u16, values: &[i32]) -> Vec<u8> {
    let mut data = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut data, spec).unwrap();
    for &value in values {
        writer.write_sample(value).unwrap();
    }
    writer.finalize().unwrap();
    data.into_inner()
}

#[test]
fn accepts_all_supported_integer_widths_without_normalizing() {
    for bits in [8, 16, 24, 32] {
        let half = 1_i32 << (bits - 2);
        let clip = AudioClip::read(Cursor::new(pcm(bits, 1, &[0, half, -half])), None).unwrap();
        assert_eq!(clip.samples(), &[0.0, 0.5, -0.5]);
    }
}

#[test]
fn stereo_requires_explicit_channel_and_never_downmixes() {
    let bytes = pcm(16, 2, &[16_384, -8192, 8192, 16_384]);
    assert!(AudioClip::read(Cursor::new(&bytes), None).is_err());
    assert!(AudioClip::read(Cursor::new(&bytes), Some(2)).is_err());
    let clip = AudioClip::read(Cursor::new(&bytes), Some(1)).unwrap();
    assert_eq!(clip.samples(), &[-0.25, 0.5]);
    assert_eq!(clip.metadata().selected_channel, 1);
}

#[test]
fn reads_metadata_chunks_and_rejects_truncated_data() {
    let mut bytes = pcm(24, 1, &[0, 1000, -1000]);
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let chunk = [b'J', b'U', b'N', b'K', 2, 0, 0, 0, 7, 7];
    bytes.splice(12..12, chunk);
    bytes[4..8].copy_from_slice(&(size + 10).to_le_bytes());
    assert!(AudioClip::read(Cursor::new(&bytes), None).is_ok());
    bytes.pop();
    assert!(AudioClip::read(Cursor::new(&bytes), None).is_err());
}

#[test]
fn float_gain_is_preserved_but_nonfinite_samples_are_rejected() {
    for values in [[0.0, 1.5, -2.0], [0.0, f32::NAN, 0.0]] {
        let mut data = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(
            &mut data,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for value in values {
            writer.write_sample(value).unwrap();
        }
        writer.finalize().unwrap();
        let result = AudioClip::read(Cursor::new(data.into_inner()), None);
        if values[1].is_nan() {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap().samples(), &[0.0, 1.5, -2.0]);
        }
    }
}
