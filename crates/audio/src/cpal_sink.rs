//! The real output: default device through cpal, fed by a lock-free ring buffer. The audio
//! callback never blocks or allocates in the steady state; it pops samples, applies the
//! volume and counts played frames. Pause outputs silence instead of stopping the stream,
//! so `clear` can always rely on the callback to empty the buffer.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use crate::sink::{AudioSink, SinkError, SinkFormat};

/// Ring buffer size: one second of audio, double the engine's 500 ms target.
const BUFFER_SECONDS: usize = 1;

#[derive(Default)]
struct Shared {
    played_frames: AtomicU64,
    paused: AtomicBool,
    flush: AtomicBool,
    /// `f32` bits of the output amplitude.
    amplitude: AtomicU32,
    error: Mutex<Option<String>>,
}

pub struct CpalSink {
    format: SinkFormat,
    producer: HeapProd<f32>,
    shared: Arc<Shared>,
    // Keeps the stream alive. Not `Send`: the sink lives on the engine thread.
    _stream: cpal::Stream,
}

impl CpalSink {
    pub fn open_default() -> Result<Self, SinkError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(SinkError::NoDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| SinkError::Device(e.to_string()))?;
        let config = supported.config();
        let format = SinkFormat {
            sample_rate: config.sample_rate.0,
            channels: config.channels,
        };
        let capacity = format.sample_rate as usize * format.channels as usize * BUFFER_SECONDS;
        let (producer, consumer) = HeapRb::<f32>::new(capacity).split();
        let shared = Arc::new(Shared {
            amplitude: AtomicU32::new(1f32.to_bits()),
            ..Default::default()
        });
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, &config, consumer, shared.clone()),
            SampleFormat::I16 => build::<i16>(&device, &config, consumer, shared.clone()),
            SampleFormat::U16 => build::<u16>(&device, &config, consumer, shared.clone()),
            SampleFormat::I32 => build::<i32>(&device, &config, consumer, shared.clone()),
            other => return Err(SinkError::Device(format!("unsupported sample format {other}"))),
        }?;
        stream.play().map_err(|e| SinkError::Device(e.to_string()))?;
        Ok(Self {
            format,
            producer,
            shared,
            _stream: stream,
        })
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: HeapCons<f32>,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, SinkError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let errors = shared.clone();
    let mut scratch: Vec<f32> = Vec::with_capacity(16_384);
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                // empty first, acknowledge second: `clear` returns as soon as it sees `false`
                if shared.flush.load(Ordering::Acquire) {
                    consumer.clear();
                    shared.flush.store(false, Ordering::Release);
                }
                let n = if shared.paused.load(Ordering::Acquire) {
                    0
                } else {
                    scratch.resize(out.len(), 0.0);
                    consumer.pop_slice(&mut scratch)
                };
                let amplitude = f32::from_bits(shared.amplitude.load(Ordering::Relaxed));
                for (o, s) in out.iter_mut().zip(&scratch[..n]) {
                    *o = T::from_sample(s * amplitude);
                }
                for o in &mut out[n..] {
                    *o = T::EQUILIBRIUM;
                }
                shared.played_frames.fetch_add((n / channels) as u64, Ordering::Release);
            },
            move |err| {
                *errors.error.lock().unwrap() = Some(err.to_string());
            },
            None,
        )
        .map_err(|e| SinkError::Device(e.to_string()))
}

impl AudioSink for CpalSink {
    fn format(&self) -> SinkFormat {
        self.format
    }

    fn write(&mut self, samples: &[f32]) -> usize {
        let channels = self.format.channels as usize;
        let room = self.producer.vacant_len() / channels * channels;
        self.producer.push_slice(&samples[..samples.len().min(room)])
    }

    fn queued_frames(&self) -> u64 {
        (self.producer.occupied_len() / self.format.channels as usize) as u64
    }

    fn played_frames(&self) -> u64 {
        self.shared.played_frames.load(Ordering::Acquire)
    }

    fn set_paused(&mut self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
    }

    fn set_volume(&mut self, amplitude: f32) {
        self.shared.amplitude.store(amplitude.to_bits(), Ordering::Relaxed);
    }

    /// Asks the callback to empty the buffer and waits for it (it runs every few ms even
    /// when paused). Gives up after 200 ms, e.g. if the device died meanwhile.
    fn clear(&mut self) {
        self.shared.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_millis(200);
        while self.shared.flush.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn take_error(&mut self) -> Option<SinkError> {
        self.shared.error.lock().unwrap().take().map(SinkError::Device)
    }
}
