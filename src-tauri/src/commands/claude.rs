//! The Uso do Claude's panel (spec 0017 F5): the last reading of the
//! plan's limits. Only the panel calls this.

use tauri::WebviewWindow;

use crate::windows::panel;

/// The last reading (`null` before the first); `fresh`: read again now
/// (the panel just opened), the answer coming a moment later.
#[cfg(windows)]
#[tauri::command]
pub fn claude_usage(
    window: WebviewWindow,
    fresh: bool,
) -> Result<Option<crate::windows::claude_usage::Reading>, String> {
    if window.label() != panel::LABEL {
        return Err("only the panel reads the usage".into());
    }
    if fresh {
        crate::windows::claude_usage::read_now();
    }
    Ok(crate::windows::claude_usage::reading())
}
