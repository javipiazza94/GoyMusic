//! Brings decoded audio to the sink's format: channel mapping, then resampling with rubato
//! when the file's sample rate differs from the device's.

use rubato::{FftFixedInOut, Resampler};

use crate::error::{AudioError, Result};

const CHUNK_FRAMES: usize = 1024;

pub struct Converter {
    in_rate: u32,
    in_channels: usize,
    out_channels: usize,
    resampler: Option<Resampling>,
}

struct Resampling {
    inner: FftFixedInOut<f32>,
    /// Input not yet resampled, one Vec per output channel.
    pending: Vec<Vec<f32>>,
    /// Leading output frames still to drop: the resampler's own delay.
    delay_left: usize,
    in_rate: u64,
    out_rate: u64,
    frames_in: u64,
    frames_out: u64,
}

impl Converter {
    pub fn new(in_rate: u32, in_channels: usize, out_rate: u32, out_channels: usize) -> Result<Self> {
        let resampler = if in_rate == out_rate {
            None
        } else {
            let inner = FftFixedInOut::new(in_rate as usize, out_rate as usize, CHUNK_FRAMES, out_channels)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            Some(Resampling {
                delay_left: inner.output_delay(),
                inner,
                pending: vec![Vec::new(); out_channels],
                in_rate: u64::from(in_rate),
                out_rate: u64::from(out_rate),
                frames_in: 0,
                frames_out: 0,
            })
        };
        Ok(Self {
            in_rate,
            in_channels,
            out_channels,
            resampler,
        })
    }

    /// Converts a block of interleaved input; may return fewer frames than given (the
    /// resampler holds a partial chunk until more input or [`Converter::finish`]).
    pub fn push(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let mapped = map_channels(input, self.in_channels, self.out_channels);
        match &mut self.resampler {
            None => Ok(mapped),
            Some(r) => r.push(&mapped, self.out_channels),
        }
    }

    /// `(sample rate, channels)` this converter accepts. A gapless successor with the same
    /// input format keeps using it, so the resampler's history carries across the join
    /// instead of being flushed with zeros (which clicks).
    pub fn input_format(&self) -> (u32, usize) {
        (self.in_rate, self.in_channels)
    }

    /// Output frames still owed for the input pushed so far (held inside the resampler).
    pub fn owed_frames(&self) -> u64 {
        self.resampler.as_ref().map_or(0, |r| {
            (r.frames_in * r.out_rate / r.in_rate).saturating_sub(r.frames_out)
        })
    }

    /// End of the track: flushes what the resampler still holds, so consecutive tracks join
    /// without losing or adding frames.
    pub fn finish(&mut self) -> Result<Vec<f32>> {
        match &mut self.resampler {
            None => Ok(Vec::new()),
            Some(r) => r.finish(self.out_channels),
        }
    }
}

/// Same channel count: unchanged. To mono: average. Otherwise output channel `c` takes input
/// channel `c`, or the last input channel when there are fewer (mono → both sides).
fn map_channels(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let mut out = Vec::with_capacity(input.len() / from * to);
    for frame in input.chunks_exact(from) {
        if to == 1 {
            out.push(frame.iter().sum::<f32>() / from as f32);
        } else {
            out.extend((0..to).map(|c| frame[c.min(from - 1)]));
        }
    }
    out
}

impl Resampling {
    fn push(&mut self, interleaved: &[f32], channels: usize) -> Result<Vec<f32>> {
        for frame in interleaved.chunks_exact(channels) {
            for (c, &s) in frame.iter().enumerate() {
                self.pending[c].push(s);
            }
        }
        self.frames_in += (interleaved.len() / channels) as u64;
        let mut out = Vec::new();
        while self.pending[0].len() >= self.inner.input_frames_next() {
            let n = self.inner.input_frames_next();
            let chunk: Vec<Vec<f32>> = self.pending.iter_mut().map(|p| p.drain(..n).collect()).collect();
            let resampled = self
                .inner
                .process(&chunk, None)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            self.emit(&resampled, &mut out, u64::MAX);
        }
        Ok(out)
    }

    fn finish(&mut self, channels: usize) -> Result<Vec<f32>> {
        let expected = self.frames_in * self.out_rate / self.in_rate;
        let mut out = Vec::new();
        let mut tail = Some(std::mem::replace(&mut self.pending, vec![Vec::new(); channels]));
        while self.frames_out < expected {
            let resampled = match tail.take() {
                Some(t) => self.inner.process_partial(Some(&t), None),
                None => self.inner.process_partial::<Vec<f32>>(None, None),
            }
            .map_err(|e| AudioError::Resample(e.to_string()))?;
            self.emit(&resampled, &mut out, expected);
        }
        Ok(out)
    }

    /// Appends resampled frames to `out`, skipping the leading delay and stopping at `limit`.
    fn emit(&mut self, resampled: &[Vec<f32>], out: &mut Vec<f32>, limit: u64) {
        let frames = resampled[0].len();
        let skip = self.delay_left.min(frames);
        self.delay_left -= skip;
        for i in skip..frames {
            if self.frames_out >= limit {
                return;
            }
            out.extend(resampled.iter().map(|ch| ch[i]));
            self.frames_out += 1;
        }
    }
}
