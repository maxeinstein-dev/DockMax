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

/// The jobs this executable has besides being the app, told by its first
/// argument: the watchdog that gives the taskbar back when the app is killed
/// and the manual restore (ADR 0009), and the launcher's icon reader. Returns whether it did one, in
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
            // The launcher's icon reader (spec 0012 L1): a helper process.
            [flag] if flag == windows::launcher::icons::FLAG => {
                windows::launcher::icons::serve();
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
            commands::widgets::share_widget,
            commands::workspaces::list_workspaces,
            commands::workspaces::create_workspace,
            commands::workspaces::rename_workspace,
            commands::workspaces::recolor_workspace,
            commands::workspaces::move_workspace,
            commands::workspaces::delete_workspace,
            commands::workspaces::move_app_windows,
            commands::workspaces::move_open_windows,
            commands::widgets::update_widget,
            commands::widgets::get_note,
            commands::widgets::save_note,
            commands::widgets::dismiss_storage_notice,
            commands::tray::list_tray,
            commands::tray::tray_click,
            commands::system::read_system_activity,
            commands::calendar::calendar_events,
            commands::calendar::open_calendar_link,
            commands::calendar::complete_calendar_task,
            commands::calendar::cycle_status,
            commands::calendar::cycle_test_notice,
            commands::pomodoro::pomodoro_status,
            commands::pomodoro::pomodoro_act,
            commands::google::google_status,
            commands::google::set_google_client,
            commands::google::connect_google_account,
            commands::google::cancel_google_sign_in,
            commands::google::google_calendars,
            commands::google::disconnect_google_account,
            commands::tray::tray_to_dock,
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
            commands::launcher::launcher_status,
            commands::launcher::clear_launcher_history,
            commands::launcher::set_launcher_hotkey,
            commands::launcher::capture_launcher_hotkey
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
            commands::widgets::share_widget,
            commands::workspaces::list_workspaces,
            commands::workspaces::create_workspace,
            commands::workspaces::rename_workspace,
            commands::workspaces::recolor_workspace,
            commands::workspaces::move_workspace,
            commands::workspaces::delete_workspace,
            commands::workspaces::move_app_windows,
            commands::workspaces::move_open_windows,
            commands::widgets::update_widget,
            commands::widgets::get_note,
            commands::widgets::save_note,
            commands::widgets::dismiss_storage_notice,
            commands::tray::list_tray,
            commands::tray::tray_click,
            commands::system::read_system_activity,
            commands::calendar::calendar_events,
            commands::calendar::open_calendar_link,
            commands::calendar::complete_calendar_task,
            commands::calendar::cycle_status,
            commands::calendar::cycle_test_notice,
            commands::pomodoro::pomodoro_status,
            commands::pomodoro::pomodoro_act,
            commands::google::google_status,
            commands::google::set_google_client,
            commands::google::connect_google_account,
            commands::google::cancel_google_sign_in,
            commands::google::google_calendars,
            commands::google::disconnect_google_account,
            commands::tray::tray_to_dock,
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
            commands::launcher::launcher_status,
            commands::launcher::clear_launcher_history,
            commands::launcher::set_launcher_hotkey,
            commands::launcher::capture_launcher_hotkey,
            e2e::e2e_dock_view,
            e2e::e2e_press_dock,
            e2e::e2e_launcher_view,
            e2e::e2e_launcher_toggle,
            e2e::e2e_launcher_type,
            e2e::e2e_launcher_key
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
            windows::window_spaces::release_all();
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
            // The name of the DockMax's Windows notices (spec 0017 F2b-2).
            #[cfg(windows)]
            windows::toast::set_identifier(&app.config().identifier);
            commands::settings::sync_launch_at_login(app.handle());
            windows::dock::setup(app.handle())?;
            #[cfg(windows)]
            windows::open_apps::start(app.handle());
            #[cfg(windows)]
            windows::badges::start(app.handle());
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
            // The dock is not a Tauri window (spec 0011 H3): when the panel or
            // the Configurações app, made on demand, closes and no Tauri window
            // is left, Tauri asks to exit, with no code. The app stays; Sair
            // in the tray asks with a code and is let through. The test build
            // keeps Tauri's way: the suite quits it by closing its windows.
            #[cfg(not(feature = "e2e"))]
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = &event
            {
                api.prevent_exit();
                return;
            }
            if let RunEvent::Exit = event {
                // The edge and the taskbar go back before anything else.
                #[cfg(windows)]
                {
                    windows::tray_host::stop();
                    windows::appbar::remove();
                    windows::taskbar::release();
                    // The windows the workspaces hid (spec 0013 C2).
                    windows::window_spaces::release_all();
                    // Direct2D is let go while DXGI still answers: at the
                    // end of the process, it crashed every exit.
                    windows::launcher::shutdown();
                    windows::native_dock::shutdown();
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
