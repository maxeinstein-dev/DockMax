//! The dock, drawn natively (spec 0011 H3, ADR 0011): a layered window
//! painted with Direct2D, with no WebView2. The WebView2 is loaded only
//! when a panel or the Configurações app opens.
//!
//! One dock per monitor (spec 0010 D4b), each with its own window, its own
//! band on its monitor's edge and its own full screen rule; they show the
//! same items.
//!
//! Everything about the docks lives on the main thread (the one of the event
//! loop, which also runs their windows' procedure). Other threads reach them
//! through `post`, which queues the work and wakes them; what other threads
//! need to read (whether they are on screen, where they are) is kept in
//! `SHARED`.

pub(crate) mod access;
mod faces;
mod look;
mod paint;
mod strip;
mod svg;
mod theme;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyCursor, DestroyWindow, GetWindowRect,
    HCURSOR, HWND_MESSAGE, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, PostMessageW,
    RegisterClassW, SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursor, SetTimer, SetWindowPos,
    ShowWindow, SystemParametersInfoW, WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_GETOBJECT, WM_HOTKEY, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_SETTINGCHANGE,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{PCWSTR, w};

use self::paint::{Dot, Face, Painter, Scene, Status, StripCell, Tile, Tip};
use self::strip::Cell;
use self::theme::{Color, Palette};
use super::dock::Rect;
use super::menu;
use super::monitors::Monitor;
use super::panel::{LogicalRect, PanelId};
use crate::domain::apps::AppsSnapshot;
use crate::domain::badges::{self, Badge};
use crate::domain::dock_items::{self, Item, Names, SystemItem};
use crate::domain::dock_layout::{
    self, Area, DRAG_GROW, DRAG_LIFT, DRAG_THRESHOLD, DockSize, Hover, Magnification, Shape,
    drag_offset, drag_pitch, drop_index, make_room, slide_step,
};
use crate::domain::open_apps::OpenSnapshot;
use crate::domain::settings::{DockEdge, MiddleClick, OpenAnimation, Settings, TaskbarMode, Theme};
use crate::domain::widgets::{WidgetConfig, WidgetsSnapshot};
use crate::domain::workspaces::Workspace;
use crate::persistence::database::Storage;

/// The class of the docks' windows (and of the hidden one that wakes them).
const CLASS: PCWSTR = w!("DockMaxDock");

/// `WM_MOUSELEAVE`, which the features in use do not name.
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Runs the work queued by `post`.
const WM_RUN: u32 = WM_APP + 0x31;

const ANIMATION_TIMER: usize = 1;
const CLOCK_TIMER: usize = 2;
const WARM_TIMER: usize = 3;
/// The pointer rested on an open program for the preview's delay (spec
/// 0015).
const PREVIEW_TIMER: usize = 6;
const STATUS_TIMER: usize = 4;
/// The pointer rested on the Calendário: its card shows (spec 0017 F2a-4).
const CARD_TIMER: usize = 8;
/// How long the pointer rests before the Calendário's card shows.
const CARD_DELAY: u32 = 450;
/// Reads the processor, the memory and the network again while a system
/// widget is on the dock (spec 0017 F1).
const ACTIVITY_TIMER: usize = 7;
/// The next frame of a long text that rolls (the Tocando agora, F4).
const MARQUEE_TIMER: usize = 9;
/// Looks at the pointer while the dock hides on its own.
const AUTO_HIDE_TIMER: usize = 5;
/// The key of a workspace dot for assistive technology, before its id.
const WORKSPACE_KEY: &str = "workspace:";
const AUTO_HIDE_EVERY: u32 = 100;

/// How often the battery, the network and the volume are read again.
const STATUS_EVERY: u32 = 3_000;
/// How often the system widgets read again.
const ACTIVITY_EVERY: u32 = 2_000;

/// A frame of the animations.
const FRAME: Duration = Duration::from_millis(16);
/// How long a program's icon animates after a click.
const OPENING: Duration = Duration::from_millis(1500);
/// How long the pointer rests on an item before its panel is warmed
/// (spec 0011 H2).
const WARM_DELAY: u32 = 150;
/// For how long after a click on the dock the panel it opens may take the
/// focus: the click is the last input, so Windows lets DockMax have it.
const CLICK_GRANTS_FOCUS: Duration = Duration::from_secs(2);

/// What other threads read about one dock.
#[derive(Debug, Clone)]
struct Placed {
    hwnd: isize,
    /// Its monitor: its key, its handle (an `HMONITOR` as a number), its
    /// rectangle and its work area.
    monitor: String,
    handle: isize,
    rect: Rect,
    work: Rect,
    visible: bool,
    /// The window on the screen, in physical pixels.
    window: Rect,
    scale: f64,
    /// The bar on the screen, in physical pixels.
    bar: Rect,
}

/// What other threads read about the docks (one per monitor).
#[derive(Debug, Default)]
struct Shared {
    /// The primary monitor's first.
    docks: Vec<Placed>,
    /// The window of the dock whose item opened the panel last: the panel
    /// anchors to it.
    active: isize,
    /// When a dock was last clicked.
    clicked: Option<Instant>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    docks: Vec::new(),
    active: 0,
    clicked: None,
});

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The hidden window that wakes the docks for the work of other threads
/// (0 before it is made).
static WAKE: AtomicIsize = AtomicIsize::new(0);

/// Whether the docks are on screen, as the user (or the app) last said: a
/// dock made for a monitor plugged in meanwhile shows only then.
static SHOWN: AtomicBool = AtomicBool::new(false);

static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

type Work = Box<dyn FnOnce(&mut Vec<Dock>) + Send>;

static QUEUE: Mutex<Vec<Work>> = Mutex::new(Vec::new());

thread_local! {
    /// One per monitor, the primary monitor's first.
    static DOCKS: RefCell<Vec<Dock>> = const { RefCell::new(Vec::new()) };
}

/// Something the dock asks of the rest of the app, done once the dock's own
/// state is no longer borrowed (it may call back into the dock).
type Effect = Box<dyn FnOnce(&AppHandle)>;

/// A press on an item that may become a drag.
#[derive(Debug, Clone, Copy)]
struct Press {
    index: usize,
    /// The item's group, where it can go: its first index and its size.
    group: (usize, usize),
    start_x: f64,
    dragging: bool,
    /// Off the dock: a program's tile on its way to a folder (spec 0018).
    out: bool,
}

/// A drag let go that moved its item: the item stays where it was let go,
/// and the others keep the room they made, until the dock reads the new
/// order (or for `LANDING`, when it never comes).
#[derive(Debug, Clone, Copy)]
struct Landing {
    press: Press,
    /// The pointer's x when it was let go.
    x: f64,
    since: Instant,
}

/// The longest a drag let go waits for the new order.
const LANDING: Duration = Duration::from_secs(1);

/// How far the items of a workspace just shown start along the bar, in
/// logical pixels, before they glide to their places (spec 0013 C3).
const SWITCH_SLIDE: f64 = 36.0;

impl Press {
    /// The item's place in its group.
    fn local(&self) -> usize {
        self.index - self.group.0
    }

    /// The indices of its group.
    fn range(&self) -> std::ops::Range<usize> {
        self.group.0..self.group.0 + self.group.1
    }
}

/// The cover of a track, and the icon of its program, as bitmaps (F4).
type MediaArt = (
    (String, String, String),
    Option<ID2D1Bitmap>,
    Option<ID2D1Bitmap>,
);

/// What the dock reads from the rest of the app.
struct Data {
    settings: Settings,
    widgets: Option<WidgetsSnapshot>,
    apps: Option<AppsSnapshot>,
    open: OpenSnapshot,
    /// The workspace the dock's monitor shows (spec 0013), and every one,
    /// for the switcher; `None` and empty when they can't be read.
    workspace: Option<Workspace>,
    workspaces: Vec<Workspace>,
    /// The icons of the notification area (while the dock holds it).
    tray: Vec<crate::commands::tray::TrayIcon>,
}

/// A program's icon: being read, or read (`None` when it has none).
enum Loaded {
    Loading,
    Ready(Option<ID2D1Bitmap>),
}

pub struct Dock {
    app: AppHandle,
    hwnd: HWND,
    /// The monitor it is on.
    monitor: Monitor,
    painter: Painter,
    data: Data,
    items: Vec<Item>,
    names: Vec<String>,
    groups: Vec<usize>,
    label_widths: Vec<f64>,
    dark: bool,
    motion: bool,
    /// The window in physical pixels and the monitor's scale.
    window: Rect,
    scale: f64,
    /// Each item's growth as drawn now, moving toward its target.
    growth: Vec<Magnification>,
    hovered: Option<usize>,
    status_hovered: bool,
    /// The workspace dot under the pointer (spec 0013).
    switcher_hovered: Option<usize>,
    /// The small icons at the end, and the one under the pointer.
    strip: Vec<Cell>,
    strip_hovered: Option<usize>,
    /// The battery, the network and the volume, as last read.
    system: Option<super::system_status::Status>,
    /// The last reading for the system widgets; `None` while none is on
    /// the dock, when nothing is read.
    activity: Option<crate::domain::system_activity::Reading>,
    /// How far each item has risen above the bar, 0 to 1 (a Calendário
    /// under the pointer).
    widen: Vec<f64>,
    /// The tray icons' images, by icon key: the image they came from and
    /// the bitmap.
    tray_images: HashMap<String, (String, Option<ID2D1Bitmap>)>,
    /// The item held down by the left button.
    pressed: Option<usize>,
    press: Option<Press>,
    /// While dragging: the pointer's x.
    drag_x: f64,
    /// The icon the pointer shows while a program's tile is off the dock.
    drag_cursor: Option<HCURSOR>,
    /// The tile under files dragged over the dock (spec 0018).
    drop_hover: Option<usize>,
    /// The cover of what plays now, as a bitmap, with its track, and the
    /// icon of the program that plays it (F4).
    media_art: Option<MediaArt>,
    /// When the track now playing showed: where its long title rolls to.
    media_since: Instant,
    landing: Option<Landing>,
    /// How far each item is drawn from its place at rest, sideways: the
    /// room made for a dragged item, or the way to a new place, eased.
    slide: Vec<f64>,
    /// The panel open, anchored to one of the items.
    open: Option<PanelId>,
    /// The programs whose icon is animating after a click, by item key.
    opening: HashMap<String, Instant>,
    icons: HashMap<(Option<String>, String), Loaded>,
    badges: HashMap<String, Loaded>,
    /// The notification badge of each item (spec 0016), as the taskbar
    /// last had it.
    notices: Vec<Option<Badge>>,
    tracking: bool,
    animating: bool,
    visible: bool,
    /// Hidden because a program is in full screen on its monitor (spec 0010
    /// D4): it comes back when that ends.
    hidden_for_fullscreen: bool,
    /// Hiding on its own: how far off the screen the dock is drawn (0 in
    /// place, 1 all out), whether it is headed out, and since when the
    /// pointer is on the edge or away.
    tucked: f64,
    tucking: bool,
    on_edge_since: Option<Instant>,
    away_since: Option<Instant>,
    /// The dock's menu is open (it runs a loop of its own).
    menu_open: bool,
    effects: Vec<Effect>,
}

/// Queues `work` for the docks, from any thread, and wakes them.
fn queue(work: Work) {
    QUEUE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(work);
    let hwnd = WAKE.load(Ordering::SeqCst);
    if hwnd != 0 {
        let _ = unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_RUN, WPARAM(0), LPARAM(0)) };
    }
}

/// Queues `work` for every dock.
pub fn post(work: impl Fn(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        for dock in docks.iter_mut() {
            work(dock);
        }
    }));
}

/// Queues `work` for the dock of the window `hwnd`, if it is still there.
fn post_to(hwnd: isize, work: impl FnOnce(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        if let Some(dock) = docks.iter_mut().find(|dock| dock.hwnd.0 as isize == hwnd) {
            work(dock);
        }
    }));
}

/// Queues `work` for the dock of the primary monitor.
#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
fn post_primary(work: impl FnOnce(&mut Dock) + Send + 'static) {
    queue(Box::new(move |docks| {
        if let Some(dock) = docks.first_mut() {
            work(dock);
        }
    }));
}

/// Makes the docks' windows, one per monitor, hidden. On the main thread,
/// once.
pub fn start(app: &AppHandle) -> windows::core::Result<()> {
    // COM and OLE on the main thread, for the images (WIC) and the drop
    // target: WebView2 used to do it, and none runs at startup any more.
    if let Err(error) = unsafe { windows::Win32::System::Ole::OleInitialize(None) } {
        log::warn!("dock: OLE could not be initialized: {error}");
    }
    let _ = APP.set(app.clone());
    let instance = unsafe { GetModuleHandleW(None) }?;
    let class_data = WNDCLASSW {
        style: CS_DBLCLKS,
        lpfnWndProc: Some(procedure),
        hInstance: instance.into(),
        lpszClassName: CLASS,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    // A window only for messages: what other threads ask of the docks.
    let wake = unsafe {
        CreateWindowExW(
            Default::default(),
            CLASS,
            w!(""),
            Default::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
    }?;
    WAKE.store(wake.0 as isize, Ordering::SeqCst);
    with_docks(sync);
    Ok(())
}

/// Makes a dock on `monitor`, hidden.
fn make_dock(app: &AppHandle, monitor: Monitor) -> Option<Dock> {
    let instance = unsafe { GetModuleHandleW(None) }.ok()?;
    // Never activated by a click: the program the user is in stays in
    // front, and a click on a program knows which one that is.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("DockMax"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .inspect_err(|error| log::warn!("dock: the window could not be made: {error}"))
    .ok()?;
    let painter = match Painter::new() {
        Ok(painter) => painter,
        Err(error) => {
            log::warn!("dock: nothing to paint with: {error}");
            let _ = unsafe { DestroyWindow(hwnd) };
            return None;
        }
    };
    // Files dropped on the dock are pinned (spec 0006 B2).
    super::drop_target::install(app, hwnd);
    let data = read_data(app, &monitor.key);
    let dock = Dock {
        app: app.clone(),
        hwnd,
        monitor,
        painter,
        data,
        items: Vec::new(),
        names: Vec::new(),
        groups: Vec::new(),
        label_widths: Vec::new(),
        dark: theme::system_dark(),
        motion: system_motion(),
        window: Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        scale: 1.0,
        growth: Vec::new(),
        hovered: None,
        status_hovered: false,
        switcher_hovered: None,
        strip: Vec::new(),
        strip_hovered: None,
        system: Some(super::system_status::read()),
        activity: None,
        widen: Vec::new(),
        tray_images: HashMap::new(),
        pressed: None,
        press: None,
        drag_x: 0.0,
        drag_cursor: None,
        drop_hover: None,
        media_art: None,
        media_since: Instant::now(),
        landing: None,
        slide: Vec::new(),
        open: None,
        opening: HashMap::new(),
        icons: HashMap::new(),
        badges: HashMap::new(),
        notices: Vec::new(),
        tracking: false,
        animating: false,
        visible: false,
        hidden_for_fullscreen: false,
        tucked: 0.0,
        tucking: false,
        on_edge_since: None,
        away_since: None,
        menu_open: false,
        effects: Vec::new(),
    };
    dock.arm_clock();
    unsafe { SetTimer(Some(dock.hwnd), STATUS_TIMER, STATUS_EVERY, None) };
    Some(dock)
}

/// One dock per monitor (spec 0010 D4b), or only the primary monitor's when
/// the user asks for that: a dock is made for a monitor plugged in and
/// destroyed for one unplugged; the others learn where their monitor is now.
fn sync(docks: &mut Vec<Dock>) {
    let Some(app) = APP.get() else {
        return;
    };
    let every = crate::commands::settings::current(app).dock.every_monitor;
    // The primary monitor's first: alone, it is the one kept.
    let mut monitors = super::monitors::all();
    if !every {
        monitors.truncate(1);
    }
    docks.retain_mut(|dock| {
        let keep = monitors
            .iter()
            .any(|monitor| monitor.key == dock.monitor.key);
        if !keep {
            dock.destroy();
        }
        keep
    });
    for monitor in &monitors {
        match docks
            .iter_mut()
            .find(|dock| dock.monitor.key == monitor.key)
        {
            Some(dock) => dock.monitor = monitor.clone(),
            None => {
                if let Some(dock) = make_dock(app, monitor.clone()) {
                    docks.push(dock);
                }
            }
        }
    }
    // The primary monitor's first.
    docks.sort_by_key(|dock| {
        monitors
            .iter()
            .position(|monitor| monitor.key == dock.monitor.key)
    });
    let keys: Vec<String> = docks.iter().map(|dock| dock.monitor.key.clone()).collect();
    super::appbar::retain(&keys);
    let shown = SHOWN.load(Ordering::SeqCst);
    for dock in docks.iter_mut() {
        dock.rebuild();
        if shown && !dock.visible && !dock.hidden_for_fullscreen {
            dock.show();
        }
    }
}

/// Runs `change` on the docks, then what they asked of the app.
fn with_docks(change: impl FnOnce(&mut Vec<Dock>)) {
    let effects = DOCKS.with(|slot| {
        let Ok(mut docks) = slot.try_borrow_mut() else {
            return Vec::new();
        };
        change(&mut docks);
        docks
            .iter_mut()
            .flat_map(|dock| std::mem::take(&mut dock.effects))
            .collect::<Vec<Effect>>()
    });
    if let Some(app) = APP.get() {
        for effect in effects {
            effect(app);
        }
    }
}

/// Runs `change` on the dock of the window `hwnd`.
fn with_dock(hwnd: HWND, change: impl FnOnce(&mut Dock)) {
    with_docks(|docks| {
        if let Some(dock) = docks.iter_mut().find(|dock| dock.hwnd == hwnd) {
            change(dock);
        }
    });
}

/// The work queued by other threads, in order.
fn run_queue() {
    loop {
        let work: Vec<Work> =
            std::mem::take(&mut *QUEUE.lock().unwrap_or_else(PoisonError::into_inner));
        if work.is_empty() {
            return;
        }
        for item in work {
            with_docks(item);
        }
    }
}

/// What the dock on the monitor `monitor` (its device name) shows.
fn read_data(app: &AppHandle, monitor: &str) -> Data {
    let settings = crate::commands::settings::current(app);
    #[cfg(windows)]
    let tray = crate::commands::tray::icons();
    let (widgets, apps, workspace, workspaces): (_, _, Option<Workspace>, _) =
        match app.try_state::<Mutex<Storage>>() {
            Some(state) => {
                let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
                (
                    crate::commands::widgets::snapshot(&storage).ok(),
                    crate::commands::apps::snapshot(&storage).ok(),
                    storage
                        .shown_on(monitor)
                        .inspect_err(|error| log::warn!("dock: no workspace to show: {error}"))
                        .ok(),
                    storage.workspaces().unwrap_or_default(),
                )
            }
            None => (None, None, None, Vec::new()),
        };
    Data {
        settings,
        widgets,
        apps,
        open: super::open_apps::snapshot_for(workspace.as_ref().map(|w| w.id.as_str())),
        workspace,
        workspaces,
        tray,
    }
}

/// Whether Windows animates (its "animation effects"); on when it can't be
/// told.
fn system_motion() -> bool {
    let mut on = windows::core::BOOL(1);
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(std::ptr::from_mut(&mut on).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_err() || on.as_bool()
}

fn names() -> Names {
    let dock = &crate::i18n::messages().dock;
    Names {
        settings: dock.settings,
        clock: dock.clock,
        time_progress: dock.time_progress,
        note: dock.note,
        shortcut: dock.shortcut,
        files: dock.files,
        activity: dock.activity,
        network: dock.network,
        calendar: dock.calendar,
        cycle: dock.cycle,
        countdown: dock.countdown,
        pomodoro: dock.pomodoro,
        todo: dock.todo,
        water: dock.water,
        media: dock.media,
    }
}

/// The panel an item opens: a program's is its menu.
fn panel_of(item: &Item) -> PanelId {
    match item {
        Item::App(app) => PanelId::App(app.id.clone()),
        Item::Widget(widget) => PanelId::Widget(widget.id.clone()),
        Item::Other(other) => PanelId::Open(other.key.clone()),
        Item::System(SystemItem::Settings) => PanelId::Settings,
    }
}

fn logical(area: Area) -> LogicalRect {
    LogicalRect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    }
}

impl Dock {
    fn edge(&self) -> DockEdge {
        self.data.settings.dock.edge
    }

    fn shape(&self) -> Shape {
        let dock = &self.data.settings.dock;
        Shape {
            size: DockSize::of(dock.icon_size),
            groups: self.groups.clone(),
            hover: Hover::of(dock.hover, dock.hover_scale),
            status: dock.taskbar != TaskbarMode::Visible,
            tray: self.strip.len(),
            switcher: self.data.workspaces.len(),
            spans: self.items.iter().map(Item::span).collect(),
        }
    }

    /// The window's client area, in logical pixels.
    fn viewport(&self) -> (f64, f64) {
        (
            f64::from(self.window.width) / self.scale,
            f64::from(self.window.height) / self.scale,
        )
    }

    /// The Calendário two icons wide under the pointer, which rises: not
    /// while a panel is open or an item is dragged.
    fn opened_calendar(&self) -> Option<usize> {
        let dragging = self.press.is_some_and(|press| press.dragging);
        if self.open.is_some() || dragging {
            return None;
        }
        self.hovered.filter(|&index| {
            matches!(
                self.items.get(index),
                Some(Item::Widget(widget))
                    if matches!(&widget.config, WidgetConfig::Calendar(config) if config.width == 2)
            )
        })
    }

    /// The item magnified now: the one under the pointer, unless a panel is
    /// open, an item is being dragged or Windows asks for less motion.
    fn magnified(&self) -> Option<usize> {
        let dragging = self.press.is_some_and(|press| press.dragging);
        if !self.motion || self.open.is_some() || dragging {
            return None;
        }
        self.hovered.filter(|&index| index < self.items.len())
    }

    /// Reads the data again and lays the dock out for it. After a switch of
    /// workspace, the items of the new one glide in from the side it lies on
    /// (spec 0013 C3).
    fn reload(&mut self) {
        let position = |data: &Data| {
            let id = &data.workspace.as_ref()?.id;
            data.workspaces.iter().position(|w| &w.id == id)
        };
        let before = position(&self.data);
        self.data = read_data(&self.app, &self.monitor.key);
        let enter = match (before, position(&self.data)) {
            (Some(before), Some(after)) if after > before => SWITCH_SLIDE,
            (Some(before), Some(after)) if after < before => -SWITCH_SLIDE,
            _ => 0.0,
        };
        self.rebuild_entering(enter);
    }

    /// Lays the dock out for the data it has: the items, their names, the
    /// window, and paints.
    fn rebuild(&mut self) {
        self.rebuild_entering(0.0);
    }

    /// `rebuild`, with the items that were not on the dock coming from
    /// `enter` logical pixels along the bar.
    fn rebuild_entering(&mut self, enter: f64) {
        // Where each item is drawn now, on the screen, by key: from there it
        // slides to its new place.
        let drawn: HashMap<String, f64> = self.drawn_x().collect();
        self.rebuild_layout();
        let rest = self.rest();
        let left = f64::from(self.window.x) / self.scale;
        let (width, _) = self.viewport();
        self.slide = self
            .items
            .iter()
            .zip(&rest.items)
            .map(|(item, area)| {
                let Some(before) = drawn.get(&item.key()) else {
                    return if self.motion { enter } else { 0.0 };
                };
                let offset = before - (left + area.x);
                // Another monitor, or no motion: straight there.
                if !self.motion || offset.abs() > width {
                    0.0
                } else {
                    offset
                }
            })
            .collect();
        self.landing = None;
        self.arm_auto_hide();
        arm_hotkeys(self.data.workspaces.len());
        self.animate();
        self.paint();
    }

    /// Each item's x as drawn now, on the screen in logical pixels, by key.
    fn drawn_x(&self) -> impl Iterator<Item = (String, f64)> + '_ {
        let rest = self.rest();
        let left = f64::from(self.window.x) / self.scale;
        let shown = self.shown_offsets(&rest);
        self.items
            .iter()
            .zip(rest.items)
            .zip(shown)
            .map(move |((item, area), offset)| (item.key(), left + area.x + offset))
    }

    /// The items, their names and the window for the data the dock has.
    fn rebuild_layout(&mut self) {
        let settings = &self.data.settings;
        let found = dock_items::items(dock_items::Sources {
            widgets: self.data.widgets.as_ref(),
            apps: self.data.apps.as_ref(),
            open: &self.data.open,
            show_open: settings.dock.show_open_apps,
            workspace: self.data.workspace.as_ref().map(|w| w.items.as_slice()),
        });
        self.strip = strip::cells(
            settings.dock.taskbar,
            self.system.as_ref(),
            &self.data.tray,
            &settings.dock.tray_in_dock,
        );
        if self
            .strip_hovered
            .is_some_and(|index| index >= self.strip.len())
        {
            self.strip_hovered = None;
        }
        self.decode_tray_images();
        // The growth of each item follows it by key across changes.
        let before: HashMap<String, Magnification> = self
            .items
            .iter()
            .map(Item::key)
            .zip(self.growth.iter().copied())
            .collect();
        let hovered_key = self
            .hovered
            .and_then(|index| self.items.get(index))
            .map(Item::key);
        let names = names();
        self.names = found.items.iter().map(|item| names.of(item)).collect();
        self.items = found.items;
        self.arm_activity();
        self.groups = found.groups;
        self.growth = self
            .items
            .iter()
            .map(|item| {
                before.get(&item.key()).copied().unwrap_or(Magnification {
                    scale: 1.0,
                    lift: 0.0,
                })
            })
            .collect();
        self.hovered =
            hovered_key.and_then(|key| self.items.iter().position(|item| item.key() == key));
        let keys: HashSet<String> = self.items.iter().map(Item::key).collect();
        self.opening.retain(|key, _| keys.contains(key));
        if self
            .press
            .is_some_and(|press| press.index >= self.items.len())
        {
            self.press = None;
        }
        self.label_widths = self
            .names
            .clone()
            .iter()
            .map(|name| self.painter.label_width(name))
            .collect();
        self.notices = self.read_notices();
        self.request_pictures();
        self.place();
    }

    /// The notification badge of each item: a program's, from its button of
    /// the taskbar (spec 0016). None while the setting is off.
    fn read_notices(&self) -> Vec<Option<Badge>> {
        let buttons = if self.data.settings.dock.show_badges {
            super::badges::buttons()
        } else {
            Vec::new()
        };
        self.items
            .iter()
            .map(|item| {
                // A Ciclo's notice not seen yet (F2b-3): whatever the
                // setting, it is the dock's own.
                if let Item::Widget(widget) = item {
                    match widget.config {
                        WidgetConfig::Cycle(_) => {
                            return super::calendar::pending(widget.id.as_str())
                                .then_some(Badge::Dot);
                        }
                        // A stage of the Pomodoro ended (F2c-2).
                        WidgetConfig::Pomodoro(_) => {
                            return super::pomodoro::pending(widget.id.as_str())
                                .then_some(Badge::Dot);
                        }
                        // Time for a cup (F3-2).
                        WidgetConfig::Water(_) => {
                            return super::water::pending(widget.id.as_str()).then_some(Badge::Dot);
                        }
                        _ => {}
                    }
                }
                if buttons.is_empty() {
                    return None;
                }
                match item {
                    Item::App(app) => badges::for_pinned(
                        &buttons,
                        super::open_apps::known_identity(&app.app_id).as_ref(),
                        &app.name,
                    ),
                    Item::Other(other) => badges::for_other(
                        &buttons,
                        super::open_apps::group_key_of_other(&other.key).as_deref(),
                        &other.name,
                    ),
                    _ => None,
                }
            })
            .collect()
    }

    /// The taskbar's badges changed: drawn again if this dock's did.
    fn notices_changed(&mut self) {
        let notices = self.read_notices();
        if notices != self.notices {
            self.notices = notices;
            self.paint();
        }
    }

    /// The images of the tray icons kept in the dock, read again when one
    /// changed.
    fn decode_tray_images(&mut self) {
        use base64::Engine as _;
        for cell in &self.strip {
            let Cell::Tray(icon) = cell else {
                continue;
            };
            let Some(image) = &icon.image else {
                self.tray_images.remove(&icon.key);
                continue;
            };
            if self
                .tray_images
                .get(&icon.key)
                .is_some_and(|(from, _)| from == image)
            {
                continue;
            }
            let bitmap = image
                .strip_prefix("data:image/png;base64,")
                .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok())
                .and_then(|png| self.painter.bitmap(&png));
            self.tray_images
                .insert(icon.key.clone(), (image.clone(), bitmap));
        }
    }

    /// The battery, the network and the volume, read again; laid out again
    /// when they changed (an icon may come or go).
    /// Reads the system for its widgets every [`ACTIVITY_EVERY`] while one
    /// is on the dock, and stops when none is: then nothing is read.
    fn arm_activity(&mut self) {
        let calendar = self.items.iter().any(|item| {
            matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Calendar(_)))
        });
        let cycles: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget(widget) => match &widget.config {
                    WidgetConfig::Cycle(config) => {
                        Some((widget.id.as_str().to_owned(), config.clone()))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        super::calendar::want(self.hwnd.0 as isize, calendar, cycles);
        let media = self.items.iter().any(|item| {
            matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Media(_)))
        });
        super::media::want(self.hwnd.0 as isize, media);
        let wanted = self.items.iter().any(|item| {
            matches!(
                item,
                Item::Widget(widget) if matches!(
                    widget.config,
                    WidgetConfig::Activity(_) | WidgetConfig::Network(_)
                )
            )
        });
        match (wanted, self.activity.is_some()) {
            (true, false) => {
                self.activity = Some(super::system_activity::sample());
                unsafe { SetTimer(Some(self.hwnd), ACTIVITY_TIMER, ACTIVITY_EVERY, None) };
            }
            (false, true) => {
                self.activity = None;
                let _ = unsafe { KillTimer(Some(self.hwnd), ACTIVITY_TIMER) };
            }
            _ => {}
        }
    }

    fn read_activity(&mut self) {
        if self.activity.is_none() {
            return;
        }
        let reading = super::system_activity::sample();
        if self.activity.as_ref() != Some(&reading) {
            self.activity = Some(reading);
            self.paint();
        }
    }

    fn read_system(&mut self) {
        let now = Some(super::system_status::read());
        if now != self.system {
            self.system = now;
            self.rebuild();
        }
    }

    /// The cells of the strip, where they are drawn now.
    fn strip_cells(&self) -> Vec<Area> {
        let icon = self.shape().size.geometry().icon;
        self.placement_now()
            .tray
            .map(|area| dock_layout::tray_cells(area, self.strip.len(), icon))
            .unwrap_or_default()
    }

    /// `tray_drop_index`, on the dock's thread.
    fn tray_drop_index(&self, x: i32, y: i32) -> Option<Option<String>> {
        if !self.visible {
            return None;
        }
        let x = f64::from(x - self.window.x) / self.scale;
        let y = f64::from(y - self.window.y) / self.scale;
        let placement = self.placement_now();
        // The bar, with a little room around it: a drop is not aimed to the
        // pixel.
        let bar = placement.bar;
        let near = x >= bar.x - 8.0
            && x < bar.x + bar.width + 8.0
            && y >= bar.y - 8.0
            && y < bar.y + bar.height + 8.0;
        if !near {
            return None;
        }
        // From the cell under the pointer on (before network, volume and
        // battery, after the arrow), the first tray icon kept in the dock.
        let from = self.strip_hit(x, y).unwrap_or(self.strip.len());
        Some(self.strip[from..].iter().find_map(|cell| match cell {
            Cell::Tray(icon) => icon.id.clone(),
            _ => None,
        }))
    }

    /// The cell of the strip under the point.
    fn strip_hit(&self, x: f64, y: f64) -> Option<usize> {
        self.strip_cells()
            .iter()
            .position(|cell| cell.contains(x, y))
    }

    /// A click on a cell of the strip: the arrow opens the tray panel, the
    /// battery, the network and the volume the quick settings of Windows, a
    /// tray icon its program's own action.
    fn strip_click(&mut self, index: usize, right: bool) {
        let Some(cell) = self.strip.get(index).cloned() else {
            return;
        };
        match cell {
            Cell::Arrow if !right => {
                let Some(area) = self.strip_cells().get(index).copied() else {
                    return;
                };
                self.strip_hovered = None;
                self.toggle(PanelId::Tray, area);
            }
            Cell::Tray(icon) => {
                let button = if right {
                    super::tray_host::Button::Right
                } else {
                    super::tray_host::Button::Left
                };
                // Its cell on the screen: the program opens its menu there.
                let place = self.strip_cells().get(index).map(|area| {
                    (
                        self.window.x + (area.x * self.scale).round() as i32,
                        self.window.y + (area.y * self.scale).round() as i32,
                        (area.width * self.scale).round() as i32,
                        (area.height * self.scale).round() as i32,
                    )
                });
                self.effects.push(Box::new(move |_| {
                    if let Err(error) = super::tray_host::click_at(&icon.key, button, place) {
                        log::warn!("dock: a tray icon refused the click: {error}");
                    }
                }));
            }
            Cell::Network(_) | Cell::Volume(_) | Cell::Battery(_) if !right => {
                self.effects.push(Box::new(|_| {
                    if let Err(error) = super::flyout::open(super::flyout::Flyout::QuickSettings) {
                        log::warn!("dock: the quick settings could not be opened: {error}");
                    }
                }));
            }
            _ => self.dock_menu(None),
        }
    }

    /// Asks for the icons and badges not read yet, off the main thread.
    fn request_pictures(&mut self) {
        let pack = self.data.settings.dock.icon_pack.clone();
        let hwnd = self.hwnd.0 as isize;
        for item in &self.items {
            let app_id = match item {
                Item::App(app) => app.app_id.clone(),
                Item::Other(other) => other.app_id.clone(),
                _ => continue,
            };
            let key = (pack.clone(), app_id.clone());
            if !self.icons.contains_key(&key) {
                self.icons.insert(key.clone(), Loaded::Loading);
                let app = self.app.clone();
                std::thread::spawn(move || {
                    let png = crate::commands::apps::dock_icon_png(&app, &app_id);
                    post_to(hwnd, move |dock| {
                        let bitmap = png.and_then(|png| dock.painter.bitmap(&png));
                        dock.icons.insert(key, Loaded::Ready(bitmap));
                        dock.paint();
                    });
                });
            }
            if let Item::Other(other) = item
                && other.badge
                && !self.badges.contains_key(&other.key)
            {
                self.badges.insert(other.key.clone(), Loaded::Loading);
                let key = other.key.clone();
                std::thread::spawn(move || {
                    let png = super::open_apps::aumid_of_other(&key)
                        .and_then(|aumid| super::profiles::badge_png(&aumid));
                    post_to(hwnd, move |dock| {
                        let bitmap = png.and_then(|png| dock.painter.bitmap(&png));
                        dock.badges.insert(key, Loaded::Ready(bitmap));
                        dock.paint();
                    });
                });
            }
        }
    }

    /// Puts the window where the settings say, the size its content needs.
    fn place(&mut self) {
        let shape = self.shape();
        let size = dock_layout::window_size(&shape, self.motion, &self.label_widths);
        let bar_height = {
            let geometry = shape.size.geometry();
            geometry.icon + 2.0 * geometry.pad
        };
        let Some((window, scale)) = super::dock::place_native(
            &self.app,
            &self.data.settings.dock,
            &self.monitor,
            size,
            bar_height,
        ) else {
            return;
        };
        if (scale - self.scale).abs() > f64::EPSILON {
            self.painter.set_scale(scale);
        }
        self.window = window;
        self.scale = scale;
        self.publish();
    }

    /// Tells the other threads where the dock is.
    fn publish(&self) {
        let rest = dock_layout::place(&self.shape(), self.edge(), self.viewport(), None);
        let to_screen = |area: Area| Rect {
            x: self.window.x + (area.x * self.scale).round() as i32,
            y: self.window.y + (area.y * self.scale).round() as i32,
            width: (area.width * self.scale).round().max(0.0) as u32,
            height: (area.height * self.scale).round().max(0.0) as u32,
        };
        let hwnd = self.hwnd.0 as isize;
        let placed = Placed {
            hwnd,
            monitor: self.monitor.key.clone(),
            handle: self.monitor.handle,
            rect: self.monitor.rect,
            work: self.monitor.work,
            visible: self.visible,
            window: self.window,
            scale: self.scale,
            bar: to_screen(rest.bar),
        };
        {
            let mut shared = shared();
            match shared.docks.iter_mut().find(|known| known.hwnd == hwnd) {
                Some(known) => *known = placed,
                None => shared.docks.push(placed),
            }
            // The primary monitor's first, as the docks are.
            let primary = shared
                .docks
                .iter()
                .position(|known| known.monitor == self.monitor.key && self.monitor.primary);
            if let Some(primary) = primary {
                let first = shared.docks.remove(primary);
                shared.docks.insert(0, first);
            }
        }
        // For assistive technology: each item's name (with the notice on
        // Configurações) and its box on the screen.
        let attention = dock_items::needs_attention(self.data.widgets.as_ref());
        let notice = crate::i18n::messages().dock.with_notice;
        // The workspace dots first, as on the screen (spec 0013 C3).
        let text = &crate::i18n::messages().dock;
        let shown = self.data.workspace.as_ref().map(|w| w.id.as_str());
        let side = 2.0 * dock_layout::ACTIVE_DOT_RADIUS;
        let dots: Vec<access::Entry> = rest
            .switcher
            .map(|area| dock_layout::switcher_dots(area, self.data.workspaces.len()))
            .unwrap_or_default()
            .into_iter()
            .zip(&self.data.workspaces)
            .map(|((x, y), workspace)| {
                let on_screen = to_screen(Area {
                    x: x - side / 2.0,
                    y: y - side / 2.0,
                    width: side,
                    height: side,
                });
                let name = if Some(workspace.id.as_str()) == shown {
                    text.workspace_shown
                } else {
                    text.workspace
                };
                access::Entry {
                    key: format!("{WORKSPACE_KEY}{}", workspace.id),
                    name: name.replace("{name}", &workspace.name),
                    rect: (
                        f64::from(on_screen.x),
                        f64::from(on_screen.y),
                        f64::from(on_screen.width),
                        f64::from(on_screen.height),
                    ),
                }
            })
            .collect();
        access::publish(
            hwnd,
            dots.into_iter()
                .chain(
                    self.items
                        .iter()
                        .zip(&self.names)
                        .zip(&rest.items)
                        .map(|((item, name), area)| {
                            let on_screen = to_screen(*area);
                            access::Entry {
                                key: item.key(),
                                name: if attention
                                    && matches!(item, Item::System(SystemItem::Settings))
                                {
                                    notice.replace("{name}", name)
                                } else {
                                    name.clone()
                                },
                                rect: (
                                    f64::from(on_screen.x),
                                    f64::from(on_screen.y),
                                    f64::from(on_screen.width),
                                    f64::from(on_screen.height),
                                ),
                            }
                        })
                        .chain(
                            self.strip
                                .iter()
                                .zip(self.strip_cells())
                                .map(|(cell, area)| {
                                    let on_screen = to_screen(area);
                                    access::Entry {
                                        key: cell.key(),
                                        name: cell.tip(),
                                        rect: (
                                            f64::from(on_screen.x),
                                            f64::from(on_screen.y),
                                            f64::from(on_screen.width),
                                            f64::from(on_screen.height),
                                        ),
                                    }
                                }),
                        ),
                )
                .collect(),
        );
    }

    /// Presses the item `key` (assistive technology), as a click would.
    fn press(&mut self, key: &str) {
        if let Some(id) = key.strip_prefix(WORKSPACE_KEY) {
            if let Some(index) = self.data.workspaces.iter().position(|w| w.id == id) {
                self.switch_to(index);
            }
        } else if let Some(index) = self.items.iter().position(|item| item.key() == key) {
            self.click(index);
            self.paint();
        } else if let Some(index) = self.strip.iter().position(|cell| cell.key() == key) {
            self.strip_click(index, false);
            self.paint();
        }
    }

    /// The dock's monitor is gone (or the user keeps only one dock).
    fn destroy(&mut self) {
        self.end_preview();
        self.hide();
        let hwnd = self.hwnd.0 as isize;
        shared().docks.retain(|known| known.hwnd != hwnd);
        access::forget(hwnd);
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }

    fn show(&mut self) {
        self.place();
        self.visible = true;
        self.paint();
        self.show_window();
        self.publish();
    }

    /// Puts the window on the screen, above the others, without the focus.
    fn show_window(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    /// Starts or stops looking at the pointer, as the setting says; with it
    /// off, the dock comes back in place.
    fn arm_auto_hide(&mut self) {
        if self.data.settings.dock.auto_hide {
            unsafe { SetTimer(Some(self.hwnd), AUTO_HIDE_TIMER, AUTO_HIDE_EVERY, None) };
        } else {
            let _ = unsafe { KillTimer(Some(self.hwnd), AUTO_HIDE_TIMER) };
            self.untuck();
        }
    }

    /// Brings the dock up; hiding on its own, it goes down again once the
    /// pointer is away.
    fn untuck(&mut self) {
        self.on_edge_since = None;
        self.away_since = None;
        if self.tucking || self.tucked > 0.0 {
            self.tucking = false;
            if self.visible {
                self.show_window();
            }
            self.animate();
        }
    }

    /// One look at the pointer while the dock hides on its own: down once
    /// the pointer is away for a moment, unless something of the dock is in
    /// use; up once it rests on the screen edge.
    fn watch_pointer(&mut self) {
        if !self.data.settings.dock.auto_hide || !self.visible {
            return;
        }
        let mut at = POINT::default();
        if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) }.is_err() {
            return;
        }
        let point = (at.x, at.y);
        let now = Instant::now();
        let edge = self.edge();
        let dock = &self.data.settings.dock;
        let reveal_after = Duration::from_millis(u64::from(dock.auto_hide_reveal));
        let tuck_after = Duration::from_millis(u64::from(dock.auto_hide_delay));
        if self.tucking {
            if at_screen_edge(point, self.monitor.rect, edge) {
                let since = *self.on_edge_since.get_or_insert(now);
                if now.duration_since(since) >= reveal_after {
                    self.untuck();
                }
            } else {
                self.on_edge_since = None;
            }
            return;
        }
        let busy = self.open.is_some()
            || self.press.is_some()
            || self.landing.is_some()
            || self.menu_open
            || self.previewing()
            || menu_on_screen();
        if busy || near_dock(point, self.window, self.monitor.rect, edge) {
            self.away_since = None;
            return;
        }
        let since = *self.away_since.get_or_insert(now);
        if now.duration_since(since) >= tuck_after {
            self.away_since = None;
            self.end_preview();
            self.tucking = true;
            self.hovered = None;
            self.status_hovered = false;
            self.strip_hovered = None;
            self.animate();
        }
    }

    fn hide(&mut self) {
        self.visible = false;
        self.hovered = None;
        self.press = None;
        self.pressed = None;
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        self.publish();
    }

    /// The item under the point (logical, in the window), as drawn now: the
    /// one whose column holds it, inside the bar or on a grown tile.
    fn hit(&self, x: f64, y: f64) -> Option<usize> {
        let placement = self.placement_now();
        let gap = self.shape().size.geometry().gap;
        let on_tile = placement.tiles.iter().any(|tile| tile.contains(x, y));
        if !placement.bar.contains(x, y) && !on_tile {
            return None;
        }
        placement
            .items
            .iter()
            .position(|item| x >= item.x - gap / 2.0 && x < item.x + item.width + gap / 2.0)
    }

    fn placement_now(&self) -> dock_layout::Placement {
        let growth = self.growth.clone();
        dock_layout::place_with(&self.shape(), self.edge(), self.viewport(), &move |index| {
            growth.get(index).copied().unwrap_or(Magnification {
                scale: 1.0,
                lift: 0.0,
            })
        })
    }

    fn rest(&self) -> dock_layout::Placement {
        dock_layout::place(&self.shape(), self.edge(), self.viewport(), None)
    }

    /// The drag drawn: the one going on, or the one let go while its item
    /// waits for its new place; with the pointer's x.
    fn shown_drag(&self) -> Option<(Press, f64)> {
        self.press
            .filter(|press| press.dragging)
            .map(|press| (press, self.drag_x))
            .or(self.landing.map(|landing| (landing.press, landing.x)))
    }

    /// How far the dragged item is from its place at rest: with the pointer,
    /// inside its group.
    fn drag_shift(press: Press, x: f64, rest: &dock_layout::Placement) -> f64 {
        let first = rest.items[press.group.0];
        let last = rest.items[press.group.0 + press.group.1 - 1];
        let item = rest.items[press.index];
        drag_offset(
            x,
            press.start_x,
            item.x,
            item.width,
            first.x,
            last.x + last.width,
        )
    }

    /// Where each item is headed, from its place at rest: the dragged item
    /// with the pointer, the others making room where it would land.
    fn slide_targets(&self, rest: &dock_layout::Placement) -> Vec<f64> {
        let mut targets = vec![0.0; self.items.len()];
        let Some((press, x)) = self
            .shown_drag()
            .filter(|(press, _)| press.range().end <= rest.items.len())
        else {
            return targets;
        };
        let group = &rest.items[press.range()];
        let centers: Vec<f64> = group.iter().map(Area::center_x).collect();
        let target = drop_index(&centers, press.local(), x);
        let room = make_room(
            press.group.1,
            press.local(),
            target,
            drag_pitch(group, press.local()),
        );
        for (local, offset) in room.into_iter().enumerate() {
            targets[press.group.0 + local] = offset;
        }
        targets[press.index] = Self::drag_shift(press, x, rest);
        targets
    }

    /// How far each item is drawn from its place at rest now: the dragged
    /// one right with the pointer, the others where their slide got to.
    fn shown_offsets(&self, rest: &dock_layout::Placement) -> Vec<f64> {
        let mut offsets: Vec<f64> = (0..self.items.len())
            .map(|index| self.slide.get(index).copied().unwrap_or(0.0))
            .collect();
        if let Some((press, x)) = self
            .shown_drag()
            .filter(|(press, _)| press.range().end <= rest.items.len())
        {
            offsets[press.index] = Self::drag_shift(press, x, rest);
        }
        offsets
    }

    /// Starts the frames, if anything moves.
    fn animate(&mut self) {
        if !self.animating {
            self.animating = true;
            unsafe {
                SetTimer(
                    Some(self.hwnd),
                    ANIMATION_TIMER,
                    FRAME.as_millis() as u32,
                    None,
                )
            };
        }
    }

    /// One frame: each item moves a part of the way to its growth. Stops the
    /// frames once nothing moves.
    fn step(&mut self) {
        let shape = self.shape();
        let magnified = self.magnified();
        let mut moving = false;
        // A Calendário two icons wide rises above the bar under the
        // pointer, as wide as three icons, to show the meeting after the
        // next one (spec 0017 F2a); its neighbours stay where they are.
        let opened = self.opened_calendar();
        self.widen.resize(self.items.len(), 0.0);
        for (index, now) in self.widen.iter_mut().enumerate() {
            let target = if opened == Some(index) { 1.0 } else { 0.0 };
            if !self.motion || (*now - target).abs() < 0.01 {
                *now = target;
            } else {
                *now += (target - *now) * 0.3;
                moving = true;
            }
        }
        for (index, now) in self.growth.iter_mut().enumerate() {
            let target = shape.hover.of_item(index, magnified);
            if !self.motion {
                *now = target;
                continue;
            }
            let next = |from: f64, to: f64| from + (to - from) * 0.35;
            let scale = next(now.scale, target.scale);
            let lift = next(now.lift, target.lift);
            if (scale - target.scale).abs() < 0.002 && (lift - target.lift).abs() < 0.05 {
                *now = target;
            } else {
                *now = Magnification { scale, lift };
                moving = true;
            }
        }
        if self
            .landing
            .is_some_and(|landing| landing.since.elapsed() >= LANDING)
        {
            // The new order never came: back to where it was.
            self.landing = None;
        }
        let rest = self.rest();
        let targets = self.slide_targets(&rest);
        let held = self.shown_drag().map(|(press, _)| press.index);
        self.slide.resize(self.items.len(), 0.0);
        for (index, (now, target)) in self.slide.iter_mut().zip(targets).enumerate() {
            // The dragged item is with the pointer, not behind it.
            *now = if !self.motion || held == Some(index) {
                target
            } else {
                slide_step(*now, target)
            };
            if *now != target {
                moving = true;
            }
        }
        let out = if self.tucking { 1.0 } else { 0.0 };
        if self.tucked != out {
            self.tucked = if self.motion {
                tuck_step(self.tucked, out)
            } else {
                out
            };
            if self.tucked == 1.0 {
                // All out: off the screen, and off any monitor beyond it.
                let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            } else if self.tucked != out {
                moving = true;
            }
        }
        let now = Instant::now();
        self.opening
            .retain(|_, started| now.saturating_duration_since(*started) < OPENING);
        let dragging = self.press.is_some_and(|press| press.dragging) || self.landing.is_some();
        if !moving && self.opening.is_empty() && !dragging {
            self.animating = false;
            let _ = unsafe { KillTimer(Some(self.hwnd), ANIMATION_TIMER) };
        }
        self.paint();
    }

    /// Repaints at the next minute, for the clock and the live tiles.
    fn arm_clock(&self) {
        let now = local_now();
        let left = 60_000 - (u32::from(now.wSecond) * 1000 + u32::from(now.wMilliseconds));
        unsafe { SetTimer(Some(self.hwnd), CLOCK_TIMER, left + 50, None) };
    }

    fn paint(&mut self) {
        if !self.visible {
            return;
        }
        let scene = self.scene();
        // Hiding on its own: drawn that far past the screen edge.
        let away = (self.tucked * f64::from(self.window.height)).round() as i32;
        let origin = POINT {
            x: self.window.x,
            y: match self.edge() {
                DockEdge::Bottom => self.window.y + away,
                DockEdge::Top => self.window.y - away,
            },
        };
        if let Err(error) = self.painter.present(
            self.hwnd,
            origin,
            self.window.width as i32,
            self.window.height as i32,
            &scene,
        ) {
            log::warn!("dock: could not paint: {error}");
        }
        // A long title rolls: painted again when it moves next.
        match self.painter.marquee {
            Some(next) => {
                let ms = (next * 1000.0).ceil().clamp(16.0, 60_000.0) as u32;
                unsafe { SetTimer(Some(self.hwnd), MARQUEE_TIMER, ms, None) };
            }
            None => {
                let _ = unsafe { KillTimer(Some(self.hwnd), MARQUEE_TIMER) };
            }
        }
    }

    /// Whether the dock is drawn dark: the setting, or Windows' own.
    fn dark_now(&self) -> bool {
        match self.data.settings.theme {
            Theme::System => self.dark,
            Theme::Light => false,
            Theme::Dark => true,
        }
    }

    fn palette(&self) -> Palette {
        Palette::of(self.dark_now(), self.data.settings.dock.opacity)
    }

    fn scene(&mut self) -> Scene {
        let settings = self.data.settings.dock.clone();
        let palette = self.palette();
        let shape = self.shape();
        let geometry = shape.size.geometry();
        let edge = self.edge();
        let now_placement = self.placement_now();
        let target = dock_layout::place(&shape, edge, self.viewport(), self.magnified());
        let rest = self.rest();
        let look = look::Look::of(&settings);
        let attention = dock_items::needs_attention(self.data.widgets.as_ref());
        let clock = local_now();
        let drag = self.press.filter(|press| press.dragging);
        let held = self.shown_drag().map(|(press, _)| press.index);
        let offsets = self.shown_offsets(&rest);
        let mut tiles = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let mut area = now_placement.tiles[index];
            let mut box_area = now_placement.items[index];
            area.x += offsets[index];
            box_area.x += offsets[index];
            let dragging = held == Some(index);
            if dragging {
                // Picked up: off the bar, a little bigger.
                let lift = match edge {
                    DockEdge::Bottom => -DRAG_LIFT,
                    DockEdge::Top => DRAG_LIFT,
                };
                let (width, height) = (area.width * DRAG_GROW, area.height * DRAG_GROW);
                area = Area {
                    x: area.x - (width - area.width) / 2.0,
                    y: area.y + lift - (height - area.height) / 2.0,
                    width,
                    height,
                };
            }
            let risen = self.widen.get(index).copied().unwrap_or(0.0);
            if risen > 0.0 {
                // A floor above the others: wider by an icon, and up.
                let wide = area.width * (1.0 + 0.5 * risen);
                let up = geometry.icon * 0.38 * risen;
                area = Area {
                    x: area.x - (wide - area.width) / 2.0,
                    y: match edge {
                        DockEdge::Bottom => area.y - up,
                        DockEdge::Top => area.y + up,
                    },
                    width: wide,
                    ..area
                };
            }
            if let Some(started) = self.opening.get(&item.key()) {
                let (dx, dy, grow) = opening_offset(
                    settings.open_animation,
                    started.elapsed(),
                    geometry.icon,
                    edge,
                );
                let side = area.width * grow;
                area = Area {
                    x: area.x + dx - (side - area.width) / 2.0,
                    y: area.y + dy - (side - area.height) / 2.0,
                    width: side,
                    height: side,
                };
            }
            let open_here = self.open.as_ref() == Some(&panel_of(item));
            let running = dock_items::running(&self.data.open, item);
            let dot = match (open_here, running) {
                (true, _) | (_, Some(true)) => 1.0,
                (false, Some(false)) => 0.55,
                (false, None) => 0.0,
            };
            let face = self.face(item, index, &settings, shape.size, &clock);
            tiles.push(Tile {
                area,
                face,
                pressed: self.pressed == Some(index) && !dragging,
                attention: attention && matches!(item, Item::System(SystemItem::Settings)),
                dragging,
                raised: risen > 0.0,
                dot: Some((box_area.center_x(), dot)),
                item: box_area,
                notice: self.notices.get(index).copied().flatten(),
            });
        }
        let tip = self
            .magnified()
            .or(self.hovered.filter(|_| self.open.is_none()))
            .filter(|_| drag.is_none())
            // The risen Calendário says it all itself.
            .filter(|&index| self.widen.get(index).copied().unwrap_or(0.0) == 0.0)
            .filter(|&index| {
                self.items
                    .get(index)
                    .is_some_and(|item| self.open.as_ref() != Some(&panel_of(item)))
            })
            .and_then(|index| {
                let tile = target.tiles.get(index)?;
                Some(Tip {
                    anchor: dock_layout::tip_anchor(*tile, edge),
                    text: self.names.get(index)?.clone(),
                })
            });
        let cells = self.strip_cells();
        let strip: Vec<StripCell> = self
            .strip
            .iter()
            .zip(&cells)
            .enumerate()
            .map(|(index, (cell, area))| StripCell {
                area: *area,
                glyph: cell.glyph(edge),
                image: match cell {
                    Cell::Tray(icon) => self
                        .tray_images
                        .get(&icon.key)
                        .and_then(|(_, bitmap)| bitmap.clone()),
                    _ => None,
                },
                hovered: self.strip_hovered == Some(index),
            })
            .collect();
        // Over a cell of the strip, its label.
        let tip = tip.or_else(|| {
            let index = self.strip_hovered.filter(|_| self.open.is_none())?;
            Some(Tip {
                anchor: dock_layout::tip_anchor(*cells.get(index)?, edge),
                text: self.strip.get(index)?.tip(),
            })
        });
        // The workspace dots, and the name of the one under the pointer.
        let dot_centers = now_placement
            .switcher
            .map(|area| dock_layout::switcher_dots(area, self.data.workspaces.len()))
            .unwrap_or_default();
        let shown = self.data.workspace.as_ref().map(|w| w.id.as_str());
        let dots: Vec<Dot> = dot_centers
            .iter()
            .zip(&self.data.workspaces)
            .enumerate()
            .map(|(index, (&center, workspace))| Dot {
                center,
                color: Color::hex(&workspace.color).unwrap_or(palette.accent),
                active: Some(workspace.id.as_str()) == shown,
                hovered: self.switcher_hovered == Some(index),
            })
            .collect();
        let tip = tip.or_else(|| {
            let index = self.switcher_hovered.filter(|_| self.open.is_none())?;
            let (x, y) = *dot_centers.get(index)?;
            let side = 2.0 * dock_layout::ACTIVE_DOT_RADIUS;
            Some(Tip {
                anchor: dock_layout::tip_anchor(
                    Area {
                        x: x - side / 2.0,
                        y: y - side / 2.0,
                        width: side,
                        height: side,
                    },
                    edge,
                ),
                text: self.data.workspaces.get(index)?.name.clone(),
            })
        });
        let status = now_placement.status.map(|area| Status {
            area,
            time: format!("{:02}:{:02}", clock.wHour, clock.wMinute),
            date: format!("{:02}/{:02}/{:04}", clock.wDay, clock.wMonth, clock.wYear),
            hovered: self.status_hovered,
        });
        let workspace_color = settings
            .indicator_follows_workspace
            .then_some(self.data.workspace.as_ref())
            .flatten()
            .and_then(|workspace| Color::hex(&workspace.color));
        let indicator_color = workspace_color
            .or_else(|| settings.indicator_color.as_deref().and_then(Color::hex))
            .or(look.indicator)
            .unwrap_or(palette.accent);
        Scene {
            palette,
            look,
            edge,
            pad: geometry.pad,
            icon: geometry.icon,
            bar: now_placement.bar,
            separators: now_placement.separators.clone(),
            dots,
            tiles,
            status,
            strip,
            tip,
            indicator: settings.indicator,
            indicator_color,
        }
    }

    fn face(
        &self,
        item: &Item,
        index: usize,
        settings: &crate::domain::settings::DockSettings,
        size: DockSize,
        clock: &windows::Win32::Foundation::SYSTEMTIME,
    ) -> Face {
        let pack = settings.icon_pack.clone();
        let program = |app_id: &str, badge: Option<&str>| {
            let icon = match self.icons.get(&(pack.clone(), app_id.to_string())) {
                Some(Loaded::Ready(Some(icon))) => Some(icon.clone()),
                _ => None,
            };
            let badge = badge.and_then(|key| match self.badges.get(key) {
                Some(Loaded::Ready(Some(badge))) => Some(badge.clone()),
                _ => None,
            });
            let letter = self
                .names
                .get(index)
                .and_then(|name| name.trim().chars().next())
                .map(|c| c.to_uppercase().collect::<String>())
                .unwrap_or_else(|| "?".into());
            Face::App {
                icon,
                letter,
                shape: settings.icon_shape,
                badge,
            }
        };
        match item {
            Item::App(app) => program(&app.app_id, None),
            Item::Other(other) => program(&other.app_id, other.badge.then_some(other.key.as_str())),
            Item::Widget(widget) if matches!(widget.config, WidgetConfig::Media(_)) => {
                self.media_face(widget.config.width(), size)
            }
            Item::Widget(widget) => {
                let calendar = matches!(widget.config, WidgetConfig::Calendar(_))
                    .then(super::calendar::snapshot)
                    .flatten();
                faces::widget(
                    widget,
                    size,
                    clock,
                    self.activity.as_ref(),
                    calendar.as_ref(),
                    self.widen.get(index).copied().unwrap_or(0.0) > 0.5,
                )
            }
            Item::System(SystemItem::Settings) => Face::Glyph {
                glaze: theme::Glaze::SETTINGS,
                paths: paint::SETTINGS_GLYPH,
            },
        }
    }

    // ---- The mouse ----

    fn point(&self, lparam: LPARAM) -> (f64, f64) {
        let x = f64::from((lparam.0 & 0xffff) as u16 as i16);
        let y = f64::from(((lparam.0 >> 16) & 0xffff) as u16 as i16);
        (x / self.scale, y / self.scale)
    }

    fn track(&mut self) {
        if self.tracking {
            return;
        }
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: self.hwnd,
            dwHoverTime: 0,
        };
        self.tracking = unsafe { TrackMouseEvent(&mut track) }.is_ok();
    }

    fn mouse_move(&mut self, x: f64, y: f64) {
        self.track();
        let program = self
            .press
            .is_some_and(|press| self.program_id(press.index).is_some());
        // A program pulled straight off the dock is dragged too.
        let off = program && pointer_off(self.hwnd);
        if let Some(press) = self.press.as_mut() {
            if !press.dragging && ((x - press.start_x).abs() >= DRAG_THRESHOLD || off) {
                press.dragging = true;
                self.hovered = None;
                self.pressed = None;
            }
            if press.dragging {
                let index = press.index;
                let out = off;
                press.out = out;
                self.drag_x = if out { press.start_x } else { x };
                self.show_drag_cursor(out.then_some(index));
                self.animate();
                return;
            }
        }
        // Frozen while a panel is open.
        if self.open.is_some() {
            return;
        }
        let status = self
            .placement_now()
            .status
            .is_some_and(|area| area.contains(x, y));
        let cell = self.strip_hit(x, y);
        let dot = self.switcher_hit(x, y);
        let hovered = if status || cell.is_some() || dot.is_some() {
            None
        } else {
            self.hit(x, y)
        };
        if hovered != self.hovered
            || status != self.status_hovered
            || cell != self.strip_hovered
            || dot != self.switcher_hovered
        {
            self.hovered = hovered;
            self.status_hovered = status;
            self.strip_hovered = cell;
            self.switcher_hovered = dot;
            let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
            if self.warmable() {
                unsafe { SetTimer(Some(self.hwnd), WARM_TIMER, WARM_DELAY, None) };
            }
            self.arm_preview();
            self.animate();
            self.paint();
        }
    }

    /// Whether this dock's preview card shows.
    fn previewing(&self) -> bool {
        super::previews::shown_by() == Some(self.hwnd.0 as isize)
    }

    /// The pointer is on the Calendário: its card shows after a moment, or
    /// at once when it already shows (spec 0017 F2a-4).
    fn arm_card(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), CARD_TIMER) };
        let calendar = self.hovered.and_then(|index| self.items.get(index)).is_some_and(
            |item| matches!(item, Item::Widget(widget) if matches!(widget.config, WidgetConfig::Calendar(_))),
        );
        if !calendar {
            return;
        }
        if super::agenda::shown_by() == Some(self.hwnd.0 as isize) {
            self.show_card();
        } else {
            unsafe { SetTimer(Some(self.hwnd), CARD_TIMER, CARD_DELAY, None) };
        }
    }

    /// Shows the card of the Calendário under the pointer.
    fn show_card(&mut self) {
        let busy = self.press.is_some() || self.open.is_some() || self.menu_open || self.tucking;
        let Some(index) = self.hovered.filter(|_| !busy) else {
            return;
        };
        let Some(Item::Widget(widget)) = self.items.get(index) else {
            return;
        };
        let WidgetConfig::Calendar(config) = &widget.config else {
            return;
        };
        let Some(snapshot) = super::calendar::snapshot() else {
            return;
        };
        let Some(area) = self.placement_now().tiles.get(index).copied() else {
            return;
        };
        let now = super::calendar::now();
        let shown: Vec<_> = snapshot
            .events
            .iter()
            .filter(|event| config.shows(&event.calendar))
            .collect();
        let tasks = crate::domain::agenda::ordered(&snapshot.tasks, &config.task_order);
        let words = &crate::i18n::messages().dock;
        let rows = crate::domain::agenda::rows(
            &shown,
            &tasks,
            config.card_tasks && config.tasks,
            crate::domain::agenda::range(config.card_period, now, snapshot.offset),
            now,
            snapshot.offset,
            &crate::domain::agenda::Words {
                today: words.calendar_today,
                tomorrow: words.calendar_tomorrow_title,
                weekdays: words.weekdays,
                all_day: words.calendar_all_day,
                now: words.calendar_now,
                busy: words.calendar_busy,
                untitled: words.calendar_untitled,
                tasks: words.calendar_tasks,
                free: words.calendar_nothing,
            },
        );
        let rect = self.monitor.rect;
        super::agenda::show(super::agenda::Request {
            owner: self.hwnd.0 as isize,
            widget: widget.id.clone(),
            tile: Area {
                x: f64::from(self.window.x) + area.x * self.scale,
                y: f64::from(self.window.y) + area.y * self.scale,
                width: area.width * self.scale,
                height: area.height * self.scale,
            },
            screen: Area {
                x: f64::from(rect.x),
                y: f64::from(rect.y),
                width: f64::from(rect.width),
                height: f64::from(rect.height),
            },
            edge: self.edge(),
            scale: self.scale,
            dark: self.dark_now(),
            rows,
        });
    }

    /// Opens the Calendário `widget`'s panel, on `event`'s details when
    /// given (a click on the card).
    fn open_calendar(&mut self, widget: &crate::domain::widgets::WidgetId, event: Option<String>) {
        super::calendar::open_event(event);
        let Some(index) = self
            .items
            .iter()
            .position(|item| matches!(item, Item::Widget(each) if &each.id == widget))
        else {
            return;
        };
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        if self.open.as_ref() == Some(&panel_of(&item)) {
            // Already open: it reads the event it should show.
            self.effects.push(Box::new(|app| {
                let _ = tauri::Emitter::emit(app, super::calendar::OPEN_EVENT, ());
            }));
            return;
        }
        let anchor = self.rest().items[index];
        self.toggle(panel_of(&item), anchor);
    }

    /// The pointer is on another item: with a card already shown it changes
    /// at once (spec 0015, proposal 1), else it shows after the delay. The
    /// card hides itself once the pointer has left the icon and the card.
    fn arm_preview(&mut self) {
        self.arm_card();
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        let settings = &self.data.settings.dock;
        let program = self
            .hovered
            .and_then(|index| self.items.get(index))
            .is_some_and(Item::is_program);
        if !settings.previews || !program {
            return;
        }
        if self.previewing() || settings.preview_delay == 0 {
            self.show_preview();
        } else {
            let delay = u32::from(settings.preview_delay);
            unsafe { SetTimer(Some(self.hwnd), PREVIEW_TIMER, delay, None) };
        }
    }

    /// Shows the card of the program under the pointer: its windows in this
    /// dock's workspace.
    fn show_preview(&mut self) {
        let busy = self.press.is_some() || self.open.is_some() || self.menu_open || self.tucking;
        let Some(index) = self.hovered.filter(|_| !busy) else {
            return;
        };
        let workspace = self.workspace_id();
        let windows = match self.items.get(index) {
            Some(Item::App(app)) => {
                super::open_apps::windows_of_pinned(app.id.as_str(), workspace.as_deref())
            }
            Some(Item::Other(other)) => {
                super::open_apps::windows_of_other(&other.key, workspace.as_deref())
            }
            _ => return,
        };
        let settings = &self.data.settings.dock;
        if !crate::domain::previews::shows(windows.len(), settings.previews_only_several) {
            if self.previewing() {
                super::previews::hide();
            }
            return;
        }
        let Some(area) = self.placement_now().items.get(index).copied() else {
            return;
        };
        let rect = self.monitor.rect;
        super::previews::show(super::previews::Request {
            owner: self.hwnd.0 as isize,
            icon: Area {
                x: f64::from(self.window.x) + area.x * self.scale,
                y: f64::from(self.window.y) + area.y * self.scale,
                width: area.width * self.scale,
                height: area.height * self.scale,
            },
            screen: Area {
                x: f64::from(rect.x),
                y: f64::from(rect.y),
                width: f64::from(rect.width),
                height: f64::from(rect.height),
            },
            edge: self.edge(),
            scale: self.scale,
            dark: self.dark_now(),
            windows,
            name: self.names.get(index).cloned().unwrap_or_default(),
            motion: self.motion,
            media: None,
        });
    }

    /// Hides this dock's card, if it shows (a click, a menu, a drag, the dock
    /// going down).
    fn end_preview(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        if self.previewing() {
            super::previews::hide();
        }
        // The Calendário's card goes the same way.
        let _ = unsafe { KillTimer(Some(self.hwnd), CARD_TIMER) };
        if super::agenda::shown_by() == Some(self.hwnd.0 as isize) {
            super::agenda::hide();
        }
    }

    /// Whether the item under the pointer opens a panel that warming helps:
    /// not a program (a click opens it) nor Configurações (an app of its own).
    fn warmable(&self) -> bool {
        // The tray's arrow opens the tray panel.
        if self
            .strip_hovered
            .and_then(|index| self.strip.get(index))
            .is_some_and(|cell| *cell == Cell::Arrow)
        {
            return true;
        }
        self.hovered
            .and_then(|index| self.items.get(index))
            .is_some_and(|item| {
                !item.is_program() && !matches!(item, Item::System(SystemItem::Settings))
            })
    }

    fn mouse_leave(&mut self) {
        self.tracking = false;
        // A card already shown hides itself once the pointer is off it too.
        let _ = unsafe { KillTimer(Some(self.hwnd), PREVIEW_TIMER) };
        if self.press.is_some() {
            return;
        }
        self.hovered = None;
        self.switcher_hovered = None;
        self.status_hovered = false;
        self.strip_hovered = None;
        self.pressed = None;
        let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
        self.animate();
        self.paint();
    }

    /// The workspace this dock shows (spec 0013); `None` when they can't be
    /// read.
    fn workspace_id(&self) -> Option<String> {
        self.data.workspace.as_ref().map(|w| w.id.clone())
    }

    /// The workspace dot under the point, when the bar starts with them.
    fn switcher_hit(&self, x: f64, y: f64) -> Option<usize> {
        let area = self.placement_now().switcher?;
        dock_layout::switcher_hit(area, self.data.workspaces.len(), x, y)
    }

    /// Shows the workspace at `index` on this dock's monitor.
    fn switch_to(&mut self, index: usize) {
        let Some(id) = self.data.workspaces.get(index).map(|w| w.id.clone()) else {
            return;
        };
        if self.data.workspace.as_ref().is_some_and(|w| w.id == id) {
            return;
        }
        let monitor = self.monitor.key.clone();
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::switch(app, &monitor, &id);
        }));
    }

    /// The mouse wheel over the dock: the next workspace down, the one
    /// before up (spec 0013).
    fn wheel(&mut self, delta: i16) {
        if self.data.workspaces.len() < 2 || delta == 0 {
            return;
        }
        self.end_preview();
        let monitor = self.monitor.key.clone();
        let forward = delta < 0;
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::switch_by(app, &monitor, forward);
        }));
    }

    /// The user is on this dock: its workspace is the one in use (where a
    /// program pinned from a menu goes).
    fn use_here(&mut self) {
        let monitor = self.monitor.key.clone();
        self.effects.push(Box::new(move |app| {
            crate::commands::workspaces::use_from_dock(app, &monitor);
        }));
    }

    fn left_down(&mut self, x: f64, y: f64) {
        self.end_preview();
        self.use_here();
        shared().clicked = Some(Instant::now());
        let index = self.hit(x, y);
        self.pressed = index;
        // A program alone in its group still goes out to a folder.
        let group = |index: usize| {
            self.group_of(index)
                .or_else(|| self.program_id(index).map(|_| (index, 1)))
        };
        if let Some((index, group)) = index.and_then(|index| Some((index, group(index)?)))
            && self.open.is_none()
        {
            self.press = Some(Press {
                index,
                group,
                start_x: x,
                dragging: false,
                out: false,
            });
            self.drag_x = x;
            unsafe { SetCapture(self.hwnd) };
        }
        self.paint();
    }

    fn left_up(&mut self, x: f64, y: f64) {
        let press = self.press.take();
        let pressed = self.pressed.take();
        if press.is_some() {
            let _ = unsafe { ReleaseCapture() };
        }
        self.show_drag_cursor(None);
        if let Some(press) = press.filter(|press| press.dragging && press.out) {
            // Let go off the dock: the program opens with what is there.
            if let Some(id) = self.program_id(press.index) {
                let mut at = POINT::default();
                let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) };
                self.effects.push(Box::new(move |_| {
                    super::open_with::open_at(id, at.x, at.y);
                }));
            }
            self.animate();
            self.paint();
            return;
        }
        if let Some(press) = press.filter(|press| press.dragging) {
            if self.drop_at(press, x) {
                self.landing = Some(Landing {
                    press,
                    x,
                    since: Instant::now(),
                });
            }
            self.animate();
            self.paint();
            return;
        }
        if let Some(index) = self.switcher_hit(x, y) {
            self.switch_to(index);
            self.paint();
            return;
        }
        let status = self
            .placement_now()
            .status
            .is_some_and(|area| area.contains(x, y));
        if status {
            self.effects.push(Box::new(|_| {
                if let Err(error) = super::flyout::open(super::flyout::Flyout::Calendar) {
                    log::warn!("dock: the calendar could not be opened: {error}");
                }
            }));
        } else if let Some(cell) = self.strip_hit(x, y) {
            self.strip_click(cell, false);
        } else if let Some(index) = self.hit(x, y).filter(|&index| Some(index) == pressed) {
            match self.media_button_at(index, x, y) {
                Some(action) => super::media::control(action),
                None => self.click(index),
            }
        }
        self.paint();
    }

    /// The Tocando agora's tile (spec 0017 F4): the cover of what plays now
    /// and, wider, its title and artist.
    fn media_face(&self, width: u8, size: DockSize) -> Face {
        let now = super::media::snapshot();
        let (art, badge) = match (&now, &self.media_art) {
            (Some(now), Some((track, art, icon))) if *track == now.track() => {
                (art.clone(), icon.clone())
            }
            _ => (None, None),
        };
        let (title, artist) = match &now {
            Some(now) if width > 1 && size != DockSize::Small => {
                (Some(now.title.clone()), Some(now.artist.clone()))
            }
            _ => (None, None),
        };
        Face::Media {
            // The cover's color, as a phone's media notification has it.
            glaze: now
                .as_ref()
                .and_then(|now| now.tint)
                .map_or(theme::Glaze::MUSIC, theme::Glaze::ground_of),
            art,
            badge,
            playing: now.as_ref().is_some_and(|now| now.playing),
            title,
            artist,
            idle: now.is_none(),
            controls: width == 3 && size != DockSize::Small && now.is_some(),
            since: self.media_since.elapsed().as_secs_f64(),
        }
    }

    /// The button of the Tocando agora at `index` under `(x, y)`, three
    /// icons wide while something plays (spec 0017 F4).
    fn media_button_at(&self, index: usize, x: f64, y: f64) -> Option<super::media::Action> {
        let Some(Item::Widget(widget)) = self.items.get(index) else {
            return None;
        };
        if !matches!(widget.config, WidgetConfig::Media(_))
            || widget.config.width() != 3
            || self.shape().size == DockSize::Small
            || super::media::snapshot().is_none()
        {
            return None;
        }
        let tile = *self.placement_now().tiles.get(index)?;
        let actions = [
            super::media::Action::Previous,
            super::media::Action::Toggle,
            super::media::Action::Next,
        ];
        dock_layout::media_buttons(tile)
            .into_iter()
            .zip(actions)
            .find(|(button, _)| {
                // A little more than the glyph, easier to hit.
                let pad = button.width * 0.2;
                x >= button.x - pad
                    && x <= button.x + button.width + pad
                    && y >= button.y - pad
                    && y <= button.y + button.height + pad
            })
            .map(|(_, action)| action)
    }

    /// What plays changed: a new cover is read into a bitmap, and the tile
    /// painted again.
    fn media_changed(&mut self) {
        let now = super::media::snapshot();
        let track = now.as_ref().map(super::media::Snapshot::track);
        let kept = self.media_art.as_ref().map(|(track, _, _)| track.clone());
        if track != kept {
            self.media_since = Instant::now();
            self.media_art = now.as_ref().map(|now| {
                let art = now.art.as_ref().and_then(|art| self.painter.bitmap(&art.1));
                let icon = now.icon.as_ref().and_then(|icon| self.painter.bitmap(icon));
                (now.track(), art, icon)
            });
        }
        self.paint();
    }

    /// The Beber água's reminder (spec 0017 F3-2), looked at every minute.
    fn check_water(&self) {
        let waters: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Widget(widget) => match &widget.config {
                    WidgetConfig::Water(config) => {
                        Some((widget.id.as_str().to_owned(), config.clone()))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if !waters.is_empty() {
            super::water::check(&waters, super::calendar::now(), super::calendar::offset());
        }
    }

    /// The id in the Apps folder of the program at `index`: a pinned one or
    /// an open one. What a folder or files open with (spec 0018).
    fn program_id(&self, index: usize) -> Option<String> {
        match self.items.get(index)? {
            Item::App(app) => Some(app.app_id.clone()),
            Item::Other(other) => Some(other.app_id.clone()).filter(|id| !id.is_empty()),
            _ => None,
        }
    }

    /// The pointer shows the icon of the program at `index` while its tile
    /// is off the dock, and the arrow again with `None`.
    fn show_drag_cursor(&mut self, index: Option<usize>) {
        match index {
            Some(index) => {
                if self.drag_cursor.is_none() {
                    let side = (48.0 * self.scale).round() as i32;
                    self.drag_cursor = self
                        .program_id(index)
                        .and_then(|id| super::apps::drag_cursor(&id, side));
                }
                let cursor = self
                    .drag_cursor
                    .or_else(|| unsafe { LoadCursorW(None, IDC_ARROW) }.ok());
                unsafe { SetCursor(cursor) };
            }
            None => {
                if let Some(cursor) = self.drag_cursor.take() {
                    unsafe {
                        SetCursor(LoadCursorW(None, IDC_ARROW).ok());
                        let _ = DestroyCursor(cursor);
                    }
                }
            }
        }
    }

    /// Files dragged over the dock at the screen point `at` (physical
    /// pixels), or gone with `None`: the program under them is shown as
    /// under the pointer. Its id, when there is one.
    fn drop_over(&mut self, at: Option<(i32, i32)>) -> Option<String> {
        let index = at.and_then(|(x, y)| {
            let mut point = POINT { x, y };
            unsafe { windows::Win32::Graphics::Gdi::ScreenToClient(self.hwnd, &mut point) }
                .as_bool()
                .then(|| {
                    self.hit(
                        f64::from(point.x) / self.scale,
                        f64::from(point.y) / self.scale,
                    )
                })
                .flatten()
                .filter(|&index| self.program_id(index).is_some())
        });
        if index != self.drop_hover {
            self.drop_hover = index;
            self.hovered = index;
            self.animate();
            self.paint();
        }
        index.and_then(|index| self.program_id(index))
    }

    /// The group an item can be dragged in, as its first index and its size:
    /// the pinned programs and the widgets, mixed, or the open programs. The
    /// system items stay where they are.
    fn group_of(&self, index: usize) -> Option<(usize, usize)> {
        if matches!(self.items.get(index)?, Item::System(_)) {
            return None;
        }
        let mut start = 0;
        for &size in &self.groups {
            if index < start + size {
                // A group of one has nowhere to go.
                return (size > 1).then_some((start, size));
            }
            start += size;
        }
        None
    }

    /// A drag let go: the item goes where it was let go, in its group.
    /// Returns whether it moves (the dock then reads the new order).
    fn drop_at(&mut self, press: Press, x: f64) -> bool {
        let rest = self.rest();
        let centers: Vec<f64> = rest.items[press.range()]
            .iter()
            .map(Area::center_x)
            .collect();
        let target = drop_index(&centers, press.local(), x);
        if target == press.local() {
            return false;
        }
        let Some(item) = self.items.get(press.index).cloned() else {
            return false;
        };
        if let Item::Other(other) = item {
            self.effects.push(Box::new(move |app| {
                super::open_apps::place_other(app, &other.key, target);
            }));
            return true;
        }
        // A program or a widget, among both: the order of the workspace
        // this dock shows (spec 0013).
        let group = &self.items[press.range()];
        let Some((order, _)) = dock_items::moved(group, press.local(), target) else {
            return false;
        };
        let Some(workspace) = self.data.workspace.as_ref().map(|w| w.id.clone()) else {
            return false;
        };
        self.effects.push(Box::new(move |app| {
            {
                let state = app.state::<Mutex<Storage>>();
                let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
                if let Err(error) = storage.reorder_workspace(&workspace, &order) {
                    log::warn!("dock: the order could not be kept: {error}");
                }
            }
            crate::commands::workspaces::notify(app);
            super::dock::refresh();
        }));
        true
    }

    fn click(&mut self, index: usize) {
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        match &item {
            Item::App(app) => {
                let id = app.id.as_str().to_string();
                let key = item.key();
                let running = dock_items::running(&self.data.open, &item).is_some();
                let hwnd = self.hwnd.0 as isize;
                let workspace = self.workspace_id();
                if running {
                    self.effects.push(Box::new(move |app| {
                        if !super::open_apps::activate(app, &id, workspace.as_deref()) {
                            post_to(hwnd, move |dock| dock.launch_pinned(key, id));
                        }
                    }));
                } else {
                    self.launch_pinned(key, id);
                }
            }
            Item::Other(other) => {
                let key = other.key.clone();
                let workspace = self.workspace_id();
                self.effects.push(Box::new(move |app| {
                    super::open_apps::activate_other(app, &key, workspace.as_deref());
                }));
            }
            _ => {
                if let Item::Widget(widget) = &item
                    && (super::calendar::seen(widget.id.as_str())
                        | super::pomodoro::seen(widget.id.as_str())
                        | super::water::seen(widget.id.as_str()))
                {
                    post(Dock::notices_changed);
                }
                let anchor = self.rest().items[index];
                self.toggle(panel_of(&item), anchor);
            }
        }
    }

    /// The widgets `widgets` have a notice (a Ciclo's, F2b-3; a Pomodoro's
    /// stage ended, F2c-2): their badge, and their tile bounces as a
    /// program's does when it opens.
    fn nudged(&mut self, widgets: &[String]) {
        let now = Instant::now();
        for item in &self.items {
            if let Item::Widget(widget) = item
                && widgets.iter().any(|each| each == widget.id.as_str())
            {
                self.opening.insert(item.key(), now);
            }
        }
        self.notices = self.read_notices();
        self.animate();
    }

    fn toggle(&mut self, panel: PanelId, anchor: Area) {
        // The panel anchors to this dock.
        shared().active = self.hwnd.0 as isize;
        // The panel opens where the item rests: nothing magnified.
        self.hovered = None;
        self.animate();
        self.effects.push(Box::new(move |app| {
            if let Err(error) = crate::commands::panel::toggle(app, panel, logical(anchor)) {
                log::warn!("dock: toggling a panel failed: {error}");
            }
        }));
    }

    /// Opens the pinned program and animates its icon a moment; one that
    /// could not be opened stops at once.
    fn launch_pinned(&mut self, key: String, id: String) {
        self.opening.insert(key.clone(), Instant::now());
        self.animate();
        let hwnd = self.hwnd.0 as isize;
        self.effects.push(Box::new(move |app| {
            crate::commands::apps::launch_pinned(app, &id, move || {
                post_to(hwnd, move |dock| {
                    dock.opening.remove(&key);
                    dock.paint();
                });
            });
        }));
    }

    fn middle_click(&mut self, index: usize) {
        // The Calendário joins the meeting that starts within 10 minutes
        // (spec 0017 F2a), whatever the programs' middle click does.
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && let WidgetConfig::Calendar(config) = &widget.config
        {
            let config = config.clone();
            self.effects.push(Box::new(move |_| {
                let Some(snapshot) = super::calendar::snapshot() else {
                    return;
                };
                let shown: Vec<_> = snapshot
                    .events
                    .into_iter()
                    .filter(|event| config.shows(&event.calendar))
                    .collect();
                let now = super::calendar::now();
                if let Some(join) = crate::domain::calendar::joinable(&shown, now)
                    .and_then(|event| event.join.clone())
                    && let Err(error) = super::google::open_meeting(&join)
                {
                    log::warn!("calendar: the meeting could not be opened: {error}");
                }
            }));
            return;
        }
        // The Tocando agora plays or pauses (spec 0017 F4).
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && matches!(widget.config, WidgetConfig::Media(_))
        {
            super::media::control(super::media::Action::Toggle);
            return;
        }
        // The Pomodoro starts or pauses (spec 0017 F2c-2).
        if let Some(Item::Widget(widget)) = self.items.get(index)
            && let WidgetConfig::Pomodoro(config) = &widget.config
        {
            let (id, config) = (widget.id.as_str().to_owned(), config.clone());
            self.effects.push(Box::new(move |_| {
                use super::pomodoro::{Action, act, view};
                let action = if view(&id, &config).running {
                    Action::Pause
                } else {
                    Action::Start
                };
                act(&id, &config, action);
            }));
            return;
        }
        if self.data.settings.dock.middle_click != MiddleClick::NewInstance {
            return;
        }
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        let key = item.key();
        match item {
            Item::App(app) => self.launch_pinned(key, app.id.as_str().to_string()),
            Item::Other(other) => {
                self.opening.insert(key.clone(), Instant::now());
                self.animate();
                let hwnd = self.hwnd.0 as isize;
                self.effects.push(Box::new(move |_| {
                    crate::commands::apps::launch_other(&other.key, move || {
                        post_to(hwnd, move |dock| {
                            dock.opening.remove(&key);
                            dock.paint();
                        });
                    });
                }));
            }
            _ => {}
        }
    }

    fn right_click(&mut self, index: usize) {
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        if !item.is_program() {
            return;
        }
        let anchor = self.rest().items[index];
        self.toggle(panel_of(&item), anchor);
    }

    /// The dock's own menu, on the right button away from a program: what
    /// the tray icon's menu has (author's decision, 2026-10-26: Configurações
    /// left the dock's items; the Task Manager as in the taskbar's menu).
    /// Shown once the dock's state is let go: the menu runs a loop of its own.
    /// The dock's menu; on a widget that takes more than one width
    /// (`index`), the widths come first.
    fn dock_menu(&mut self, index: Option<usize>) {
        let widget = index.and_then(|index| match self.items.get(index) {
            Some(Item::Widget(widget)) => Some(widget.clone()),
            _ => None,
        });
        self.hovered = None;
        self.animate();
        self.paint();
        let hwnd = self.hwnd.0 as isize;
        let edge = self.edge();
        let shown = self.data.workspace.as_ref().map(|w| w.id.clone());
        let workspaces: Vec<MenuWorkspace> = self
            .data
            .workspaces
            .iter()
            .map(|w| MenuWorkspace {
                name: w.name.clone(),
                color: w.color.clone(),
                shown: Some(&w.id) == shown.as_ref(),
            })
            .collect();
        let mut items = widget
            .as_ref()
            .map(|widget| width_items(widget.config.widths(), widget.config.width()))
            .unwrap_or_default();
        items.extend(dock_menu_items(
            self.data.settings.dock.auto_hide,
            &workspaces,
            crate::commands::workspaces::can_create(&self.data.workspaces),
        ));
        let widget = widget.map(|widget| widget.id);
        let monitor = self.monitor.key.clone();
        let ids: Vec<String> = self.data.workspaces.iter().map(|w| w.id.clone()).collect();
        // Up, under its menu, until it closes.
        self.menu_open = true;
        self.effects.push(Box::new(move |app| {
            let chosen = show_dock_menu(edge, items);
            post_to(hwnd, |dock: &mut Dock| dock.menu_open = false);
            if let Some(choice) = chosen {
                act(app, choice, &monitor, &ids, widget.as_ref());
            }
        }));
    }

    // ---- What the app tells the dock ----

    fn set_open(&mut self, panel: Option<PanelId>) {
        if panel.is_some() {
            self.hovered = None;
            self.status_hovered = false;
            // A panel anchors to the dock: up while it is open.
            self.untuck();
        }
        self.open = panel;
        self.animate();
        self.paint();
    }

    /// Opens the panel of the item that has it, as a click would, unless it
    /// is open already.
    fn request(&mut self, panel: PanelId) {
        if self.open.as_ref() == Some(&panel) {
            return;
        }
        let rest = self.rest();
        let Some(index) = self.items.iter().position(|item| panel_of(item) == panel) else {
            return;
        };
        let anchor = rest.items[index];
        self.toggle(panel, anchor);
    }
}

/// Whether the pointer at `point` rests on the screen edge the dock is on,
/// on its monitor (physical pixels): what brings a dock hiding on its own
/// up.
fn at_screen_edge(point: (i32, i32), monitor: Rect, edge: DockEdge) -> bool {
    let (x, y) = point;
    let right = monitor.x.saturating_add_unsigned(monitor.width);
    let bottom = monitor.y.saturating_add_unsigned(monitor.height);
    if x < monitor.x || x >= right {
        return false;
    }
    match edge {
        DockEdge::Bottom => y >= bottom - 2 && y < bottom,
        DockEdge::Top => y >= monitor.y && y < monitor.y + 2,
    }
}

/// Whether the pointer is on the dock or between it and the screen edge:
/// where the dock stays up.
fn near_dock(point: (i32, i32), window: Rect, monitor: Rect, edge: DockEdge) -> bool {
    let (x, y) = point;
    let right = window.x.saturating_add_unsigned(window.width);
    if x < window.x || x >= right {
        return false;
    }
    match edge {
        DockEdge::Bottom => y >= window.y && y < monitor.y.saturating_add_unsigned(monitor.height),
        DockEdge::Top => y >= monitor.y && y < window.y.saturating_add_unsigned(window.height),
    }
}

/// Whether a menu is on the screen (the dock's, or one a tray icon in the
/// dock opened): the dock stays up under it.
fn menu_on_screen() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};
    // Windows' menus (a tray icon's) and the DockMax's own (spec 0014 R3).
    [w!("#32768"), menu::CLASS].into_iter().any(|class| {
        unsafe { FindWindowW(class, None) }
            .is_ok_and(|menu| unsafe { IsWindowVisible(menu) }.as_bool())
    })
}

/// One frame of the dock going out or coming in (`now` and `target` from 0,
/// in place, to 1, all out): a part of the way, and there once close.
fn tuck_step(now: f64, target: f64) -> f64 {
    let next = now + (target - now) * 0.25;
    if (target - next).abs() < 0.01 {
        target
    } else {
        next
    }
}

/// How the icon of a program that is opening is moved and grown,
/// `elapsed` into its animation: away from the screen edge for a bounce
/// (twice, 0.7 s each), a swell (three times, 0.5 s) or a shake (twice,
/// 0.5 s).
fn opening_offset(
    animation: OpenAnimation,
    elapsed: Duration,
    icon: f64,
    edge: DockEdge,
) -> (f64, f64, f64) {
    let away = match edge {
        DockEdge::Bottom => -1.0,
        DockEdge::Top => 1.0,
    };
    let seconds = elapsed.as_secs_f64();
    // Linear between the keyframes of a cycle of `period`.
    let keyframes = |period: f64, cycles: f64, frames: &[(f64, f64)]| {
        if seconds >= period * cycles {
            return 0.0;
        }
        let t = (seconds % period) / period;
        frames
            .windows(2)
            .find(|pair| t >= pair[0].0 && t <= pair[1].0)
            .map(|pair| {
                let (t0, v0) = pair[0];
                let (t1, v1) = pair[1];
                if t1 <= t0 {
                    v1
                } else {
                    v0 + (v1 - v0) * (t - t0) / (t1 - t0)
                }
            })
            .unwrap_or(0.0)
    };
    match animation {
        OpenAnimation::Bounce => {
            let lift = keyframes(
                0.7,
                2.0,
                &[(0.0, 0.0), (0.3, 0.5), (0.6, 0.0), (0.8, 0.14), (1.0, 0.0)],
            );
            (0.0, away * icon * lift, 1.0)
        }
        OpenAnimation::Pulse => {
            let grow = keyframes(0.5, 3.0, &[(0.0, 1.0), (0.5, 1.22), (1.0, 1.0)]);
            (0.0, 0.0, if grow == 0.0 { 1.0 } else { grow })
        }
        OpenAnimation::Shake => {
            let x = keyframes(
                0.5,
                2.0,
                &[
                    (0.0, 0.0),
                    (0.2, -4.0),
                    (0.4, 4.0),
                    (0.6, -4.0),
                    (0.8, 4.0),
                    (1.0, 0.0),
                ],
            );
            (x, 0.0, 1.0)
        }
    }
}

/// How many `Ctrl+Alt+<number>` shortcuts are registered (spec 0013): one
/// per workspace, and none with a single workspace, so that `AltGr+<number>`
/// (which Windows reports as `Ctrl+Alt`) still types where there is nothing to
/// switch to.
static HOTKEYS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Registers `Ctrl+Alt+1…N` for `count` workspaces on the wake window. A
/// shortcut another program already has is skipped, with a line in the log.
fn arm_hotkeys(count: usize) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    let wanted = if count >= 2 { count.min(9) } else { 0 };
    let had = HOTKEYS.swap(wanted, Ordering::SeqCst);
    if had == wanted {
        return;
    }
    let wake = HWND(WAKE.load(Ordering::SeqCst) as _);
    for number in 1..=9i32 {
        let _ = unsafe { UnregisterHotKey(Some(wake), number) };
    }
    for number in 1..=wanted as i32 {
        let key = 0x30 + number as u32;
        let registered = unsafe {
            RegisterHotKey(
                Some(wake),
                number,
                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                key,
            )
        };
        if registered.is_err() {
            log::warn!("dock: Ctrl+Alt+{number} is taken by another program");
        }
    }
}

/// `Ctrl+Alt+<number>`: the dock on the monitor the pointer is on goes to
/// that workspace.
fn hotkey(number: usize) {
    let Some(index) = number.checked_sub(1) else {
        return;
    };
    let mut at = POINT::default();
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) };
    let monitor = unsafe {
        windows::Win32::Graphics::Gdi::MonitorFromPoint(
            at,
            windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        )
    };
    let key = {
        let shared = shared();
        shared
            .docks
            .iter()
            .find(|placed| placed.handle == monitor.0 as isize)
            .or(shared.docks.first())
            .map(|placed| placed.monitor.clone())
    };
    if let (Some(key), Some(app)) = (key, APP.get()) {
        crate::commands::workspaces::switch_to_index(app, &key, index);
    }
}

/// What the dock's menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuChoice {
    TaskManager,
    Launcher,
    Settings,
    AutoHide,
    Quit,
    /// Show the workspace at this index (spec 0013).
    Workspace(usize),
    NewWorkspace,
    /// The widget clicked becomes this many icons wide (spec 0017 F1b).
    Width(u8),
}

/// One line of the dock's menu: what it does and how it shows (spec 0014
/// R3).
#[derive(Debug, Clone, PartialEq)]
struct MenuEntry {
    choice: Option<MenuChoice>,
    line: menu::Line,
}

/// Glyphs of Windows' icon font for the dock's menu.
const SEARCH_GLYPH: char = '\u{E721}';
const TASKS_GLYPH: char = '\u{E9D9}';
const ADD_GLYPH: char = '\u{E710}';
const SETTINGS_GLYPH: char = '\u{E713}';
const HIDE_GLYPH: char = '\u{E70D}';
const QUIT_GLYPH: char = '\u{E7E8}';

impl MenuEntry {
    fn item(choice: MenuChoice, item: menu::Item) -> MenuEntry {
        MenuEntry {
            choice: Some(choice),
            line: menu::Line::Item(item),
        }
    }

    fn separator() -> MenuEntry {
        MenuEntry {
            choice: None,
            line: menu::Line::Separator,
        }
    }

    fn title(text: &str) -> MenuEntry {
        MenuEntry {
            choice: None,
            line: menu::Line::Title(text.to_owned()),
        }
    }
}

/// A workspace as the dock's menu lists it: its name, its color (`#RRGGBB`)
/// and whether it is the one shown.
struct MenuWorkspace {
    name: String,
    color: String,
    shown: bool,
}

/// The dock's menu, in order. `workspaces`: with two or more, they come
/// under a title, with their color and their shortcut, the shown one
/// checked; "Novo espaço" comes while one more can be made. The tray icon's
/// menu is this one with no workspaces.
fn dock_menu_items(
    auto_hide: bool,
    workspaces: &[MenuWorkspace],
    can_create: bool,
) -> Vec<MenuEntry> {
    let text = &crate::i18n::messages().tray;
    let mut items = vec![
        MenuEntry::item(
            MenuChoice::Launcher,
            menu::Item::new(text.search_apps, SEARCH_GLYPH),
        ),
        MenuEntry::item(
            MenuChoice::TaskManager,
            menu::Item::new(text.task_manager, TASKS_GLYPH),
        ),
        MenuEntry::separator(),
    ];
    if workspaces.len() >= 2 {
        items.push(MenuEntry::title(text.workspaces));
        for (index, workspace) in workspaces.iter().enumerate() {
            let color = Color::hex(&workspace.color).unwrap_or(Color::WHITE);
            let rgb = |part: f32| (part * 255.0).round() as u8;
            items.push(MenuEntry::item(
                MenuChoice::Workspace(index),
                menu::Item::new(&workspace.name, SEARCH_GLYPH)
                    .swatch((rgb(color.r), rgb(color.g), rgb(color.b)))
                    .shortcut(format!("Ctrl+Alt+{}", index + 1))
                    .checked(workspace.shown),
            ));
        }
    }
    if can_create {
        items.push(MenuEntry::item(
            MenuChoice::NewWorkspace,
            menu::Item::new(text.new_workspace, ADD_GLYPH),
        ));
    }
    if workspaces.len() >= 2 || can_create {
        items.push(MenuEntry::separator());
    }
    items.extend([
        MenuEntry::item(
            MenuChoice::Settings,
            menu::Item::new(text.open_settings, SETTINGS_GLYPH),
        ),
        MenuEntry::item(
            MenuChoice::AutoHide,
            menu::Item::new(text.auto_hide, HIDE_GLYPH).checked(auto_hide),
        ),
        MenuEntry::separator(),
        MenuEntry::item(MenuChoice::Quit, menu::Item::new(text.quit, QUIT_GLYPH)),
    ]);
    items
}

/// Glyph of the widths in a widget's menu.
const SIZE_GLYPH: char = '\u{E8A9}';

/// What the dock's menu starts with when it opens on a widget that takes
/// more than one width (spec 0017 F1b): the widths under a title, the one
/// in use checked, then a separator. Nothing for the other widgets.
fn width_items(widths: std::ops::RangeInclusive<u8>, current: u8) -> Vec<MenuEntry> {
    if widths.start() == widths.end() {
        return Vec::new();
    }
    let text = &crate::i18n::messages().tray;
    let mut items = vec![MenuEntry::title(text.size)];
    for width in widths {
        let label = if width == 1 {
            text.one_icon.to_owned()
        } else {
            text.icons.replace("{n}", &width.to_string())
        };
        items.push(MenuEntry::item(
            MenuChoice::Width(width),
            menu::Item::new(&label, SIZE_GLYPH).checked(width == current),
        ));
    }
    items.push(MenuEntry::separator());
    items
}

/// Shows the menu `items` at the pointer, opening away from the screen edge,
/// and returns what was chosen.
fn show_dock_menu(edge: DockEdge, items: Vec<MenuEntry>) -> Option<MenuChoice> {
    let (choices, lines): (Vec<_>, Vec<_>) = items
        .into_iter()
        .map(|entry| (entry.choice, entry.line))
        .unzip();
    let chosen = menu::show(None, edge == DockEdge::Bottom, lines)?;
    choices.get(chosen).copied().flatten()
}

/// The tray icon's menu (spec 0014 R3: the DockMax's own, as the dock's),
/// opened at the pointer: what the dock's has, without the workspaces.
pub fn tray_menu(app: &AppHandle) {
    let auto_hide = crate::commands::settings::current(app).dock.auto_hide;
    // The taskbar is at the bottom of the screen, as a dock most often is.
    let chosen = show_dock_menu(DockEdge::Bottom, dock_menu_items(auto_hide, &[], false));
    if chosen == Some(MenuChoice::Settings) {
        // From the tray, the dock comes up too if it is hiding.
        show();
    }
    if let Some(choice) = chosen {
        act(app, choice, "", &[], None);
    }
}

/// Does what was chosen in the dock's menu (or the tray icon's): `monitor`
/// and `ids` are where the workspaces switch and which they are; `widget`
/// is the widget the menu opened on, if any.
fn act(
    app: &AppHandle,
    choice: MenuChoice,
    monitor: &str,
    ids: &[String],
    widget: Option<&crate::domain::widgets::WidgetId>,
) {
    match choice {
        MenuChoice::TaskManager => {
            if let Err(error) = super::apps::open_task_manager() {
                log::warn!("dock: the Task Manager could not be opened: {error}");
            }
        }
        MenuChoice::Settings => {
            if let Err(error) = super::settings_window::open(app) {
                log::warn!("dock: the settings could not be opened: {error}");
            }
        }
        MenuChoice::Launcher => super::launcher::toggle(),
        MenuChoice::AutoHide => {
            let changed = crate::commands::settings::change(app, |settings| {
                settings.dock.auto_hide = !settings.dock.auto_hide;
            });
            if let Err(error) = changed {
                log::warn!("dock: hiding on its own could not be changed: {error}");
            }
        }
        MenuChoice::Quit => app.exit(0),
        MenuChoice::Workspace(index) => {
            if let Some(id) = ids.get(index) {
                crate::commands::workspaces::switch(app, monitor, id);
            }
        }
        MenuChoice::NewWorkspace => {
            crate::commands::workspaces::create_and_switch(app, monitor);
        }
        MenuChoice::Width(width) => {
            if let Some(id) = widget {
                crate::commands::widgets::resize(app, id, width);
            }
        }
    }
}

fn local_now() -> windows::Win32::Foundation::SYSTEMTIME {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let mut handled = true;
    match message {
        WM_RUN => run_queue(),
        WM_MOUSEWHEEL => with_dock(hwnd, |dock| {
            // The high word of `wparam`: notches of 120, negative toward the
            // user.
            dock.wheel((wparam.0 >> 16) as u16 as i16);
        }),
        WM_HOTKEY => hotkey(wparam.0),
        WM_MOUSEMOVE => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.mouse_move(x, y);
        }),
        WM_MOUSELEAVE => with_dock(hwnd, Dock::mouse_leave),
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.left_down(x, y);
        }),
        WM_LBUTTONUP => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            dock.left_up(x, y);
        }),
        WM_MBUTTONDOWN => {
            shared().clicked = Some(Instant::now());
        }
        WM_MBUTTONUP => with_dock(hwnd, |dock| {
            let (x, y) = dock.point(lparam);
            if let Some(index) = dock.hit(x, y) {
                dock.middle_click(index);
            }
        }),
        WM_RBUTTONUP => with_dock(hwnd, |dock| {
            dock.end_preview();
            shared().clicked = Some(Instant::now());
            dock.use_here();
            let (x, y) = dock.point(lparam);
            if let Some(cell) = dock.strip_hit(x, y) {
                dock.strip_click(cell, true);
                return;
            }
            match dock.hit(x, y) {
                Some(index) if dock.items.get(index).is_some_and(Item::is_program) => {
                    dock.right_click(index);
                }
                // Anywhere else on the dock, a widget too: its own menu,
                // with the widget's widths first.
                hit => dock.dock_menu(hit),
            }
        }),
        WM_CAPTURECHANGED => with_dock(hwnd, |dock| {
            // The capture went elsewhere: the drag is over.
            dock.show_drag_cursor(None);
            if dock.press.take().is_some() {
                dock.pressed = None;
                dock.animate();
                dock.paint();
            }
        }),
        WM_TIMER => match wparam.0 {
            ANIMATION_TIMER => with_dock(hwnd, Dock::step),
            CLOCK_TIMER => with_dock(hwnd, |dock| {
                dock.paint();
                dock.arm_clock();
                dock.check_water();
            }),
            STATUS_TIMER => with_dock(hwnd, Dock::read_system),
            CARD_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), CARD_TIMER) };
                dock.show_card();
            }),
            ACTIVITY_TIMER => with_dock(hwnd, Dock::read_activity),
            MARQUEE_TIMER => with_dock(hwnd, Dock::paint),
            AUTO_HIDE_TIMER => with_dock(hwnd, Dock::watch_pointer),
            PREVIEW_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), PREVIEW_TIMER) };
                dock.show_preview();
            }),
            WARM_TIMER => with_dock(hwnd, |dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), WARM_TIMER) };
                if dock.warmable() && dock.open.is_none() {
                    dock.effects.push(Box::new(super::panel_window::warm));
                }
            }),
            _ => handled = false,
        },
        WM_SETTINGCHANGE => with_dock(hwnd, |dock| {
            dock.dark = theme::system_dark();
            dock.motion = system_motion();
            dock.place();
            dock.paint();
        }),
        // A monitor came, went or changed: a dock each, placed again.
        WM_DISPLAYCHANGE => with_docks(sync),
        WM_DPICHANGED => with_dock(hwnd, |dock| {
            dock.place();
            dock.paint();
        }),
        // Alt+F4 hides the dock instead of closing it.
        WM_CLOSE => with_dock(hwnd, Dock::hide),
        WM_GETOBJECT => match access::get_object(hwnd, wparam, lparam) {
            Some(result) => return result,
            None => handled = false,
        },
        _ => handled = false,
    }
    if handled {
        LRESULT(0)
    } else {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }
}

// ---- What the rest of the app calls ----

/// The data changed (programs, widgets, open programs, settings): the docks
/// read it again.
/// Lets go of the docks and what draws them, on the main thread, as the app
/// ends (`RunEvent::Exit`). Left in the thread's storage, they would be
/// dropped inside `ExitProcess`, after Windows has ended the other threads:
/// Direct2D then calls DXGI, which refuses (`DXGI_ERROR_INVALID_CALL`) and
/// ends the process with an exception, so every exit was a crash. The
/// windows go with the process.
pub fn shutdown() {
    let docks = DOCKS.with(|slot| {
        slot.try_borrow_mut()
            .map(|mut docks| std::mem::take(&mut *docks))
            .unwrap_or_default()
    });
    drop(docks);
}

pub fn refresh() {
    post(Dock::reload);
}

/// The Calendário's events were read again: the docks repaint.
pub fn calendar_changed() {
    post(Dock::paint);
}

/// A click on the Calendário's card: its panel opens on the dock `owner`,
/// on `event`'s details when given.
pub fn open_calendar(
    owner: isize,
    widget: crate::domain::widgets::WidgetId,
    event: Option<String>,
) {
    post_to(owner, move |dock| dock.open_calendar(&widget, event));
}

/// The widgets `widgets` have a notice not seen yet: a Ciclo's (F2b-3) or
/// a Pomodoro's (F2c-2).
pub fn nudged(widgets: Vec<String>) {
    post(move |dock| dock.nudged(&widgets));
}

/// Files dragged over the dock `hwnd` at the screen point `at`, or gone
/// with `None` (spec 0018): the id of the program under them.
pub fn drop_over(hwnd: isize, at: Option<(i32, i32)>) -> Option<String> {
    let mut found = None;
    with_dock(HWND(hwnd as _), |dock| found = dock.drop_over(at));
    found
}

/// Whether the pointer is off the dock's window.
fn pointer_off(hwnd: HWND) -> bool {
    let mut at = POINT::default();
    let mut window = windows::Win32::Foundation::RECT::default();
    unsafe {
        if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at).is_err()
            || GetWindowRect(hwnd, &mut window).is_err()
        {
            return false;
        }
    }
    at.x < window.left || at.x >= window.right || at.y < window.top || at.y >= window.bottom
}

/// What plays now changed (spec 0017 F4).
pub fn media_changed() {
    post(Dock::media_changed);
}

/// The notification badges of the taskbar changed (spec 0016).
pub fn badges_changed() {
    post(Dock::notices_changed);
}

/// The icon pack or its folder changed: reads the icons again.
pub fn forget_icons() {
    post(|dock| {
        dock.icons.clear();
        dock.reload();
    });
}

/// Shows the docks, up if they hide on their own, but not one whose monitor
/// is in full screen.
pub fn show() {
    SHOWN.store(true, Ordering::SeqCst);
    post(|dock| {
        if !dock.hidden_for_fullscreen {
            dock.show();
            dock.untuck();
        }
    });
}

/// The monitor `key` is in full screen (`full`) or not any more (spec 0010
/// D4: each dock looks only at its own monitor): its dock steps aside, and
/// comes back after unless the user hid the docks meanwhile.
pub fn set_fullscreen(key: String, full: bool) {
    post(move |dock| {
        if dock.monitor.key != key {
            return;
        }
        if full && dock.visible {
            log::info!("dock: a program is in full screen on its monitor, stepping aside");
            dock.hidden_for_fullscreen = true;
            dock.hide();
        } else if !full && dock.hidden_for_fullscreen {
            log::info!("dock: full screen is over on its monitor, back");
            dock.hidden_for_fullscreen = false;
            if SHOWN.load(Ordering::SeqCst) {
                dock.show();
            }
        }
    });
}

/// The monitors the docks are on: their key, handle and rectangle, for the
/// full screen check.
pub fn monitors() -> Vec<(String, isize, Rect)> {
    shared()
        .docks
        .iter()
        .map(|placed| (placed.monitor.clone(), placed.handle, placed.rect))
        .collect()
}

/// What the E2E suite reads of the dock (test build only): each item's name
/// and what its face draws, and whether it is painted dark.
#[cfg(feature = "e2e")]
pub struct View {
    pub items: Vec<(String, &'static str)>,
    pub dark: bool,
}

/// Asks the primary monitor's dock for its `View`, from a thread other than
/// the main one.
#[cfg(feature = "e2e")]
pub fn view() -> Option<View> {
    let (sent, received) = std::sync::mpsc::channel();
    post_primary(move |dock| {
        let items = dock
            .items
            .iter()
            .zip(&dock.names)
            .map(|(item, name)| {
                let face = match item {
                    Item::App(_) | Item::Other(_) => "app",
                    Item::System(_) => "glyph",
                    Item::Widget(widget) => match &widget.config {
                        crate::domain::widgets::WidgetConfig::Clock(clock) => {
                            match clock.icon_style {
                                crate::domain::widgets::clock::ClockIconStyle::Analog => "dial",
                                crate::domain::widgets::clock::ClockIconStyle::Digital => "time",
                            }
                        }
                        crate::domain::widgets::WidgetConfig::TimeProgress(progress) => {
                            if progress.width > 1 { "meters" } else { "ring" }
                        }
                        crate::domain::widgets::WidgetConfig::Activity(_) => "meters",
                        crate::domain::widgets::WidgetConfig::Network(_) => "rates",
                        crate::domain::widgets::WidgetConfig::Calendar(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Cycle(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Countdown(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Pomodoro(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Todo(_) => "agenda",
                        crate::domain::widgets::WidgetConfig::Water(_) => "ring",
                        crate::domain::widgets::WidgetConfig::Media(_) => "media",
                        _ => "glyph",
                    },
                };
                (name.clone(), face)
            })
            .collect();
        let _ = sent.send(View {
            items,
            dark: dock.palette().dark,
        });
    });
    received.recv_timeout(Duration::from_secs(5)).ok()
}

/// Presses the item named `name` on the primary monitor's dock, as a click
/// would (test build only, from a thread other than the main one). `false`
/// when the dock has no such item.
#[cfg(feature = "e2e")]
pub fn press_named(name: String) -> bool {
    let (sent, received) = std::sync::mpsc::channel();
    post_primary(move |dock| {
        let key = dock
            .names
            .iter()
            .position(|shown| *shown == name)
            .and_then(|index| dock.items.get(index))
            .map(Item::key);
        if let Some(key) = &key {
            shared().clicked = Some(Instant::now());
            dock.press(key);
        }
        // Configurações is in the dock's menu, not an item.
        let menu = key.is_none() && name == crate::i18n::messages().tray.open_settings;
        if menu {
            let app = dock.app.clone();
            dock.effects.push(Box::new(move |_| {
                if let Err(error) = super::settings_window::open(&app) {
                    log::warn!("e2e: the settings could not be opened: {error}");
                }
            }));
        }
        let _ = sent.send(key.is_some() || menu);
    });
    received
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or(false)
}

/// Where a tray icon dragged out of the tray panel and let go at `(x, y)`
/// (physical pixels on the screen) goes among those the dock shows: `None`
/// when it was not let go on a dock; else before the tray icon kept in the
/// dock that follows the cell under the pointer (its lasting id), or at the
/// end (`Some(None)`). From a thread other than the main one.
pub fn tray_drop_index(x: i32, y: i32) -> Option<Option<String>> {
    let (sent, received) = std::sync::mpsc::channel();
    queue(Box::new(move |docks| {
        let found = docks.iter().find_map(|dock| dock.tray_drop_index(x, y));
        let _ = sent.send(found);
    }));
    received.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

/// The monitors or the bands changed under the docks: they are placed again
/// (and made or destroyed for a monitor plugged in or unplugged).
pub fn reposition() {
    queue(Box::new(sync));
}

/// The open panel changed: the dock it anchors to marks its item and stops
/// magnifying; the others have none open.
pub fn set_open(panel: Option<PanelId>) {
    queue(Box::new(move |docks| {
        let active = shared().active;
        for dock in docks.iter_mut() {
            let mine = dock.hwnd.0 as isize == active;
            dock.set_open(if mine { panel.clone() } else { None });
        }
    }));
}

/// The settings moved the docks (or put them on every monitor or on one):
/// lays them out again now and, once placed, reopens the panel the move
/// closed at the dock it was anchored to.
pub fn settings_moved(app: &AppHandle) {
    let app = app.clone();
    queue(Box::new(move |docks| {
        for dock in docks.iter_mut() {
            dock.data = read_data(&dock.app, &dock.monitor.key);
        }
        sync(docks);
        let reopen = super::panel::lock(&app).dock_settled(false, Instant::now());
        if let Some(panel) = reopen {
            let active = shared().active;
            let index = docks
                .iter()
                .position(|dock| dock.hwnd.0 as isize == active)
                .unwrap_or(0);
            if let Some(dock) = docks.get_mut(index) {
                dock.request(panel);
            }
        }
    }));
}

/// Whether Windows shows apps dark (the launcher follows it too).
pub fn system_dark() -> bool {
    theme::system_dark()
}

/// The monitor (its device name) of the dock whose window is `hwnd`.
pub fn monitor_of(hwnd: isize) -> Option<String> {
    shared()
        .docks
        .iter()
        .find(|placed| placed.hwnd == hwnd)
        .map(|placed| placed.monitor.clone())
}

/// Whether a dock is on screen.
pub fn is_visible() -> bool {
    shared().docks.iter().any(|placed| placed.visible)
}

/// The dock the panel anchors to: the one whose item opened it, or the
/// primary monitor's.
fn anchor_dock(shared: &Shared) -> Option<&Placed> {
    shared
        .docks
        .iter()
        .find(|placed| placed.hwnd == shared.active)
        .or_else(|| shared.docks.first())
}

/// The anchoring dock's window origin on the screen (physical pixels), its
/// scale and its bar on the screen: where the panels anchor.
pub fn geometry() -> Option<((i32, i32), f64, Rect)> {
    let shared = shared();
    let placed = anchor_dock(&shared)?;
    Some(((placed.window.x, placed.window.y), placed.scale, placed.bar))
}

/// The work area of the anchoring dock's monitor, and its scale.
pub fn work_area() -> Option<(Rect, f64)> {
    let shared = shared();
    let placed = anchor_dock(&shared)?;
    Some((placed.work, placed.scale))
}

/// Whether the user clicked a dock a moment ago: the panel that click opens
/// may take the focus.
pub fn clicked_recently() -> bool {
    shared()
        .clicked
        .is_some_and(|at| at.elapsed() < CLICK_GRANTS_FOCUS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Rect = Rect {
        x: -1920,
        y: 6,
        width: 1920,
        height: 1200,
    };

    #[test]
    fn the_pointer_on_the_screen_edge_brings_the_dock_up() {
        assert!(at_screen_edge((-900, 1205), MONITOR, DockEdge::Bottom));
        assert!(at_screen_edge((-1920, 1204), MONITOR, DockEdge::Bottom));
        assert!(!at_screen_edge((-900, 1200), MONITOR, DockEdge::Bottom));
        // The other monitor's edge is not this one's.
        assert!(!at_screen_edge((100, 1205), MONITOR, DockEdge::Bottom));
        assert!(at_screen_edge((-900, 6), MONITOR, DockEdge::Top));
        assert!(!at_screen_edge((-900, 1205), MONITOR, DockEdge::Top));
    }

    #[test]
    fn the_dock_stays_up_with_the_pointer_on_it_or_below_it() {
        let window = Rect {
            x: -1387,
            y: 1065,
            width: 853,
            height: 138,
        };

        assert!(near_dock((-1000, 1100), window, MONITOR, DockEdge::Bottom));
        // Between the dock and the screen edge.
        assert!(near_dock((-1000, 1205), window, MONITOR, DockEdge::Bottom));
        assert!(!near_dock((-1000, 1000), window, MONITOR, DockEdge::Bottom));
        assert!(!near_dock((-1500, 1100), window, MONITOR, DockEdge::Bottom));
    }

    #[test]
    fn the_dock_slides_out_and_in() {
        let mut now = 0.0;
        let mut frames = 0;
        while now != 1.0 {
            let next = tuck_step(now, 1.0);
            assert!(next > now && next <= 1.0);
            now = next;
            frames += 1;
        }
        // About a quarter of a second at 60 frames a second.
        assert!((8..=20).contains(&frames), "{frames} frames");
        assert_eq!(tuck_step(1.0, 0.0), 0.75);
    }

    /// The menu as text: an item's label (with ✓ when checked), "—" for a
    /// separator and "# title" for a title.
    fn labels(items: &[MenuEntry]) -> Vec<String> {
        items
            .iter()
            .map(|entry| match &entry.line {
                menu::Line::Item(item) => {
                    let check = if item.checked { " ✓" } else { "" };
                    format!("{}{check}", item.label)
                }
                menu::Line::Separator => "—".to_owned(),
                menu::Line::Title(title) => format!("# {title}"),
            })
            .collect()
    }

    fn workspace(name: &str, color: &str, shown: bool) -> MenuWorkspace {
        MenuWorkspace {
            name: name.to_owned(),
            color: color.to_owned(),
            shown,
        }
    }

    #[test]
    fn the_dock_menu_has_what_the_tray_menu_has() {
        // One workspace: no list to switch, "Novo espaço" to make the second.
        let one = [workspace("Principal", "#0A84FF", true)];
        assert_eq!(
            labels(&dock_menu_items(true, &one, true)),
            [
                "Buscar apps…",
                "Gerenciador de Tarefas",
                "—",
                "Novo espaço",
                "—",
                "Configurações",
                "Ocultar automaticamente ✓",
                "—",
                "Sair",
            ]
        );
        // The tray icon's: the same, without the workspaces.
        assert_eq!(
            labels(&dock_menu_items(false, &[], false)),
            [
                "Buscar apps…",
                "Gerenciador de Tarefas",
                "—",
                "Configurações",
                "Ocultar automaticamente",
                "—",
                "Sair",
            ]
        );
    }

    #[test]
    fn every_item_has_a_glyph_and_auto_hide_says_it_turns() {
        for entry in dock_menu_items(true, &[], true) {
            if let menu::Line::Item(item) = entry.line {
                assert!(item.glyph.is_some(), "{}", item.label);
                let turns = entry.choice == Some(MenuChoice::AutoHide);
                assert_eq!(item.toggle, turns, "{}", item.label);
            }
        }
    }

    #[test]
    fn the_dock_menu_switches_workspaces_with_their_color_and_shortcut() {
        let two = [
            workspace("Principal", "#0A84FF", false),
            workspace("Foco", "#30D158", true),
        ];
        let items = dock_menu_items(false, &two, false);
        assert_eq!(
            labels(&items)[3..7],
            ["# Espaços de trabalho", "Principal", "Foco ✓", "—"]
        );
        assert_eq!(items[5].choice, Some(MenuChoice::Workspace(1)));
        let menu::Line::Item(focus) = &items[5].line else {
            panic!("not an item");
        };
        assert_eq!(focus.swatch, Some((0x30, 0xD1, 0x58)));
        assert_eq!(focus.glyph, None);
        assert_eq!(focus.shortcut.as_deref(), Some("Ctrl+Alt+2"));
        // A title chooses nothing.
        assert_eq!(items[3].choice, None);
    }

    #[test]
    fn a_widget_that_takes_widths_lists_them_first() {
        let items = width_items(1..=3, 2);
        assert_eq!(
            labels(&items),
            ["# Tamanho", "1 ícone", "2 ícones ✓", "3 ícones", "—"]
        );
        assert_eq!(items[3].choice, Some(MenuChoice::Width(3)));
        // One width: nothing to choose.
        assert!(width_items(1..=1, 1).is_empty());
    }

    #[test]
    fn a_bounce_rises_away_from_the_edge_and_lands() {
        let at = |ms: u64, edge| {
            opening_offset(OpenAnimation::Bounce, Duration::from_millis(ms), 48.0, edge)
        };
        assert_eq!(at(0, DockEdge::Bottom), (0.0, 0.0, 1.0));
        assert!((at(210, DockEdge::Bottom).1 + 24.0).abs() < 1e-6);
        assert!((at(210, DockEdge::Top).1 - 24.0).abs() < 1e-6);
        assert_eq!(at(1400, DockEdge::Bottom), (0.0, 0.0, 1.0));
    }

    #[test]
    fn a_pulse_swells_and_a_shake_moves_sideways() {
        let pulse = opening_offset(
            OpenAnimation::Pulse,
            Duration::from_millis(250),
            48.0,
            DockEdge::Bottom,
        );
        assert!((pulse.2 - 1.22).abs() < 1e-6);
        let shake = opening_offset(
            OpenAnimation::Shake,
            Duration::from_millis(100),
            48.0,
            DockEdge::Bottom,
        );
        assert!((shake.0 + 4.0).abs() < 1e-6);
        assert_eq!(
            opening_offset(
                OpenAnimation::Pulse,
                Duration::from_millis(1600),
                48.0,
                DockEdge::Bottom
            ),
            (0.0, 0.0, 1.0)
        );
    }
}
