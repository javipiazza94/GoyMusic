//! Runs the [`Engine`] on its own thread (spec §5) and exposes the player service API (§4).
//! Commands go in over a channel; state comes out as events on the core `EventBus`.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use goymusic_core::events::{EventBus, PlaybackState};
use goymusic_core::queue::{Queue, RepeatMode};
use goymusic_core::TrackId;

use crate::engine::{Engine, PlayerSnapshot, Resolver};
use crate::sink::SinkFactory;

/// Poll interval while audio is flowing: well under the 500 ms buffer.
const ACTIVE_TICK: Duration = Duration::from_millis(5);
const IDLE_TICK: Duration = Duration::from_millis(250);
const POSITION_EVERY: Duration = Duration::from_millis(250);
const DEVICE_RETRY_EVERY: Duration = Duration::from_secs(2);

enum Command {
    PlayContext(Vec<TrackId>, usize),
    Pause,
    Resume,
    Next,
    Previous,
    Seek(u64),
    SetShuffle(bool),
    SetRepeat(RepeatMode),
    SetVolume(f32),
    SetReplayGain(bool),
    QueueAdd(Vec<TrackId>),
    QueueRemove(usize),
    QueueMove(usize, usize),
    Restore(Queue, u64),
    Snapshot(Sender<PlayerSnapshot>),
    Shutdown,
}

/// Handle to the player thread. Dropping it stops playback and joins the thread.
pub struct Player {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    pub fn spawn(factory: SinkFactory, resolver: Resolver, events: Arc<EventBus>) -> Self {
        let (commands, rx) = channel();
        let thread = std::thread::Builder::new()
            .name("goymusic-audio".into())
            .spawn(move || run(Engine::new(factory, resolver, events), rx))
            .expect("spawn audio thread");
        Self {
            commands,
            thread: Some(thread),
        }
    }

    fn send(&self, command: Command) {
        // the thread only exits on Shutdown, so a send error means it panicked; nothing to do
        let _ = self.commands.send(command);
    }

    pub fn play(&self, context: Vec<TrackId>, start_index: usize) {
        self.send(Command::PlayContext(context, start_index));
    }
    pub fn pause(&self) {
        self.send(Command::Pause);
    }
    pub fn resume(&self) {
        self.send(Command::Resume);
    }
    pub fn next(&self) {
        self.send(Command::Next);
    }
    pub fn previous(&self) {
        self.send(Command::Previous);
    }
    pub fn seek(&self, position_ms: u64) {
        self.send(Command::Seek(position_ms));
    }
    pub fn set_shuffle(&self, on: bool) {
        self.send(Command::SetShuffle(on));
    }
    pub fn set_repeat(&self, mode: RepeatMode) {
        self.send(Command::SetRepeat(mode));
    }
    pub fn set_volume(&self, volume: f32) {
        self.send(Command::SetVolume(volume));
    }
    pub fn set_replaygain(&self, on: bool) {
        self.send(Command::SetReplayGain(on));
    }
    pub fn queue_add(&self, tracks: Vec<TrackId>) {
        self.send(Command::QueueAdd(tracks));
    }
    pub fn queue_remove(&self, index: usize) {
        self.send(Command::QueueRemove(index));
    }
    pub fn queue_move(&self, from: usize, to: usize) {
        self.send(Command::QueueMove(from, to));
    }
    pub fn restore(&self, queue: Queue, position_ms: u64) {
        self.send(Command::Restore(queue, position_ms));
    }

    /// Current state, answered by the audio thread after every command sent before it.
    pub fn state(&self) -> Option<PlayerSnapshot> {
        let (tx, rx) = channel();
        self.send(Command::Snapshot(tx));
        rx.recv_timeout(Duration::from_secs(2)).ok()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(mut engine: Engine, commands: Receiver<Command>) {
    let mut last_position = Instant::now();
    let mut last_retry = Instant::now();
    loop {
        let tick = if engine.state() == PlaybackState::Playing {
            ACTIVE_TICK
        } else {
            IDLE_TICK
        };
        match commands.recv_timeout(tick) {
            Ok(command) => {
                if !apply(&mut engine, command) {
                    return;
                }
                while let Ok(command) = commands.try_recv() {
                    if !apply(&mut engine, command) {
                        return;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        engine.pump();
        if last_position.elapsed() >= POSITION_EVERY {
            engine.publish_position();
            last_position = Instant::now();
        }
        if engine.needs_sink() && last_retry.elapsed() >= DEVICE_RETRY_EVERY {
            engine.try_reopen_sink();
            last_retry = Instant::now();
        }
    }
}

/// `false` on shutdown.
fn apply(engine: &mut Engine, command: Command) -> bool {
    match command {
        Command::PlayContext(tracks, start) => engine.play_context(tracks, start),
        Command::Pause => engine.pause(),
        Command::Resume => engine.resume(),
        Command::Next => engine.next(),
        Command::Previous => engine.previous(),
        Command::Seek(ms) => engine.seek(ms),
        Command::SetShuffle(on) => engine.set_shuffle(on),
        Command::SetRepeat(mode) => engine.set_repeat(mode),
        Command::SetVolume(v) => engine.set_volume(v),
        Command::SetReplayGain(on) => engine.set_replaygain(on),
        Command::QueueAdd(tracks) => engine.queue_add(&tracks),
        Command::QueueRemove(i) => engine.queue_remove(i),
        Command::QueueMove(from, to) => engine.queue_move(from, to),
        Command::Restore(queue, ms) => engine.restore(queue, ms),
        Command::Snapshot(reply) => {
            let _ = reply.send(engine.snapshot());
        }
        Command::Shutdown => return false,
    }
    true
}
