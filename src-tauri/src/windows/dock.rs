//! Where the docks sit and what the rest of the app asks of them (spec 0011
//! H3, spec 0010 D4b): each dock is a native window of `native_dock`, one per
//! monitor; this places each on its monitor's edge, holds that edge as an app
//! bar when the taskbar is taken, and shows, hides and tells them what
//! changed.

use std::error::Error;

use tauri::{AppHandle, Manager};

use super::native_dock;
use super::panel::{self, PanelId};
use crate::commands;
use crate::domain::settings::{DockEdge, DockSettings};

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
    #[cfg(test)]
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

/// Gap between the dock window (and the bar, flush with its screen-facing
/// edge) and the work area edge, in logical pixels, before `bottomSpace` was
/// a setting: what the tests of the placing rules use.
#[cfg(test)]
const MARGIN: f64 = 8.0;

/// Physical rectangle of the dock window of `window_size` logical pixels:
/// centered horizontally in the work area, with its screen-facing side
/// (where the bar sits, flush) `MARGIN` away from the given edge.
#[cfg(test)]
pub fn dock_rect(work_area: Rect, scale_factor: f64, window_size: (f64, f64), edge: Edge) -> Rect {
    dock_rect_with_margin(work_area, scale_factor, window_size, edge, MARGIN)
}

/// `dock_rect` with the distance from the edge (`bottomSpace`) as a setting,
/// in logical pixels. The size is capped so the window stays inside the work
/// area. Logical values are rounded half away from zero.
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
        || before.every_monitor != after.every_monitor
        || before.auto_hide != after.auto_hide
}

/// Makes the dock and shows it on the primary monitor. On the main thread.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    {
        // A previous run that was killed may have left the taskbar changed.
        if let Some(path) = taskbar_state_path(app) {
            super::taskbar::recover(&path);
        }
        apply_taskbar(app, commands::settings::current(app).dock.taskbar);
    }
    native_dock::start(app)?;
    native_dock::show();
    Ok(())
}

/// The programs, the widgets, the open programs or the settings changed: the
/// dock reads them again.
pub fn refresh() {
    native_dock::refresh();
}

/// The icon pack or its folder changed.
pub fn forget_icons() {
    native_dock::forget_icons();
}

/// The open panel changed: the dock marks its item and stops magnifying
/// while one is open.
pub fn set_open_panel(panel_id: Option<PanelId>) {
    native_dock::set_open(panel_id);
}

/// The settings changed: the dock reads them again; when it moves (edge,
/// size, taskbar, distance), the open panel, whose anchor moved, closes and
/// opens again at its item once the dock is placed.
pub fn settings_changed(app: &AppHandle, before: &DockSettings, after: &DockSettings) {
    if !moves_the_dock(before, after) {
        refresh();
        return;
    }
    #[cfg(windows)]
    if before.taskbar != after.taskbar {
        apply_taskbar(app, after.taskbar);
    }
    panel::close_for_move(app);
    native_dock::settings_moved(app);
}

/// A program is in full screen on the monitor `key` (`full`), or not any
/// more: that monitor's dock steps aside, and comes back after unless the user
/// hid the docks meanwhile (spec 0010 D4).
pub fn set_fullscreen(key: &str, full: bool) {
    native_dock::set_fullscreen(key.to_string(), full);
}

/// Whether the dock is on screen.
pub fn is_visible() -> bool {
    native_dock::is_visible()
}

/// Shows the dock (the app was launched again, or the tray asked for a
/// panel), bringing it up if it is hiding on its own. Like every show, it
/// doesn't take the foreground.
pub fn reveal(_app: &AppHandle) -> Result<(), Box<dyn Error>> {
    native_dock::show();
    Ok(())
}

/// The dock window's origin on the screen and its scale, and the bar on the
/// screen: what the panels anchor to.
pub fn geometry() -> Option<((i32, i32), f64, Rect)> {
    native_dock::geometry()
}

/// The work area of the monitor of the dock the panel anchors to, in physical
/// pixels, and its scale.
pub fn work_area() -> Option<(Rect, f64)> {
    native_dock::work_area()
}

/// Where the dock window of `size` logical pixels goes on `monitor`, with a
/// bar `bar_height` tall, and the monitor's scale: on the edge the settings
/// choose, in the band of its own when it holds the edge. Called by the dock
/// on the main thread.
#[cfg(windows)]
pub fn place_native(
    app: &AppHandle,
    settings: &DockSettings,
    monitor: &super::monitors::Monitor,
    size: (f64, f64),
    bar_height: f64,
) -> Option<(Rect, f64)> {
    let edge = Edge::from(settings.edge);
    let margin = f64::from(settings.bottom_space);
    let scale = monitor.scale;
    let rect = match reserve_edge(app, settings, monitor, bar_height) {
        Some(band) => docked_rect(band, monitor.rect, scale, size, edge, margin),
        None => dock_rect_with_margin(monitor.work, scale, size, edge, margin),
    };
    Some((rect, scale))
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

/// With the taskbar taken, reserves the band along the edge of `monitor`
/// and returns it; `None` (and the band given back) when the taskbar stays,
/// the dock hides on its own (its room is the programs') or the band could
/// not be had.
#[cfg(windows)]
fn reserve_edge(
    app: &AppHandle,
    settings: &DockSettings,
    monitor: &super::monitors::Monitor,
    bar_height: f64,
) -> Option<Rect> {
    use super::appbar;
    let _ = app;
    if !settings.taskbar.reserves_the_edge() || !super::taskbar::is_held() || settings.auto_hide {
        appbar::set(&monitor.key, None);
        return None;
    }
    // From the app bar's thread: only queues the work for the docks.
    appbar::start(native_dock::reposition);
    let thickness =
        appbar::band_thickness(bar_height, f64::from(settings.bottom_space), monitor.scale);
    appbar::set(
        &monitor.key,
        Some(appbar::Request {
            edge: Edge::from(settings.edge),
            monitor: monitor.rect,
            thickness,
        }),
    )
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
    fn the_distance_from_the_edge_is_the_margin_of_the_window() {
        let work_area = rect(0, 0, 2560, 1040);
        for margin in [0.0, 2.0, 8.0, 64.0] {
            let dock = dock_rect_with_margin(work_area, 1.0, SIZE, Edge::Bottom, margin);
            assert_eq!(dock.y + dock.height as i32, 1040 - margin as i32);
        }
    }
}
