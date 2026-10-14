mod commands;
mod tray;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            windows::panel::setup(app.handle())?;
            windows::dock::setup(app.handle())?;
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel::toggle_panel,
            commands::panel::hide_panel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
