//! Win32 glue: keeps DockMax windows out of Alt+Tab, keeps the dock a
//! well-formed layered window and reports display configuration changes,
//! which Tauri does not.

use std::error::Error;

use ::windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindowVisible,
    LWA_ALPHA, PostMessageW, SPI_SETWORKAREA, STYLESTRUCT, SetLayeredWindowAttributes,
    SetWindowLongPtrW, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_NCDESTROY, WM_SETTINGCHANGE,
    WM_STYLECHANGING, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
};
use tauri::WebviewWindow;

const SUBCLASS_ID: usize = 1;
const WM_DISPLAY_CHANGED: u32 = WM_APP + 1;

/// How a window treats `WS_EX_LAYERED`. Each window states its own next
/// to its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layering {
    /// `WS_EX_LAYERED` stays on for good, with the layered attributes set
    /// once. For a window whose mouse capture is toggled with
    /// `set_ignore_cursor_events`, the dock: tao adds and removes
    /// `WS_EX_LAYERED` along with `WS_EX_TRANSPARENT` and never calls
    /// `SetLayeredWindowAttributes`, and the first time such a window
    /// stopped being layered Windows painted its transparent area opaque
    /// white. Kept layered, the window only toggles `WS_EX_TRANSPARENT`,
    /// which is what lets clicks through.
    Always,
    /// `WS_EX_LAYERED` is left as tao sets it. For a window that never
    /// ignores the mouse, which tao never makes layered.
    AsTaoSets,
}

struct Subclass {
    window: WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
    layering: Layering,
}

/// Turns the window into a tool window (left out of Alt+Tab), keeps its
/// `WS_EX_LAYERED` as `layering` says and, when `on_display_change` is
/// given, calls it after the resolution, the scale or the work area change.
pub fn install(
    window: &WebviewWindow,
    on_display_change: Option<fn(&WebviewWindow)>,
    layering: Layering,
) -> Result<(), Box<dyn Error>> {
    let hwnd = window.hwnd()?;
    let data = Box::into_raw(Box::new(Subclass {
        window: window.clone(),
        on_display_change,
        layering,
    })) as usize;
    unsafe {
        if !SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, data).as_bool() {
            drop(Box::from_raw(data as *mut Subclass));
            return Err("failed to subclass the window".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style(style, layering) as isize);
        if layering == Layering::Always {
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
/// `WS_EX_APPWINDOW` is left out of Alt+Tab; `Layering::Always` also keeps
/// `WS_EX_LAYERED`. Any other bit, `WS_EX_TRANSPARENT` included, is left as
/// tao set it.
fn ex_style(style: u32, layering: Layering) -> u32 {
    let style = (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
    match layering {
        Layering::Always => style | WS_EX_LAYERED.0,
        Layering::AsTaoSets => style,
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
            style.styleNew = ex_style(style.styleNew, subclass.layering);
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

    use super::super::{dock, panel};
    use super::*;

    // What tao writes for the dock: topmost, plus TRANSPARENT|LAYERED while
    // it ignores the mouse, and APPWINDOW when shown.
    const CAPTURED: u32 = WS_EX_TOPMOST.0 | WS_EX_APPWINDOW.0;
    const RELEASED: u32 = CAPTURED | WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0;

    // What tao writes for the panel, which never ignores the mouse:
    // topmost, plus APPWINDOW when shown.
    const PANEL_STYLES: [u32; 2] = [WS_EX_TOPMOST.0, WS_EX_TOPMOST.0 | WS_EX_APPWINDOW.0];

    #[test]
    fn every_window_becomes_a_tool_window_without_appwindow() {
        for layering in [Layering::Always, Layering::AsTaoSets] {
            let style = ex_style(CAPTURED, layering);
            assert_ne!(style & WS_EX_TOOLWINDOW.0, 0);
            assert_eq!(style & WS_EX_APPWINDOW.0, 0);
        }
    }

    #[test]
    fn always_layered_window_stays_layered_when_tao_captures_the_mouse() {
        let style = ex_style(CAPTURED, Layering::Always);

        assert_ne!(style & WS_EX_LAYERED.0, 0);
        // Only WS_EX_TRANSPARENT tells captured from released.
        assert_eq!(style & WS_EX_TRANSPARENT.0, 0);
        assert_eq!(
            ex_style(RELEASED, Layering::Always),
            style | WS_EX_TRANSPARENT.0
        );
    }

    #[test]
    fn other_windows_keep_the_layering_tao_chose() {
        assert_eq!(ex_style(CAPTURED, Layering::AsTaoSets) & WS_EX_LAYERED.0, 0);
        assert_ne!(ex_style(RELEASED, Layering::AsTaoSets) & WS_EX_LAYERED.0, 0);
    }

    #[test]
    fn only_the_dock_is_always_layered() {
        assert_eq!(dock::LAYERING, Layering::Always);
        assert_eq!(panel::LAYERING, Layering::AsTaoSets);
        for style in [CAPTURED, RELEASED] {
            assert_ne!(ex_style(style, dock::LAYERING) & WS_EX_LAYERED.0, 0);
        }
        for style in PANEL_STYLES {
            assert_eq!(
                ex_style(style, panel::LAYERING) & WS_EX_LAYERED.0,
                0,
                "panel style {style:#x}"
            );
        }
    }
}
