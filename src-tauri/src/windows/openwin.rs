//! The windows the user has open (spec 0009): which of the top-level windows
//! the taskbar would give a button, and who owns each. Reads only; it never
//! changes a window.

#[cfg(windows)]
use std::path::PathBuf;

/// One window with a taskbar button.
#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWindow {
    pub hwnd: isize,
    pub pid: u32,
    pub title: String,
    /// The AppID the program set for the window (`System.AppUserModel.ID`),
    /// which is how the shell groups buttons; `None` when it set none.
    pub aumid: Option<String>,
    /// The program's file (the process image).
    pub exe: Option<PathBuf>,
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;

    use ::windows::Win32::Foundation::{
        CloseHandle, ERROR_SUCCESS, HWND, LPARAM, MAX_PATH, PROPERTYKEY, WPARAM,
    };
    use ::windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use ::windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
    use ::windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use ::windows::Win32::UI::Shell::PropertiesSystem::{
        IPropertyStore, SHGetPropertyStoreForWindow,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GA_ROOTOWNER, GW_OWNER, GWL_EXSTYLE, GetAncestor,
        GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW, SW_MINIMIZE,
        SW_RESTORE, SetForegroundWindow, ShowWindow, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW,
    };
    use ::windows::core::{BOOL, PWSTR};

    use super::OpenWindow;
    use crate::domain::fullscreen::{Bounds, Candidate};
    use ::windows::Win32::Foundation::RECT;
    use ::windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetClassNameW, GetWindowRect, IsZoomed, WS_CAPTION,
    };

    /// `System.AppUserModel.ID`.
    const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
        fmtid: ::windows::core::GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
        pid: 5,
    };

    fn is_cloaked(hwnd: HWND) -> bool {
        let mut cloaked = 0u32;
        let read = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut _,
                size_of::<u32>() as u32,
            )
        };
        read.is_ok() && cloaked != 0
    }

    /// Whether the taskbar would give `hwnd` a button: visible, not a tool
    /// window, not owned by another (a dialog), not hidden by the shell (a
    /// window on another virtual desktop, a suspended Store app), not one
    /// that never takes the focus.
    fn has_button(hwnd: HWND) -> bool {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return false;
        }
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
        let app_window = style & WS_EX_APPWINDOW.0 != 0;
        if style & WS_EX_TOOLWINDOW.0 != 0 && !app_window {
            return false;
        }
        if style & WS_EX_NOACTIVATE.0 != 0 && !app_window {
            return false;
        }
        let owned = unsafe { GetWindow(hwnd, GW_OWNER) }.is_ok_and(|owner| !owner.is_invalid());
        if owned && !app_window {
            return false;
        }
        !is_cloaked(hwnd)
    }

    fn title_of(hwnd: HWND) -> String {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let read = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..read.max(0) as usize])
    }

    pub(super) fn exe_of(pid: u32) -> Option<PathBuf> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buffer = vec![0u16; MAX_PATH as usize * 4];
        let mut length = buffer.len() as u32;
        let read = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };
        let _ = unsafe { CloseHandle(process) };
        read.ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..length as usize],
        )))
    }

    /// The AppID the window sets itself, or, for a packaged (Store) app, the
    /// one of its package.
    fn aumid_of(hwnd: HWND, pid: u32) -> Option<String> {
        let own = unsafe { SHGetPropertyStoreForWindow::<IPropertyStore>(hwnd) }
            .ok()
            .and_then(|store| unsafe { store.GetValue(&PKEY_APP_USER_MODEL_ID) }.ok())
            .map(|value| value.to_string())
            .filter(|text| !text.is_empty());
        own.or_else(|| packaged_aumid_of(pid))
    }

    fn packaged_aumid_of(pid: u32) -> Option<String> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut length = 256u32;
        let mut buffer = vec![0u16; length as usize];
        let status = unsafe {
            GetApplicationUserModelId(process, &mut length, Some(PWSTR(buffer.as_mut_ptr())))
        };
        let _ = unsafe { CloseHandle(process) };
        if status != ERROR_SUCCESS || length == 0 {
            return None;
        }
        // `length` counts the terminating null.
        Some(String::from_utf16_lossy(&buffer[..(length as usize - 1)]))
    }

    unsafe extern "system" fn collect(hwnd: HWND, state: LPARAM) -> BOOL {
        let found = unsafe { &mut *(state.0 as *mut Vec<OpenWindow>) };
        if !has_button(hwnd) {
            return true.into();
        }
        let title = title_of(hwnd);
        if title.is_empty() {
            return true.into();
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == unsafe { GetCurrentProcessId() } {
            return true.into();
        }
        found.push(OpenWindow {
            hwnd: hwnd.0 as isize,
            pid,
            title,
            aumid: aumid_of(hwnd, pid),
            exe: exe_of(pid),
        });
        true.into()
    }

    /// The shell's own windows, which cover a monitor without being a
    /// program in full screen: the desktop and the taskbars.
    const SHELL_CLASSES: [&str; 4] = [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
    ];

    fn class_of(hwnd: HWND) -> String {
        let mut buffer = [0u16; 64];
        let read = unsafe { GetClassNameW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..read.max(0) as usize])
    }

    unsafe extern "system" fn collect_candidate(hwnd: HWND, state: LPARAM) -> BOOL {
        let found = unsafe { &mut *(state.0 as *mut Vec<Candidate>) };
        if !has_button(hwnd) || unsafe { IsIconic(hwnd) }.as_bool() {
            return true.into();
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == unsafe { GetCurrentProcessId() } {
            return true.into();
        }
        if SHELL_CLASSES.contains(&class_of(hwnd).as_str()) {
            return true.into();
        }
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
            return true.into();
        }
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        found.push(Candidate {
            monitor: monitor.0 as isize,
            bounds: Bounds {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            },
            maximized: unsafe { IsZoomed(hwnd) }.as_bool() && style & WS_CAPTION.0 == WS_CAPTION.0,
        });
        true.into()
    }

    /// The windows that could be in full screen, top first.
    pub fn candidates() -> Vec<Candidate> {
        let mut found: Vec<Candidate> = Vec::new();
        let _ = unsafe {
            EnumWindows(
                Some(collect_candidate),
                LPARAM(&mut found as *mut Vec<Candidate> as isize),
            )
        };
        found
    }

    /// The window the user is working in: the one in front, or, if that is
    /// a dialog, the window that owns it.
    pub fn foreground() -> Option<isize> {
        let front = unsafe { GetForegroundWindow() };
        if front.is_invalid() {
            return None;
        }
        let root = unsafe { GetAncestor(front, GA_ROOTOWNER) };
        Some(if root.is_invalid() { front.0 } else { root.0 } as isize)
    }

    /// Asks the window to close, as its close button does: the program may
    /// ask to save first or refuse.
    pub fn close(hwnd: isize) {
        let _ = unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }

    /// Whether the window still exists.
    pub fn exists(hwnd: isize) -> bool {
        unsafe { IsWindow(Some(HWND(hwnd as _))) }.as_bool()
    }

    pub fn minimize(hwnd: isize) {
        let _ = unsafe { ShowWindow(HWND(hwnd as _), SW_MINIMIZE) };
    }

    /// Brings the window to the front, restoring it first if it is
    /// minimized. Windows lets a program take the front only if it has had
    /// the last input; the dock does not take the focus, so when the plain
    /// call is refused the input of the window in front is shared for the
    /// instant it takes.
    pub fn focus(hwnd: isize) {
        let window = HWND(hwnd as _);
        if unsafe { IsIconic(window) }.as_bool() {
            let _ = unsafe { ShowWindow(window, SW_RESTORE) };
        }
        if unsafe { SetForegroundWindow(window) }.as_bool() {
            return;
        }
        let front = unsafe { GetForegroundWindow() };
        let front_thread = unsafe { GetWindowThreadProcessId(front, None) };
        let mine = unsafe { GetCurrentThreadId() };
        if front_thread == 0 || front_thread == mine {
            return;
        }
        let _ = unsafe { AttachThreadInput(mine, front_thread, true) };
        let _ = unsafe { BringWindowToTop(window) };
        let _ = unsafe { SetForegroundWindow(window) };
        let _ = unsafe { AttachThreadInput(mine, front_thread, false) };
    }

    /// The windows with a taskbar button, from the top of the stacking order
    /// down (the order `EnumWindows` gives).
    pub fn enumerate() -> Vec<OpenWindow> {
        let mut found: Vec<OpenWindow> = Vec::new();
        let _ = unsafe {
            EnumWindows(
                Some(collect),
                LPARAM(&mut found as *mut Vec<OpenWindow> as isize),
            )
        };
        found
    }
}

/// The windows with a taskbar button, topmost first.
#[cfg(windows)]
pub fn enumerate() -> Vec<OpenWindow> {
    imp::enumerate()
}

/// The window the user is working in (its owner root, for a dialog).
#[cfg(windows)]
pub fn foreground() -> Option<isize> {
    imp::foreground()
}

/// Asks the window `hwnd` to close.
#[cfg(windows)]
pub fn close(hwnd: isize) {
    imp::close(hwnd);
}

/// The windows that could be a program in full screen (`domain::fullscreen`),
/// top first.
#[cfg(windows)]
pub fn fullscreen_candidates() -> Vec<crate::domain::fullscreen::Candidate> {
    imp::candidates()
}

/// The file of the program that owns the window `hwnd`.
#[cfg(windows)]
pub fn exe_of_window(hwnd: isize) -> Option<PathBuf> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as _), Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    imp::exe_of(pid)
}

/// Whether the window `hwnd` still exists.
#[cfg(windows)]
pub fn exists(hwnd: isize) -> bool {
    imp::exists(hwnd)
}

/// Minimizes the window `hwnd`.
#[cfg(windows)]
pub fn minimize(hwnd: isize) {
    imp::minimize(hwnd);
}

/// Brings the window `hwnd` to the front, restoring it if it is minimized.
#[cfg(windows)]
pub fn focus(hwnd: isize) {
    imp::focus(hwnd);
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// Not a check: prints what is open on this machine, to see what the
    /// shell would call each window. `cargo test spike_ -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the windows of the machine"]
    fn spike_matches_windows_to_the_catalog() {
        let catalog = crate::windows::apps::catalog().unwrap();
        let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
        for window in enumerate() {
            let mut how: Vec<String> = Vec::new();
            for app in &catalog {
                if let Some(aumid) = &window.aumid {
                    if same(aumid, &app.id) {
                        how.push(format!("id==aumid: {} [{}]", app.name, app.id));
                    }
                    if crate::windows::apps::app_user_model_id(&app.id)
                        .is_some_and(|own| same(&own, aumid))
                    {
                        how.push(format!("appid==aumid: {} [{}]", app.name, app.id));
                    }
                }
                if let (Some(exe), Some(target)) =
                    (&window.exe, crate::windows::apps::target_of(&app.id))
                    && same(&exe.to_string_lossy(), &target.to_string_lossy())
                {
                    how.push(format!("exe==target: {} [{}]", app.name, app.id));
                }
            }
            println!(
                "{} | aumid={} | {}\n    {}",
                window.title.chars().take(40).collect::<String>(),
                window.aumid.as_deref().unwrap_or("-"),
                window
                    .exe
                    .as_deref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                if how.is_empty() {
                    "(no match)".to_owned()
                } else {
                    how.join("\n    ")
                }
            );
        }
    }

    #[test]
    #[ignore = "reads the windows of the machine"]
    fn spike_lists_the_fullscreen_candidates() {
        println!("monitors: {:?}", super::super::monitors::all());
        for candidate in fullscreen_candidates().iter().take(8) {
            println!("{candidate:?}");
        }
    }

    #[test]
    #[ignore = "reads the windows of the machine"]
    fn spike_lists_the_open_windows() {
        for window in enumerate() {
            println!(
                "{:>10} pid={:<6} aumid={:<60} exe={:<70} title={}",
                window.hwnd,
                window.pid,
                window.aumid.as_deref().unwrap_or("-"),
                window
                    .exe
                    .as_deref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "-".into()),
                window.title
            );
        }
    }
}
