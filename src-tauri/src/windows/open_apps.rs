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
}

static STATE: Mutex<State> = Mutex::new(State {
    found: Vec::new(),
    others: Vec::new(),
    order: Vec::new(),
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
/// id for the icon and its name, or `None` for one with no way to show it.
static DESCRIPTIONS: Mutex<Option<HashMap<String, Description>>> = Mutex::new(None);

/// The programs of the Apps folder by the file they run (lowercased): the id
/// and the name. Read once, when the first open program that needs it shows
/// up: it takes the shell about a second.
static CATALOG_BY_FILE: Mutex<Option<Vec<(String, String, String)>>> = Mutex::new(None);

/// A look at the windows is wanted (an event came, or the pinned programs
/// changed).
static DIRTY: AtomicBool = AtomicBool::new(false);

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
    if let Some(known) = DESCRIPTIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|known| known.get(key))
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
    DESCRIPTIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(HashMap::new)
        .insert(key.to_owned(), described.clone());
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
    };
    let after = snapshot_of(&after_state);
    let before = snapshot_of(&state());
    *state() = after_state;
    if before != after
        && let Err(error) = app.emit(CHANGED_EVENT, &after)
    {
        log::warn!("windows: could not announce the open programs: {error}");
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
pub fn activate(app: &AppHandle, id: &str) -> bool {
    let hwnds = state()
        .found
        .iter()
        .find(|found| found.id == id)
        .map(|found| found.hwnds.clone())
        .unwrap_or_default();
    click_on(app, &hwnds)
}

/// The same for an open program that is not pinned, named by the `key` the
/// dock was given for it. `false` when it is no longer open.
pub fn activate_other(app: &AppHandle, key: &str) -> bool {
    let hwnds = state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| other.hwnds.clone())
        .unwrap_or_default();
    click_on(app, &hwnds)
}

/// Asks every window of the pinned program `id` to close; how many it asked.
pub fn close_pinned(app: &AppHandle, id: &str) -> usize {
    let hwnds: Vec<isize> = state()
        .found
        .iter()
        .find(|found| found.id == id)
        .map(|found| found.hwnds.clone())
        .unwrap_or_default();
    close_all(app, &hwnds)
}

/// The same for the open program `key` that is not pinned.
pub fn close_other(app: &AppHandle, key: &str) -> usize {
    let hwnds: Vec<isize> = state()
        .others
        .iter()
        .find(|other| other.id == key)
        .map(|other| other.hwnds.clone())
        .unwrap_or_default();
    close_all(app, &hwnds)
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
        EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND,
        EVENT_SYSTEM_MINIMIZESTART, GA_PARENT, GetAncestor, GetDesktopWindow, MSG, OBJID_WINDOW,
        PM_REMOVE, PeekMessageW, PostThreadMessageW, QS_ALLINPUT, TranslateMessage,
        WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    };

    use super::*;

    /// The listening thread is woken by this message.
    const WM_WAKE: u32 = WM_APP + 1;

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        _event: u32,
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
        DIRTY.store(true, Ordering::SeqCst);
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
            let wait = if DIRTY.load(Ordering::SeqCst) {
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
            if DIRTY.load(Ordering::SeqCst) && quiet {
                DIRTY.store(false, Ordering::SeqCst);
                look(app);
            }
        }
    }
}
