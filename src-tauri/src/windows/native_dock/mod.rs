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

mod access;
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
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, HWND_TOPMOST,
    IDC_ARROW, KillTimer, LoadCursorW, PostMessageW, RegisterClassW, SPI_GETCLIENTAREAANIMATION,
    SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetTimer, SetWindowPos, ShowWindow, SystemParametersInfoW,
    WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_GETOBJECT,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
    WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use self::paint::{Face, Painter, Scene, Status, StripCell, Tile, Tip};
use self::strip::Cell;
use self::theme::{Color, Palette};
use super::dock::Rect;
use super::monitors::Monitor;
use super::panel::{LogicalRect, PanelId};
use crate::domain::apps::AppsSnapshot;
use crate::domain::dock_items::{self, Item, Names, SystemItem};
use crate::domain::dock_layout::{
    self, Area, DRAG_GROW, DRAG_LIFT, DRAG_THRESHOLD, DockSize, Hover, Magnification, Shape,
    drag_offset, drag_pitch, drop_index, make_room, slide_step,
};
use crate::domain::open_apps::OpenSnapshot;
use crate::domain::settings::{DockEdge, MiddleClick, OpenAnimation, Settings, TaskbarMode, Theme};
use crate::domain::widgets::WidgetsSnapshot;
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
const STATUS_TIMER: usize = 4;
/// Looks at the pointer while the dock hides on its own.
const AUTO_HIDE_TIMER: usize = 5;
const AUTO_HIDE_EVERY: u32 = 100;

/// How often the battery, the network and the volume are read again.
const STATUS_EVERY: u32 = 3_000;

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

/// What the dock reads from the rest of the app.
struct Data {
    settings: Settings,
    widgets: Option<WidgetsSnapshot>,
    apps: Option<AppsSnapshot>,
    open: OpenSnapshot,
    /// How the programs and the widgets mix (`Storage::dock_order`).
    order: Vec<String>,
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
    /// The small icons at the end, and the one under the pointer.
    strip: Vec<Cell>,
    strip_hovered: Option<usize>,
    /// The battery, the network and the volume, as last read.
    system: Option<super::system_status::Status>,
    /// The tray icons' images, by icon key: the image they came from and
    /// the bitmap.
    tray_images: HashMap<String, (String, Option<ID2D1Bitmap>)>,
    /// The item held down by the left button.
    pressed: Option<usize>,
    press: Option<Press>,
    /// While dragging: the pointer's x.
    drag_x: f64,
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
    let dock = Dock {
        app: app.clone(),
        hwnd,
        monitor,
        painter,
        data: read_data(app),
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
        strip: Vec::new(),
        strip_hovered: None,
        system: Some(super::system_status::read()),
        tray_images: HashMap::new(),
        pressed: None,
        press: None,
        drag_x: 0.0,
        landing: None,
        slide: Vec::new(),
        open: None,
        opening: HashMap::new(),
        icons: HashMap::new(),
        badges: HashMap::new(),
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

fn read_data(app: &AppHandle) -> Data {
    let settings = crate::commands::settings::current(app);
    #[cfg(windows)]
    let tray = crate::commands::tray::icons();
    let (widgets, apps, order) = match app.try_state::<Mutex<Storage>>() {
        Some(state) => {
            let storage = state.lock().unwrap_or_else(PoisonError::into_inner);
            (
                crate::commands::widgets::snapshot(&storage).ok(),
                crate::commands::apps::snapshot(&storage).ok(),
                storage.dock_order(),
            )
        }
        None => (None, None, Vec::new()),
    };
    Data {
        settings,
        widgets,
        apps,
        open: super::open_apps::snapshot(),
        order,
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
        }
    }

    /// The window's client area, in logical pixels.
    fn viewport(&self) -> (f64, f64) {
        (
            f64::from(self.window.width) / self.scale,
            f64::from(self.window.height) / self.scale,
        )
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

    /// Reads the data again and lays the dock out for it.
    fn reload(&mut self) {
        self.data = read_data(&self.app);
        self.rebuild();
    }

    /// Lays the dock out for the data it has: the items, their names, the
    /// window, and paints.
    fn rebuild(&mut self) {
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
                    return 0.0;
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
            order: &self.data.order,
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
        self.request_pictures();
        self.place();
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
            _ => self.dock_menu(),
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
        access::publish(
            hwnd,
            self.items
                .iter()
                .zip(&self.names)
                .zip(&rest.items)
                .map(|((item, name), area)| {
                    let on_screen = to_screen(*area);
                    access::Entry {
                        key: item.key(),
                        name: if attention && matches!(item, Item::System(SystemItem::Settings)) {
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
                )
                .collect(),
        );
    }

    /// Presses the item `key` (assistive technology), as a click would.
    fn press(&mut self, key: &str) {
        if let Some(index) = self.items.iter().position(|item| item.key() == key) {
            self.click(index);
            self.paint();
        } else if let Some(index) = self.strip.iter().position(|cell| cell.key() == key) {
            self.strip_click(index, false);
            self.paint();
        }
    }

    /// The dock's monitor is gone (or the user keeps only one dock).
    fn destroy(&mut self) {
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
            || menu_on_screen();
        if busy || near_dock(point, self.window, self.monitor.rect, edge) {
            self.away_since = None;
            return;
        }
        let since = *self.away_since.get_or_insert(now);
        if now.duration_since(since) >= tuck_after {
            self.away_since = None;
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
    }

    fn palette(&self) -> Palette {
        let dark = match self.data.settings.theme {
            Theme::System => self.dark,
            Theme::Light => false,
            Theme::Dark => true,
        };
        Palette::of(dark, self.data.settings.dock.opacity)
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
                dot: Some((box_area.center_x(), dot)),
                item: box_area,
            });
        }
        let tip = self
            .magnified()
            .or(self.hovered.filter(|_| self.open.is_none()))
            .filter(|_| drag.is_none())
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
        let status = now_placement.status.map(|area| Status {
            area,
            time: format!("{:02}:{:02}", clock.wHour, clock.wMinute),
            date: format!("{:02}/{:02}/{:04}", clock.wDay, clock.wMonth, clock.wYear),
            hovered: self.status_hovered,
        });
        let indicator_color = settings
            .indicator_color
            .as_deref()
            .and_then(Color::hex)
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
            Item::Widget(widget) => faces::widget(widget, size, clock),
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
        if let Some(press) = self.press.as_mut() {
            if !press.dragging && (x - press.start_x).abs() >= DRAG_THRESHOLD {
                press.dragging = true;
                self.hovered = None;
                self.pressed = None;
            }
            if press.dragging {
                self.drag_x = x;
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
        let hovered = if status || cell.is_some() {
            None
        } else {
            self.hit(x, y)
        };
        if hovered != self.hovered || status != self.status_hovered || cell != self.strip_hovered {
            self.hovered = hovered;
            self.status_hovered = status;
            self.strip_hovered = cell;
            let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
            if self.warmable() {
                unsafe { SetTimer(Some(self.hwnd), WARM_TIMER, WARM_DELAY, None) };
            }
            self.animate();
            self.paint();
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
        if self.press.is_some() {
            return;
        }
        self.hovered = None;
        self.status_hovered = false;
        self.strip_hovered = None;
        self.pressed = None;
        let _ = unsafe { KillTimer(Some(self.hwnd), WARM_TIMER) };
        self.animate();
        self.paint();
    }

    fn left_down(&mut self, x: f64, y: f64) {
        shared().clicked = Some(Instant::now());
        let index = self.hit(x, y);
        self.pressed = index;
        if let Some((index, group)) = index.and_then(|index| Some((index, self.group_of(index)?)))
            && self.open.is_none()
        {
            self.press = Some(Press {
                index,
                group,
                start_x: x,
                dragging: false,
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
            self.click(index);
        }
        self.paint();
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
        // A program or a widget, among both: the dock's order, then the
        // place among its own kind.
        let group = &self.items[press.range()];
        let Some((order, among)) = dock_items::moved(group, press.local(), target) else {
            return false;
        };
        self.effects.push(Box::new(move |app| {
            {
                let state = app.state::<Mutex<Storage>>();
                let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
                if let Err(error) = storage.set_dock_order(&order) {
                    log::warn!("dock: the order could not be kept: {error}");
                }
            }
            match item {
                Item::App(pinned) => {
                    let state = app.state::<Mutex<Storage>>();
                    let id = pinned.id.as_str().to_string();
                    if let Err(error) =
                        crate::commands::apps::place_app(app.clone(), state, id, among)
                    {
                        log::warn!("dock: placing a program failed: {error:?}");
                    }
                }
                Item::Widget(widget) => {
                    crate::commands::widgets::place_widget(app, widget.id.as_str(), among);
                }
                _ => {}
            }
            // Only the mix may have changed: the dock reads it again.
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
                if running {
                    self.effects.push(Box::new(move |app| {
                        if !super::open_apps::activate(app, &id) {
                            post_to(hwnd, move |dock| dock.launch_pinned(key, id));
                        }
                    }));
                } else {
                    self.launch_pinned(key, id);
                }
            }
            Item::Other(other) => {
                let key = other.key.clone();
                self.effects.push(Box::new(move |app| {
                    super::open_apps::activate_other(app, &key);
                }));
            }
            _ => {
                let anchor = self.rest().items[index];
                self.toggle(panel_of(&item), anchor);
            }
        }
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
    fn dock_menu(&mut self) {
        self.hovered = None;
        self.animate();
        self.paint();
        let hwnd = self.hwnd.0 as isize;
        let edge = self.edge();
        let auto_hide = self.data.settings.dock.auto_hide;
        // Up, under its menu, until it closes.
        self.menu_open = true;
        self.effects.push(Box::new(move |app| {
            let chosen = show_dock_menu(HWND(hwnd as _), edge, auto_hide);
            post_to(hwnd, |dock: &mut Dock| dock.menu_open = false);
            match chosen {
                Some(MenuChoice::TaskManager) => {
                    if let Err(error) = super::apps::open_task_manager() {
                        log::warn!("dock: the Task Manager could not be opened: {error}");
                    }
                }
                Some(MenuChoice::Settings) => {
                    if let Err(error) = super::settings_window::open(app) {
                        log::warn!("dock: the settings could not be opened: {error}");
                    }
                }
                Some(MenuChoice::AutoHide) => {
                    let changed = crate::commands::settings::change(app, |settings| {
                        settings.dock.auto_hide = !settings.dock.auto_hide;
                    });
                    if let Err(error) = changed {
                        log::warn!("dock: hiding on its own could not be changed: {error}");
                    }
                }
                Some(MenuChoice::Quit) => app.exit(0),
                None => {}
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
    unsafe { FindWindowW(w!("#32768"), None) }
        .is_ok_and(|menu| unsafe { IsWindowVisible(menu) }.as_bool())
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

/// What the dock's menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuChoice {
    TaskManager,
    Settings,
    AutoHide,
    Quit,
}

/// The dock's menu, in order; `None` is a separator.
fn dock_menu_items() -> [Option<(MenuChoice, &'static str)>; 6] {
    let text = &crate::i18n::messages().tray;
    [
        Some((MenuChoice::TaskManager, text.task_manager)),
        None,
        Some((MenuChoice::Settings, text.open_settings)),
        Some((MenuChoice::AutoHide, text.auto_hide)),
        None,
        Some((MenuChoice::Quit, text.quit)),
    ]
}

/// Shows the dock's menu at the pointer, opening away from the screen edge,
/// with "Ocultar automaticamente" checked when `auto_hide`, and returns what
/// was chosen.
fn show_dock_menu(hwnd: HWND, edge: DockEdge, auto_hide: bool) -> Option<MenuChoice> {
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, MF_CHECKED, MF_SEPARATOR,
        MF_STRING, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_CENTERALIGN, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, TPM_TOPALIGN, TrackPopupMenu, WM_NULL,
    };
    let items = dock_menu_items();
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        for (index, item) in items.iter().enumerate() {
            // The command of an item is its place plus one: 0 is no choice.
            let added = match item {
                Some((choice, label)) => AppendMenuW(
                    menu,
                    if *choice == MenuChoice::AutoHide && auto_hide {
                        MF_STRING | MF_CHECKED
                    } else {
                        MF_STRING
                    },
                    index + 1,
                    &windows::core::HSTRING::from(*label),
                ),
                None => AppendMenuW(menu, MF_SEPARATOR, 0, None),
            };
            if added.is_err() {
                let _ = DestroyMenu(menu);
                return None;
            }
        }
        let mut at = POINT::default();
        let _ = GetCursorPos(&mut at);
        // A menu closes on a click elsewhere only if its window is in front;
        // the click on the dock was the last input, so Windows allows it.
        let _ = SetForegroundWindow(hwnd);
        let vertical = match edge {
            DockEdge::Bottom => TPM_BOTTOMALIGN,
            DockEdge::Top => TPM_TOPALIGN,
        };
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_CENTERALIGN | vertical,
            at.x,
            at.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        let chosen = usize::try_from(chosen.0).ok()?.checked_sub(1)?;
        items
            .get(chosen)
            .copied()
            .flatten()
            .map(|(choice, _)| choice)
    }
}

fn local_now() -> windows::Win32::Foundation::SYSTEMTIME {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let mut handled = true;
    match message {
        WM_RUN => run_queue(),
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
            shared().clicked = Some(Instant::now());
            let (x, y) = dock.point(lparam);
            if let Some(cell) = dock.strip_hit(x, y) {
                dock.strip_click(cell, true);
                return;
            }
            match dock.hit(x, y) {
                Some(index) if dock.items.get(index).is_some_and(Item::is_program) => {
                    dock.right_click(index);
                }
                // Anywhere else on the dock: its own menu.
                _ => dock.dock_menu(),
            }
        }),
        WM_CAPTURECHANGED => with_dock(hwnd, |dock| {
            // The capture went elsewhere: the drag is over.
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
            }),
            STATUS_TIMER => with_dock(hwnd, Dock::read_system),
            AUTO_HIDE_TIMER => with_dock(hwnd, Dock::watch_pointer),
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
pub fn refresh() {
    post(Dock::reload);
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
                        crate::domain::widgets::WidgetConfig::TimeProgress(_) => "ring",
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
            dock.data = read_data(&dock.app);
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

    #[test]
    fn the_dock_menu_has_what_the_tray_menu_has() {
        let labels: Vec<Option<&str>> = dock_menu_items()
            .iter()
            .map(|item| item.map(|(_, label)| label))
            .collect();

        assert_eq!(
            labels,
            [
                Some("Gerenciador de Tarefas"),
                None,
                Some("Configurações"),
                Some("Ocultar automaticamente"),
                None,
                Some("Sair"),
            ]
        );
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
