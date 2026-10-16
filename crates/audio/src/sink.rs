//! Audio output abstraction. The engine writes interleaved `f32` in the sink's format; the
//! sink plays it. `CpalSink` is the real device, `CaptureSink` records samples for tests.

use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SinkFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SinkError {
    #[error("no audio output device")]
    NoDevice,
    #[error("audio device error: {0}")]
    Device(String),
}

pub trait AudioSink {
    fn format(&self) -> SinkFormat;
    /// Queues interleaved samples without blocking; returns how many were accepted, always a
    /// whole number of frames. Fewer than offered means the buffer is full.
    fn write(&mut self, samples: &[f32]) -> usize;
    /// Frames queued but not played yet.
    fn queued_frames(&self) -> u64;
    /// Frames played since the sink was created. Monotonic; `clear` does not reset it.
    fn played_frames(&self) -> u64;
    fn set_paused(&mut self, paused: bool);
    /// Amplitude applied at output time (see `gain::volume_to_amplitude`).
    fn set_volume(&mut self, amplitude: f32);
    /// Drops everything queued. On return `queued_frames() == 0`.
    fn clear(&mut self);
    /// A device failure since the last call, if any.
    fn take_error(&mut self) -> Option<SinkError>;
}

/// Opens a sink. The engine calls it at start and again after a device failure. It runs on
/// the engine thread because a cpal stream cannot move between threads.
pub type SinkFactory = Box<dyn FnMut() -> Result<Box<dyn AudioSink>, SinkError> + Send>;

/// Test sink: "plays" instantly into memory and records what was heard. Paused, or in hold
/// mode, samples wait in a queue instead; `CaptureHandle::play` then plays them on demand,
/// which lets tests reproduce a device that is behind the decoder.
pub struct CaptureSink {
    format: SinkFormat,
    state: Arc<Mutex<CaptureState>>,
}

#[derive(Default)]
struct CaptureState {
    played: Vec<f32>,
    queued: Vec<f32>,
    paused: bool,
    hold: bool,
    amplitude: f32,
    error: Option<SinkError>,
    clears: usize,
}

/// The test's view of a [`CaptureSink`] that the engine owns.
#[derive(Clone)]
pub struct CaptureHandle {
    state: Arc<Mutex<CaptureState>>,
}

impl CaptureSink {
    pub fn new(format: SinkFormat) -> (Self, CaptureHandle) {
        let state = Arc::new(Mutex::new(CaptureState {
            amplitude: 1.0,
            ..Default::default()
        }));
        (
            Self {
                format,
                state: state.clone(),
            },
            CaptureHandle { state },
        )
    }
}

impl AudioSink for CaptureSink {
    fn format(&self) -> SinkFormat {
        self.format
    }

    fn write(&mut self, samples: &[f32]) -> usize {
        let mut s = self.state.lock().unwrap();
        let amplitude = s.amplitude;
        if s.paused || s.hold || !s.queued.is_empty() {
            s.queued.extend_from_slice(samples);
        } else {
            s.played.extend(samples.iter().map(|x| x * amplitude));
        }
        samples.len()
    }

    fn queued_frames(&self) -> u64 {
        (self.state.lock().unwrap().queued.len() / self.format.channels as usize) as u64
    }

    fn played_frames(&self) -> u64 {
        (self.state.lock().unwrap().played.len() / self.format.channels as usize) as u64
    }

    fn set_paused(&mut self, paused: bool) {
        let mut s = self.state.lock().unwrap();
        s.paused = paused;
        if !paused && !s.hold {
            s.play_queued(usize::MAX);
        }
    }

    fn set_volume(&mut self, amplitude: f32) {
        self.state.lock().unwrap().amplitude = amplitude;
    }

    fn clear(&mut self) {
        let mut s = self.state.lock().unwrap();
        s.queued.clear();
        s.clears += 1;
    }

    fn take_error(&mut self) -> Option<SinkError> {
        self.state.lock().unwrap().error.take()
    }
}

impl CaptureState {
    fn play_queued(&mut self, samples: usize) {
        let n = samples.min(self.queued.len());
        let amplitude = self.amplitude;
        let moved: Vec<f32> = self.queued.drain(..n).map(|x| x * amplitude).collect();
        self.played.extend(moved);
    }
}

impl CaptureHandle {
    /// Hold mode: written samples stay queued until [`CaptureHandle::play`].
    pub fn hold(&self, on: bool) {
        self.state.lock().unwrap().hold = on;
    }

    /// Plays up to `frames` queued frames (ignored while paused).
    pub fn play(&self, frames: usize, channels: usize) {
        let mut s = self.state.lock().unwrap();
        if !s.paused {
            s.play_queued(frames * channels);
        }
    }

    /// Every sample played so far, interleaved.
    pub fn played(&self) -> Vec<f32> {
        self.state.lock().unwrap().played.clone()
    }

    pub fn clears(&self) -> usize {
        self.state.lock().unwrap().clears
    }

    /// Simulates a device failure, reported at the next `take_error`.
    pub fn fail(&self, error: SinkError) {
        self.state.lock().unwrap().error = Some(error);
    }
}
