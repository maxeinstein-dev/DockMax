use tauri::WebviewWindow;

use crate::windows::dock;
#[cfg(windows)]
use crate::windows::flyout::{self, Flyout};

/// Called by the dock when the clock is clicked: opens the Windows calendar
/// (ADR 0009). Only the dock may ask, and only for the panels `Flyout`
/// names: the page cannot press any other key.
#[cfg(windows)]
#[tauri::command]
pub fn open_flyout(window: WebviewWindow, flyout: Flyout) -> Result<(), String> {
    if window.label() != dock::LABEL {
        return Err("only the dock opens the Windows panels".into());
    }
    flyout::open(flyout).map_err(|error| {
        log::warn!("flyout: could not be opened: {error}");
        "could not open the Windows panel".to_string()
    })
}
