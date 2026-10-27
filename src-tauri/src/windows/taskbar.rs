//! The Windows taskbar, handed over to the dock and always given back
//! (ADR 0009).
//!
//! Two things of the taskbar change while the dock takes the edge: its
//! auto-hide flag, which Windows keeps in the user's settings (it outlives
//! the process, and a reboot), and its windows, made see-through and deaf to
//! the mouse (`WS_EX_LAYERED` with an alpha of 0, and `WS_EX_TRANSPARENT`).
//! They stay shown so that UI Automation still finds their buttons, where
//! the notification badges are read (spec 0014): a window hidden with
//! `SW_HIDE` has none. Both are restored on every way out the dock has:
//!
//! 1. the normal exit and the end of the Windows session (`release`);
//! 2. a panic (a hook, see `lib.rs`);
//! 3. the process killed, which no code of the process survives: a watchdog,
//!    a second copy of this executable that does nothing but wait for the
//!    process to end and then restore (`watch`);
//! 4. everything else (the watchdog killed too, a power cut): the next start
//!    finds the state file and restores before anything else (`recover`).
//!
//! The state file is written before the first change, never after, and holds
//! what the taskbar was, so a restore is the same whoever runs it, and
//! running it twice does nothing.

use std::error::Error;
use std::ffi::c_void;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use ::windows::Win32::Foundation::{COLORREF, CloseHandle, HWND, LPARAM};
use ::windows::Win32::Graphics::Gdi::{
    RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RedrawWindow,
};
use ::windows::Win32::System::Threading::{
    GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, WaitForSingleObject,
};
use ::windows::Win32::UI::Shell::{
    ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetClassNameW, GetLayeredWindowAttributes, GetWindowLongPtrW,
    GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOPMOST, IsWindow, IsWindowVisible,
    LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_LAYERED,
    WS_EX_TRANSPARENT,
};
use ::windows::core::BOOL;
use serde::{Deserialize, Serialize};

use crate::domain::settings::TaskbarMode;

/// Name of the state file, in the app's local data folder.
pub const STATE_FILE: &str = "taskbar-state.json";

/// Argument that makes this executable the watchdog: `--taskbar-watchdog
/// <pid> <state file>`.
pub const WATCHDOG_FLAG: &str = "--taskbar-watchdog";

/// Argument that restores the taskbar and exits: `--restore-taskbar <state
/// file>`. For the user, when everything else failed.
pub const RESTORE_FLAG: &str = "--restore-taskbar";

const CLASSES: [&str; 2] = ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

/// `STILL_ACTIVE`, the exit code of a process that has not ended.
const STILL_ACTIVE: u32 = 259;

/// What the taskbar was before the dock touched it, and who touched it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    /// The process that changed the taskbar: only it, or its watchdog,
    /// gives it back.
    pub pid: u32,
    /// Whether the taskbar hid itself on its own.
    pub autohide: bool,
    /// The taskbar windows hidden with `SW_HIDE` (handles, as numbers), as
    /// the versions before spec 0014 hid them: kept so that the file one of
    /// them left still restores.
    #[serde(default)]
    pub hidden: Vec<isize>,
    /// The taskbar windows this process made see-through, with what they
    /// were.
    #[serde(default)]
    pub faded: Vec<Faded>,
}

/// A taskbar window made see-through, and what it was before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faded {
    /// The window's handle, as a number.
    pub hwnd: isize,
    /// Its extended style (`GWL_EXSTYLE`).
    pub ex_style: isize,
    /// Its layered attributes, when it had some: the key color, the alpha
    /// and the flags (`LWA_*`).
    pub layered: Option<(u32, u8, u32)>,
}

/// What a see-through taskbar window has on top of its own style.
const FADED_STYLE: isize = (WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as isize;

impl Saved {
    fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec(self).map_err(io::Error::other)?)?;
        fs::rename(&temp, path)
    }

    /// The saved state, or `None` when there is none (or it can't be read:
    /// an unreadable file restores nothing, and is left for the user).
    fn read(path: &Path) -> Option<Saved> {
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }
}

/// What this run holds of the taskbar, while it does: the state file, which
/// the panic hook and the exit read, and how the taskbar is held, which
/// `keep` repeats.
#[derive(Debug, Clone)]
struct Active {
    path: PathBuf,
    mode: TaskbarMode,
}

static ACTIVE: Mutex<Option<Active>> = Mutex::new(None);

fn active() -> Option<Active> {
    ACTIVE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Whether this run holds the taskbar now.
pub fn is_held() -> bool {
    active().is_some()
}

/// Holds the taskbar again the way this run holds it, when Windows or
/// Explorer undid some of it: hides the windows that came back (and turns the
/// hiding flag on again). Does nothing when nothing came back, so the
/// changes it may itself cause end it, and when this run holds no taskbar.
pub fn keep() {
    let Some(Active { path, mode }) = active() else {
        return;
    };
    let windows = taskbar_windows();
    match mode {
        TaskbarMode::Hidden => {
            let showing = windows
                .iter()
                .any(|&hwnd| unsafe { IsWindowVisible(hwnd) }.as_bool() && !is_faded(hwnd));
            let autohide = flags() & ABS_AUTOHIDE != 0;
            if !showing && autohide {
                return;
            }
            let Some(mut saved) =
                Saved::read(&path).filter(|saved| saved.pid == std::process::id())
            else {
                return;
            };
            set_autohide(true, windows.first().copied());
            if let Err(error) = fade_visible(&windows, &mut saved, &path) {
                log::warn!("taskbar: could not be kept hidden: {error}");
            }
        }
        TaskbarMode::Autohide => set_autohide(true, windows.first().copied()),
        TaskbarMode::Visible => {}
    }
}

fn hwnd_of(value: isize) -> HWND {
    HWND(value as *mut c_void)
}

/// The taskbar windows: one on the primary monitor, one on each other.
/// Explorer's only, never the dock's own tray proxy; the primary one first
/// when there is one.
pub fn taskbar_windows() -> Vec<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, found: LPARAM) -> BOOL {
        let mut name = [0u16; 64];
        let length = unsafe { GetClassNameW(hwnd, &mut name) };
        let name = String::from_utf16_lossy(&name[..usize::try_from(length).unwrap_or(0)]);
        // Not the dock's own tray proxy, which has the taskbar's class.
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process)) };
        if CLASSES.contains(&name.as_str()) && process != std::process::id() {
            let found = unsafe { &mut *(found.0 as *mut Vec<HWND>) };
            found.push(hwnd);
        }
        BOOL(1)
    }
    let mut found: Vec<HWND> = Vec::new();
    // A failed enumeration leaves what it found; there is nothing to do
    // about the rest.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut found as *mut _ as isize)) };
    found.sort_by_key(|&hwnd| !is_primary(hwnd));
    found
}

/// Whether `hwnd` is the primary monitor's taskbar (`Shell_TrayWnd`).
fn is_primary(hwnd: HWND) -> bool {
    let mut name = [0u16; 64];
    let length = unsafe { GetClassNameW(hwnd, &mut name) };
    String::from_utf16_lossy(&name[..usize::try_from(length).unwrap_or(0)]) == CLASSES[0]
}

/// The flags of the taskbar (`ABS_*`).
fn flags() -> u32 {
    let mut data = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) as u32 }
}

/// Turns the taskbar's own hiding on or off, keeping its other flags. The
/// message wants a window of the taskbar to name.
fn set_autohide(on: bool, taskbar: Option<HWND>) {
    let current = flags();
    let next = if on {
        current | ABS_AUTOHIDE
    } else {
        current & !ABS_AUTOHIDE
    };
    if next == current {
        return;
    }
    let mut data = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: taskbar.unwrap_or_default(),
        lParam: LPARAM(next as isize),
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };
}

/// Takes the taskbar the way `mode` says, keeping what it was to give it
/// back. The first call in a run writes the state file and starts the
/// watchdog, before anything changes; `TaskbarMode::Visible` gives the
/// taskbar back instead.
pub fn hold(mode: TaskbarMode, state_path: &Path) -> Result<(), Box<dyn Error>> {
    if mode == TaskbarMode::Visible {
        release();
        return Ok(());
    }
    let windows = taskbar_windows();
    let mut saved = match Saved::read(state_path).filter(|saved| saved.pid == std::process::id()) {
        // Already held in this run: the original state stays the one saved.
        Some(saved) => saved,
        None => {
            let saved = Saved {
                pid: std::process::id(),
                autohide: flags() & ABS_AUTOHIDE != 0,
                hidden: Vec::new(),
                faded: Vec::new(),
            };
            saved.write(state_path)?;
            ensure_watchdog(state_path);
            saved
        }
    };
    *ACTIVE.lock().unwrap_or_else(PoisonError::into_inner) = Some(Active {
        path: state_path.to_owned(),
        mode,
    });
    set_autohide(true, windows.first().copied());
    match mode {
        TaskbarMode::Hidden => {
            fade_visible(&windows, &mut saved, state_path)?;
            spawn_keeper();
        }
        // Autohide: the windows faded before come back.
        TaskbarMode::Autohide | TaskbarMode::Visible => show_faded(&mut saved, state_path)?,
    }
    Ok(())
}

/// Whether a taskbar window is see-through and deaf to the mouse, as
/// `fade_visible` leaves it.
fn is_faded(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    if style & FADED_STYLE != FADED_STYLE {
        return false;
    }
    let mut alpha = 255u8;
    let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
    let read =
        unsafe { GetLayeredWindowAttributes(hwnd, None, Some(&mut alpha), Some(&mut flags)) };
    read.is_ok() && flags.0 & LWA_ALPHA.0 != 0 && alpha == 0
}

/// What a taskbar window is before it is faded.
fn faded_of(hwnd: HWND) -> Faded {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let layered = if ex_style & WS_EX_LAYERED.0 as isize != 0 {
        let mut key = COLORREF(0);
        let mut alpha = 255u8;
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
        unsafe {
            GetLayeredWindowAttributes(hwnd, Some(&mut key), Some(&mut alpha), Some(&mut flags))
        }
        .ok()
        .map(|()| (key.0, alpha, flags.0))
    } else {
        None
    };
    Faded {
        hwnd: hwnd.0 as isize,
        ex_style,
        layered,
    }
}

/// Fades the taskbar windows that are showing, recording what each one was
/// first: a window faded and never recorded would stay faded. They also go
/// below every topmost window, so the dock's tray host (which is topmost) is
/// the `Shell_TrayWnd` programs find first; `restore_file` puts them back.
fn fade_visible(windows: &[HWND], saved: &mut Saved, state_path: &Path) -> io::Result<()> {
    for &hwnd in windows {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            continue;
        }
        let value = hwnd.0 as isize;
        // Recorded once: later, what it is may be what this run made it.
        if !saved.faded.iter().any(|faded| faded.hwnd == value) {
            saved.faded.push(faded_of(hwnd));
            saved.write(state_path)?;
        }
        if !is_faded(hwnd) {
            let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | FADED_STYLE) };
            let _ = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA) };
        }
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
    }
    // Explorer may have put its window back on top: so does the host.
    super::tray_host::raise();
    Ok(())
}

/// Explorer shows its windows again at times (after the hiding flag changes,
/// when a monitor comes or goes, when it restarts): a few more tries after
/// the hold, spread over a few seconds, and `keep` on the events that
/// announce it (`appbar.rs`). Not a loop that runs for ever.
fn spawn_keeper() {
    let spawned = std::thread::Builder::new()
        .name("taskbar-keeper".into())
        .spawn(|| {
            for pause in [300, 700, 1500, 3000] {
                std::thread::sleep(std::time::Duration::from_millis(pause));
                keep();
            }
        });
    if let Err(error) = spawned {
        log::warn!("taskbar: the keeper did not start: {error}");
    }
}

/// Gives a faded window what it was: its style and, if it was layered, its
/// layered attributes; then has it painted again, as a window that stops
/// being layered must be.
fn unfade(faded: &Faded) {
    let hwnd = hwnd_of(faded.hwnd);
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
    }
    unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, faded.ex_style) };
    if let Some((key, alpha, flags)) = faded.layered {
        let _ = unsafe {
            SetLayeredWindowAttributes(
                hwnd,
                COLORREF(key),
                alpha,
                LAYERED_WINDOW_ATTRIBUTES_FLAGS(flags),
            )
        };
    }
    let _ = unsafe {
        RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_ERASE | RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN,
        )
    };
}

fn show_faded(saved: &mut Saved, state_path: &Path) -> io::Result<()> {
    for value in saved.hidden.drain(..) {
        let hwnd = hwnd_of(value);
        if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            let _ = unsafe { ShowWindow(hwnd, SW_SHOWNA) };
        }
    }
    for faded in saved.faded.drain(..) {
        unfade(&faded);
    }
    saved.write(state_path)
}

/// Gives the taskbar back as it was, if this run holds it. Safe to call
/// from anywhere, any number of times.
pub fn release() {
    // Let go first: giving the taskbar back makes Explorer announce a change
    // of settings, and `keep` (on the app bar's thread, or the keeper) would
    // hide it again while this run still held it.
    let taken = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner).take();
    let Some(Active { path, .. }) = taken else {
        return;
    };
    restore_file(&path, std::process::id());
}

/// Restores the taskbar from the state file when `pid` is the process that
/// wrote it, then deletes the file. Does nothing for a file of another
/// process or without a file.
fn restore_file(path: &Path, pid: u32) {
    let Some(saved) = Saved::read(path) else {
        return;
    };
    if saved.pid != pid {
        return;
    }
    for faded in &saved.faded {
        unfade(faded);
    }
    let touched = saved
        .hidden
        .iter()
        .map(|&hwnd| (hwnd, true))
        .chain(saved.faded.iter().map(|faded| (faded.hwnd, false)));
    for (value, hidden) in touched {
        let hwnd = hwnd_of(value);
        if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            // A faded window was never hidden: Explorer alone says whether
            // it shows.
            if hidden {
                let _ = unsafe { ShowWindow(hwnd, SW_SHOWNA) };
            }
            // The taskbar is a topmost window; `fade_visible` lowered it.
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                )
            };
        }
    }
    // A fresh handle: the one saved may be gone with an Explorer restart.
    set_autohide(saved.autohide, taskbar_windows().first().copied());
    let _ = fs::remove_file(path);
}

/// Whether a process is still running.
fn is_alive(pid: u32) -> bool {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let mut code = 0;
    let alive = unsafe { GetExitCodeProcess(handle, &mut code) }.is_ok() && code == STILL_ACTIVE;
    let _ = unsafe { CloseHandle(handle) };
    alive
}

/// At startup: a state file of a process that is gone means the taskbar was
/// left changed (the watchdog failed too, or the machine lost power):
/// restores it before the dock takes anything.
pub fn recover(state_path: &Path) {
    let Some(saved) = Saved::read(state_path) else {
        return;
    };
    if saved.pid == std::process::id() || is_alive(saved.pid) {
        return;
    }
    log::warn!("taskbar: restoring what a previous run left changed");
    restore_file(state_path, saved.pid);
}

/// Name of the copy of this executable the watchdog runs as. A different
/// name, so that ending `dockmax.exe` by name (`taskkill /IM`, the Details
/// tab of the Task Manager) leaves the watchdog alone.
pub const GUARD_NAME: &str = "dockmax-guard.exe";

/// The copy of `source` the watchdog runs as, in `dir`: made when it is
/// missing or differs from `source` (another size or date, as after an
/// update). A copy that cannot be made (a watchdog of an earlier run is
/// still running from it) leaves the one there.
fn ensure_guard(source: &Path, dir: &Path) -> io::Result<PathBuf> {
    let guard = dir.join(GUARD_NAME);
    let same = || -> Option<bool> {
        let from = fs::metadata(source).ok()?;
        let to = fs::metadata(&guard).ok()?;
        Some(from.len() == to.len() && from.modified().ok()? == to.modified().ok()?)
    };
    if same() == Some(true) {
        return Ok(guard);
    }
    fs::create_dir_all(dir)?;
    match fs::copy(source, &guard) {
        Ok(_) => Ok(guard),
        Err(_) if guard.exists() => Ok(guard),
        Err(error) => Err(error),
    }
}

/// The command line that starts the watchdog out of the process tree of the
/// app: `cmd` starts it and ends at once, so the watchdog is the child of a
/// process that is gone, and ending the tree of the app (the Task Manager's
/// "End task", `taskkill /T`) does not reach it. `None` for a path `cmd`
/// would read as something else (`&`, `%`, `^`, a quote).
fn watchdog_command_line(guard: &Path, pid: u32, state_path: &Path) -> Option<String> {
    let unsafe_for_cmd = |path: &Path| {
        path.to_string_lossy()
            .chars()
            .any(|c| matches!(c, '&' | '%' | '^' | '"' | '|' | '<' | '>' | '(' | ')' | '!'))
    };
    if unsafe_for_cmd(guard) || unsafe_for_cmd(state_path) {
        return None;
    }
    Some(format!(
        "/c start \"\" /b \"{}\" {WATCHDOG_FLAG} {pid} \"{}\"",
        guard.display(),
        state_path.display()
    ))
}

/// Whether this run started its watchdog.
static WATCHDOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Starts the watchdog once in a run: for the taskbar, or for the windows
/// the workspaces hide (spec 0013 C2). `state_path` is the taskbar's state
/// file, whether there is one or not; the parked windows' file is next to it.
pub fn ensure_watchdog(state_path: &Path) {
    if !WATCHDOG.swap(true, std::sync::atomic::Ordering::SeqCst) {
        spawn_watchdog(state_path);
    }
}

/// Starts the watchdog: a copy of this executable that outlives the app,
/// however it ends, and restores the taskbar then. No window, no console.
fn spawn_watchdog(state_path: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    let pid = std::process::id();
    let started = std::env::current_exe().and_then(|exe| {
        let dir = state_path.parent().unwrap_or_else(|| Path::new("."));
        let guard = ensure_guard(&exe, dir).unwrap_or_else(|error| {
            log::warn!("taskbar: no copy for the watchdog, it runs as the app: {error}");
            exe
        });
        match watchdog_command_line(&guard, pid, state_path) {
            Some(line) => std::process::Command::new("cmd")
                .raw_arg(line)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn(),
            // A path `cmd` would misread: started directly, inside the tree.
            None => std::process::Command::new(&guard)
                .arg(WATCHDOG_FLAG)
                .arg(pid.to_string())
                .arg(state_path)
                .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
                .spawn(),
        }
    });
    if let Err(error) = started {
        // The other ways out still hold; the dock goes on without this one.
        log::warn!("taskbar: the watchdog did not start: {error}");
    }
}

/// The watchdog's whole life: waits for `pid` to end, then restores what it
/// left. Called from `main`, before anything of the app starts.
pub fn watch(pid: u32, state_path: &Path) {
    // Gone already when the process can't be opened: nothing to wait for.
    if let Ok(handle) = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
        unsafe { WaitForSingleObject(handle, INFINITE) };
        let _ = unsafe { CloseHandle(handle) };
    }
    restore_file(state_path, pid);
    super::window_spaces::restore_file(&state_path.with_file_name(super::window_spaces::FILE), pid);
}

/// `--restore-taskbar <state file>`: restores whatever the file holds,
/// whoever wrote it.
pub fn restore_now(state_path: &Path) {
    if let Some(saved) = Saved::read(state_path) {
        restore_file(state_path, saved.pid);
    }
    super::window_spaces::restore_now(&state_path.with_file_name(super::window_spaces::FILE));
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn saved() -> Saved {
        Saved {
            pid: 4242,
            autohide: true,
            hidden: vec![1, 2, 3],
            faded: vec![Faded {
                hwnd: 4,
                ex_style: 0x0800_0088,
                layered: Some((0, 255, 2)),
            }],
        }
    }

    #[test]
    fn the_state_file_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);

        saved().write(&path).unwrap();

        assert_eq!(Saved::read(&path), Some(saved()));
    }

    #[test]
    fn the_state_file_is_written_whole_or_not_at_all() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("deep").join(STATE_FILE);

        saved().write(&path).unwrap();

        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [STATE_FILE]);
    }

    #[test]
    fn a_missing_or_unreadable_file_is_no_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        assert_eq!(Saved::read(&path), None);

        fs::write(&path, b"not json").unwrap();
        assert_eq!(Saved::read(&path), None);
        fs::write(&path, br#"{"pid":1}"#).unwrap();
        assert_eq!(Saved::read(&path), None);
        // An unreadable file is left where it is.
        assert!(path.exists());
    }

    #[test]
    fn a_file_of_an_earlier_version_still_reads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        // Before spec 0014 the windows were hidden, and the file had no
        // `faded`.
        fs::write(&path, br#"{"pid":7,"autohide":false,"hidden":[10,20]}"#).unwrap();

        assert_eq!(
            Saved::read(&path),
            Some(Saved {
                pid: 7,
                autohide: false,
                hidden: vec![10, 20],
                faded: Vec::new(),
            })
        );
    }

    #[test]
    fn a_file_of_another_process_is_not_restored() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        saved().write(&path).unwrap();

        restore_file(&path, 1);

        assert!(path.exists(), "only the process that wrote it restores it");
    }

    #[test]
    fn restoring_twice_does_nothing_the_second_time() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        Saved {
            // Hidden handles that are not windows, and the flag as it is:
            // the restore must change nothing on this machine.
            autohide: flags() & ABS_AUTOHIDE != 0,
            hidden: Vec::new(),
            faded: Vec::new(),
            ..saved()
        }
        .write(&path)
        .unwrap();

        restore_file(&path, 4242);
        assert!(!path.exists());
        restore_file(&path, 4242);
        assert!(!path.exists());
    }

    #[test]
    fn this_process_is_alive_and_a_gone_one_is_not() {
        assert!(is_alive(std::process::id()));
        // The largest pid is never in use.
        assert!(!is_alive(u32::MAX));
    }

    #[test]
    fn recovery_leaves_alone_a_file_of_a_living_process() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        Saved {
            pid: std::process::id(),
            ..saved()
        }
        .write(&path)
        .unwrap();

        recover(&path);

        assert!(path.exists());
    }

    #[test]
    fn recovery_restores_what_a_dead_process_left() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        Saved {
            pid: u32::MAX,
            autohide: flags() & ABS_AUTOHIDE != 0,
            hidden: Vec::new(),
            faded: Vec::new(),
        }
        .write(&path)
        .unwrap();

        recover(&path);

        assert!(!path.exists());
    }

    #[test]
    fn the_guard_is_a_copy_with_a_name_of_its_own() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("dockmax.exe");
        fs::write(&source, b"the program").unwrap();
        let folder = dir.path().join("data");

        let guard = ensure_guard(&source, &folder).unwrap();

        assert_eq!(guard, folder.join(GUARD_NAME));
        assert_eq!(fs::read(&guard).unwrap(), b"the program");
        assert_ne!(guard.file_name(), source.file_name());
    }

    #[test]
    fn an_up_to_date_guard_is_not_copied_again() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("dockmax.exe");
        fs::write(&source, b"the program").unwrap();
        let guard = ensure_guard(&source, dir.path()).unwrap();
        // Same size and same date: left alone (a mark would be overwritten).
        let before = fs::metadata(&guard).unwrap().modified().unwrap();

        ensure_guard(&source, dir.path()).unwrap();

        assert_eq!(fs::metadata(&guard).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn a_guard_of_another_version_is_replaced() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("dockmax.exe");
        fs::write(&source, b"version one").unwrap();
        ensure_guard(&source, dir.path()).unwrap();
        fs::write(&source, b"version two, longer").unwrap();

        let guard = ensure_guard(&source, dir.path()).unwrap();

        assert_eq!(fs::read(&guard).unwrap(), b"version two, longer");
    }

    #[test]
    fn a_missing_program_has_no_guard() {
        let dir = tempdir().unwrap();

        assert!(ensure_guard(&dir.path().join("nothing.exe"), dir.path()).is_err());
    }

    #[test]
    fn the_watchdog_starts_through_cmd_with_every_path_quoted() {
        let line = watchdog_command_line(
            Path::new(r"C:\Users\Maxsuel Einstein\AppData\Local\dev.x\dockmax-guard.exe"),
            4242,
            Path::new(r"C:\Users\Maxsuel Einstein\AppData\Local\dev.x\taskbar-state.json"),
        )
        .unwrap();

        assert_eq!(
            line,
            r#"/c start "" /b "C:\Users\Maxsuel Einstein\AppData\Local\dev.x\dockmax-guard.exe" --taskbar-watchdog 4242 "C:\Users\Maxsuel Einstein\AppData\Local\dev.x\taskbar-state.json""#
        );
    }

    #[test]
    fn a_path_cmd_would_misread_is_refused() {
        for bad in [
            r"C:\a&b\x.exe",
            r"C:\100%\x.exe",
            r"C:\a^b\x.exe",
            "C:\\a\"b\\x.exe",
        ] {
            assert_eq!(
                watchdog_command_line(Path::new(bad), 1, Path::new(r"C:\ok\s.json")),
                None,
                "{bad}"
            );
            assert_eq!(
                watchdog_command_line(Path::new(r"C:\ok\x.exe"), 1, Path::new(bad)),
                None,
                "{bad}"
            );
        }
    }
}
