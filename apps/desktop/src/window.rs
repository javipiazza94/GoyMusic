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
