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
