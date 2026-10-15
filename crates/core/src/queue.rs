//! Playback queue: a context (album, playlist, search results...) played in order, plus a
//! user queue that takes priority. Pure logic, no I/O.

use std::collections::VecDeque;

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::TrackId;

/// "Previous" within this many milliseconds goes back a track; later it restarts the track.
pub const RESTART_THRESHOLD_MS: u64 = 3_000;
const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Previous {
    Restart,
    Track(TrackId),
}

/// A track that was current, with the context index the queue was at. Storing the context
/// index (not the position in the play order) keeps history valid across shuffle toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HistoryEntry {
    track: TrackId,
    context_index: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    context: Vec<TrackId>,
    /// Play order: indices into `context`.
    order: Vec<usize>,
    /// Position in `order` of the last context track that was current.
    cursor: Option<usize>,
    user_queue: VecDeque<TrackId>,
    history: Vec<HistoryEntry>,
    current: Option<TrackId>,
    shuffle: bool,
    repeat: RepeatMode,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<TrackId> {
        self.current
    }

    pub fn context(&self) -> &[TrackId] {
        &self.context
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Replaces the context and makes `tracks[start_index]` current. History is cleared:
    /// it refers to the previous context.
    pub fn play_context(&mut self, tracks: Vec<TrackId>, start_index: usize, rng: &mut impl Rng) -> Option<TrackId> {
        self.history.clear();
        self.context = tracks;
        if start_index >= self.context.len() {
            self.order.clear();
            self.cursor = None;
            self.current = None;
            return None;
        }
        self.order = self.build_order(start_index, rng);
        self.cursor = self.order.iter().position(|&i| i == start_index);
        self.current = Some(self.context[start_index]);
        self.current
    }

    /// Play order for the context. Natural order; with shuffle, `anchor` first and the
    /// rest in random order.
    fn build_order(&self, anchor: usize, rng: &mut impl Rng) -> Vec<usize> {
        if !self.shuffle {
            return (0..self.context.len()).collect();
        }
        let mut rest: Vec<usize> = (0..self.context.len()).filter(|&i| i != anchor).collect();
        rest.shuffle(rng);
        std::iter::once(anchor).chain(rest).collect()
    }

    /// Turns shuffle on/off keeping the current context track in place.
    pub fn set_shuffle(&mut self, on: bool, rng: &mut impl Rng) {
        if on == self.shuffle {
            return;
        }
        self.shuffle = on;
        let Some(anchor) = self.cursor.map(|c| self.order[c]) else {
            return;
        };
        self.order = self.build_order(anchor, rng);
        self.cursor = self.order.iter().position(|&i| i == anchor);
    }

    pub fn queue_add(&mut self, tracks: &[TrackId]) {
        self.user_queue.extend(tracks.iter().copied());
    }

    pub fn queue_remove(&mut self, index: usize) -> Option<TrackId> {
        self.user_queue.remove(index)
    }

    /// Moves a user-queue entry; `false` if either index is out of range.
    pub fn queue_move(&mut self, from: usize, to: usize) -> bool {
        if from >= self.user_queue.len() || to >= self.user_queue.len() {
            return false;
        }
        let track = self.user_queue.remove(from).expect("index checked");
        self.user_queue.insert(to, track);
        true
    }

    pub fn user_queue(&self) -> Vec<TrackId> {
        self.user_queue.iter().copied().collect()
    }

    /// What plays next: the user queue, then the rest of the context in play order.
    pub fn upcoming(&self, limit: usize) -> Vec<TrackId> {
        let from = self.cursor.map_or(0, |c| c + 1);
        self.user_queue
            .iter()
            .copied()
            .chain(self.order.iter().skip(from).map(|&i| self.context[i]))
            .take(limit)
            .collect()
    }

    /// Manual "next": always moves on, even with repeat-one.
    #[allow(clippy::should_implement_trait)] // a playback queue's `next` is not an iterator
    pub fn next(&mut self) -> Option<TrackId> {
        self.advance(false)
    }

    /// The current track finished playing on its own.
    pub fn on_track_ended(&mut self) -> Option<TrackId> {
        self.advance(true)
    }

    fn advance(&mut self, track_ended: bool) -> Option<TrackId> {
        if track_ended && self.repeat == RepeatMode::One && self.current.is_some() {
            return self.current;
        }
        self.push_history();
        if let Some(track) = self.user_queue.pop_front() {
            self.current = Some(track);
            return self.current;
        }
        let next = self.cursor.map_or(0, |c| c + 1);
        let position = if next < self.order.len() {
            Some(next)
        } else if self.repeat != RepeatMode::Off && !self.order.is_empty() {
            Some(0)
        } else {
            None
        };
        match position {
            Some(p) => {
                self.cursor = Some(p);
                self.current = Some(self.context[self.order[p]]);
            }
            None => self.current = None,
        }
        self.current
    }

    fn push_history(&mut self) {
        if let Some(track) = self.current {
            let context_index = self.cursor.map(|c| self.order[c]);
            self.history.push(HistoryEntry { track, context_index });
            if self.history.len() > HISTORY_LIMIT {
                self.history.remove(0);
            }
        }
    }

    /// "Previous" button: restart the track after [`RESTART_THRESHOLD_MS`], otherwise go
    /// back to the last track that was current.
    pub fn previous(&mut self, position_ms: u64) -> Previous {
        if position_ms > RESTART_THRESHOLD_MS {
            return Previous::Restart;
        }
        match self.history.pop() {
            Some(entry) => {
                self.current = Some(entry.track);
                self.cursor = entry
                    .context_index
                    .and_then(|ci| self.order.iter().position(|&i| i == ci));
                Previous::Track(entry.track)
            }
            None => Previous::Restart,
        }
    }
}
