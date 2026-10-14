use tauri::AppHandle;

use crate::windows::hit_test;
use crate::windows::panel::LogicalRect;

/// Called by the dock when its visible bar is laid out. `rect` is relative to
/// the dock's client area, in logical pixels.
#[tauri::command]
pub fn set_dock_hit_rect(app: AppHandle, rect: LogicalRect) -> Result<(), String> {
    let values = [rect.x, rect.y, rect.width, rect.height];
    if values.iter().any(|value| !value.is_finite()) || rect.width < 0.0 || rect.height < 0.0 {
        return Err("invalid dock rect".into());
    }
    hit_test::tracker(&app).set_bar(rect);
    Ok(())
}
