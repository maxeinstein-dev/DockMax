//! The dock, drawn natively (spec 0011 H3, ADR 0011): a layered window
//! painted with Direct2D, with no WebView2. The WebView2 is loaded only
//! when a panel or the Configurações app opens.
//!
//! Everything about the dock lives on the main thread (the one of the event
//! loop, which also runs this window's procedure). Other threads reach it
//! through `post`, which queues the work and wakes the window; what other
//! threads need to read (whether it is on screen, where it is) is kept in
//! `SHARED`.

mod access;
mod faces;
mod look;
mod paint;
mod svg;
mod theme;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicIsize, Ordering};
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
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW,
    PostMessageW, RegisterClassW, SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetTimer,
    SetWindowPos, ShowWindow, SystemParametersInfoW, WM_APP, WM_CAPTURECHANGED, WM_CLOSE,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_GETOBJECT, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::w;

use self::paint::{Face, Painter, Scene, Status, Tile, Tip};
use self::theme::{Color, Palette};
use super::dock::Rect;
use super::panel::{LogicalRect, PanelId};
use crate::domain::apps::AppsSnapshot;
use crate::domain::dock_items::{self, Item, Names, SystemItem};
use crate::domain::dock_layout::{
    self, Area, DRAG_THRESHOLD, DockSize, Hover, Magnification, Shape, drag_offset, drop_index,
    drop_mark,
};
use crate::domain::open_apps::OpenSnapshot;
use crate::domain::settings::{DockEdge, MiddleClick, OpenAnimation, Settings, TaskbarMode, Theme};
use crate::domain::widgets::WidgetsSnapshot;
use crate::persistence::database::Storage;

/// `WM_MOUSELEAVE`, which the features in use do not name.
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Runs the work queued by `post`.
const WM_RUN: u32 = WM_APP + 0x31;

const ANIMATION_TIMER: usize = 1;
const CLOCK_TIMER: usize = 2;
const WARM_TIMER: usize = 3;

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

/// What other threads read about the dock.
#[derive(Debug, Default)]
struct Shared {
    visible: bool,
    /// The window on the screen, in physical pixels.
    window: Option<Rect>,
    scale: f64,
    /// The bar on the screen, in physical pixels.
    bar: Option<Rect>,
    /// Each item's box at rest, relative to the window, in logical pixels:
    /// where the panel of each anchors.
    anchors: Vec<(PanelId, LogicalRect)>,
    /// When the dock was last clicked.
    clicked: Option<Instant>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    visible: false,
    window: None,
    scale: 1.0,
    bar: None,
    anchors: Vec::new(),
    clicked: None,
});

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The window, once made (0 before).
static WINDOW: AtomicIsize = AtomicIsize::new(0);

type Work = Box<dyn FnOnce(&mut Dock) + Send>;

static QUEUE: Mutex<Vec<Work>> = Mutex::new(Vec::new());

thread_local! {
    static DOCK: RefCell<Option<Dock>> = const { RefCell::new(None) };
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
}

/// A program's icon: being read, or read (`None` when it has none).
enum Loaded {
    Loading,
    Ready(Option<ID2D1Bitmap>),
}

pub struct Dock {
    app: AppHandle,
    hwnd: HWND,
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
    /// The item held down by the left button.
    pressed: Option<usize>,
    press: Option<Press>,
    /// While dragging: the pointer's x.
    drag_x: f64,
    /// The panel open, anchored to one of the items.
    open: Option<PanelId>,
    /// The programs whose icon is animating after a click, by item key.
    opening: HashMap<String, Instant>,
    icons: HashMap<(Option<String>, String), Loaded>,
    badges: HashMap<String, Loaded>,
    tracking: bool,
    animating: bool,
    visible: bool,
    effects: Vec<Effect>,
}

/// Queues `work` for the dock, from any thread; nothing happens before the
/// dock is made.
pub fn post(work: impl FnOnce(&mut Dock) + Send + 'static) {
    QUEUE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(Box::new(work));
    let hwnd = WINDOW.load(Ordering::SeqCst);
    if hwnd != 0 {
        let _ = unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_RUN, WPARAM(0), LPARAM(0)) };
    }
}

/// Makes the dock's window, hidden. On the main thread, once.
pub fn start(app: &AppHandle) -> windows::core::Result<()> {
    // COM and OLE on the main thread, for the images (WIC) and the drop
    // target: WebView2 used to do it, and none runs at startup any more.
    if let Err(error) = unsafe { windows::Win32::System::Ole::OleInitialize(None) } {
        log::warn!("dock: OLE could not be initialized: {error}");
    }
    let instance = unsafe { GetModuleHandleW(None) }?;
    let class = w!("DockMaxDock");
    let class_data = WNDCLASSW {
        style: CS_DBLCLKS,
        lpfnWndProc: Some(procedure),
        hInstance: instance.into(),
        lpszClassName: class,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    // Never activated by a click: the program the user is in stays in
    // front, and a click on a program knows which one that is.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
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
    }?;
    let painter = Painter::new()?;
    // Files dropped on the dock are pinned (spec 0006 B2).
    super::drop_target::install(app, hwnd);
    let dock = Dock {
        app: app.clone(),
        hwnd,
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
        pressed: None,
        press: None,
        drag_x: 0.0,
        open: None,
        opening: HashMap::new(),
        icons: HashMap::new(),
        badges: HashMap::new(),
        tracking: false,
        animating: false,
        visible: false,
        effects: Vec::new(),
    };
    DOCK.with(|slot| *slot.borrow_mut() = Some(dock));
    WINDOW.store(hwnd.0 as isize, Ordering::SeqCst);
    with_dock(|dock| {
        dock.rebuild();
        dock.arm_clock();
    });
    Ok(())
}

/// Runs `change` on the dock, then what it asked of the app.
fn with_dock(change: impl FnOnce(&mut Dock)) {
    let effects = DOCK.with(|slot| {
        let Ok(mut slot) = slot.try_borrow_mut() else {
            return None;
        };
        let dock = slot.as_mut()?;
        change(dock);
        Some((dock.app.clone(), std::mem::take(&mut dock.effects)))
    });
    if let Some((app, effects)) = effects {
        for effect in effects {
            effect(&app);
        }
    }
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
            with_dock(item);
        }
    }
}

fn read_data(app: &AppHandle) -> Data {
    let settings = crate::commands::settings::current(app);
    let (widgets, apps) = match app.try_state::<Mutex<Storage>>() {
        Some(state) => {
            let storage = state.lock().unwrap_or_else(PoisonError::into_inner);
            (
                crate::commands::widgets::snapshot(&storage).ok(),
                crate::commands::apps::snapshot(&storage).ok(),
            )
        }
        None => (None, None),
    };
    Data {
        settings,
        widgets,
        apps,
        open: super::open_apps::snapshot(),
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
        about: dock.about,
        tray: dock.tray,
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
        Item::System(SystemItem::About) => PanelId::About,
        Item::System(SystemItem::Tray) => PanelId::Tray,
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
        let settings = &self.data.settings;
        let found = dock_items::items(dock_items::Sources {
            widgets: self.data.widgets.as_ref(),
            apps: self.data.apps.as_ref(),
            open: &self.data.open,
            show_open: settings.dock.show_open_apps,
            tray: settings.dock.taskbar == TaskbarMode::Hidden,
        });
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
        self.animate();
        self.paint();
    }

    /// Asks for the icons and badges not read yet, off the main thread.
    fn request_pictures(&mut self) {
        let pack = self.data.settings.dock.icon_pack.clone();
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
                    post(move |dock| {
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
                    post(move |dock| {
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
        let Some((window, scale)) =
            super::dock::place_native(&self.app, &self.data.settings.dock, size, bar_height)
        else {
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
        let mut shared = shared();
        shared.window = Some(self.window);
        shared.scale = self.scale;
        shared.bar = Some(to_screen(rest.bar));
        shared.anchors = self
            .items
            .iter()
            .zip(&rest.items)
            .map(|(item, area)| (panel_of(item), logical(*area)))
            .collect();
        shared.visible = self.visible;
        drop(shared);
        // For assistive technology: each item's name (with the notice on
        // Configurações) and its box on the screen.
        let attention = dock_items::needs_attention(self.data.widgets.as_ref());
        let notice = crate::i18n::messages().dock.with_notice;
        access::publish(
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
                .collect(),
        );
    }

    /// Presses the item `key` (assistive technology), as a click would.
    fn press(&mut self, key: &str) {
        if let Some(index) = self.items.iter().position(|item| item.key() == key) {
            self.click(index);
            self.paint();
        }
    }

    fn show(&mut self) {
        self.place();
        self.visible = true;
        self.paint();
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
        self.publish();
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
        let now = Instant::now();
        self.opening
            .retain(|_, started| now.saturating_duration_since(*started) < OPENING);
        let dragging = self.press.is_some_and(|press| press.dragging);
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
        let origin = POINT {
            x: self.window.x,
            y: self.window.y,
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
        let drop = drag.and_then(|press| {
            let centers: Vec<f64> = rest.items[press.range()]
                .iter()
                .map(Area::center_x)
                .collect();
            let target = drop_index(&centers, press.local(), self.drag_x);
            drop_mark(press.group.1, press.local(), target)
                .map(|(at, side)| (press.group.0 + at, side))
        });
        let mut tiles = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let mut area = now_placement.tiles[index];
            let box_area = now_placement.items[index];
            let dragging = drag.is_some_and(|press| press.index == index);
            if let Some(press) = drag.filter(|_| dragging) {
                // Within its own group.
                let first = rest.items[press.group.0];
                let last = rest.items[press.group.0 + press.group.1 - 1];
                let rest_item = rest.items[index];
                area.x += drag_offset(
                    self.drag_x,
                    press.start_x,
                    rest_item.x,
                    rest_item.width,
                    first.x,
                    last.x + last.width,
                );
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
                drop: drop.and_then(|(at, side)| {
                    (at == index).then_some(match side {
                        dock_layout::DropSide::Before => box_area.x - 4.0 + 1.0,
                        dock_layout::DropSide::After => box_area.x + box_area.width + 4.0 - 1.0,
                    })
                }),
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
            Item::System(SystemItem::About) => Face::Glyph {
                glaze: theme::Glaze::ABOUT,
                paths: paint::ABOUT_GLYPH,
            },
            Item::System(SystemItem::Tray) => Face::Glyph {
                glaze: theme::Glaze::TRAY,
                paths: paint::TRAY_GLYPH,
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
        let hovered = if status { None } else { self.hit(x, y) };
        if hovered != self.hovered || status != self.status_hovered {
            self.hovered = hovered;
            self.status_hovered = status;
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
            self.drop_at(press, x);
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
        } else if let Some(index) = self.hit(x, y).filter(|&index| Some(index) == pressed) {
            self.click(index);
        }
        self.paint();
    }

    /// The group an item can be dragged in, as its first index and its size:
    /// the pinned programs, the widgets or the open programs. The system
    /// items stay where they are.
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
    fn drop_at(&mut self, press: Press, x: f64) {
        let rest = self.rest();
        let centers: Vec<f64> = rest.items[press.range()]
            .iter()
            .map(Area::center_x)
            .collect();
        let target = drop_index(&centers, press.local(), x);
        if target == press.local() {
            return;
        }
        let Some(item) = self.items.get(press.index).cloned() else {
            return;
        };
        self.effects.push(Box::new(move |app| match item {
            Item::App(pinned) => {
                let state = app.state::<Mutex<Storage>>();
                let id = pinned.id.as_str().to_string();
                if let Err(error) = crate::commands::apps::place_app(app.clone(), state, id, target)
                {
                    log::warn!("dock: placing a program failed: {error:?}");
                }
            }
            Item::Widget(widget) => {
                crate::commands::widgets::place_widget(app, widget.id.as_str(), target);
            }
            Item::Other(other) => super::open_apps::place_other(app, &other.key, target),
            Item::System(_) => {}
        }));
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
                if running {
                    self.effects.push(Box::new(move |app| {
                        if !super::open_apps::activate(app, &id) {
                            post(move |dock| dock.launch_pinned(key, id));
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
        self.effects.push(Box::new(move |app| {
            crate::commands::apps::launch_pinned(app, &id, move || {
                post(move |dock| {
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
                self.effects.push(Box::new(move |_| {
                    crate::commands::apps::launch_other(&other.key, move || {
                        post(move |dock| {
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

    // ---- What the app tells the dock ----

    fn set_open(&mut self, panel: Option<PanelId>) {
        if panel.is_some() {
            self.hovered = None;
            self.status_hovered = false;
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

fn local_now() -> windows::Win32::Foundation::SYSTEMTIME {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let mut handled = true;
    match message {
        WM_RUN => run_queue(),
        WM_MOUSEMOVE => with_dock(|dock| {
            let (x, y) = dock.point(lparam);
            dock.mouse_move(x, y);
        }),
        WM_MOUSELEAVE => with_dock(Dock::mouse_leave),
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => with_dock(|dock| {
            let (x, y) = dock.point(lparam);
            dock.left_down(x, y);
        }),
        WM_LBUTTONUP => with_dock(|dock| {
            let (x, y) = dock.point(lparam);
            dock.left_up(x, y);
        }),
        WM_MBUTTONDOWN => {
            shared().clicked = Some(Instant::now());
        }
        WM_MBUTTONUP => with_dock(|dock| {
            let (x, y) = dock.point(lparam);
            if let Some(index) = dock.hit(x, y) {
                dock.middle_click(index);
            }
        }),
        WM_RBUTTONUP => with_dock(|dock| {
            shared().clicked = Some(Instant::now());
            let (x, y) = dock.point(lparam);
            if let Some(index) = dock.hit(x, y) {
                dock.right_click(index);
            }
        }),
        WM_CAPTURECHANGED => with_dock(|dock| {
            // The capture went elsewhere: the drag is over.
            if dock.press.take().is_some() {
                dock.pressed = None;
                dock.animate();
                dock.paint();
            }
        }),
        WM_TIMER => match wparam.0 {
            ANIMATION_TIMER => with_dock(Dock::step),
            CLOCK_TIMER => with_dock(|dock| {
                dock.paint();
                dock.arm_clock();
            }),
            WARM_TIMER => with_dock(|dock| {
                let _ = unsafe { KillTimer(Some(dock.hwnd), WARM_TIMER) };
                if dock.warmable() && dock.open.is_none() {
                    dock.effects.push(Box::new(super::panel_window::warm));
                }
            }),
            _ => handled = false,
        },
        WM_SETTINGCHANGE => with_dock(|dock| {
            dock.dark = theme::system_dark();
            dock.motion = system_motion();
            dock.place();
            dock.paint();
        }),
        WM_DISPLAYCHANGE | WM_DPICHANGED => with_dock(|dock| {
            dock.place();
            dock.paint();
        }),
        // Alt+F4 hides the dock instead of closing it.
        WM_CLOSE => with_dock(Dock::hide),
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

/// The data changed (programs, widgets, open programs, settings): reads it
/// again.
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

pub fn show() {
    post(Dock::show);
}

/// What the E2E suite reads of the dock (test build only): each item's name
/// and what its face draws, and whether it is painted dark.
#[cfg(feature = "e2e")]
pub struct View {
    pub items: Vec<(String, &'static str)>,
    pub dark: bool,
}

/// Asks the dock for its `View`, from a thread other than the main one.
#[cfg(feature = "e2e")]
pub fn view() -> Option<View> {
    let (sent, received) = std::sync::mpsc::channel();
    post(move |dock| {
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

/// Presses the item named `name`, as a click would (test build only, from a
/// thread other than the main one). `false` when the dock has no such item.
#[cfg(feature = "e2e")]
pub fn press_named(name: String) -> bool {
    let (sent, received) = std::sync::mpsc::channel();
    post(move |dock| {
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
        let _ = sent.send(key.is_some());
    });
    received
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or(false)
}

/// The monitors or the band changed under the dock: places it again.
pub fn reposition() {
    post(|dock| {
        dock.place();
        dock.paint();
    });
}

pub fn hide() {
    post(Dock::hide);
}

pub fn set_open(panel: Option<PanelId>) {
    post(move |dock| dock.set_open(panel));
}

/// The settings moved the dock: lays it out again now and, once placed,
/// reopens the panel the move closed.
pub fn settings_moved(app: &AppHandle) {
    let app = app.clone();
    post(move |dock| {
        dock.reload();
        let reopen = super::panel::lock(&app).dock_settled(false, Instant::now());
        if let Some(panel) = reopen {
            dock.request(panel);
        }
    });
}

pub fn is_visible() -> bool {
    shared().visible
}

/// The dock window's origin on the screen (physical pixels), its scale and
/// the bar on the screen: where the panels anchor.
pub fn geometry() -> Option<((i32, i32), f64, Rect)> {
    let shared = shared();
    let window = shared.window?;
    Some(((window.x, window.y), shared.scale, shared.bar?))
}

/// Whether the user clicked the dock a moment ago: the panel that click
/// opens may take the focus.
pub fn clicked_recently() -> bool {
    shared()
        .clicked
        .is_some_and(|at| at.elapsed() < CLICK_GRANTS_FOCUS)
}

#[cfg(test)]
mod tests {
    use super::*;

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
