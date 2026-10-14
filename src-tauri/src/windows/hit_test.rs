//! Lets clicks pass through the dock's transparent headroom.
//!
//! The dock window is taller than the visible bar so a hovered icon can grow
//! past it. The window ignores the mouse by default; a background thread
//! follows the cursor and captures the mouse only where the dock paints:
//! it captures when the cursor enters the visible bar and keeps capturing
//! while the cursor stays on the bar or on a magnified icon.

use std::error::Error;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use super::dock::{self, Rect};
use super::panel::{LogicalRect, RectError, to_screen};

/// Cursor sampling interval (~60 Hz) while the dock is visible.
const INTERVAL: Duration = Duration::from_millis(16);

/// Event sent to the dock when it stops capturing the mouse: the page may
/// get no more pointer events, so it clears its hover state.
const RELEASED_EVENT: &str = "dock-released";

/// Most magnified icons the dock reports at once (the hovered icon and its
/// neighbours need 3).
pub const MAX_HOLD_RECTS: usize = 16;

/// Where the dock paints, relative to its client area, in logical pixels:
/// the visible bar and the final rects of the magnified icons.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HitRegion {
    pub bar: LogicalRect,
    pub hold: Vec<LogicalRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionError {
    Bar(RectError),
    Hold(usize, RectError),
    TooManyHoldRects,
}

impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegionError::Bar(error) => write!(f, "bar: {error}"),
            RegionError::Hold(index, error) => write!(f, "hold rect {index}: {error}"),
            RegionError::TooManyHoldRects => {
                write!(f, "more than {MAX_HOLD_RECTS} hold rects")
            }
        }
    }
}

impl Error for RegionError {}

impl HitRegion {
    /// Checks a region received from the frontend: every rect valid and at
    /// most `MAX_HOLD_RECTS` hold rects.
    pub fn validate(&self) -> Result<(), RegionError> {
        if self.hold.len() > MAX_HOLD_RECTS {
            return Err(RegionError::TooManyHoldRects);
        }
        self.bar.validate().map_err(RegionError::Bar)?;
        for (index, rect) in self.hold.iter().enumerate() {
            rect.validate()
                .map_err(|error| RegionError::Hold(index, error))?;
        }
        Ok(())
    }
}

/// Whether the dock captures the mouse after the cursor moves to `cursor`.
/// Everything is in physical screen pixels. A released dock captures only
/// when the cursor enters the visible `bar`; a captured dock stays captured
/// while the cursor is on the bar or on any `hold` rect (a magnified icon),
/// and releases everywhere else, the transparent rest of the window
/// included.
pub fn next_captured(captured: bool, cursor: (i32, i32), bar: Option<Rect>, hold: &[Rect]) -> bool {
    let on_bar = bar.is_some_and(|bar| bar.contains(cursor));
    on_bar || (captured && hold.iter().any(|rect| rect.contains(cursor)))
}

#[derive(Debug, Default)]
struct Shared {
    /// The dock's client area in physical screen pixels, and its scale.
    window: Option<(Rect, f64)>,
    /// Where the dock paints, as last reported by its page.
    region: Option<HitRegion>,
    visible: bool,
    stopped: bool,
}

impl Shared {
    fn to_screen(&self, rect: LogicalRect) -> Option<Rect> {
        let (window, scale_factor) = self.window?;
        to_screen(rect, (window.x, window.y), scale_factor)
    }

    fn bar_on_screen(&self) -> Option<Rect> {
        self.to_screen(self.region.as_ref()?.bar)
    }

    /// The magnified icons in physical screen pixels; one that does not fit
    /// on screen is left out. Empty, without allocating, while no icon is
    /// magnified.
    fn hold_on_screen(&self) -> Vec<Rect> {
        self.region.as_ref().map_or_else(Vec::new, |region| {
            region
                .hold
                .iter()
                .filter_map(|&rect| self.to_screen(rect))
                .collect()
        })
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

    /// The dock reported where it paints. Validated by the caller.
    pub fn set_region(&self, region: HitRegion) {
        self.lock().region = Some(region);
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
/// webview can report its region.
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
        if shared.stopped {
            return;
        }
        if !shared.visible && captured {
            drop(shared);
            if set_captured(window, false) {
                captured = false;
            } else {
                // Retry at the sampling rate instead of spinning.
                thread::sleep(INTERVAL);
            }
            continue;
        }
        let shared = tracker
            .wake
            .wait_while(shared, |shared| !shared.visible && !shared.stopped)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if shared.stopped {
            return;
        }
        let bar = shared.bar_on_screen();
        let hold = shared.hold_on_screen();
        drop(shared);

        if let Some(cursor) = cursor_position() {
            let next = next_captured(captured, cursor, bar, &hold);
            // On failure the window keeps its old state: retry next tick.
            if next != captured && set_captured(window, next) {
                captured = next;
            }
        }
        thread::sleep(INTERVAL);
    }
}

/// Tauri forwards the change to the main thread. Returns whether it worked.
/// On release, tells the dock's page to clear its hover state.
fn set_captured(window: &WebviewWindow, captured: bool) -> bool {
    if let Err(error) = window.set_ignore_cursor_events(!captured) {
        eprintln!("failed to change the dock's mouse capture: {error}");
        return false;
    }
    if !captured && let Err(error) = window.emit_to(dock::LABEL, RELEASED_EVENT, ()) {
        eprintln!("failed to tell the dock it was released: {error}");
    }
    true
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

    fn logical(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width,
            height,
        }
    }

    // Bottom dock at 100%: a 240x104 window at (840, 920) whose bottom 64 px
    // hold the visible bar, 200x64, centered. The first item (100x48 at
    // (868, 968)) is hovered: scaled 1.4 from its bottom center and lifted
    // 6 px it covers (848, 942.8)-(988, 1010). Its neighbour (76x48 at
    // (976, 968)), scaled 1.15, covers (970.3, 960.8)-(1057.7, 1016).
    const BAR: Rect = Rect {
        x: 860,
        y: 960,
        width: 200,
        height: 64,
    };
    const HOVERED: Rect = Rect {
        x: 848,
        y: 943,
        width: 140,
        height: 67,
    };
    const NEIGHBOR: Rect = Rect {
        x: 970,
        y: 961,
        width: 87,
        height: 55,
    };
    const HOLD: [Rect; 2] = [HOVERED, NEIGHBOR];

    #[test]
    fn released_dock_captures_when_the_cursor_enters_the_bar() {
        assert!(next_captured(false, (900, 1000), Some(BAR), &[]));
        assert!(next_captured(false, (860, 960), Some(BAR), &HOLD));
        assert!(next_captured(false, (1059, 1023), Some(BAR), &HOLD));
    }

    #[test]
    fn captured_dock_stays_captured_on_the_bar() {
        assert!(next_captured(true, (1040, 1000), Some(BAR), &HOLD));
        assert!(next_captured(true, (1040, 1000), Some(BAR), &[]));
    }

    #[test]
    fn captured_dock_stays_captured_inside_the_magnified_icon() {
        // Above the bar, on the part of the icon that grew into the headroom.
        assert!(next_captured(true, (900, 950), Some(BAR), &HOLD));
        assert!(next_captured(true, (900, 943), Some(BAR), &HOLD));
        // Left of the bar, where the magnified icon overflows it.
        assert!(next_captured(true, (850, 1000), Some(BAR), &HOLD));
    }

    #[test]
    fn captured_dock_releases_above_the_magnified_icon() {
        assert!(!next_captured(true, (900, 942), Some(BAR), &HOLD));
        assert!(!next_captured(true, (900, 925), Some(BAR), &HOLD));
    }

    #[test]
    fn captured_dock_releases_above_a_part_of_the_bar_that_is_not_magnified() {
        // Over the bar's right end, beside the magnified icons.
        assert!(!next_captured(true, (1058, 955), Some(BAR), &HOLD));
        // Over the neighbour, above its smaller growth.
        assert!(!next_captured(true, (1000, 955), Some(BAR), &HOLD));
        // Nothing magnified: the headroom over the bar never holds.
        assert!(!next_captured(true, (900, 950), Some(BAR), &[]));
    }

    #[test]
    fn captured_dock_releases_beside_the_bar_at_bar_height() {
        // The user's C2 case: inside the window, level with the bar.
        assert!(!next_captured(true, (845, 1000), Some(BAR), &HOLD));
        assert!(!next_captured(true, (1060, 1000), Some(BAR), &HOLD));
        assert!(!next_captured(true, (1075, 1020), Some(BAR), &HOLD));
    }

    #[test]
    fn released_dock_ignores_the_headroom_and_the_magnified_icon() {
        assert!(!next_captured(false, (900, 930), Some(BAR), &HOLD));
        assert!(!next_captured(false, (900, 950), Some(BAR), &HOLD));
        assert!(!next_captured(false, (900, 959), Some(BAR), &HOLD));
        assert!(!next_captured(false, (850, 1000), Some(BAR), &HOLD));
        assert!(!next_captured(false, (1060, 1000), Some(BAR), &HOLD));
    }

    #[test]
    fn dock_without_a_reported_bar_never_captures() {
        assert!(!next_captured(false, (900, 1000), None, &[]));
        assert!(!next_captured(false, (900, 950), None, &HOLD));
    }

    #[test]
    fn full_pass_enters_the_bar_rises_into_the_icon_and_leaves_beside_it() {
        let path = [
            ((900, 900), false),  // above the window
            ((900, 950), false),  // headroom while released
            ((900, 1000), true),  // enters the bar
            ((900, 950), true),   // rises onto the magnified icon
            ((1010, 950), false), // moves beside it, above the bar
            ((900, 950), false),  // back over the icon: still released
            ((1040, 1000), true), // enters the bar again
            ((845, 1000), false), // leaves sideways at bar height
        ];
        let mut captured = false;
        for (cursor, expected) in path {
            captured = next_captured(captured, cursor, Some(BAR), &HOLD);
            assert_eq!(captured, expected, "at {cursor:?}");
        }
    }

    #[test]
    fn top_dock_holds_the_icon_growing_below_the_bar() {
        // Bar on the window's top 64 px; the hovered icon grows downward.
        let bar = rect(860, 8, 200, 64);
        let hovered = rect(848, 14, 140, 67);

        assert!(next_captured(false, (900, 40), Some(bar), &[hovered]));
        assert!(!next_captured(false, (900, 78), Some(bar), &[hovered]));
        assert!(next_captured(true, (900, 78), Some(bar), &[hovered]));
        assert!(!next_captured(true, (1040, 78), Some(bar), &[hovered]));
        assert!(!next_captured(true, (900, 81), Some(bar), &[hovered]));
    }

    #[test]
    fn scale_125_converts_the_reported_region_to_screen_pixels() {
        let window = rect(1130, 1240, 300, 130);
        let shared = Shared {
            window: Some((window, 1.25)),
            region: Some(HitRegion {
                bar: logical(20.0, 40.0, 200.0, 64.0),
                hold: vec![logical(16.0, 24.0, 140.0, 67.2)],
            }),
            ..Shared::default()
        };

        let bar = shared.bar_on_screen();
        let hold = shared.hold_on_screen();

        assert_eq!(bar, Some(rect(1155, 1290, 250, 80)));
        assert_eq!(hold, vec![rect(1150, 1270, 175, 84)]);
        assert!(!next_captured(false, (1200, 1289), bar, &hold));
        assert!(next_captured(false, (1200, 1290), bar, &hold));
        assert!(next_captured(true, (1200, 1275), bar, &hold));
        assert!(!next_captured(true, (1200, 1269), bar, &hold));
        assert!(!next_captured(true, (1400, 1275), bar, &hold));
    }

    #[test]
    fn hold_rects_that_do_not_fit_on_screen_are_dropped() {
        let shared = Shared {
            window: Some((rect(i32::MAX - 10, 0, 300, 130), 3.0)),
            region: Some(HitRegion {
                bar: logical(-1.0, 0.0, 1.0, 1.0),
                hold: vec![
                    logical(100_000.0, 0.0, 1.0, 1.0),
                    logical(-2.0, 0.0, 1.0, 1.0),
                ],
            }),
            ..Shared::default()
        };

        assert_eq!(shared.hold_on_screen(), vec![rect(i32::MAX - 16, 0, 3, 3)]);
    }

    #[test]
    fn nothing_is_held_before_the_window_or_the_region_is_known() {
        let region = HitRegion {
            bar: logical(0.0, 0.0, 10.0, 10.0),
            hold: vec![logical(0.0, 0.0, 10.0, 10.0)],
        };
        let without_window = Shared {
            region: Some(region),
            ..Shared::default()
        };
        let without_region = Shared {
            window: Some((rect(0, 0, 100, 100), 1.0)),
            ..Shared::default()
        };

        assert_eq!(without_window.bar_on_screen(), None);
        assert!(without_window.hold_on_screen().is_empty());
        assert_eq!(without_region.bar_on_screen(), None);
        assert!(without_region.hold_on_screen().is_empty());
    }

    const VALID: LogicalRect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    #[test]
    fn region_with_up_to_16_hold_rects_is_valid() {
        for count in [0, 1, MAX_HOLD_RECTS] {
            let region = HitRegion {
                bar: VALID,
                hold: vec![VALID; count],
            };
            assert_eq!(region.validate(), Ok(()), "{count} hold rects");
        }
        assert_eq!(MAX_HOLD_RECTS, 16);
    }

    #[test]
    fn region_with_too_many_hold_rects_is_rejected() {
        let region = HitRegion {
            bar: VALID,
            hold: vec![VALID; MAX_HOLD_RECTS + 1],
        };

        assert_eq!(region.validate(), Err(RegionError::TooManyHoldRects));
    }

    #[test]
    fn region_with_an_invalid_rect_is_rejected() {
        let bad = LogicalRect {
            width: f64::NAN,
            ..VALID
        };
        let huge = LogicalRect { x: 1e12, ..VALID };

        assert_eq!(
            HitRegion {
                bar: bad,
                hold: vec![]
            }
            .validate(),
            Err(RegionError::Bar(RectError::NotFinite))
        );
        assert_eq!(
            HitRegion {
                bar: VALID,
                hold: vec![VALID, huge]
            }
            .validate(),
            Err(RegionError::Hold(1, RectError::TooLarge))
        );
    }

    #[test]
    fn region_is_read_from_the_frontend_json() {
        let region: HitRegion = serde_json::from_str(
            r#"{"bar":{"x":20,"y":40,"width":200,"height":64},
                "hold":[{"x":-4,"y":-13.2,"width":140,"height":67.2}]}"#,
        )
        .unwrap();

        assert_eq!(
            region,
            HitRegion {
                bar: logical(20.0, 40.0, 200.0, 64.0),
                hold: vec![logical(-4.0, -13.2, 140.0, 67.2)],
            }
        );
        let without_hold = r#"{"bar":{"x":0,"y":0,"width":1,"height":1}}"#;
        assert!(serde_json::from_str::<HitRegion>(without_hold).is_err());
    }
}
