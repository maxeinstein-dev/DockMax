use tauri::AppHandle;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::TrayIconBuilder;

use crate::windows::dock;

// PT-BR until `i18n.rs` arrives.
const TOOLTIP: &str = "DockMax";
const TOGGLE_DOCK_TEXT: &str = "Mostrar/ocultar dock";
const QUIT_TEXT: &str = "Sair";

const TOGGLE_DOCK_ID: &str = "toggle-dock";
const QUIT_ID: &str = "quit";

/// Adds the DockMax icon to the notification area. Its menu opens on
/// right-click.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TOGGLE_DOCK_ID, TOGGLE_DOCK_TEXT, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, QUIT_TEXT, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
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
        TOGGLE_DOCK_ID => {
            if let Err(error) = dock::toggle(app) {
                eprintln!("failed to toggle the dock: {error}");
            }
        }
        QUIT_ID => app.exit(0),
        _ => {}
    }
}
