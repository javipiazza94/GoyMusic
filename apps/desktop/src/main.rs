// Release builds on Windows would otherwise open a console window (Linux is unaffected).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    goymusic_desktop::run();
}
