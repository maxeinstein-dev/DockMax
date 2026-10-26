//! The dock steps aside for a program in full screen on its monitor (spec
//! 0010 D4), and comes back when it leaves. Asked by the thread that follows
//! the windows (`open_apps`) after each burst of window events.

use tauri::AppHandle;

use crate::domain::fullscreen::is_fullscreen;

/// Checks the dock's monitor and hides or shows the dock to match.
pub fn check(app: &AppHandle) {
    let Some((monitor, bounds)) = super::openwin::primary_monitor() else {
        return;
    };
    let full = is_fullscreen(&super::openwin::fullscreen_candidates(), monitor, bounds);
    super::dock::set_fullscreen(app, full);
}
