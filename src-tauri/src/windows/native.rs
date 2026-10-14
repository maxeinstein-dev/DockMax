//! Win32 glue: keeps DockMax windows out of Alt+Tab, keeps the dock a
//! well-formed layered window and reports display configuration changes,
//! which Tauri does not.

use std::error::Error;

use ::windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, LWA_ALPHA,
    PostMessageW, SPI_SETWORKAREA, STYLESTRUCT, SetLayeredWindowAttributes, SetWindowLongPtrW,
    WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_NCDESTROY, WM_SETTINGCHANGE, WM_STYLECHANGING,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
};
use tauri::WebviewWindow;

const SUBCLASS_ID: usize = 1;
const WM_DISPLAY_CHANGED: u32 = WM_APP + 1;

struct Subclass {
    window: WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
    always_layered: bool,
}

/// Turns the window into a tool window (left out of Alt+Tab) and, when
/// `on_display_change` is given, calls it after the resolution, the scale
/// or the work area change.
///
/// `always_layered` keeps `WS_EX_LAYERED` on for good, for a window whose
/// mouse capture is toggled with `set_ignore_cursor_events`: tao adds and
/// removes `WS_EX_LAYERED` along with `WS_EX_TRANSPARENT` and never calls
/// `SetLayeredWindowAttributes`, and the first time such a window stops
/// being layered Windows paints its transparent area opaque white. Kept
/// layered, with its attributes set once, the window only toggles
/// `WS_EX_TRANSPARENT`, which is what lets clicks through.
pub fn install(
    window: &WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
    always_layered: bool,
) -> Result<(), Box<dyn Error>> {
    let hwnd = window.hwnd()?;
    let data = Box::into_raw(Box::new(Subclass {
        window: window.clone(),
        on_display_change,
        always_layered,
    })) as usize;
    unsafe {
        if !SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, data).as_bool() {
            drop(Box::from_raw(data as *mut Subclass));
            return Err("failed to subclass the window".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style(style, always_layered) as isize);
        if always_layered {
            // Fully opaque as a whole; the page's transparent pixels stay
            // transparent.
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
        }
    }
    Ok(())
}

/// Whether the foreground window belongs to this process.
pub fn owns_foreground() -> bool {
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process_id)) };
    process_id == std::process::id()
}

/// The extended style DockMax keeps: a tool window without
/// `WS_EX_APPWINDOW` is left out of Alt+Tab; `always_layered` also keeps
/// `WS_EX_LAYERED`. Any other bit, `WS_EX_TRANSPARENT` included, is left as
/// tao set it.
fn ex_style(style: u32, always_layered: bool) -> u32 {
    let style = (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
    if always_layered {
        style | WS_EX_LAYERED.0
    } else {
        style
    }
}

fn post_display_changed(hwnd: HWND) {
    // Posted, so it runs after tao has finished handling the message
    // (on WM_DPICHANGED tao moves the window itself).
    let _ = unsafe { PostMessageW(Some(hwnd), WM_DISPLAY_CHANGED, WPARAM(0), LPARAM(0)) };
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let subclass = unsafe { &*(data as *const Subclass) };
    match msg {
        // tao rewrites the extended style whenever the window is shown or
        // hidden, which would bring back WS_EX_APPWINDOW, and whenever it
        // ignores the mouse or stops ignoring it, which would add or remove
        // WS_EX_LAYERED.
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            let style = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
            style.styleNew = ex_style(style.styleNew, subclass.always_layered);
        }
        WM_DPICHANGED | WM_DISPLAYCHANGE if subclass.on_display_change.is_some() => {
            post_display_changed(hwnd)
        }
        WM_SETTINGCHANGE
            if wparam.0 == SPI_SETWORKAREA.0 as usize && subclass.on_display_change.is_some() =>
        {
            post_display_changed(hwnd)
        }
        WM_DISPLAY_CHANGED => {
            if let Some(on_display_change) = subclass.on_display_change {
                on_display_change(&subclass.window);
            }
            return LRESULT(0);
        }
        WM_NCDESTROY => unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID);
            drop(Box::from_raw(data as *mut Subclass));
        },
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use ::windows::Win32::UI::WindowsAndMessaging::{WS_EX_TOPMOST, WS_EX_TRANSPARENT};

    use super::*;

    // What tao writes for the dock: topmost, plus TRANSPARENT|LAYERED while
    // it ignores the mouse, and APPWINDOW when shown.
    const CAPTURED: u32 = WS_EX_TOPMOST.0 | WS_EX_APPWINDOW.0;
    const RELEASED: u32 = CAPTURED | WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0;

    #[test]
    fn every_window_becomes_a_tool_window_without_appwindow() {
        for always_layered in [false, true] {
            let style = ex_style(CAPTURED, always_layered);
            assert_ne!(style & WS_EX_TOOLWINDOW.0, 0);
            assert_eq!(style & WS_EX_APPWINDOW.0, 0);
        }
    }

    #[test]
    fn always_layered_window_stays_layered_when_tao_captures_the_mouse() {
        let style = ex_style(CAPTURED, true);

        assert_ne!(style & WS_EX_LAYERED.0, 0);
        // Only WS_EX_TRANSPARENT tells captured from released.
        assert_eq!(style & WS_EX_TRANSPARENT.0, 0);
        assert_eq!(ex_style(RELEASED, true), style | WS_EX_TRANSPARENT.0);
    }

    #[test]
    fn other_windows_keep_the_layering_tao_chose() {
        assert_eq!(ex_style(CAPTURED, false) & WS_EX_LAYERED.0, 0);
        assert_ne!(ex_style(RELEASED, false) & WS_EX_LAYERED.0, 0);
    }
}
