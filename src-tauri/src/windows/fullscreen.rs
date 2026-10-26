//! Each dock steps aside for a program in full screen on its own monitor
//! (spec 0010 D4, D4b: one dock per monitor), and comes back when it leaves.
//! Asked by the thread that follows the windows (`open_apps`) after each
//! burst of window events.

use tauri::AppHandle;

use crate::domain::fullscreen::{Bounds, is_fullscreen};

/// Checks each dock's monitor and hides or shows that dock to match. Using
/// the other monitor changes nothing here: each monitor is looked at alone.
pub fn check(_app: &AppHandle) {
    let candidates = super::openwin::fullscreen_candidates();
    for (key, handle, rect) in super::native_dock::monitors() {
        let bounds = Bounds {
            left: rect.x,
            top: rect.y,
            right: rect.x + rect.width as i32,
            bottom: rect.y + rect.height as i32,
        };
        super::dock::set_fullscreen(&key, is_fullscreen(&candidates, handle, bounds));
    }
}
