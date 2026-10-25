use std::error::Error;
use std::fmt;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use serde::Deserialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};

use super::hit_test::{self, HitRegion, RegionError};
use super::panel::{self, LogicalRect, LogicalSize, MAX_LOGICAL, PanelId, PanelPayload, SizeError};
use crate::commands;
use crate::domain::settings::{DockEdge, DockSettings};

pub const LABEL: &str = "dock";

/// Always layered: the dock toggles its mouse capture, and tao would make
/// it stop being layered (see `native::Layering::Always`).
#[cfg(windows)]
pub const LAYERING: super::native::Layering = super::native::Layering::Always;

/// Event sent to the dock asking it to open a panel anchored to its own
/// item, with `{ "panelId": <id> }`: the tray's "Configurações", and the
/// panel closed by a settings change that moved the dock, once it settled.
pub const REQUEST_PANEL_EVENT: &str = "dock-request-panel";

/// Gap between the dock window (and the bar, flush with its screen-facing
/// edge) and the work area edge, in logical pixels, before `bottomSpace` was
/// a setting: what the tests of the placing rules use.
#[cfg(test)]
const MARGIN: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Top,
}

impl From<DockEdge> for Edge {
    fn from(edge: DockEdge) -> Self {
        match edge {
            DockEdge::Bottom => Edge::Bottom,
            DockEdge::Top => Edge::Top,
        }
    }
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Whether the point lies inside (right and bottom edges excluded).
    pub fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x
            && y >= self.y
            && i64::from(x) < i64::from(self.x) + i64::from(self.width)
            && i64::from(y) < i64::from(self.y) + i64::from(self.height)
    }
}

/// Narrows to i32, saturating at its bounds.
pub fn saturate(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Physical rectangle of the dock window of `window_size` logical pixels:
/// centered horizontally in the work area, with its screen-facing side
/// (where the page lays the bar, flush) `MARGIN` away from the given edge.
/// The size is capped so the window stays inside the work area. Logical
/// values are rounded the same way Tauri converts logical to physical
/// pixels.
#[cfg(test)]
pub fn dock_rect(work_area: Rect, scale_factor: f64, window_size: (f64, f64), edge: Edge) -> Rect {
    dock_rect_with_margin(work_area, scale_factor, window_size, edge, MARGIN)
}

/// `dock_rect` with the distance from the edge (`bottomSpace`) as a setting,
/// in logical pixels.
pub fn dock_rect_with_margin(
    work_area: Rect,
    scale_factor: f64,
    window_size: (f64, f64),
    edge: Edge,
    margin: f64,
) -> Rect {
    // Every term fits in i32/u32 (`as` saturates), so sums in i64 can't
    // overflow.
    let margin = i64::from((margin * scale_factor).round() as i32);
    let max_height =
        u32::try_from((i64::from(work_area.height) - margin).max(0)).unwrap_or(work_area.height);
    let width = ((window_size.0 * scale_factor).round() as u32).min(work_area.width);
    let height = ((window_size.1 * scale_factor).round() as u32).min(max_height);
    let x = i64::from(work_area.x) + (i64::from(work_area.width) - i64::from(width)).div_euclid(2);
    let y = match edge {
        Edge::Bottom => {
            i64::from(work_area.y) + i64::from(work_area.height) - margin - i64::from(height)
        }
        Edge::Top => i64::from(work_area.y) + margin,
    };
    Rect {
        x: saturate(x),
        y: saturate(y),
        width,
        height,
    }
}

/// Physical rectangle of the dock window when the dock holds a band of its
/// own along the edge (an app bar, ADR 0009): centered across the monitor,
/// with its screen-facing side `margin` logical pixels inside the band's
/// outer edge. The window is taller than the band (it holds the
/// magnification and the label) and overlays what is beyond it, so only the
/// monitor, not the band, caps its height.
pub fn docked_rect(
    band: Rect,
    monitor: Rect,
    scale_factor: f64,
    window_size: (f64, f64),
    edge: Edge,
    margin: f64,
) -> Rect {
    let margin = i64::from((margin * scale_factor).round() as i32);
    let max_height =
        u32::try_from((i64::from(monitor.height) - margin).max(0)).unwrap_or(monitor.height);
    let width = ((window_size.0 * scale_factor).round() as u32).min(monitor.width);
    let height = ((window_size.1 * scale_factor).round() as u32).min(max_height);
    let x = i64::from(monitor.x) + (i64::from(monitor.width) - i64::from(width)).div_euclid(2);
    let y = match edge {
        Edge::Bottom => i64::from(band.y) + i64::from(band.height) - margin - i64::from(height),
        Edge::Top => i64::from(band.y) + margin,
    };
    Rect {
        x: saturate(x),
        y: saturate(y),
        width,
        height,
    }
}

/// The height of the bar before the page reports its layout, in logical
/// pixels: the icon and the padding the size presets use (`SIZE_GEOMETRY`
/// in `src/dock/layout.ts`: 40 + 2 × 6, 48 + 2 × 7, 64 + 2 × 9).
fn estimated_bar_height(icon_size: u16) -> f64 {
    let icon = f64::from(icon_size);
    icon + 2.0 * (icon * 7.0 / 48.0).round()
}

/// Largest icon the dock draws, in logical pixels.
const MAX_ICON: u32 = crate::domain::settings::MAX_ICON_SIZE as u32;

/// Padding around the icons in the bar, at most, in whole logical pixels:
/// the presets use about 0.14 of the icon (DESIGN.md "Layout"), 0.15
/// rounded up leaves room.
const MAX_PAD: u32 = (MAX_ICON * 3).div_ceil(20);

/// Tallest bar the page may report, in logical pixels: the largest icon
/// and its padding.
pub const MAX_BAR_HEIGHT: f64 = (MAX_ICON + 2 * MAX_PAD) as f64;

/// Tallest window the page may ask for, in logical pixels, by the D1
/// formula (`dockWindowSize` in `src/dock/layout.ts`): the padding, the
/// largest icon magnified 1.4 (rounded up) and lifted 6 px, the label pill
/// (10 px gap, 24 px tall) and 8 px of room. Across, the window is capped
/// to the work area by `dock_rect`, and the hit test clips the bar to the
/// window.
pub const MAX_WINDOW_HEIGHT: f64 = (MAX_PAD + (MAX_ICON * 7).div_ceil(5) + 6 + 10 + 24 + 8) as f64;

/// What the dock page reports whenever its layout changes
/// (`DockLayout` in `src/core/dock.ts`), in logical pixels relative to its
/// client area: where it paints (the bar and the magnified items) and the
/// window size its content needs (bar, magnification and label).
///
/// Read from the frontend only through `validate`: a layout that fails it
/// is rejected by the deserializer, before `set_dock_layout` runs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawDockLayout")]
pub struct DockLayout {
    pub bar: LogicalRect,
    pub hold: Vec<LogicalRect>,
    pub window: LogicalSize,
}

/// A `DockLayout` as the frontend sends it; its rects and size are checked
/// on their own as they are read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDockLayout {
    bar: LogicalRect,
    hold: Vec<LogicalRect>,
    window: LogicalSize,
}

impl TryFrom<RawDockLayout> for DockLayout {
    type Error = LayoutError;

    fn try_from(RawDockLayout { bar, hold, window }: RawDockLayout) -> Result<Self, LayoutError> {
        let layout = DockLayout { bar, hold, window };
        layout.validate()?;
        Ok(layout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    Region(RegionError),
    Window(SizeError),
    BarTooTall,
    WindowTooTall,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::Region(error) => write!(f, "{error}"),
            LayoutError::Window(error) => write!(f, "window: {error}"),
            LayoutError::BarTooTall => {
                write!(f, "bar: taller than {MAX_BAR_HEIGHT} logical pixels")
            }
            LayoutError::WindowTooTall => {
                write!(f, "window: taller than {MAX_WINDOW_HEIGHT} logical pixels")
            }
        }
    }
}

impl Error for LayoutError {}

impl DockLayout {
    /// Checks a layout received from the dock page: every rect valid, at
    /// most `MAX_HOLD_RECTS` hold rects, a bar at most `MAX_BAR_HEIGHT`
    /// tall and a window size from 1 to `MAX_LOGICAL` logical pixels, at
    /// most `MAX_WINDOW_HEIGHT` tall.
    fn validate(&self) -> Result<(), LayoutError> {
        hit_test::validate_region(&self.bar, &self.hold).map_err(LayoutError::Region)?;
        self.window
            .validate(MAX_LOGICAL)
            .map_err(LayoutError::Window)?;
        if self.bar.height > MAX_BAR_HEIGHT {
            return Err(LayoutError::BarTooTall);
        }
        if self.window.height > MAX_WINDOW_HEIGHT {
            return Err(LayoutError::WindowTooTall);
        }
        Ok(())
    }

    /// The window size to give the dock, in logical pixels: what the page
    /// asked for, grown to the bar's own size so the bar always fits.
    /// Not to where the bar ends: the page lays the bar flush with the
    /// window it is in now, so its far edge is that window's edge, and
    /// growing to it would keep the window from ever shrinking.
    /// `dock_rect` caps it to the work area.
    pub fn window_size(&self) -> (f64, f64) {
        (
            self.window.width.max(self.bar.width),
            self.window.height.max(self.bar.height),
        )
    }

    fn into_region(self) -> HitRegion {
        HitRegion {
            bar: self.bar,
            hold: self.hold,
        }
    }
}

/// Where the dock window was put, so a layout report that changes nothing
/// doesn't move or resize it.
#[derive(Debug, Default)]
pub struct Placement {
    /// Window size the page asked for, in logical pixels; until its first
    /// report, the size declared in tauri.conf.json.
    size: Option<(f64, f64)>,
    /// Physical rectangle last given to the window.
    applied: Option<Rect>,
    /// Height of the bar the page reported, in logical pixels: the band an
    /// app bar reserves is as thick as this and the distance from the edge.
    bar_height: Option<f64>,
}

impl Placement {
    /// Whether the window must be moved or resized to `rect`.
    fn needs(&self, rect: Rect, force: bool) -> bool {
        force || self.applied != Some(rect)
    }
}

fn placement(app: &AppHandle) -> MutexGuard<'_, Placement> {
    app.state::<Mutex<Placement>>()
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The edge the dock sits on, from the settings.
pub fn edge(app: &AppHandle) -> Edge {
    commands::settings::current(app).dock.edge.into()
}

/// Whether a settings change moves the dock or its items: then the open
/// panel, anchored to an item, is closed and reopened at the item's new
/// place.
pub fn moves_the_dock(before: &DockSettings, after: &DockSettings) -> bool {
    before.edge != after.edge
        || before.icon_size != after.icon_size
        || before.taskbar != after.taskbar
        || before.bottom_space != after.bottom_space
}

/// Shows the dock on the primary monitor. Closing it (Alt+F4) hides it.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
    hit_test::start(app, &window)?;
    #[cfg(windows)]
    {
        super::native::install(&window, Some(reposition), LAYERING)?;
        // A previous run that was killed may have left the taskbar changed.
        if let Some(path) = taskbar_state_path(app) {
            super::taskbar::recover(&path);
        }
        apply_taskbar(app, commands::settings::current(app).dock.taskbar);
    }
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Err(error) = hide(&handle) {
                log::warn!("dock: failed to hide: {error}");
            }
        }
        // A program or a shortcut dropped on the bar is pinned (spec 0006
        // B2): the window hears the drop itself, so the page has no say in
        // what is pinned.
        WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
            commands::apps::pin_dropped(&handle, paths.clone());
        }
        _ => {}
    });
    show(&window)
}

/// The dock page reported a new layout (validated): resizes and moves the
/// window when its size changed, and captures the mouse where it paints.
/// Once a layout leaves the window as it was (the page laid it out for the
/// window it is in), asks the dock to reopen the panel a settings change
/// closed.
pub fn set_layout(app: &AppHandle, layout: DockLayout) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing")?;
    {
        let mut placement = placement(app);
        placement.size = Some(layout.window_size());
        placement.bar_height = Some(layout.bar.height);
    }
    let placed = place(&window, false);
    // Even when placing failed: the region is relative to the window.
    hit_test::tracker(app).set_region(layout.into_region());
    let window_changed = placed?;
    let reopen = panel::lock(app).dock_settled(window_changed, Instant::now());
    if let Some(panel_id) = reopen {
        log::debug!("dock: settled after a move, reopening the panel");
        request_panel(app, panel_id)?;
    }
    Ok(())
}

/// Asks the dock to open a panel anchored to its own item, as a click
/// would (unless that panel is already open).
pub fn request_panel(app: &AppHandle, panel_id: PanelId) -> tauri::Result<()> {
    app.emit_to(LABEL, REQUEST_PANEL_EVENT, PanelPayload { panel_id })
}

/// The settings changed: when the dock moves (edge) or its items resize,
/// closes the open panel, whose anchor moved, and places the dock again.
/// The page reports the new layout after applying the settings; once the
/// dock settles there, `set_layout` has the panel reopened at its item.
pub fn settings_changed(app: &AppHandle, before: &DockSettings, after: &DockSettings) {
    if !moves_the_dock(before, after) {
        return;
    }
    #[cfg(windows)]
    if before.taskbar != after.taskbar {
        apply_taskbar(app, after.taskbar);
    }
    panel::close_for_move(app);
    let placed = app
        .get_webview_window(LABEL)
        .ok_or_else(|| Box::<dyn Error>::from("dock window is missing"))
        .and_then(|window| place(&window, false));
    if let Err(error) = placed {
        log::warn!("dock: failed to place after a settings change: {error}");
    }
}

/// Whether the dock window is on screen. `false` when it can't be told.
pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// Hides the dock (and its panel) when visible, shows it otherwise.
pub fn toggle(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing")?;
    if window.is_visible()? {
        hide(app)
    } else {
        show(&window)
    }
}

/// Shows the dock if it is hidden (the app was launched again, or the tray
/// asked for a panel). Like every show, it doesn't take the foreground.
pub fn reveal(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("dock window is missing")?;
    if window.is_visible()? {
        return Ok(());
    }
    show(&window)
}

/// Places the dock and shows it without taking the foreground from the app
/// in use.
fn show(window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    // The work area may have changed while hidden.
    place(window, true)?;
    // tao shows windows with SW_SHOW, which activates them unless they
    // can't take focus.
    window.set_focusable(false)?;
    let shown = window.show();
    // Restored even when showing failed.
    let restored = window.set_focusable(true);
    shown?;
    restored?;
    hit_test::tracker(window.app_handle()).set_visible(true);
    Ok(())
}

/// Hides the dock and the open panel.
fn hide(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let action = panel::lock(app).hide();
    panel::apply(app, action)?;
    app.get_webview_window(LABEL)
        .ok_or("dock window is missing")?
        .hide()?;
    hit_test::tracker(app).set_visible(false);
    Ok(())
}

#[cfg(windows)]
fn reposition(window: &WebviewWindow) {
    // Forced: on a scale change tao has already moved the window itself.
    if let Err(error) = place(window, true) {
        log::warn!("dock: failed to reposition: {error}");
    }
}

/// Moves and sizes the dock on the edge of the primary monitor's work area
/// the settings choose. Unless `force`d, leaves the window alone when it
/// already has that rectangle. Returns whether the window was moved or
/// resized.
fn place(window: &WebviewWindow, force: bool) -> Result<bool, Box<dyn Error>> {
    let app = window.app_handle();
    let (work_area, scale_factor) = primary_work_area(window)?;
    let size = placement(app).size;
    let size = match size {
        Some(size) => size,
        None => initial_size(app)?,
    };
    let settings = commands::settings::current(app).dock;
    let edge = Edge::from(settings.edge);
    let margin = f64::from(settings.bottom_space);
    #[cfg(windows)]
    let held = reserve_edge(window, &settings, scale_factor);
    #[cfg(not(windows))]
    let held: Option<(Rect, Rect)> = None;
    let rect = match held {
        // The dock holds a band of its own: it sits in it.
        Some((band, monitor)) => docked_rect(band, monitor, scale_factor, size, edge, margin),
        None => dock_rect_with_margin(work_area, scale_factor, size, edge, margin),
    };
    if !placement(app).needs(rect, force) {
        return Ok(false);
    }
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    placement(app).applied = Some(rect);
    let origin = window.inner_position()?;
    let size = window.inner_size()?;
    hit_test::tracker(app).set_window(
        Rect {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        },
        window.scale_factor()?,
    );
    Ok(true)
}

/// Where the settings keep the taskbar's state file, in the app's local
/// data folder; `None` when the folder can't be resolved (then the taskbar is
/// never changed).
#[cfg(windows)]
fn taskbar_state_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join(super::taskbar::STATE_FILE))
}

/// Takes the taskbar the way `mode` says, or gives it back. A failure is
/// logged and leaves the taskbar as it was: the dock then sits above it.
#[cfg(windows)]
fn apply_taskbar(app: &AppHandle, mode: crate::domain::settings::TaskbarMode) {
    use crate::domain::settings::TaskbarMode;
    // The test build runs on the machine of whoever tests: it never touches
    // the taskbar, whatever the default is.
    if cfg!(feature = "e2e") {
        return;
    }
    if mode == TaskbarMode::Visible && super::taskbar::is_held() {
        commands::tray::stop(app);
        super::taskbar::release();
        return;
    }
    if mode == TaskbarMode::Visible {
        return;
    }
    let Some(path) = taskbar_state_path(app) else {
        log::warn!("taskbar: no data folder, the taskbar stays");
        return;
    };
    if let Err(error) = super::taskbar::hold(mode, &path) {
        log::warn!("taskbar: could not be taken: {error}");
        return;
    }
    // The notification area is the dock's only while the taskbar is hidden
    // altogether: with it hiding on its own, its own tray is a hover away.
    if mode == TaskbarMode::Hidden {
        commands::tray::start(app);
    } else {
        commands::tray::stop(app);
    }
}

/// With the taskbar taken, reserves the band along the edge of the primary
/// monitor and returns it with the monitor's rectangle; `None` (and the band
/// given back) when the taskbar stays or the band could not be had.
#[cfg(windows)]
fn reserve_edge(
    window: &WebviewWindow,
    settings: &DockSettings,
    scale_factor: f64,
) -> Option<(Rect, Rect)> {
    use super::appbar;
    if !settings.taskbar.reserves_the_edge() || !super::taskbar::is_held() {
        appbar::set(None);
        return None;
    }
    let app = window.app_handle();
    {
        let app = app.clone();
        appbar::start(move || {
            // From the app bar's thread: only queues the work for the main
            // one, which the dock's own placing waits on.
            let again = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(window) = again.get_webview_window(LABEL) {
                    reposition(&window);
                }
            });
        });
    }
    let monitor = window.primary_monitor().ok()??;
    let monitor = Rect {
        x: monitor.position().x,
        y: monitor.position().y,
        width: monitor.size().width,
        height: monitor.size().height,
    };
    let bar_height = placement(app)
        .bar_height
        .unwrap_or_else(|| estimated_bar_height(settings.icon_size));
    let thickness =
        appbar::band_thickness(bar_height, f64::from(settings.bottom_space), scale_factor);
    let granted = appbar::set(Some(appbar::Request {
        edge: Edge::from(settings.edge),
        monitor,
        thickness,
    }))?;
    Some((granted, monitor))
}

/// The primary monitor's work area, in physical pixels, and its scale factor.
pub fn primary_work_area(window: &WebviewWindow) -> Result<(Rect, f64), Box<dyn Error>> {
    let monitor = window
        .primary_monitor()?
        .ok_or("no primary monitor found")?;
    let work_area = monitor.work_area();
    let work_area = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    };
    Ok((work_area, monitor.scale_factor()))
}

/// The window size before the page reports its layout, in logical pixels:
/// the dock size declared in tauri.conf.json.
fn initial_size(app: &AppHandle) -> Result<(f64, f64), Box<dyn Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == LABEL)
        .ok_or("dock window is missing from tauri.conf.json")?;
    Ok((config.width, config.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window of 240x104 logical pixels: a 64 px bar plus 40 px for the
    /// magnification above it.
    const SIZE: (f64, f64) = (240.0, 104.0);

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn bottom_at_100_percent_puts_the_window_bottom_8px_above_the_work_area() {
        let work_area = rect(0, 0, 1920, 1032);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(840, 1032 - 8 - 104, 240, 104));
        // The bar is flush with the window bottom.
        assert_eq!(dock.y + dock.height as i32, 1032 - 8);
    }

    #[test]
    fn bottom_at_125_percent_scales_size_and_margin() {
        let work_area = rect(0, 0, 2560, 1380);

        let dock = dock_rect(work_area, 1.25, SIZE, Edge::Bottom);

        // 104 * 1.25 = 130, 240 * 1.25 = 300, 8 * 1.25 = 10.
        assert_eq!(dock, rect(1130, 1380 - 10 - 130, 300, 130));
    }

    #[test]
    fn bottom_at_150_percent_scales_size_and_margin() {
        let work_area = rect(0, 0, 2560, 1368);

        let dock = dock_rect(work_area, 1.5, SIZE, Edge::Bottom);

        // 104 * 1.5 = 156, 8 * 1.5 = 12.
        assert_eq!(dock, rect(1100, 1368 - 12 - 156, 360, 156));
    }

    #[test]
    fn top_puts_the_window_top_8px_below_the_work_area_top_at_every_scale() {
        let work_area = rect(0, 0, 1920, 1032);

        // The bar is flush with the window top; the rest hangs below it.
        assert_eq!(
            dock_rect(work_area, 1.0, SIZE, Edge::Top),
            rect(840, 8, 240, 104)
        );
        assert_eq!(
            dock_rect(work_area, 1.25, SIZE, Edge::Top),
            rect(810, 10, 300, 130)
        );
        assert_eq!(
            dock_rect(work_area, 1.5, SIZE, Edge::Top),
            rect(780, 12, 360, 156)
        );
    }

    #[test]
    fn window_size_follows_the_layout() {
        let work_area = rect(0, 0, 1920, 1032);

        // A large dock with its label: 412x148 at 125%.
        let dock = dock_rect(work_area, 1.25, (412.0, 148.0), Edge::Bottom);

        assert_eq!(dock, rect((1920 - 515) / 2, 1032 - 10 - 185, 515, 185));
    }

    #[test]
    fn window_larger_than_the_work_area_is_capped_inside_it() {
        let work_area = rect(100, 50, 800, 600);

        let bottom = dock_rect(work_area, 1.0, (5000.0, 5000.0), Edge::Bottom);
        let top = dock_rect(work_area, 1.0, (5000.0, 5000.0), Edge::Top);

        assert_eq!(bottom, rect(100, 50, 800, 592));
        assert_eq!(top, rect(100, 58, 800, 592));
    }

    #[test]
    fn work_area_offset_by_a_top_taskbar_moves_the_dock_with_it() {
        let work_area = rect(0, 48, 1920, 1032);

        assert_eq!(dock_rect(work_area, 1.0, SIZE, Edge::Top).y, 56);
        assert_eq!(
            dock_rect(work_area, 1.0, SIZE, Edge::Bottom).y,
            48 + 1032 - 8 - 104
        );
    }

    #[test]
    fn work_area_offset_by_a_left_taskbar_centers_within_the_work_area() {
        let work_area = rect(62, 0, 1858, 1080);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock.x, 62 + (1858 - 240) / 2);
        assert_eq!(dock.x + dock.width as i32 / 2, 62 + 1858 / 2);
    }

    #[test]
    fn negative_work_area_origin_is_supported() {
        let work_area = rect(-1920, -200, 1920, 1040);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock, rect(-1920 + 840, -200 + 1040 - 8 - 104, 240, 104));
    }

    #[test]
    fn fractional_physical_sizes_round_half_away_from_zero() {
        let work_area = rect(0, 0, 1000, 1000);

        // 161 * 1.25 = 201.25 -> 201, 103 * 1.25 = 128.75 -> 129,
        // 161 * 1.5 = 241.5 -> 242, 103 * 1.5 = 154.5 -> 155,
        // 8 * 1.75 = 14, 104 * 1.75 = 182.
        let dock = dock_rect(work_area, 1.25, (161.0, 103.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (201, 129));

        let dock = dock_rect(work_area, 1.5, (161.0, 103.0), Edge::Bottom);
        assert_eq!((dock.width, dock.height), (242, 155));

        let dock = dock_rect(work_area, 1.75, SIZE, Edge::Bottom);
        assert_eq!(dock.y, 1000 - 14 - 182);
    }

    #[test]
    fn odd_leftover_width_rounds_the_x_offset_down() {
        let work_area = rect(0, 0, 1001, 1000);

        let dock = dock_rect(work_area, 1.0, SIZE, Edge::Bottom);

        assert_eq!(dock.x, 380);
    }

    #[test]
    fn extreme_work_areas_saturate_instead_of_overflowing() {
        let cases = [
            (rect(i32::MAX, i32::MAX, u32::MAX, u32::MAX), 1.0),
            (rect(i32::MIN, i32::MIN, 0, 0), 3.0),
            (rect(i32::MAX - 10, 0, 100, 100), 3.0),
            (rect(0, 0, 1920, 1080), f64::MAX),
            (rect(0, 0, 1920, 1080), f64::NAN),
            (rect(0, 0, 1920, 4), 1.0),
        ];
        for (work_area, scale_factor) in cases {
            for edge in [Edge::Bottom, Edge::Top] {
                for size in [SIZE, (f64::MAX, f64::MAX), (0.0, 0.0)] {
                    // Must not panic.
                    let _ = dock_rect(work_area, scale_factor, size, edge);
                }
            }
        }
        assert_eq!(
            dock_rect(rect(i32::MAX, 0, u32::MAX, 1000), 1.0, SIZE, Edge::Top).x,
            i32::MAX
        );
        // A work area thinner than the margin gives an empty window.
        assert_eq!(
            dock_rect(rect(0, 0, 1920, 4), 1.0, SIZE, Edge::Bottom).height,
            0
        );
    }

    #[test]
    fn rect_contains_excludes_the_right_and_bottom_edges() {
        let r = rect(-10, 20, 30, 40);

        assert!(r.contains((-10, 20)));
        assert!(r.contains((19, 59)));
        assert!(!r.contains((20, 30)));
        assert!(!r.contains((0, 60)));
        assert!(!r.contains((-11, 30)));
        assert!(!r.contains((0, 19)));
    }

    #[test]
    fn settings_edges_map_to_window_edges() {
        assert_eq!(Edge::from(DockEdge::Bottom), Edge::Bottom);
        assert_eq!(Edge::from(DockEdge::Top), Edge::Top);
        assert_eq!(Edge::from(DockEdge::default()), Edge::Bottom);
    }

    #[test]
    fn only_edge_and_size_changes_move_the_dock() {
        let before = DockSettings::default();
        let moved = |change: fn(&mut DockSettings)| {
            let mut after = before.clone();
            change(&mut after);
            moves_the_dock(&before, &after)
        };

        assert!(!moved(|_| {}));
        assert!(moved(|dock| dock.edge = DockEdge::Top));
        assert!(moved(|dock| dock.icon_size = 64));
        assert!(moved(
            |dock| dock.taskbar = crate::domain::settings::TaskbarMode::Visible
        ));
        assert!(moved(|dock| dock.bottom_space = 12));
        assert!(!moved(|dock| {
            dock.opacity = crate::domain::settings::DockOpacity::Solid
        }));
    }

    /// A 2560 × 1080 monitor with a band of 64 px at the bottom, as a bar of
    /// 62 px and 2 px from the edge reserve.
    const MONITOR: Rect = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1080,
    };
    const BAND: Rect = Rect {
        x: 0,
        y: 1016,
        width: 2560,
        height: 64,
    };

    #[test]
    fn docked_window_sits_in_the_band_with_its_bar_flush_to_the_margin() {
        // 240 × 104: the bar is the lower 62 px, the rest is room for the
        // magnification. The window's bottom is 2 px above the band's.
        let dock = docked_rect(BAND, MONITOR, 1.0, SIZE, Edge::Bottom, 2.0);

        assert_eq!(dock, rect(1160, 1080 - 2 - 104, 240, 104));
        assert_eq!(dock.y + dock.height as i32, BAND.y + BAND.height as i32 - 2);
    }

    #[test]
    fn docked_window_is_taller_than_the_band_and_overlays_what_is_beyond() {
        let dock = docked_rect(BAND, MONITOR, 1.0, SIZE, Edge::Bottom, 2.0);

        assert!(dock.height > BAND.height);
        assert!(dock.y < BAND.y);
    }

    #[test]
    fn docked_window_follows_the_band_a_taskbar_pushed_up() {
        // Another bar of 48 px holds the edge: Windows grants the band above it.
        let granted = Rect {
            y: BAND.y - 48,
            ..BAND
        };

        let dock = docked_rect(granted, MONITOR, 1.0, SIZE, Edge::Bottom, 2.0);

        assert_eq!(dock.y + dock.height as i32, 1080 - 48 - 2);
    }

    #[test]
    fn docked_window_on_top_hangs_from_the_band() {
        let band = Rect { y: 0, ..BAND };

        let dock = docked_rect(band, MONITOR, 1.0, SIZE, Edge::Top, 2.0);

        assert_eq!(dock.y, 2);
    }

    #[test]
    fn docked_window_is_centered_on_the_monitor_not_on_the_band() {
        let left = Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1200,
        };
        let band = Rect {
            y: 1136,
            height: 64,
            ..left
        };

        let dock = docked_rect(band, left, 1.0, SIZE, Edge::Bottom, 0.0);

        assert_eq!(dock.x, -1920 + (1920 - 240) / 2);
        assert_eq!(dock.y + dock.height as i32, 1200);
    }

    #[test]
    fn docked_window_scales_its_margin_and_its_size() {
        let band = Rect {
            y: 1080 - 96,
            height: 96,
            ..BAND
        };

        let dock = docked_rect(band, MONITOR, 1.5, SIZE, Edge::Bottom, 2.0);

        assert_eq!((dock.width, dock.height), (360, 156));
        assert_eq!(dock.y + dock.height as i32, 1080 - 3);
    }

    #[test]
    fn docked_window_never_leaves_the_monitor() {
        let dock = docked_rect(BAND, MONITOR, 1.0, (5000.0, 5000.0), Edge::Bottom, 2.0);

        assert_eq!(dock.width, 2560);
        assert_eq!(dock.height, 1080 - 2);
        assert!(dock.y >= 0);
    }

    #[test]
    fn the_bar_height_before_the_page_reports_matches_the_presets() {
        assert_eq!(estimated_bar_height(40), 52.0);
        assert_eq!(estimated_bar_height(48), 62.0);
        assert_eq!(estimated_bar_height(64), 82.0);
    }

    #[test]
    fn the_distance_from_the_edge_is_the_margin_of_the_window() {
        let work_area = rect(0, 0, 2560, 1040);
        for margin in [0.0, 2.0, 8.0, 64.0] {
            let dock = dock_rect_with_margin(work_area, 1.0, SIZE, Edge::Bottom, margin);
            assert_eq!(dock.y + dock.height as i32, 1040 - margin as i32);
        }
    }

    #[test]
    fn placement_moves_the_window_only_when_its_rect_changes() {
        let mut placement = Placement::default();
        let first = rect(840, 920, 240, 104);

        assert!(placement.needs(first, false));
        placement.applied = Some(first);
        assert!(!placement.needs(first, false));
        assert!(placement.needs(first, true));
        assert!(placement.needs(rect(840, 8, 240, 104), false));
        assert!(placement.needs(rect(830, 900, 260, 124), false));
    }

    fn logical(x: f64, y: f64, width: f64, height: f64) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width,
            height,
        }
    }

    const VALID: LogicalRect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    fn layout(hold: Vec<LogicalRect>, width: f64, height: f64) -> DockLayout {
        DockLayout {
            bar: VALID,
            hold,
            window: LogicalSize { width, height },
        }
    }

    #[test]
    fn layout_is_read_from_the_frontend_json() {
        let layout: DockLayout = serde_json::from_str(
            r#"{"bar":{"x":20,"y":40,"width":200,"height":64},
                "hold":[{"x":-4,"y":-13.2,"width":140,"height":67.2}],
                "window":{"width":240,"height":104}}"#,
        )
        .unwrap();

        assert_eq!(
            layout,
            DockLayout {
                bar: logical(20.0, 40.0, 200.0, 64.0),
                hold: vec![logical(-4.0, -13.2, 140.0, 67.2)],
                window: LogicalSize {
                    width: 240.0,
                    height: 104.0
                },
            }
        );
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn layout_with_missing_or_unknown_fields_is_rejected() {
        let cases = [
            // The old hit region, without the window size.
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1},"hold":[]}"#,
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1},"window":{"width":1,"height":1}}"#,
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1},"hold":[],
                "window":{"width":1,"height":1},"edge":"top"}"#,
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1},"hold":[],
                "window":{"width":1,"height":1,"scale":2}}"#,
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1,"left":0},"hold":[],
                "window":{"width":1,"height":1}}"#,
            r#"{"bar":{"x":0,"y":0,"width":1,"height":1},
                "hold":[{"x":0,"y":0,"width":1,"height":1,"right":1}],
                "window":{"width":1,"height":1}}"#,
        ];
        for json in cases {
            assert!(serde_json::from_str::<DockLayout>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn invalid_layouts_are_rejected_when_deserialized() {
        let rect = r#"{"x":0,"y":0,"width":1,"height":1}"#;
        let hold = |count: usize| vec![rect; count].join(",");
        let cases = [
            // A rect or the window out of bounds.
            r#"{"bar":{"x":0,"y":0,"width":-1,"height":1},"hold":[],"window":{"width":1,"height":1}}"#
                .to_string(),
            format!(
                r#"{{"bar":{rect},"hold":[{{"x":1e12,"y":0,"width":1,"height":1}}],"window":{{"width":1,"height":1}}}}"#
            ),
            format!(r#"{{"bar":{rect},"hold":[],"window":{{"width":0,"height":1}}}}"#),
            format!(r#"{{"bar":{rect},"hold":[],"window":{{"width":1e300,"height":1}}}}"#),
            // Taller than the largest icon needs.
            format!(r#"{{"bar":{rect},"hold":[],"window":{{"width":240,"height":198.5}}}}"#),
            r#"{"bar":{"x":0,"y":0,"width":240,"height":126.5},"hold":[],"window":{"width":240,"height":190}}"#
                .to_string(),
            // Too many magnified icons.
            format!(
                r#"{{"bar":{rect},"hold":[{}],"window":{{"width":1,"height":1}}}}"#,
                hold(17)
            ),
            // Not a number, or an unknown field.
            format!(r#"{{"bar":{rect},"hold":[],"window":{{"width":"NaN","height":1}}}}"#),
            format!(r#"{{"bar":{rect},"hold":[],"window":{{"width":1,"height":1}},"edge":"top"}}"#),
        ];
        for json in &cases {
            assert!(serde_json::from_str::<DockLayout>(json).is_err(), "{json}");
        }
        let largest = format!(
            r#"{{"bar":{{"x":0,"y":0,"width":240,"height":{MAX_BAR_HEIGHT}}},"hold":[{}],"window":{{"width":100000,"height":{MAX_WINDOW_HEIGHT}}}}}"#,
            hold(16)
        );
        assert!(serde_json::from_str::<DockLayout>(&largest).is_ok());
    }

    #[test]
    fn layout_with_up_to_16_hold_rects_is_valid() {
        for count in [0, 1, hit_test::MAX_HOLD_RECTS] {
            assert_eq!(
                layout(vec![VALID; count], 240.0, 104.0).validate(),
                Ok(()),
                "{count} hold rects"
            );
        }
        assert_eq!(hit_test::MAX_HOLD_RECTS, 16);
    }

    #[test]
    fn layout_with_too_many_hold_rects_is_rejected() {
        let layout = layout(vec![VALID; hit_test::MAX_HOLD_RECTS + 1], 240.0, 104.0);

        assert_eq!(
            layout.validate(),
            Err(LayoutError::Region(RegionError::TooManyHoldRects))
        );
    }

    #[test]
    fn layout_with_an_invalid_rect_is_rejected() {
        use super::panel::RectError;
        let bad = LogicalRect {
            width: f64::NAN,
            ..VALID
        };
        let huge = LogicalRect { x: 1e12, ..VALID };

        assert_eq!(
            DockLayout {
                bar: bad,
                ..layout(vec![], 240.0, 104.0)
            }
            .validate(),
            Err(LayoutError::Region(RegionError::Bar(RectError::NotFinite)))
        );
        assert_eq!(
            layout(vec![VALID, huge], 240.0, 104.0).validate(),
            Err(LayoutError::Region(RegionError::Hold(
                1,
                RectError::TooLarge
            )))
        );
    }

    #[test]
    fn layout_with_a_bad_window_size_is_rejected() {
        let cases = [
            (f64::NAN, 104.0, SizeError::NotFinite),
            (240.0, f64::INFINITY, SizeError::NotFinite),
            (0.0, 104.0, SizeError::NotPositive),
            (240.0, -104.0, SizeError::NotPositive),
            (240.0, 0.5, SizeError::NotPositive),
            (MAX_LOGICAL + 1.0, 104.0, SizeError::TooLarge),
            (240.0, f64::MAX, SizeError::TooLarge),
        ];
        for (width, height, expected) in cases {
            assert_eq!(
                layout(vec![], width, height).validate(),
                Err(LayoutError::Window(expected)),
                "{width}x{height}"
            );
        }
    }

    #[test]
    fn dock_height_limits_follow_the_largest_icon() {
        // D1 (src/dock/layout.ts) for a 96 px icon: a padding of 0.15 of
        // it (14.4, up to 15), magnified 1.4 (134.4, up to 135) and lifted
        // 6 px, the label pill (10 + 24) and the room around it (8).
        assert_eq!(crate::domain::settings::MAX_ICON_SIZE, 96);
        assert_eq!(MAX_BAR_HEIGHT, 96.0 + 2.0 * 15.0);
        assert_eq!(MAX_WINDOW_HEIGHT, 15.0 + 135.0 + 6.0 + 10.0 + 24.0 + 8.0);
    }

    #[test]
    fn height_limits_match_the_frontend_contract() {
        // src/core/dock.ts repeats these limits so layout.test.ts can keep
        // every preset within them; both sides must agree.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/core/dock.ts");
        let source = std::fs::read_to_string(&path).expect("src/core/dock.ts must exist");
        let value = |name: &str| -> f64 {
            source
                .split(&format!("export const {name} = "))
                .nth(1)
                .and_then(|rest| rest.split(';').next())
                .and_then(|number| number.trim().parse().ok())
                .unwrap_or_else(|| panic!("src/core/dock.ts: `{name}` not found"))
        };

        assert_eq!(value("MAX_DOCK_BAR_HEIGHT"), MAX_BAR_HEIGHT);
        assert_eq!(value("MAX_DOCK_WINDOW_HEIGHT"), MAX_WINDOW_HEIGHT);
    }

    #[test]
    fn layout_of_every_preset_is_within_the_limits() {
        // Grande, the largest preset: a 82 px bar in a 147 px window.
        let large = DockLayout {
            bar: logical(12.0, 65.0, 166.0, 82.0),
            ..layout(vec![], 190.0, 147.0)
        };

        assert_eq!(large.validate(), Ok(()));
    }

    #[test]
    fn layout_taller_than_the_largest_icon_needs_is_rejected() {
        let tall_window = layout(vec![], 240.0, MAX_WINDOW_HEIGHT + 1.0);
        let tall_bar = DockLayout {
            bar: logical(0.0, 0.0, 240.0, MAX_BAR_HEIGHT + 1.0),
            ..layout(vec![], 240.0, 104.0)
        };

        assert_eq!(tall_window.validate(), Err(LayoutError::WindowTooTall));
        assert_eq!(tall_bar.validate(), Err(LayoutError::BarTooTall));
        // Wide is fine: `dock_rect` caps the window to the work area, and
        // the hit test clips the bar to the window.
        assert_eq!(
            DockLayout {
                bar: logical(0.0, 0.0, 50_000.0, 64.0),
                ..layout(vec![], MAX_LOGICAL, MAX_WINDOW_HEIGHT)
            }
            .validate(),
            Ok(())
        );
    }

    /// What the page reports in a window whose client area is `viewport`:
    /// the bar flush with the side facing the screen edge and centered
    /// across (`placeDock` in `src/dock/layout.ts`), and the window it
    /// needs.
    fn page_layout(
        viewport: (f64, f64),
        bar: (f64, f64),
        needed: (f64, f64),
        edge: Edge,
    ) -> DockLayout {
        DockLayout {
            bar: logical(
                (viewport.0 - bar.0) / 2.0,
                match edge {
                    Edge::Bottom => viewport.1 - bar.1,
                    Edge::Top => 0.0,
                },
                bar.0,
                bar.1,
            ),
            hold: vec![],
            window: LogicalSize {
                width: needed.0,
                height: needed.1,
            },
        }
    }

    #[test]
    fn window_reaches_the_size_the_page_needs_growing_and_shrinking() {
        // Grande and Pequeno (src/dock/layout.ts): bar and window needed.
        let large = ((166.0, 82.0), (190.0, 147.0));
        let small = ((104.0, 52.0), (128.0, 110.0));
        for edge in [Edge::Bottom, Edge::Top] {
            for (from, to) in [(large, small), (small, large)] {
                // The window is where the previous size left it; the page
                // reports for it, then for each window Rust gives it.
                let mut viewport = from.1;
                let mut steps = 0;
                loop {
                    let next = page_layout(viewport, to.0, to.1, edge).window_size();
                    if next == viewport {
                        break;
                    }
                    viewport = next;
                    steps += 1;
                    assert!(steps < 4, "{edge:?}: stuck at {viewport:?}");
                }
                assert_eq!(viewport, to.1, "{edge:?} from {:?}", from.1);
            }
        }
    }

    #[test]
    fn window_size_grows_to_the_bar_size() {
        let bar = logical(20.0, 40.0, 200.0, 64.0);
        let roomy = DockLayout {
            bar,
            ..layout(vec![], 240.0, 104.0)
        };
        let cramped = DockLayout {
            bar,
            ..layout(vec![], 100.0, 50.0)
        };

        assert_eq!(roomy.window_size(), (240.0, 104.0));
        assert_eq!(cramped.window_size(), (200.0, 64.0));
    }
}
