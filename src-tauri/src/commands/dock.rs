use tauri::AppHandle;

use crate::windows::hit_test::{self, HitRegion};

/// Called by the dock when its bar is laid out and whenever its magnified
/// icons change. The region is relative to the dock's client area, in
/// logical pixels; the mouse is captured only there.
#[tauri::command]
pub fn set_dock_hit_region(app: AppHandle, region: HitRegion) -> Result<(), String> {
    // Before touching the tracker: a rejected region changes nothing.
    region
        .validate()
        .map_err(|error| format!("invalid dock region: {error}"))?;
    hit_test::tracker(&app).set_region(region);
    Ok(())
}
