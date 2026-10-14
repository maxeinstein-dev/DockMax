mod commands;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            windows::dock::setup(app.handle())?;
            windows::panel::setup(app.handle())
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel::toggle_panel,
            commands::panel::hide_panel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
