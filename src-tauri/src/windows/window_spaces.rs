//! The windows of each workspace (spec 0013 C2): which workspace each window
//! belongs to, and the switch that parks the windows of the others and
//! brings back the ones of the workspace shown. Rules in
//! `domain/window_spaces.rs`.
//!
//! Parking hides a window of another program (`ShowWindow(SW_HIDE)`, which
//! takes it off the screen and out of Alt+Tab). Nothing is hidden before it
//! is written down: the parked windows are kept in a file next to the
//! taskbar's state, and every way out shows them again, like the taskbar
//! (ADR 0009): the normal exit and the end of the Windows session
//! (`release_all`), the watchdog when the DockMax dies (`restore_file`), and
//! the next start (`setup`).
//!
//! Which workspace each window belongs to is kept too, in a session file
//! (the PrimoDock's `session.json`): every way out shows every window, and the
//! next start gives each one back its workspace and parks again those of the
//! workspaces not shown. A window is known by its handle and its process, so
//! a handle Windows gave to another window since is not mistaken for it.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, IsIconic, IsWindow, SW_HIDE, SW_SHOWMINNOACTIVE, SW_SHOWNA,
    ShowWindow,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::domain::window_spaces::{Parked, Tracked, counts_for, follow, to_park, to_unpark};
use crate::persistence::database::Storage;

/// Name of the file of the parked windows, next to the taskbar's state.
pub const FILE: &str = "parked-windows.json";

/// Name of the session file: the workspace of each window.
pub const SESSION_FILE: &str = "window-workspaces.json";

/// One window of the session: its handle, its process and its workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Owned {
    hwnd: isize,
    pid: u32,
    workspace: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Session {
    windows: Vec<Owned>,
}

/// What the file holds: who parked the windows, and which.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Saved {
    pid: u32,
    windows: Vec<Parked>,
}

#[derive(Default)]
struct Inner {
    known: HashMap<isize, Tracked>,
    parked: Vec<Parked>,
    /// The file of the parked windows; `None` until `setup`, and then nothing
    /// is ever parked.
    file: Option<PathBuf>,
    /// The session file, and what it held at the start: the workspace of
    /// each window of the last run, until the first look gives it back.
    session: Option<PathBuf>,
    remembered: Option<Vec<Owned>>,
    /// The session as last written, to write it only when it changes.
    written: Vec<Owned>,
}

static STATE: Mutex<Option<Inner>> = Mutex::new(None);

fn with_state<T>(change: impl FnOnce(&mut Inner) -> T) -> T {
    let mut state = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    change(state.get_or_insert_with(Inner::default))
}

fn hwnd(value: isize) -> HWND {
    HWND(value as *mut _)
}

/// Shows a parked window again as it was: minimized if it was.
fn show(parked: &Parked) {
    let window = hwnd(parked.hwnd);
    if !unsafe { IsWindow(Some(window)) }.as_bool() {
        return;
    }
    let how = if parked.minimized {
        SW_SHOWMINNOACTIVE
    } else {
        SW_SHOWNA
    };
    let _ = unsafe { ShowWindow(window, how) };
}

/// Writes the parked windows down, or deletes the file when there are none.
fn write(path: &Path, windows: &[Parked]) -> std::io::Result<()> {
    if windows.is_empty() {
        return match fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    let saved = Saved {
        pid: std::process::id(),
        windows: windows.to_vec(),
    };
    let text = serde_json::to_string(&saved).map_err(std::io::Error::other)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// Shows the windows the file at `path` holds when `pid` wrote it (or any
/// writer, with `None`), then deletes it.
fn show_from_file(path: &Path, pid: Option<u32>) {
    let Some(saved) = fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Saved>(&text).ok())
    else {
        return;
    };
    if pid.is_some_and(|pid| pid != saved.pid) {
        return;
    }
    for parked in &saved.windows {
        show(parked);
    }
    let _ = fs::remove_file(path);
}

/// The watchdog's part: `pid` is gone; the windows it parked come back.
pub fn restore_file(path: &Path, pid: u32) {
    show_from_file(path, Some(pid));
}

/// `--restore-taskbar`: whatever is parked comes back, whoever parked it.
pub fn restore_now(path: &Path) {
    show_from_file(path, None);
}

/// At startup, in `dir` (the app's local data folder): windows a run that is
/// gone left parked come back, then the switch can park.
pub fn setup(dir: &Path) {
    let path = dir.join(FILE);
    if let Some(saved) = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Saved>(&text).ok())
        && saved.pid != std::process::id()
    {
        log::warn!("workspaces: showing the windows a previous run left hidden");
        show_from_file(&path, None);
    }
    let session = dir.join(SESSION_FILE);
    let remembered = fs::read_to_string(&session)
        .ok()
        .and_then(|text| serde_json::from_str::<Session>(&text).ok())
        .map(|session| session.windows)
        .unwrap_or_default();
    with_state(|inner| {
        inner.file = Some(path);
        inner.session = Some(session);
        inner.remembered = Some(remembered);
    });
}

/// The process of the window `value`; 0 when it is gone.
fn process_of(value: isize) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd(value), Some(&mut pid)) };
    pid
}

/// The session as it is: every window known, parked ones included, with its
/// process.
fn session_of(inner: &Inner) -> Vec<Owned> {
    let mut windows: Vec<Owned> = inner
        .known
        .iter()
        .map(|(&value, tracked)| Owned {
            hwnd: value,
            pid: process_of(value),
            workspace: tracked.workspace.clone(),
        })
        .filter(|owned| owned.pid != 0)
        .collect();
    windows.sort_by_key(|owned| owned.hwnd);
    windows
}

/// Writes the session down when it changed.
fn keep_session(inner: &mut Inner) {
    let Some(path) = inner.session.clone() else {
        return;
    };
    let windows = session_of(inner);
    if windows == inner.written {
        return;
    }
    let text = match serde_json::to_string(&Session {
        windows: windows.clone(),
    }) {
        Ok(text) => text,
        Err(_) => return,
    };
    let temporary = path.with_extension("json.tmp");
    let written = fs::write(&temporary, text).and_then(|()| fs::rename(&temporary, &path));
    match written {
        Ok(()) => inner.written = windows,
        Err(error) => log::warn!("workspaces: the windows' workspaces could not be kept: {error}"),
    }
}

/// The monitors by their handle (an `HMONITOR` as a number): their device
/// names.
fn monitor_names() -> HashMap<isize, String> {
    super::monitors::all()
        .into_iter()
        .map(|monitor| (monitor.handle, monitor.key))
        .collect()
}

/// The windows with a taskbar button on the screen now, each with its
/// monitor, topmost first.
fn on_screen(hwnds: &[isize]) -> Vec<(isize, String)> {
    let names = monitor_names();
    hwnds
        .iter()
        .filter_map(|&value| {
            let monitor = unsafe { MonitorFromWindow(hwnd(value), MONITOR_DEFAULTTONEAREST) };
            names
                .get(&(monitor.0 as isize))
                .map(|name| (value, name.clone()))
        })
        .collect()
}

/// After a look at the windows (`hwnds`, the ones with a taskbar button,
/// topmost first): each window gets its workspace (spec 0013 C2).
pub fn follow_windows(app: &AppHandle, hwnds: &[isize]) {
    let now = on_screen(hwnds);
    let mut shown: HashMap<String, Option<String>> = HashMap::new();
    let existing: Vec<String> = {
        let state = app.state::<Mutex<Storage>>();
        let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
        for (_, monitor) in &now {
            if !shown.contains_key(monitor) {
                let workspace = storage.shown_on(monitor).ok().map(|w| w.id);
                shown.insert(monitor.clone(), workspace);
            }
        }
        storage
            .workspaces()
            .map(|all| all.into_iter().map(|w| w.id).collect())
            .unwrap_or_default()
    };
    // The first look of a run gives each window the workspace it had in the
    // last run, then parks again those not shown on their monitor. A
    // workspace that is gone since (deleted, or another database) gives
    // nothing back: the window joins what its monitor shows.
    let restoring = with_state(|inner| {
        let remembered = inner.remembered.take()?;
        for (value, monitor) in &now {
            let pid = process_of(*value);
            if let Some(owned) = remembered
                .iter()
                .find(|owned| owned.hwnd == *value && owned.pid == pid && pid != 0)
                .filter(|owned| existing.contains(&owned.workspace))
            {
                inner.known.insert(
                    *value,
                    Tracked {
                        workspace: owned.workspace.clone(),
                        monitor: monitor.clone(),
                    },
                );
            }
        }
        Some(())
    });
    with_state(|inner| {
        let parked = inner.parked.clone();
        follow(&mut inner.known, &now, &parked, &|monitor| {
            shown.get(monitor).cloned().flatten()
        });
        keep_session(inner);
    });
    if restoring.is_some() {
        for (monitor, workspace) in &shown {
            if let Some(workspace) = workspace {
                switch(monitor, workspace, workspace);
            }
        }
    }
}

/// Whether the window `hwnd` counts on the dock of `workspace`.
pub fn counts(hwnd: isize, workspace: Option<&str>) -> bool {
    with_state(|inner| counts_for(&inner.known, hwnd, workspace))
}

/// The monitor `monitor` shows `target` now (it showed `from`): its windows
/// of other workspaces are parked, the ones parked there of `target` come
/// back. Returns the window brought back on top, for the focus. Nothing is
/// hidden when the list of the parked can't be written.
pub fn switch(monitor: &str, from: &str, target: &str) -> Option<isize> {
    let windows: Vec<isize> = super::openwin::enumerate()
        .iter()
        .map(|window| window.hwnd)
        .collect();
    let now = on_screen(&windows);
    let (hide, back, path, parked) = with_state(|inner| {
        let path = inner.file.clone()?;
        let hide: Vec<isize> = to_park(&inner.known, &now, monitor, target);
        let back: Vec<Parked> = to_unpark(&inner.parked, monitor, target)
            .into_iter()
            .cloned()
            .collect();
        let mut parked: Vec<Parked> = inner
            .parked
            .iter()
            .filter(|p| !back.contains(p))
            .cloned()
            .collect();
        for &value in &hide {
            parked.push(Parked {
                hwnd: value,
                workspace: inner
                    .known
                    .get(&value)
                    .map_or_else(|| from.to_owned(), |tracked| tracked.workspace.clone()),
                monitor: monitor.to_owned(),
                minimized: unsafe { IsIconic(hwnd(value)) }.as_bool(),
            });
        }
        Some((hide, back, path, parked))
    })?;
    if !hide.is_empty() {
        // The watchdog shows them again if the DockMax dies.
        super::taskbar::ensure_watchdog(&path.with_file_name(super::taskbar::STATE_FILE));
    }
    if let Err(error) = write(&path, &parked) {
        log::warn!("workspaces: the windows to hide could not be written down: {error}");
        return None;
    }
    with_state(|inner| {
        inner.parked = parked;
        keep_session(inner);
    });
    for value in &hide {
        let _ = unsafe { ShowWindow(hwnd(*value), SW_HIDE) };
    }
    for parked in &back {
        show(parked);
    }
    back.iter().find(|p| !p.minimized).map(|p| p.hwnd)
}

/// The workspace `id` was deleted: its parked windows come back, and its
/// windows get the workspace of their monitor at the next look.
pub fn forget_workspace(id: &str) {
    let back: Vec<Parked> = with_state(|inner| {
        inner.known.retain(|_, tracked| tracked.workspace != id);
        let (back, kept): (Vec<Parked>, Vec<Parked>) =
            inner.parked.drain(..).partition(|p| p.workspace == id);
        inner.parked = kept;
        keep_session(inner);
        if let Some(path) = &inner.file
            && let Err(error) = write(path, &inner.parked)
        {
            log::warn!("workspaces: the parked windows could not be written: {error}");
        }
        back
    });
    for parked in &back {
        show(parked);
    }
}

/// Every parked window comes back (the DockMax ends, or the session). The
/// workspace of each window stays written down for the next start.
pub fn release_all() {
    let (back, path) = with_state(|inner| {
        keep_session(inner);
        (std::mem::take(&mut inner.parked), inner.file.clone())
    });
    for parked in &back {
        show(parked);
    }
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn parked(hwnd: isize) -> Parked {
        Parked {
            hwnd,
            workspace: "w".into(),
            monitor: r"\\.\DISPLAY1".into(),
            minimized: false,
        }
    }

    #[test]
    fn the_parked_windows_are_written_and_the_file_goes_with_the_last() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(FILE);
        write(&path, &[parked(1), parked(2)]).unwrap();
        let saved: Saved = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.pid, std::process::id());
        assert_eq!(saved.windows, [parked(1), parked(2)]);
        // No temporary file left.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

        write(&path, &[]).unwrap();
        assert!(!path.exists());
        // Nothing to delete is no error.
        write(&path, &[]).unwrap();
    }

    #[test]
    fn the_watchdog_shows_only_what_its_process_parked() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(FILE);
        // Handles that are no window: showing them does nothing.
        write(&path, &[parked(0x7fff_0001)]).unwrap();
        restore_file(&path, std::process::id() + 1);
        assert!(path.exists(), "another process's file stays");
        restore_file(&path, std::process::id());
        assert!(!path.exists());
    }
}
