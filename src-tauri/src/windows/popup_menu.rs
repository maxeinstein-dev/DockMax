//! The menu a program opens from the tray panel keeps the panel open.
//!
//! A right click on a tray icon makes its program open its own menu (the
//! FortiClient VPNs, Docker's actions). That menu takes the foreground, so
//! the panel loses focus, and a panel hides when it loses focus. The panel
//! should stay: the user clicks another icon, or the same one again.
//!
//! The panel is held open from the click until the menu is gone: Windows says
//! when a menu starts and ends (`EVENT_SYSTEM_MENUPOPUPSTART` and `END`, one
//! pair for each submenu). If no menu starts in a few seconds (the program
//! showed none), the hold ends by itself. When the menu ends, the panel gets
//! the focus back, unless the user clicked in another window, in which case
//! the panel closes as it always did.
//!
//! Touches nothing of Tauri: the app tells it what to do when the hold ends
//! (`set_handler`).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

#[cfg(windows)]
use ::windows::Win32::Foundation::{HMODULE, HWND};
#[cfg(windows)]
use ::windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
#[cfg(windows)]
use ::windows::Win32::UI::WindowsAndMessaging::{
    EVENT_SYSTEM_MENUPOPUPEND, EVENT_SYSTEM_MENUPOPUPSTART, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

/// How long a click waits for its program's menu before the panel stops being
/// held.
pub const WAIT: Duration = Duration::from_secs(4);

/// Where the hold is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No click is waiting for a menu.
    Idle,
    /// A right click was sent at this moment; its menu has not started.
    Waiting(Instant),
    /// A menu is open: the number of menus and submenus not yet closed.
    Open(u32),
}

/// The state of the hold, apart from the clock and from Windows, so it can be
/// tested.
#[derive(Debug)]
pub struct MenuHold {
    phase: Phase,
}

impl MenuHold {
    pub const fn new() -> Self {
        MenuHold { phase: Phase::Idle }
    }

    /// A right click was sent: its program may open a menu.
    pub fn expect(&mut self, now: Instant) {
        // A menu already open stays open: the new click is for it.
        if !matches!(self.phase, Phase::Open(_)) {
            self.phase = Phase::Waiting(now);
        }
    }

    /// Whether losing focus must not hide the panel now.
    pub fn blocks_blur(&self, now: Instant) -> bool {
        match self.phase {
            Phase::Idle => false,
            Phase::Waiting(since) => now.saturating_duration_since(since) <= WAIT,
            Phase::Open(_) => true,
        }
    }

    /// A menu (or a submenu) started. Menus nobody asked for are not ours.
    pub fn menu_started(&mut self) {
        self.phase = match self.phase {
            Phase::Idle => Phase::Idle,
            Phase::Waiting(_) => Phase::Open(1),
            Phase::Open(depth) => Phase::Open(depth + 1),
        };
    }

    /// A menu (or a submenu) ended. Returns whether it was the last one, and
    /// the hold is over.
    pub fn menu_ended(&mut self) -> bool {
        match self.phase {
            Phase::Open(1) => {
                self.phase = Phase::Idle;
                true
            }
            Phase::Open(depth) => {
                self.phase = Phase::Open(depth - 1);
                false
            }
            _ => false,
        }
    }

    /// The wait for a menu ran out. Returns whether it did, and the hold is
    /// over.
    pub fn expire(&mut self, now: Instant) -> bool {
        match self.phase {
            Phase::Waiting(since) if now.saturating_duration_since(since) > WAIT => {
                self.phase = Phase::Idle;
                true
            }
            _ => false,
        }
    }
}

impl Default for MenuHold {
    fn default() -> Self {
        Self::new()
    }
}

static HOLD: Mutex<MenuHold> = Mutex::new(MenuHold::new());

/// What to do when the hold ends: the app puts the focus back on the panel,
/// or closes it.
type Handler = Arc<dyn Fn() + Send + Sync>;
static HANDLER: Mutex<Option<Handler>> = Mutex::new(None);

fn hold() -> std::sync::MutexGuard<'static, MenuHold> {
    HOLD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Sets what happens when the hold ends.
pub fn set_handler(handler: impl Fn() + Send + Sync + 'static) {
    *HANDLER.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(handler));
}

fn finished() {
    let handler = HANDLER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    if let Some(handler) = handler {
        handler();
    }
}

/// Whether the panel must stay open although it lost focus.
pub fn blocks_blur() -> bool {
    hold().blocks_blur(Instant::now())
}

/// A right click was sent to a program: holds the panel open for its menu,
/// and lets go if none comes.
pub fn expect_menu() {
    hold().expect(Instant::now());
    let spawned = std::thread::Builder::new()
        .name("menu-wait".into())
        .spawn(|| {
            std::thread::sleep(WAIT + Duration::from_millis(200));
            if hold().expire(Instant::now()) {
                finished();
            }
        });
    if let Err(error) = spawned {
        log::warn!("tray: the wait for the program's menu did not start: {error}");
    }
}

#[cfg(windows)]
unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    _hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    match event {
        EVENT_SYSTEM_MENUPOPUPSTART => hold().menu_started(),
        // The guard is the work: it counts the menu out, and is true for the
        // last one.
        EVENT_SYSTEM_MENUPOPUPEND if hold().menu_ended() => finished(),
        _ => {}
    }
}

/// Starts listening to the menus of other programs. Must be called from a
/// thread that runs a message loop, which delivers the events.
#[cfg(windows)]
pub fn install_hook() -> Option<HWINEVENTHOOK> {
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MENUPOPUPSTART,
            EVENT_SYSTEM_MENUPOPUPEND,
            Some(HMODULE::default()),
            Some(on_event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    (!hook.is_invalid()).then_some(hook)
}

#[cfg(windows)]
pub fn remove_hook(hook: HWINEVENTHOOK) {
    let _ = unsafe { UnhookWinEvent(hook) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later(now: Instant, millis: u64) -> Instant {
        now + Duration::from_millis(millis)
    }

    #[test]
    fn nothing_is_held_until_a_click_asks_for_a_menu() {
        let now = Instant::now();
        let mut hold = MenuHold::new();

        assert!(!hold.blocks_blur(now));
        // A menu nobody asked for (the user's own) changes nothing.
        hold.menu_started();
        assert!(!hold.blocks_blur(now));
        assert!(!hold.menu_ended());
    }

    #[test]
    fn a_click_holds_the_panel_while_it_waits_for_the_menu() {
        let now = Instant::now();
        let mut hold = MenuHold::new();

        hold.expect(now);

        assert!(hold.blocks_blur(later(now, 100)));
        assert!(hold.blocks_blur(now + WAIT));
        assert!(!hold.blocks_blur(now + WAIT + Duration::from_millis(1)));
    }

    #[test]
    fn the_hold_lasts_as_long_as_the_menu_and_ends_with_it() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(now);

        hold.menu_started();
        // However long the user takes to choose.
        assert!(hold.blocks_blur(later(now, 600_000)));
        assert!(hold.menu_ended());
        assert!(!hold.blocks_blur(later(now, 600_001)));
    }

    #[test]
    fn a_submenu_does_not_end_the_hold() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(now);

        hold.menu_started();
        hold.menu_started();
        assert!(!hold.menu_ended(), "the submenu closed, the menu is open");
        assert!(hold.blocks_blur(later(now, 10)));
        assert!(hold.menu_ended());
        assert!(!hold.blocks_blur(later(now, 20)));
    }

    #[test]
    fn a_wait_without_a_menu_runs_out_by_itself() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(now);

        assert!(!hold.expire(later(now, 1000)), "still waiting");
        assert!(hold.expire(now + WAIT + Duration::from_millis(1)));
        assert!(!hold.expire(now + WAIT * 2), "only once");
        assert!(!hold.blocks_blur(now + WAIT * 2));
    }

    #[test]
    fn a_wait_that_got_its_menu_does_not_run_out() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(now);
        hold.menu_started();

        assert!(!hold.expire(now + WAIT * 10));
        assert!(hold.blocks_blur(now + WAIT * 10));
    }

    #[test]
    fn a_second_click_while_the_menu_is_open_leaves_it_open() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(now);
        hold.menu_started();

        hold.expect(later(now, 5000));

        assert!(hold.blocks_blur(later(now, 100_000)));
        assert!(hold.menu_ended());
    }

    #[test]
    fn an_end_with_nothing_open_is_not_the_end_of_a_hold() {
        let mut hold = MenuHold::new();

        assert!(!hold.menu_ended());
        hold.expect(Instant::now());
        assert!(!hold.menu_ended(), "no menu had started");
    }

    #[test]
    fn a_clock_that_went_back_does_not_end_the_wait() {
        let now = Instant::now();
        let mut hold = MenuHold::new();
        hold.expect(later(now, 10_000));

        assert!(hold.blocks_blur(now));
        assert!(!hold.expire(now));
    }
}
