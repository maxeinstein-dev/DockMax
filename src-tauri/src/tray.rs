use std::error::Error;

use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::i18n;
use crate::windows::dock;

/// The product name: the same in every locale.
const TOOLTIP: &str = "DockMax";

const LAUNCHER_ID: &str = "launcher";
const TASK_MANAGER_ID: &str = "task-manager";
const AUTO_HIDE_ID: &str = "auto-hide";
const OPEN_SETTINGS_ID: &str = "open-settings";
const QUIT_ID: &str = "quit";

/// "Ocultar automaticamente", kept to follow the setting.
struct AutoHideItem(CheckMenuItem<Wry>);

/// Adds the DockMax icon to the notification area. Its menu opens on
/// right-click, with what the dock's own menu has.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let text = &i18n::messages().tray;
    let item = |id: &str, label: &str| MenuItem::with_id(app, id, label, true, None::<&str>);
    let launcher = item(LAUNCHER_ID, text.search_apps)?;
    let task_manager = item(TASK_MANAGER_ID, text.task_manager)?;
    let settings = item(OPEN_SETTINGS_ID, text.open_settings)?;
    let auto_hide = CheckMenuItem::with_id(
        app,
        AUTO_HIDE_ID,
        text.auto_hide,
        true,
        crate::commands::settings::current(app).dock.auto_hide,
        None::<&str>,
    )?;
    let quit = item(QUIT_ID, text.quit)?;
    let menu = Menu::with_items(
        app,
        &[
            &launcher,
            &task_manager,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &auto_hide,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    app.manage(AutoHideItem(auto_hide));
    let mut tray = TrayIconBuilder::new()
        .tooltip(TOOLTIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The setting changed (here, in the dock's menu or in Configurações): the
/// check follows it.
pub fn show_auto_hide(app: &AppHandle, on: bool) {
    if let Some(item) = app.try_state::<AutoHideItem>()
        && let Err(error) = item.0.set_checked(on)
    {
        log::warn!("tray: the auto-hide check could not follow: {error}");
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        #[cfg(windows)]
        LAUNCHER_ID => crate::windows::launcher::toggle(),
        TASK_MANAGER_ID => {
            if let Err(error) = crate::windows::apps::open_task_manager() {
                log::warn!("tray: the Task Manager could not be opened: {error}");
            }
        }
        AUTO_HIDE_ID => {
            let changed = crate::commands::settings::change(app, |settings| {
                settings.dock.auto_hide = !settings.dock.auto_hide;
            });
            if let Err(error) = changed {
                log::warn!("tray: hiding on its own could not be changed: {error}");
            }
        }
        OPEN_SETTINGS_ID => {
            if let Err(error) = open_settings(app) {
                log::warn!("tray: failed to open the settings: {error}");
            }
        }
        QUIT_ID => app.exit(0),
        _ => {}
    }
}

/// Opens the Configurações app (spec 0011 H1), showing the dock too if it
/// is hidden, without taking the foreground for it.
fn open_settings(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    dock::reveal(app)?;
    crate::windows::settings_window::open(app)
}
