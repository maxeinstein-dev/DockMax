//! The launcher (spec 0012): a search box opened by Alt+Space from
//! anywhere, or by "Buscar apps…" in the dock's and the tray's menus, that
//! finds the programs of the Apps folder and opens them (L1); and, mixed in
//! the same list (L2), the result of a sum, a web search by keyword and the
//! settings pages of Windows and of the DockMax. Each result does one action
//! of a closed list (`search::Action`), checked before it runs.
//!
//! A native window, not WebView2 (author's decision, 2026-10-26): it opens in
//! a few milliseconds and costs a few megabytes only while open. The frame is
//! a plain pop-up with Windows' rounded corners, holding two children: a
//! Windows text field (`EDIT`, which brings the IME, pasting, undo and its
//! own accessibility) and the list of results, drawn with Direct2D, with a
//! UI Automation provider of its own (`access`).
//!
//! Everything lives on the main thread, like the dock: other threads reach
//! the launcher through `post`. The windows are made once at startup (the
//! hotkey belongs to the frame); what draws the list, and the icons, are made
//! when it opens and dropped 45 s after it closes.

mod access;
pub mod icons;
mod layout;
mod paint;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Mutex, PoisonError};
#[cfg(feature = "e2e")]
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap;
use windows::Win32::Graphics::Dwm::{
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLOR_GRAYTEXT, COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint,
    FW_NORMAL, FillRect, GetSysColor, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect,
    MONITOR_DEFAULTTONEAREST, MonitorFromPoint, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SYS_COLOR_INDEX,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, SetFocus, UnregisterHotKey, VK_DOWN,
    VK_ESCAPE, VK_RETURN, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, ES_AUTOHSCROLL, GetCursorPos, GetWindowTextLengthW,
    GetWindowTextW, HMENU, HWND_TOPMOST, IDC_ARROW, IsWindowVisible, KillTimer, LoadCursorW,
    PostMessageW, RegisterClassW, SPI_GETHIGHCONTRAST, SW_HIDE, SWP_NOACTIVATE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW, SetForegroundWindow,
    SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, SystemParametersInfoW, WA_INACTIVE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_COMMAND, WM_CTLCOLOREDIT,
    WM_DPICHANGED, WM_ERASEBKGND, WM_GETOBJECT, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_PAINT, WM_SETFONT, WM_SETTINGCHANGE, WM_SIZE, WM_SYSCHAR, WM_SYSKEYDOWN, WM_TIMER,
    WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, w};

use self::layout::{BOX, Colors, ROW, Rgb};
use crate::domain::hotkey::{self, Hotkey};
use crate::domain::launcher::{self as search, Action, Candidate, MAX_RESULTS};
use crate::domain::launcher_calc;
use crate::domain::launcher_pages::{self, Target};
use crate::domain::launcher_web::{self, WebKeyword};
use crate::persistence::database::Storage;

/// The classes of the frame and of the list.
const FRAME_CLASS: PCWSTR = w!("DockMaxLauncher");
const LIST_CLASS: PCWSTR = w!("DockMaxLauncherList");

/// Runs the work queued by `post`.
const WM_RUN: u32 = WM_APP + 0x41;
/// `EN_CHANGE`: the text field's text changed.
const EN_CHANGE: u32 = 0x0300;
/// `EM_SETCUEBANNER`: the text the field shows while empty.
const EM_SETCUEBANNER: u32 = 0x1501;

/// The hotkey's id (one per window), and Alt+Space.
const HOTKEY: i32 = 1;

/// The text field's id among the frame's children.
const EDIT_ID: usize = 1;
/// The subclass of the text field.
const EDIT_SUBCLASS: usize = 1;

/// Drops what draws the list, 45 s after it closes (as the panel, spec
/// 0011 H2).
const RELEASE_TIMER: usize = 1;
const RELEASE_AFTER: u32 = 45_000;
/// Says how many results there are once the typing pauses.
const ANNOUNCE_TIMER: usize = 2;
const ANNOUNCE_AFTER: u32 = 300;
/// Takes the hotkey back if a capture never ended (Configurações closed in
/// the middle).
const CAPTURE_TIMER: usize = 3;
const CAPTURE_FOR: u32 = 60_000;

/// The search glyph of Windows' icon font.
const SEARCH_GLYPH: &str = "\u{E721}";
/// The glyphs of the results that are not programs: a settings page, the
/// calculator, the web.
const SETTINGS_GLYPH: char = '\u{E713}';
const CALCULATOR_GLYPH: char = '\u{E8EF}';
const WEB_GLYPH: char = '\u{E774}';

/// Event sent to the windows when the hotkey's state changes (Configurações
/// shows it).
pub const STATUS_EVENT: &str = "launcher-status";

/// The frame's window (0 before it is made).
static FRAME: AtomicIsize = AtomicIsize::new(0);
/// Whether another program holds the hotkey (the launcher then opens only
/// from the menus).
static HOTKEY_TAKEN: AtomicBool = AtomicBool::new(false);
/// Configurações is capturing a new hotkey: the current one is let go.
static CAPTURING: AtomicBool = AtomicBool::new(false);

static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

type Work = Box<dyn FnOnce(&mut Launcher) + Send>;

static QUEUE: Mutex<Vec<Work>> = Mutex::new(Vec::new());

thread_local! {
    static LAUNCHER: RefCell<Option<Launcher>> = const { RefCell::new(None) };
    /// What the text field is painted with, read while the launcher may be
    /// busy: the text's color, the background's, and its brush.
    static EDIT_LOOK: Cell<(COLORREF, COLORREF, isize)> = const { Cell::new((COLORREF(0), COLORREF(0), 0)) };
}

/// What a result is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    App,
    WindowsPage,
    DockMaxPage,
    Calculator,
    Web,
}

impl Kind {
    /// Opening it is kept in the use (the apps and the pages).
    fn remembered(self) -> bool {
        matches!(self, Kind::App | Kind::WindowsPage | Kind::DockMaxPage)
    }

    fn glyph(self) -> Option<char> {
        match self {
            Kind::App => None,
            Kind::WindowsPage | Kind::DockMaxPage => Some(SETTINGS_GLYPH),
            Kind::Calculator => Some(CALCULATOR_GLYPH),
            Kind::Web => Some(WEB_GLYPH),
        }
    }
}

/// One result.
#[derive(Debug, Clone)]
struct Found {
    /// Tells it from the others: the key its use is kept under (`app:<id>`,
    /// `setting:<id>`), or `calc` and `web`.
    key: String,
    kind: Kind,
    name: String,
    /// What it shows under its name; the kind's when `None`.
    subtitle: Option<String>,
    action: Action,
}

impl Found {
    /// The program's id, for a program (its icon).
    fn app_id(&self) -> Option<&str> {
        match &self.action {
            Action::OpenApp(id) if self.kind == Kind::App => Some(id),
            _ => None,
        }
    }

    fn subtitle(&self) -> &str {
        if let Some(subtitle) = &self.subtitle {
            return subtitle;
        }
        let text = &crate::i18n::messages().launcher;
        match self.kind {
            Kind::App => text.app,
            Kind::WindowsPage => text.windows_page,
            Kind::DockMaxPage => text.dockmax_page,
            Kind::Calculator => text.calculator,
            Kind::Web => text.web,
        }
    }

    fn page(page: &launcher_pages::Page) -> Found {
        let (kind, action) = match page.target {
            Target::Windows(uri) => (Kind::WindowsPage, Action::OpenUri(uri.to_owned())),
            Target::DockMax(section) => (Kind::DockMaxPage, Action::OpenSection(section)),
        };
        Found {
            key: launcher_pages::page_key(page.id),
            kind,
            name: page.name.to_owned(),
            subtitle: None,
            action,
        }
    }

    /// The search of `term` with `keyword`, if it makes a valid address.
    fn web(keyword: &WebKeyword, term: &str) -> Option<Found> {
        let url = launcher_web::url(&keyword.template, term)?;
        let text = &crate::i18n::messages().launcher;
        Some(Found {
            key: "web".to_owned(),
            kind: Kind::Web,
            name: text.search_for.replace("{term}", term),
            subtitle: Some(text.search_with.replace("{name}", &keyword.name)),
            action: Action::OpenUri(url),
        })
    }
}

/// The pages of the fixed list, as results and as the search sees them.
fn pages() -> (Vec<Found>, Vec<Candidate>) {
    launcher_pages::PAGES
        .iter()
        .map(|page| (Found::page(page), Candidate::page(page)))
        .unzip()
}

/// A program's icon: being read, or read (`None` when it has none).
enum Loaded {
    Loading,
    Ready(Option<ID2D1Bitmap>),
}

/// What the list shows besides the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Note {
    None,
    Reading,
    Hint,
    NothingFound,
}

pub struct Launcher {
    app: AppHandle,
    frame: HWND,
    edit: HWND,
    list: HWND,
    /// What the search looks in, and its view of it: the programs of the
    /// Apps folder as last read, then the settings pages.
    entries: Vec<Found>,
    candidates: Vec<Candidate>,
    /// The Apps folder was read at least once.
    apps_read: bool,
    reading: bool,
    /// The web search's keywords and the decimal mark, read when the
    /// launcher opens.
    keywords: Vec<WebKeyword>,
    decimal: char,
    /// Each key's weight now, read when the launcher opens.
    usage: HashMap<String, f64>,
    query: String,
    results: Vec<Found>,
    /// The results are the most used (nothing typed).
    recent: bool,
    note: Note,
    selected: usize,
    /// The result being opened, and the one that could not be.
    launching: Option<String>,
    failed: Option<String>,
    painter: Option<paint::Painter>,
    icons: HashMap<(Option<String>, String), Loaded>,
    /// Reads the programs' icons from Windows in a helper process, while
    /// the launcher is open.
    icon_reader: Option<icons::Reader>,
    font: Option<HFONT>,
    glyph_font: Option<HFONT>,
    brush: Option<HBRUSH>,
    colors: Colors,
    scale: f64,
    /// The pointer's last place over the list: a row is selected by the
    /// mouse only when it moves, not when the list opens under it.
    pointer: Option<(i32, i32)>,
}

/// Queues `work` for the launcher, from any thread, and wakes it.
pub fn post(work: impl FnOnce(&mut Launcher) + Send + 'static) {
    QUEUE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(Box::new(work));
    let hwnd = FRAME.load(Ordering::SeqCst);
    if hwnd != 0 {
        let _ = unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_RUN, WPARAM(0), LPARAM(0)) };
    }
}

/// Runs `change` on the launcher; nothing while it is busy (a message sent
/// while it works, such as the focus moving as it shows itself).
fn with(change: impl FnOnce(&mut Launcher)) {
    LAUNCHER.with(|slot| {
        if let Ok(mut launcher) = slot.try_borrow_mut()
            && let Some(launcher) = launcher.as_mut()
        {
            change(launcher);
        }
    });
}

fn run_queue() {
    loop {
        let work: Vec<Work> =
            std::mem::take(&mut *QUEUE.lock().unwrap_or_else(PoisonError::into_inner));
        if work.is_empty() {
            return;
        }
        for item in work {
            with(item);
        }
    }
}

/// Makes the launcher's windows, hidden, and takes Alt+Space when the
/// setting is on. On the main thread, once.
pub fn start(app: &AppHandle) -> windows::core::Result<()> {
    let _ = APP.set(app.clone());
    let instance = unsafe { GetModuleHandleW(None) }?;
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default();
    for (class, procedure) in [
        (FRAME_CLASS, frame_procedure as WindowProcedure),
        (LIST_CLASS, list_procedure as WindowProcedure),
    ] {
        let class_data = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: class,
            hCursor: cursor,
            ..Default::default()
        };
        unsafe { RegisterClassW(&class_data) };
    }
    let title = HSTRING::from(crate::i18n::messages().launcher.title);
    let frame = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            FRAME_CLASS,
            &title,
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }?;
    let edit = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            0,
            0,
            1,
            1,
            Some(frame),
            Some(HMENU(EDIT_ID as _)),
            Some(instance.into()),
            None,
        )
    }?;
    let list = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            LIST_CLASS,
            w!(""),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            1,
            1,
            Some(frame),
            None,
            Some(instance.into()),
            None,
        )
    }?;
    unsafe {
        let _ = SetWindowSubclass(edit, Some(edit_procedure), EDIT_SUBCLASS, 0);
        let cue = HSTRING::from(crate::i18n::messages().launcher.placeholder);
        SendMessageW(
            edit,
            EM_SETCUEBANNER,
            Some(WPARAM(1)),
            Some(LPARAM(cue.as_ptr() as isize)),
        );
        let corners = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            frame,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corners).cast(),
            size_of_val(&corners) as u32,
        );
    }
    FRAME.store(frame.0 as isize, Ordering::SeqCst);
    let (entries, candidates) = pages();
    let launcher = Launcher {
        app: app.clone(),
        frame,
        edit,
        list,
        entries,
        candidates,
        apps_read: false,
        reading: false,
        keywords: Vec::new(),
        decimal: ',',
        usage: HashMap::new(),
        query: String::new(),
        results: Vec::new(),
        recent: false,
        note: Note::Hint,
        selected: 0,
        launching: None,
        failed: None,
        painter: None,
        icons: HashMap::new(),
        icon_reader: None,
        font: None,
        glyph_font: None,
        brush: None,
        colors: layout::colors(false),
        scale: 1.0,
        pointer: None,
    };
    LAUNCHER.with(|slot| *slot.borrow_mut() = Some(launcher));
    register(crate::commands::settings::current(app).launcher);
    Ok(())
}

type WindowProcedure = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// Lets go of the launcher and what draws it, on the main thread, as the app
/// ends: for the same reason as the docks (`native_dock::shutdown`).
pub fn shutdown() {
    let launcher = LAUNCHER.with(|slot| {
        slot.try_borrow_mut()
            .ok()
            .and_then(|mut launcher| launcher.take())
    });
    drop(launcher);
}

/// Takes the hotkey of the settings (`on`) or lets it go, from any thread:
/// the hotkey belongs to the launcher's window, on the main thread.
pub fn set_enabled(on: bool) {
    post(move |_| register(on));
}

/// The hotkey of the settings; Alt+Space when it is not one the launcher
/// can take (a file edited by hand).
fn configured() -> Hotkey {
    APP.get()
        .and_then(|app| {
            hotkey::check(&crate::commands::settings::current(app).launcher_hotkey).ok()
        })
        .or_else(|| Hotkey::parse(hotkey::DEFAULT))
        .expect("the default hotkey is valid")
}

/// Asks Windows for `hotkey` on the launcher's window.
fn take(frame: HWND, hotkey: Hotkey) -> windows::core::Result<()> {
    unsafe {
        RegisterHotKey(
            Some(frame),
            HOTKEY,
            HOT_KEY_MODIFIERS(hotkey.modifiers()) | MOD_NOREPEAT,
            hotkey.key.vk(),
        )
    }
}

/// Takes the hotkey of the settings (`on`) or lets it go; nothing while
/// Configurações captures a new one. When another program holds it, the
/// launcher opens only from the menus, and Configurações says so.
fn register(on: bool) {
    let frame = FRAME.load(Ordering::SeqCst);
    if frame == 0 {
        return;
    }
    let frame = HWND(frame as _);
    let _ = unsafe { UnregisterHotKey(Some(frame), HOTKEY) };
    let taken = if on && !CAPTURING.load(Ordering::SeqCst) {
        let hotkey = configured();
        match take(frame, hotkey) {
            Ok(()) => false,
            Err(error) => {
                log::warn!("launcher: {hotkey} is held by another program: {error}");
                true
            }
        }
    } else {
        false
    };
    set_taken(taken);
}

/// Configurações captures a new hotkey (`on`): the current one is let go,
/// or the combination would open the launcher. It comes back when the
/// capture ends, or on its own after `CAPTURE_FOR`, should Configurações
/// close in the middle.
pub fn capture(on: bool) {
    post(move |launcher| {
        CAPTURING.store(on, Ordering::SeqCst);
        unsafe {
            if on {
                SetTimer(Some(launcher.frame), CAPTURE_TIMER, CAPTURE_FOR, None);
            } else {
                let _ = KillTimer(Some(launcher.frame), CAPTURE_TIMER);
            }
        }
        register(launcher.enabled());
    });
}

/// Why a new hotkey could not be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryError {
    /// Another program holds it.
    Taken,
    /// The launcher did not answer.
    NotReady,
}

/// Tries `hotkey` for real before it is saved: on success it is held from
/// now on (the capture ends); when another program holds it, the one of the
/// settings is taken back. From any thread but the main one.
pub fn try_hotkey(hotkey: Hotkey) -> Result<(), TryError> {
    let (sent, received) = std::sync::mpsc::channel();
    post(move |launcher| {
        CAPTURING.store(false, Ordering::SeqCst);
        let _ = unsafe { KillTimer(Some(launcher.frame), CAPTURE_TIMER) };
        let _ = unsafe { UnregisterHotKey(Some(launcher.frame), HOTKEY) };
        let result = take(launcher.frame, hotkey).map_err(|error| {
            log::warn!("launcher: {hotkey} is held by another program: {error}");
            register(launcher.enabled());
            TryError::Taken
        });
        if result.is_ok() {
            set_taken(false);
        }
        let _ = sent.send(result);
    });
    received
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or(Err(TryError::NotReady))
}

/// Keeps whether another program holds the hotkey, and tells the windows
/// when it changes.
fn set_taken(taken: bool) {
    let before = HOTKEY_TAKEN.swap(taken, Ordering::SeqCst);
    if before != taken
        && let Some(app) = APP.get()
        && let Err(error) = app.emit(STATUS_EVENT, status())
    {
        log::warn!("launcher: the windows could not be told of the hotkey: {error}");
    }
}

/// What Configurações shows about the launcher.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Another program holds Alt+Space.
    pub hotkey_taken: bool,
}

pub fn status() -> Status {
    Status {
        hotkey_taken: HOTKEY_TAKEN.load(Ordering::SeqCst),
    }
}

/// Opens the launcher (the menus' "Buscar apps…"), or closes it when it is
/// open. From any thread.
pub fn toggle() {
    post(Launcher::toggle);
}

/// Whether the launcher is on screen.
pub fn is_open() -> bool {
    let frame = FRAME.load(Ordering::SeqCst);
    frame != 0 && unsafe { IsWindowVisible(HWND(frame as _)) }.as_bool()
}

/// Whether Windows' contrast mode is on.
fn high_contrast() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some(std::ptr::from_mut(&mut contrast).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_ok() && contrast.dwFlags.contains(HCF_HIGHCONTRASTON)
}

fn system_color(index: SYS_COLOR_INDEX) -> Rgb {
    let value = unsafe { GetSysColor(index) };
    (
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
    )
}

fn colorref(c: Rgb) -> COLORREF {
    COLORREF(u32::from(c.0) | (u32::from(c.1) << 8) | (u32::from(c.2) << 16))
}

/// How much bigger Windows' "Text size" makes text: 1 at 100 %.
fn text_scale() -> f64 {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let mut value: u32 = 100;
    let mut size = size_of::<u32>() as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Accessibility"),
            w!("TextScaleFactor"),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
    };
    if read.is_ok() {
        f64::from(value.clamp(100, 225)) / 100.0
    } else {
        1.0
    }
}

/// The decimal mark of the user's region (Windows' "Formato regional").
fn decimal_mark() -> char {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let mut value = [0u16; 8];
    let mut size = size_of_val(&value) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Control Panel\International"),
            w!("sDecimal"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    match String::from_utf16_lossy(&value).chars().next() {
        Some(mark @ (',' | '.')) if read.is_ok() => mark,
        _ => ',',
    }
}

/// Puts `text` in the clipboard, owned by `owner`.
fn copy_text(owner: HWND, text: &str) -> bool {
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    if unsafe { OpenClipboard(Some(owner)) }.is_err() {
        return false;
    }
    let copied = unsafe {
        EmptyClipboard().and_then(|()| {
            let memory = GlobalAlloc(GMEM_MOVEABLE, wide.len() * size_of::<u16>())?;
            let pointer = GlobalLock(memory).cast::<u16>();
            if pointer.is_null() {
                let _ = GlobalFree(Some(memory));
                return Err(windows::core::Error::from_thread());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), pointer, wide.len());
            let _ = GlobalUnlock(memory);
            // The clipboard owns the memory once it takes it.
            SetClipboardData(u32::from(CF_UNICODETEXT.0), Some(HANDLE(memory.0)))
                .inspect_err(|_| {
                    let _ = GlobalFree(Some(memory));
                })
                .map(|_| ())
        })
    };
    let _ = unsafe { CloseClipboard() };
    copied.is_ok()
}

fn delete_font(font: Option<HFONT>) {
    if let Some(font) = font {
        let _ = unsafe { DeleteObject(HGDIOBJ(font.0)) };
    }
}

fn make_font(size: f64, family: PCWSTR) -> Option<HFONT> {
    let font = unsafe {
        CreateFontW(
            -(size.round() as i32),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            family,
        )
    };
    (!font.is_invalid()).then_some(font)
}

impl Launcher {
    /// Whether the launcher's switch is on in the settings.
    fn enabled(&self) -> bool {
        crate::commands::settings::current(&self.app).launcher
    }

    fn toggle(&mut self) {
        if is_open() {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Opens on the monitor under the pointer, with the keyboard in the
    /// search box.
    fn show(&mut self) {
        let _ = unsafe { KillTimer(Some(self.frame), RELEASE_TIMER) };
        let mut at = POINT::default();
        let _ = unsafe { GetCursorPos(&mut at) };
        let handle = unsafe { MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST) };
        let monitor = super::monitors::all()
            .into_iter()
            .find(|monitor| monitor.handle == handle.0 as isize)
            .or_else(|| super::monitors::all().into_iter().next());
        let Some(monitor) = monitor else {
            return;
        };
        self.scale = monitor.scale * text_scale();
        self.restyle();
        self.usage = self.read_usage();
        self.keywords = crate::commands::settings::current(&self.app).launcher_keywords;
        self.decimal = decimal_mark();
        self.query.clear();
        self.failed = None;
        self.launching = None;
        self.pointer = None;
        unsafe {
            let _ = SetWindowTextW(self.edit, w!(""));
        }
        self.read_catalog();
        self.refresh();
        let height = BOX + self.list_height();
        let placed = layout::place(monitor.rect, self.scale, height);
        unsafe {
            let _ = SetWindowPos(
                self.frame,
                Some(HWND_TOPMOST),
                placed.x,
                placed.y,
                placed.width as i32,
                placed.height as i32,
                SWP_SHOWWINDOW,
            );
        }
        self.arrange();
        self.request_icons();
        self.publish();
        self.paint_soon();
        unsafe {
            // Alt+Space and the menus are the user's input to this process:
            // Windows lets it take the foreground.
            if !SetForegroundWindow(self.frame).as_bool() {
                log::warn!("launcher: Windows did not give it the keyboard");
            }
            let _ = SetFocus(Some(self.edit));
        }
        log::info!("launcher: open");
        self.announce_soon();
    }

    /// Closes, empty, and drops what draws it 45 s later.
    fn hide(&mut self) {
        if !is_open() {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.frame, SW_HIDE);
            let _ = SetWindowTextW(self.edit, w!(""));
            SetTimer(Some(self.frame), RELEASE_TIMER, RELEASE_AFTER, None);
            let _ = KillTimer(Some(self.frame), ANNOUNCE_TIMER);
        }
        self.query.clear();
        self.results.clear();
        access::publish(Vec::new());
        // The helper leaves, and the memory of reading the icons with it;
        // the icons it had not answered are asked again next time.
        self.icon_reader = None;
        self.icons
            .retain(|_, loaded| matches!(loaded, Loaded::Ready(_)));
        log::info!("launcher: closed");
    }

    /// Drops what draws the list and the icons it read.
    fn release(&mut self) {
        let _ = unsafe { KillTimer(Some(self.frame), RELEASE_TIMER) };
        if is_open() {
            return;
        }
        self.painter = None;
        self.icons.clear();
        log::info!("launcher: released");
    }

    /// The colors and fonts of the look Windows has now, at the scale of the
    /// monitor it opens on.
    fn restyle(&mut self) {
        let dark = super::native_dock::system_dark();
        self.colors = if high_contrast() {
            layout::contrast_colors(
                system_color(COLOR_WINDOW),
                system_color(COLOR_WINDOWTEXT),
                system_color(COLOR_GRAYTEXT),
                system_color(COLOR_HIGHLIGHT),
                system_color(COLOR_HIGHLIGHTTEXT),
            )
        } else {
            layout::colors(dark)
        };
        unsafe {
            let dark_frame = windows::core::BOOL::from(dark && !self.colors.contrast);
            let _ = DwmSetWindowAttribute(
                self.frame,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                std::ptr::from_ref(&dark_frame).cast(),
                size_of_val(&dark_frame) as u32,
            );
        }
        if let Some(brush) = self.brush.take() {
            let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
        }
        let brush = unsafe { CreateSolidBrush(colorref(self.colors.background)) };
        self.brush = Some(brush);
        EDIT_LOOK.with(|look| {
            look.set((
                colorref(self.colors.text),
                colorref(self.colors.background),
                brush.0 as isize,
            ));
        });
        delete_font(self.font.take());
        delete_font(self.glyph_font.take());
        self.font = make_font(layout::EDIT_FONT * self.scale, w!("Segoe UI Variable Text"));
        self.glyph_font = make_font(layout::GLYPH_FONT * self.scale, w!("Segoe Fluent Icons"));
        if let Some(font) = self.font {
            unsafe {
                SendMessageW(
                    self.edit,
                    WM_SETFONT,
                    Some(WPARAM(font.0 as usize)),
                    Some(LPARAM(1)),
                );
            }
        }
        if let Some(painter) = &mut self.painter {
            painter.set_scale(self.scale);
        }
    }

    /// Places the text field and the list in the frame.
    fn arrange(&mut self) {
        let s = self.scale;
        let px = |v: f64| (v * s).round() as i32;
        let width = layout::WIDTH;
        unsafe {
            let _ = SetWindowPos(
                self.edit,
                None,
                px(layout::EDIT_LEFT),
                px((BOX - layout::EDIT_HEIGHT) / 2.0),
                px(width - layout::EDIT_LEFT - layout::EDIT_RIGHT),
                px(layout::EDIT_HEIGHT),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = SetWindowPos(
                self.list,
                None,
                0,
                px(BOX),
                px(width),
                px(self.list_height()),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = InvalidateRect(Some(self.frame), None, true);
        }
        // The list's `WM_SIZE` came while the launcher was busy here.
        if let Some(painter) = &mut self.painter {
            painter.resize();
        }
    }

    fn list_height(&self) -> f64 {
        layout::list_height(self.results.len(), self.recent && !self.results.is_empty())
    }

    /// The use kept in the database, as it weighs now.
    fn read_usage(&self) -> HashMap<String, f64> {
        let Some(state) = self.app.try_state::<Mutex<Storage>>() else {
            return HashMap::new();
        };
        let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
        let now = crate::persistence::database::now_ms();
        match storage.connection() {
            Ok(conn) => crate::persistence::launcher_repo::usage(conn, now).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    /// Reads the Apps folder off the main thread: the first time, the list
    /// says "Lendo os apps…" until it comes; after that, the copy in memory
    /// answers at once and is replaced when the new read arrives.
    fn read_catalog(&mut self) {
        if self.reading {
            return;
        }
        self.reading = true;
        let spawned = std::thread::Builder::new()
            .name("launcher-catalog".into())
            .spawn(|| {
                let read = super::apps::catalog();
                post(move |launcher| launcher.catalog_read(read));
            });
        if let Err(error) = spawned {
            log::warn!("launcher: the apps could not be read: {error}");
            self.reading = false;
        }
    }

    fn catalog_read(&mut self, read: Result<Vec<super::apps::CatalogApp>, String>) {
        self.reading = false;
        let apps = match read {
            Ok(apps) => apps,
            Err(error) => {
                log::warn!("launcher: the apps could not be read: {error}");
                self.apps_read = true;
                self.refresh();
                self.after_change();
                return;
            }
        };
        let (pages, page_candidates) = pages();
        let mut candidates: Vec<Candidate> = apps
            .iter()
            .map(|app| Candidate::app(&app.id, &app.name))
            .collect();
        let mut entries: Vec<Found> = apps
            .into_iter()
            .map(|app| Found {
                key: search::app_key(&app.id),
                kind: Kind::App,
                name: app.name,
                subtitle: None,
                action: Action::OpenApp(app.id),
            })
            .collect();
        entries.extend(pages);
        candidates.extend(page_candidates);
        self.entries = entries;
        self.candidates = candidates;
        self.apps_read = true;
        if is_open() {
            let selected_key = self
                .results
                .get(self.selected)
                .map(|found| found.key.clone());
            self.refresh();
            if let Some(index) =
                selected_key.and_then(|key| self.results.iter().position(|found| found.key == key))
            {
                self.selected = index;
            }
            self.after_change();
        }
    }

    /// The results for what is typed now: the sum's result first; then a
    /// keyword's search alone, or the apps and the pages; and, when nothing
    /// matched, the search on the web with the first keyword.
    fn refresh(&mut self) {
        let query = self.query.trim().to_owned();
        let mut results = Vec::new();
        if query.is_empty() {
            results.extend(
                search::recents(&self.candidates, &self.usage, MAX_RESULTS)
                    .into_iter()
                    .map(|index| self.entries[index].clone()),
            );
        } else {
            if let Some(value) = launcher_calc::evaluate(&query) {
                let shown = launcher_calc::format(value, self.decimal);
                results.push(Found {
                    key: "calc".to_owned(),
                    kind: Kind::Calculator,
                    name: format!("= {shown}"),
                    subtitle: None,
                    action: Action::Copy(shown),
                });
            }
            if let Some((keyword, term)) = launcher_web::matching(&self.keywords, &query) {
                results.extend(Found::web(keyword, term));
            } else {
                let room = MAX_RESULTS - results.len();
                results.extend(
                    search::search(&self.candidates, &self.usage, &query, room)
                        .into_iter()
                        .map(|index| self.entries[index].clone()),
                );
                if results.is_empty()
                    && let Some(keyword) = self.keywords.first()
                {
                    results.extend(Found::web(keyword, &query));
                }
            }
        }
        self.recent = query.is_empty();
        self.results = results;
        self.note = match (self.results.is_empty(), query.is_empty(), self.apps_read) {
            (false, ..) => Note::None,
            (true, _, false) => Note::Reading,
            (true, true, true) => Note::Hint,
            (true, false, true) => Note::NothingFound,
        };
        self.selected = 0;
    }

    /// Puts `text` in the search box, as typing it would (UI Automation's
    /// `SetValue`, the e2e build).
    fn set_query(&mut self, text: &str) {
        let wide = HSTRING::from(text);
        unsafe {
            let _ = SetWindowTextW(self.edit, &wide);
        }
        // The field's change notice came while the launcher was busy here.
        self.text_changed();
    }

    /// The text changed: new results, the list resized and painted.
    fn text_changed(&mut self) {
        self.query = read_text(self.edit);
        self.failed = None;
        self.refresh();
        self.after_change();
        self.announce_soon();
    }

    /// The results changed: the window takes their height (its top stays),
    /// and the list is painted.
    fn after_change(&mut self) {
        if !is_open() {
            return;
        }
        let mut frame = RECT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.frame, &mut frame);
        }
        let height = ((BOX + self.list_height()) * self.scale).round() as i32;
        unsafe {
            let _ = SetWindowPos(
                self.frame,
                None,
                frame.left,
                frame.top,
                frame.right - frame.left,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        self.arrange();
        self.request_icons();
        self.publish();
        self.paint_soon();
    }

    fn paint_soon(&self) {
        let _ = unsafe { InvalidateRect(Some(self.list), None, false) };
    }

    /// Asks for the icons of the rows shown, off the main thread.
    fn request_icons(&mut self) {
        let pack = crate::commands::settings::current(&self.app).dock.icon_pack;
        if self.icons.len() > paint::MAX_ICONS {
            self.icons.clear();
        }
        let shown: Vec<(String, String)> = self
            .results
            .iter()
            .filter_map(|found| Some((found.app_id()?.to_owned(), found.name.clone())))
            .collect();
        for (id, name) in shown {
            let key = (pack.clone(), id.clone());
            if self.icons.contains_key(&key) {
                continue;
            }
            self.icons.insert(key.clone(), Loaded::Loading);
            let Some(pack) = pack.clone() else {
                self.ask_windows(id);
                continue;
            };
            // The icon pack's image is a file: read here, off the main
            // thread; Windows answers for a program the pack lacks.
            let app = self.app.clone();
            let names = crate::commands::icons::names_of(&id, &name);
            let spawned = std::thread::Builder::new()
                .name("launcher-icon".into())
                .spawn(move || {
                    let png = crate::commands::icons::pack_icon(&app, &pack, &names);
                    post(move |launcher| match png {
                        Some(png) => launcher.icon_read(&id, Some(png)),
                        None => launcher.ask_windows(id),
                    });
                });
            if spawned.is_err() {
                self.icons.remove(&key);
            }
        }
    }

    /// Asks Windows for the icon of the program `id`, through the helper.
    fn ask_windows(&mut self, id: String) {
        if self.icon_reader.is_none() {
            self.icon_reader = icons::Reader::start(|id, png| {
                post(move |launcher| launcher.icon_read(&id, png));
            });
        }
        let size = (layout::ICON * self.scale).round().clamp(16.0, 256.0) as u32;
        match &self.icon_reader {
            Some(reader) => reader.ask(id, size),
            None => self.icon_read(&id, None),
        }
    }

    /// The icon of the program `id` came (`None`: it has none).
    fn icon_read(&mut self, id: &str, png: Option<Vec<u8>>) {
        let bitmap = png.and_then(|png| self.painter().and_then(|painter| painter.bitmap(&png)));
        for ((_, key_id), loaded) in &mut self.icons {
            if key_id == id && matches!(loaded, Loaded::Loading) {
                *loaded = Loaded::Ready(bitmap.clone());
            }
        }
        self.paint_soon();
    }

    /// Tells assistive technology what the list shows now.
    fn publish(&self) {
        let mut origin = POINT::default();
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::ClientToScreen(self.list, &mut origin);
        }
        let s = self.scale;
        let top = layout::LIST_PAD + if self.recent { layout::TITLE } else { 0.0 };
        let entries = self
            .results
            .iter()
            .enumerate()
            .map(|(index, found)| access::Entry {
                key: found.key.clone(),
                name: found.name.clone(),
                kind: found.subtitle().to_owned(),
                rect: (
                    f64::from(origin.x) + layout::SIDE * s,
                    f64::from(origin.y) + (top + index as f64 * ROW) * s,
                    (layout::WIDTH - 2.0 * layout::SIDE) * s,
                    ROW * s,
                ),
                selected: index == self.selected,
            })
            .collect();
        access::publish(entries);
    }

    fn announce_soon(&self) {
        unsafe { SetTimer(Some(self.frame), ANNOUNCE_TIMER, ANNOUNCE_AFTER, None) };
    }

    /// Says how many results there are (or why there are none).
    fn announce(&self) {
        let _ = unsafe { KillTimer(Some(self.frame), ANNOUNCE_TIMER) };
        let text = crate::i18n::messages();
        let said = match self.note {
            Note::Reading => text.launcher.reading.to_owned(),
            Note::Hint => text.launcher.hint.to_owned(),
            Note::NothingFound => text.launcher.nothing_found.to_owned(),
            Note::None if self.results.len() == 1 => text.launcher.one_result.to_owned(),
            Note::None => text
                .launcher
                .results
                .replace("{n}", &self.results.len().to_string()),
        };
        access::announce(self.list.0 as isize, &said);
    }

    fn select(&mut self, index: usize) {
        if index >= self.results.len() || index == self.selected {
            return;
        }
        self.selected = index;
        self.publish();
        self.paint_soon();
        let found = &self.results[index];
        let said = crate::i18n::messages()
            .launcher
            .selected
            .replace("{name}", &found.name)
            .replace("{kind}", found.subtitle())
            .replace("{n}", &(index + 1).to_string())
            .replace("{count}", &self.results.len().to_string());
        access::selected(self.list.0 as isize, &found.key, &said);
    }

    /// Moves the selection by `step`, stopping at the ends.
    fn step(&mut self, step: isize) {
        if self.results.is_empty() {
            return;
        }
        let last = self.results.len() as isize - 1;
        let index = (self.selected as isize + step).clamp(0, last) as usize;
        self.select(index);
    }

    fn select_key(&mut self, key: &str) {
        if let Some(index) = self.results.iter().position(|found| found.key == key) {
            self.select(index);
        }
    }

    fn open_key(&mut self, key: &str) {
        if let Some(index) = self.results.iter().position(|found| found.key == key) {
            self.open(index);
        }
    }

    /// Does what the result `index` does, once its action is checked: the
    /// launcher closes once it is done, or stays saying it could not.
    fn open(&mut self, index: usize) {
        let Some(found) = self.results.get(index).cloned() else {
            return;
        };
        if self.launching.is_some() {
            return;
        }
        let key = found.key.clone();
        let remembered = found.kind.remembered();
        if !search::is_allowed(&found.action) {
            log::warn!("launcher: a result's action was refused");
            self.launched(&key, false, false);
            return;
        }
        match found.action {
            Action::OpenApp(id) => {
                self.open_off_thread(key, remembered, move || super::apps::launch(&id))
            }
            Action::OpenUri(uri) => {
                self.open_off_thread(key, remembered, move || super::apps::open_uri(&uri))
            }
            Action::OpenSection(section) => {
                let opened = super::settings_window::open_section(&self.app, section)
                    .inspect_err(|error| {
                        log::warn!("launcher: Configurações did not open: {error}")
                    })
                    .is_ok();
                self.launched(&key, remembered, opened);
            }
            Action::Copy(text) => {
                let copied = copy_text(self.frame, &text);
                if copied {
                    access::announce(
                        self.list.0 as isize,
                        crate::i18n::messages().launcher.copied,
                    );
                }
                self.launched(&key, false, copied);
            }
        }
    }

    /// Runs `open` on a thread of its own (Windows may take a while to
    /// start a program), then tells the launcher.
    fn open_off_thread(
        &mut self,
        key: String,
        remembered: bool,
        open: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) {
        self.launching = Some(key.clone());
        let thread_key = key.clone();
        let spawned = std::thread::Builder::new()
            .name("launcher-open".into())
            .spawn(move || {
                let opened = open()
                    .inspect_err(|error| log::warn!("launcher: it did not open: {error}"))
                    .is_ok();
                post(move |launcher| launcher.launched(&thread_key, remembered, opened));
            });
        if spawned.is_err() {
            self.launched(&key, remembered, false);
        }
    }

    fn launched(&mut self, key: &str, remembered: bool, opened: bool) {
        self.launching = None;
        if opened {
            if remembered {
                self.record(key);
            }
            self.hide();
        } else {
            log::warn!("launcher: a result could not be opened");
            self.failed = Some(key.to_owned());
            self.paint_soon();
            access::announce(
                self.list.0 as isize,
                crate::i18n::messages().launcher.failed,
            );
        }
    }

    /// One more opening of `key`, in the database.
    fn record(&self, key: &str) {
        let Some(state) = self.app.try_state::<Mutex<Storage>>() else {
            return;
        };
        let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
        let now = crate::persistence::database::now_ms();
        if let Ok(conn) = storage.connection()
            && crate::persistence::launcher_repo::record(conn, key, now).is_err()
        {
            log::warn!("launcher: the use could not be kept");
        }
    }

    /// Paints the frame: the plate and the search glyph (the children paint
    /// themselves).
    fn paint_frame(&self, dc: HDC, client: &RECT) {
        unsafe {
            if let Some(brush) = self.brush {
                FillRect(dc, client, brush);
            }
            if let Some(font) = self.glyph_font {
                let previous = SelectObject(dc, HGDIOBJ(font.0));
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, colorref(self.colors.dim));
                let s = self.scale;
                let mut area = RECT {
                    left: (16.0 * s).round() as i32,
                    top: 0,
                    right: (layout::EDIT_LEFT * s).round() as i32,
                    bottom: (BOX * s).round() as i32,
                };
                let mut glyph: Vec<u16> = SEARCH_GLYPH.encode_utf16().collect();
                DrawTextW(
                    dc,
                    &mut glyph,
                    &mut area,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
                SelectObject(dc, previous);
            }
        }
    }

    /// What draws the list, made the first time it is needed after a
    /// release.
    fn painter(&mut self) -> Option<&mut paint::Painter> {
        if self.painter.is_none() {
            match paint::Painter::new(self.list) {
                Ok(mut painter) => {
                    painter.set_scale(self.scale);
                    self.painter = Some(painter);
                }
                Err(error) => log::warn!("launcher: nothing to paint with: {error}"),
            }
        }
        self.painter.as_mut()
    }

    /// Paints the list.
    fn paint_list(&mut self) {
        if self.painter().is_none() {
            return;
        }
        let pack = crate::commands::settings::current(&self.app).dock.icon_pack;
        let text = &crate::i18n::messages().launcher;
        let message = match self.note {
            Note::None => None,
            Note::Reading => Some(text.reading),
            Note::Hint => Some(text.hint),
            Note::NothingFound => Some(text.nothing_found),
        };
        let rows: Vec<paint::Row> = self
            .results
            .iter()
            .enumerate()
            .map(|(index, found)| {
                let icon = found.app_id().and_then(|id| {
                    match self.icons.get(&(pack.clone(), id.to_owned())) {
                        Some(Loaded::Ready(Some(bitmap))) => Some(bitmap),
                        _ => None,
                    }
                });
                let subtitle = if self.failed.as_deref() == Some(found.key.as_str()) {
                    text.failed
                } else {
                    found.subtitle()
                };
                paint::Row {
                    name: &found.name,
                    subtitle,
                    icon,
                    glyph: found.kind.glyph(),
                    letter: found.name.chars().next().unwrap_or(' '),
                    selected: index == self.selected,
                    shortcut: layout::shortcut(index),
                }
            })
            .collect();
        let scene = paint::Scene {
            colors: self.colors,
            title: (self.recent && !self.results.is_empty()).then_some(text.recent),
            rows,
            message,
        };
        let Some(painter) = self.painter.as_mut() else {
            return;
        };
        painter.set_scale(self.scale);
        let kept = painter.paint(&scene, layout::WIDTH);
        if !kept {
            // Direct2D lost the device: the icons were made with it.
            self.icons.clear();
            self.request_icons();
            self.paint_soon();
        }
    }

    /// The pointer moved over the list: the row under it is selected.
    fn pointer_moved(&mut self, x: i32, y: i32) {
        if self.pointer == Some((x, y)) {
            return;
        }
        let first = self.pointer.is_none();
        self.pointer = Some((x, y));
        if first {
            // The list opened under the pointer: it has not moved yet.
            return;
        }
        let at = f64::from(y) / self.scale;
        if let Some(index) = layout::row_at(at, self.results.len(), self.recent) {
            self.select(index);
        }
    }

    fn clicked(&mut self, y: i32) {
        let at = f64::from(y) / self.scale;
        if let Some(index) = layout::row_at(at, self.results.len(), self.recent) {
            self.selected = index;
            self.open(index);
        }
    }
}

/// The text typed in `edit`.
fn read_text(edit: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(edit) }.max(0) as usize;
    let mut buffer = vec![0u16; length + 1];
    let read = unsafe { GetWindowTextW(edit, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..read])
}

fn point(lparam: LPARAM) -> (i32, i32) {
    let x = i32::from((lparam.0 & 0xFFFF) as u16 as i16);
    let y = i32::from(((lparam.0 >> 16) & 0xFFFF) as u16 as i16);
    (x, y)
}

extern "system" fn frame_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_RUN => run_queue(),
        WM_HOTKEY if wparam.0 as i32 == HOTKEY => with(Launcher::toggle),
        WM_ACTIVATE => {
            // Somewhere else took the keyboard: the launcher closes, as the
            // Ulauncher does.
            if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE {
                with(Launcher::hide);
            }
        }
        WM_COMMAND => {
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            if (wparam.0 & 0xFFFF) == EDIT_ID && code == EN_CHANGE {
                with(Launcher::text_changed);
            }
        }
        WM_CTLCOLOREDIT => {
            let (text, background, brush) = EDIT_LOOK.with(Cell::get);
            let dc = HDC(wparam.0 as _);
            unsafe {
                SetTextColor(dc, text);
                windows::Win32::Graphics::Gdi::SetBkColor(dc, background);
            }
            return LRESULT(brush);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut client = RECT::default();
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client);
            }
            with(|launcher| launcher.paint_frame(dc, &client));
            let _ = unsafe { EndPaint(hwnd, &paint) };
        }
        WM_TIMER => match wparam.0 {
            RELEASE_TIMER => with(Launcher::release),
            CAPTURE_TIMER => with(|launcher| {
                let _ = unsafe { KillTimer(Some(launcher.frame), CAPTURE_TIMER) };
                CAPTURING.store(false, Ordering::SeqCst);
                register(launcher.enabled());
            }),
            ANNOUNCE_TIMER => with(|launcher| launcher.announce()),
            _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        },
        // The launcher places itself on each opening, at its monitor's scale.
        WM_DPICHANGED => {}
        WM_SETTINGCHANGE => {
            with(|launcher| {
                if is_open() {
                    launcher.restyle();
                    launcher.arrange();
                    launcher.paint_soon();
                }
            });
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
    LRESULT(0)
}

extern "system" fn list_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            unsafe { BeginPaint(hwnd, &mut paint) };
            with(Launcher::paint_list);
            let _ = unsafe { EndPaint(hwnd, &paint) };
        }
        WM_SIZE => with(|launcher| {
            if let Some(painter) = &mut launcher.painter {
                painter.resize();
            }
        }),
        WM_MOUSEMOVE => {
            let (x, y) = point(lparam);
            with(|launcher| launcher.pointer_moved(x, y));
        }
        WM_LBUTTONUP => {
            let (_, y) = point(lparam);
            with(|launcher| launcher.clicked(y));
        }
        WM_GETOBJECT => match access::get_object(hwnd, wparam, lparam) {
            Some(result) => return result,
            None => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        },
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
    LRESULT(0)
}

/// The search box's own keys: the arrows move the selection, Enter opens,
/// Esc closes, Alt and a number opens that row. The rest is the text field's.
unsafe extern "system" fn edit_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    let key = wparam.0 as u16;
    match message {
        WM_KEYDOWN if key == VK_UP.0 => {
            with(|launcher| launcher.step(-1));
            return LRESULT(0);
        }
        WM_KEYDOWN if key == VK_DOWN.0 => {
            with(|launcher| launcher.step(1));
            return LRESULT(0);
        }
        WM_KEYDOWN if key == VK_RETURN.0 => {
            with(|launcher| launcher.open(launcher.selected));
            return LRESULT(0);
        }
        WM_KEYDOWN if key == VK_ESCAPE.0 => {
            with(Launcher::hide);
            return LRESULT(0);
        }
        WM_KEYDOWN if key == VK_TAB.0 => return LRESULT(0),
        // Enter, Esc and Tab would beep in a one-line field.
        WM_CHAR if matches!(wparam.0, 0x0D | 0x1B | 0x09) => return LRESULT(0),
        WM_SYSKEYDOWN if (u16::from(b'1')..=u16::from(b'9')).contains(&key) => {
            let index = usize::from(key - u16::from(b'1'));
            with(|launcher| launcher.open(index));
            return LRESULT(0);
        }
        // Alt and a number would beep: there is no menu to look in.
        WM_SYSCHAR if (u32::from(b'1')..=u32::from(b'9')).contains(&(wparam.0 as u32)) => {
            return LRESULT(0);
        }
        WM_GETOBJECT => {
            if let Some(result) = access::get_field_object(hwnd, wparam, lparam) {
                return result;
            }
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// What the E2E suite reads of the launcher (test build only).
#[cfg(feature = "e2e")]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub open: bool,
    pub query: String,
    pub results: Vec<String>,
    /// What each result shows under its name.
    pub subtitles: Vec<String>,
    pub selected: Option<usize>,
    pub note: &'static str,
}

/// Asks the launcher for its `View`, from a thread other than the main one.
#[cfg(feature = "e2e")]
pub fn view() -> Option<View> {
    let (sent, received) = std::sync::mpsc::channel();
    post(move |launcher| {
        let _ = sent.send(View {
            open: is_open(),
            query: launcher.query.clone(),
            results: launcher
                .results
                .iter()
                .map(|found| found.name.clone())
                .collect(),
            subtitles: launcher
                .results
                .iter()
                .map(|found| found.subtitle().to_owned())
                .collect(),
            selected: (!launcher.results.is_empty()).then_some(launcher.selected),
            note: match launcher.note {
                Note::None => "none",
                Note::Reading => "reading",
                Note::Hint => "hint",
                Note::NothingFound => "nothingFound",
            },
        });
    });
    received.recv_timeout(Duration::from_secs(5)).ok()
}

/// Types `text` into the open launcher's box, as the keyboard would (test
/// build only).
#[cfg(feature = "e2e")]
pub fn type_text(text: String) {
    post(move |launcher| launcher.set_query(&text));
}

/// Presses a key of the box (`up`, `down`, `escape`), as the keyboard would
/// (test build only). Enter is left out: it opens a program.
#[cfg(feature = "e2e")]
pub fn press_key(key: String) -> bool {
    let code = match key.as_str() {
        "up" => VK_UP,
        "down" => VK_DOWN,
        "escape" => VK_ESCAPE,
        _ => return false,
    };
    let frame = FRAME.load(Ordering::SeqCst);
    post(move |launcher| unsafe {
        let _ = PostMessageW(
            Some(launcher.edit),
            WM_KEYDOWN,
            WPARAM(usize::from(code.0)),
            LPARAM(0),
        );
    });
    frame != 0
}
