// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The taskbar watchdog and the manual restore are this executable too,
    // and start nothing of the app.
    if dockyard_lib::run_special_mode() {
        return;
    }
    dockyard_lib::run()
}
