use std::sync::Arc;

use tauri::RunEvent;

mod commands;
mod domain;
mod logging;
mod persistence;
mod tray;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(logging::plugin())
        .manage(Arc::new(windows::hit_test::Tracker::default()))
        .setup(|app| {
            // First: the windows may ask for the settings as soon as they load.
            commands::settings::setup(app.handle());
            windows::panel::setup(app.handle())?;
            windows::dock::setup(app.handle())?;
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel::toggle_panel,
            commands::panel::hide_panel,
            commands::dock::set_dock_hit_region,
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
