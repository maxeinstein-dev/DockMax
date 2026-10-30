//! The launcher (spec 0012): a search box opened by Alt+Space from
//! anywhere, or by "Buscar apps…" in the dock's and the tray's menus, that
//! finds the programs of the Apps folder and opens them (L1); and, mixed in
//! the same list (L2), the result of a sum, a web search by keyword and the
//! settings pages of Windows and of the Dockyard; and, under them (L4), the
//! files and folders of the computer (`files`, in the helper process); and,
//! alone, the folder of a path typed (L5, `paths`); and, between the apps
//! and the files, or alone with `j ` (L7, `open_windows`), the open windows.
//! Each result does one
//! action of a closed list (`search::Action`), checked before it runs; its
//! menu of actions (L6: Shift+Enter, the Menu key, the right button) offers
//! the others its kind has (`launcher_actions`), in the app's own menu.
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
mod files;
pub mod icons;
pub(crate) mod layout;
mod open_windows;
mod paint;
mod paths;
mod rates;

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
    GetKeyState, HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, SetFocus, UnregisterHotKey,
    VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_SHIFT, VK_TAB, VK_UP,
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
    WM_PAINT, WM_RBUTTONUP, WM_SETFONT, WM_SETTINGCHANGE, WM_SIZE, WM_SYSCHAR, WM_SYSKEYDOWN,
    WM_TIMER, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, w};

use self::layout::{BOX, Colors, ROW, Rgb};
use crate::domain::hotkey::{self, Hotkey};
use crate::domain::launcher::{self as search, Action, Candidate, MAX_RESULTS};
use crate::domain::launcher_actions::{self, Subject, Verb};
use crate::domain::launcher_calc;
use crate::domain::launcher_files::{self, FileHit, FileQuery, Generation, Mode};
use crate::domain::launcher_pages::{self, Target};
use crate::domain::launcher_paths::{self, Browse, Entry, PathQuery, Problem, Reading};
use crate::domain::launcher_units;
use crate::domain::launcher_web::{self, WebKeyword};
use crate::domain::launcher_windows::{self, Window, WindowQuery};
use crate::persistence::database::Storage;

/// The classes of the frame and of the list.
const FRAME_CLASS: PCWSTR = w!("DockMaxLauncher");
const LIST_CLASS: PCWSTR = w!("DockMaxLauncherList");

/// Runs the work queued by `post`.
const WM_RUN: u32 = WM_APP + 0x41;
/// Opens the menu of actions of the selected result (L6), out of the
/// launcher's borrow: the menu runs a loop of its own.
const WM_ACTION_MENU: u32 = WM_APP + 0x42;
/// `WM_CONTEXTMENU`: the Menu key or Shift+F10 in the text field (its
/// `lParam` is -1 then), or its right button.
const WM_CONTEXTMENU: u32 = 0x007B;
/// The character of Ctrl+C, and of Ctrl+Enter.
const COPY_CHAR: usize = 0x03;
const LINE_FEED_CHAR: usize = 0x0A;
/// `EN_CHANGE`: the text field's text changed.
const EN_CHANGE: u32 = 0x0300;
/// `EM_SETCUEBANNER`: the text the field shows while empty.
const EM_SETCUEBANNER: u32 = 0x1501;
/// `EM_GETSEL` and `EM_SETSEL`: the field's selection (the cursor).
const EM_GETSEL: u32 = 0x00B0;
const EM_SETSEL: u32 = 0x00B1;
/// The characters of Backspace and Tab, as `WM_CHAR` brings them.
const BACKSPACE_CHAR: usize = 0x08;

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
/// Looks for the files a moment after the last key (L4): the apps answer at
/// once.
const FILES_TIMER: usize = 4;
/// Stops waiting for the folder of a path typed (L5).
const FOLDER_TIMER: usize = 5;

/// `WM_MOUSEWHEEL`, and the rows one notch of the wheel moves.
const WM_MOUSEWHEEL: u32 = 0x020A;
const WHEEL_ROWS: isize = 3;

/// The search glyph of Windows' icon font.
const SEARCH_GLYPH: &str = "\u{E721}";
/// The glyphs of the results that are not programs: a settings page, the
/// calculator, the web.
const SETTINGS_GLYPH: char = '\u{E713}';
const CALCULATOR_GLYPH: char = '\u{E8EF}';
const WEB_GLYPH: char = '\u{E774}';
/// A conversion's (spec 0012 §5b).
const CONVERSION_GLYPH: char = '\u{E8AB}';
/// A file's and a folder's, while their icon is read.
const FILE_GLYPH: char = '\u{E7C3}';
const FOLDER_GLYPH: char = '\u{E8B7}';
/// An open window's, while its program's icon is read, or when it has none
/// (L7).
const WINDOW_GLYPH: char = '\u{E737}';

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
/// The launcher stays open when it loses the keyboard: to its menu of
/// actions, or to Windows' question of an opening as administrator (L6).
static HOLD: AtomicBool = AtomicBool::new(false);
/// The menu of actions is on screen.
static MENU_OPEN: AtomicBool = AtomicBool::new(false);

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
    DockyardPage,
    Calculator,
    /// A conversion of units or currencies (§5b).
    Conversion,
    Web,
    File,
    Folder,
    /// An open window (L7).
    Window,
}

impl Kind {
    /// Opening it is kept in the use (the apps, the pages and the files);
    /// switching to a window is not (a window lives a short while).
    fn remembered(self) -> bool {
        matches!(
            self,
            Kind::App | Kind::WindowsPage | Kind::DockyardPage | Kind::File | Kind::Folder
        )
    }

    fn glyph(self) -> Option<char> {
        match self {
            Kind::App => None,
            Kind::WindowsPage | Kind::DockyardPage => Some(SETTINGS_GLYPH),
            Kind::Calculator => Some(CALCULATOR_GLYPH),
            Kind::Conversion => Some(CONVERSION_GLYPH),
            Kind::Web => Some(WEB_GLYPH),
            Kind::File => Some(FILE_GLYPH),
            Kind::Folder => Some(FOLDER_GLYPH),
            Kind::Window => Some(WINDOW_GLYPH),
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
    /// The open window it is, for one (L7).
    window: Option<Window>,
}

impl Found {
    /// The program's id, for a program (its icon).
    fn app_id(&self) -> Option<&str> {
        match &self.action {
            Action::OpenApp(id) if self.kind == Kind::App => Some(id),
            _ => None,
        }
    }

    /// The id its icon is read by, and its program's name: a program's, or
    /// an open window's program's (L7).
    fn icon_app(&self) -> Option<(&str, &str)> {
        if let Some(id) = self.app_id() {
            return Some((id, &self.name));
        }
        let window = self.window.as_ref()?;
        Some((window.icon.as_deref()?, &window.app))
    }

    /// An open window (L7): its title, and under it "Janela · <program>".
    fn window(window: &Window) -> Found {
        let text = &crate::i18n::messages().launcher;
        Found {
            key: launcher_windows::key(window.hwnd),
            kind: Kind::Window,
            name: window.title.clone(),
            subtitle: (!window.app.is_empty())
                .then(|| text.window_of.replace("{app}", &window.app)),
            action: Action::FocusWindow(window.hwnd),
            window: Some(window.clone()),
        }
    }

    fn subtitle(&self) -> &str {
        if let Some(subtitle) = &self.subtitle {
            return subtitle;
        }
        self.kind_name()
    }

    /// What it is, for assistive technology: a file's and a folder's say so
    /// ("Arquivo", "Pasta"), the others are what is under their name.
    fn kind_name(&self) -> &str {
        let text = &crate::i18n::messages().launcher;
        match self.kind {
            Kind::App => text.app,
            Kind::WindowsPage => text.windows_page,
            Kind::DockyardPage => text.dockyard_page,
            Kind::Calculator => text.calculator,
            Kind::Conversion => text.conversion,
            Kind::Web => text.web,
            Kind::File => text.file,
            Kind::Folder => text.folder,
            Kind::Window => text.window,
        }
    }

    /// A file or folder found: its name, and under it the folder it is in,
    /// the user's profile written `~`.
    fn file(hit: &FileHit, profile: &str) -> Found {
        Found {
            key: launcher_files::file_key(hit),
            kind: if hit.folder { Kind::Folder } else { Kind::File },
            name: hit.name().to_owned(),
            subtitle: Some(launcher_files::abbreviate(hit.parent(), profile)),
            action: Action::OpenFile(hit.path.clone()),
            window: None,
        }
    }

    /// An entry of the folder `folder` (ending in `\`) of a path typed
    /// (L5): its name, and under it what it is ("Pasta", "Arquivo"), the
    /// folder being the one typed. `None` when its path is not one the
    /// launcher opens (too long).
    fn entry(folder: &str, entry: &Entry) -> Option<Found> {
        let hit = FileHit {
            path: format!("{folder}{}", entry.name),
            folder: entry.folder,
        };
        launcher_files::is_openable(&hit.path).then(|| Found {
            key: launcher_files::file_key(&hit),
            kind: if hit.folder { Kind::Folder } else { Kind::File },
            name: entry.name.clone(),
            subtitle: None,
            action: Action::OpenFile(hit.path),
            window: None,
        })
    }

    /// The path of the file or folder it opens, for one.
    fn path(&self) -> Option<&str> {
        match &self.action {
            Action::OpenFile(path) => Some(path),
            _ => None,
        }
    }

    /// The file or folder it opens, for one.
    fn file_hit(&self) -> Option<FileHit> {
        match &self.action {
            Action::OpenFile(path) if matches!(self.kind, Kind::File | Kind::Folder) => {
                Some(FileHit {
                    path: path.clone(),
                    folder: self.kind == Kind::Folder,
                })
            }
            _ => None,
        }
    }

    /// What its icon is kept under, with the icon pack `pack`: a program's
    /// id, or a file's `launcher_files::icon_key` behind a control
    /// character no program's id has.
    fn icon_key(&self, pack: &Option<String>) -> Option<(Option<String>, String)> {
        if let Some((id, _)) = self.icon_app() {
            return Some((pack.clone(), id.to_owned()));
        }
        let hit = self.file_hit()?;
        // On a share (L5) the row keeps its glyph: the shell would go to the
        // network for it, and hold the helper's other icons meanwhile.
        if !launcher_files::is_valid_path(&hit.path) {
            return None;
        }
        Some((None, format!("\u{1}{}", launcher_files::icon_key(&hit))))
    }

    fn page(page: &launcher_pages::Page) -> Found {
        let (kind, action) = match page.target {
            Target::Windows(uri) => (Kind::WindowsPage, Action::OpenUri(uri.to_owned())),
            Target::Dockyard(section) => (Kind::DockyardPage, Action::OpenSection(section)),
        };
        Found {
            key: launcher_pages::page_key(page.id),
            kind,
            name: page.name.to_owned(),
            subtitle: None,
            action,
            window: None,
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
            window: None,
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
    /// The folder of a path typed shows nothing, and why (L5).
    Folder(Problem),
}

impl Note {
    /// What the list says, if anything.
    fn text(self) -> Option<&'static str> {
        let text = &crate::i18n::messages().launcher;
        match self {
            Note::None => None,
            Note::Reading => Some(text.reading),
            Note::Hint => Some(text.hint),
            Note::NothingFound => Some(text.nothing_found),
            Note::Folder(Problem::NotFound) => Some(text.folder_not_found),
            Note::Folder(Problem::Denied) => Some(text.folder_denied),
            Note::Folder(Problem::Slow) => Some(text.folder_slow),
        }
    }
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
    /// Reads the icons from Windows and searches the files in a helper
    /// process, while the launcher is open.
    helper: Option<icons::Reader>,
    /// What is typed asks for files (L4), and the ones of the last search,
    /// in the order shown.
    file_query: Option<FileQuery>,
    files: Vec<FileHit>,
    /// Tells the searches of files apart: an answer to an older typing is
    /// dropped.
    generation: Generation,
    /// What is typed is a path (L5): its folder and filter, and the reading
    /// of that folder, done once while the filter changes.
    path_query: Option<PathQuery>,
    browse: Browse,
    /// The user's profile folder, written `~` under a file's name.
    profile: String,
    /// The first row shown: the list scrolls past `MAX_RESULTS` rows.
    first: usize,
    font: Option<HFONT>,
    glyph_font: Option<HFONT>,
    brush: Option<HBRUSH>,
    colors: Colors,
    scale: f64,
    /// The pointer's last place over the list: a row is selected by the
    /// mouse only when it moves, not when the list opens under it.
    pointer: Option<(i32, i32)>,
    /// The file of each classic program the menu of actions asked the shell
    /// about (`None`: it has none to show), until the launcher is released.
    app_files: HashMap<String, Option<String>>,
    /// The menu of actions was asked for, and where it opens.
    menu_request: Option<Placement>,
    /// What is typed asks for the open windows (L7), and the windows read
    /// for it, in the stacking order; with what describes their programs:
    /// the Apps folder's programs (id, name) and the dock's pinned ones
    /// (the dock's id, the program's id, its name), read on opening.
    window_query: Option<WindowQuery>,
    windows: Vec<Window>,
    apps: Vec<(String, String)>,
    pinned: Vec<(String, String, String)>,
}

/// Where the menu of actions opens.
#[derive(Debug, Clone, Copy)]
enum Placement {
    /// At the pointer, on screen (the right button).
    Pointer(POINT),
    /// Beside the selected row (the keyboard).
    Row,
}

/// A menu of actions ready to open.
struct MenuPlan {
    owner: HWND,
    /// The result it is about, and its actions, one per line.
    key: String,
    verbs: Vec<Verb>,
    lines: Vec<super::menu::Line>,
    at: POINT,
    keyboard: bool,
}

/// The glyphs of the menu of actions.
const OPEN_GLYPH: char = '\u{E8A7}';
const ADMIN_GLYPH: char = '\u{E7EF}';
const PIN_GLYPH: char = '\u{E718}';
const UNPIN_GLYPH: char = '\u{E77A}';
const REVEAL_GLYPH: char = '\u{E838}';
const COPY_GLYPH: char = '\u{E8C8}';
const SWITCH_GLYPH: char = '\u{E8A7}';
const CLOSE_GLYPH: char = '\u{E8BB}';

/// A line of the menu of actions: its text, its glyph and the keys that do
/// the same without the menu.
fn menu_line(verb: Verb) -> super::menu::Line {
    let text = &crate::i18n::messages().launcher;
    let (label, glyph, keys) = match verb {
        Verb::Open => (text.action_open, OPEN_GLYPH, Some("Enter")),
        Verb::OpenAsAdmin => (text.action_admin, ADMIN_GLYPH, Some("Ctrl+Shift+Enter")),
        Verb::Pin => (text.action_pin, PIN_GLYPH, None),
        Verb::Unpin => (text.action_unpin, UNPIN_GLYPH, None),
        Verb::Reveal => (text.action_reveal, REVEAL_GLYPH, None),
        Verb::CopyPath => (text.action_copy_path, COPY_GLYPH, Some("Ctrl+C")),
        Verb::CopyResult => (text.action_copy_result, COPY_GLYPH, Some("Ctrl+C")),
        Verb::Switch => (text.action_switch, SWITCH_GLYPH, Some("Enter")),
        Verb::CloseWindow => (text.action_close_window, CLOSE_GLYPH, None),
    };
    let item = super::menu::Item::new(label, glyph);
    super::menu::Line::Item(match keys {
        Some(keys) => item.shortcut(keys),
        None => item,
    })
}

/// Opens the menu of actions asked for (`WM_ACTION_MENU`), out of the
/// launcher's borrow: while the menu's loop runs, the launcher answers its
/// messages (the files, the icons) and stays open without the keyboard.
fn open_action_menu() {
    let mut plan = None;
    with(|launcher| plan = launcher.menu_plan());
    let Some(plan) = plan else {
        return;
    };
    HOLD.store(true, Ordering::SeqCst);
    MENU_OPEN.store(true, Ordering::SeqCst);
    let chosen = super::menu::show_from(
        Some(plan.owner),
        Some(plan.at),
        false,
        plan.lines,
        plan.keyboard,
    );
    MENU_OPEN.store(false, Ordering::SeqCst);
    HOLD.store(false, Ordering::SeqCst);
    let verb = chosen.and_then(|index| plan.verbs.get(index).copied());
    with(|launcher| launcher.menu_closed(&plan.key, verb));
}

/// The outcome of opening something as administrator.
type Elevated = Result<(), super::apps::Elevation>;

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
        helper: None,
        file_query: None,
        files: Vec::new(),
        generation: Generation::default(),
        path_query: None,
        browse: Browse::default(),
        profile: String::new(),
        first: 0,
        font: None,
        glyph_font: None,
        brush: None,
        colors: layout::colors(false),
        scale: 1.0,
        pointer: None,
        app_files: HashMap::new(),
        menu_request: None,
        window_query: None,
        windows: Vec::new(),
        apps: Vec::new(),
        pinned: Vec::new(),
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

/// Whether Configurações is capturing a new hotkey.
pub fn capturing() -> bool {
    CAPTURING.load(Ordering::SeqCst)
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
pub(crate) fn high_contrast() -> bool {
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

pub(crate) fn system_color(index: SYS_COLOR_INDEX) -> Rgb {
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
pub(crate) fn text_scale() -> f64 {
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

/// The conversions `query` asks for (§5b), as rows: "= 6,21 milhas" over
/// "10 km"; Enter copies the number.
fn conversions(query: &str, decimal: char) -> Vec<Found> {
    let text = &crate::i18n::messages().launcher;
    let cache = rates::current();
    let rates = cache.as_deref().map(|cache| &cache.rates);
    launcher_units::convert(query, rates)
        .iter()
        .enumerate()
        .map(|(index, conversion)| {
            let shown = launcher_units::show(
                conversion,
                decimal,
                |id, plural| text.unit_name(id, plural),
                text.rates_of,
            );
            Found {
                key: format!("convert:{index}"),
                kind: Kind::Conversion,
                name: shown.title,
                subtitle: Some(shown.subtitle),
                action: Action::Copy(shown.copied),
                window: None,
            }
        })
        .collect()
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
        self.profile = std::env::var("USERPROFILE").unwrap_or_default();
        self.query.clear();
        self.file_query = None;
        self.files.clear();
        self.path_query = None;
        self.browse.clear();
        self.failed = None;
        self.launching = None;
        self.pointer = None;
        self.window_query = None;
        self.pinned = self.read_pinned();
        self.read_windows();
        unsafe {
            let _ = SetWindowTextW(self.edit, w!(""));
        }
        self.read_catalog();
        rates::prepare(&self.app, || post(Launcher::rates_arrived));
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
            let _ = KillTimer(Some(self.frame), FILES_TIMER);
            let _ = KillTimer(Some(self.frame), FOLDER_TIMER);
        }
        self.query.clear();
        self.results.clear();
        self.menu_request = None;
        self.file_query = None;
        self.files.clear();
        // A search of files, or a folder being read, answers nothing.
        self.generation.next();
        self.path_query = None;
        self.browse.clear();
        self.window_query = None;
        self.windows.clear();
        access::publish(Vec::new());
        // The helper leaves, and the memory of reading the icons and of
        // searching the files with it; the icons it had not answered are
        // asked again next time.
        self.helper = None;
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
        self.app_files.clear();
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
        layout::list_height(self.shown(), self.recent && !self.results.is_empty())
    }

    /// How many rows are on screen: the list scrolls past `MAX_RESULTS`.
    fn shown(&self) -> usize {
        self.results.len().min(MAX_RESULTS)
    }

    /// The results on screen, as indices.
    fn visible(&self) -> std::ops::Range<usize> {
        let end = (self.first + MAX_RESULTS).min(self.results.len());
        self.first.min(end)..end
    }

    /// Scrolls so that the row `index` is on screen; whether it moved.
    fn reveal(&mut self, index: usize) -> bool {
        let before = self.first;
        if index < self.first {
            self.first = index;
        } else if index >= self.first + MAX_RESULTS {
            self.first = index + 1 - MAX_RESULTS;
        }
        self.first != before
    }

    /// Scrolls by `rows` (the mouse wheel), staying in the list.
    fn scroll(&mut self, rows: isize) {
        let last = self.results.len().saturating_sub(MAX_RESULTS) as isize;
        let first = (self.first as isize + rows).clamp(0, last.max(0)) as usize;
        if first != self.first {
            self.first = first;
            self.request_icons();
            self.publish();
            self.paint_soon();
        }
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
        self.apps = apps
            .iter()
            .map(|app| (app.id.clone(), app.name.clone()))
            .collect();
        let mut entries: Vec<Found> = apps
            .into_iter()
            .map(|app| Found {
                key: search::app_key(&app.id),
                kind: Kind::App,
                name: app.name,
                subtitle: None,
                action: Action::OpenApp(app.id),
                window: None,
            })
            .collect();
        entries.extend(pages);
        candidates.extend(page_candidates);
        self.entries = entries;
        self.candidates = candidates;
        self.apps_read = true;
        if is_open() {
            self.refresh_keeping_selection();
            self.after_change();
        }
    }

    /// The results for what is typed now: the sum's result first; then a
    /// keyword's search alone, or the apps and the pages; and, when nothing
    /// matched, the search on the web with the first keyword.
    fn refresh(&mut self) {
        if self.path_query.is_some() {
            self.refresh_path();
            return;
        }
        let query = self.query.trim().to_owned();
        let mut results = Vec::new();
        let files: Vec<Found> = self
            .files
            .iter()
            .map(|hit| Found::file(hit, &self.profile))
            .collect();
        let windows: Vec<Found> = match &self.window_query {
            Some(wanted) => launcher_windows::rank(&self.windows, wanted)
                .into_iter()
                .map(|index| Found::window(&self.windows[index]))
                .collect(),
            None => Vec::new(),
        };
        let windows_only = self
            .window_query
            .as_ref()
            .is_some_and(|wanted| wanted.mode == launcher_windows::Mode::Only);
        if windows_only {
            // `j `: the open windows alone, every one with nothing after it.
            results = windows;
        } else if query.is_empty() {
            results = self.recents();
        } else if self
            .file_query
            .as_ref()
            .is_some_and(|wanted| wanted.mode == Mode::Only)
        {
            // `f term`: the files and folders alone.
            results = files;
        } else {
            if let Some(value) = launcher_calc::evaluate(&query) {
                let shown = launcher_calc::format(value, self.decimal);
                results.push(Found {
                    key: "calc".to_owned(),
                    kind: Kind::Calculator,
                    name: format!("= {shown}"),
                    subtitle: None,
                    action: Action::Copy(shown),
                    window: None,
                });
            } else {
                // A conversion, first too (§5b).
                results.extend(conversions(&query, self.decimal));
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
                let fallback = self
                    .keywords
                    .first()
                    .and_then(|keyword| Found::web(keyword, &query));
                // The windows in a band of their own after the apps and
                // the pages (L7), then the files, after everything else.
                results = launcher_windows::mix(results, windows, files, fallback);
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
        self.first = 0;
    }

    /// The currency rates came (§5b): a currency typed meanwhile converts.
    fn rates_arrived(&mut self) {
        let cache = rates::current();
        let rates = cache.as_deref().map(|cache| &cache.rates);
        if is_open()
            && self.path_query.is_none()
            && launcher_units::is_currency(&launcher_units::convert(self.query.trim(), rates))
        {
            self.refresh();
            self.after_change();
        }
    }

    /// The results of a path typed (L5), alone: the entries of its folder
    /// the filter keeps, or why there are none.
    fn refresh_path(&mut self) {
        let Some(path) = &self.path_query else {
            return;
        };
        let (results, note) = match (&self.browse.reading, &path.folder) {
            (Reading::Ready(entries), Some(folder)) => {
                let results: Vec<Found> =
                    launcher_paths::list(entries, &path.filter, launcher_paths::LIMIT)
                        .into_iter()
                        .filter_map(|entry| Found::entry(folder, entry))
                        .collect();
                let note = if results.is_empty() {
                    Note::NothingFound
                } else {
                    Note::None
                };
                (results, note)
            }
            (Reading::Failed(problem), _) => (Vec::new(), Note::Folder(*problem)),
            (_, None) => (Vec::new(), Note::Folder(Problem::NotFound)),
            // Being read: a moment without rows.
            _ => (Vec::new(), Note::None),
        };
        self.recent = false;
        self.results = results;
        self.note = note;
        self.selected = 0;
        self.first = 0;
    }

    /// What is typed is a path (L5): reads its folder off the launcher's
    /// thread when it is not the one already read; the filter alone
    /// filters what was read.
    fn browse_path(&mut self, path: PathQuery) {
        if let Some(generation) = self.browse.want(path.folder.as_deref())
            && let Some(folder) = path.folder.clone()
        {
            unsafe {
                SetTimer(
                    Some(self.frame),
                    FOLDER_TIMER,
                    launcher_paths::TIMEOUT_MS,
                    None,
                )
            };
            paths::read(folder, move |read| {
                post(move |launcher| launcher.folder_read(generation, read));
            });
        }
        self.path_query = Some(path);
    }

    /// The folder of the read `generation` came: shown when it is still
    /// the one typed, dropped when the typing went on or it came late.
    fn folder_read(&mut self, generation: u64, read: Result<Vec<Entry>, Problem>) {
        if !is_open() || !self.browse.answered(generation, read) {
            return;
        }
        let _ = unsafe { KillTimer(Some(self.frame), FOLDER_TIMER) };
        self.refresh();
        self.after_change();
        self.announce_soon();
    }

    /// The folder typed did not answer in time: the row says so.
    fn folder_late(&mut self) {
        let _ = unsafe { KillTimer(Some(self.frame), FOLDER_TIMER) };
        if let Reading::Waiting(generation) = self.browse.reading
            && self.browse.timed_out(generation)
        {
            log::info!("launcher: a folder did not answer in time");
            self.refresh();
            self.after_change();
            self.announce_soon();
        }
    }

    /// Tab: the selected file or folder goes into the field as a path, and
    /// a folder is entered (its `\`), as in the Ulauncher. Of a path typed,
    /// its name after the folder typed (`~\Doc` → `~\Documents\`); of a file
    /// found (L4) or a recent, its full path. Whether it did.
    fn complete(&mut self) -> bool {
        let Some(found) = self.results.get(self.selected) else {
            return false;
        };
        let Some(hit) = found.file_hit() else {
            return false;
        };
        let text = match &self.path_query {
            Some(path) => launcher_paths::complete(&path.typed, &found.name, hit.folder),
            None => launcher_paths::complete("", hit.path.trim_end_matches('\\'), hit.folder),
        };
        self.set_query(&text);
        self.caret_to_end();
        true
    }

    /// Backspace with the cursor after a `\` of a path typed: the folder
    /// above, as in the Ulauncher (`C:\Users\ana\` → `C:\Users\`). Whether
    /// it did; when not, Backspace erases a character, as always.
    fn go_up(&mut self) -> bool {
        let text = read_text(self.edit);
        let length = text.encode_utf16().count();
        let (mut start, mut end) = (0u32, 0u32);
        unsafe {
            SendMessageW(
                self.edit,
                EM_GETSEL,
                Some(WPARAM(std::ptr::from_mut(&mut start) as usize)),
                Some(LPARAM(std::ptr::from_mut(&mut end) as isize)),
            );
        }
        // Only with the cursor at the end, nothing selected.
        if start != end || end as usize != length {
            return false;
        }
        let Some(above) = launcher_paths::up(text.trim_start()) else {
            return false;
        };
        self.set_query(&above);
        self.caret_to_end();
        true
    }

    /// The cursor after the last character of the field.
    fn caret_to_end(&self) {
        let length = read_text(self.edit).encode_utf16().count();
        unsafe {
            SendMessageW(
                self.edit,
                EM_SETSEL,
                Some(WPARAM(length)),
                Some(LPARAM(length as isize)),
            );
        }
    }

    /// The most used, with nothing typed: the apps, the pages and the files
    /// together, at most `MAX_RESULTS`.
    fn recents(&self) -> Vec<Found> {
        let weight = |key: &str| self.usage.get(key).copied().unwrap_or(0.0);
        let mut used: Vec<(f64, Found)> =
            search::recents(&self.candidates, &self.usage, MAX_RESULTS)
                .into_iter()
                .map(|index| {
                    let found = self.entries[index].clone();
                    (weight(&found.key), found)
                })
                .collect();
        used.extend(self.usage.iter().filter_map(|(key, weight)| {
            let hit = launcher_files::hit_of_key(key)?;
            (*weight > 0.0).then(|| (*weight, Found::file(&hit, &self.profile)))
        }));
        used.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.name.cmp(&b.1.name))
                .then_with(|| a.1.key.cmp(&b.1.key))
        });
        used.into_iter()
            .take(MAX_RESULTS)
            .map(|(_, found)| found)
            .collect()
    }

    /// What `query` asks of the files, if anything: not with a sum nor a
    /// keyword's web search, which come alone.
    fn file_query_for(&self, query: &str) -> Option<FileQuery> {
        let wanted = launcher_files::parse(query, self.files_keyword_taken())?;
        let alone = launcher_calc::evaluate(query).is_some()
            || launcher_web::matching(&self.keywords, query).is_some();
        // `j term` (L7, read first) shows the windows alone: no files.
        let windows_only = self
            .window_query
            .as_ref()
            .is_some_and(|windows| windows.mode == launcher_windows::Mode::Only);
        (wanted.mode == Mode::Only || (!alone && !windows_only)).then_some(wanted)
    }

    /// What `typed` (as typed) asks of the open windows (L7), if anything:
    /// not with a sum nor a keyword's web search, which come alone; `j `
    /// asks for them alone, unless the user made `j` a web keyword.
    fn window_query_for(&self, typed: &str) -> Option<WindowQuery> {
        let taken = self.keywords.iter().any(|keyword| {
            keyword
                .keyword
                .eq_ignore_ascii_case(launcher_windows::KEYWORD)
        });
        let wanted = launcher_windows::parse(typed, taken)?;
        let query = typed.trim();
        let alone = launcher_calc::evaluate(query).is_some()
            || launcher_web::matching(&self.keywords, query).is_some()
            || launcher_files::parse(query, self.files_keyword_taken())
                .is_some_and(|files| files.mode == Mode::Only);
        (wanted.mode == launcher_windows::Mode::Only || !alone).then_some(wanted)
    }

    /// Whether the user made `f` a web keyword (which then wins over the
    /// files').
    fn files_keyword_taken(&self) -> bool {
        self.keywords.iter().any(|keyword| {
            keyword
                .keyword
                .eq_ignore_ascii_case(launcher_files::KEYWORD)
        })
    }

    /// The dock's pinned programs (its id, the program's id and name), for
    /// the windows' names and icons (L7).
    fn read_pinned(&self) -> Vec<(String, String, String)> {
        let Some(state) = self.app.try_state::<Mutex<Storage>>() else {
            return Vec::new();
        };
        let storage = state.lock().unwrap_or_else(PoisonError::into_inner);
        storage
            .apps_snapshot()
            .map(|snapshot| {
                snapshot
                    .apps
                    .into_iter()
                    .map(|app| (app.id.to_string(), app.app_id, app.name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reads the open windows again (L7), on this thread: it takes well
    /// under 5 ms (`open_windows`).
    fn read_windows(&mut self) {
        let start = std::time::Instant::now();
        self.windows = open_windows::read(&open_windows::Programs {
            pinned: &self.pinned,
            catalog: &self.apps,
        });
        let took = start.elapsed();
        if took > std::time::Duration::from_millis(5) {
            log::info!(
                "launcher: reading {} windows took {} ms",
                self.windows.len(),
                took.as_millis()
            );
        }
    }

    /// The results changed under the selection: it stays on the same result
    /// when that one is still there.
    fn refresh_keeping_selection(&mut self) {
        let selected_key = self
            .results
            .get(self.selected)
            .map(|found| found.key.clone());
        let first = self.first;
        self.refresh();
        if let Some(index) =
            selected_key.and_then(|key| self.results.iter().position(|found| found.key == key))
        {
            self.selected = index;
            self.first = first.min(self.results.len().saturating_sub(MAX_RESULTS));
            self.reveal(index);
        }
    }

    /// Asks the helper for the files of what is typed (the timer's, once
    /// the typing pauses).
    fn search_files(&mut self) {
        let _ = unsafe { KillTimer(Some(self.frame), FILES_TIMER) };
        let Some(wanted) = self.file_query.clone() else {
            return;
        };
        let generation = self.generation.next();
        let limit = wanted.limit();
        self.helper().ask(icons::Request::Files {
            generation,
            words: wanted.words,
            limit,
        });
    }

    /// The files of the search `generation` came: shown when it is the
    /// last one, dropped when the typing went on.
    fn files_found(&mut self, generation: u64, hits: Vec<FileHit>) {
        if !self.generation.is_current(generation) || !is_open() {
            return;
        }
        let Some(wanted) = &self.file_query else {
            return;
        };
        self.files = launcher_files::rank(hits, &wanted.words, &self.usage, wanted.limit());
        self.refresh_keeping_selection();
        self.after_change();
        self.announce_soon();
    }

    /// The helper, started the first time it is needed after the launcher
    /// opens.
    fn helper(&mut self) -> &icons::Reader {
        self.helper.get_or_insert_with(|| {
            icons::Reader::start(|answer| {
                post(move |launcher| match answer {
                    icons::Answer::Icon { key, png } => launcher.icon_read(&key, png),
                    icons::Answer::Files { generation, hits } => {
                        launcher.files_found(generation, hits);
                    }
                });
            })
            .unwrap_or_else(icons::Reader::closed)
        })
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
        // A search of files of the text before answers nothing.
        self.generation.next();
        // A path (L5): only its folder answers, no apps, no web.
        let path = launcher_paths::parse(self.query.trim_start(), &self.profile, |name| {
            std::env::var(name).ok()
        });
        if let Some(path) = path {
            self.file_query = None;
            self.files.clear();
            self.window_query = None;
            let _ = unsafe { KillTimer(Some(self.frame), FILES_TIMER) };
            self.browse_path(path);
            self.refresh();
            self.after_change();
            self.announce_soon();
            return;
        }
        if self.path_query.take().is_some() {
            self.browse.clear();
            let _ = unsafe { KillTimer(Some(self.frame), FOLDER_TIMER) };
        }
        // The open windows (L7) are read again at each key: cheap, and a
        // window opened or closed meanwhile shows up or goes.
        self.window_query = self.window_query_for(&self.query);
        if self.window_query.is_some() {
            self.read_windows();
        }
        let wanted = self.file_query_for(self.query.trim());
        match &wanted {
            Some(wanted) => {
                // The rows that still match stay until the new ones come,
                // so the list does not blink at each key.
                self.files
                    .retain(|hit| launcher_files::still_matches(hit, &wanted.words));
                self.files.truncate(wanted.limit());
                unsafe {
                    SetTimer(
                        Some(self.frame),
                        FILES_TIMER,
                        launcher_files::DEBOUNCE_MS,
                        None,
                    )
                };
            }
            None => {
                self.files.clear();
                let _ = unsafe { KillTimer(Some(self.frame), FILES_TIMER) };
            }
        }
        self.file_query = wanted;
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
        let size = self.icon_size();
        let visible = self.visible();
        let files: Vec<((Option<String>, String), String)> = self.results[visible.clone()]
            .iter()
            .filter_map(|found| Some((found.icon_key(&pack)?, found.file_hit()?.path)))
            .collect();
        for (key, path) in files {
            if self.icons.contains_key(&key) {
                continue;
            }
            self.icons.insert(key.clone(), Loaded::Loading);
            self.helper().ask(icons::Request::File {
                key: key.1,
                path,
                size,
            });
        }
        let shown: Vec<(String, String)> = self.results[visible]
            .iter()
            .filter_map(|found| {
                let (id, name) = found.icon_app()?;
                Some((id.to_owned(), name.to_owned()))
            })
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
                    let dark = super::native_dock::system_dark();
                    let png = crate::commands::icons::pack_icon(&app, &pack, &names, dark);
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

    /// The side of an icon, in pixels.
    fn icon_size(&self) -> u32 {
        (layout::ICON * self.scale).round().clamp(16.0, 256.0) as u32
    }

    /// Asks Windows for the icon of the program `id`, through the helper.
    fn ask_windows(&mut self, id: String) {
        let size = self.icon_size();
        self.helper().ask(icons::Request::App { id, size });
    }

    /// The icon of the program or file kept under `id` came (`None`: it has
    /// none).
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
        let visible = self.visible();
        let entries = self
            .results
            .iter()
            .enumerate()
            .map(|(index, found)| access::Entry {
                key: found.key.clone(),
                name: found.name.clone(),
                kind: found.kind_name().to_owned(),
                // A row scrolled out of the list has no box on the screen.
                rect: if visible.contains(&index) {
                    (
                        f64::from(origin.x) + layout::SIDE * s,
                        f64::from(origin.y) + (top + (index - self.first) as f64 * ROW) * s,
                        (layout::WIDTH - 2.0 * layout::SIDE) * s,
                        ROW * s,
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                },
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
            Note::None if self.results.len() == 1 => text.launcher.one_result.to_owned(),
            // Being read (L5): the count comes when the folder does.
            Note::None if self.results.is_empty() => return,
            note @ (Note::Reading | Note::Hint | Note::NothingFound | Note::Folder(_)) => {
                note.text().unwrap_or_default().to_owned()
            }
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
        if self.reveal(index) {
            self.request_icons();
        }
        self.publish();
        self.paint_soon();
        let found = &self.results[index];
        let said = crate::i18n::messages()
            .launcher
            .selected
            .replace("{name}", &found.name)
            .replace("{kind}", found.kind_name())
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

    /// Does what the result `index` does (Enter).
    fn open(&mut self, index: usize) {
        if let Some(found) = self.results.get(index).cloned() {
            let action = found.action.clone();
            self.perform(&found, action);
        }
    }

    /// Does `verb` of the menu of actions to the result `index` (L6).
    fn act(&mut self, index: usize, verb: Verb) {
        let Some(found) = self.results.get(index).cloned() else {
            return;
        };
        // The program's file: a classic program's, as the shell answered; a
        // window's program's (L7), as its process says.
        let file = match &found.window {
            Some(window) => window
                .exe
                .clone()
                .filter(|exe| launcher_windows::is_revealable(exe)),
            None => found
                .app_id()
                .and_then(|id| self.app_files.get(id).cloned().flatten()),
        };
        match launcher_actions::action(verb, &found.action, file.as_deref()) {
            Some(action) => self.perform(&found, action),
            None => log::warn!("launcher: an action does not apply to its result"),
        }
    }

    /// Does `action` to `found`, once it is checked: the launcher closes
    /// once it is done, or stays saying it could not.
    fn perform(&mut self, found: &Found, action: Action) {
        if self.launching.is_some() {
            return;
        }
        let key = found.key.clone();
        // Opening counts in the use, as administrator too; showing, copying
        // and pinning do not.
        let remembered = found.kind.remembered()
            && matches!(
                action,
                Action::OpenApp(_)
                    | Action::OpenUri(_)
                    | Action::OpenFile(_)
                    | Action::OpenSection(_)
                    | Action::OpenAppAsAdmin(_)
                    | Action::OpenFileAsAdmin(_)
            );
        if !search::is_allowed(&action) {
            log::warn!("launcher: a result's action was refused");
            self.launched(&key, false, false);
            return;
        }
        let text = &crate::i18n::messages().launcher;
        match action {
            Action::OpenApp(id) => {
                self.open_off_thread(key, remembered, move || super::apps::launch(&id))
            }
            Action::OpenUri(uri) => {
                self.open_off_thread(key, remembered, move || super::apps::open_uri(&uri))
            }
            Action::OpenFile(path) => {
                self.open_off_thread(key, remembered, move || super::apps::open_path(&path))
            }
            Action::Reveal(path) => {
                self.open_off_thread(key, false, move || super::apps::reveal_path(&path))
            }
            Action::OpenAppAsAdmin(id) => self.elevate(key, remembered, move |parent| {
                super::apps::open_app_as_admin(&id, parent)
            }),
            Action::OpenFileAsAdmin(path) => self.elevate(key, remembered, move |parent| {
                super::apps::open_path_as_admin(&path, parent)
            }),
            Action::OpenSection(section) => {
                let opened = super::settings_window::open_section(&self.app, section)
                    .inspect_err(|error| {
                        log::warn!("launcher: Configurações did not open: {error}")
                    })
                    .is_ok();
                self.launched(&key, remembered, opened);
            }
            Action::Copy(copied) => self.copy(&key, &copied, text.copied),
            Action::CopyPath(path) => self.copy(&key, &path, text.path_copied),
            Action::PinApp(id) => self.pin(&key, &id, &found.name, true),
            Action::UnpinApp(id) => self.pin(&key, &id, &found.name, false),
            Action::FocusWindow(hwnd) => self.window_action(&key, hwnd, true),
            Action::CloseWindow(hwnd) => self.window_action(&key, hwnd, false),
        }
    }

    /// Brings the window `hwnd` to the front (`focus`), restoring it if it
    /// is minimized, or asks it to close (L7), with the dock's own calls;
    /// the launcher closes. Never counted in the use. A window that is gone
    /// since it was listed (or whose handle went to another) says it could
    /// not.
    fn window_action(&mut self, key: &str, hwnd: isize, focus: bool) {
        if !open_windows::still_listed(hwnd) {
            log::info!("launcher: a window was gone");
            self.launched(key, false, false);
            return;
        }
        if focus {
            // The launcher is in front: Windows lets it hand the front over.
            super::openwin::focus(hwnd);
        } else {
            super::openwin::close(hwnd);
        }
        super::open_apps::wake(&self.app);
        self.launched(key, false, true);
    }

    /// Pins the program `id` to the dock (`wanted`) or takes it out, as the
    /// panel does; the launcher closes.
    fn pin(&mut self, key: &str, id: &str, name: &str, wanted: bool) {
        let done = crate::commands::apps::pin_from_launcher(&self.app, id, name, wanted)
            .inspect_err(|error| log::warn!("launcher: the dock did not change: {error:?}"))
            .is_ok();
        self.launched(key, false, done);
    }

    /// Copies `text` and says `said` (the launcher closes, as opening).
    fn copy(&mut self, key: &str, text: &str, said: &str) {
        let copied = copy_text(self.frame, text);
        if copied {
            access::announce(self.list.0 as isize, said);
        }
        self.launched(key, false, copied);
    }

    /// Runs `run` as administrator on a thread of its own (Windows asks the
    /// user first); the launcher stays open while Windows asks. Saying no
    /// is not an error: the launcher stays, with nothing said.
    fn elevate(
        &mut self,
        key: String,
        remembered: bool,
        run: impl FnOnce(isize) -> Elevated + Send + 'static,
    ) {
        self.launching = Some(key.clone());
        HOLD.store(true, Ordering::SeqCst);
        let parent = self.frame.0 as isize;
        let thread_key = key.clone();
        let spawned = std::thread::Builder::new()
            .name("launcher-admin".into())
            .spawn(move || {
                let answer = run(parent);
                post(move |launcher| launcher.elevated(&thread_key, remembered, answer));
            });
        if spawned.is_err() {
            HOLD.store(false, Ordering::SeqCst);
            self.launched(&key, remembered, false);
        }
    }

    /// Windows answered an opening as administrator.
    fn elevated(&mut self, key: &str, remembered: bool, answer: Elevated) {
        HOLD.store(false, Ordering::SeqCst);
        match answer {
            Ok(()) => self.launched(key, remembered, true),
            Err(super::apps::Elevation::Cancelled) => {
                log::info!("launcher: opening as administrator was cancelled");
                self.launching = None;
                self.take_keyboard(true);
            }
            Err(super::apps::Elevation::Failed(error)) => {
                log::warn!("launcher: it did not open as administrator: {error}");
                self.launched(key, remembered, false);
                self.take_keyboard(true);
            }
        }
    }

    /// Whether the workspace in use has the program `id` pinned.
    fn pinned(&self, id: &str) -> bool {
        let Some(state) = self.app.try_state::<Mutex<Storage>>() else {
            return false;
        };
        let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
        crate::commands::apps::pinned_here(&mut storage, id)
            .ok()
            .flatten()
            .is_some()
    }

    /// What `found` is, for its menu of actions; `pinned` asks the dock
    /// whether a program is pinned (only the menu shows it).
    fn subject<'a>(&self, found: &'a Found, pinned: bool) -> Subject<'a> {
        match found.kind {
            Kind::App => {
                let id = found.app_id().unwrap_or_default();
                Subject::App {
                    id,
                    pinned: pinned && self.pinned(id),
                    has_file: self.app_files.get(id).is_some_and(Option::is_some),
                }
            }
            Kind::File => Subject::File {
                path: found.path().unwrap_or_default(),
            },
            Kind::Folder => Subject::Folder,
            Kind::WindowsPage | Kind::DockyardPage => Subject::Page,
            Kind::Web => Subject::Web,
            // A conversion copies its number, as a sum (§5b).
            Kind::Calculator | Kind::Conversion => Subject::Calculator,
            Kind::Window => Subject::Window {
                revealable: found
                    .window
                    .as_ref()
                    .and_then(|window| window.exe.as_deref())
                    .is_some_and(launcher_windows::is_revealable),
            },
        }
    }

    /// The actions of the selected result.
    fn selected_verbs(&self, pinned: bool) -> Vec<Verb> {
        self.results
            .get(self.selected)
            .map(|found| launcher_actions::verbs(self.subject(found, pinned)))
            .unwrap_or_default()
    }

    /// Shift+Enter: the menu of actions of the selected result; with one
    /// action only (a page, a web search, a sum), that action at once.
    fn shift_enter(&mut self) {
        let verbs = self.selected_verbs(false);
        if launcher_actions::opens_menu(&verbs) {
            self.request_menu(Placement::Row);
        } else if let Some(&verb) = verbs.first() {
            self.act(self.selected, verb);
        }
    }

    /// Ctrl+Shift+Enter: the selected result as administrator, when its
    /// kind can be (a program, a program's file or a shortcut).
    fn open_as_admin(&mut self) {
        if self.selected_verbs(false).contains(&Verb::OpenAsAdmin) {
            self.act(self.selected, Verb::OpenAsAdmin);
        }
    }

    /// Ctrl+C: with nothing selected in the field, the selected result's
    /// path (or a sum's result) is copied. Whether it was the result's.
    fn copy_selected(&mut self) -> bool {
        let (start, end) = self.selection();
        let verbs = self.selected_verbs(false);
        if !launcher_actions::ctrl_c_copies_result(start, end, &verbs) {
            return false;
        }
        if let Some(verb) = launcher_actions::copy_verb(&verbs) {
            self.act(self.selected, verb);
        }
        true
    }

    /// Whether the selected result has a menu of actions.
    fn selected_has_menu(&self) -> bool {
        launcher_actions::opens_menu(&self.selected_verbs(false))
    }

    /// Asks for the menu of actions of the selected result, at `placement`.
    /// For a classic program, the shell is asked for its file first (off the
    /// main thread), to know whether "Mostrar no Explorador" is offered.
    fn request_menu(&mut self, placement: Placement) {
        if self.launching.is_some() || MENU_OPEN.load(Ordering::SeqCst) {
            return;
        }
        let Some(found) = self.results.get(self.selected) else {
            return;
        };
        if let Some(id) = found.app_id()
            && launcher_actions::may_have_file(id)
            && !self.app_files.contains_key(id)
        {
            let id = id.to_owned();
            let key = found.key.clone();
            let thread_id = id.clone();
            let spawned = std::thread::Builder::new()
                .name("launcher-app-file".into())
                .spawn(move || {
                    let file = super::apps::target_of(&thread_id)
                        .filter(|file| file.is_file())
                        .map(|file| file.to_string_lossy().into_owned())
                        .filter(|file| launcher_files::is_openable(file));
                    post(move |launcher| {
                        launcher.app_files.insert(thread_id, file);
                        let same = launcher
                            .results
                            .get(launcher.selected)
                            .is_some_and(|found| found.key == key);
                        if is_open() && same {
                            launcher.menu_request = Some(placement);
                            launcher.post_menu();
                        }
                    });
                });
            if spawned.is_ok() {
                return;
            }
            self.app_files.insert(id, None);
        }
        self.menu_request = Some(placement);
        self.post_menu();
    }

    /// Has the frame open the menu asked for, once the launcher is free.
    fn post_menu(&self) {
        let _ = unsafe { PostMessageW(Some(self.frame), WM_ACTION_MENU, WPARAM(0), LPARAM(0)) };
    }

    /// The menu of actions asked for, ready to open; `None` when it was not
    /// asked for, or the selected result has no menu.
    fn menu_plan(&mut self) -> Option<MenuPlan> {
        let placement = self.menu_request.take()?;
        if !is_open() || self.launching.is_some() {
            return None;
        }
        let found = self.results.get(self.selected)?;
        let verbs = launcher_actions::verbs(self.subject(found, true));
        if !launcher_actions::opens_menu(&verbs) {
            return None;
        }
        let (at, keyboard) = match placement {
            Placement::Pointer(at) => (at, false),
            Placement::Row => (self.row_point(), true),
        };
        Some(MenuPlan {
            owner: self.frame,
            key: found.key.clone(),
            lines: verbs.iter().copied().map(menu_line).collect(),
            verbs,
            at,
            keyboard,
        })
    }

    /// Where the menu of the selected row opens: under it, at its right
    /// end, on screen.
    fn row_point(&self) -> POINT {
        let mut origin = POINT::default();
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::ClientToScreen(self.list, &mut origin);
        }
        let s = self.scale;
        let top = layout::LIST_PAD + if self.recent { layout::TITLE } else { 0.0 };
        let row = self.selected.saturating_sub(self.first) as f64;
        POINT {
            x: origin.x + ((layout::WIDTH - layout::SIDE) * s).round() as i32,
            y: origin.y + ((top + (row + 1.0) * ROW) * s).round() as i32,
        }
    }

    /// The menu of actions closed, with `verb` chosen for the result `key`
    /// or nothing. Let go by a click elsewhere, the launcher closes too, as
    /// it does when it loses the keyboard; let go by Esc, it takes the
    /// keyboard back.
    fn menu_closed(&mut self, key: &str, verb: Option<Verb>) {
        if !is_open() {
            return;
        }
        if !self.take_keyboard(false) {
            return;
        }
        let Some(verb) = verb else {
            return;
        };
        if let Some(index) = self.results.iter().position(|found| found.key == key) {
            self.act(index, verb);
        }
    }

    /// Takes the keyboard back into the search box after the menu or
    /// Windows' question. Unless `always`, only when the foreground is still
    /// the Dockyard's: another program took it, the launcher closes. Whether
    /// it stayed open.
    fn take_keyboard(&mut self, always: bool) -> bool {
        use windows::Win32::System::Threading::GetCurrentProcessId;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        if !is_open() {
            return false;
        }
        let foreground = unsafe { GetForegroundWindow() };
        let mut process = 0u32;
        if !foreground.is_invalid() {
            unsafe { GetWindowThreadProcessId(foreground, Some(&mut process)) };
        }
        let ours = foreground.is_invalid() || process == unsafe { GetCurrentProcessId() };
        if !always && !ours {
            self.hide();
            return false;
        }
        let taken =
            unsafe { SetForegroundWindow(self.frame) }.as_bool() || foreground == self.frame;
        if !taken && !ours {
            self.hide();
            return false;
        }
        let _ = unsafe { SetFocus(Some(self.edit)) };
        true
    }

    /// The text field's selection, as UTF-16 positions.
    fn selection(&self) -> (u32, u32) {
        let (mut start, mut end) = (0u32, 0u32);
        unsafe {
            SendMessageW(
                self.edit,
                EM_GETSEL,
                Some(WPARAM(std::ptr::from_mut(&mut start) as usize)),
                Some(LPARAM(std::ptr::from_mut(&mut end) as isize)),
            );
        }
        (start, end)
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
        let message = self.note.text();
        let rows: Vec<paint::Row> = self
            .results
            .iter()
            .enumerate()
            .skip(self.first)
            .take(MAX_RESULTS)
            .map(|(index, found)| {
                let icon = found
                    .icon_key(&pack)
                    .and_then(|key| match self.icons.get(&key) {
                        Some(Loaded::Ready(Some(bitmap))) => Some(bitmap),
                        _ => None,
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
                    shortcut: layout::shortcut(index - self.first),
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
        if let Some(index) = layout::row_at(at, self.shown(), self.recent) {
            self.select(self.first + index);
        }
    }

    fn clicked(&mut self, y: i32) {
        let at = f64::from(y) / self.scale;
        if let Some(index) = layout::row_at(at, self.shown(), self.recent) {
            self.selected = self.first + index;
            self.open(self.first + index);
        }
    }

    /// The right button on a row: it is selected and its menu of actions
    /// opens at the pointer (L6); nothing on a result with one action only.
    fn right_clicked(&mut self, x: i32, y: i32) {
        let at = f64::from(y) / self.scale;
        let Some(index) = layout::row_at(at, self.shown(), self.recent) else {
            return;
        };
        self.select(self.first + index);
        if !self.selected_has_menu() {
            return;
        }
        let mut point = POINT { x, y };
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::ClientToScreen(self.list, &mut point);
        }
        self.request_menu(Placement::Pointer(point));
    }
}

/// The text typed in `edit`.
fn read_text(edit: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(edit) }.max(0) as usize;
    let mut buffer = vec![0u16; length + 1];
    let read = unsafe { GetWindowTextW(edit, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..read])
}

/// The rows a turn of the wheel scrolls (down is positive).
fn wheel_rows(wparam: WPARAM) -> isize {
    let delta = isize::from(((wparam.0 >> 16) & 0xFFFF) as u16 as i16);
    -delta.signum() * WHEEL_ROWS
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
        WM_ACTION_MENU => open_action_menu(),
        // While the menu of actions is open, the hotkey waits: the menu
        // belongs to the launcher.
        WM_HOTKEY if wparam.0 as i32 == HOTKEY => {
            if !MENU_OPEN.load(Ordering::SeqCst) {
                with(Launcher::toggle);
            }
        }
        WM_ACTIVATE => {
            // Somewhere else took the keyboard: the launcher closes, as the
            // Ulauncher does; not to its own menu of actions, nor while
            // Windows asks about an opening as administrator (L6).
            if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE && !HOLD.load(Ordering::SeqCst) {
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
            FILES_TIMER => with(Launcher::search_files),
            FOLDER_TIMER => with(Launcher::folder_late),
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
        // The right button on a row: its menu of actions, at the pointer (L6).
        WM_RBUTTONUP => {
            let (x, y) = point(lparam);
            with(|launcher| launcher.right_clicked(x, y));
        }
        WM_MOUSEWHEEL => with(|launcher| launcher.scroll(wheel_rows(wparam))),
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
        // Enter opens; Shift+Enter, the menu of actions; Ctrl+Shift+Enter,
        // as administrator (L6).
        WM_KEYDOWN if key == VK_RETURN.0 => {
            let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
            let control = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
            with(|launcher| match (control, shift) {
                (true, true) => launcher.open_as_admin(),
                (false, true) => launcher.shift_enter(),
                _ => launcher.open(launcher.selected),
            });
            return LRESULT(0);
        }
        // The Menu key and Shift+F10 (`lParam` -1): the menu of actions of
        // the selected result, when it has one; otherwise, and for the
        // right button on the field, the field's own menu.
        WM_CONTEXTMENU if lparam.0 as i32 == -1 => {
            let mut opened = false;
            with(|launcher| {
                opened = launcher.selected_has_menu();
                if opened {
                    launcher.request_menu(Placement::Row);
                }
            });
            if opened {
                return LRESULT(0);
            }
        }
        // Ctrl+C with nothing selected copies the selected result's path
        // (L6); with text selected, the text, as always.
        WM_CHAR if wparam.0 == COPY_CHAR => {
            let mut copied = false;
            with(|launcher| copied = launcher.copy_selected());
            if copied {
                return LRESULT(0);
            }
        }
        WM_KEYDOWN if key == VK_ESCAPE.0 => {
            with(Launcher::hide);
            return LRESULT(0);
        }
        // Tab completes the selected file or folder into the field (L5); the
        // focus stays in the field, as it always did (the list takes no
        // focus: the screen reader follows the selection through UI
        // Automation). Shift+Tab does nothing.
        WM_KEYDOWN if key == VK_TAB.0 => {
            let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
            if !shift {
                with(|launcher| {
                    launcher.complete();
                });
            }
            return LRESULT(0);
        }
        // Backspace after a `\` of a path goes up a folder (L5); in the
        // character, since the field erases there.
        WM_CHAR if wparam.0 == BACKSPACE_CHAR => {
            let mut went_up = false;
            with(|launcher| went_up = launcher.go_up());
            if went_up {
                return LRESULT(0);
            }
        }
        // Enter, Ctrl+Enter, Esc and Tab would beep in a one-line field.
        WM_CHAR if matches!(wparam.0, 0x0D | LINE_FEED_CHAR | 0x1B | 0x09) => {
            return LRESULT(0);
        }
        WM_SYSKEYDOWN if (u16::from(b'1')..=u16::from(b'9')).contains(&key) => {
            let index = usize::from(key - u16::from(b'1'));
            with(|launcher| launcher.open(launcher.first + index));
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
        // The wheel over the search box scrolls the list.
        WM_MOUSEWHEEL => {
            with(|launcher| launcher.scroll(wheel_rows(wparam)));
            return LRESULT(0);
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
                Note::Folder(Problem::NotFound) => "folderNotFound",
                Note::Folder(Problem::Denied) => "folderDenied",
                Note::Folder(Problem::Slow) => "folderSlow",
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

/// Presses a key of the box (`up`, `down`, `escape`, and since L5 `tab` and
/// `backspace`), as the keyboard would (test build only). Enter is left out:
/// it opens a program.
#[cfg(feature = "e2e")]
pub fn press_key(key: String) -> bool {
    let (message, code) = match key.as_str() {
        "up" => (WM_KEYDOWN, usize::from(VK_UP.0)),
        "down" => (WM_KEYDOWN, usize::from(VK_DOWN.0)),
        "escape" => (WM_KEYDOWN, usize::from(VK_ESCAPE.0)),
        "tab" => (WM_KEYDOWN, usize::from(VK_TAB.0)),
        // The field erases on the character, not on the key.
        "backspace" => (WM_CHAR, BACKSPACE_CHAR),
        _ => return false,
    };
    let frame = FRAME.load(Ordering::SeqCst);
    post(move |launcher| unsafe {
        let _ = PostMessageW(Some(launcher.edit), message, WPARAM(code), LPARAM(0));
    });
    frame != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str) -> Found {
        Found {
            key: format!("app:{name}"),
            kind: Kind::App,
            name: name.to_owned(),
            subtitle: None,
            action: Action::OpenApp(name.to_owned()),
            window: None,
        }
    }

    #[test]
    fn a_conversion_is_a_row_that_copies_its_number() {
        let rows = conversions("10 km em milhas", ',');
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.kind, Kind::Conversion);
        assert_eq!(row.name, "= 6,21 milhas");
        assert_eq!(row.subtitle(), "10 km");
        assert_eq!(row.action, Action::Copy("6,21".into()));
        assert!(search::is_allowed(&row.action));
        assert!(
            !row.kind.remembered(),
            "a conversion is not kept in the use"
        );
        // "Copiar resultado", as a sum's: Enter, Shift+Enter and Ctrl+C.
        let verbs = launcher_actions::verbs(Subject::Calculator);
        assert_eq!(verbs, [Verb::CopyResult]);
        assert_eq!(
            launcher_actions::action(Verb::CopyResult, &row.action, None),
            Some(Action::Copy("6,21".into()))
        );
        // A unit alone: its common conversions, each its own row.
        let keys: Vec<String> = conversions("10 km", ',')
            .into_iter()
            .map(|row| row.key)
            .collect();
        assert_eq!(keys, ["convert:0", "convert:1", "convert:2"]);
    }

    #[test]
    fn a_conversion_comes_first_and_holds_the_web_search_back() {
        let mut first = conversions("5 gb em mb", ',');
        first.push(app("Gerenciador de Tarefas"));
        let fallback = Some(app("web"));
        let mixed = launcher_windows::mix(
            first,
            vec![app("janela")],
            vec![app("arquivo")],
            fallback.clone(),
        );
        let names: Vec<&str> = mixed.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(
            names,
            ["= 5000 MB", "Gerenciador de Tarefas", "janela", "arquivo"]
        );
        // Alone, it is still a result: no search on the web.
        let mixed = launcher_windows::mix(
            conversions("5 gb em mb", ','),
            Vec::new(),
            Vec::new(),
            fallback,
        );
        assert_eq!(mixed.len(), 1);
        assert_eq!(mixed[0].kind, Kind::Conversion);
    }

    #[test]
    fn without_rates_a_currency_is_no_row() {
        // No rates are fetched in the tests: the currencies give nothing.
        assert!(rates::current().is_none());
        assert!(conversions("100 usd em brl", ',').is_empty());
        assert!(conversions("50 euros para reais", ',').is_empty());
    }
}
