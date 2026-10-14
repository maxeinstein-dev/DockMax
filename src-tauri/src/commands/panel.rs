use std::time::Instant;

use tauri::AppHandle;

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
    if let Err(error) = panel::apply(&app, action, Some(anchor)) {
        eprintln!("failed to toggle the panel: {error}");
        if matches!(action, Action::Show(_)) {
            let reset = panel::lock(&app).hide();
            let _ = panel::apply(&app, reset, None);
        }
        return Err("could not open the panel".into());
    }
    Ok(())
}

/// Called by the panel to close itself (Esc).
#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<(), String> {
    let action = panel::lock(&app).hide();
    panel::apply(&app, action, None).map_err(|error| {
        eprintln!("failed to hide the panel: {error}");
        "could not close the panel".to_string()
    })
}
