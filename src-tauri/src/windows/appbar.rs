//! The dock as an app bar: it reserves a band along an edge of each screen
//! it is on, so maximized windows stop at the band instead of going under
//! the dock (ADR 0009, spec 0010 D4b: one band per monitor).
//!
//! A band is full width and as thick as the bar plus its distance from the
//! edge; the dock window is smaller, centered, and taller (it holds the
//! magnification and the label), and overlays the band. Windows wants a
//! window of ours for each band to talk to, and a message loop to hear them:
//! hidden windows on a thread of their own, which never touches Tauri (the
//! main thread waits for it in `set`, so it must not wait for the main
//! thread). What it has to tell the dock, it asks of `on_change`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;

use ::windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Shell::{
    ABE_BOTTOM, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA, SHAppBarMessage,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, MSG,
    RegisterClassW, RegisterWindowMessageW, SendMessageW, TranslateMessage, WM_APP,
    WM_DISPLAYCHANGE, WM_ENDSESSION, WM_SETTINGCHANGE, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};
use ::windows::core::{PCWSTR, w};

use super::dock::{Edge, Rect};

/// Posted to a band's window by Windows about the band (`ABN_*` in
/// `wparam`).
const WM_CALLBACK: u32 = WM_APP + 2;
/// Sent by `set` to a band's window: puts the band where it is wanted.
const WM_APPLY: u32 = WM_APP + 3;
/// Sent to the thread's own window: makes a band's window (its key in
/// `lparam`, a `*const String`) and answers its handle.
const WM_NEW_BAND: u32 = WM_APP + 4;
/// Sent to a band's window: gives the band back and closes the window.
const WM_DROP_BAND: u32 = WM_APP + 5;

/// `ABN_POSCHANGED`: something else on the edge moved (the taskbar hid, a
/// bar came or went); the band is asked again.
const ABN_POSCHANGED: usize = 1;

/// The band the dock wants on a monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub edge: Edge,
    /// The monitor's rectangle, in physical pixels.
    pub monitor: Rect,
    /// Thickness of the band, in physical pixels.
    pub thickness: u32,
}

/// One monitor's band.
#[derive(Debug, Default)]
struct Band {
    /// Its window (an `HWND` as a number).
    hwnd: isize,
    wanted: Option<Request>,
    granted: Option<Rect>,
    registered: bool,
}

#[derive(Default)]
struct Shared {
    /// By monitor key.
    bands: HashMap<String, Band>,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
}

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

/// The thread's own window (an `HWND` as a number), 0 until the thread made
/// it.
static WINDOW: AtomicIsize = AtomicIsize::new(0);
/// The id Windows gave the `TaskbarCreated` message.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
/// Explorer's taskbar window when the bands were last asked for (an `HWND`
/// as a number, 0 for none).
static EXPLORER_TASKBAR: AtomicIsize = AtomicIsize::new(0);

fn with_shared<T>(change: impl FnOnce(&mut Shared) -> T) -> T {
    let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
    change(shared.get_or_insert_with(Shared::default))
}

fn window() -> Option<HWND> {
    match WINDOW.load(Ordering::Acquire) {
        0 => None,
        value => Some(HWND(value as *mut _)),
    }
}

/// Starts the thread and its window, once. `on_change` is called from that
/// thread whenever a band or the monitors changed under the dock and the
/// dock must place itself again; it must not block on the caller of `set`.
pub fn start(on_change: impl Fn() + Send + Sync + 'static) {
    if window().is_some() {
        return;
    }
    with_shared(|shared| shared.on_change = Some(Arc::new(on_change)));
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

/// Reserves the band `request` describes on the monitor `key`, or gives it
/// back with `None`; returns the band Windows granted, which can differ from
/// the one asked for (another bar may hold the edge). Waits until the thread
/// has done it.
pub fn set(key: &str, request: Option<Request>) -> Option<Rect> {
    let manager = window()?;
    let existing = with_shared(|shared| shared.bands.get(key).map(|band| band.hwnd));
    let hwnd = match existing {
        Some(hwnd) => hwnd,
        None if request.is_none() => return None,
        None => {
            let name = key.to_string();
            let made = unsafe {
                SendMessageW(
                    manager,
                    WM_NEW_BAND,
                    None,
                    Some(LPARAM(std::ptr::from_ref(&name) as isize)),
                )
            };
            if made.0 == 0 {
                return None;
            }
            made.0
        }
    };
    let unchanged = with_shared(|shared| {
        let band = shared.bands.entry(key.to_string()).or_default();
        band.hwnd = hwnd;
        if band.wanted == request && band.granted.is_some() == request.is_some() {
            return Some(band.granted);
        }
        band.wanted = request;
        None
    });
    if let Some(granted) = unchanged {
        return granted;
    }
    unsafe { SendMessageW(HWND(hwnd as _), WM_APPLY, None, None) };
    with_shared(|shared| shared.bands.get(key).and_then(|band| band.granted))
}

/// Gives back the bands of the monitors not in `keep` (they were
/// unplugged, or the dock left them).
pub fn retain(keep: &[String]) {
    let gone: Vec<isize> = with_shared(|shared| {
        let gone = shared
            .bands
            .iter()
            .filter(|(key, _)| !keep.contains(key))
            .map(|(_, band)| band.hwnd)
            .collect();
        shared.bands.retain(|key, _| keep.contains(key));
        gone
    });
    for hwnd in gone {
        unsafe { SendMessageW(HWND(hwnd as _), WM_DROP_BAND, None, None) };
    }
}

/// Gives every band back (the process is ending).
pub fn remove() {
    retain(&[]);
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
/// strip along the edge. Returns what Windows granted; `granted` is the band
/// held now, which is kept as it is when Windows offers the same again: each
/// `ABM_SETPOS` tells the other bands their edge moved, and they would answer
/// in kind, one monitor's band to the other's, without end.
fn place(hwnd: HWND, request: Request, granted: Option<Rect>) -> Rect {
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
    if granted == Some(narrow(data.rc)) {
        return narrow(data.rc);
    }
    unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
    narrow(data.rc)
}

/// The band of the window `hwnd`, as `(wanted, granted, registered)`.
fn band_of(hwnd: HWND) -> Option<(Option<Request>, Option<Rect>, bool)> {
    with_shared(|shared| {
        shared
            .bands
            .values()
            .find(|band| band.hwnd == hwnd.0 as isize)
            .map(|band| (band.wanted, band.granted, band.registered))
    })
}

fn update_band(hwnd: HWND, change: impl FnOnce(&mut Band)) {
    with_shared(|shared| {
        if let Some(band) = shared
            .bands
            .values_mut()
            .find(|band| band.hwnd == hwnd.0 as isize)
        {
            change(band);
        }
    });
}

/// Makes the band of the window `hwnd` what it wants; `drop` gives it back
/// whatever it wants. Asked of Explorer's taskbar itself: a band asked
/// through the tray host (`tray_host`) is granted but left out of the work
/// area, and maximized windows would go under the dock.
fn apply(hwnd: HWND, drop: bool) {
    super::tray_host::aside(|| apply_now(hwnd, drop));
}

fn apply_now(hwnd: HWND, drop: bool) {
    let (wanted, granted, registered) = band_of(hwnd).unwrap_or((None, None, true));
    let wanted = if drop { None } else { wanted };
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
            update_band(hwnd, |band| {
                band.registered = false;
                band.granted = None;
            });
        }
        Some(request) => {
            // Newly registered, the band is held nowhere yet.
            let held = if registered { granted } else { None };
            if !registered {
                unsafe { SHAppBarMessage(ABM_NEW, &mut data) };
                update_band(hwnd, |band| band.registered = true);
            }
            let granted = place(hwnd, request, held);
            update_band(hwnd, |band| band.granted = Some(granted));
        }
    }
}

/// Every band again, registered anew: Explorer started again and forgot
/// them.
fn apply_all() {
    let windows: Vec<isize> = with_shared(|shared| {
        shared
            .bands
            .values_mut()
            .map(|band| {
                band.registered = false;
                band.granted = None;
                band.hwnd
            })
            .collect()
    });
    for hwnd in windows {
        apply(HWND(hwnd as _), false);
    }
}

/// Whether Explorer's taskbar window is another than when last asked,
/// remembering the one there is now.
fn explorer_taskbar_changed() -> bool {
    let now = super::tray_host::explorer_taskbar().unwrap_or(0);
    EXPLORER_TASKBAR.swap(now, Ordering::Relaxed) != now
}

fn changed() {
    let callback = with_shared(|shared| shared.on_change.clone());
    if let Some(callback) = callback {
        callback();
    }
}

fn class_instance() -> HINSTANCE {
    unsafe { GetModuleHandleW(PCWSTR::null()) }
        .map(|module| HINSTANCE(module.0))
        .unwrap_or_default()
}

/// A hidden window of the thread.
fn make_window(title: PCWSTR) -> Option<HWND> {
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("DockMaxAppBar"),
            title,
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(class_instance()),
            None,
        )
    }
    .ok()
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let manager = window().is_some_and(|manager| manager == hwnd);
    match msg {
        WM_NEW_BAND if manager => {
            let made = make_window(w!("Dockyard app bar band"));
            return LRESULT(made.map_or(0, |band| band.0 as isize));
        }
        WM_APPLY => {
            apply(hwnd, false);
            return LRESULT(0);
        }
        WM_DROP_BAND => {
            apply(hwnd, true);
            let _ = unsafe { DestroyWindow(hwnd) };
            return LRESULT(0);
        }
        WM_CALLBACK => {
            if wparam.0 == ABN_POSCHANGED {
                // Explorer moved its taskbar: it may have shown it again.
                super::taskbar::keep();
                apply(hwnd, false);
                changed();
            }
            return LRESULT(0);
        }
        WM_DISPLAYCHANGE if manager => {
            super::taskbar::keep();
            changed();
        }
        WM_SETTINGCHANGE if manager => super::taskbar::keep(),
        // The session ends without `RunEvent::Exit`: the bands and the
        // taskbar go back now.
        WM_ENDSESSION if manager && wparam.0 != 0 => {
            let windows: Vec<isize> =
                with_shared(|shared| shared.bands.values().map(|band| band.hwnd).collect());
            for band in windows {
                apply(HWND(band as _), true);
            }
            super::tray_host::stop();
            super::taskbar::release();
            super::window_spaces::release_all();
        }
        // The tray host sends `TaskbarCreated` too, so that programs send
        // their icons to it: only a new taskbar window means Explorer
        // started again and forgot the bands. Asking for them lowers the
        // host for an instant, and an icon put back meanwhile (the dock's
        // own, on that very message) would miss it.
        _ if manager
            && msg == TASKBAR_CREATED.load(Ordering::Relaxed)
            && msg != 0
            && explorer_taskbar_changed() =>
        {
            super::taskbar::keep();
            apply_all();
            changed();
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// The thread: a hidden window, the bands' windows and their message loop,
/// for as long as the process lives.
fn run(ready: mpsc::Sender<()>) {
    let class_data = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: class_instance(),
        lpszClassName: w!("DockMaxAppBar"),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    TASKBAR_CREATED.store(
        unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        Ordering::Relaxed,
    );
    explorer_taskbar_changed();
    match make_window(w!("Dockyard app bar")) {
        Some(hwnd) => WINDOW.store(hwnd.0 as isize, Ordering::Release),
        None => log::warn!("appbar: the window was not created"),
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
            set(
                "monitor",
                Some(Request {
                    edge: Edge::Bottom,
                    monitor: Rect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    thickness: 10,
                })
            ),
            None
        );
    }
}
