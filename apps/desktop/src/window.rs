//! The main window as seen by the desktop integrations (MPRIS, tray, second instance).

use tauri::{AppHandle, Manager};

/// Shows, unminimizes and focuses the main window.
pub fn show_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Whether the main window is in front of the user (false if it cannot be told).
pub fn main_is_focused(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false)
}

/// Shows the mini player window (created hidden at startup).
pub fn show_mini(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("mini") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// What closing the main window does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAction {
    /// Hide it and keep playing; the tray brings it back.
    Hide,
    /// Quit: without a tray, a hidden window could never be reached again.
    Quit,
}

pub fn on_close(tray_available: bool) -> CloseAction {
    if tray_available {
        CloseAction::Hide
    } else {
        CloseAction::Quit
    }
}
