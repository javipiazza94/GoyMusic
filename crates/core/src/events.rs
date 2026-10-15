//! Core events. Services publish them; adapters (Tauri now, WebSocket later) forward them
//! to the UI, so the UI never polls.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    LibraryUpdated,
    ScanProgress { done: u64, total: u64 },
    PlaylistsChanged,
    LikesChanged,
    QueueChanged,
}

#[derive(Default)]
pub struct EventBus {
    subscribers: Mutex<Vec<Sender<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.subscribers.lock().expect("event bus poisoned").push(tx);
        rx
    }

    /// Sends `event` to every live subscriber and forgets the ones whose receiver was dropped.
    pub fn publish(&self, event: Event) {
        self.subscribers
            .lock()
            .expect("event bus poisoned")
            .retain(|s| s.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().expect("event bus poisoned").len()
    }
}
