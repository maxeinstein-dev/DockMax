//! The dock as an app bar: it reserves a band along an edge of the screen,
//! so maximized windows stop at the band instead of going under the dock
//! (ADR 0009).
//!
//! The band is full width and as thick as the bar plus its distance from the
//! edge; the dock window is smaller, centered, and taller (it holds the
//! magnification and the label), and overlays the band. Windows wants a
//! window of ours to talk to and a message loop to hear it: a hidden window
//! on a thread of its own, which never touches Tauri (the main thread waits
//! for it in `set`, so it must not wait for the main thread). What it has to
//! tell the dock, it asks of `on_change`.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;

use ::windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Shell::{
    ABE_BOTTOM, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA, SHAppBarMessage,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
    RegisterWindowMessageW, SendMessageW, TranslateMessage, WM_APP, WM_DISPLAYCHANGE,
    WM_ENDSESSION, WM_SETTINGCHANGE, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use ::windows::core::{PCWSTR, w};

use super::dock::{Edge, Rect};

/// Posted to the hidden window by Windows about the band (`ABN_*` in
/// `wparam`).
const WM_CALLBACK: u32 = WM_APP + 2;
/// Sent by `set`: puts the band where `Shared::wanted` says.
const WM_APPLY: u32 = WM_APP + 3;

/// `ABN_POSCHANGED`: something else on the edge moved (the taskbar hid, a
/// bar came or went); the band is asked again.
const ABN_POSCHANGED: usize = 1;

/// The band the dock wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub edge: Edge,
    /// The monitor's rectangle, in physical pixels.
    pub monitor: Rect,
    /// Thickness of the band, in physical pixels.
    pub thickness: u32,
}

#[derive(Default)]
struct Shared {
    wanted: Option<Request>,
    granted: Option<Rect>,
    registered: bool,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    wanted: None,
    granted: None,
    registered: false,
    on_change: None,
});

/// The hidden window (an `HWND` as a number), 0 until the thread made it.
static WINDOW: AtomicIsize = AtomicIsize::new(0);
/// The id Windows gave the `TaskbarCreated` message.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

fn window() -> Option<HWND> {
    match WINDOW.load(Ordering::Acquire) {
        0 => None,
        value => Some(HWND(value as *mut _)),
    }
}

/// Starts the thread and its window, once. `on_change` is called from that
/// thread whenever the band or the monitors changed under the dock and the
/// dock must place itself again; it must not block on the caller of `set`.
pub fn start(on_change: impl Fn() + Send + Sync + 'static) {
    if window().is_some() {
        return;
    }
    shared().on_change = Some(Arc::new(on_change));
    let (ready, started) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("appbar".into())
        .spawn(move || run(ready));
    match spawned {
        // Waits for the window, so `set` finds it.
        Ok(_) => {
            let _ = started.recv();
        }
        Err(error) => log::warn!("appbar: the thread did not start: {error}"),
    }
}

/// Reserves the band `request` describes, or gives it back with `None`;
/// returns the band Windows granted, which can differ from the one asked
/// for (another bar may hold the edge). Waits until the thread has done it.
pub fn set(request: Option<Request>) -> Option<Rect> {
    let hwnd = window()?;
    {
        let mut state = shared();
        if state.wanted == request && state.granted.is_some() == request.is_some() {
            return state.granted;
        }
        state.wanted = request;
    }
    unsafe { SendMessageW(hwnd, WM_APPLY, None, None) };
    shared().granted
}

/// Gives the band back (the process is ending).
pub fn remove() {
    set(None);
}

fn wide(rect: Rect) -> RECT {
    RECT {
        left: rect.x,
        top: rect.y,
        right: super::dock::saturate(i64::from(rect.x) + i64::from(rect.width)),
        bottom: super::dock::saturate(i64::from(rect.y) + i64::from(rect.height)),
    }
}

fn narrow(rect: RECT) -> Rect {
    Rect {
        x: rect.left,
        y: rect.top,
        width: u32::try_from(rect.right - rect.left).unwrap_or(0),
        height: u32::try_from(rect.bottom - rect.top).unwrap_or(0),
    }
}

/// The band as Windows keeps it, to be asked for again: the full-width
/// strip along the edge. Returns what Windows granted.
fn place(hwnd: HWND, request: Request) -> Rect {
    let edge = match request.edge {
        Edge::Bottom => ABE_BOTTOM,
        Edge::Top => ABE_TOP,
    };
    let monitor = wide(request.monitor);
    let thickness = i32::try_from(request.thickness).unwrap_or(i32::MAX);
    let mut data = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uEdge: edge,
        rc: monitor,
        ..Default::default()
    };
    // Windows moves the side away from the edge past any bar already there;
    // the thickness is then set from the edge that stays.
    unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut data) };
    match request.edge {
        Edge::Bottom => data.rc.top = data.rc.bottom - thickness,
        Edge::Top => data.rc.bottom = data.rc.top + thickness,
    }
    unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
    narrow(data.rc)
}

/// Makes the band what `Shared::wanted` says.
fn apply(hwnd: HWND) {
    let (wanted, registered) = {
        let state = shared();
        (state.wanted, state.registered)
    };
    let mut data = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: WM_CALLBACK,
        ..Default::default()
    };
    match wanted {
        None => {
            if registered {
                unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
            }
            let mut state = shared();
            state.registered = false;
            state.granted = None;
        }
        Some(request) => {
            if !registered {
                unsafe { SHAppBarMessage(ABM_NEW, &mut data) };
                shared().registered = true;
            }
            let granted = place(hwnd, request);
            shared().granted = Some(granted);
        }
    }
}

fn changed() {
    let callback = shared().on_change.clone();
    if let Some(callback) = callback {
        callback();
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APPLY => {
            apply(hwnd);
            return LRESULT(0);
        }
        WM_CALLBACK => {
            if wparam.0 == ABN_POSCHANGED {
                // Explorer moved its taskbar: it may have shown it again.
                super::taskbar::keep();
                apply(hwnd);
                changed();
            }
            return LRESULT(0);
        }
        WM_DISPLAYCHANGE => {
            super::taskbar::keep();
            changed();
        }
        WM_SETTINGCHANGE => super::taskbar::keep(),
        // The session ends without `RunEvent::Exit`: the band and the
        // taskbar go back now.
        WM_ENDSESSION if wparam.0 != 0 => {
            shared().wanted = None;
            apply(hwnd);
            super::tray_host::stop();
            super::taskbar::release();
        }
        _ if msg == TASKBAR_CREATED.load(Ordering::Relaxed) && msg != 0 => {
            // Explorer started again and showed its taskbar.
            super::taskbar::keep();
            apply(hwnd);
            changed();
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// The thread: a hidden window and its message loop, for as long as the
/// process lives.
fn run(ready: mpsc::Sender<()>) {
    let class = w!("DockMaxAppBar");
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }
        .map(|module| HINSTANCE(module.0))
        .unwrap_or_default();
    let class_data = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: instance,
        lpszClassName: class,
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    TASKBAR_CREATED.store(
        unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        Ordering::Relaxed,
    );
    let created = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("DockMax app bar"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    };
    match created {
        Ok(hwnd) => WINDOW.store(hwnd.0 as isize, Ordering::Release),
        Err(error) => log::warn!("appbar: the window was not created: {error}"),
    }
    let _ = ready.send(());
    if window().is_none() {
        return;
    }
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// The band to ask for: `monitor`'s full width, `bar_height` plus the
/// distance from the edge thick (physical pixels).
pub fn band_thickness(bar_height: f64, bottom_space: f64, scale_factor: f64) -> u32 {
    let thickness = ((bar_height + bottom_space) * scale_factor).round();
    thickness.clamp(1.0, f64::from(u16::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_band_is_the_bar_plus_the_distance_from_the_edge() {
        // A 62 px bar and 2 px from the edge, at 100 % and at 150 %.
        assert_eq!(band_thickness(62.0, 2.0, 1.0), 64);
        assert_eq!(band_thickness(62.0, 2.0, 1.5), 96);
        // The default distance of 8 px.
        assert_eq!(band_thickness(62.0, 8.0, 1.0), 70);
    }

    #[test]
    fn the_band_is_never_empty_nor_absurd() {
        assert_eq!(band_thickness(0.0, 0.0, 1.0), 1);
        assert_eq!(band_thickness(1e12, 0.0, 1.0), u32::from(u16::MAX));
    }

    #[test]
    fn rects_convert_both_ways() {
        let rect = Rect {
            x: -1920,
            y: 6,
            width: 1920,
            height: 1200,
        };

        assert_eq!(narrow(wide(rect)), rect);
        let raw = wide(rect);
        assert_eq!(
            (raw.left, raw.top, raw.right, raw.bottom),
            (-1920, 6, 0, 1206)
        );
    }

    #[test]
    fn asking_for_a_band_without_the_thread_does_nothing() {
        // No `start` in the tests: no window, nothing to ask.
        assert_eq!(
            set(Some(Request {
                edge: Edge::Bottom,
                monitor: Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
                thickness: 10,
            })),
            None
        );
    }
}
