use std::error::Error;

use tauri::AppHandle;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

use crate::i18n;
use crate::windows::dock;

/// The product name: the same in every locale.
const TOOLTIP: &str = "DockMax";

const TASK_MANAGER_ID: &str = "task-manager";
const TOGGLE_DOCK_ID: &str = "toggle-dock";
const OPEN_SETTINGS_ID: &str = "open-settings";
const QUIT_ID: &str = "quit";

/// Adds the DockMax icon to the notification area. Its menu opens on
/// right-click, with what the dock's own menu has.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let text = &i18n::messages().tray;
    let item = |id: &str, label: &str| MenuItem::with_id(app, id, label, true, None::<&str>);
    let task_manager = item(TASK_MANAGER_ID, text.task_manager)?;
    let toggle = item(TOGGLE_DOCK_ID, text.toggle_dock)?;
    let settings = item(OPEN_SETTINGS_ID, text.open_settings)?;
    let quit = item(QUIT_ID, text.quit)?;
    let menu = Menu::with_items(
        app,
        &[
            &task_manager,
            &PredefinedMenuItem::separator(app)?,
            &toggle,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
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

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        TASK_MANAGER_ID => {
            if let Err(error) = crate::windows::apps::open_task_manager() {
                log::warn!("tray: the Task Manager could not be opened: {error}");
            }
        }
        TOGGLE_DOCK_ID => {
            if let Err(error) = dock::toggle(app) {
                log::warn!("tray: failed to toggle the dock: {error}");
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
