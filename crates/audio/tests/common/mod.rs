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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use goymusic_audio::engine::Engine;
use goymusic_audio::sink::{AudioSink, CaptureHandle, CaptureSink, SinkError, SinkFactory, SinkFormat};
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

/// An engine over capture sinks. Every sink the factory opens is kept in `sinks`.
pub struct Rig {
    pub engine: Engine,
    pub events: Receiver<Event>,
    pub sinks: Arc<Mutex<Vec<CaptureHandle>>>,
    pub factory_fails: Arc<AtomicBool>,
    pub format: SinkFormat,
}

pub fn capture_factory(format: SinkFormat) -> (SinkFactory, Arc<Mutex<Vec<CaptureHandle>>>, Arc<AtomicBool>) {
    let sinks = Arc::new(Mutex::new(Vec::new()));
    let fails = Arc::new(AtomicBool::new(false));
    let (s, f) = (sinks.clone(), fails.clone());
    let factory: SinkFactory = Box::new(move || {
        if f.load(Ordering::SeqCst) {
            return Err(SinkError::NoDevice);
        }
        let (sink, handle) = CaptureSink::new(format);
        s.lock().unwrap().push(handle);
        Ok(Box::new(sink) as Box<dyn AudioSink>)
    });
    (factory, sinks, fails)
}

pub fn resolver(files: &[(i64, PathBuf)]) -> goymusic_audio::engine::Resolver {
    let map: HashMap<TrackId, PathBuf> = files.iter().map(|(id, p)| (TrackId(*id), p.clone())).collect();
    Box::new(move |id| map.get(&id).cloned())
}

impl Rig {
    pub fn new(format: SinkFormat, files: &[(i64, PathBuf)]) -> Self {
        let bus = Arc::new(EventBus::new());
        let events = bus.subscribe();
        let (factory, sinks, factory_fails) = capture_factory(format);
        let engine = Engine::new(factory, resolver(files), bus);
        Self {
            engine,
            events,
            sinks,
            factory_fails,
            format,
        }
    }

    pub fn sink(&self, i: usize) -> CaptureHandle {
        self.sinks.lock().unwrap()[i].clone()
    }

    /// Pumps until playback stops (ended, idle, paused) or `max` pumps.
    pub fn run(&mut self, max: usize) {
        for _ in 0..max {
            self.engine.pump();
            if self.engine.state() != PlaybackState::Playing {
                return;
            }
        }
    }

    /// Lets a held sink play `frames` frames, pumping in between like the real thread does.
    pub fn advance(&mut self, sink: usize, frames: usize) {
        let handle = self.sink(sink);
        let channels = self.format.channels as usize;
        let mut left = frames;
        while left > 0 {
            self.engine.pump();
            let step = left.min(400);
            handle.play(step, channels);
            left -= step;
        }
    }

    pub fn drain_events(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

/// Tracks announced as audible, in order, without consecutive repeats.
pub fn heard(events: &[Event]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for e in events {
        if let Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(t),
        } = e
        {
            if out.last() != Some(&t.0) {
                out.push(t.0);
            }
        }
    }
    out
}

/// Longest run of samples (per channel 0) below `threshold` in absolute value.
pub fn longest_quiet_run(interleaved: &[f32], channels: usize, threshold: f32) -> usize {
    let (mut best, mut run) = (0, 0);
    for frame in interleaved.chunks_exact(channels) {
        if frame[0].abs() < threshold {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}
