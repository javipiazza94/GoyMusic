//! One open audio file: probes the container, decodes packets to interleaved `f32` and seeks
//! to an exact frame. Gapless trimming of encoder delay/padding (MP3, Vorbis) is done by
//! symphonia itself once `enable_gapless` is set, so it must not be repeated here.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder as CodecDecoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{AudioError, Result};

/// Consecutive undecodable packets tolerated before the file is declared broken.
const MAX_BAD_PACKETS: u32 = 50;

pub struct Decoder {
    format: Box<dyn FormatReader>,
    codec: Box<dyn CodecDecoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
    total_frames: Option<u64>,
    /// Frames to drop from the next decoded packets (accurate seek lands before the target).
    skip_frames: u64,
    buffer: Option<SampleBuffer<f32>>,
}

impl Decoder {
    pub fn open(path: &Path) -> Result<Self> {
        let open_err = |reason: String| AudioError::Open {
            path: path.display().to_string(),
            reason,
        };
        let file = File::open(path).map_err(|e| open_err(e.to_string()))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                MediaSourceStream::new(Box::new(file), Default::default()),
                &format_opts,
                &MetadataOptions::default(),
            )
            .map_err(|e| open_err(e.to_string()))?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AudioError::Unsupported("no audio track".into()))?;
        let params = &track.codec_params;
        let sample_rate = params
            .sample_rate
            .ok_or_else(|| AudioError::Unsupported("unknown sample rate".into()))?;
        let channels = params
            .channels
            .map(|c| c.count())
            .ok_or_else(|| AudioError::Unsupported("unknown channel layout".into()))?;
        let codec = symphonia::default::get_codecs()
            .make(params, &DecoderOptions::default())
            .map_err(|e| AudioError::Unsupported(e.to_string()))?;
        Ok(Self {
            track_id: track.id,
            total_frames: params.n_frames,
            format,
            codec,
            sample_rate,
            channels,
            skip_frames: 0,
            buffer: None,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Length in frames when the container declares it (after gapless trimming).
    pub fn total_frames(&self) -> Option<u64> {
        self.total_frames
    }

    /// Next block of interleaved samples; `None` at the end of the stream.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>> {
        let mut bad_packets = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(SymphoniaError::ResetRequired) => return Ok(None),
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.codec.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::DecodeError(e)) => {
                    bad_packets += 1;
                    if bad_packets > MAX_BAD_PACKETS {
                        return Err(AudioError::Decode(e.to_string()));
                    }
                    continue;
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };
            if decoded.frames() == 0 {
                continue;
            }
            let needed = decoded.capacity() as u64;
            let buffer = match &mut self.buffer {
                Some(b) if b.capacity() as u64 >= needed => b,
                slot => slot.insert(SampleBuffer::new(needed, *decoded.spec())),
            };
            buffer.copy_interleaved_ref(decoded);
            let mut samples = buffer.samples().to_vec();
            if self.skip_frames > 0 {
                let frames = (samples.len() / self.channels) as u64;
                let drop = self.skip_frames.min(frames);
                self.skip_frames -= drop;
                samples.drain(..drop as usize * self.channels);
            }
            if !samples.is_empty() {
                return Ok(Some(samples));
            }
        }
    }

    /// Positions the stream so the next sample returned is frame `frame` (clamped to the end).
    pub fn seek(&mut self, frame: u64) -> Result<()> {
        let frame = self
            .total_frames
            .map_or(frame, |total| frame.min(total.saturating_sub(1)));
        // Timestamps are in the track's time base (usually 1/sample_rate). Integer maths: a
        // float round trip loses a frame (29_999 / 8_000 s seeks to frame 29_998).
        let (numer, denom) = self.time_base();
        let ts = (u128::from(frame) * u128::from(denom) / (u128::from(numer) * u128::from(self.sample_rate))) as u64;
        let seeked = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::TimeStamp {
                    ts,
                    track_id: self.track_id,
                },
            )
            .map_err(|e| AudioError::Decode(format!("seek: {e}")))?;
        self.codec.reset();
        let late = seeked.required_ts.saturating_sub(seeked.actual_ts);
        self.skip_frames =
            (u128::from(late) * u128::from(numer) * u128::from(self.sample_rate) / u128::from(denom)) as u64;
        Ok(())
    }

    /// `(numer, denom)` of the track's time base; 1/sample_rate when the container omits it.
    fn time_base(&self) -> (u32, u32) {
        self.format
            .tracks()
            .iter()
            .find(|t| t.id == self.track_id)
            .and_then(|t| t.codec_params.time_base)
            .map_or((1, self.sample_rate), |tb| (tb.numer, tb.denom))
    }
}
