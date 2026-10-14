use std::error::Error;

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

use super::panel;

pub const LABEL: &str = "dock";

/// Gap between the dock and the work area edge, in logical pixels.
const MARGIN: f64 = 8.0;

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

/// Physical rectangle of the dock window: centered horizontally in the work
/// area and `MARGIN` away from the given edge. Logical values are rounded
/// the same way Tauri converts logical to physical pixels.
pub fn dock_rect(work_area: Rect, scale_factor: f64, logical_size: (f64, f64), edge: Edge) -> Rect {
    let width = (logical_size.0 * scale_factor).round() as u32;
    let height = (logical_size.1 * scale_factor).round() as u32;
    let margin = (MARGIN * scale_factor).round() as i32;
    let x = work_area.x + (work_area.width as i32 - width as i32).div_euclid(2);
    let y = match edge {
        Edge::Bottom => work_area.y + work_area.height as i32 - margin - height as i32,
        Edge::Top => work_area.y + margin,
    };
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Shows the dock on the primary monitor. Closing it (Alt+F4) hides it.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
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
    Ok(())
}

/// Hides the dock and the open panel.
fn hide(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let action = panel::lock(app).hide();
    panel::apply(app, action, None)?;
    app.get_webview_window(LABEL)
        .ok_or("dock window is missing")?
        .hide()?;
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
        logical_size(window.app_handle())?,
        EDGE,
    );
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
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

/// The dock size declared in tauri.conf.json, in logical pixels.
fn logical_size(app: &AppHandle) -> Result<(f64, f64), Box<dyn Error>> {
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
    fn bottom_at_100_percent_sits_8px_above_the_work_area() {
        let work_area = rect(0, 0, 1920, 1032);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(840, 1032 - 8 - 64, 240, 64));
    }

    #[test]
    fn bottom_at_125_percent_scales_size_and_margin() {
        let work_area = rect(0, 0, 2560, 1380);

        let dock = dock_rect(work_area, 1.25, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(1130, 1380 - 10 - 80, 300, 80));
    }

    #[test]
    fn bottom_at_150_percent_scales_size_and_margin() {
        let work_area = rect(0, 0, 2560, 1368);

        let dock = dock_rect(work_area, 1.5, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(1100, 1368 - 12 - 96, 360, 96));
    }

    #[test]
    fn top_edge_keeps_the_margin_below_the_work_area_top() {
        let work_area = rect(0, 0, 1920, 1032);

        let dock = dock_rect(work_area, 1.5, SIZE, Edge::Top);

        assert_eq!(dock, rect(780, 12, 360, 96));
    }

    #[test]
    fn work_area_offset_by_a_top_taskbar_moves_the_dock_with_it() {
        let work_area = rect(0, 48, 1920, 1032);

        assert_eq!(dock_rect(work_area, 1.0, SIZE, Edge::Top).y, 56);
        assert_eq!(dock_rect(work_area, 1.0, SIZE, Edge::Bottom).y, 1008);
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

        assert_eq!(dock, rect(-1920 + 840, -200 + 1040 - 8 - 64, 240, 64));
    }

    #[test]
    fn fractional_physical_sizes_round_half_away_from_zero() {
        let work_area = rect(0, 0, 1000, 1000);

        // 161 * 1.25 = 201.25 -> 201, 63 * 1.25 = 78.75 -> 79,
        // 161 * 1.5 = 241.5 -> 242, 63 * 1.5 = 94.5 -> 95, 8 * 1.75 = 14.
        let dock = dock_rect(work_area, 1.25, (161.0, 63.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (201, 79));

        let dock = dock_rect(work_area, 1.5, (161.0, 63.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (242, 95));

        let dock = dock_rect(work_area, 1.75, SIZE, Edge::Bottom);
        assert_eq!(dock.y, 1000 - 14 - 112);
    }

    #[test]
    fn odd_leftover_width_rounds_the_x_offset_down() {
        let work_area = rect(0, 0, 1001, 1000);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock.x, 380);
    }
}
