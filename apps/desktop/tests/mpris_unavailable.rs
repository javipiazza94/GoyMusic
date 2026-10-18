//! Without a session bus, MPRIS gives up quietly: the app keeps playing (Review Focus 4).
//! Its own test binary, because it changes a process-wide environment variable.

#[path = "common_player.rs"]
mod common_player;

use std::sync::Arc;
use std::time::{Duration, Instant};

use goymusic_desktop::control::fake::RecordingControl;
use goymusic_desktop::mpris::{self, MprisHooks};

#[test]
fn mpris_unavailable_is_logged_not_fatal() {
    // SAFETY: the only test in this binary; nothing else reads the environment concurrently
    unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/goymusic-test-bus") };
    let control = Arc::new(RecordingControl::with_view(common_player::playing("Faro")));
    let (_tx, rx) = std::sync::mpsc::channel();
    let started = Instant::now();
    let thread = mpris::spawn(
        control,
        rx,
        MprisHooks {
            raise: Box::new(|| {}),
            quit: Box::new(|| {}),
        },
    );
    assert!(thread.join().is_ok(), "the MPRIS thread must end without panicking");
    assert!(started.elapsed() < Duration::from_secs(10));
}
