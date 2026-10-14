use std::error::Error;

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const LABEL: &str = "dock";

/// Gap between the dock and the work area edge, in logical pixels.
const MARGIN: f64 = 8.0;

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

/// Positions the dock on the primary monitor and shows it.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
    #[cfg(windows)]
    native::install(&window)?;
    place(&window)?;
    window.show()?;
    Ok(())
}

/// Moves and sizes the dock above the primary monitor's work area bottom.
fn place(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
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
    let rect = dock_rect(
        work_area,
        monitor.scale_factor(),
        logical_size(window.app_handle())?,
        Edge::Bottom,
    );
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    Ok(())
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

/// Win32 glue: keeps the dock out of Alt+Tab and repositions it when the
/// display configuration changes, which Tauri does not report.
#[cfg(windows)]
mod native {
    use std::error::Error;

    use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, PostMessageW, SPI_SETWORKAREA, STYLESTRUCT,
        SetWindowLongPtrW, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_NCDESTROY, WM_SETTINGCHANGE,
        WM_STYLECHANGING, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    use tauri::WebviewWindow;

    const SUBCLASS_ID: usize = 1;
    const WM_DOCK_REPOSITION: u32 = WM_APP + 1;

    pub fn install(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
        let hwnd = window.hwnd()?;
        let data = Box::into_raw(Box::new(window.clone())) as usize;
        unsafe {
            if !SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, data).as_bool() {
                drop(Box::from_raw(data as *mut WebviewWindow));
                return Err("failed to subclass the dock window".into());
            }
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, tool_window(style) as isize);
        }
        Ok(())
    }

    /// A tool window without `WS_EX_APPWINDOW` is left out of Alt+Tab.
    fn tool_window(style: u32) -> u32 {
        (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0
    }

    fn post_reposition(hwnd: HWND) {
        // Posted, so it runs after tao has finished handling the message
        // (on WM_DPICHANGED tao moves the window itself).
        let _ = unsafe { PostMessageW(Some(hwnd), WM_DOCK_REPOSITION, WPARAM(0), LPARAM(0)) };
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        match msg {
            // tao rewrites the extended style whenever the window is shown or
            // hidden, which would bring back WS_EX_APPWINDOW.
            WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
                let style = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
                style.styleNew = tool_window(style.styleNew);
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE => post_reposition(hwnd),
            WM_SETTINGCHANGE if wparam.0 == SPI_SETWORKAREA.0 as usize => post_reposition(hwnd),
            WM_DOCK_REPOSITION => {
                let window = unsafe { &*(data as *const WebviewWindow) };
                if let Err(error) = super::place(window) {
                    eprintln!("failed to reposition the dock: {error}");
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID);
                drop(Box::from_raw(data as *mut WebviewWindow));
            },
            _ => {}
        }
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }
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
