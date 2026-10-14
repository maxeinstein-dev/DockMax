//! Win32 glue: keeps DockMax windows out of Alt+Tab and reports display
//! configuration changes, which Tauri does not.

use std::error::Error;

use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, PostMessageW,
    SPI_SETWORKAREA, STYLESTRUCT, SetWindowLongPtrW, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_NCDESTROY, WM_SETTINGCHANGE, WM_STYLECHANGING, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};
use tauri::WebviewWindow;

const SUBCLASS_ID: usize = 1;
const WM_DISPLAY_CHANGED: u32 = WM_APP + 1;

struct Subclass {
    window: WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
}

/// Turns the window into a tool window (left out of Alt+Tab) and, when
/// `on_display_change` is given, calls it after the resolution, the scale
/// or the work area change.
pub fn install(
    window: &WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
) -> Result<(), Box<dyn Error>> {
    let hwnd = window.hwnd()?;
    let data = Box::into_raw(Box::new(Subclass {
        window: window.clone(),
        on_display_change,
    })) as usize;
    unsafe {
        if !SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, data).as_bool() {
            drop(Box::from_raw(data as *mut Subclass));
            return Err("failed to subclass the window".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, tool_window(style) as isize);
    }
    Ok(())
}

/// Whether the foreground window belongs to this process.
pub fn owns_foreground() -> bool {
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process_id)) };
    process_id == std::process::id()
}

/// A tool window without `WS_EX_APPWINDOW` is left out of Alt+Tab.
fn tool_window(style: u32) -> u32 {
    (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0
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
        // hidden, which would bring back WS_EX_APPWINDOW.
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            let style = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
            style.styleNew = tool_window(style.styleNew);
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
