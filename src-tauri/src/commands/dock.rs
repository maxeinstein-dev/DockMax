use tauri::{AppHandle, WebviewWindow};

use crate::windows::dock::{self, DockLayout};

/// Called by the dock whenever its layout changes (size, edge,
/// magnification, label). The layout is relative to the dock's client area,
/// in logical pixels: the window is resized to `layout.window` and placed
/// on its edge, and the mouse is captured only where the dock paints.
#[tauri::command]
pub fn set_dock_layout(
    app: AppHandle,
    window: WebviewWindow,
    layout: DockLayout,
) -> Result<(), String> {
    if window.label() != dock::LABEL {
        return Err("only the dock reports its layout".into());
    }
    // `layout` was checked when deserialized: an invalid one never gets
    // here, so a rejected layout changes nothing.
    dock::set_layout(&app, layout).map_err(|error| {
        log::warn!("dock: failed to apply the layout: {error}");
        "could not place the dock".to_string()
    })
}
