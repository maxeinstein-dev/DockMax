//! The notification area, hosted by the dock while the Windows taskbar is
//! hidden (ADR 0009, Fase E).
//!
//! An app puts its icon in the notification area with `Shell_NotifyIcon`,
//! which finds the window of class `Shell_TrayWnd` and sends it the icon as
//! `WM_COPYDATA`. The same window receives the app bar messages
//! (`SHAppBarMessage`) and a few more, so a window of that class cannot just
//! take the taskbar's place: everything it does not handle itself would stop
//! working for every program.
//!
//! So the host is a **proxy**. It is a window of class `Shell_TrayWnd` of its
//! own, on top of the taskbar's in the z-order (which is how `FindWindow`
//! picks), that looks at the icons going by and forwards every message,
//! unchanged, to the real taskbar window, and answers with its answer.
//! Explorer's taskbar goes on working, hidden, with the same icons, so
//! nothing is lost if the host goes away: the window dies with the process and
//! programs find Explorer's again. Programs that started before the host did
//! are asked to send their icons again with the `TaskbarCreated` broadcast,
//! the one Explorer sends when it starts.
//!
//! What goes by is data from any program on the machine: it is read with
//! bounds checks and never trusted (`parse`).
//!
//! This module touches nothing of Tauri: the example `tray_probe` runs it on
//! its own.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;

use ::windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use ::windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, HGDIOBJ, ReleaseDC,
};
use ::windows::Win32::System::DataExchange::COPYDATASTRUCT;
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CopyIcon, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyWindow, DispatchMessageW, EnumWindows, GetClassNameW, GetCursorPos, GetIconInfo,
    GetMessageW, GetWindowThreadProcessId, HICON, HWND_BROADCAST, HWND_NOTOPMOST, HWND_TOPMOST,
    ICONINFO, IsWindow, KillTimer, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SendMessageTimeoutW, SetTimer, SetWindowPos, TranslateMessage, WM_CLOSE, WM_COPYDATA,
    WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use ::windows::core::{BOOL, PCWSTR, w};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// The class of the taskbar's window, which programs look for.
const TRAY_CLASS: &str = "Shell_TrayWnd";

/// `COPYDATASTRUCT::dwData` of an icon message. Others (the app bars, 0)
/// are only forwarded.
const NOTIFY_ICON_DATA: usize = 1;

/// Marks the start of what `Shell_NotifyIcon` sends.
const TRAY_MAGIC: u32 = 0x3475_3423;

/// How long a burst of changes waits before the interface is told once.
const CHANGE_DELAY_MS: u32 = 120;
const TIMER_ID: usize = 1;

/// Programs answer `TaskbarCreated` at their own pace (some take their icon
/// down and put it up again, some are slow to start), so the broadcast goes
/// out again a few times in the first seconds. Not a loop that runs for ever.
const REBROADCAST_ID: usize = 2;
const REBROADCAST_PAUSES_MS: [u32; 4] = [1500, 2500, 4000, 8000];

/// The pause before the `index`th repeat of the broadcast, or `None` after
/// the last one.
fn rebroadcast_pause(index: usize) -> Option<u32> {
    REBROADCAST_PAUSES_MS.get(index).copied()
}

/// How many repeats of the broadcast have gone out.
static REBROADCASTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Asks every program to send its icon again, the way Explorer does when it
/// starts.
fn broadcast_taskbar_created() {
    let message = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    let _ = unsafe { PostMessageW(Some(HWND_BROADCAST), message, WPARAM(0), LPARAM(0)) };
}

/// The largest icon read, in pixels a side: icons are 16 to 48 in practice.
const MAX_ICON_SIDE: i32 = 256;

/// What `Shell_NotifyIcon` was asked to do (`NIM_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Add,
    Modify,
    Delete,
    SetFocus,
    SetVersion,
}

impl Action {
    fn from_raw(value: u32) -> Option<Action> {
        Some(match value {
            0 => Action::Add,
            1 => Action::Modify,
            2 => Action::Delete,
            3 => Action::SetFocus,
            4 => Action::SetVersion,
            _ => return None,
        })
    }
}

const NIF_MESSAGE: u32 = 0x01;
const NIF_ICON: u32 = 0x02;
const NIF_TIP: u32 = 0x04;
const NIF_STATE: u32 = 0x08;
const NIS_HIDDEN: u32 = 0x01;

/// One `Shell_NotifyIcon` call, as it goes by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub action: Action,
    /// The window of the program, which gets the clicks; handles in this
    /// message are 32 bits wide whatever the program, so they are widened
    /// the way Windows does.
    pub hwnd: isize,
    pub uid: u32,
    pub flags: u32,
    pub callback: u32,
    pub icon: isize,
    pub tip: String,
    pub state: u32,
    pub state_mask: u32,
    pub version: u32,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// A 32-bit handle as the number Windows widens it to (sign-extended).
fn widen(handle: u32) -> isize {
    handle as i32 as isize
}

/// A null-terminated UTF-16 string in a fixed field of `bytes`.
fn text_at(bytes: &[u8], at: usize, units: usize) -> Option<String> {
    let field = bytes.get(at..at + 2 * units)?;
    let units: Vec<u16> = field
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .take_while(|&unit| unit != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

/// Reads the data of an icon message (`SHELLTRAYDATA`: a magic number, the
/// action, then the `NOTIFYICONDATA`). `None` for anything that is not one,
/// is cut short or has an action it does not know: the proxy then only
/// forwards it.
pub fn parse(bytes: &[u8]) -> Option<Message> {
    if u32_at(bytes, 0)? != TRAY_MAGIC {
        return None;
    }
    let action = Action::from_raw(u32_at(bytes, 4)?)?;
    // NOTIFYICONDATAW from byte 8: cbSize, hWnd, uID, uFlags,
    // uCallbackMessage, hIcon, szTip[128], dwState, dwStateMask, szInfo[256],
    // uTimeout/uVersion.
    let nid = 8;
    Some(Message {
        action,
        hwnd: widen(u32_at(bytes, nid + 4)?),
        uid: u32_at(bytes, nid + 8)?,
        flags: u32_at(bytes, nid + 12)?,
        callback: u32_at(bytes, nid + 16)?,
        icon: widen(u32_at(bytes, nid + 20)?),
        tip: text_at(bytes, nid + 24, 128).unwrap_or_default(),
        state: u32_at(bytes, nid + 280).unwrap_or(0),
        state_mask: u32_at(bytes, nid + 284).unwrap_or(0),
        version: u32_at(bytes, nid + 800).unwrap_or(0),
    })
}

/// An icon of the notification area, as the interface sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// `hwnd:uid`: names the icon for a click.
    pub key: String,
    pub hwnd: isize,
    pub uid: u32,
    pub callback: u32,
    pub version: u32,
    pub tip: String,
    /// The image, a PNG as a `data:` URL; `None` when it could not be read.
    pub png: Option<String>,
    /// The program asked not to show it.
    pub hidden: bool,
}

fn key_of(hwnd: isize, uid: u32) -> String {
    format!("{hwnd}:{uid}")
}

/// Applies a message to the list of icons. Returns whether something the
/// interface shows changed. `png` is the image when the message carries a
/// new one.
fn apply(icons: &mut Vec<Icon>, message: &Message, png: Option<Option<String>>) -> bool {
    let key = key_of(message.hwnd, message.uid);
    let at = icons.iter().position(|icon| icon.key == key);
    match message.action {
        Action::Delete => match at {
            Some(index) => {
                icons.remove(index);
                true
            }
            None => false,
        },
        Action::SetFocus => false,
        Action::SetVersion => match at {
            Some(index) => {
                icons[index].version = message.version;
                false
            }
            None => false,
        },
        Action::Add | Action::Modify => {
            let index = at.unwrap_or_else(|| {
                icons.push(Icon {
                    key,
                    hwnd: message.hwnd,
                    uid: message.uid,
                    callback: 0,
                    version: 0,
                    tip: String::new(),
                    png: None,
                    hidden: false,
                });
                icons.len() - 1
            });
            let before = icons[index].clone();
            let icon = &mut icons[index];
            if message.flags & NIF_MESSAGE != 0 {
                icon.callback = message.callback;
            }
            if message.flags & NIF_TIP != 0 {
                icon.tip = message.tip.clone();
            }
            if message.flags & NIF_STATE != 0 && message.state_mask & NIS_HIDDEN != 0 {
                icon.hidden = message.state & NIS_HIDDEN != 0;
            }
            if let Some(png) = png {
                icon.png = png;
            }
            at.is_none() || *icon != before
        }
    }
}

struct Host {
    icons: Vec<Icon>,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    timer_pending: bool,
}

static HOST: Mutex<Host> = Mutex::new(Host {
    icons: Vec::new(),
    on_change: None,
    timer_pending: false,
});

/// Prints each icon message that goes by, for the probe.
static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Turns the printing of each icon message on or off (the probe's).
#[allow(dead_code, reason = "the probe and a tracing run turn it on")]
pub fn trace(on: bool) {
    TRACE.store(on, Ordering::Relaxed);
}

/// The window of the proxy (an `HWND` as a number), 0 while there is none.
static WINDOW: AtomicIsize = AtomicIsize::new(0);

fn host() -> MutexGuard<'static, Host> {
    HOST.lock().unwrap_or_else(PoisonError::into_inner)
}

fn window() -> Option<HWND> {
    match WINDOW.load(Ordering::Acquire) {
        0 => None,
        value => Some(HWND(value as *mut _)),
    }
}

/// The icons now, without those whose program is gone (a program that died
/// without removing its icon leaves it behind).
pub fn icons() -> Vec<Icon> {
    let mut host = host();
    host.icons.retain(|icon| {
        let alive = unsafe { IsWindow(Some(HWND(icon.hwnd as *mut _))) }.as_bool();
        if !alive && TRACE.load(Ordering::Relaxed) {
            log::warn!("tray trace: dropped {:?} (its window is gone)", icon.tip);
        }
        alive
    });
    host.icons.clone()
}

/// The taskbar's own window: the `Shell_TrayWnd` that is not ours.
fn real_tray() -> Option<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, found: LPARAM) -> BOOL {
        let mut name = [0u16; 32];
        let length = unsafe { GetClassNameW(hwnd, &mut name) };
        let name = String::from_utf16_lossy(&name[..usize::try_from(length).unwrap_or(0)]);
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process)) };
        if name == TRAY_CLASS && process != std::process::id() {
            let found = unsafe { &mut *(found.0 as *mut Option<HWND>) };
            *found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut found: Option<HWND> = None;
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut found as *mut _ as isize)) };
    found
}

/// Passes `msg` on to the taskbar's window and returns its answer; `fallback`
/// when there is none (Explorer is restarting) or it does not answer.
fn forward(msg: u32, wparam: WPARAM, lparam: LPARAM, fallback: isize) -> LRESULT {
    let Some(target) = real_tray() else {
        return LRESULT(fallback);
    };
    let mut result = 0;
    let sent = unsafe {
        SendMessageTimeoutW(
            target,
            msg,
            wparam,
            lparam,
            SMTO_ABORTIFHUNG,
            2000,
            Some(&mut result),
        )
    };
    if sent.0 == 0 {
        LRESULT(fallback)
    } else {
        LRESULT(result as isize)
    }
}

/// `COPYDATASTRUCT::dwData` of the question "where is this icon?"
/// (`Shell_NotifyIconGetRect`), which comes as two messages.
const ICON_RECT_DATA: usize = 3;

/// Which half of an icon's rectangle a question asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectPart {
    /// Where the rectangle starts: left and top.
    Position,
    /// How big it is: width and height.
    Size,
}

/// One of the two questions `Shell_NotifyIconGetRect` asks the taskbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectQuery {
    pub part: RectPart,
    pub hwnd: isize,
    pub uid: u32,
}

/// Reads the question: the magic number, 1 (position) or 2 (size), then a
/// `NOTIFYICONIDENTIFIER` (its size, 32 on the wire, a pad, the program's
/// window, the icon's id). `None` for anything else.
pub fn parse_rect_query(bytes: &[u8]) -> Option<RectQuery> {
    if u32_at(bytes, 0)? != TRAY_MAGIC {
        return None;
    }
    let part = match u32_at(bytes, 4)? {
        1 => RectPart::Position,
        2 => RectPart::Size,
        _ => return None,
    };
    Some(RectQuery {
        part,
        hwnd: widen(u32_at(bytes, 16)?),
        uid: u32_at(bytes, 20)?,
    })
}

/// Two 16-bit numbers as one answer: the first in the low word, the second in
/// the high word (`MAKELONG`), which is how the taskbar answers a question
/// about an icon's rectangle.
pub fn pack_pair(low: i32, high: i32) -> isize {
    (((high as u32 & 0xFFFF) << 16) | (low as u32 & 0xFFFF)) as i32 as isize
}

/// Side of the rectangle the host gives an icon, in pixels: the size of a
/// button of the tray panel.
const ANCHOR_SIDE: i32 = 32;

/// How long a click keeps the answer about where the icon is.
const ANCHOR_LIFETIME: std::time::Duration = std::time::Duration::from_secs(5);

/// Where the last click was, for answering "where is this icon?" while its
/// program opens a menu: Explorer, with its taskbar hidden, answers with the
/// top right corner of the screen, so the menu opened there.
static ANCHOR: Mutex<Option<(i32, i32, std::time::Instant)>> = Mutex::new(None);

fn remember_click(x: i32, y: i32) {
    *ANCHOR.lock().unwrap_or_else(PoisonError::into_inner) =
        Some((x, y, std::time::Instant::now()));
}

/// The rectangle to give an icon: `ANCHOR_SIDE` square around the click, for
/// `ANCHOR_LIFETIME` after it; `None` otherwise (Explorer's answer stands).
fn anchored_rect(
    anchor: Option<(i32, i32, std::time::Instant)>,
    now: std::time::Instant,
) -> Option<(i32, i32, i32, i32)> {
    let (x, y, at) = anchor?;
    (now.saturating_duration_since(at) <= ANCHOR_LIFETIME).then_some((
        x - ANCHOR_SIDE / 2,
        y - ANCHOR_SIDE / 2,
        ANCHOR_SIDE,
        ANCHOR_SIDE,
    ))
}

/// The answer to a question about an icon's rectangle, when the host knows
/// the icon and a click is fresh; `None` leaves it to Explorer.
fn answer_rect(query: RectQuery) -> Option<isize> {
    let known = host()
        .icons
        .iter()
        .any(|icon| icon.key == key_of(query.hwnd, query.uid));
    if !known {
        if TRACE.load(Ordering::Relaxed) {
            let keys: Vec<String> = host().icons.iter().map(|icon| icon.key.clone()).collect();
            log::warn!("tray trace: rect query {query:?}: icon unknown among {keys:?}");
        }
        return None;
    }
    let anchor = *ANCHOR.lock().unwrap_or_else(PoisonError::into_inner);
    let rect = anchored_rect(anchor, std::time::Instant::now());
    if TRACE.load(Ordering::Relaxed) {
        log::warn!("tray trace: rect query {query:?}: icon known, rect {rect:?}");
    }
    let (x, y, width, height) = rect?;
    Some(match query.part {
        RectPart::Position => pack_pair(x, y),
        RectPart::Size => pack_pair(width, height),
    })
}

/// Whether the host answers `TRUE` to an icon message it took: it is the
/// notification area now, so a program is told its icon was added, changed
/// or taken away. Explorer's own answer, which the program would otherwise
/// get, is "no" while its taskbar is hidden, and a program that is told its
/// icon was not added (FortiClient) takes it down. A delete of an icon the
/// host never had is the one thing it cannot say yes to.
fn accepted(action: Action, existed: bool) -> bool {
    match action {
        Action::Add | Action::Modify | Action::SetVersion | Action::SetFocus => true,
        Action::Delete => existed,
    }
}

/// The answer to a question about an icon's rectangle, if that is what the
/// message is and the host can answer it.
fn answer_to_rect_query(lparam: LPARAM) -> Option<isize> {
    let data = unsafe { (lparam.0 as *const COPYDATASTRUCT).as_ref() }?;
    if data.dwData != ICON_RECT_DATA || data.lpData.is_null() {
        return None;
    }
    let length = (data.cbData as usize).min(64);
    let bytes = unsafe { std::slice::from_raw_parts(data.lpData as *const u8, length) };
    answer_rect(parse_rect_query(bytes)?)
}

/// What goes by in a `WM_COPYDATA`: reads it if it is an icon message, and
/// says whether the host takes it (`accepted`); `None` when it is not one.
fn observe(lparam: LPARAM) -> Option<bool> {
    let data = lparam.0 as *const COPYDATASTRUCT;
    if data.is_null() {
        return None;
    }
    let data = unsafe { &*data };
    if data.dwData != NOTIFY_ICON_DATA || data.lpData.is_null() {
        return None;
    }
    // Memory of a message from another process: bounded by what it says it
    // holds, and cut to what a message can ever be.
    let length = (data.cbData as usize).min(4096);
    let bytes = unsafe { std::slice::from_raw_parts(data.lpData as *const u8, length) };
    let message = parse(bytes)?;
    if TRACE.load(Ordering::Relaxed) {
        // `warn`, so the app's log (which keeps warnings) has it; only with
        // `DOCKMAX_TRAY_TRACE` set.
        log::warn!(
            "tray trace: {:?} hwnd={} uid={} flags={:#x} cb={:#x} version={} bytes={} tip={:?}",
            message.action,
            message.hwnd,
            message.uid,
            message.flags,
            message.callback,
            message.version,
            bytes.len(),
            message.tip
        );
    }
    let png = (matches!(message.action, Action::Add | Action::Modify)
        && message.flags & NIF_ICON != 0)
        .then(|| icon_png(message.icon));
    let existed = host()
        .icons
        .iter()
        .any(|icon| icon.key == key_of(message.hwnd, message.uid));
    let changed = apply(&mut host().icons, &message, png);
    if changed {
        schedule_change();
    }
    Some(accepted(message.action, existed))
}

/// Tells the interface once for a burst of changes: many programs change
/// their icon in quick succession.
fn schedule_change() {
    let Some(hwnd) = window() else {
        return;
    };
    let mut host = host();
    if host.timer_pending {
        return;
    }
    host.timer_pending = true;
    unsafe { SetTimer(Some(hwnd), TIMER_ID, CHANGE_DELAY_MS, None) };
}

fn changed() {
    let callback = {
        let mut host = host();
        host.timer_pending = false;
        host.on_change.clone()
    };
    if let Some(callback) = callback {
        callback();
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COPYDATA => {
            if let Some(answer) = answer_to_rect_query(lparam) {
                return LRESULT(answer);
            }
            let taken = observe(lparam);
            // Success for what only the taskbar could answer, when it is not
            // there to answer.
            let answer = forward(msg, wparam, lparam, 1);
            // The icons are the host's: it answers for them.
            let answer = if taken == Some(true) {
                LRESULT(1)
            } else {
                answer
            };
            if TRACE.load(Ordering::Relaxed) {
                let data = unsafe { (lparam.0 as *const COPYDATASTRUCT).as_ref() };
                log::warn!(
                    "tray trace: forwarded dwData={:?} -> {}",
                    data.map(|data| data.dwData),
                    answer.0
                );
                // What is neither an icon nor an app bar message: its bytes.
                if let Some(data) = data
                    && !matches!(data.dwData, 0 | 1)
                    && !data.lpData.is_null()
                {
                    let length = (data.cbData as usize).min(64);
                    let bytes =
                        unsafe { std::slice::from_raw_parts(data.lpData as *const u8, length) };
                    let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                    log::warn!(
                        "tray trace: dwData={} cbData={} answer={:#x} bytes={}",
                        data.dwData,
                        data.cbData,
                        answer.0,
                        hex.join(" ")
                    );
                }
            }
            return answer;
        }
        WM_TIMER if wparam.0 == TIMER_ID => {
            let _ = unsafe { KillTimer(Some(hwnd), TIMER_ID) };
            changed();
            return LRESULT(0);
        }
        WM_TIMER if wparam.0 == REBROADCAST_ID => {
            let _ = unsafe { KillTimer(Some(hwnd), REBROADCAST_ID) };
            raise();
            broadcast_taskbar_created();
            let done = REBROADCASTS.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(pause) = rebroadcast_pause(done) {
                unsafe { SetTimer(Some(hwnd), REBROADCAST_ID, pause, None) };
            }
            return LRESULT(0);
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(hwnd) };
            return LRESULT(0);
        }
        WM_DESTROY => {
            WINDOW.store(0, Ordering::Release);
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Puts the host's window above every other topmost one, the taskbar's
/// included: programs find the first `Shell_TrayWnd` in the z-order, and the
/// taskbar's own window takes the top again at times (its position changes,
/// a monitor comes or goes), after which new icons would go to it and never
/// reach the dock. Cheap, and idempotent.
pub fn raise() {
    let Some(hwnd) = window() else {
        return;
    };
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    // Already topmost, `HWND_TOPMOST` alone changes nothing: down and up
    // again puts it at the top of the topmost windows.
    let _ = unsafe { SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags) };
    let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags) };
}

/// Starts the host: the window on a thread of its own. `on_change` is called
/// from the host's thread, once for each burst of changes. Does nothing when
/// it is running. Returns whether it is.
pub fn start(on_change: impl Fn() + Send + Sync + 'static) -> bool {
    if window().is_some() {
        return true;
    }
    {
        let mut host = host();
        host.icons.clear();
        host.on_change = Some(Arc::new(on_change));
    }
    let (ready, started) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("tray-host".into())
        .spawn(move || run(ready));
    if spawned.is_err() {
        return false;
    }
    let _ = started.recv();
    window().is_some()
}

/// Stops the host: the window goes, and programs find the taskbar's again.
pub fn stop() {
    if let Some(hwnd) = window() {
        let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
    host().icons.clear();
}

fn run(ready: mpsc::Sender<()>) {
    let class = w!("Shell_TrayWnd");
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }
        .map(|module| HINSTANCE(module.0))
        .unwrap_or_default();
    let class_data = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: instance,
        lpszClassName: class,
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    let created = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    };
    match created {
        Ok(hwnd) => {
            // Above the taskbar's window among the topmost ones: that is the
            // one `FindWindow` returns.
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
            WINDOW.store(hwnd.0 as isize, Ordering::Release);
        }
        Err(error) => log::warn!("tray: the window was not created: {error}"),
    }
    let _ = ready.send(());
    if window().is_none() {
        return;
    }
    // Programs that were running send their icons again, now and a few more
    // times.
    broadcast_taskbar_created();
    REBROADCASTS.store(0, Ordering::Relaxed);
    if let (Some(hwnd), Some(pause)) = (window(), rebroadcast_pause(0)) {
        unsafe { SetTimer(Some(hwnd), REBROADCAST_ID, pause, None) };
    }
    // The menus programs open from the tray panel (`popup_menu`): the events
    // come to this thread's message loop.
    let menu_hook = super::popup_menu::install_hook();
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    if let Some(hook) = menu_hook {
        super::popup_menu::remove_hook(hook);
    }
}

/// Which mouse button of an icon was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Button {
    Left,
    Right,
}

/// The messages Windows sends a program for a click on its icon, in order.
/// A program that asked for version 4 gets the position in `wparam`, the
/// message in the low word of `lparam` and the icon's id in the high word,
/// and the newer `NIN_SELECT` and `WM_CONTEXTMENU` besides.
fn click_messages(icon: &Icon, button: Button, cursor: (i32, i32)) -> Vec<(WPARAM, LPARAM)> {
    const NIN_SELECT: u32 = 0x400;
    const WM_CONTEXTMENU: u32 = 0x7B;
    let (down, up, extra) = match button {
        Button::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP, NIN_SELECT),
        Button::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP, WM_CONTEXTMENU),
    };
    if icon.version >= 4 {
        let at = ((cursor.0 as u32 & 0xFFFF) | ((cursor.1 as u32 & 0xFFFF) << 16)) as usize;
        let pack = |message: u32| {
            (
                WPARAM(at),
                LPARAM(((icon.uid << 16) | (message & 0xFFFF)) as isize),
            )
        };
        vec![pack(down), pack(up), pack(extra)]
    } else {
        let pack = |message: u32| (WPARAM(icon.uid as usize), LPARAM(message as isize));
        vec![pack(down), pack(up)]
    }
}

/// Clicks the icon `key` the way the taskbar would: sends its program the
/// messages it asked for, and lets it take the foreground (for its menu).
pub fn click(key: &str, button: Button) -> Result<(), String> {
    let icon = icons()
        .into_iter()
        .find(|icon| icon.key == key)
        .ok_or("no such icon")?;
    if icon.callback == 0 {
        return Err("the icon takes no clicks".into());
    }
    let target = HWND(icon.hwnd as *mut _);
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(target, Some(&mut process)) };
    let _ = unsafe { AllowSetForegroundWindow(process) };
    let mut cursor = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor) };
    remember_click(cursor.x, cursor.y);
    for (wparam, lparam) in click_messages(&icon, button, (cursor.x, cursor.y)) {
        unsafe { PostMessageW(Some(target), icon.callback, wparam, lparam) }
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Sends the icon `key` just the events listed (`WM_*` or `NIN_*` numbers),
/// in the format its version asks for. For the probe, to find out which
/// messages a program answers.
#[allow(dead_code, reason = "only the probe example uses it")]
pub fn send_events(key: &str, events: &[u32]) -> Result<(), String> {
    let icon = icons()
        .into_iter()
        .find(|icon| icon.key == key)
        .ok_or("no such icon")?;
    let target = HWND(icon.hwnd as *mut _);
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(target, Some(&mut process)) };
    let _ = unsafe { AllowSetForegroundWindow(process) };
    let mut cursor = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor) };
    remember_click(cursor.x, cursor.y);
    let at = ((cursor.x as u32 & 0xFFFF) | ((cursor.y as u32 & 0xFFFF) << 16)) as usize;
    for &event in events {
        let (wparam, lparam) = if icon.version >= 4 {
            (
                WPARAM(at),
                LPARAM(((icon.uid << 16) | (event & 0xFFFF)) as isize),
            )
        } else {
            (WPARAM(icon.uid as usize), LPARAM(event as isize))
        };
        unsafe { PostMessageW(Some(target), icon.callback, wparam, lparam) }
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// The image of an icon as a PNG `data:` URL, or `None` when it can't be
/// read (a handle that is not an icon, a black-and-white one).
fn icon_png(handle: isize) -> Option<String> {
    if handle == 0 {
        return None;
    }
    // Icon handles are shared by the whole session: copied, the image is
    // ours to read and to free.
    let copy = unsafe { CopyIcon(HICON(handle as *mut _)) }.ok()?;
    let rgba = icon_rgba(copy);
    let _ = unsafe { DestroyIcon(copy) };
    let (width, height, pixels) = rgba?;
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&pixels).ok()?;
    }
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

/// The pixels of an icon, as straight RGBA, with its size.
fn icon_rgba(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut info) }.ok()?;
    let read = (|| {
        if info.hbmColor.is_invalid() {
            // Black and white: the mask holds both halves.
            return None;
        }
        let mut bitmap = BITMAP::default();
        let got = unsafe {
            GetObjectW(
                HGDIOBJ(info.hbmColor.0),
                size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut _ as *mut _),
            )
        };
        if got == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        if bitmap.bmWidth > MAX_ICON_SIDE || bitmap.bmHeight > MAX_ICON_SIDE {
            return None;
        }
        let (width, height) = (bitmap.bmWidth as u32, bitmap.bmHeight as u32);
        let color = dib(info.hbmColor, width, height)?;
        let mask = dib(info.hbmMask, width, height)?;
        Some((width, height, to_rgba(&color, &mask)))
    })();
    unsafe {
        let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
        let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
    }
    read
}

/// A bitmap's pixels as top-down 32-bit BGRA.
fn dib(
    bitmap: ::windows::Win32::Graphics::Gdi::HBITMAP,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    if bitmap.is_invalid() {
        return None;
    }
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let screen = unsafe { GetDC(None) };
    let lines = unsafe {
        GetDIBits(
            screen,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    unsafe { ReleaseDC(None, screen) };
    (lines != 0).then_some(pixels)
}

/// BGRA color and mask to straight RGBA. An icon whose color bitmap has no
/// alpha at all takes its transparency from the mask (black is opaque).
fn to_rgba(color: &[u8], mask: &[u8]) -> Vec<u8> {
    let color = color.as_chunks::<4>().0;
    let mask = mask.as_chunks::<4>().0;
    let has_alpha = color.iter().any(|pixel| pixel[3] != 0);
    color
        .iter()
        .zip(mask)
        .flat_map(|(pixel, hole)| {
            let alpha = if has_alpha {
                pixel[3]
            } else if hole[0] == 0 {
                255
            } else {
                0
            };
            [pixel[2], pixel[1], pixel[0], alpha]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Shell_NotifyIcon` message as shell32 lays it out.
    #[allow(clippy::too_many_arguments)]
    fn data(
        action: u32,
        hwnd: u32,
        uid: u32,
        flags: u32,
        callback: u32,
        icon: u32,
        tip: &str,
        state: (u32, u32),
        version: u32,
    ) -> Vec<u8> {
        let mut bytes = vec![0u8; 8 + 960];
        let mut put =
            |at: usize, value: u32| bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        put(0, TRAY_MAGIC);
        put(4, action);
        put(8, 960);
        put(12, hwnd);
        put(16, uid);
        put(20, flags);
        put(24, callback);
        put(28, icon);
        put(8 + 280, state.0);
        put(8 + 284, state.1);
        put(8 + 800, version);
        for (index, unit) in tip.encode_utf16().enumerate() {
            let at = 8 + 24 + 2 * index;
            bytes[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        bytes
    }

    fn add(hwnd: u32, uid: u32, tip: &str) -> Message {
        parse(&data(
            0,
            hwnd,
            uid,
            NIF_MESSAGE | NIF_TIP,
            0x401,
            0,
            tip,
            (0, 0),
            0,
        ))
        .unwrap()
    }

    #[test]
    fn reads_an_icon_message() {
        let message = parse(&data(
            0,
            0x1234,
            7,
            NIF_MESSAGE | NIF_ICON | NIF_TIP,
            0x8001,
            0x55,
            "Docker Desktop",
            (0, 0),
            0,
        ))
        .unwrap();

        assert_eq!(message.action, Action::Add);
        assert_eq!(message.hwnd, 0x1234);
        assert_eq!(message.uid, 7);
        assert_eq!(message.callback, 0x8001);
        assert_eq!(message.icon, 0x55);
        assert_eq!(message.tip, "Docker Desktop");
    }

    #[test]
    fn widens_a_handle_the_way_windows_does() {
        // A handle with the high bit set is a negative 32-bit number.
        let message = parse(&data(1, 0xFFFF_FF00, 1, 0, 0, 0xFFFF_FF01, "", (0, 0), 0)).unwrap();

        assert_eq!(message.hwnd, -256);
        assert_eq!(message.icon, -255);
    }

    #[test]
    fn reads_accented_and_long_tips_to_the_terminator() {
        let message = add(1, 1, "Segurança do Windows");
        assert_eq!(message.tip, "Segurança do Windows");

        let long = "x".repeat(127);
        assert_eq!(add(1, 1, &long).tip, long);
    }

    #[test]
    fn refuses_what_is_not_an_icon_message() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0; 7]), None);
        // No magic number.
        let mut bytes = data(0, 1, 1, 0, 0, 0, "", (0, 0), 0);
        bytes[0] ^= 0xFF;
        assert_eq!(parse(&bytes), None);
        // An action that does not exist.
        assert_eq!(parse(&data(9, 1, 1, 0, 0, 0, "", (0, 0), 0)), None);
    }

    #[test]
    fn a_message_cut_short_never_panics() {
        let full = data(0, 1, 1, NIF_TIP, 0, 0, "tip", (0, 0), 0);
        for length in 0..full.len() {
            // Whatever the cut, it is read or refused, never out of bounds.
            let _ = parse(&full[..length]);
        }
        // Cut inside the header: refused.
        assert_eq!(parse(&full[..20]), None);
    }

    #[test]
    fn garbage_never_panics() {
        for seed in 0u32..200 {
            let bytes: Vec<u8> = (0..1000u32)
                .map(|i| (i.wrapping_mul(seed).wrapping_add(seed >> 3) >> 2) as u8)
                .collect();
            let _ = parse(&bytes);
        }
    }

    #[test]
    fn adding_an_icon_lists_it_once() {
        let mut icons = Vec::new();

        assert!(apply(&mut icons, &add(1, 1, "A"), None));
        assert!(!apply(&mut icons, &add(1, 1, "A"), None));

        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].tip, "A");
        assert_eq!(icons[0].callback, 0x401);
        assert_eq!(icons[0].key, "1:1");
    }

    #[test]
    fn two_icons_of_one_program_are_two_icons() {
        let mut icons = Vec::new();

        apply(&mut icons, &add(1, 1, "A"), None);
        apply(&mut icons, &add(1, 2, "B"), None);
        apply(&mut icons, &add(2, 1, "C"), None);

        assert_eq!(icons.len(), 3);
    }

    #[test]
    fn modifying_changes_only_what_the_flags_say() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 1, "A"), None);
        let modify = parse(&data(1, 1, 1, NIF_TIP, 0, 0, "B", (0, 0), 0)).unwrap();

        assert!(apply(&mut icons, &modify, None));

        assert_eq!(icons[0].tip, "B");
        // The callback was not in the flags: it stays.
        assert_eq!(icons[0].callback, 0x401);
    }

    #[test]
    fn modifying_an_unknown_icon_adds_it() {
        let mut icons = Vec::new();
        let modify = parse(&data(1, 5, 5, NIF_TIP, 0, 0, "late", (0, 0), 0)).unwrap();

        assert!(apply(&mut icons, &modify, None));

        assert_eq!(icons[0].tip, "late");
    }

    #[test]
    fn deleting_removes_the_icon_and_only_that_one() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 1, "A"), None);
        apply(&mut icons, &add(1, 2, "B"), None);
        let delete = parse(&data(2, 1, 1, 0, 0, 0, "", (0, 0), 0)).unwrap();

        assert!(apply(&mut icons, &delete, None));
        assert!(!apply(&mut icons, &delete, None));

        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].tip, "B");
    }

    #[test]
    fn the_hidden_state_follows_the_mask() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 1, "A"), None);
        let hide = parse(&data(
            1,
            1,
            1,
            NIF_STATE,
            0,
            0,
            "",
            (NIS_HIDDEN, NIS_HIDDEN),
            0,
        ))
        .unwrap();
        apply(&mut icons, &hide, None);
        assert!(icons[0].hidden);

        // A state without the bit in the mask says nothing about it.
        let quiet = parse(&data(1, 1, 1, NIF_STATE, 0, 0, "", (0, 0), 0)).unwrap();
        apply(&mut icons, &quiet, None);
        assert!(icons[0].hidden);

        let show = parse(&data(1, 1, 1, NIF_STATE, 0, 0, "", (0, NIS_HIDDEN), 0)).unwrap();
        apply(&mut icons, &show, None);
        assert!(!icons[0].hidden);
    }

    #[test]
    fn a_version_in_the_struct_of_an_add_is_not_the_icons_version() {
        // FortiClient writes 4 in the struct of every message and never sends
        // `NIM_SETVERSION`: Windows treats such an icon as the old kind, and
        // sending it clicks in the new format makes it ignore them.
        let mut icons = Vec::new();
        let add = parse(&data(
            0,
            1,
            129,
            NIF_MESSAGE | NIF_TIP,
            0x40a,
            0,
            "Forti",
            (0, 0),
            4,
        ))
        .unwrap();
        apply(&mut icons, &add, None);
        let modify = parse(&data(1, 1, 129, NIF_TIP, 0, 0, "Forti 2", (0, 0), 4)).unwrap();
        apply(&mut icons, &modify, None);

        assert_eq!(icons[0].version, 0);
    }

    #[test]
    fn only_the_set_version_message_sets_the_version() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 2, "V4"), None);
        let set = parse(&data(4, 1, 2, 0, 0, 0, "", (0, 0), 4)).unwrap();

        apply(&mut icons, &set, None);
        assert_eq!(icons[0].version, 4);

        // A later add or modify, whatever its struct holds, leaves it.
        let modify = parse(&data(1, 1, 2, NIF_TIP, 0, 0, "V4b", (0, 0), 0)).unwrap();
        apply(&mut icons, &modify, None);
        assert_eq!(icons[0].version, 4);
    }

    #[test]
    fn the_version_is_kept_and_changes_nothing_on_screen() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 1, "A"), None);
        let version = parse(&data(4, 1, 1, 0, 0, 0, "", (0, 0), 4)).unwrap();

        assert!(!apply(&mut icons, &version, None));

        assert_eq!(icons[0].version, 4);
    }

    #[test]
    fn a_new_image_replaces_the_old_one() {
        let mut icons = Vec::new();
        apply(&mut icons, &add(1, 1, "A"), Some(Some("data:one".into())));
        assert_eq!(icons[0].png.as_deref(), Some("data:one"));

        assert!(apply(
            &mut icons,
            &add(1, 1, "A"),
            Some(Some("data:two".into()))
        ));

        assert_eq!(icons[0].png.as_deref(), Some("data:two"));
    }

    fn icon(version: u32, uid: u32) -> Icon {
        Icon {
            key: "1:1".into(),
            hwnd: 1,
            uid,
            callback: 0x401,
            version,
            tip: String::new(),
            png: None,
            hidden: false,
        }
    }

    #[test]
    fn a_legacy_click_is_the_two_mouse_messages_with_the_id_in_wparam() {
        let messages = click_messages(&icon(0, 7), Button::Left, (10, 20));

        assert_eq!(
            messages,
            vec![
                (WPARAM(7), LPARAM(WM_LBUTTONDOWN as isize)),
                (WPARAM(7), LPARAM(WM_LBUTTONUP as isize)),
            ]
        );
    }

    #[test]
    fn a_version_4_click_packs_the_position_and_the_id_and_adds_the_new_message() {
        let messages = click_messages(&icon(4, 7), Button::Left, (10, 20));

        let at = 10 | (20 << 16);
        assert_eq!(messages.len(), 3);
        assert!(messages.iter().all(|(wparam, _)| wparam.0 == at));
        assert_eq!(messages[0].1.0 as u32, (7 << 16) | WM_LBUTTONDOWN);
        assert_eq!(messages[1].1.0 as u32, (7 << 16) | WM_LBUTTONUP);
        assert_eq!(messages[2].1.0 as u32, (7 << 16) | 0x400);
    }

    #[test]
    fn a_right_click_ends_in_the_context_menu_message_for_version_4() {
        let legacy = click_messages(&icon(0, 1), Button::Right, (0, 0));
        assert_eq!(legacy.len(), 2);
        assert_eq!(legacy[1].1.0 as u32, WM_RBUTTONUP);

        let modern = click_messages(&icon(4, 1), Button::Right, (0, 0));
        assert_eq!(modern.len(), 3);
        assert_eq!(modern[2].1.0 as u32 & 0xFFFF, 0x7B);
    }

    #[test]
    fn a_negative_cursor_position_packs_into_two_words() {
        // A monitor to the left of the primary one has negative x.
        let messages = click_messages(&icon(4, 1), Button::Left, (-100, 50));

        assert_eq!(messages[0].0.0, (0xFF9C) | (50 << 16));
    }

    #[test]
    fn an_icon_with_alpha_keeps_it_and_one_without_takes_the_mask() {
        // One pixel, BGRA.
        let with_alpha = to_rgba(&[10, 20, 30, 128], &[255, 255, 255, 0]);
        assert_eq!(with_alpha, [30, 20, 10, 128]);

        // No alpha in the color: a black mask pixel is opaque, a white one is not.
        let opaque = to_rgba(&[10, 20, 30, 0], &[0, 0, 0, 0]);
        assert_eq!(opaque, [30, 20, 10, 255]);
        let hole = to_rgba(&[10, 20, 30, 0], &[255, 255, 255, 0]);
        assert_eq!(hole, [30, 20, 10, 0]);
    }

    #[test]
    fn the_host_says_yes_to_what_it_took_and_no_to_a_delete_of_nothing() {
        for action in [
            Action::Add,
            Action::Modify,
            Action::SetVersion,
            Action::SetFocus,
        ] {
            assert!(accepted(action, false), "{action:?}");
            assert!(accepted(action, true), "{action:?}");
        }
        assert!(accepted(Action::Delete, true));
        assert!(!accepted(Action::Delete, false));
    }

    #[test]
    fn the_broadcast_is_repeated_a_few_times_with_growing_pauses_and_then_stops() {
        let pauses: Vec<u32> = (0..).map_while(rebroadcast_pause).collect();

        assert_eq!(pauses, [1500, 2500, 4000, 8000]);
        assert!(pauses.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(rebroadcast_pause(4), None);
        // Under half a minute in all: the host is not a loop.
        assert!(pauses.iter().sum::<u32>() < 30_000);
    }

    /// The question a program asked while it was placing a menu, as the host
    /// received it from Windows (captured on this machine).
    const QUERY_POSITION: [u8; 40] = [
        0x23, 0x34, 0x75, 0x34, 0x01, 0, 0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 0x76, 0x03, 0x40, 0,
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn reads_the_question_about_where_an_icon_is() {
        let position = parse_rect_query(&QUERY_POSITION).unwrap();
        assert_eq!(position.part, RectPart::Position);
        assert_eq!(position.hwnd, 0x40_0376);
        assert_eq!(position.uid, 1);

        let mut size = QUERY_POSITION;
        size[4] = 2;
        assert_eq!(parse_rect_query(&size).unwrap().part, RectPart::Size);
    }

    #[test]
    fn refuses_a_question_that_is_not_one() {
        assert_eq!(parse_rect_query(&[]), None);
        assert_eq!(parse_rect_query(&QUERY_POSITION[..20]), None);
        let mut bytes = QUERY_POSITION;
        bytes[4] = 3;
        assert_eq!(parse_rect_query(&bytes), None);
        bytes[4] = 1;
        bytes[0] = 0;
        assert_eq!(parse_rect_query(&bytes), None);
        // Cut anywhere, it is read or refused, never out of bounds.
        for length in 0..QUERY_POSITION.len() {
            let _ = parse_rect_query(&QUERY_POSITION[..length]);
        }
    }

    #[test]
    fn packs_two_numbers_the_way_the_taskbar_answers() {
        // What Explorer answered for an icon at 2314, 0 of 32 by 48.
        assert_eq!(pack_pair(2314, 0), 0x090a);
        assert_eq!(pack_pair(32, 48), 0x0030_0020);
        // A monitor to the left of the primary one has negative x: 16 bits.
        assert_eq!(pack_pair(-100, 50) & 0xFFFF, 0xFF9C);
    }

    #[test]
    fn the_rectangle_is_around_a_fresh_click_and_gone_after_it() {
        let now = std::time::Instant::now();

        assert_eq!(anchored_rect(None, now), None);
        assert_eq!(
            anchored_rect(Some((100, 200, now)), now),
            Some((84, 184, 32, 32))
        );
        let late = now + ANCHOR_LIFETIME + std::time::Duration::from_millis(1);
        assert_eq!(anchored_rect(Some((100, 200, now)), late), None);
        // A clock that went back is not a stale click.
        assert!(anchored_rect(Some((1, 1, late)), now).is_some());
    }

    #[test]
    fn a_handle_that_is_not_an_icon_has_no_image() {
        assert_eq!(icon_png(0), None);
        assert_eq!(icon_png(0x7FFF_FFF0), None);
    }

    /// Creates a real `Shell_TrayWnd` and broadcasts `TaskbarCreated` to
    /// every window of the session: run by hand (`--ignored`), not on every
    /// `cargo test`.
    #[test]
    #[ignore = "broadcasts TaskbarCreated to the whole session"]
    fn the_host_starts_forwards_and_stops() {
        // Real windows: the proxy is created and found like the taskbar's.
        assert!(start(|| {}));
        let ours = window().unwrap();
        let mut name = [0u16; 32];
        let length = unsafe { GetClassNameW(ours, &mut name) };
        assert_eq!(
            String::from_utf16_lossy(&name[..length as usize]),
            TRAY_CLASS
        );

        // The taskbar's window is still the one that is not ours.
        if let Some(real) = real_tray() {
            assert_ne!(real, ours);
        }
        assert!(start(|| {}), "a second start does nothing");

        stop();
        for _ in 0..50 {
            if window().is_none() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(window().is_none());
    }
}
