//! The panel window on demand (spec 0011 H2): made when a panel is about to
//! open (or the pointer rests on an item that opens one, which warms it), and
//! destroyed after `KEEP_WARM` with no panel open, so nothing of it stays
//! loaded the rest of the day.
//!
//! A new window's page is not listening yet when the dock asks for a panel:
//! the show waits here (`pending`) and the page takes it when it has mounted
//! (`mounted`), so no `panel-show` is lost.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::panel::{LABEL, PanelId};

/// How long the window stays made after the last use (a panel closed, or a
/// warm-up nobody clicked), before it is destroyed. Opening a panel again
/// within it is warm (~110-140 ms to paint instead of ~360 ms, spec 0011 §2).
pub const KEEP_WARM: Duration = Duration::from_secs(45);

/// A show asked for while the window was being made or its page loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub panel_id: PanelId,
    pub wait: u64,
}

struct Life {
    /// The window is being made (off the main thread).
    making: bool,
    /// Its page has mounted and listens for `panel-show`.
    mounted: bool,
    pending: Option<Pending>,
    /// The last time the window was wanted.
    last_use: Instant,
}

static LIFE: Mutex<Option<Life>> = Mutex::new(None);

/// Whether a check that may destroy the window is already waiting.
static CHECKING: AtomicBool = AtomicBool::new(false);

fn with_life<T>(change: impl FnOnce(&mut Life) -> T) -> T {
    let mut life = LIFE.lock().unwrap_or_else(PoisonError::into_inner);
    let life = life.get_or_insert_with(|| Life {
        making: false,
        mounted: false,
        pending: None,
        last_use: Instant::now(),
    });
    change(life)
}

/// The window, if it is made and its page has mounted: ready for a show.
pub fn ready(app: &AppHandle) -> Option<WebviewWindow> {
    let window = app.get_webview_window(LABEL)?;
    with_life(|life| life.mounted).then_some(window)
}

/// Keeps a show for the page to take when it has mounted, and makes the
/// window if it is not there.
pub fn defer(app: &AppHandle, pending: Pending) {
    with_life(|life| life.pending = Some(pending));
    ensure(app);
}

/// The page has mounted and listens: returns the show waiting for it, if
/// any.
pub fn mounted() -> Option<Pending> {
    with_life(|life| {
        life.mounted = true;
        life.last_use = Instant::now();
        life.pending.take()
    })
}

/// The pointer rests on an item that opens a panel: makes the window now,
/// so the click finds it warm.
pub fn warm(app: &AppHandle) {
    ensure(app);
}

/// The window was used now (a panel closed): it stays for `KEEP_WARM`.
pub fn touch(app: &AppHandle) {
    with_life(|life| life.last_use = Instant::now());
    check_later(app);
}

/// Makes the window if it is neither there nor being made.
fn ensure(app: &AppHandle) {
    with_life(|life| life.last_use = Instant::now());
    check_later(app);
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let start = with_life(|life| !std::mem::replace(&mut life.making, true));
    if !start {
        return;
    }
    let app = app.clone();
    // Made off the main thread: the callers (commands) run on it, and making
    // a webview window there waits on the main thread itself.
    let spawned = thread::Builder::new()
        .name("panel-window".into())
        .spawn(move || {
            let built =
                WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("panel.html".into()))
                    .title("DockMax")
                    .inner_size(360.0, 480.0)
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .resizable(false)
                    .skip_taskbar(true)
                    .shadow(false)
                    .visible(false)
                    .focused(false)
                    .additional_browser_args(super::BROWSER_ARGS)
                    .build();
            match built {
                Ok(window) => {
                    let handle = app.clone();
                    // The window's own setup (subclass, events) on its
                    // thread.
                    let installed = app.run_on_main_thread(move || {
                        if let Err(error) = super::panel::install(&handle, &window) {
                            log::warn!("panel: the window could not be set up: {error}");
                        }
                    });
                    if let Err(error) = installed {
                        log::warn!("panel: the window setup was not scheduled: {error}");
                    }
                }
                Err(error) => log::warn!("panel: the window could not be made: {error}"),
            }
            with_life(|life| life.making = false);
        });
    if let Err(error) = spawned {
        log::warn!("panel: the window was not made: {error}");
        with_life(|life| life.making = false);
    }
}

/// After `KEEP_WARM` without use, and with no panel open or waiting,
/// destroys the window. One check waits at a time; a use meanwhile moves
/// the deadline, and the check waits again.
fn check_later(app: &AppHandle) {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("panel-window-release".into())
        .spawn(move || {
            loop {
                let idle = with_life(|life| life.last_use.elapsed());
                if idle < KEEP_WARM {
                    thread::sleep(KEEP_WARM - idle);
                    continue;
                }
                let busy = super::panel::lock(&app).open().is_some()
                    || with_life(|life| life.pending.is_some() || life.making);
                if busy {
                    with_life(|life| life.last_use = Instant::now());
                    continue;
                }
                break;
            }
            CHECKING.store(false, Ordering::SeqCst);
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || release(&handle));
        });
    if spawned.is_err() {
        CHECKING.store(false, Ordering::SeqCst);
    }
}

/// Destroys the window, unless it was wanted again meanwhile.
fn release(app: &AppHandle) {
    let wanted = with_life(|life| life.last_use.elapsed() < KEEP_WARM || life.pending.is_some())
        || super::panel::lock(app).open().is_some();
    if wanted {
        check_later(app);
        return;
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        with_life(|life| life.mounted = false);
        log::debug!("panel: released the window after a while unused");
        if let Err(error) = window.destroy() {
            log::warn!("panel: the window could not be released: {error}");
        }
    }
}
