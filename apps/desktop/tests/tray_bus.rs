//! Whether a tray host exists is asked on the session bus: Tauri builds the tray icon even when
//! nothing displays it (GNOME without the AppIndicator extension), so success there proves nothing.
//! Run with: dbus-run-session -- cargo test -p goymusic-desktop --test tray_bus -- --ignored

use goymusic_desktop::tray::status_notifier_host_present;
use mpris_server::zbus::Connection;

#[test]
#[ignore = "needs a session bus: run under dbus-run-session"]
fn the_tray_counts_only_when_a_status_notifier_host_is_on_the_bus() {
    assert!(!status_notifier_host_present(), "a fresh bus has no tray host");
    let _host = futures_lite::future::block_on(async {
        let conn = Connection::session().await.unwrap();
        conn.request_name("org.kde.StatusNotifierWatcher").await.unwrap();
        conn
    });
    assert!(status_notifier_host_present());
}
