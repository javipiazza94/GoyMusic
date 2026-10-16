//! Forwards every core event to the webview as the typed `CoreEvent` (spec §3: the UI never
//! polls).

use std::sync::Arc;

use goymusic_core::events::{Event, EventBus};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event as _;

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct CoreEvent(pub Event);

/// Runs until the bus is dropped (the services shut down).
pub fn start(app: AppHandle, events: &Arc<EventBus>) {
    let rx = events.subscribe();
    std::thread::Builder::new()
        .name("goymusic-forward".into())
        .spawn(move || {
            for event in rx {
                if let Err(e) = CoreEvent(event).emit(&app) {
                    tracing::warn!(error = %e, "could not forward event to the UI");
                }
            }
        })
        .expect("spawn event forwarding thread");
}
