use std::error::Error;

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter};

use crate::i18n;
use crate::windows::dock;
use crate::windows::panel::{self, PanelId, PanelPayload};

/// The product name: the same in every locale.
const TOOLTIP: &str = "DockMax";

const TOGGLE_DOCK_ID: &str = "toggle-dock";
const OPEN_SETTINGS_ID: &str = "open-settings";
const QUIT_ID: &str = "quit";

/// Adds the DockMax icon to the notification area. Its menu opens on
/// right-click.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let text = &i18n::messages().tray;
    let toggle = MenuItem::with_id(app, TOGGLE_DOCK_ID, text.toggle_dock, true, None::<&str>)?;
    let settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_ID,
        text.open_settings,
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, text.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
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

/// Whether the dock must be asked to open the Settings panel, given the
/// panel open now: not when it is already open (asking would toggle it
/// closed).
fn must_request_settings(open: Option<PanelId>) -> bool {
    open != Some(PanelId::Settings)
}

/// Shows the dock if it is hidden (without taking the foreground) and asks
/// it to open the Settings panel, anchored to its own item. Opening the
/// tray menu may have blurred an open panel; this is an explicit request,
/// so the reopen guard of that blur does not apply.
fn open_settings(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    dock::reveal(app)?;
    let request = {
        let mut state = panel::lock(app);
        state.forget_blur();
        must_request_settings(state.open())
    };
    if request {
        app.emit_to(
            dock::LABEL,
            dock::REQUEST_PANEL_EVENT,
            PanelPayload {
                panel_id: PanelId::Settings,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_requested_unless_already_open() {
        assert!(must_request_settings(None));
        assert!(must_request_settings(Some(PanelId::About)));
        assert!(!must_request_settings(Some(PanelId::Settings)));
    }

    #[test]
    fn request_event_names_the_settings_panel() {
        assert_eq!(dock::REQUEST_PANEL_EVENT, "dock-request-panel");
        assert_eq!(
            serde_json::to_value(PanelPayload {
                panel_id: PanelId::Settings
            })
            .unwrap(),
            serde_json::json!({ "panelId": "settings" })
        );
    }
}
