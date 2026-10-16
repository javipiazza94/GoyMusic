#![allow(dead_code)] // each test binary uses a different subset

use std::f64::consts::PI;
use std::path::Path;

/// `frames` frames of a sine wave, interleaved over `channels`, starting at frame `start` of
/// the same wave. Two calls with consecutive ranges join seamlessly, which is what the
/// gapless tests rely on. Amplitude 0.25 of 16-bit full scale.
pub fn sine(rate: u32, channels: u16, freq: f64, start: u64, frames: u64) -> Vec<i16> {
    let mut out = Vec::with_capacity((frames * u64::from(channels)) as usize);
    for i in start..start + frames {
        let v = (i as f64 * freq * 2.0 * PI / f64::from(rate)).sin() * 8192.0;
        for _ in 0..channels {
            out.push(v.round() as i16);
        }
    }
    out
}

/// Frame `i` holds the value `i` (mono), so a decoded sample says which frame it came from.
pub fn ramp(frames: u64) -> Vec<i16> {
    (0..frames).map(|i| i as i16).collect()
}

pub fn write_wav(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

pub fn write_flac(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;
    let samples: Vec<i32> = samples.iter().map(|&s| i32::from(s)).collect();
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(&samples, channels as usize, 16, rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size).unwrap();
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).unwrap();
    let mut bytes = sink.as_slice().to_vec();
    // flacenc 0.5 counts the short last block in STREAMINFO's minimum block size; the FLAC
    // spec excludes it. With min != max, symphonia expects variable-size blocks and rejects
    // every frame. Bytes 8..10 are the minimum, 10..12 the maximum: copy max over min.
    let (min, max) = (8..10, 10..12);
    let max_block = bytes[max].to_vec();
    bytes[min].copy_from_slice(&max_block);
    std::fs::write(path, bytes).unwrap();
}

/// Decoded `f32` back to the 16-bit integer it was written as.
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|&s| (s * 32768.0).round() as i16).collect()
}
