use std::time::Instant;

use tauri::{AppHandle, WebviewWindow};

use crate::windows::dock;
use crate::windows::panel::{
    self, Action, LogicalRect, PanelGeometry, PanelId, PanelInset, PanelSize,
};

/// Called by the dock when an item is clicked. `anchor` is the item's
/// rectangle relative to the dock's client area, in logical pixels. Opening
/// asks the panel page to render first (`panel-show`); the window shows on
/// `panel_ready` or after `panel::READY_TIMEOUT`.
#[tauri::command]
pub fn toggle_panel(app: AppHandle, panel_id: PanelId, anchor: LogicalRect) -> Result<(), String> {
    // `anchor` was checked when deserialized: an invalid one never gets
    // here, so a rejected click changes nothing.
    let action = panel::lock(&app).toggle(panel_id, anchor, Instant::now());
    let show = matches!(action, Action::Prepare(..));
    if show && !dock::is_visible(&app) {
        // A click queued before the dock was hidden from the tray: a panel
        // anchored to a hidden dock stays closed.
        panel::close(&app);
        return Ok(());
    }
    if let Err(error) = panel::apply(&app, action) {
        log::warn!("panel: failed to toggle: {error}");
        if show {
            panel::close(&app);
        }
        return Err("could not open the panel".into());
    }
    Ok(())
}

/// Called by the panel to close itself (Esc).
#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<(), String> {
    let action = panel::lock(&app).hide();
    panel::apply(&app, action).map_err(|error| {
        log::warn!("panel: failed to hide: {error}");
        "could not close the panel".to_string()
    })
}

/// Called by the panel once the content `panel-show` asked for has
/// rendered, with that show's `wait`, the window size it needs and the
/// transparent inset around its visible card, in logical pixels. Shows the
/// panel there; a signal for a panel that is no longer open, or for an
/// earlier show of it, is ignored. Called again for the open panel, places
/// it again.
#[tauri::command]
pub fn panel_ready(
    app: AppHandle,
    window: WebviewWindow,
    panel_id: PanelId,
    wait: u64,
    size: PanelSize,
    inset: PanelInset,
) -> Result<(), String> {
    if window.label() != panel::LABEL {
        return Err("only the panel reports that it rendered".into());
    }
    // `size` and `inset` were checked when deserialized.
    let action = panel::lock(&app).ready(panel_id, wait, PanelGeometry::new(size, inset));
    if action == Action::Nothing {
        log::debug!("panel: ignored a stale ready signal");
        return Ok(());
    }
    panel::reveal_or_close(&app, action).map_err(|error| {
        log::warn!("panel: failed to show: {error}");
        "could not open the panel".to_string()
    })
}
