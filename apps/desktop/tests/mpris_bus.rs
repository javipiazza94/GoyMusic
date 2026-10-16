//! MPRIS on a real (isolated) session bus, driven the way `playerctl` or GNOME would.
//! Run with: dbus-run-session -- cargo test -p goymusic-desktop --test mpris_bus -- --ignored

#[path = "common_player.rs"]
mod common_player;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use goymusic_desktop::control::fake::RecordingControl;
use goymusic_desktop::mpris::{self, MprisHooks};
use mpris_server::zbus::{Connection, Proxy};

const DEST: &str = "org.mpris.MediaPlayer2.goymusic";
const PATH: &str = "/org/mpris/MediaPlayer2";

async fn player_proxy(conn: &Connection) -> Proxy<'static> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let proxy = Proxy::new(conn, DEST, PATH, "org.mpris.MediaPlayer2.Player")
            .await
            .unwrap();
        if proxy.get_property::<String>("PlaybackStatus").await.is_ok() {
            return proxy;
        }
        assert!(Instant::now() < deadline, "GoyMusic never appeared on the bus");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore = "needs a session bus: run under dbus-run-session"]
fn playerctl_style_clients_see_and_drive_the_player() {
    let control = Arc::new(RecordingControl::with_view(common_player::playing("Faro")));
    let raised = Arc::new(AtomicBool::new(false));
    let (_tx, rx) = std::sync::mpsc::channel();
    let r = raised.clone();
    mpris::spawn(
        control.clone(),
        rx,
        MprisHooks {
            raise: Box::new(move || r.store(true, Ordering::SeqCst)),
            quit: Box::new(|| {}),
        },
    );

    futures_lite::future::block_on(async {
        let conn = Connection::session().await.unwrap();
        let player = player_proxy(&conn).await;
        assert_eq!(
            player.get_property::<String>("PlaybackStatus").await.unwrap(),
            "Playing"
        );
        player.call_method("PlayPause", &()).await.unwrap();
        player.call_method("Next", &()).await.unwrap();
        let root = Proxy::new(&conn, DEST, PATH, "org.mpris.MediaPlayer2").await.unwrap();
        assert_eq!(root.get_property::<String>("Identity").await.unwrap(), "GoyMusic");
        root.call_method("Raise", &()).await.unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while control.calls().len() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(control.calls(), vec!["pause", "next"]);
    assert!(raised.load(Ordering::SeqCst));
}
