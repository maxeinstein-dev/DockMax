use tauri::AppHandle;

use crate::windows::hit_test;
use crate::windows::panel::LogicalRect;

/// Called by the dock when its visible bar is laid out. `rect` is relative to
/// the dock's client area, in logical pixels.
#[tauri::command]
pub fn set_dock_hit_rect(app: AppHandle, rect: LogicalRect) -> Result<(), String> {
    rect.validate()
        .map_err(|error| format!("invalid dock rect: {error}"))?;
    hit_test::tracker(&app).set_bar(rect);
    Ok(())
}
