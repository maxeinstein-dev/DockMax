//! Which pinned programs are open, and what a click on one does (spec 0009).
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

use crate::commands::apps::CHANGED_EVENT as APPS_CHANGED_EVENT;
use crate::commands::widgets::lock;
use crate::domain::open_apps::{Click, Found, Identity, OpenApp, WindowFacts, click, summarize};
use crate::persistence::database::Storage;

/// Event sent to every window with the `[OpenApp]` list after a change.
pub const CHANGED_EVENT: &str = "open-windows-changed";

/// What the windows of the pinned programs were at the last look.
static FOUND: Mutex<Vec<Found>> = Mutex::new(Vec::new());

/// What the shell calls each pinned program, by the program's id in the Apps
/// folder: read once, as it costs the shell's time and does not change while
/// the program stays pinned.
static IDENTITIES: Mutex<Option<HashMap<String, Identity>>> = Mutex::new(None);

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
    lock_found()
        .iter()
        .flat_map(|found| found.hwnds.iter())
        .any(|hwnd| !super::openwin::exists(*hwnd))
}

fn lock_found() -> std::sync::MutexGuard<'static, Vec<Found>> {
    FOUND.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The pinned programs that have windows now, as the dock sees them.
pub fn snapshot() -> Vec<OpenApp> {
    lock_found().iter().map(OpenApp::from).collect()
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

/// Looks at the windows and, if what the dock shows changed, tells it.
fn look(app: &AppHandle) {
    super::apps::init_com();
    let pinned = {
        let state = app.state::<std::sync::Mutex<Storage>>();
        match lock(&state).apps_snapshot() {
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
    let windows: Vec<WindowFacts> = super::openwin::enumerate()
        .into_iter()
        .map(|window| WindowFacts {
            hwnd: window.hwnd,
            aumid: window.aumid,
            exe: window.exe.and_then(|path| path.to_str().map(str::to_owned)),
        })
        .collect();
    let found = summarize(&programs, &windows, super::openwin::foreground());
    let before = snapshot_of(&lock_found());
    let after = snapshot_of(&found);
    *lock_found() = found;
    if before != after
        && let Err(error) = app.emit(CHANGED_EVENT, &after)
    {
        log::warn!("windows: could not announce the open programs: {error}");
    }
}

fn snapshot_of(found: &[Found]) -> Vec<OpenApp> {
    found.iter().map(OpenApp::from).collect()
}

/// Brings the program `id` (the dock's own id of a pinned program) to the
/// front, minimizes it, or passes to its next window (`domain::open_apps`).
/// `false` when it has no window: the caller opens it.
pub fn activate(app: &AppHandle, id: &str) -> bool {
    let hwnds = lock_found()
        .iter()
        .find(|found| found.id == id)
        .map(|found| found.hwnds.clone())
        .unwrap_or_default();
    let Some(action) = click(&hwnds, super::openwin::foreground()) else {
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
