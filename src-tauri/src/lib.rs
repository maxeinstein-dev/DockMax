use std::sync::{Arc, Mutex};

use tauri::{RunEvent, Wry, plugin::TauriPlugin};

mod commands;
mod domain;
mod i18n;
mod logging;
mod persistence;
mod tray;
mod windows;

/// Attaches the logger and loads the settings before any page can use them.
/// Tauri creates the windows of `tauri.conf.json` before the app's `setup`
/// runs, so a page could call `get_settings` (or log) before either existed;
/// plugins, on the other hand, are initialized in `Builder::build`, before
/// any window. Never fails.
fn startup() -> TauriPlugin<Wry> {
    tauri::plugin::Builder::new("dockmax-startup")
        .setup(|app, _api| {
            // First, so the settings can report a bad file.
            logging::setup(app);
            commands::settings::setup(app);
            Ok(())
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // First: a second launch exits here, before creating anything, and
        // shows the dock of the running instance. Its argv and working
        // folder are ignored and never logged.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Err(error) = windows::dock::reveal(app) {
                log::warn!("single instance: failed to show the dock: {error}");
            }
        }))
        .plugin(logging::plugin())
        .manage(Arc::new(windows::hit_test::Tracker::default()))
        .manage(Mutex::new(windows::dock::Placement::default()))
        .plugin(startup())
        .setup(|app| {
            windows::panel::setup(app.handle())?;
            windows::dock::setup(app.handle())?;
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel::toggle_panel,
            commands::panel::hide_panel,
            commands::panel::panel_ready,
            commands::dock::set_dock_layout,
            commands::settings::get_settings,
            commands::settings::update_settings
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                windows::hit_test::tracker(app).stop();
            }
        });
}
