//! The playback state machine: owns the queue, decodes the current track into the sink and,
//! near the end, the next one right behind it (gapless). Synchronous and deterministic: the
//! `Player` thread drives it, and tests drive it directly by calling [`Engine::pump`].
//!
//! Invariant: `queue.current()` is the track being *heard*. A successor already decoded into
//! the sink buffer is only committed to the queue (`on_track_ended`) when it becomes audible.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::queue::{Previous, Queue, RepeatMode};
use goymusic_core::TrackId;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::convert::Converter;
use crate::decoder::Decoder;
use crate::gain::{self, ReplayGain};
use crate::sink::{AudioSink, SinkFactory, SinkFormat};

/// Decode ahead until the sink holds this much audio.
const BUFFER_TARGET_MS: u64 = 500;
/// Open the next track's decoder when this much of the current one is left to decode.
pub const PRELOAD_MS: u64 = 10_000;
/// Bound on the work done by one `pump` call, so commands stay responsive.
const MAX_CHUNKS_PER_PUMP: usize = 16;

/// Track id → file on disk. The library provides it in the app; tests use a map.
pub type Resolver = Box<dyn Fn(TrackId) -> Option<PathBuf> + Send>;

/// Everything needed to save and restore the player (spec §5: restored paused).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    pub state: PlaybackState,
    pub track: Option<TrackId>,
    pub position_ms: u64,
    pub volume: f32,
    pub replaygain: bool,
    pub queue: Queue,
}

/// A track being decoded and converted to the sink's format.
struct Feed {
    track: TrackId,
    decoder: Decoder,
    converter: Converter,
    replaygain: f32,
    /// Frames produced so far, in sink frames, counted from the start of the track.
    frames_out: u64,
    /// Track length in sink frames, when the container declares it.
    total_out: Option<u64>,
    /// Whether any audio came out of this track at all.
    produced: bool,
}

impl Feed {
    /// Next converted block (possibly empty while the resampler fills); `None` at the end
    /// of the file. The converter is not flushed here: a gapless successor may inherit it.
    fn next_samples(&mut self, channels: usize) -> Result<Option<Vec<f32>>, String> {
        let Some(chunk) = self.decoder.next_chunk().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let mut samples = self.converter.push(&chunk).map_err(|e| e.to_string())?;
        gain::apply(&mut samples, self.replaygain);
        self.frames_out += (samples.len() / channels) as u64;
        self.produced |= !samples.is_empty();
        Ok(Some(samples))
    }

    fn frames_left(&self) -> Option<u64> {
        self.total_out.map(|t| t.saturating_sub(self.frames_out))
    }
}

/// A track's audio inside the sink buffer, starting at frame `start` of what was written.
struct Segment {
    track: TrackId,
    start: u64,
    /// Track position (sink frames) of the segment's first frame: non-zero after a seek.
    offset: u64,
}

pub struct Engine {
    factory: SinkFactory,
    sink: Option<Box<dyn AudioSink>>,
    resolver: Resolver,
    events: Arc<EventBus>,
    queue: Queue,
    rng: StdRng,
    state: PlaybackState,
    published: Option<(PlaybackState, Option<TrackId>)>,
    volume: f32,
    replaygain: bool,
    feed: Option<Feed>,
    preload: Option<Feed>,
    /// Next-track id already tried for preloading, so a broken file is not reopened each pump.
    preload_tried: Option<TrackId>,
    /// Converted samples the sink has not accepted yet.
    pending: Vec<f32>,
    /// Frames accepted by the sink since the last reset.
    written: u64,
    /// `sink.played_frames()` at the last reset.
    played_base: u64,
    segments: VecDeque<Segment>,
    /// Where to resume once a sink is available again (device lost / none at start).
    resume_at: Option<(TrackId, u64)>,
    /// Consecutive tracks that ended without producing audio (empty or undecodable files).
    /// Once it exceeds the queue length, every track left is silent: stop instead of spinning.
    silent_streak: usize,
}

impl Engine {
    pub fn new(mut factory: SinkFactory, resolver: Resolver, events: Arc<EventBus>) -> Self {
        let sink = factory().ok();
        let mut engine = Self {
            factory,
            sink,
            resolver,
            events,
            queue: Queue::new(),
            rng: StdRng::from_entropy(),
            state: PlaybackState::Idle,
            published: None,
            volume: 1.0,
            replaygain: false,
            feed: None,
            preload: None,
            preload_tried: None,
            pending: Vec::new(),
            written: 0,
            played_base: 0,
            segments: VecDeque::new(),
            resume_at: None,
            silent_streak: 0,
        };
        engine.apply_volume();
        engine
    }

    // ---- queries ----

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// The track being heard.
    pub fn current(&self) -> Option<TrackId> {
        self.segments.front().map(|s| s.track).or(self.queue.current())
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn position_ms(&self) -> u64 {
        let (Some(sink), Some(heard)) = (&self.sink, self.segments.front()) else {
            return self.resume_at.map_or(0, |(_, ms)| ms);
        };
        let played = self.played();
        let frames = heard.offset + played.saturating_sub(heard.start);
        frames * 1000 / u64::from(sink.format().sample_rate)
    }

    /// Next track whose decoder is already open (diagnostics and tests).
    pub fn preloaded(&self) -> Option<TrackId> {
        self.preload.as_ref().map(|f| f.track)
    }

    /// Tracks present in the sink buffer, heard first (diagnostics and tests).
    pub fn buffered_tracks(&self) -> Vec<TrackId> {
        self.segments.iter().map(|s| s.track).collect()
    }

    pub fn needs_sink(&self) -> bool {
        self.sink.is_none()
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            state: self.state,
            track: self.current(),
            position_ms: self.position_ms(),
            volume: self.volume,
            replaygain: self.replaygain,
            queue: self.queue.clone(),
        }
    }

    // ---- commands ----

    pub fn play_context(&mut self, tracks: Vec<TrackId>, start_index: usize) {
        self.silent_streak = 0;
        let first = self.queue.play_context(tracks, start_index, &mut self.rng);
        self.start(first, 0, PlaybackState::Playing);
        self.events.publish(Event::QueueChanged);
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.set_state(PlaybackState::Paused);
        }
    }

    /// From `Paused`, continues. From `Ended`, plays the context again from the start.
    pub fn resume(&mut self) {
        if self.state == PlaybackState::Ended {
            let context = self.queue.context().to_vec();
            if !context.is_empty() {
                self.play_context(context, 0);
            }
            return;
        }
        if self.state == PlaybackState::Paused {
            if self.sink.is_none() && !self.try_reopen_sink() {
                return;
            }
            self.set_state(PlaybackState::Playing);
        }
    }

    /// Manual "next": always moves on, even with repeat-one.
    pub fn next(&mut self) {
        if self.queue.current().is_none() {
            return;
        }
        self.silent_streak = 0;
        let next = self.queue.next();
        self.start(next, 0, PlaybackState::Playing);
        self.events.publish(Event::QueueChanged);
    }

    /// Restarts the track after 3 s, otherwise goes back; also works once the queue has
    /// ended (the history still holds the last track).
    pub fn previous(&mut self) {
        let target = match self.queue.previous(self.position_ms()) {
            Previous::Restart => self.queue.current(),
            Previous::Track(track) => Some(track),
        };
        if target.is_some() {
            self.silent_streak = 0;
            self.start(target, 0, PlaybackState::Playing);
            self.events.publish(Event::QueueChanged);
        }
    }

    pub fn seek(&mut self, position_ms: u64) {
        if let Some(track) = self.queue.current() {
            let then = if self.state == PlaybackState::Paused {
                PlaybackState::Paused
            } else {
                PlaybackState::Playing
            };
            self.start(Some(track), position_ms, then);
            self.announce_position();
        }
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.queue.set_shuffle(on, &mut self.rng);
        self.queue_changed();
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.queue.set_repeat(mode);
        self.queue_changed();
    }

    pub fn queue_add(&mut self, tracks: &[TrackId]) {
        self.queue.queue_add(tracks);
        self.queue_changed();
    }

    pub fn queue_remove(&mut self, index: usize) {
        if self.queue.queue_remove(index).is_some() {
            self.queue_changed();
        }
    }

    pub fn queue_move(&mut self, from: usize, to: usize) {
        if self.queue.queue_move(from, to) {
            self.queue_changed();
        }
    }

    /// Volume 0.0–1.0; heard immediately (applied by the sink).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_volume();
    }

    /// Takes effect from the next track that is opened.
    pub fn set_replaygain(&mut self, on: bool) {
        self.replaygain = on;
    }

    /// Restores a saved queue, paused at `position_ms` of its current track.
    pub fn restore(&mut self, queue: Queue, position_ms: u64) {
        self.queue = queue;
        self.silent_streak = 0;
        let current = self.queue.current();
        self.start(current, position_ms, PlaybackState::Paused);
        self.events.publish(Event::QueueChanged);
        self.announce_position();
    }

    // ---- driving ----

    /// Moves audio from the decoder to the sink and notices track changes and the end of
    /// the queue. Call it in a loop; it does a bounded amount of work.
    pub fn pump(&mut self) {
        if let Some(error) = self.sink.as_mut().and_then(|s| s.take_error()) {
            self.device_failed(error.to_string());
            return;
        }
        if self.state != PlaybackState::Playing {
            return;
        }
        let Some(format) = self.sink.as_ref().map(|s| s.format()) else {
            return;
        };
        let target = BUFFER_TARGET_MS * u64::from(format.sample_rate) / 1000;
        for _ in 0..MAX_CHUNKS_PER_PUMP {
            if !self.flush_pending(format) {
                break;
            }
            if self.sink.as_ref().is_some_and(|s| s.queued_frames() >= target) {
                break;
            }
            if !self.decode_more(format) {
                break;
            }
            self.maybe_preload(format);
        }
        if self.state != PlaybackState::Playing {
            return; // decode_more gave up on a queue of silent tracks
        }
        self.flush_pending(format);
        self.maybe_preload(format);
        self.update_heard();
        let drained = self.sink.as_ref().is_some_and(|s| s.queued_frames() == 0);
        if self.feed.is_none() && self.pending.is_empty() && drained {
            match self.queue.on_track_ended() {
                Some(next) => self.start(Some(next), 0, PlaybackState::Playing),
                None => {
                    self.set_state(PlaybackState::Ended);
                    self.reset_output();
                }
            }
            self.events.publish(Event::QueueChanged);
        }
    }

    pub fn publish_position(&self) {
        if let (PlaybackState::Playing, Some(track)) = (self.state, self.current()) {
            self.events.publish(Event::PositionChanged {
                track,
                position_ms: self.position_ms(),
            });
        }
    }

    /// Tries to open the output again after a failure. Returns whether a sink is available.
    pub fn try_reopen_sink(&mut self) -> bool {
        if self.sink.is_some() {
            return true;
        }
        match (self.factory)() {
            Ok(sink) => {
                self.sink = Some(sink);
                self.played_base = self.sink.as_ref().map_or(0, |s| s.played_frames());
                self.apply_volume();
                if let Some((track, position_ms)) = self.resume_at.take() {
                    self.start(Some(track), position_ms, PlaybackState::Paused);
                }
                true
            }
            Err(_) => false,
        }
    }

    // ---- internals ----

    /// `PositionChanged` in any state: after a seek or restore the UI must move the slider
    /// even while paused (it never polls).
    fn announce_position(&self) {
        if let Some(track) = self.current() {
            self.events.publish(Event::PositionChanged {
                track,
                position_ms: self.position_ms(),
            });
        }
    }

    fn played(&self) -> u64 {
        self.sink
            .as_ref()
            .map_or(0, |s| s.played_frames())
            .saturating_sub(self.played_base)
    }

    fn apply_volume(&mut self) {
        let amplitude = gain::volume_to_amplitude(self.volume);
        if let Some(sink) = self.sink.as_mut() {
            sink.set_volume(amplitude);
        }
    }

    fn set_state(&mut self, state: PlaybackState) {
        self.state = state;
        if let Some(sink) = self.sink.as_mut() {
            sink.set_paused(state != PlaybackState::Playing);
        }
        let now = (state, self.current());
        if self.published != Some(now) {
            self.published = Some(now);
            self.events.publish(Event::PlayerStateChanged {
                state: now.0,
                track: now.1,
            });
        }
    }

    /// Drops all buffered audio and decoders.
    fn reset_output(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.clear();
        }
        self.played_base = self.sink.as_ref().map_or(0, |s| s.played_frames());
        self.written = 0;
        self.pending.clear();
        self.feed = None;
        self.preload = None;
        self.preload_tried = None;
        self.segments.clear();
    }

    /// Starts `first` at `position_ms`. A track that cannot be opened is reported with
    /// `PlaybackError` and skipped, at most once per track in the queue, so a queue of
    /// broken files on repeat ends instead of looping forever.
    fn start(&mut self, first: Option<TrackId>, position_ms: u64, then: PlaybackState) {
        self.reset_output();
        if self.sink.is_none() && !self.try_reopen_sink() {
            if let Some(track) = first {
                self.resume_at = Some((track, position_ms));
                self.set_state(PlaybackState::Paused);
                self.events.publish(Event::AudioDeviceLost {
                    reason: "no audio output device".into(),
                });
            }
            return;
        }
        let format = self.sink.as_ref().expect("sink checked above").format();
        let mut attempts = self.queue.context().len() + self.queue.user_queue().len() + 1;
        let mut candidate = first;
        let mut position_ms = position_ms;
        while let Some(track) = candidate {
            self.segments.clear();
            self.segments.push_back(Segment {
                track,
                start: 0,
                offset: 0,
            });
            self.set_state(PlaybackState::Loading);
            match self.open_feed(track, position_ms, format) {
                Ok(feed) => {
                    self.segments[0].offset = feed.frames_out;
                    self.feed = Some(feed);
                    self.set_state(then);
                    return;
                }
                Err(reason) => {
                    self.events.publish(Event::PlaybackError { track, reason });
                    attempts -= 1;
                    if attempts == 0 {
                        break;
                    }
                    candidate = self.queue.next();
                    position_ms = 0;
                }
            }
        }
        self.segments.clear();
        self.set_state(if first.is_some() {
            PlaybackState::Ended
        } else {
            PlaybackState::Idle
        });
    }

    fn open_feed(&self, track: TrackId, position_ms: u64, format: SinkFormat) -> Result<Feed, String> {
        let path = (self.resolver)(track).ok_or_else(|| "track not in library".to_string())?;
        let mut decoder = Decoder::open(&path).map_err(|e| e.to_string())?;
        let in_rate = u64::from(decoder.sample_rate());
        let out_rate = u64::from(format.sample_rate);
        let start_frame = position_ms * in_rate / 1000;
        if start_frame > 0 {
            decoder.seek(start_frame).map_err(|e| e.to_string())?;
        }
        let converter = Converter::new(
            decoder.sample_rate(),
            decoder.channels(),
            format.sample_rate,
            format.channels as usize,
        )
        .map_err(|e| e.to_string())?;
        let replaygain = if self.replaygain {
            ReplayGain::read(&path).factor()
        } else {
            1.0
        };
        Ok(Feed {
            track,
            total_out: decoder.total_frames().map(|n| n * out_rate / in_rate),
            frames_out: start_frame * out_rate / in_rate,
            decoder,
            converter,
            replaygain,
            produced: false,
        })
    }

    /// Writes pending samples; `true` when nothing is left pending.
    fn flush_pending(&mut self, format: SinkFormat) -> bool {
        if self.pending.is_empty() {
            return true;
        }
        let Some(sink) = self.sink.as_mut() else {
            return false;
        };
        let accepted = sink.write(&self.pending);
        self.written += (accepted / format.channels as usize) as u64;
        self.pending.drain(..accepted);
        self.pending.is_empty()
    }

    /// Decodes one block into `pending`. At the end of a track (or on a decode error) the
    /// successor starts feeding right behind it. `false` when there is nothing to decode.
    fn decode_more(&mut self, format: SinkFormat) -> bool {
        let Some(feed) = self.feed.as_mut() else {
            return false;
        };
        match feed.next_samples(format.channels as usize) {
            Ok(Some(samples)) => {
                if !samples.is_empty() {
                    self.silent_streak = 0;
                }
                self.pending.extend(samples);
                true
            }
            Ok(None) => {
                let done = self.feed.take().expect("feed checked above");
                self.track_finished(done, format)
            }
            Err(reason) => {
                let done = self.feed.take().expect("feed checked above");
                self.events.publish(Event::PlaybackError {
                    track: done.track,
                    reason,
                });
                self.track_finished(done, format)
            }
        }
    }

    /// The feed ran out (or failed). Starts the successor behind it, unless every track in
    /// the queue has turned out silent, in which case playback ends. `false` when stopped.
    fn track_finished(&mut self, done: Feed, format: SinkFormat) -> bool {
        if !done.produced {
            self.silent_streak += 1;
            if self.silent_streak > self.queue.context().len() + self.queue.user_queue().len() {
                self.set_state(PlaybackState::Ended);
                self.reset_output();
                return false;
            }
        }
        self.start_successor(format, done.converter, done.replaygain);
        true
    }

    /// Flushes a converter's held frames into `pending` (end of a track with no gapless
    /// successor in the same input format).
    fn flush_converter(&mut self, mut converter: Converter, replaygain: f32) {
        if let Ok(mut tail) = converter.finish() {
            gain::apply(&mut tail, replaygain);
            self.pending.extend(tail);
        }
    }

    /// The track that follows the last one in the buffer, if the queue were left alone.
    fn successor(&self) -> Option<TrackId> {
        let mut queue = self.queue.clone();
        let mut next = None;
        for _ in 0..self.segments.len() {
            next = queue.on_track_ended();
        }
        next
    }

    /// Starts feeding the next track right behind the one that ended. If it has the same
    /// input format it inherits `previous` (the ended track's converter), so the resampler
    /// runs on across the join without the zero-padded flush that would click.
    fn start_successor(&mut self, format: SinkFormat, previous: Converter, previous_gain: f32) {
        let channels = format.channels as usize;
        // A successor that fails to open is left alone: when the buffer drains, `pump`
        // advances the queue and `start` reports the error and skips it.
        let next = self.successor().and_then(|track| match self.preload.take() {
            Some(preloaded) if preloaded.track == track => Some(preloaded),
            _ => self.open_feed(track, 0, format).ok(),
        });
        let Some(mut feed) = next else {
            self.flush_converter(previous, previous_gain);
            return;
        };
        let start = if previous.input_format() == feed.converter.input_format() {
            // frames of the ended track still inside the resampler come out first
            let start = self.written + (self.pending.len() / channels) as u64 + previous.owed_frames();
            feed.converter = previous;
            start
        } else {
            self.flush_converter(previous, previous_gain);
            self.written + (self.pending.len() / channels) as u64
        };
        self.segments.push_back(Segment {
            track: feed.track,
            start,
            offset: 0,
        });
        self.feed = Some(feed);
    }

    fn maybe_preload(&mut self, format: SinkFormat) {
        if self.preload.is_some() || self.segments.len() != 1 {
            return;
        }
        let rate = u64::from(format.sample_rate);
        let near_end = self
            .feed
            .as_ref()
            .and_then(Feed::frames_left)
            .is_some_and(|left| left <= PRELOAD_MS * rate / 1000);
        if !near_end {
            return;
        }
        let Some(next) = self.queue.peek_next() else {
            return;
        };
        if self.preload_tried == Some(next) {
            return;
        }
        self.preload_tried = Some(next);
        self.preload = self.open_feed(next, 0, format).ok();
    }

    /// Commits successors that became audible to the queue and announces them.
    fn update_heard(&mut self) {
        let played = self.played();
        while self.segments.len() > 1 && self.segments[1].start <= played {
            self.segments.pop_front();
            let heard = self.segments[0].track;
            let committed = self.queue.on_track_ended();
            if committed != Some(heard) {
                // `queue_changed` keeps the buffer in step with the queue, so this is a
                // safety net: follow the queue rather than desynchronise.
                let state = self.state;
                self.start(committed, 0, state);
                return;
            }
            self.preload_tried = None;
            let state = self.state;
            self.set_state(state);
            self.events.publish(Event::QueueChanged);
        }
    }

    /// Whether the tracks buffered behind the heard one still follow the queue's order.
    fn buffer_matches_queue(&self) -> bool {
        let mut queue = self.queue.clone();
        self.segments
            .iter()
            .skip(1)
            .all(|s| queue.on_track_ended() == Some(s.track))
    }

    /// After a queue edit: a preload or buffered successors that are no longer next are
    /// dropped. Buffered ones mean restarting the heard track where it is (rare: only in
    /// the last half second of a track).
    fn queue_changed(&mut self) {
        let next = self.queue.peek_next();
        if self.preload.as_ref().is_some_and(|p| Some(p.track) != next) {
            self.preload = None;
        }
        self.preload_tried = None;
        if !self.buffer_matches_queue() {
            if let Some(track) = self.queue.current() {
                let position_ms = self.position_ms();
                let state = self.state;
                self.start(Some(track), position_ms, state);
            }
        }
        self.events.publish(Event::QueueChanged);
    }

    /// The sink reported an error. Reopen the default device at once and carry on where
    /// the music was (ALSA also reports transient errors); only if that fails, pause, tell
    /// the UI and leave the retries to the player thread.
    fn device_failed(&mut self, reason: String) {
        let was = self.state;
        if let Some(track) = self.current() {
            self.resume_at = Some((track, self.position_ms()));
        }
        self.sink = None;
        self.feed = None;
        self.preload = None;
        self.pending.clear();
        self.segments.clear();
        if self.try_reopen_sink() {
            if was == PlaybackState::Playing {
                self.set_state(PlaybackState::Playing);
            }
            return;
        }
        self.set_state(PlaybackState::Paused);
        self.events.publish(Event::AudioDeviceLost { reason });
    }
}
