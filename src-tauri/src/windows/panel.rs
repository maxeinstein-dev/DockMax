use std::error::Error;
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};

use super::dock::{self, Edge, Rect};
use super::hit_test;

pub const LABEL: &str = "panel";

/// The panel never ignores the mouse, so tao never makes it layered.
#[cfg(windows)]
pub const LAYERING: super::native::Layering = super::native::Layering::AsTaoSets;

/// Event sent to the panel window right before it is shown.
const SHOW_EVENT: &str = "panel-show";

/// Event sent to the dock when the open panel changes, with
/// `{ "panelId": <id> | null }` (`PanelChanged` in `src/core/panels.ts`).
#[expect(dead_code, reason = "emitted in R1")]
pub const CHANGED_EVENT: &str = "panel-changed";

/// Gap between the panel and the dock, and between the panel and the work
/// area edges, in logical pixels.
const MARGIN: f64 = 8.0;

/// How long after a blur-hide a toggle of the same panel still counts as
/// the click that closed it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelId {
    Settings,
    About,
}

impl PanelId {
    /// Fixed panel size in logical pixels.
    pub fn logical_size(self) -> (f64, f64) {
        match self {
            PanelId::Settings => (360.0, 480.0),
            PanelId::About => (320.0, 240.0),
        }
    }
}

/// Largest coordinate or size accepted from the frontend, in logical pixels.
/// Far beyond any real monitor layout, small enough that scaling it can't
/// leave the `i32` range.
pub const MAX_LOGICAL: f64 = 100_000.0;

/// A rectangle in logical pixels, relative to a window's client area.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectError {
    NotFinite,
    NegativeSize,
    TooLarge,
}

impl fmt::Display for RectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RectError::NotFinite => "a value is not a finite number",
            RectError::NegativeSize => "the width or height is negative",
            RectError::TooLarge => "a value is beyond 100000 logical pixels",
        })
    }
}

impl Error for RectError {}

impl LogicalRect {
    /// Checks a rectangle received from the frontend: finite values,
    /// non-negative size and every value within `MAX_LOGICAL`.
    pub fn validate(&self) -> Result<(), RectError> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(RectError::NotFinite);
        }
        if self.width < 0.0 || self.height < 0.0 {
            return Err(RectError::NegativeSize);
        }
        if values.iter().any(|value| value.abs() > MAX_LOGICAL) {
            return Err(RectError::TooLarge);
        }
        Ok(())
    }
}

/// Converts a rectangle relative to a window's client area into physical
/// screen coordinates. Returns `None` for an invalid rectangle or scale, or
/// when the result leaves the `i32` range.
pub fn to_screen(rect: LogicalRect, client_origin: (i32, i32), scale_factor: f64) -> Option<Rect> {
    rect.validate().ok()?;
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return None;
    }
    // `as` saturates; the checked conversions reject what saturated.
    let scale = |value: f64| (value * scale_factor).round() as i64;
    Some(Rect {
        x: i32::try_from(i64::from(client_origin.0).checked_add(scale(rect.x))?).ok()?,
        y: i32::try_from(i64::from(client_origin.1).checked_add(scale(rect.y))?).ok()?,
        width: u32::try_from(scale(rect.width)).ok()?,
        height: u32::try_from(scale(rect.height)).ok()?,
    })
}

/// Physical rectangle of a panel anchored to a dock item: `MARGIN` away from
/// the visible dock bar on the side facing the screen, horizontally centered
/// on the anchor, and kept `MARGIN` inside the work area. A panel larger than the
/// work area is pinned to the left (or top) margin.
pub fn panel_rect(
    anchor: Rect,
    bar: Rect,
    work_area: Rect,
    scale_factor: f64,
    logical_size: (f64, f64),
    edge: Edge,
) -> Rect {
    // Every term fits in i32/u32 (`as` saturates), so sums in i64 can't
    // overflow; the result is saturated back into i32.
    let width = (logical_size.0 * scale_factor).round() as u32;
    let height = (logical_size.1 * scale_factor).round() as u32;
    let margin = i64::from((MARGIN * scale_factor).round() as i32);
    let anchor_center = i64::from(anchor.x) + i64::from(anchor.width).div_euclid(2);
    let x = anchor_center - i64::from(width).div_euclid(2);
    let y = match edge {
        Edge::Bottom => i64::from(bar.y) - margin - i64::from(height),
        Edge::Top => i64::from(bar.y) + i64::from(bar.height) + margin,
    };
    Rect {
        x: clamp_axis(x, width, work_area.x, work_area.width, margin),
        y: clamp_axis(y, height, work_area.y, work_area.height, margin),
        width,
        height,
    }
}

fn clamp_axis(position: i64, size: u32, start: i32, length: u32, margin: i64) -> i32 {
    let min = i64::from(start) + margin;
    let max = i64::from(start) + i64::from(length) - margin - i64::from(size);
    dock::saturate(if max < min {
        min
    } else {
        position.clamp(min, max)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Show(PanelId),
    Hide,
    Nothing,
}

/// Which panel is open. Only one panel is open at a time.
#[derive(Debug, Default)]
pub struct PanelState {
    open: Option<PanelId>,
    /// Panel hidden by losing focus, and when.
    blurred: Option<(PanelId, Instant)>,
}

impl PanelState {
    /// A click on a dock item. Clicking the item of an open panel makes the
    /// panel lose focus (and hide) before the toggle arrives, so a toggle of
    /// that panel right after a blur-hide keeps it closed.
    pub fn toggle(&mut self, id: PanelId, now: Instant) -> Action {
        let blurred = self.blurred.take();
        match self.open {
            Some(open) if open == id => {
                self.open = None;
                Action::Hide
            }
            Some(_) => {
                self.open = Some(id);
                Action::Show(id)
            }
            None => match blurred {
                Some((closed, at))
                    if closed == id && now.saturating_duration_since(at) < REOPEN_GUARD =>
                {
                    Action::Nothing
                }
                _ => {
                    self.open = Some(id);
                    Action::Show(id)
                }
            },
        }
    }

    /// The panel window lost focus.
    pub fn blur(&mut self, now: Instant) -> Action {
        match self.open.take() {
            Some(id) => {
                self.blurred = Some((id, now));
                Action::Hide
            }
            None => Action::Nothing,
        }
    }

    /// The panel asked to close itself (Esc), or the dock is hiding. Keeps a
    /// pending reopen guard: a late hide after a blur-hide must not let the
    /// click that closed the panel reopen it.
    pub fn hide(&mut self) -> Action {
        match self.open.take() {
            Some(_) => Action::Hide,
            None => Action::Nothing,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowPayload {
    panel_id: PanelId,
}

/// Registers the panel state and hides the panel when it loses focus or is
/// closed (Alt+F4).
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("panel window is missing from tauri.conf.json")?;
    #[cfg(windows)]
    super::native::install(&window, None, LAYERING)?;
    app.manage(Mutex::new(PanelState::default()));
    let app = app.clone();
    window.on_window_event(move |event| {
        let action = match event {
            WindowEvent::Focused(false) => lock(&app).blur(Instant::now()),
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                lock(&app).hide()
            }
            _ => return,
        };
        if let Err(error) = apply(&app, action, None) {
            log::warn!("panel: failed to hide: {error}");
        }
    });
    Ok(())
}

pub fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, PanelState> {
    // The state stays consistent even if a holder panicked.
    app.state::<Mutex<PanelState>>()
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Carries out an action on the panel window. `anchor` is the clicked dock
/// item, relative to the dock's client area; it is required to show.
/// Must be called without holding the state lock: hiding the window
/// synchronously triggers the blur handler.
pub fn apply(
    app: &AppHandle,
    action: Action,
    anchor: Option<LogicalRect>,
) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("panel window is missing")?;
    match action {
        Action::Show(id) => {
            let anchor = anchor.ok_or("missing anchor")?;
            place(app, &window, id, anchor)?;
            window.emit_to(LABEL, SHOW_EVENT, ShowPayload { panel_id: id })?;
            window.show()?;
            focus(&window)?;
        }
        Action::Hide => window.hide()?,
        Action::Nothing => {}
    }
    Ok(())
}

fn place(
    app: &AppHandle,
    window: &WebviewWindow,
    id: PanelId,
    anchor: LogicalRect,
) -> Result<(), Box<dyn Error>> {
    let dock_window = app
        .get_webview_window(dock::LABEL)
        .ok_or("dock window is missing")?;
    let scale_factor = dock_window.scale_factor()?;
    let origin = dock_window.inner_position()?;
    // The gap is measured from the visible bar, not from the taller window.
    let bar = hit_test::tracker(app)
        .bar_on_screen()
        .ok_or("the dock bar has not been measured yet")?;
    let anchor = to_screen(anchor, (origin.x, origin.y), scale_factor).ok_or("invalid anchor")?;
    let (work_area, _) = dock::primary_work_area(&dock_window)?;
    let rect = panel_rect(
        anchor,
        bar,
        work_area,
        scale_factor,
        id.logical_size(),
        dock::EDGE,
    );
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    Ok(())
}

/// Focuses the panel only when DockMax already owns the foreground (the
/// user just clicked the dock). Otherwise tao's `set_focus` would fake an
/// Alt key press to steal the foreground from the app in use.
fn focus(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    if !super::native::owns_foreground() {
        return Ok(());
    }
    window.set_focus()
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

    const WORK_AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1032,
    };
    const BAR: Rect = Rect {
        x: 1160,
        y: 960,
        width: 240,
        height: 64,
    };

    #[test]
    fn panel_ids_use_lowercase_names_in_json() {
        assert_eq!(
            serde_json::to_string(&PanelId::Settings).unwrap(),
            "\"settings\""
        );
        assert_eq!(serde_json::to_string(&PanelId::About).unwrap(), "\"about\"");
        assert_eq!(
            serde_json::from_str::<PanelId>("\"about\"").unwrap(),
            PanelId::About
        );
        assert!(serde_json::from_str::<PanelId>("\"About\"").is_err());
    }

    #[test]
    fn panels_have_fixed_sizes() {
        assert_eq!(PanelId::Settings.logical_size(), (360.0, 480.0));
        assert_eq!(PanelId::About.logical_size(), (320.0, 240.0));
    }

    #[test]
    fn to_screen_offsets_and_scales_the_client_rect() {
        let anchor = LogicalRect {
            x: 10.0,
            y: 20.5,
            width: 99.5,
            height: 30.0,
        };

        assert_eq!(
            to_screen(anchor, (100, 200), 1.5),
            Some(rect(115, 231, 149, 45))
        );
    }

    #[test]
    fn to_screen_rejects_non_finite_or_negative_values() {
        let valid = LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };

        assert_eq!(
            to_screen(
                LogicalRect {
                    x: f64::NAN,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
        assert_eq!(
            to_screen(
                LogicalRect {
                    width: f64::INFINITY,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
        assert_eq!(
            to_screen(
                LogicalRect {
                    height: -1.0,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
    }

    const VALID: LogicalRect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    #[test]
    fn validate_accepts_rects_within_the_limit() {
        let cases = [
            VALID,
            LogicalRect {
                x: -100_000.0,
                y: 100_000.0,
                width: 100_000.0,
                height: 0.0,
            },
            LogicalRect {
                x: 16.5,
                y: -0.0,
                width: 104.25,
                height: 40.0,
            },
        ];
        for rect in cases {
            assert_eq!(rect.validate(), Ok(()), "{rect:?}");
        }
    }

    #[test]
    fn validate_rejects_bad_rects() {
        use RectError::*;
        let cases = [
            (
                LogicalRect {
                    x: f64::NAN,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    y: f64::INFINITY,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    width: f64::NEG_INFINITY,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    height: -1.0,
                    ..VALID
                },
                NegativeSize,
            ),
            (
                LogicalRect {
                    width: -1e12,
                    ..VALID
                },
                NegativeSize,
            ),
            (LogicalRect { x: 1e12, ..VALID }, TooLarge),
            (LogicalRect { x: -1e12, ..VALID }, TooLarge),
            (
                LogicalRect {
                    y: f64::MAX,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    y: f64::MIN,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    width: f64::MAX,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    height: 1e12,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    x: 100_000.5,
                    ..VALID
                },
                TooLarge,
            ),
        ];
        for (rect, expected) in cases {
            assert_eq!(rect.validate(), Err(expected), "{rect:?}");
        }
    }

    #[test]
    fn to_screen_never_panics_and_rejects_what_does_not_fit() {
        let cases = [
            (LogicalRect { x: 1e12, ..VALID }, (0, 0), 1.0),
            (LogicalRect { x: -1e12, ..VALID }, (0, 0), 1.0),
            (
                LogicalRect {
                    y: f64::MAX,
                    ..VALID
                },
                (0, 0),
                1.0,
            ),
            (
                LogicalRect {
                    width: f64::MAX,
                    ..VALID
                },
                (0, 0),
                3.0,
            ),
            (
                LogicalRect {
                    x: f64::NAN,
                    ..VALID
                },
                (0, 0),
                1.0,
            ),
            // Valid rect, origin near the i32 limits.
            (
                LogicalRect {
                    x: 100_000.0,
                    ..VALID
                },
                (i32::MAX - 10, 0),
                3.0,
            ),
            (
                LogicalRect {
                    y: -100_000.0,
                    ..VALID
                },
                (0, i32::MIN + 10),
                3.0,
            ),
            // Valid rect, unusable scale.
            (VALID, (0, 0), f64::NAN),
            (VALID, (0, 0), f64::INFINITY),
            (VALID, (0, 0), 0.0),
            (VALID, (0, 0), -1.0),
            (
                LogicalRect {
                    width: 100_000.0,
                    ..VALID
                },
                (0, 0),
                f64::MAX,
            ),
        ];
        for (rect, origin, scale_factor) in cases {
            assert_eq!(
                to_screen(rect, origin, scale_factor),
                None,
                "{rect:?} {origin:?} {scale_factor}"
            );
        }
    }

    #[test]
    fn to_screen_rejects_a_huge_finite_scale_without_panicking() {
        let right = LogicalRect { x: 1.0, ..VALID };
        let left = LogicalRect { x: -1.0, ..VALID };

        assert_eq!(to_screen(right, (1, 0), f64::MAX), None);
        assert_eq!(to_screen(left, (-1, 0), f64::MAX), None);
    }

    #[test]
    fn to_screen_converts_the_largest_valid_rect_at_scale_300() {
        let rect = LogicalRect {
            x: -100_000.0,
            y: 100_000.0,
            width: 100_000.0,
            height: 100_000.0,
        };

        assert_eq!(
            to_screen(rect, (i32::MAX - 300_000, 0), 3.0),
            Some(Rect {
                x: i32::MAX - 600_000,
                y: 300_000,
                width: 300_000,
                height: 300_000,
            })
        );
    }

    #[test]
    fn panel_rect_saturates_instead_of_overflowing() {
        let max = rect(i32::MAX, i32::MAX, u32::MAX, u32::MAX);
        let min = rect(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
        let cases = [
            (max, BAR, WORK_AREA, 3.0),
            (min, min, WORK_AREA, 3.0),
            (BAR, max, max, 1.0),
            (BAR, min, min, 1.0),
            (max, max, max, f64::MAX),
            (min, min, min, f64::NAN),
        ];
        for (anchor, bar, work_area, scale_factor) in cases {
            for edge in [Edge::Bottom, Edge::Top] {
                // Must not panic.
                let _ = panel_rect(anchor, bar, work_area, scale_factor, (360.0, 480.0), edge);
            }
        }
        let panel = panel_rect(
            BAR,
            BAR,
            rect(i32::MAX, 0, u32::MAX, 1032),
            1.0,
            (360.0, 480.0),
            Edge::Bottom,
        );
        assert_eq!(panel.x, i32::MAX);
    }

    #[test]
    fn bottom_panel_is_centered_on_the_anchor_8px_above_the_bar() {
        let anchor = rect(1200, 976, 100, 32);

        let panel = panel_rect(anchor, BAR, WORK_AREA, 1.0, (360.0, 480.0), Edge::Bottom);

        assert_eq!(panel, rect(1250 - 180, 960 - 8 - 480, 360, 480));
    }

    #[test]
    fn panel_near_the_left_edge_is_clamped_to_the_margin() {
        let anchor = rect(20, 976, 40, 32);

        let panel = panel_rect(anchor, BAR, WORK_AREA, 1.0, (360.0, 480.0), Edge::Bottom);

        assert_eq!(panel.x, 8);
    }

    #[test]
    fn panel_near_the_right_edge_is_clamped_to_the_margin() {
        let anchor = rect(2500, 976, 40, 32);

        let panel = panel_rect(anchor, BAR, WORK_AREA, 1.0, (360.0, 480.0), Edge::Bottom);

        assert_eq!(panel.x, 2560 - 8 - 360);
    }

    #[test]
    fn top_panel_sits_8px_below_the_bar() {
        let bar = rect(1160, 8, 240, 64);
        let anchor = rect(1200, 24, 100, 32);

        let panel = panel_rect(anchor, bar, WORK_AREA, 1.0, (320.0, 240.0), Edge::Top);

        assert_eq!(panel, rect(1250 - 160, 8 + 64 + 8, 320, 240));
    }

    #[test]
    fn scale_125_scales_size_gap_and_margin() {
        let work_area = rect(0, 0, 2048, 824);
        let bar = rect(874, 734, 300, 80);
        let anchor = rect(900, 754, 125, 40);

        let panel = panel_rect(anchor, bar, work_area, 1.25, (360.0, 480.0), Edge::Bottom);

        // 360 * 1.25 = 450, 480 * 1.25 = 600, 8 * 1.25 = 10.
        assert_eq!(panel, rect(962 - 225, 734 - 10 - 600, 450, 600));
    }

    #[test]
    fn scale_150_scales_size_gap_and_margin() {
        let work_area = rect(0, 0, 2560, 1368);
        let bar = rect(1100, 1260, 360, 96);
        let anchor = rect(1130, 1284, 150, 48);

        let panel = panel_rect(anchor, bar, work_area, 1.5, (320.0, 240.0), Edge::Bottom);

        // 320 * 1.5 = 480, 240 * 1.5 = 360, 8 * 1.5 = 12.
        assert_eq!(panel, rect(1205 - 240, 1260 - 12 - 360, 480, 360));
    }

    #[test]
    fn scale_150_clamps_with_the_scaled_margin() {
        let work_area = rect(0, 0, 2560, 1368);
        let bar = rect(1100, 1260, 360, 96);
        let anchor = rect(0, 1284, 20, 48);

        let panel = panel_rect(anchor, bar, work_area, 1.5, (320.0, 240.0), Edge::Bottom);

        assert_eq!(panel.x, 12);
    }

    #[test]
    fn panel_wider_than_the_work_area_is_pinned_to_the_left_margin() {
        let work_area = rect(100, 0, 300, 1032);

        let panel = panel_rect(
            rect(200, 976, 40, 32),
            BAR,
            work_area,
            1.0,
            (360.0, 480.0),
            Edge::Bottom,
        );

        assert_eq!(panel.x, 108);
        assert_eq!(panel.width, 360);
    }

    #[test]
    fn panel_taller_than_the_space_above_the_bar_stays_inside_the_work_area() {
        let work_area = rect(0, 40, 2560, 600);
        let bar = rect(1160, 400, 240, 64);

        let panel = panel_rect(
            rect(1200, 416, 100, 32),
            bar,
            work_area,
            1.0,
            (360.0, 480.0),
            Edge::Bottom,
        );

        // 400 - 8 - 480 = -88 would leave the work area.
        assert_eq!(panel.y, 48);
    }

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn toggle_opens_a_closed_panel() {
        let mut state = PanelState::default();

        assert_eq!(
            state.toggle(PanelId::Settings, Instant::now()),
            Action::Show(PanelId::Settings)
        );
    }

    #[test]
    fn toggle_of_the_open_panel_hides_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);

        assert_eq!(state.toggle(PanelId::Settings, at(t, 1000)), Action::Hide);
        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 1100)),
            Action::Show(PanelId::Settings)
        );
    }

    #[test]
    fn toggle_of_another_panel_switches_content() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);

        assert_eq!(
            state.toggle(PanelId::About, at(t, 50)),
            Action::Show(PanelId::About)
        );
        assert_eq!(state.toggle(PanelId::About, at(t, 100)), Action::Hide);
    }

    #[test]
    fn blur_hides_the_open_panel() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::About, t);

        assert_eq!(state.blur(at(t, 10)), Action::Hide);
        assert_eq!(state.blur(at(t, 20)), Action::Nothing);
    }

    #[test]
    fn blur_without_an_open_panel_does_nothing() {
        let mut state = PanelState::default();

        assert_eq!(state.blur(Instant::now()), Action::Nothing);
    }

    #[test]
    fn toggle_right_after_a_blur_hide_of_the_same_panel_keeps_it_closed() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 1299)),
            Action::Nothing
        );
        // The guard is consumed: the next click opens again.
        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 1350)),
            Action::Show(PanelId::Settings)
        );
    }

    #[test]
    fn toggle_300ms_or_more_after_a_blur_hide_reopens() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 1300)),
            Action::Show(PanelId::Settings)
        );
    }

    #[test]
    fn toggle_of_another_panel_right_after_a_blur_hide_opens_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            state.toggle(PanelId::About, at(t, 1050)),
            Action::Show(PanelId::About)
        );
    }

    #[test]
    fn hide_closes_the_open_panel_without_arming_the_reopen_guard() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);

        assert_eq!(state.hide(), Action::Hide);
        // The focus loss caused by hiding must not arm the guard.
        assert_eq!(state.blur(at(t, 10)), Action::Nothing);
        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 50)),
            Action::Show(PanelId::Settings)
        );
    }

    #[test]
    fn hide_after_a_blur_hide_keeps_the_reopen_guard() {
        let mut state = PanelState::default();
        let t = Instant::now();
        state.toggle(PanelId::Settings, t);
        state.blur(at(t, 1000));

        // A late hide (Esc or the dock hiding) after the blur-hide.
        assert_eq!(state.hide(), Action::Nothing);
        // The click that closed the panel must not reopen it.
        assert_eq!(
            state.toggle(PanelId::Settings, at(t, 1100)),
            Action::Nothing
        );
    }

    #[test]
    fn hide_without_an_open_panel_does_nothing() {
        let mut state = PanelState::default();

        assert_eq!(state.hide(), Action::Nothing);
    }
}
