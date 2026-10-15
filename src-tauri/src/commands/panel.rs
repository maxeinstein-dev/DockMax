use std::time::Instant;

use tauri::AppHandle;

use crate::windows::dock;
use crate::windows::panel::{self, Action, LogicalRect, PanelId};

/// Called by the dock when an item is clicked. `anchor` is the item's
/// rectangle relative to the dock's client area, in logical pixels.
#[tauri::command]
pub fn toggle_panel(app: AppHandle, panel_id: PanelId, anchor: LogicalRect) -> Result<(), String> {
    // Before touching the panel state: a rejected click changes nothing.
    anchor
        .validate()
        .map_err(|error| format!("invalid anchor: {error}"))?;
    let action = panel::lock(&app).toggle(panel_id, Instant::now());
    let show = matches!(action, Action::Show(_));
    if show && !dock::is_visible(&app) {
        // A click queued before the dock was hidden from the tray: a panel
        // anchored to a hidden dock stays closed.
        close(&app);
        return Ok(());
    }
    if let Err(error) = panel::apply(&app, action, Some(anchor)) {
        log::warn!("panel: failed to toggle: {error}");
        if show {
            close(&app);
        }
        return Err("could not open the panel".into());
    }
    Ok(())
}

/// Brings the state back to closed after a `Show` that won't happen.
fn close(app: &AppHandle) {
    let reset = panel::lock(app).hide();
    if let Err(error) = panel::apply(app, reset, None) {
        log::warn!("panel: failed to hide: {error}");
    }
}

/// Called by the panel to close itself (Esc).
#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<(), String> {
    let action = panel::lock(&app).hide();
    panel::apply(&app, action, None).map_err(|error| {
        log::warn!("panel: failed to hide: {error}");
        "could not close the panel".to_string()
    })
}
