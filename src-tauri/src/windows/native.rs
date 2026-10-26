//! Win32 glue for the WebView2 windows: keeps them out of Alt+Tab, and tells
//! whose the foreground is. (The dock is a window of its own,
//! `native_dock`.)

use std::error::Error;

use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindowVisible,
    STYLESTRUCT, SetWindowLongPtrW, WM_NCDESTROY, WM_STYLECHANGING, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW,
};
use tauri::WebviewWindow;

const SUBCLASS_ID: usize = 1;

/// Turns the window into a tool window (left out of Alt+Tab), and keeps it
/// one.
pub fn install(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    let hwnd = window.hwnd()?;
    unsafe {
        if !SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0).as_bool() {
            return Err("failed to subclass the window".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style(style) as isize);
    }
    Ok(())
}

/// Whether the foreground window belongs to this process.
pub fn owns_foreground() -> bool {
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process_id)) };
    process_id == std::process::id()
}

/// Whether the foreground window is a visible window of another program: the
/// user went there. The hidden window of a tray program that has just shown a
/// menu is not.
pub fn foreground_is_other_visible_window() -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return false;
    }
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
    process_id != std::process::id() && unsafe { IsWindowVisible(window) }.as_bool()
}

/// The extended style DockMax keeps: a tool window without
/// `WS_EX_APPWINDOW` is left out of Alt+Tab. Any other bit is left as tao
/// set it.
fn ex_style(style: u32) -> u32 {
    (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        // tao rewrites the extended style whenever the window is shown or
        // hidden, which would bring back WS_EX_APPWINDOW.
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            let style = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
            style.styleNew = ex_style(style.styleNew);
        }
        WM_NCDESTROY => unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID);
        },
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use ::windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_EX_TOPMOST};

    use super::*;

    #[test]
    fn a_window_becomes_a_tool_window_without_appwindow() {
        // What tao writes for the panel: topmost, plus APPWINDOW when shown.
        for style in [WS_EX_TOPMOST.0, WS_EX_TOPMOST.0 | WS_EX_APPWINDOW.0] {
            let kept = ex_style(style);
            assert_ne!(kept & WS_EX_TOOLWINDOW.0, 0);
            assert_eq!(kept & WS_EX_APPWINDOW.0, 0);
            assert_ne!(kept & WS_EX_TOPMOST.0, 0);
            // Never made layered: tao never makes the panel layered.
            assert_eq!(kept & WS_EX_LAYERED.0, 0);
        }
    }
}
