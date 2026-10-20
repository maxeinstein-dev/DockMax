use std::sync::{Arc, Mutex, PoisonError};

use tauri::{Manager, RunEvent, Wry, plugin::TauriPlugin, webview::PageLoadEvent};

mod commands;
mod domain;
#[cfg(feature = "e2e")]
mod e2e;
mod i18n;
mod logging;
mod persistence;
mod tray;
mod windows;

/// Attaches the logger, loads the settings and opens the widget database
/// before any page can use them.
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
            // After the settings: the dock and Settings work even when the
            // widget database can't be used.
            commands::widgets::setup(app);
            Ok(())
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg_attr(not(feature = "e2e"), expect(unused_mut))]
    let mut context = tauri::generate_context!();
    // Test build only: before anything reads the identifier or a folder.
    #[cfg(feature = "e2e")]
    e2e::isolate(&mut context);

    let builder = tauri::Builder::default()
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
        .manage(Mutex::new(windows::panel::PanelState::default()))
        .plugin(startup())
        // Each window is at rest once its page has settled (spec 0003 §2.12).
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                windows::memory::page_loaded(webview.app_handle(), webview.label());
            }
        })
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
            commands::settings::update_settings,
            commands::widgets::list_widgets,
            commands::widgets::add_widget,
            commands::widgets::remove_widget,
            commands::widgets::move_widget,
            commands::widgets::update_widget,
            commands::widgets::get_note,
            commands::widgets::save_note,
            commands::widgets::dismiss_storage_notice
        ]);
    // Test build only: the WebDriver server the E2E suite drives the windows
    // through, on the port in `TAURI_WEBDRIVER_PORT` (4445 by default).
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                windows::hit_test::tracker(app).stop();
                // The process ends without closing the widget database:
                // leave the file complete on its own, `-wal` applied.
                if let Some(storage) = app.try_state::<Mutex<persistence::database::Storage>>() {
                    storage
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .checkpoint();
                }
            }
        });
}
