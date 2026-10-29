//! The Uso do Claude (spec 0017 F5): the plan's limits, asked of Anthropic
//! as the Claude Code's `/usage` asks them, every five minutes while a dock
//! has the widget and when its panel opens. The Claude Code's sign-in is
//! read from its credentials file at each ask and kept nowhere; the token
//! never reaches a log or the page. An expired sign-in is renewed by the
//! Claude Code itself: `claude auth status` renews it without spending use.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, Once};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::domain::claude_usage::{self, Usage};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The header the Claude Code sends with its sign-in.
const BETA: (&str, &str) = ("anthropic-beta", "oauth-2025-04-20");
const EVERY: Duration = Duration::from_secs(5 * 60);

/// What the widget shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Reading {
    /// The usage, and when it was read (Unix seconds).
    Ready { usage: Usage, at: i64 },
    /// No Claude Code signed in on this computer.
    SignedOut,
    /// The sign-in expired and the Claude Code couldn't renew it.
    Expired,
    /// Anthropic didn't answer, or not as expected.
    Unavailable,
}

static STATE: Mutex<Option<Reading>> = Mutex::new(None);
static WANTERS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static WANTED: AtomicBool = AtomicBool::new(false);
static STARTED: Once = Once::new();
static WAKE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

/// The last reading; `None` before the first.
pub fn reading() -> Option<Reading> {
    *STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether the dock `dock` has a Uso do Claude: the reading runs while any
/// has one.
pub fn want(dock: isize, wanted: bool) {
    let on = {
        let mut wanters = WANTERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        wanters.retain(|each| *each != dock);
        if wanted {
            wanters.push(dock);
        }
        !wanters.is_empty()
    };
    let was = WANTED.swap(on, Ordering::Relaxed);
    if on {
        STARTED.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("claude-usage".into())
                .spawn(run);
        });
        if !was {
            read_now();
        }
    }
}

/// Reads again now (the panel opened).
pub fn read_now() {
    let (lock, wake) = &WAKE;
    *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    wake.notify_all();
}

/// The Claude Code's credentials file: `%USERPROFILE%\.claude\.credentials.json`.
fn credentials_file() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("USERPROFILE")?;
    Some(
        std::path::Path::new(&home)
            .join(".claude")
            .join(".credentials.json"),
    )
}

/// `CREATE_NO_WINDOW`: the renewal never shows a console.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// How long the Claude Code may take to renew.
const RENEWAL: Duration = Duration::from_secs(30);

/// Asks Anthropic for the usage, renewing an expired sign-in once.
fn read() -> Reading {
    match ask() {
        Reading::Expired if renew() => ask(),
        reading => reading,
    }
}

/// Has the Claude Code renew its sign-in (it writes the credentials file
/// itself); whether it ran and finished well in time.
fn renew() -> bool {
    use std::os::windows::process::CommandExt;
    let child = std::process::Command::new("cmd")
        .args(["/C", "claude", "auth", "status"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    let Ok(mut child) = child else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < RENEWAL => {
                std::thread::sleep(Duration::from_millis(250));
            }
            _ => {
                let _ = child.kill();
                log::warn!("claude usage: the Claude Code didn't renew in time");
                return false;
            }
        }
    }
}

/// Opens a terminal running the Claude Code's sign-in.
pub fn sign_in() -> Result<(), ()> {
    use std::os::windows::process::CommandExt;
    // `start` gives the sign-in a console of its own, which stays open
    // (`/K`) to show how it went.
    std::process::Command::new("cmd")
        .args([
            "/C",
            "start",
            "Claude Code",
            "cmd",
            "/K",
            "claude",
            "auth",
            "login",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(drop)
        .map_err(|error| log::warn!("claude usage: no terminal for the sign-in: {error}"))
}

/// Asks Anthropic for the usage once.
fn ask() -> Reading {
    let Some(credentials) = credentials_file()
        .and_then(|file| std::fs::read(file).ok())
        .and_then(|json| claude_usage::parse_credentials(&json))
    else {
        return Reading::SignedOut;
    };
    let now_ms = jiff::Timestamp::now().as_millisecond();
    if credentials
        .expires_at
        .is_some_and(|expires| expires <= now_ms)
    {
        return Reading::Expired;
    }
    match super::http::get_with(USAGE_URL, &credentials.access_token, &[BETA]) {
        Ok(response) if response.status == 200 => {
            claude_usage::parse_usage(&response.body).map_or(Reading::Unavailable, |usage| {
                Reading::Ready {
                    usage,
                    at: jiff::Timestamp::now().as_second(),
                }
            })
        }
        Ok(response) if response.status == 401 => Reading::Expired,
        Ok(response) => {
            log::warn!("claude usage: Anthropic answered {}", response.status);
            Reading::Unavailable
        }
        Err(error) => {
            log::warn!("claude usage: no answer: {error:?}");
            Reading::Unavailable
        }
    }
}

fn run() {
    let mut last: Option<Instant> = None;
    loop {
        {
            let (lock, wake) = &WAKE;
            let mut asked = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let wait = last.map_or(Duration::ZERO, |at| EVERY.saturating_sub(at.elapsed()));
            if !*asked && !wait.is_zero() {
                asked = wake
                    .wait_timeout(asked, wait)
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .0;
            }
            *asked = false;
        }
        if !WANTED.load(Ordering::Relaxed) {
            last = Some(Instant::now());
            continue;
        }
        let next = read();
        last = Some(Instant::now());
        let changed = {
            let mut state = STATE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let changed = *state != Some(next);
            *state = Some(next);
            changed
        };
        if changed {
            super::native_dock::calendar_changed();
        }
    }
}
