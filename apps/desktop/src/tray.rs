//! System tray (spec §8): playback controls, the window, the mini player and «Salir». With
//! a tray, closing the window keeps the music playing.

use std::sync::mpsc::Receiver;

use goymusic_core::events::{Event, PlaybackState};
use goymusic_services::PlayerView;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Wry};

use crate::control::SharedControl;
use crate::window;

/// One tray menu entry, as the player's state dictates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuEntry {
    pub id: &'static str,
    pub label: String,
    pub enabled: bool,
}

pub fn menu_entries(view: Option<&PlayerView>) -> Vec<MenuEntry> {
    let has_track = view.is_some_and(|v| v.track.is_some());
    let playing = view.is_some_and(|v| v.state == PlaybackState::Playing);
    let entry = |id, label: &str, enabled| MenuEntry {
        id,
        label: label.into(),
        enabled,
    };
    vec![
        entry("play_pause", if playing { "Pausa" } else { "Reproducir" }, has_track),
        entry("previous", "Anterior", has_track),
        entry("next", "Siguiente", has_track),
        entry("show", "Mostrar GoyMusic", true),
        entry("mini", "Mini reproductor", true),
        entry("quit", "Salir", true),
    ]
}

/// The D-Bus name a StatusNotifier host (the panel that draws tray icons) owns.
const STATUS_NOTIFIER_WATCHER: &str = "org.kde.StatusNotifierWatcher";

/// Whether anything on the session bus will actually draw the tray icon. Building the icon
/// succeeds even when nobody shows it (GNOME without the AppIndicator extension), so the
/// window must not hide into a tray that isn't there.
pub fn status_notifier_host_present() -> bool {
    futures_lite::future::block_on(async {
        let conn = mpris_server::zbus::Connection::session().await.ok()?;
        let dbus = mpris_server::zbus::fdo::DBusProxy::new(&conn).await.ok()?;
        let name = STATUS_NOTIFIER_WATCHER.try_into().ok()?;
        dbus.name_has_owner(name).await.ok()
    })
    .unwrap_or(false)
}

/// Builds the tray icon and keeps its menu in step with the player. `Err` when the desktop
/// has no tray (e.g. GNOME without the AppIndicator extension): the caller then treats
/// closing the window as quitting.
pub fn create(app: &AppHandle, control: SharedControl, events: Receiver<Event>) -> tauri::Result<()> {
    let entries = menu_entries(control.view().as_ref());
    let items = entries
        .iter()
        .map(|e| MenuItem::with_id(app, e.id, &e.label, e.enabled, None::<&str>))
        .collect::<tauri::Result<Vec<MenuItem<Wry>>>>()?;
    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = items.iter().map(|i| i as _).collect();
    let menu = Menu::with_items(app, &refs)?;
    let c = control.clone();
    TrayIconBuilder::with_id("goymusic")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or(tauri::Error::InvalidIcon(std::io::Error::other("no app icon")))?,
        )
        .tooltip("GoyMusic")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "play_pause" => c.play_pause(),
            "previous" => c.previous(),
            "next" => c.next(),
            "show" => window::show_main(app),
            "mini" => window::show_mini(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    std::thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                if matches!(event, Event::PlayerStateChanged { .. } | Event::QueueChanged) {
                    for (item, entry) in items.iter().zip(menu_entries(control.view().as_ref())) {
                        let _ = item.set_text(&entry.label);
                        let _ = item.set_enabled(entry.enabled);
                    }
                }
            }
        })
        .map_err(tauri::Error::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use goymusic_core::events::PlaybackState;
    use goymusic_core::model::Track;
    use goymusic_core::TrackId;

    use super::*;
    use crate::control::fake::empty_view;

    fn with_track(state: PlaybackState) -> goymusic_services::PlayerView {
        let mut v = empty_view();
        v.state = state;
        v.track = Some(Track {
            id: TrackId(1),
            path: "/m/1.flac".into(),
            title: "Faro".into(),
            artists: vec![],
            album: None,
            album_id: None,
            album_artist: None,
            track_no: None,
            disc_no: None,
            year: None,
            genre: None,
            duration_ms: 1,
        });
        v
    }

    fn label(items: &[MenuEntry], id: &str) -> (String, bool) {
        let e = items.iter().find(|e| e.id == id).unwrap();
        (e.label.clone(), e.enabled)
    }

    #[test]
    fn menu_labels_follow_the_player() {
        let playing = menu_entries(Some(&with_track(PlaybackState::Playing)));
        assert_eq!(label(&playing, "play_pause"), ("Pausa".into(), true));
        let paused = menu_entries(Some(&with_track(PlaybackState::Paused)));
        assert_eq!(label(&paused, "play_pause"), ("Reproducir".into(), true));
        let empty = menu_entries(Some(&empty_view()));
        for id in ["play_pause", "previous", "next"] {
            assert!(!label(&empty, id).1, "{id} must be disabled with nothing loaded");
        }
        for id in ["show", "mini", "quit"] {
            assert!(label(&empty, id).1);
        }
        assert_eq!(
            playing.iter().map(|e| e.id).collect::<Vec<_>>(),
            ["play_pause", "previous", "next", "show", "mini", "quit"]
        );
        assert_eq!(label(&playing, "quit").0, "Salir");
    }

    #[test]
    fn closing_hides_only_when_the_tray_exists() {
        assert_eq!(crate::window::on_close(true), crate::window::CloseAction::Hide);
        assert_eq!(crate::window::on_close(false), crate::window::CloseAction::Quit);
    }
}
