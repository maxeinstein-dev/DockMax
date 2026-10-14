//! Lets clicks pass through the dock's transparent headroom.
//!
//! The dock window is taller than the visible bar so a hovered icon can grow
//! past it. The window ignores the mouse by default; a background thread
//! follows the cursor and captures the mouse only when the cursor enters the
//! visible bar, keeping it while the cursor stays inside the window (so the
//! magnified icon in the headroom keeps its hover).

use std::error::Error;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow};

use super::dock::Rect;
use super::panel::{LogicalRect, to_screen};

/// Cursor sampling interval (~60 Hz) while the dock is visible.
const INTERVAL: Duration = Duration::from_millis(16);

/// Whether the dock captures the mouse after the cursor moves to `cursor`
/// (physical screen pixels). A released dock captures only when the cursor
/// enters the visible `bar`; a captured dock stays captured while the cursor
/// is inside the whole `window`.
pub fn next_captured(captured: bool, cursor: (i32, i32), window: Rect, bar: Option<Rect>) -> bool {
    if captured {
        window.contains(cursor)
    } else {
        bar.is_some_and(|bar| bar.contains(cursor))
    }
}

#[derive(Debug, Default)]
struct Shared {
    /// The dock's client area in physical screen pixels, and its scale.
    window: Option<(Rect, f64)>,
    /// The visible bar, relative to the dock's client area, in logical px.
    bar: Option<LogicalRect>,
    visible: bool,
    stopped: bool,
}

impl Shared {
    fn bar_on_screen(&self) -> Option<Rect> {
        let (window, scale_factor) = self.window?;
        to_screen(self.bar?, (window.x, window.y), scale_factor)
    }
}

/// Dock geometry shared with the cursor thread.
#[derive(Debug, Default)]
pub struct Tracker {
    shared: Mutex<Shared>,
    wake: Condvar,
}

impl Tracker {
    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The dock window was placed: `window` is its client area.
    pub fn set_window(&self, window: Rect, scale_factor: f64) {
        self.lock().window = Some((window, scale_factor));
    }

    /// The dock reported where its visible bar is.
    pub fn set_bar(&self, bar: LogicalRect) {
        self.lock().bar = Some(bar);
    }

    /// Following the cursor runs only while the dock is visible.
    pub fn set_visible(&self, visible: bool) {
        self.lock().visible = visible;
        self.wake.notify_all();
    }

    /// Ends the cursor thread (the app is exiting).
    pub fn stop(&self) {
        self.lock().stopped = true;
        self.wake.notify_all();
    }

    /// The visible bar in physical screen pixels, once reported.
    pub fn bar_on_screen(&self) -> Option<Rect> {
        self.lock().bar_on_screen()
    }
}

/// Makes the dock ignore the mouse and starts following the cursor. The
/// tracker itself is managed by the builder, so it exists before the dock's
/// webview can report its bar.
pub fn start(app: &AppHandle, window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    let tracker = tracker(app);
    window.set_ignore_cursor_events(true)?;
    let window = window.clone();
    thread::Builder::new()
        .name("dock-hit-test".into())
        .spawn(move || follow_cursor(&tracker, &window))?;
    Ok(())
}

pub fn tracker(app: &AppHandle) -> Arc<Tracker> {
    Arc::clone(app.state::<Arc<Tracker>>().inner())
}

fn follow_cursor(tracker: &Tracker, window: &WebviewWindow) {
    let mut captured = false;
    loop {
        let shared = tracker.lock();
        if !shared.visible && captured {
            drop(shared);
            set_captured(window, false);
            captured = false;
            continue;
        }
        let shared = tracker
            .wake
            .wait_while(shared, |shared| !shared.visible && !shared.stopped)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if shared.stopped {
            return;
        }
        let geometry = shared
            .window
            .map(|(rect, _)| (rect, shared.bar_on_screen()));
        drop(shared);

        if let (Some((rect, bar)), Some(cursor)) = (geometry, cursor_position()) {
            let next = next_captured(captured, cursor, rect, bar);
            if next != captured {
                set_captured(window, next);
                captured = next;
            }
        }
        thread::sleep(INTERVAL);
    }
}

/// Tauri forwards the change to the main thread.
fn set_captured(window: &WebviewWindow, captured: bool) {
    if let Err(error) = window.set_ignore_cursor_events(!captured) {
        eprintln!("failed to change the dock's mouse capture: {error}");
    }
}

/// Read straight from Win32: Tauri's `cursor_position()` waits for a round
/// trip through the main thread's event loop, which would wake it ~60 times
/// a second.
#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    use ::windows::Win32::Foundation::POINT;
    use ::windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    // Fails while the secure desktop (lock screen, UAC) is shown.
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some((point.x, point.y))
}

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    // Bottom dock at 100%: a 240x104 window whose bottom 64 px are the bar
    // area; the visible bar is 200x64, centered.
    const WINDOW: Rect = Rect {
        x: 840,
        y: 920,
        width: 240,
        height: 104,
    };
    const BAR: Rect = Rect {
        x: 860,
        y: 960,
        width: 200,
        height: 64,
    };

    #[test]
    fn released_dock_captures_when_the_cursor_enters_the_bar() {
        assert!(next_captured(false, (900, 1000), WINDOW, Some(BAR)));
        assert!(next_captured(false, (860, 960), WINDOW, Some(BAR)));
    }

    #[test]
    fn captured_dock_stays_captured_in_the_headroom() {
        assert!(next_captured(true, (900, 930), WINDOW, Some(BAR)));
        // Beside the bar, still inside the window.
        assert!(next_captured(true, (845, 1000), WINDOW, Some(BAR)));
    }

    #[test]
    fn captured_dock_releases_when_the_cursor_leaves_the_window() {
        assert!(!next_captured(true, (900, 919), WINDOW, Some(BAR)));
        assert!(!next_captured(true, (900, 1024), WINDOW, Some(BAR)));
        assert!(!next_captured(true, (839, 1000), WINDOW, Some(BAR)));
        assert!(!next_captured(true, (1080, 1000), WINDOW, Some(BAR)));
    }

    #[test]
    fn released_dock_ignores_the_cursor_in_the_headroom() {
        assert!(!next_captured(false, (900, 930), WINDOW, Some(BAR)));
        assert!(!next_captured(false, (900, 959), WINDOW, Some(BAR)));
        // Beside the bar.
        assert!(!next_captured(false, (845, 1000), WINDOW, Some(BAR)));
        assert!(!next_captured(false, (1060, 1000), WINDOW, Some(BAR)));
    }

    #[test]
    fn released_dock_without_a_reported_bar_never_captures() {
        assert!(!next_captured(false, (900, 1000), WINDOW, None));
    }

    #[test]
    fn full_pass_enters_the_bar_rises_into_the_headroom_and_leaves() {
        let path = [
            ((900, 900), false), // above the window
            ((900, 930), false), // headroom while released
            ((900, 1000), true), // enters the bar
            ((900, 930), true),  // rises to the magnified icon
            ((900, 915), false), // leaves through the top
            ((900, 930), false), // back into the headroom: still released
        ];
        let mut captured = false;
        for (cursor, expected) in path {
            captured = next_captured(captured, cursor, WINDOW, Some(BAR));
            assert_eq!(captured, expected, "at {cursor:?}");
        }
    }

    #[test]
    fn top_dock_keeps_the_headroom_below_the_bar() {
        // Window 8 px below the work area top, bar on its top 64 px.
        let window = rect(840, 8, 240, 104);
        let bar = rect(860, 8, 200, 64);

        assert!(next_captured(false, (900, 40), window, Some(bar)));
        assert!(!next_captured(false, (900, 90), window, Some(bar)));
        assert!(next_captured(true, (900, 90), window, Some(bar)));
        assert!(!next_captured(true, (900, 112), window, Some(bar)));
    }

    #[test]
    fn scale_125_converts_the_reported_bar_to_screen_pixels() {
        let window = rect(1130, 1240, 300, 130);
        let shared = Shared {
            window: Some((window, 1.25)),
            bar: Some(LogicalRect {
                x: 20.0,
                y: 40.0,
                width: 200.0,
                height: 64.0,
            }),
            ..Shared::default()
        };

        let bar = shared.bar_on_screen();

        assert_eq!(bar, Some(rect(1155, 1290, 250, 80)));
        assert!(!next_captured(false, (1200, 1289), window, bar));
        assert!(next_captured(false, (1200, 1290), window, bar));
        assert!(next_captured(true, (1200, 1245), window, bar));
        assert!(!next_captured(true, (1200, 1370), window, bar));
    }
}
