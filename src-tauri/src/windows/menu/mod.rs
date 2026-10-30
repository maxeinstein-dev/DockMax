//! The Dockyard's own menus (spec 0014 R3): a native window drawn with
//! Direct2D, as the launcher, in place of Windows' menu (`TrackPopupMenu`),
//! which can't have the look of the rest of the app.
//!
//! `show` works as `TrackPopupMenu` does: it opens the menu, runs a loop of
//! its own until something is chosen or the menu is let go (Esc, Alt, a
//! click elsewhere), and returns what was chosen. The pointer, the keyboard
//! (arrows, Home, End, Enter, Space, a letter) and UI Automation choose.
//! On the main thread only, one menu at a time.

mod access;
mod paint;

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VIRTUAL_KEY, VK_DOWN, VK_END, VK_ESCAPE,
    VK_F10, VK_HOME, VK_MENU, VK_RETURN, VK_SPACE, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    HWND_TOPMOST, IDC_ARROW, LoadCursorW, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    SW_SHOW, SWP_NOACTIVATE, SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage,
    WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_ERASEBKGND, WM_GETOBJECT, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_NULL, WM_PAINT, WM_RBUTTONUP, WM_SYSKEYDOWN, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::domain::menu::{self as model, Area};
pub use crate::domain::menu::{Item, Line};
use crate::windows::launcher::layout::{self as look, Colors};
use windows::Win32::Graphics::Gdi::{
    COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT,
};

/// The window class of the menus: `menu_on_screen` (the dock stays up under
/// a menu) looks for it.
pub const CLASS: PCWSTR = w!("DockMaxMenu");

/// UI Automation chose the line in `wparam`, or put the focus on it.
const WM_MENU_INVOKE: u32 = WM_APP + 0x61;
const WM_MENU_FOCUS: u32 = WM_APP + 0x62;
/// `WM_MOUSELEAVE`, which the features in use do not name.
const WM_MOUSELEAVE: u32 = 0x02A3;

struct Menu {
    hwnd: HWND,
    lines: Vec<Line>,
    /// The line under the pointer or the keyboard.
    hot: Option<usize>,
    chosen: Option<usize>,
    done: bool,
    /// Physical pixels per logical one.
    scale: f64,
    /// Logical pixels.
    width: f64,
    colors: Colors,
    painter: paint::Painter,
}

thread_local! {
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

/// Runs `change` on the open menu, if any. Nothing in it may send a message
/// to the menu's window (that would come back here while it is borrowed).
fn with_menu<T>(change: impl FnOnce(&mut Menu) -> T) -> Option<T> {
    MENU.with(|slot| slot.borrow_mut().as_mut().map(change))
}

fn register() {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&class);
    });
}

/// The colors of the menu: the launcher's, Windows' own in contrast mode.
fn colors() -> (Colors, bool) {
    let dark = super::native_dock::system_dark();
    if super::launcher::high_contrast() {
        let c = super::launcher::system_color;
        (
            look::contrast_colors(
                c(COLOR_WINDOW),
                c(COLOR_WINDOWTEXT),
                c(COLOR_GRAYTEXT),
                c(COLOR_HIGHLIGHT),
                c(COLOR_HIGHLIGHTTEXT),
            ),
            false,
        )
    } else {
        (look::colors(dark), dark)
    }
}

/// The work area of the monitor at `at`, and the menu's scale there.
fn monitor_at(at: POINT) -> (Area, f64) {
    unsafe {
        let monitor = MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let work = info.rcWork;
        let (mut dpi, mut _y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut _y);
        (
            Area {
                x: work.left,
                y: work.top,
                width: work.right - work.left,
                height: work.bottom - work.top,
            },
            // Windows' "Text size" too, as the launcher.
            f64::from(dpi.max(96)) / 96.0 * super::launcher::text_scale(),
        )
    }
}

/// Opens a menu of `lines` at `at` (the pointer when `None`), above it when
/// `up` (a dock at the bottom of the screen) and below it otherwise, and
/// returns the index of the line chosen; `None` when it was let go.
pub fn show(at: Option<POINT>, up: bool, lines: Vec<Line>) -> Option<usize> {
    if lines.is_empty() || MENU.with(|slot| slot.borrow().is_some()) {
        return None;
    }
    register();
    let at = at.unwrap_or_else(|| {
        let mut point = POINT::default();
        let _ = unsafe { GetCursorPos(&mut point) };
        point
    });
    let (work, scale) = monitor_at(at);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS,
            w!("Dockyard"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            GetModuleHandleW(None).ok().map(Into::into),
            None,
        )
    }
    .inspect_err(|error| log::warn!("menu: the window could not be made: {error}"))
    .ok()?;
    let (colors, dark) = colors();
    unsafe {
        let corners = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corners).cast(),
            size_of_val(&corners) as u32,
        );
        let dark = windows::core::BOOL::from(dark);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::from_ref(&dark).cast(),
            size_of_val(&dark) as u32,
        );
    }
    let Ok(mut painter) = paint::Painter::new(hwnd, scale) else {
        let _ = unsafe { DestroyWindow(hwnd) };
        return None;
    };
    let (mut widest_label, mut widest_shortcut) = (0.0f64, 0.0f64);
    for line in &lines {
        match line {
            Line::Item(item) => {
                widest_label = widest_label.max(painter.measure(&item.label, paint::LABEL_SIZE));
                if let Some(keys) = &item.shortcut {
                    widest_shortcut =
                        widest_shortcut.max(painter.measure(keys, paint::SHORTCUT_SIZE));
                }
            }
            Line::Title(title) => {
                widest_label = widest_label.max(painter.measure(title, paint::SHORTCUT_SIZE));
            }
            Line::Separator => {}
        }
    }
    let width = model::width(widest_label, widest_shortcut);
    let height = model::height(&lines);
    let px = |logical: f64| (logical * scale).round() as i32;
    let area = model::place(at.x, at.y, px(width), px(height), up, work);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            area.x,
            area.y,
            area.width,
            area.height,
            SWP_NOACTIVATE,
        );
    }
    let entries = model::tops(&lines)
        .into_iter()
        .zip(&lines)
        .map(|(top, line)| {
            let tall = match line {
                Line::Item(_) => model::ITEM,
                Line::Separator => model::SEPARATOR,
                Line::Title(_) => model::TITLE,
            };
            access::Entry {
                line: line.clone(),
                rect: (
                    f64::from(area.x),
                    f64::from(area.y) + top * scale,
                    f64::from(area.width),
                    tall * scale,
                ),
            }
        })
        .collect();
    MENU.with(|slot| {
        *slot.borrow_mut() = Some(Menu {
            hwnd,
            lines,
            hot: None,
            chosen: None,
            done: false,
            scale,
            width,
            colors,
            painter,
        });
    });
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        // A menu takes the keyboard: the click that opened it gives the app
        // the right to come to the front.
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(hwnd));
    }
    access::opened(hwnd, entries);
    run_loop();
    let chosen = MENU
        .with(|slot| slot.borrow_mut().take())
        .and_then(|menu| menu.chosen);
    access::closed(hwnd);
    let _ = unsafe { DestroyWindow(hwnd) };
    chosen
}

/// The menu's own loop: every window's messages go on, as under Windows'
/// menus, until the menu is done.
fn run_loop() {
    let mut message = MSG::default();
    loop {
        if with_menu(|menu| menu.done).unwrap_or(true) {
            return;
        }
        let got = unsafe { GetMessageW(&mut message, None, 0, 0) };
        match got.0 {
            // WM_QUIT: the app is ending; the loop above must see it too.
            0 => {
                unsafe { PostQuitMessage(message.wParam.0 as i32) };
                with_menu(|menu| menu.done = true);
                return;
            }
            -1 => return,
            _ => unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            },
        }
    }
}

impl Menu {
    fn finish(&mut self, chosen: Option<usize>) {
        self.chosen = chosen.filter(
            |&index| matches!(self.lines.get(index), Some(Line::Item(item)) if item.enabled),
        );
        self.done = true;
        // Wakes the loop if it is waiting.
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };
    }

    /// Puts the highlight on `index`; tells UI Automation when it was the
    /// keyboard.
    fn set_hot(&mut self, index: Option<usize>, keyboard: bool) -> Option<Option<usize>> {
        if self.hot == index {
            return None;
        }
        self.hot = index;
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
        keyboard.then_some(index)
    }

    fn line_at(&self, lparam: LPARAM) -> Option<usize> {
        let y = f64::from((lparam.0 >> 16) as i16) / self.scale;
        model::item_at(&self.lines, y)
    }

    fn key(&mut self, key: VIRTUAL_KEY) -> Option<Option<usize>> {
        let lines = &self.lines;
        match key {
            VK_DOWN => {
                let next = model::step(lines, self.hot, true);
                self.set_hot(next, true)
            }
            VK_UP => {
                let next = model::step(lines, self.hot, false);
                self.set_hot(next, true)
            }
            VK_HOME => {
                let next = model::first(lines);
                self.set_hot(next, true)
            }
            VK_END => {
                let next = model::last(lines);
                self.set_hot(next, true)
            }
            VK_RETURN | VK_SPACE => {
                if self.hot.is_some() {
                    self.finish(self.hot);
                }
                None
            }
            VK_ESCAPE => {
                self.finish(None);
                None
            }
            _ => None,
        }
    }

    fn paint(&mut self) {
        let scene = paint::Scene {
            lines: &self.lines,
            hot: self.hot,
            colors: self.colors,
            width: self.width,
        };
        if !self.painter.paint(&scene) {
            let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
        }
    }
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // What UI Automation hears after the menu is let go of.
    let mut focus: Option<Option<usize>> = None;
    let handled = match message {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            unsafe { BeginPaint(hwnd, &mut paint) };
            with_menu(Menu::paint);
            let _ = unsafe { EndPaint(hwnd, &paint) };
            true
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_MOUSEMOVE => {
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = unsafe { TrackMouseEvent(&mut track) };
            // The screen reader follows the pointer too, as in Windows' menus.
            focus = with_menu(|menu| {
                let at = menu.line_at(lparam);
                menu.set_hot(at, at.is_some())
            })
            .flatten();
            true
        }
        WM_MOUSELEAVE => {
            with_menu(|menu| menu.set_hot(None, false));
            true
        }
        WM_LBUTTONUP | WM_RBUTTONUP => {
            with_menu(|menu| {
                if let Some(at) = menu.line_at(lparam) {
                    menu.finish(Some(at));
                }
            });
            true
        }
        WM_KEYDOWN => {
            focus = with_menu(|menu| menu.key(VIRTUAL_KEY(wparam.0 as u16))).flatten();
            true
        }
        WM_SYSKEYDOWN => {
            let key = VIRTUAL_KEY(wparam.0 as u16);
            if key == VK_MENU || key == VK_F10 {
                with_menu(|menu| menu.finish(None));
                true
            } else {
                false
            }
        }
        WM_CHAR => {
            let typed = char::from_u32(wparam.0 as u32).filter(|c| c.is_alphanumeric());
            if let Some(typed) = typed {
                focus = with_menu(|menu| {
                    let (index, unique) = model::by_letter(&menu.lines, menu.hot, typed)?;
                    let moved = menu.set_hot(Some(index), true);
                    if unique {
                        menu.finish(Some(index));
                    }
                    moved
                })
                .flatten();
            }
            true
        }
        WM_ACTIVATE => {
            // A click anywhere else (or Alt+Tab) lets the menu go.
            if u32::from(wparam.0 as u16) == WA_INACTIVE {
                with_menu(|menu| menu.finish(None));
            }
            true
        }
        WM_MENU_INVOKE => {
            with_menu(|menu| menu.finish(Some(wparam.0)));
            true
        }
        WM_MENU_FOCUS => {
            focus = with_menu(|menu| menu.set_hot(Some(wparam.0), true)).flatten();
            true
        }
        WM_GETOBJECT => {
            if let Some(result) = access::get_object(hwnd, wparam, lparam) {
                return result;
            }
            false
        }
        _ => false,
    };
    if let Some(index) = focus {
        access::focused(hwnd, index);
    }
    if handled {
        LRESULT(0)
    } else {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }
}
