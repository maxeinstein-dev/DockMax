use std::error::Error;

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

use super::{hit_test, panel};

pub const LABEL: &str = "dock";

/// Gap between the visible bar and the work area edge, in logical pixels.
const MARGIN: f64 = 8.0;

/// Transparent space added to the window, on the side away from the screen
/// edge, for a hovered icon to grow into, in logical pixels.
pub const HEADROOM: f64 = 40.0;

/// Screen edge the dock sits on.
pub const EDGE: Edge = Edge::Bottom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    #[allow(dead_code)] // Top placement arrives with the dock settings.
    Top,
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Whether the point lies inside (right and bottom edges excluded).
    pub fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x
            && y >= self.y
            && i64::from(x) < i64::from(self.x) + i64::from(self.width)
            && i64::from(y) < i64::from(self.y) + i64::from(self.height)
    }
}

/// Narrows to i32, saturating at its bounds.
pub fn saturate(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Physical rectangle of the dock window for a bar of `bar_size` logical
/// pixels: centered horizontally in the work area, `HEADROOM` taller than
/// the bar on the side away from the edge, with the bar's outer side
/// `MARGIN` away from the given edge. Logical values are rounded the same
/// way Tauri converts logical to physical pixels.
pub fn dock_rect(work_area: Rect, scale_factor: f64, bar_size: (f64, f64), edge: Edge) -> Rect {
    // Every term fits in i32/u32 (`as` saturates), so sums in i64 can't
    // overflow.
    let width = (bar_size.0 * scale_factor).round() as u32;
    let height = ((bar_size.1 + HEADROOM) * scale_factor).round() as u32;
    let margin = i64::from((MARGIN * scale_factor).round() as i32);
    let x = i64::from(work_area.x) + (i64::from(work_area.width) - i64::from(width)).div_euclid(2);
    let y = match edge {
        Edge::Bottom => {
            i64::from(work_area.y) + i64::from(work_area.height) - margin - i64::from(height)
        }
        Edge::Top => i64::from(work_area.y) + margin,
    };
    Rect {
        x: saturate(x),
        y: saturate(y),
        width,
        height,
    }
}

/// Shows the dock on the primary monitor. Closing it (Alt+F4) hides it.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
    hit_test::start(app, &window)?;
    #[cfg(windows)]
    super::native::install(&window, Some(reposition))?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = hide(&handle) {
                eprintln!("failed to hide the dock: {error}");
            }
        }
    });
    show(&window)
}

/// Hides the dock (and its panel) when visible, shows it otherwise.
pub fn toggle(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing")?;
    if window.is_visible()? {
        hide(app)
    } else {
        show(&window)
    }
}

/// Places the dock and shows it without taking the foreground from the app
/// in use.
fn show(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    place(window)?;
    // tao shows windows with SW_SHOW, which activates them unless they
    // can't take focus.
    window.set_focusable(false)?;
    window.show()?;
    window.set_focusable(true)?;
    hit_test::tracker(window.app_handle()).set_visible(true);
    Ok(())
}

/// Hides the dock and the open panel.
fn hide(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let action = panel::lock(app).hide();
    panel::apply(app, action, None)?;
    app.get_webview_window(LABEL)
        .ok_or("dock window is missing")?
        .hide()?;
    hit_test::tracker(app).set_visible(false);
    Ok(())
}

#[cfg(windows)]
fn reposition(window: &WebviewWindow) {
    if let Err(error) = place(window) {
        eprintln!("failed to reposition the dock: {error}");
    }
}

/// Moves and sizes the dock above the primary monitor's work area bottom.
fn place(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    let (work_area, scale_factor) = primary_work_area(window)?;
    let rect = dock_rect(
        work_area,
        scale_factor,
        bar_size(window.app_handle())?,
        EDGE,
    );
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    let origin = window.inner_position()?;
    let size = window.inner_size()?;
    hit_test::tracker(window.app_handle()).set_window(
        Rect {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        },
        window.scale_factor()?,
    );
    Ok(())
}

/// The primary monitor's work area, in physical pixels, and its scale factor.
pub fn primary_work_area(window: &WebviewWindow) -> Result<(Rect, f64), Box<dyn Error>> {
    let monitor = window
        .primary_monitor()?
        .ok_or("no primary monitor found")?;
    let work_area = monitor.work_area();
    let work_area = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    };
    Ok((work_area, monitor.scale_factor()))
}

/// The bar size, in logical pixels: the dock size declared in
/// tauri.conf.json. The window adds `HEADROOM` to it.
fn bar_size(app: &AppHandle) -> Result<(f64, f64), Box<dyn Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
    Ok((config.width, config.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: (f64, f64) = (240.0, 64.0);

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn bottom_at_100_percent_keeps_the_bar_8px_above_the_work_area() {
        let work_area = rect(0, 0, 1920, 1032);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        // 64 px bar + 40 px headroom above it; the bar is the bottom 64 px.
        assert_eq!(dock, rect(840, 1032 - 8 - 104, 240, 104));
        assert_eq!(dock.y + dock.height as i32, 1032 - 8);
    }

    #[test]
    fn bottom_at_125_percent_scales_size_headroom_and_margin() {
        let work_area = rect(0, 0, 2560, 1380);

        let dock = dock_rect(work_area, 1.25, SIZE, Edge::Bottom);

        // (64 + 40) * 1.25 = 130, 8 * 1.25 = 10.
        assert_eq!(dock, rect(1130, 1380 - 10 - 130, 300, 130));
    }

    #[test]
    fn bottom_at_150_percent_scales_size_headroom_and_margin() {
        let work_area = rect(0, 0, 2560, 1368);

        let dock = dock_rect(work_area, 1.5, SIZE, Edge::Bottom);

        // (64 + 40) * 1.5 = 156, 8 * 1.5 = 12.
        assert_eq!(dock, rect(1100, 1368 - 12 - 156, 360, 156));
    }

    #[test]
    fn top_edge_keeps_the_bar_8px_below_the_work_area_top() {
        let work_area = rect(0, 0, 1920, 1032);

        let dock = dock_rect(work_area, 1.5, SIZE, Edge::Top);

        // The bar is the top 96 px; the headroom hangs below it.
        assert_eq!(dock, rect(780, 12, 360, 156));
    }

    #[test]
    fn work_area_offset_by_a_top_taskbar_moves_the_dock_with_it() {
        let work_area = rect(0, 48, 1920, 1032);

        assert_eq!(dock_rect(work_area, 1.0, SIZE, Edge::Top).y, 56);
        assert_eq!(
            dock_rect(work_area, 1.0, SIZE, Edge::Bottom).y,
            48 + 1032 - 8 - 104
        );
    }

    #[test]
    fn work_area_offset_by_a_left_taskbar_centers_within_the_work_area() {
        let work_area = rect(62, 0, 1858, 1080);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock.x, 62 + (1858 - 240) / 2);
        assert_eq!(dock.x + dock.width as i32 / 2, 62 + 1858 / 2);
    }

    #[test]
    fn negative_work_area_origin_is_supported() {
        let work_area = rect(-1920, -200, 1920, 1040);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(-1920 + 840, -200 + 1040 - 8 - 104, 240, 104));
    }

    #[test]
    fn fractional_physical_sizes_round_half_away_from_zero() {
        let work_area = rect(0, 0, 1000, 1000);

        // 161 * 1.25 = 201.25 -> 201, (63 + 40) * 1.25 = 128.75 -> 129,
        // 161 * 1.5 = 241.5 -> 242, (63 + 40) * 1.5 = 154.5 -> 155,
        // 8 * 1.75 = 14, (64 + 40) * 1.75 = 182.
        let dock = dock_rect(work_area, 1.25, (161.0, 63.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (201, 129));

        let dock = dock_rect(work_area, 1.5, (161.0, 63.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (242, 155));

        let dock = dock_rect(work_area, 1.75, SIZE, Edge::Bottom);
        assert_eq!(dock.y, 1000 - 14 - 182);
    }

    #[test]
    fn odd_leftover_width_rounds_the_x_offset_down() {
        let work_area = rect(0, 0, 1001, 1000);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock.x, 380);
    }

    #[test]
    fn extreme_work_areas_saturate_instead_of_overflowing() {
        let cases = [
            (rect(i32::MAX, i32::MAX, u32::MAX, u32::MAX), 1.0),
            (rect(i32::MIN, i32::MIN, 0, 0), 3.0),
            (rect(i32::MAX - 10, 0, 100, 100), 3.0),
            (rect(0, 0, 1920, 1080), f64::MAX),
            (rect(0, 0, 1920, 1080), f64::NAN),
        ];
        for (work_area, scale_factor) in cases {
            for edge in [Edge::Bottom, Edge::Top] {
                // Must not panic.
                let _ = dock_rect(work_area, scale_factor, SIZE, edge);
            }
        }
        assert_eq!(
            dock_rect(rect(i32::MAX, 0, u32::MAX, 1000), 1.0, SIZE, Edge::Top).x,
            i32::MAX
        );
    }

    #[test]
    fn rect_contains_excludes_the_right_and_bottom_edges() {
        let r = rect(-10, 20, 30, 40);

        assert!(r.contains((-10, 20)));
        assert!(r.contains((19, 59)));
        assert!(!r.contains((20, 30)));
        assert!(!r.contains((0, 60)));
        assert!(!r.contains((-11, 30)));
        assert!(!r.contains((0, 19)));
    }
}
