use tauri::AppHandle;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

/// The product name: the same in every locale.
const TOOLTIP: &str = "Dockyard";

/// Adds the Dockyard icon to the notification area. Its menu, the Dockyard's
/// own (spec 0014 R3), opens on right-click with what the dock's own menu
/// has.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::new()
        .tooltip(TOOLTIP)
        .on_tray_icon_event(|icon, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                #[cfg(windows)]
                crate::windows::native_dock::tray_menu(icon.app_handle());
                #[cfg(not(windows))]
                let _ = icon;
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
