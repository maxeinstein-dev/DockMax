use std::sync::{Mutex, PoisonError};

use tauri::{
    Builder, Manager, RunEvent, Runtime, Wry, plugin::TauriPlugin, webview::PageLoadEvent,
};

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

/// Registers the state the windows and the commands read (`state()`,
/// which aborts the startup when nothing was managed yet: "state() called
/// before manage()"). On the builder before any `setup`, the plugins' and
/// the app's: `windows::panel::setup` reads the panel's state there.
///
/// Public (and hidden) only so `tests/managed_state.rs` can build a mock app
/// with it.
#[doc(hidden)]
pub fn with_state<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.manage(Mutex::new(windows::panel::PanelState::default()))
}

/// The states `with_state` manages that `app` has, by type name.
#[doc(hidden)]
pub fn managed_state<R: Runtime>(app: &impl Manager<R>) -> Vec<&'static str> {
    let mut found = Vec::new();
    if app
        .try_state::<Mutex<windows::panel::PanelState>>()
        .is_some()
    {
        found.push("Mutex<PanelState>");
    }
    found
}

/// The two jobs this executable has besides being the app (ADR 0009), told
/// by its first argument: the watchdog that gives the taskbar back when the
/// app is killed, and the manual restore. Returns whether it did one, in
/// which case `main` has nothing left to do.
pub fn run_special_mode() -> bool {
    #[cfg(windows)]
    {
        use std::path::Path;
        use windows::taskbar::{RESTORE_FLAG, WATCHDOG_FLAG};
        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.as_slice() {
            [flag, pid, path] if flag == WATCHDOG_FLAG => {
                if let Ok(pid) = pid.parse() {
                    windows::taskbar::watch(pid, Path::new(path));
                }
                return true;
            }
            [flag, path] if flag == RESTORE_FLAG => {
                windows::taskbar::restore_now(Path::new(path));
                return true;
            }
            _ => {}
        }
    }
    false
}

/// The app's commands, and the ones the E2E suite drives the dock with in
/// the test build (`e2e.rs`): the dock is native, with no page to look into.
#[cfg(not(feature = "e2e"))]
macro_rules! handler {
    () => {
        tauri::generate_handler![
            commands::panel::hide_panel,
            commands::panel::panel_ready,
            commands::panel::panel_mounted,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::widgets::list_widgets,
            commands::widgets::add_widget,
            commands::widgets::remove_widget,
            commands::widgets::move_widget,
            commands::widgets::update_widget,
            commands::widgets::get_note,
            commands::widgets::save_note,
            commands::widgets::dismiss_storage_notice,
            commands::tray::list_tray,
            commands::tray::tray_click,
            commands::apps::list_apps,
            commands::apps::list_catalog,
            commands::apps::app_icon,
            commands::apps::pin_app,
            commands::apps::unpin_app,
            commands::apps::move_app,
            commands::apps::place_app,
            commands::apps::launch_app,
            commands::apps::list_open_apps,
            commands::apps::activate_app,
            commands::apps::activate_open,
            commands::apps::launch_open,
            commands::apps::open_badge,
            commands::apps::close_app,
            commands::apps::close_open,
            commands::apps::pin_open,
            commands::apps::app_menu,
            commands::apps::run_app_entry,
            commands::apps::reveal_app,
            commands::icons::list_icon_packs,
            commands::icons::reload_icon_packs,
            commands::icons::open_icon_packs_folder
        ]
    };
}

#[cfg(feature = "e2e")]
macro_rules! handler {
    () => {
        tauri::generate_handler![
            commands::panel::hide_panel,
            commands::panel::panel_ready,
            commands::panel::panel_mounted,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::widgets::list_widgets,
            commands::widgets::add_widget,
            commands::widgets::remove_widget,
            commands::widgets::move_widget,
            commands::widgets::update_widget,
            commands::widgets::get_note,
            commands::widgets::save_note,
            commands::widgets::dismiss_storage_notice,
            commands::tray::list_tray,
            commands::tray::tray_click,
            commands::apps::list_apps,
            commands::apps::list_catalog,
            commands::apps::app_icon,
            commands::apps::pin_app,
            commands::apps::unpin_app,
            commands::apps::move_app,
            commands::apps::place_app,
            commands::apps::launch_app,
            commands::apps::list_open_apps,
            commands::apps::activate_app,
            commands::apps::activate_open,
            commands::apps::launch_open,
            commands::apps::open_badge,
            commands::apps::close_app,
            commands::apps::close_open,
            commands::apps::pin_open,
            commands::apps::app_menu,
            commands::apps::run_app_entry,
            commands::apps::reveal_app,
            commands::icons::list_icon_packs,
            commands::icons::reload_icon_packs,
            commands::icons::open_icon_packs_folder,
            e2e::e2e_dock_view,
            e2e::e2e_press_dock
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A panic gives the taskbar back before the process aborts (release
    // builds abort on panic, so nothing is dropped).
    #[cfg(windows)]
    {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            windows::taskbar::release();
            previous(info);
        }));
    }
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
        .plugin(logging::plugin());
    let builder = with_state(builder)
        .plugin(startup())
        // Each window is at rest once its page has settled (spec 0003 §2.12).
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                windows::memory::page_loaded(webview.app_handle(), webview.label());
                #[cfg(windows)]
                if let Some(window) = webview.app_handle().get_webview_window(webview.label()) {
                    windows::recovery::watch(&window);
                }
            }
        })
        .setup(|app| {
            commands::settings::sync_launch_at_login(app.handle());
            windows::dock::setup(app.handle())?;
            #[cfg(windows)]
            windows::open_apps::start(app.handle());
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(handler!());
    // Test build only: the WebDriver server the E2E suite drives the windows
    // through, on the port in `TAURI_WEBDRIVER_PORT` (4445 by default).
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // The edge and the taskbar go back before anything else.
                #[cfg(windows)]
                {
                    windows::tray_host::stop();
                    windows::appbar::remove();
                    windows::taskbar::release();
                }
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
