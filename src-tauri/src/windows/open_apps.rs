//! Which programs are open, and what a click on one does (spec 0009).
//!
//! No polling: a `SetWinEventHook` tells this thread when a window is made,
//! destroyed, shown, hidden, cloaked, minimized or brought to the front, and
//! the thread looks at the windows once, a moment after the last of a burst.
//! The window handles stay here; the dock only gets who is open and whether
//! the user is in it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Listener, Manager};

use super::openwin::OpenWindow;
use crate::commands::apps::CHANGED_EVENT as APPS_CHANGED_EVENT;
use crate::commands::settings::CHANGED_EVENT as SETTINGS_CHANGED_EVENT;
use crate::commands::widgets::lock;
use crate::domain::apps::is_pinnable_path;
use crate::domain::open_apps::{
    Click, Found, Identity, OpenApp, OpenOther, OpenSnapshot, WindowFacts, click, keep_order,
    others, short_id, summarize,
};
use crate::persistence::database::Storage;

/// Event sent to every window with the `OpenSnapshot` after a change.
pub const CHANGED_EVENT: &str = "open-windows-changed";

/// An open program that is not pinned, as it was at the last look.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Other {
    /// What it is grouped by (`domain::open_apps::group_key`).
    key: String,
    /// The opaque name the dock uses for it (`short_id` of the key).
    id: String,
    /// What `app_icon` takes for it.
    app_id: String,
    name: String,
    hwnds: Vec<isize>,
    active: bool,
    /// The AppID the windows declare, when they do: what a badge is found by.
    aumid: Option<String>,
    /// The program has a badge to draw on its icon.
    badge: bool,
}

/// What the windows were at the last look.
#[derive(Debug, Default)]
struct State {
    found: Vec<Found>,
    others: Vec<Other>,
    /// The keys of the open programs that are not pinned, in the order the
    /// dock showed them: kept while they are hidden (the setting off), so
    /// they come back where they were.
    order: Vec<String>,
    /// The window in front at the last look.
    foreground: Option<isize>,
}

static STATE: Mutex<State> = Mutex::new(State {
    found: Vec::new(),
    others: Vec::new(),
    order: Vec::new(),
    foreground: None,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the shell calls each pinned program, by the program's id in the Apps
/// folder: read once, as it costs the shell's time and does not change while
/// the program stays pinned.
static IDENTITIES: Mutex<Option<HashMap<String, Identity>>> = Mutex::new(None);

/// What an open program is shown as: the id for its icon and its name.
type Description = Option<(String, String)>;

/// What an open program that is not pinned is shown as, by its group key: its
/// id for the icon and its name, or `None` for one with no way to show it,
/// and when that was found.
static DESCRIPTIONS: Mutex<Option<HashMap<String, (Description, Instant)>>> = Mutex::new(None);

/// The most programs [`DESCRIPTIONS`] remembers: when it is full it starts
/// over, as the other caches of names do.
const MAX_DESCRIPTIONS: usize = 512;

/// How long a program with no way to show it is left alone: one installed
/// meanwhile gets its name and icon without a restart.
const NOT_FOUND_FOR: std::time::Duration = std::time::Duration::from_secs(60);

/// Whether a description found at `found` is still good `now`: one that
/// found something always is, a `None` only for [`NOT_FOUND_FOR`].
fn still_good(description: &Description, found: Instant, now: Instant) -> bool {
    description.is_some() || now.duration_since(found) < NOT_FOUND_FOR
}

/// The programs of the Apps folder by the file they run (lowercased): the id
/// and the name. Read once, when the first open program that needs it shows
/// up: it takes the shell about a second.
static CATALOG_BY_FILE: Mutex<Option<Vec<(String, String, String)>>> = Mutex::new(None);

/// A look at the windows is wanted (an event came, or the pinned programs
/// changed).
static DIRTY: AtomicBool = AtomicBool::new(false);

/// A window moved, changed size, showed or hid: whether a monitor is in full
/// screen is checked again (spec 0010 D4). Cheap, and apart from `DIRTY`,
/// which a move alone does not set.
static GEOMETRY_DIRTY: AtomicBool = AtomicBool::new(true);

/// When the last event came, in milliseconds since `START`.
static LAST_EVENT: AtomicU64 = AtomicU64::new(0);

/// The id of the thread that listens, to wake it from other threads.
static LISTENER: AtomicU32 = AtomicU32::new(0);

/// How long after the last event the windows are looked at: a burst (a
/// program opening makes a dozen) is one look.
const QUIET_MS: u64 = 200;

/// How often the windows already found are checked to still exist: a program
/// that is killed (the Task Manager, a crash) does not tell Windows its
/// windows are gone, so no event would clear its mark. Asking `IsWindow` of
/// a few handles costs nothing; a look follows only if one is gone.
const HEARTBEAT_MS: u32 = 5_000;

/// Whether a window of the programs found last no longer exists.
fn a_window_is_gone() -> bool {
    let state = state();
    state
        .found
        .iter()
        .flat_map(|found| found.hwnds.iter())
        .chain(state.others.iter().flat_map(|other| other.hwnds.iter()))
        .any(|hwnd| !super::openwin::exists(*hwnd))
}

fn snapshot_of(state: &State) -> OpenSnapshot {
    OpenSnapshot {
        apps: state.found.iter().map(OpenApp::from).collect(),
        others: state
            .others
            .iter()
            .map(|other| OpenOther {
                key: other.id.clone(),
                app_id: other.app_id.clone(),
                name: other.name.clone(),
                count: other.hwnds.len(),
                active: other.active,
                badge: other.badge,
            })
            .collect(),
    }
}

/// The programs that have windows now, as the dock sees them.
pub fn snapshot() -> OpenSnapshot {
    snapshot_of(&state())
}

/// Whether the window `hwnd` counts for the workspace `workspace` (spec
/// 0013 C2): every window when there is none.
fn counts(hwnd: isize, workspace: Option<&str>) -> bool {
    #[cfg(windows)]
    {
        super::window_spaces::counts(hwnd, workspace)
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, workspace);
        true
    }
}

/// The windows of `hwnds` that count for `workspace`.
fn of_workspace(hwnds: &[isize], workspace: Option<&str>) -> Vec<isize> {
    hwnds
        .iter()
        .copied()
        .filter(|hwnd| counts(*hwnd, workspace))
        .collect()
}

/// The programs that have windows in the workspace `workspace`, as its dock
/// sees them: the windows of the other workspaces don't count (a program
/// with none here is not open here).
pub fn snapshot_for(workspace: Option<&str>) -> OpenSnapshot {
    let state = state();
    let front = state.foreground;
    let found = state
        .found
        .iter()
        .filter_map(|found| {
            let hwnds = of_workspace(&found.hwnds, workspace);
            (!hwnds.is_empty()).then(|| Found {
                id: found.id.clone(),
                active: front.is_some_and(|front| hwnds.contains(&front)),
                hwnds,
            })
        })
        .collect();
    let others = state
        .others
        .iter()
        .filter_map(|other| {
            let hwnds = of_workspace(&other.hwnds, workspace);
            (!hwnds.is_empty()).then(|| Other {
                active: front.is_some_and(|front| hwnds.contains(&front)),
                hwnds,
                ..other.clone()
            })
        })
        .collect();
    snapshot_of(&State {
        found,
        others,
        order: Vec::new(),
        foreground: front,
    })
}

fn identity_of(app_id: &str) -> Identity {
    let mut cache = IDENTITIES.lock().unwrap_or_else(PoisonError::into_inner);
    let cache = cache.get_or_insert_with(HashMap::new);
    if let Some(known) = cache.get(app_id) {
        return known.clone();
    }
    let appid = super::apps::app_user_model_id(app_id);
    let target = super::apps::target_of(app_id);
    let identity = Identity::new(
        [Some(app_id), appid.as_deref()].into_iter().flatten(),
        target.as_deref().and_then(std::path::Path::to_str),
    );
    cache.insert(app_id.to_owned(), identity.clone());
    identity
}

/// The Apps-folder program that runs `exe` (lowercased), if there is one.
fn catalog_entry_for(exe: &str) -> Option<(String, String)> {
    let mut index = CATALOG_BY_FILE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let index = index.get_or_insert_with(|| {
        super::apps::catalog()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| {
                let target = super::apps::target_of(&entry.id)?;
                Some((
                    target.to_string_lossy().to_lowercase(),
                    entry.id,
                    entry.name,
                ))
            })
            .collect()
    });
    index
        .iter()
        .find(|(target, _, _)| target == exe)
        .map(|(_, id, name)| (id.clone(), name.clone()))
}

/// What to show an open program that is not pinned as: the id `app_icon`
/// takes for it and its name. The Apps folder's own item if there is one (its
/// AppID, or a program that runs the same file); otherwise the program's file
/// and its name.
fn describe(key: &str, window: &OpenWindow) -> Option<(String, String)> {
    if let Some((known, _)) = DESCRIPTIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|known| known.get(key))
        .filter(|(known, found)| still_good(known, *found, Instant::now()))
    {
        return known.clone();
    }
    let exe = window.exe.as_deref().and_then(std::path::Path::to_str);
    let described = window
        .aumid
        .as_deref()
        .and_then(|aumid| super::apps::name_of(aumid).map(|name| (aumid.to_owned(), name)))
        .or_else(|| exe.and_then(|exe| catalog_entry_for(&exe.to_lowercase())))
        .or_else(|| {
            let exe = exe.filter(|exe| is_pinnable_path(exe))?;
            let stem = std::path::Path::new(exe).file_stem()?.to_str()?;
            Some((exe.to_owned(), stem.to_owned()))
        });
    let mut cache = DESCRIPTIONS.lock().unwrap_or_else(PoisonError::into_inner);
    let known = cache.get_or_insert_with(HashMap::new);
    if known.len() >= MAX_DESCRIPTIONS && !known.contains_key(key) {
        known.clear();
    }
    known.insert(key.to_owned(), (described.clone(), Instant::now()));
    described
}

/// The open programs that are not pinned, one for each AppID the windows
/// declare, as the Windows taskbar groups them (the profiles of a browser each
/// have an AppID of their own, so each is an item). In the order shown last,
/// the new ones at the end.
fn described_others(
    windows: &[WindowFacts],
    open_windows: &[OpenWindow],
    found: &[Found],
    foreground: Option<isize>,
    order: &[String],
) -> Vec<Other> {
    let mut described: Vec<Other> = others(windows, found, order)
        .into_iter()
        .filter_map(|group| {
            let sample = open_windows
                .iter()
                .find(|window| group.hwnds.contains(&window.hwnd))?;
            let (app_id, name) = describe(&group.key, sample)?;
            // A Chrome window carries its profile: the name tells the
            // profiles apart, and the photo badges the icon.
            let profile = sample
                .aumid
                .as_deref()
                .and_then(super::profiles::profile_of);
            let name = match &profile {
                Some(profile) => format!("{name} ({})", profile.name),
                None => name,
            };
            Some(Other {
                id: short_id(&group.key),
                active: foreground.is_some_and(|front| group.hwnds.contains(&front)),
                badge: sample
                    .aumid
                    .as_deref()
                    .is_some_and(super::profiles::has_badge),
                aumid: sample.aumid.clone(),
                key: group.key,
                app_id,
                name,
                hwnds: group.hwnds,
            })
        })
        .collect();
    keep_order(&mut described, |other| &other.key, order);
    described
}

/// Looks at the windows and, if what the dock shows changed, tells it.
fn look(app: &AppHandle) {
    super::apps::init_com();
    let pinned = {
        let storage = app.state::<std::sync::Mutex<Storage>>();
        match lock(&storage).apps_snapshot() {
            Ok(snapshot) => snapshot.apps,
            Err(_) => Vec::new(),
        }
    };
    let programs: Vec<(String, Identity)> = pinned
        .iter()
        .map(|app| (app.id.to_string(), identity_of(&app.app_id)))
        .collect();
    // A program that is no longer pinned is forgotten.
    if let Some(cache) = IDENTITIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        cache.retain(|id, _| pinned.iter().any(|app| app.app_id == *id));
    }
    let open_windows = super::openwin::enumerate();
    #[cfg(windows)]
    super::window_spaces::follow_windows(
        app,
        &open_windows
            .iter()
            .map(|window| window.hwnd)
            .collect::<Vec<_>>(),
    );
    let windows: Vec<WindowFacts> = open_windows
        .iter()
        .map(|window| WindowFacts {
            hwnd: window.hwnd,
            aumid: window.aumid.clone(),
            exe: window
                .exe
                .as_deref()
                .and_then(|path| path.to_str().map(str::to_owned)),
        })
        .collect();
    let foreground = super::openwin::foreground();
    let found = summarize(&programs, &windows, foreground);

    let show_others = crate::commands::settings::current(app).dock.show_open_apps;
    let previous_order = state().order.clone();
    let others_now: Vec<Other> = if show_others {
        described_others(&windows, &open_windows, &found, foreground, &previous_order)
    } else {
        Vec::new()
    };
    let order = if show_others {
        others_now.iter().map(|other| other.key.clone()).collect()
    } else {
        previous_order
    };

    let after_state = State {
        found,
        others: others_now,
        order,
        foreground,
    };
    let after = snapshot_of(&after_state);
    let before = snapshot_of(&state());
    *state() = after_state;
    if before != after {
        if let Err(error) = app.emit(CHANGED_EVENT, &after) {
            log::warn!("windows: could not announce the open programs: {error}");
        }
        super::dock::refresh();
    }
}

fn click_on(app: &AppHandle, hwnds: &[isize]) -> bool {
    let Some(action) = click(hwnds, super::openwin::foreground()) else {
        return false;
    };
    match action {
        Click::Minimize(hwnd) => super::openwin::minimize(hwnd),
        Click::Focus(hwnd) => super::openwin::focus(hwnd),
    }
    // The change in front is announced by the event it causes; this is for a
    // window that was already where it should be.
    wake(app);
    true
}

/// Brings the program `id` (the dock's own id of a pinned program) to the
/// front, minimizes it, or passes to its next window (`domain::open_apps`).
/// `false` when it has no window: the caller opens it.
/// Only the windows of the workspace `workspace` count: with none there, it
/// is `false`, and the caller opens a new one (spec 0013 C2).
pub fn activate(app: &AppHandle, id: &str, workspace: Option<&str>) -> bool {
    let hwnds = state()
        .found
        .iter()
        .find(|found| found.id == id)
        .map(|found| of_workspace(&found.hwnds, workspace))
        .unwrap_or_default();
    click_on(app, &hwnds)
}

/// The same for an open program that is not pinned, named by the `key` the
/// dock was given for it. `false` when it is no longer open.
pub fn activate_other(app: &AppHandle, key: &str, workspace: Option<&str>) -> bool {
    let hwnds = state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| of_workspace(&other.hwnds, workspace))
        .unwrap_or_default();
    click_on(app, &hwnds)
}

/// Puts the open program `key` that is not pinned at `index` among the others
/// (the dock's drag). The order holds for the rest of the run: the next looks
/// keep it. Nothing when it is no longer open.
pub fn place_other(app: &AppHandle, key: &str, index: usize) {
    let after = {
        let mut state = state();
        let Some(from) = state.others.iter().position(|other| other.id == key) else {
            return;
        };
        let moved = state.others.remove(from);
        let at = index.min(state.others.len());
        state.others.insert(at, moved);
        state.order = state.others.iter().map(|other| other.key.clone()).collect();
        snapshot_of(&state)
    };
    if let Err(error) = app.emit(CHANGED_EVENT, &after) {
        log::warn!("windows: could not announce the open programs: {error}");
    }
    super::dock::refresh();
}

/// The windows of the pinned program `id` in the workspace `workspace`.
pub fn windows_of_pinned(id: &str, workspace: Option<&str>) -> Vec<isize> {
    state()
        .found
        .iter()
        .find(|found| found.id == id)
        .map(|found| of_workspace(&found.hwnds, workspace))
        .unwrap_or_default()
}

/// The same for the open program `key` that is not pinned.
pub fn windows_of_other(key: &str, workspace: Option<&str>) -> Vec<isize> {
    state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| of_workspace(&other.hwnds, workspace))
        .unwrap_or_default()
}

/// Asks every window of the pinned program `id` in the workspace
/// `workspace` to close; how many it asked.
pub fn close_pinned(app: &AppHandle, id: &str, workspace: Option<&str>) -> usize {
    close_all(app, &windows_of_pinned(id, workspace))
}

/// The same for the open program `key` that is not pinned.
pub fn close_other(app: &AppHandle, key: &str, workspace: Option<&str>) -> usize {
    close_all(app, &windows_of_other(key, workspace))
}

fn close_all(app: &AppHandle, hwnds: &[isize]) -> usize {
    for hwnd in hwnds {
        super::openwin::close(*hwnd);
    }
    // The windows that close announce themselves; this is for one that
    // refuses, or that was already gone.
    wake(app);
    hwnds.len()
}

/// What to pin for the open program `key` that is not pinned: its id for the
/// Apps folder (or its file) and its name.
pub fn pin_target_of_other(key: &str) -> Option<(String, String)> {
    state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| (other.app_id.clone(), other.name.clone()))
}

/// The AppID the windows of the open program `key` that is not pinned
/// declare, for its badge.
pub fn aumid_of_other(key: &str) -> Option<String> {
    state()
        .others
        .iter()
        .find(|other| other.id == key)
        .and_then(|other| other.aumid.clone())
}

/// What the windows of the open program `key` that is not pinned are grouped
/// by: their AppID, else their file, lowercased. For its notification badge
/// (spec 0016).
pub fn group_key_of_other(key: &str) -> Option<String> {
    state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| other.key.clone())
}

/// What the dock shows the program of each open window as, by the window's
/// handle, as at its last look: the id `app_icon` takes for it and its name.
/// A pinned program's come from `pinned` (the dock's id, the program's id
/// and its name); an open one that is not pinned, from what the dock
/// described it as. Never asks the shell. For the launcher's windows (spec
/// 0012 L7).
pub fn programs_of_windows(
    pinned: &[(String, String, String)],
) -> HashMap<isize, (String, String)> {
    let state = state();
    let mut programs = HashMap::new();
    for found in &state.found {
        let Some((_, app_id, name)) = pinned.iter().find(|(id, ..)| *id == found.id) else {
            continue;
        };
        for hwnd in &found.hwnds {
            programs.insert(*hwnd, (app_id.clone(), name.clone()));
        }
    }
    for other in &state.others {
        for hwnd in &other.hwnds {
            programs
                .entry(*hwnd)
                .or_insert_with(|| (other.app_id.clone(), other.name.clone()));
        }
    }
    programs
}

/// What the shell calls the pinned program `app_id`, if it was read already
/// (the looks at the windows read it): never asks the shell.
pub fn known_identity(app_id: &str) -> Option<Identity> {
    IDENTITIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()?
        .get(app_id)
        .cloned()
}

/// What `app_icon` and `launch` take for the open program that is not pinned
/// named by `key`: for a new instance of it. `None` when it is no longer open.
pub fn app_id_of_other(key: &str) -> Option<String> {
    state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| other.app_id.clone())
}

/// Asks the listening thread to look at the windows again.
pub fn wake(_app: &AppHandle) {
    DIRTY.store(true, Ordering::SeqCst);
    LAST_EVENT.store(now_ms(), Ordering::SeqCst);
    imp::post_wake();
}

fn now_ms() -> u64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Starts listening. Called once, at startup.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("open-windows".into())
        .spawn(move || imp::run(&handle));
    if let Err(error) = spawned {
        log::warn!("windows: the open programs can not be followed: {error}");
        return;
    }
    // The pinned programs changed: who is open may be another set.
    let woken = app.clone();
    app.listen(APPS_CHANGED_EVENT, move |_| wake(&woken));
    // "Show open programs" turned on or off.
    let woken = app.clone();
    app.listen(SETTINGS_CHANGED_EVENT, move |_| wake(&woken));
}

mod imp {
    use ::windows::Win32::Foundation::{HWND, LPARAM, WAIT_TIMEOUT, WPARAM};
    use ::windows::Win32::System::Threading::GetCurrentThreadId;
    use ::windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
    use ::windows::Win32::UI::WindowsAndMessaging::MsgWaitForMultipleObjects;
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE,
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GA_PARENT, GetAncestor,
        GetDesktopWindow, MSG, OBJID_WINDOW, PM_REMOVE, PeekMessageW, PostThreadMessageW,
        QS_ALLINPUT, TranslateMessage, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    };

    use super::*;

    /// The listening thread is woken by this message.
    const WM_WAKE: u32 = WM_APP + 1;

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        object: i32,
        child: i32,
        _thread: u32,
        _time: u32,
    ) {
        if object != OBJID_WINDOW.0 || child != 0 || hwnd.is_invalid() {
            return;
        }
        // Windows of the program and its parts come by the thousand; only
        // the top-level ones matter.
        let top_level = unsafe { GetAncestor(hwnd, GA_PARENT) } == unsafe { GetDesktopWindow() };
        if !top_level {
            return;
        }
        GEOMETRY_DIRTY.store(true, Ordering::SeqCst);
        // A move or a resize changes no program's windows.
        if event != EVENT_OBJECT_LOCATIONCHANGE {
            DIRTY.store(true, Ordering::SeqCst);
        }
        LAST_EVENT.store(now_ms(), Ordering::SeqCst);
    }

    fn hook(min: u32, max: u32) -> HWINEVENTHOOK {
        unsafe {
            SetWinEventHook(
                min,
                max,
                None,
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        }
    }

    pub fn post_wake() {
        let thread = LISTENER.load(Ordering::SeqCst);
        if thread != 0 {
            let _ = unsafe { PostThreadMessageW(thread, WM_WAKE, WPARAM(0), LPARAM(0)) };
        }
    }

    pub fn run(app: &AppHandle) {
        LISTENER.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
        let hooks = [
            hook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            hook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            hook(EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE),
            hook(EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
            // A video entering or leaving full screen is a resize.
            hook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
        ];
        // CREATE..HIDE also takes DESTROY and SHOW, which lie between. The
        // hooks live as long as the program does.
        if hooks.iter().any(|hook| hook.is_invalid()) {
            log::warn!("windows: the window events could not be followed");
        }
        // A first look: what is open when the dock starts.
        DIRTY.store(true, Ordering::SeqCst);
        LAST_EVENT.store(now_ms().saturating_sub(QUIET_MS), Ordering::SeqCst);
        loop {
            let wait = if DIRTY.load(Ordering::SeqCst) || GEOMETRY_DIRTY.load(Ordering::SeqCst) {
                let due = LAST_EVENT.load(Ordering::SeqCst) + QUIET_MS;
                due.saturating_sub(now_ms()).max(1) as u32
            } else {
                HEARTBEAT_MS
            };
            let woke = unsafe { MsgWaitForMultipleObjects(None, false, wait, QS_ALLINPUT) };
            if woke == WAIT_TIMEOUT && !DIRTY.load(Ordering::SeqCst) && a_window_is_gone() {
                DIRTY.store(true, Ordering::SeqCst);
                LAST_EVENT.store(now_ms().saturating_sub(QUIET_MS), Ordering::SeqCst);
            }
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                // A wake-up carries nothing but the flag it set.
                if message.message != WM_WAKE {
                    let _ = unsafe { TranslateMessage(&message) };
                    unsafe { DispatchMessageW(&message) };
                }
            }
            let quiet = now_ms().saturating_sub(LAST_EVENT.load(Ordering::SeqCst)) >= QUIET_MS;
            if quiet && GEOMETRY_DIRTY.swap(false, Ordering::SeqCst) {
                crate::windows::fullscreen::check(app);
                // A window taken to another monitor joins its workspace now,
                // not at the next window that opens or takes the focus.
                if crate::windows::window_spaces::changed_monitor() {
                    DIRTY.store(true, Ordering::SeqCst);
                }
            }
            if DIRTY.load(Ordering::SeqCst) && quiet {
                DIRTY.store(false, Ordering::SeqCst);
                look(app);
            }
        }
    }
}

#[cfg(all(test, windows))]
mod probe {
    use super::*;
    use crate::domain::open_apps::matches;

    /// Not a check: prints, for the machine as it is, each open window and
    /// the pinned program it is matched to (the ids in `DOCKMAX_PINNED`,
    /// separated by `|`), and, for each monitor, the topmost candidate and
    /// whether the dock counts it as full screen. Reads only.
    /// `DOCKMAX_PINNED=a|b cargo test spike_open_and_full_screen -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the windows of the machine"]
    fn spike_open_and_full_screen() {
        // Physical pixels, as the app sees them.
        let _ = unsafe {
            ::windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                ::windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            )
        };
        let pinned: Vec<String> = std::env::var("DOCKMAX_PINNED")
            .unwrap_or_default()
            .split('|')
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect();
        for window in super::super::openwin::enumerate() {
            let facts = WindowFacts {
                hwnd: window.hwnd,
                aumid: window.aumid.clone(),
                exe: window
                    .exe
                    .as_ref()
                    .map(|exe| exe.to_string_lossy().into_owned()),
            };
            let owner = pinned
                .iter()
                .find(|id| matches(&facts, &identity_of(id)))
                .map_or("-", String::as_str);
            let mut rect = ::windows::Win32::Foundation::RECT::default();
            let _ = unsafe {
                ::windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                    ::windows::Win32::Foundation::HWND(window.hwnd as _),
                    &mut rect,
                )
            };
            println!(
                "{:?} | {},{},{},{} | aumid={:?} | exe={:?} | pinned={owner}",
                window.title,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                window.aumid,
                window.exe
            );
        }
        let candidates = super::super::openwin::fullscreen_candidates();
        for monitor in super::super::monitors::all() {
            let top = candidates
                .iter()
                .find(|candidate| candidate.monitor == monitor.handle);
            let bounds = crate::domain::fullscreen::Bounds {
                left: monitor.rect.x,
                top: monitor.rect.y,
                right: monitor.rect.x + monitor.rect.width as i32,
                bottom: monitor.rect.y + monitor.rect.height as i32,
            };
            println!(
                "{} {:?}: top {:?} -> full screen {}",
                monitor.key,
                monitor.rect,
                top,
                crate::domain::fullscreen::is_fullscreen(&candidates, monitor.handle, bounds)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_with_no_description_is_looked_at_again_after_a_while() {
        let found = Instant::now();
        let none: Description = None;
        let some: Description = Some(("app".into(), "App".into()));
        assert!(still_good(&none, found, found));
        assert!(still_good(&none, found, found + NOT_FOUND_FOR / 2));
        assert!(!still_good(&none, found, found + NOT_FOUND_FOR));
        // One that was found is good for as long as the cache keeps it.
        assert!(still_good(&some, found, found + NOT_FOUND_FOR * 100));
    }
}
