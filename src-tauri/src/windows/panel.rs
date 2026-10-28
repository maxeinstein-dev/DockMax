use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};

use super::dock::{self, Edge, Rect};
use super::memory::{self, MemoryTarget};
use crate::domain::widgets::{WidgetId, WidgetKind};

pub const LABEL: &str = "panel";

/// Event sent to the panel window, while hidden, with the panel to render.
/// The page answers with `panel_ready` once it has.
const SHOW_EVENT: &str = "panel-show";

/// Gap between the panel's visible card and the dock bar, and between the
/// card and the work area edges, in logical pixels.
const MARGIN: f64 = 8.0;

/// How long after a blur-hide a toggle of the same panel still counts as
/// the click that closed it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// How long a panel closed because the dock moved waits for the dock to
/// settle before the reopen is dropped.
pub const REOPEN_TIMEOUT: Duration = Duration::from_secs(1);

/// How long the panel waits for its page to confirm it rendered before it
/// is shown anyway, with the last known size.
pub const READY_TIMEOUT: Duration = Duration::from_millis(400);

/// Largest panel window side accepted from the page, in logical pixels.
pub const MAX_PANEL_SIZE: f64 = 4096.0;

/// Largest transparent inset around the panel card accepted from the page,
/// in logical pixels: the gap between the card and the dock bar, so the
/// window (which blocks clicks) never covers the bar.
pub const MAX_INSET: f64 = MARGIN;

/// Which panel the dock asks for, as the pages send and get it:
/// `"settings"`, `"about"`, `"tray"`, `{ "widget": "<id>" }` or
/// `{ "app": "<id>" }`, the menu of a pinned program, or
/// `{ "open": "<key>" }`, the menu of an open program that is not pinned
/// (`PanelId` in `src/core/panels.ts`). The widget ID is checked when deserialized
/// ([`WidgetId::parse`]): a malformed one is refused before any command
/// runs. Whether the instance exists is checked by `toggle_panel`, which
/// turns the ID into a [`Panel`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelId {
    Settings,
    About,
    /// The notification area (ADR 0009, Fase E).
    Tray,
    Widget(WidgetId),
    /// The menu of a pinned program (spec 0006 §6): the ID is the pinned
    /// item's, checked like a widget's.
    App(WidgetId),
    /// The menu of an open program that is not pinned (spec 0009 D3): the key
    /// is the opaque one the dock was given for it, checked against the
    /// programs that are open.
    Open(String),
}

/// Transparent inset the panel page lays around the card, in logical
/// pixels (`--panel-inset` in `src/panel/Panel.css`, `PANEL_INSET` in
/// `src/panel/geometry.ts`).
const PAGE_INSET: f64 = 8.0;

/// A panel to open: its ID and, for a widget, the kind of the instance,
/// which fixes the size the panel is reserved before its page reports one.
/// `toggle_panel` builds it after checking the instance in the storage, so
/// the state never opens a panel for an instance that isn't there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    Settings,
    About,
    Tray,
    Widget {
        id: WidgetId,
        kind: WidgetKind,
    },
    /// The menu of the pinned program `id`.
    App {
        id: WidgetId,
    },
    /// The menu of the open program `key` that is not pinned.
    Open {
        key: String,
    },
}

impl Panel {
    /// The ID the pages know this panel by.
    pub fn id(&self) -> PanelId {
        match self {
            Panel::Settings => PanelId::Settings,
            Panel::About => PanelId::About,
            Panel::Tray => PanelId::Tray,
            Panel::Widget { id, .. } => PanelId::Widget(id.clone()),
            Panel::App { id } => PanelId::App(id.clone()),
            Panel::Open { key } => PanelId::Open(key.clone()),
        }
    }

    /// Whether this is the panel `id` names.
    fn is(&self, id: &PanelId) -> bool {
        match (self, id) {
            (Panel::Settings, PanelId::Settings)
            | (Panel::About, PanelId::About)
            | (Panel::Tray, PanelId::Tray) => true,
            (Panel::Widget { id: own, .. }, PanelId::Widget(other)) => own == other,
            (Panel::App { id: own }, PanelId::App(other)) => own == other,
            (Panel::Open { key: own }, PanelId::Open(other)) => own == other,
            _ => false,
        }
    }

    /// The visible card, in logical pixels: its width as `Panel.css` fixes
    /// it, and about the height its content takes in its main view (the
    /// mock, `docs/design/widgets-mock.html`).
    fn card_size(&self) -> (f64, f64) {
        match self {
            Panel::Settings => (320.0, 390.0),
            Panel::About => (300.0, 236.0),
            // Title, a row of icons, footer.
            Panel::Tray => (280.0, 150.0),
            // Title, a few entries, the actions, footer.
            Panel::App { .. } => (280.0, 300.0),
            // Title, three actions, footer.
            Panel::Open { .. } => (280.0, 190.0),
            Panel::Widget { kind, .. } => match kind {
                // Title, time and date, footer.
                WidgetKind::Clock => (300.0, 170.0),
                // Title, four periods, footer.
                WidgetKind::TimeProgress => (320.0, 284.0),
                // Title, the 210 px text field, footer.
                WidgetKind::Note => (340.0, 318.0),
                // Title, the two meters with their figures, footer.
                WidgetKind::Activity => (320.0, 300.0),
                // Title, the graph, the two rates, footer.
                WidgetKind::Network => (320.0, 300.0),
                // Title, the all-day strip, today's and tomorrow's meetings,
                // footer.
                WidgetKind::Calendar => (340.0, 440.0),
                // Never opened in this phase: `toggle_panel` refuses kinds
                // that can't be created. The Settings card, if one ever is.
                WidgetKind::Shortcut | WidgetKind::Files => (320.0, 390.0),
            },
        }
    }

    /// Size the panel window is shown with before its page ever reported
    /// one, in logical pixels: the card and `PAGE_INSET` on every side.
    pub fn logical_size(&self) -> (f64, f64) {
        let (width, height) = self.card_size();
        (width + 2.0 * PAGE_INSET, height + 2.0 * PAGE_INSET)
    }
}

/// Largest coordinate or size accepted from the frontend, in logical pixels.
/// Far beyond any real monitor layout, small enough that scaling it can't
/// leave the `i32` range.
pub const MAX_LOGICAL: f64 = 100_000.0;

/// A rectangle in logical pixels, relative to a window's client area.
/// Read from the frontend only through `validate`: a rectangle that fails
/// it is rejected by the deserializer, before any command runs.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "RawRect")]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A `LogicalRect` as the frontend sends it, before it is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl TryFrom<RawRect> for LogicalRect {
    type Error = RectError;

    fn try_from(
        RawRect {
            x,
            y,
            width,
            height,
        }: RawRect,
    ) -> Result<Self, RectError> {
        let rect = LogicalRect {
            x,
            y,
            width,
            height,
        };
        rect.validate()?;
        Ok(rect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectError {
    NotFinite,
    NegativeSize,
    TooLarge,
}

impl fmt::Display for RectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RectError::NotFinite => "a value is not a finite number",
            RectError::NegativeSize => "the width or height is negative",
            RectError::TooLarge => "a value is beyond 100000 logical pixels",
        })
    }
}

impl Error for RectError {}

impl LogicalRect {
    /// Checks a rectangle received from the frontend: finite values,
    /// non-negative size and every value within `MAX_LOGICAL`.
    pub fn validate(&self) -> Result<(), RectError> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(RectError::NotFinite);
        }
        if self.width < 0.0 || self.height < 0.0 {
            return Err(RectError::NegativeSize);
        }
        if values.iter().any(|value| value.abs() > MAX_LOGICAL) {
            return Err(RectError::TooLarge);
        }
        Ok(())
    }
}

/// A size in logical pixels. Read from the frontend only through
/// `validate` with `MAX_LOGICAL`: finite, from 1 to 100000 on each side.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "RawSize")]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

/// A `LogicalSize` as the frontend sends it, before it is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSize {
    width: f64,
    height: f64,
}

impl TryFrom<RawSize> for LogicalSize {
    type Error = SizeError;

    fn try_from(RawSize { width, height }: RawSize) -> Result<Self, SizeError> {
        let size = LogicalSize { width, height };
        size.validate(MAX_LOGICAL)?;
        Ok(size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeError {
    NotFinite,
    NotPositive,
    TooSmall,
    TooLarge,
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SizeError::NotFinite => "a value is not a finite number",
            SizeError::NotPositive => "the width or height is below 1 logical pixel",
            SizeError::TooSmall => "the width or height is below the limit",
            SizeError::TooLarge => "the width or height is beyond the limit",
        })
    }
}

impl Error for SizeError {}

impl LogicalSize {
    /// Checks a size received from the frontend: finite, at least 1 logical
    /// pixel and at most `max` on each side.
    pub fn validate(&self, max: f64) -> Result<(), SizeError> {
        let values = [self.width, self.height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(SizeError::NotFinite);
        }
        if values.iter().any(|&value| value < 1.0) {
            return Err(SizeError::NotPositive);
        }
        if values.iter().any(|&value| value > max) {
            return Err(SizeError::TooLarge);
        }
        Ok(())
    }
}

/// Smallest panel window side accepted from the page, in logical pixels:
/// whatever the inset, at least 1 logical pixel of card is left.
pub const MIN_PANEL_SIZE: f64 = 2.0 * MAX_INSET + 1.0;

/// The panel window size its page reports: a `LogicalSize` from
/// `MIN_PANEL_SIZE` to `MAX_PANEL_SIZE` on each side, checked when
/// deserialized.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "LogicalSize")]
pub struct PanelSize(LogicalSize);

impl TryFrom<LogicalSize> for PanelSize {
    type Error = SizeError;

    fn try_from(size: LogicalSize) -> Result<Self, SizeError> {
        size.validate(MAX_PANEL_SIZE)?;
        if size.width < MIN_PANEL_SIZE || size.height < MIN_PANEL_SIZE {
            return Err(SizeError::TooSmall);
        }
        Ok(PanelSize(size))
    }
}

/// The transparent inset around the panel card its page reports, in
/// logical pixels: finite, from 0 to `MAX_INSET`, checked when
/// deserialized.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "f64")]
pub struct PanelInset(f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsetError;

impl fmt::Display for InsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the inset is not a finite number from 0 to 8")
    }
}

impl Error for InsetError {}

impl TryFrom<f64> for PanelInset {
    type Error = InsetError;

    fn try_from(inset: f64) -> Result<Self, InsetError> {
        // False for NaN.
        if (0.0..=MAX_INSET).contains(&inset) {
            Ok(PanelInset(inset))
        } else {
            Err(InsetError)
        }
    }
}

/// The panel window as its page rendered it, in logical pixels: the window
/// size and the transparent inset around the visible card, on every side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelGeometry {
    pub size: LogicalSize,
    pub inset: f64,
}

impl PanelGeometry {
    /// A geometry from checked parts: its card is at least 1 logical pixel
    /// wide and tall, and its inset never wider than the gap to the bar.
    pub fn new(PanelSize(size): PanelSize, PanelInset(inset): PanelInset) -> Self {
        PanelGeometry { size, inset }
    }

    /// What the panel is shown with before its page ever reported a size:
    /// the geometry the page lays it out with, give or take the height.
    pub fn fallback(panel: &Panel) -> Self {
        let (width, height) = panel.logical_size();
        PanelGeometry {
            size: LogicalSize { width, height },
            inset: PAGE_INSET,
        }
    }
}

/// Converts a rectangle relative to a window's client area into physical
/// screen coordinates. Returns `None` for an invalid rectangle or scale, or
/// when the result leaves the `i32` range.
pub fn to_screen(rect: LogicalRect, client_origin: (i32, i32), scale_factor: f64) -> Option<Rect> {
    rect.validate().ok()?;
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return None;
    }
    // `as` saturates; the checked conversions reject what saturated.
    let scale = |value: f64| (value * scale_factor).round() as i64;
    Some(Rect {
        x: i32::try_from(i64::from(client_origin.0).checked_add(scale(rect.x))?).ok()?,
        y: i32::try_from(i64::from(client_origin.1).checked_add(scale(rect.y))?).ok()?,
        width: u32::try_from(scale(rect.width)).ok()?,
        height: u32::try_from(scale(rect.height)).ok()?,
    })
}

/// Physical rectangle of a panel window anchored to a dock item, or `None`
/// when the work area leaves no room for its card.
///
/// The visible card (the window minus `geometry.inset` on every side) sits
/// `MARGIN` away from the dock bar on the side away from the screen edge,
/// horizontally centered on the anchor. It is kept `MARGIN` inside the work
/// area, and shrunk when it doesn't fit: across, to the work area; along,
/// to the space between the bar (plus `MARGIN`) and the far work-area edge
/// (minus `MARGIN`). Only the transparent inset goes beyond the card, and
/// it is at most `MARGIN`, so the window never reaches the bar.
pub fn panel_rect(
    anchor: Rect,
    bar: Rect,
    work_area: Rect,
    scale_factor: f64,
    geometry: PanelGeometry,
    edge: Edge,
) -> Option<Rect> {
    // Every term fits in u32 or i32 (`as` saturates, NaN gives 0), so sums
    // in i64 can't overflow; the position is saturated back into i32.
    let physical = |value: f64| i64::from((value * scale_factor).round() as u32);
    let margin = physical(MARGIN);
    let width = physical(geometry.size.width);
    let height = physical(geometry.size.height);
    let inset = physical(geometry.inset)
        .min(margin)
        .min(width / 2)
        .min(height / 2);
    let end = |start: i32, length: u32| i64::from(start) + i64::from(length);
    let (left, right) = (
        i64::from(work_area.x) + margin,
        end(work_area.x, work_area.width) - margin,
    );
    let (top, bottom) = match edge {
        Edge::Bottom => (
            i64::from(work_area.y) + margin,
            (i64::from(bar.y) - margin).min(end(work_area.y, work_area.height) - margin),
        ),
        Edge::Top => (
            (end(bar.y, bar.height) + margin).max(i64::from(work_area.y) + margin),
            end(work_area.y, work_area.height) - margin,
        ),
    };
    let card_width = (width - 2 * inset).min(right - left);
    let card_height = (height - 2 * inset).min(bottom - top);
    if card_width < 1 || card_height < 1 {
        return None;
    }
    let anchor_center = i64::from(anchor.x) + i64::from(anchor.width).div_euclid(2);
    let card_x = (anchor_center - card_width.div_euclid(2)).clamp(left, right - card_width);
    let card_y = match edge {
        Edge::Bottom => bottom - card_height,
        Edge::Top => top,
    };
    Some(Rect {
        x: dock::saturate(card_x - inset),
        y: dock::saturate(card_y - inset),
        width: u32::try_from(card_width + 2 * inset).ok()?,
        height: u32::try_from(card_height + 2 * inset).ok()?,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Hide the window, ask its page to render the panel and wait for
    /// `panel_ready`, or `READY_TIMEOUT`. The number tells this wait from
    /// later ones, so a late time-out can't show a newer panel.
    Prepare(PanelId, u64),
    /// Place the window for the open panel and show it; when it is already
    /// on screen, only place it again.
    Reveal {
        anchor: LogicalRect,
        geometry: PanelGeometry,
    },
    Hide,
    Nothing,
}

/// The open panel: shown or waiting for its page.
#[derive(Debug, Clone)]
struct Open {
    panel: Panel,
    /// The dock item it is anchored to.
    anchor: LogicalRect,
    /// The wait that showed it (its `panel-show`).
    shown: u64,
}

/// Which panel is open. Only one panel is open at a time.
///
/// A panel opens in two steps: `toggle` prepares it (the window stays
/// hidden while its page renders), then `ready` or `time_out` reveals it.
#[derive(Debug, Default)]
pub struct PanelState {
    open: Option<Open>,
    /// The wait for the open panel's page, while it lasts.
    waiting: Option<u64>,
    /// Waits started so far; numbers each one.
    waits: u64,
    /// Panel hidden by losing focus, and when.
    blurred: Option<(PanelId, Instant)>,
    /// Last geometry each panel's page reported, by panel: Settings, About
    /// and each widget instance opened in this run. An instance removed
    /// later keeps its entry, a few bytes, until the app exits; its ID is
    /// never reused.
    known: HashMap<PanelId, PanelGeometry>,
    /// Panel closed because the dock moved (its edge or size changed), to
    /// reopen at its item once the dock has settled in its new place, and
    /// when the dock moved. Reopening goes through the dock's
    /// `toggle_panel` again, which checks a widget instance again.
    reopen: Option<(PanelId, Instant)>,
}

impl PanelState {
    /// A click on a dock item. Clicking the item of an open panel makes the
    /// panel lose focus (and hide) before the toggle arrives, so a toggle of
    /// that panel right after a blur-hide keeps it closed.
    pub fn toggle(&mut self, panel: Panel, anchor: LogicalRect, now: Instant) -> Action {
        // A click is an explicit request: it replaces a pending reopen.
        self.reopen = None;
        let blurred = self.blurred.take();
        match &self.open {
            Some(open) if open.panel == panel => self.close(),
            Some(_) => self.prepare(panel, anchor),
            None => match blurred {
                Some((closed, at))
                    if panel.is(&closed) && now.saturating_duration_since(at) < REOPEN_GUARD =>
                {
                    Action::Nothing
                }
                _ => self.prepare(panel, anchor),
            },
        }
    }

    fn prepare(&mut self, panel: Panel, anchor: LogicalRect) -> Action {
        self.waits += 1;
        let id = panel.id();
        self.open = Some(Open {
            panel,
            anchor,
            shown: self.waits,
        });
        self.waiting = Some(self.waits);
        Action::Prepare(id, self.waits)
    }

    fn close(&mut self) -> Action {
        self.waiting = None;
        match self.open.take() {
            Some(_) => Action::Hide,
            None => Action::Nothing,
        }
    }

    /// The panel window lost focus. Ignored while the open panel waits for
    /// its page: the window is hidden then, and only hiding it for the wait
    /// can take its focus.
    pub fn blur(&mut self, now: Instant) -> Action {
        if self.waiting.is_some() {
            return Action::Nothing;
        }
        match self.open.take() {
            Some(open) => {
                self.blurred = Some((open.panel.id(), now));
                Action::Hide
            }
            None => Action::Nothing,
        }
    }

    /// The panel asked to close itself (Esc, Alt+F4), or the dock is
    /// hiding. Drops a pending reopen (`close_for_move`), but keeps the
    /// reopen guard: a late hide after a blur-hide must not let the click
    /// that closed the panel reopen it.
    pub fn hide(&mut self) -> Action {
        self.reopen = None;
        self.close()
    }

    /// The dock is moving (its edge or size changed in the settings):
    /// closes the open panel, whose anchor moved, and remembers it, to be
    /// reopened at its item once the dock has settled (`dock_settled`).
    /// Without an open panel, a reopen still pending is kept. Either way the
    /// wait for the dock to settle starts again at `now`.
    pub fn close_for_move(&mut self, now: Instant) -> Action {
        let open = self.open();
        let action = self.close();
        let pending = self.pending_reopen(now);
        self.reopen = open.or(pending).map(|id| (id, now));
        action
    }

    /// The dock applied a layout its page reported. When the layout left
    /// the window as it was, the page measured the window it is really in,
    /// so its items are where they will stay: hands out, once, the panel
    /// to reopen. When the layout resized or moved the window, the page
    /// reports again for the new window, and the reopen waits for that, up
    /// to `REOPEN_TIMEOUT` after the move; then it is dropped, so a layout
    /// much later can't reopen a panel the user no longer expects.
    pub fn dock_settled(&mut self, window_changed: bool, now: Instant) -> Option<PanelId> {
        let pending = self.pending_reopen(now);
        if pending.is_none() {
            self.reopen = None;
        }
        if window_changed {
            return None;
        }
        self.reopen = None;
        pending
    }

    /// The panel to reopen, unless the move was `REOPEN_TIMEOUT` ago.
    fn pending_reopen(&self, now: Instant) -> Option<PanelId> {
        self.reopen
            .as_ref()
            .filter(|(_, at)| now.saturating_duration_since(*at) <= REOPEN_TIMEOUT)
            .map(|(id, _)| id.clone())
    }

    /// The panel page rendered `id` at `geometry` (validated), answering
    /// the `panel-show` of `wait`. Reveals the open panel; a signal for any
    /// other panel, or for an earlier show of the same one, is stale and
    /// ignored. A signal for the panel already on screen places it again
    /// (its content changed size, or the time-out showed it first).
    pub fn ready(&mut self, id: PanelId, wait: u64, geometry: PanelGeometry) -> Action {
        let Some(open) = &self.open else {
            return Action::Nothing;
        };
        if !open.panel.is(&id) || open.shown != wait {
            return Action::Nothing;
        }
        let anchor = open.anchor;
        self.known.insert(id, geometry);
        self.waiting = None;
        Action::Reveal { anchor, geometry }
    }

    /// `READY_TIMEOUT` passed since `Prepare(_, wait)`. Reveals the panel
    /// with the last geometry its page reported, if it is still waiting.
    pub fn time_out(&mut self, wait: u64) -> Action {
        if self.waiting != Some(wait) {
            return Action::Nothing;
        }
        self.waiting = None;
        match &self.open {
            Some(open) => Action::Reveal {
                anchor: open.anchor,
                geometry: self
                    .known
                    .get(&open.panel.id())
                    .copied()
                    .unwrap_or_else(|| PanelGeometry::fallback(&open.panel)),
            },
            None => Action::Nothing,
        }
    }

    /// The open panel, shown or waiting for its page.
    pub fn open(&self) -> Option<PanelId> {
        self.open.as_ref().map(|open| open.panel.id())
    }
}

/// `{ "panelId": <id>, "wait": <n> }`: the payload of `panel-show`. The
/// page sends `wait` back in `panel_ready`, so a late answer to an earlier
/// show is told apart.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowPayload {
    panel_id: PanelId,
    wait: u64,
}

/// Sets up a panel window just made (`panel_window`, spec 0011 H2): it hides
/// the panel when it loses focus or is closed (Alt+F4). The panel state
/// itself is managed by the builder: a page can send a command
/// (`set_dock_layout`, `toggle_panel`) any time, and the command needs the
/// state (seen as a panic at startup, spec 0003 §2.12).
pub fn install(app: &AppHandle, window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    super::native::install(window)?;
    let app = app.clone();
    window.on_window_event(move |event| {
        let action = match event {
            // A program's menu, opened from the tray panel, took the focus:
            // the panel stays (`popup_menu`).
            #[cfg(windows)]
            WindowEvent::Focused(false) if super::popup_menu::blocks_blur() => return,
            WindowEvent::Focused(false) => lock(&app).blur(Instant::now()),
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                lock(&app).hide()
            }
            _ => return,
        };
        if let Err(error) = apply(&app, action) {
            log::warn!("panel: failed to hide: {error}");
        }
    });
    Ok(())
}

/// The menu a program opened from the tray panel is gone: the panel gets the
/// focus back, so Esc and a click elsewhere close it as usual; unless the user
/// went to another window, which closes it now.
#[cfg(windows)]
pub fn after_tray_menu(app: &AppHandle) {
    if lock(app).open().is_none() {
        return;
    }
    if super::native::foreground_is_other_visible_window() {
        let action = lock(app).hide();
        if let Err(error) = apply(app, action) {
            log::warn!("panel: failed to hide after a tray menu: {error}");
        }
    } else if let Some(window) = app.get_webview_window(LABEL)
        && let Err(error) = window.set_focus()
    {
        log::warn!("panel: failed to take the focus back after a tray menu: {error}");
    }
}

pub fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, PanelState> {
    // The state stays consistent even if a holder panicked.
    app.state::<Mutex<PanelState>>()
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Closes the panel through the state, telling the dock.
pub fn close(app: &AppHandle) {
    let action = lock(app).hide();
    if let Err(error) = apply(app, action) {
        log::warn!("panel: failed to hide: {error}");
    }
}

/// Closes the panel because the dock is moving, to reopen it at its item
/// once the dock has settled (`PanelState::close_for_move`).
pub fn close_for_move(app: &AppHandle) {
    let action = lock(app).close_for_move(Instant::now());
    if let Err(error) = apply(app, action) {
        log::warn!("panel: failed to hide: {error}");
    }
}

/// Carries out an action on the panel window and tells the dock when the
/// open panel changed. Must be called without holding the state lock:
/// hiding the window synchronously triggers the blur handler.
pub fn apply(app: &AppHandle, action: Action) -> Result<(), Box<dyn Error>> {
    // Made on demand (spec 0011 H2): a show for a window not made yet, or
    // whose page has not mounted, waits for the page; closing a panel whose
    // window is gone only tells the dock.
    let Some(window) = super::panel_window::ready(app) else {
        match action {
            Action::Prepare(id, wait) => {
                notify_dock(app, Some(id.clone()));
                super::panel_window::defer(
                    app,
                    super::panel_window::Pending { panel_id: id, wait },
                );
            }
            Action::Hide => {
                notify_dock(app, None);
                super::panel_window::touch(app);
            }
            Action::Reveal { .. } => return Err("panel window is not ready".into()),
            Action::Nothing => {}
        }
        return Ok(());
    };
    if let Some(target) = memory_target(&action) {
        memory::set(&window, target);
    }
    match action {
        Action::Prepare(id, wait) => {
            notify_dock(app, Some(id.clone()));
            // Hidden while the page renders: no previous content, no flash
            // at the previous size.
            window.hide()?;
            show(app, id, wait)?;
        }
        Action::Reveal { anchor, geometry } => {
            place(app, &window, anchor, geometry)?;
            // Already on screen (placed again): showing it would activate
            // it again.
            if !window.is_visible()? {
                window.show()?;
                focus(&window)?;
            }
        }
        Action::Hide => {
            notify_dock(app, None);
            window.hide()?;
            // Kept for a while, then released.
            super::panel_window::touch(app);
        }
        Action::Nothing => {}
    }
    Ok(())
}

/// Asks the page to render the panel `id` for the show `wait`, and reveals it
/// after `READY_TIMEOUT` if the page has not confirmed by then.
fn show(app: &AppHandle, id: PanelId, wait: u64) -> Result<(), Box<dyn Error>> {
    app.emit_to(LABEL, SHOW_EVENT, ShowPayload { panel_id: id, wait })?;
    schedule_time_out(app, wait)
}

/// The panel's page has mounted and listens for `panel-show` (spec 0011 H2):
/// it gets the show that waited for it, if the state still wants it.
pub fn page_mounted(app: &AppHandle) {
    let Some(pending) = super::panel_window::mounted() else {
        return;
    };
    let still_open = lock(app).open().as_ref() == Some(&pending.panel_id);
    if !still_open {
        return;
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        memory::set(&window, MemoryTarget::Normal);
    }
    if let Err(error) = show(app, pending.panel_id, pending.wait) {
        log::warn!("panel: failed to show after the page mounted: {error}");
        close(app);
    }
}

/// The memory target an action moves the panel to: normal before its page
/// renders a panel, low once it is hidden; the others keep it.
fn memory_target(action: &Action) -> Option<MemoryTarget> {
    match action {
        Action::Prepare(..) => Some(MemoryTarget::Normal),
        Action::Hide => Some(MemoryTarget::Low),
        Action::Reveal { .. } | Action::Nothing => None,
    }
}

/// Carries out a `Reveal` (or nothing); on failure, closes the panel.
pub fn reveal_or_close(app: &AppHandle, action: Action) -> Result<(), Box<dyn Error>> {
    apply(app, action).inspect_err(|_| close(app))
}

fn notify_dock(_app: &AppHandle, panel_id: Option<PanelId>) {
    dock::set_open_panel(panel_id);
}

/// Reveals the panel after `READY_TIMEOUT` if its page has not confirmed by
/// then.
fn schedule_time_out(app: &AppHandle, wait: u64) -> Result<(), Box<dyn Error>> {
    let app = app.clone();
    thread::Builder::new()
        .name("panel-ready-timeout".into())
        .spawn(move || {
            thread::sleep(READY_TIMEOUT);
            let handle = app.clone();
            // On the main thread, in order with the commands and the window
            // events.
            let scheduled = app.run_on_main_thread(move || {
                let action = lock(&handle).time_out(wait);
                if action != Action::Nothing {
                    log::debug!("panel: no ready signal in time, showing the last known size");
                }
                if let Err(error) = reveal_or_close(&handle, action) {
                    log::warn!("panel: failed to show: {error}");
                }
            });
            if let Err(error) = scheduled {
                log::warn!("panel: failed to schedule the time-out: {error}");
            }
        })?;
    Ok(())
}

fn place(
    app: &AppHandle,
    window: &WebviewWindow,
    anchor: LogicalRect,
    geometry: PanelGeometry,
) -> Result<(), Box<dyn Error>> {
    // The gap is measured from the visible bar, not from the taller window.
    let (origin, scale_factor, bar) = dock::geometry().ok_or("the dock is not placed yet")?;
    let anchor = to_screen(anchor, origin, scale_factor).ok_or("invalid anchor")?;
    let (work_area, _) = dock::work_area().ok_or("no primary monitor found")?;
    let rect = panel_rect(
        anchor,
        bar,
        work_area,
        scale_factor,
        geometry,
        dock::edge(app),
    )
    .ok_or("the work area leaves no room for the panel")?;
    window.set_size(PhysicalSize::new(rect.width, rect.height))?;
    window.set_position(PhysicalPosition::new(rect.x, rect.y))?;
    Ok(())
}

/// Focuses the panel only when the user just clicked the dock (the click is
/// the last input, so Windows lets DockMax take the foreground) or DockMax
/// owns the foreground already. Otherwise tao's `set_focus` would fake an
/// Alt key press to steal the foreground from the app in use.
fn focus(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    if !super::native::owns_foreground() && !super::native_dock::clicked_recently() {
        return Ok(());
    }
    window.set_focus()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn geometry(width: f64, height: f64, inset: f64) -> PanelGeometry {
        PanelGeometry {
            size: LogicalSize { width, height },
            inset,
        }
    }

    /// A panel without a transparent inset: the whole window is the card.
    fn size(width: f64, height: f64) -> PanelGeometry {
        geometry(width, height, 0.0)
    }

    const WORK_AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1032,
    };
    const BAR: Rect = Rect {
        x: 1160,
        y: 960,
        width: 240,
        height: 64,
    };

    #[test]
    fn panel_ids_use_lowercase_names_in_json() {
        assert_eq!(
            serde_json::to_string(&PanelId::Settings).unwrap(),
            "\"settings\""
        );
        assert_eq!(serde_json::to_string(&PanelId::About).unwrap(), "\"about\"");
        assert_eq!(
            serde_json::from_str::<PanelId>("\"about\"").unwrap(),
            PanelId::About
        );
        assert_eq!(serde_json::to_string(&PanelId::Tray).unwrap(), "\"tray\"");
        assert_eq!(
            serde_json::from_str::<PanelId>("\"tray\"").unwrap(),
            PanelId::Tray
        );
        assert!(serde_json::from_str::<PanelId>("\"About\"").is_err());
    }

    /// Two widget instances, as Rust makes their IDs (UUID v7).
    const WIDGET: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071";
    const OTHER_WIDGET: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6075";

    fn widget_id(id: &str) -> PanelId {
        PanelId::Widget(WidgetId::parse(id).unwrap())
    }

    fn widget(id: &str, kind: WidgetKind) -> Panel {
        Panel::Widget {
            id: WidgetId::parse(id).unwrap(),
            kind,
        }
    }

    #[test]
    fn a_widget_panel_id_is_an_object_with_the_instance() {
        let json = format!(r#"{{"widget":"{WIDGET}"}}"#);
        assert_eq!(serde_json::to_string(&widget_id(WIDGET)).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<PanelId>(&json).unwrap(),
            widget_id(WIDGET)
        );
    }

    #[test]
    fn an_app_panel_id_is_an_object_with_the_pinned_item() {
        let json = format!(r#"{{"app":"{WIDGET}"}}"#);
        let id = PanelId::App(WidgetId::parse(WIDGET).unwrap());

        assert_eq!(serde_json::to_string(&id).unwrap(), json);
        assert_eq!(serde_json::from_str::<PanelId>(&json).unwrap(), id);
        for rejected in [
            r#"{"app":""}"#.to_owned(),
            r#"{"app":"Some.App!App"}"#.to_owned(),
            r#"{"app":7}"#.to_owned(),
            format!(r#"{{"app":"{WIDGET}","widget":"{WIDGET}"}}"#),
        ] {
            assert!(
                serde_json::from_str::<PanelId>(&rejected).is_err(),
                "{rejected}"
            );
        }
    }

    #[test]
    fn an_app_panel_is_the_one_its_id_names_and_not_the_widget_with_the_same_id() {
        let app = Panel::App {
            id: WidgetId::parse(WIDGET).unwrap(),
        };

        assert_eq!(app.id(), PanelId::App(WidgetId::parse(WIDGET).unwrap()));
        assert!(app.is(&PanelId::App(WidgetId::parse(WIDGET).unwrap())));
        assert!(!app.is(&PanelId::App(WidgetId::parse(OTHER_WIDGET).unwrap())));
        assert!(!app.is(&widget_id(WIDGET)));
        assert!(
            !widget(WIDGET, WidgetKind::Clock).is(&PanelId::App(WidgetId::parse(WIDGET).unwrap()))
        );
    }

    #[test]
    fn an_open_program_panel_is_named_by_its_key_and_only_by_it() {
        let open = Panel::Open {
            key: "00000000000000aa".into(),
        };

        assert_eq!(open.id(), PanelId::Open("00000000000000aa".into()));
        assert!(open.is(&PanelId::Open("00000000000000aa".into())));
        assert!(!open.is(&PanelId::Open("00000000000000bb".into())));
        assert!(!open.is(&PanelId::App(WidgetId::parse(WIDGET).unwrap())));
        assert!(!open.is(&widget_id(WIDGET)));
        let parsed: PanelId = serde_json::from_str(r#"{"open":"00000000000000aa"}"#).unwrap();
        assert_eq!(parsed, PanelId::Open("00000000000000aa".into()));
        assert!(serde_json::from_str::<PanelId>(r#"{"open":7}"#).is_err());
    }

    #[test]
    fn a_widget_panel_id_with_a_malformed_instance_or_extra_keys_is_rejected() {
        let rejected = [
            r#"{"widget":""}"#.to_owned(),
            r#"{"widget":"clock"}"#.to_owned(),
            // Uppercase, and a v4 UUID: not the form Rust makes.
            r#"{"widget":"019A1B2C-3D4E-7F60-8A1B-2C3D4E5F6071"}"#.to_owned(),
            r#"{"widget":"019a1b2c-3d4e-4f60-8a1b-2c3d4e5f6071"}"#.to_owned(),
            r#"{"widget":7}"#.to_owned(),
            r#"{"widget":null}"#.to_owned(),
            r#""widget""#.to_owned(),
            r#"{"Widget":"019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071"}"#.to_owned(),
            format!(r#"{{"widget":"{WIDGET}","kind":"clock"}}"#),
            format!(r#"{{"widget":"{WIDGET}","about":null}}"#),
            r#"{}"#.to_owned(),
            r#"[]"#.to_owned(),
        ];
        for json in rejected {
            assert!(serde_json::from_str::<PanelId>(&json).is_err(), "{json}");
        }
    }

    #[test]
    fn widget_panels_are_reserved_by_kind() {
        assert_eq!(
            widget(WIDGET, WidgetKind::Clock).logical_size(),
            (316.0, 186.0)
        );
        assert_eq!(
            widget(WIDGET, WidgetKind::TimeProgress).logical_size(),
            (336.0, 300.0)
        );
        assert_eq!(
            widget(WIDGET, WidgetKind::Note).logical_size(),
            (356.0, 334.0)
        );
    }

    #[test]
    fn a_panel_is_the_one_its_id_names() {
        let clock = widget(WIDGET, WidgetKind::Clock);
        assert_eq!(clock.id(), widget_id(WIDGET));
        assert!(clock.is(&widget_id(WIDGET)));
        assert!(!clock.is(&widget_id(OTHER_WIDGET)));
        assert!(!clock.is(&PanelId::Settings));
        assert!(Panel::Settings.is(&PanelId::Settings));
        assert!(!Panel::Settings.is(&PanelId::About));
        assert!(!Panel::About.is(&widget_id(WIDGET)));
    }

    #[test]
    fn panels_have_fixed_sizes() {
        assert_eq!(Panel::Settings.logical_size(), (336.0, 406.0));
        assert_eq!(Panel::About.logical_size(), (316.0, 252.0));
        assert_eq!(Panel::Tray.logical_size(), (296.0, 166.0));
    }

    /// The number of pixels `property` is set to in the CSS rule whose
    /// selector is exactly `selector`.
    fn css_px(css: &str, selector: &str, property: &str) -> f64 {
        let start = css
            .find(&format!("\n{selector} {{"))
            .unwrap_or_else(|| panic!("no rule for {selector}"));
        let body = &css[start..];
        let body = &body[..body.find('}').unwrap()];
        let value = body
            .split(&format!("{property}:"))
            .nth(1)
            .unwrap_or_else(|| panic!("no {property} in {selector}"));
        value
            .split("px")
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{property} of {selector} is not in px"))
    }

    #[test]
    fn fallback_geometry_is_the_card_the_page_lays_out_with_its_inset() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/panel/Panel.css");
        let css = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\r\n", "\n");
        let inset = css_px(&css, ".panel-frame", "--panel-inset");
        let widget = |kind| Panel::Widget {
            id: WidgetId::parse(WIDGET).unwrap(),
            kind,
        };
        let cards = [
            (Panel::Settings, css_px(&css, ".panel", "width")),
            (
                Panel::About,
                css_px(&css, r#".panel[data-panel="about"]"#, "width"),
            ),
            (
                Panel::Tray,
                css_px(&css, r#".panel[data-panel="tray"]"#, "width"),
            ),
            (
                widget(WidgetKind::Clock),
                css_px(&css, r#".panel[data-widget="clock"]"#, "width"),
            ),
            (
                widget(WidgetKind::TimeProgress),
                css_px(&css, r#".panel[data-widget="time-progress"]"#, "width"),
            ),
            (
                widget(WidgetKind::Note),
                css_px(&css, r#".panel[data-widget="note"]"#, "width"),
            ),
        ];

        for (id, card_width) in cards {
            let fallback = PanelGeometry::fallback(&id);
            assert_eq!(fallback.inset, inset, "{id:?}");
            assert_eq!(fallback.size.width, card_width + 2.0 * inset, "{id:?}");
            // A geometry the page could have reported.
            assert_eq!(
                reported(fallback.size.width, fallback.size.height, fallback.inset),
                Ok(fallback),
                "{id:?}"
            );
        }
    }

    #[test]
    fn to_screen_offsets_and_scales_the_client_rect() {
        let anchor = LogicalRect {
            x: 10.0,
            y: 20.5,
            width: 99.5,
            height: 30.0,
        };

        assert_eq!(
            to_screen(anchor, (100, 200), 1.5),
            Some(rect(115, 231, 149, 45))
        );
    }

    #[test]
    fn to_screen_rejects_non_finite_or_negative_values() {
        let valid = LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };

        assert_eq!(
            to_screen(
                LogicalRect {
                    x: f64::NAN,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
        assert_eq!(
            to_screen(
                LogicalRect {
                    width: f64::INFINITY,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
        assert_eq!(
            to_screen(
                LogicalRect {
                    height: -1.0,
                    ..valid
                },
                (0, 0),
                1.0
            ),
            None
        );
    }

    const VALID: LogicalRect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    #[test]
    fn validate_accepts_rects_within_the_limit() {
        let cases = [
            VALID,
            LogicalRect {
                x: -100_000.0,
                y: 100_000.0,
                width: 100_000.0,
                height: 0.0,
            },
            LogicalRect {
                x: 16.5,
                y: -0.0,
                width: 104.25,
                height: 40.0,
            },
        ];
        for rect in cases {
            assert_eq!(rect.validate(), Ok(()), "{rect:?}");
        }
    }

    #[test]
    fn validate_rejects_bad_rects() {
        use RectError::*;
        let cases = [
            (
                LogicalRect {
                    x: f64::NAN,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    y: f64::INFINITY,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    width: f64::NEG_INFINITY,
                    ..VALID
                },
                NotFinite,
            ),
            (
                LogicalRect {
                    height: -1.0,
                    ..VALID
                },
                NegativeSize,
            ),
            (
                LogicalRect {
                    width: -1e12,
                    ..VALID
                },
                NegativeSize,
            ),
            (LogicalRect { x: 1e12, ..VALID }, TooLarge),
            (LogicalRect { x: -1e12, ..VALID }, TooLarge),
            (
                LogicalRect {
                    y: f64::MAX,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    y: f64::MIN,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    width: f64::MAX,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    height: 1e12,
                    ..VALID
                },
                TooLarge,
            ),
            (
                LogicalRect {
                    x: 100_000.5,
                    ..VALID
                },
                TooLarge,
            ),
        ];
        for (rect, expected) in cases {
            assert_eq!(rect.validate(), Err(expected), "{rect:?}");
        }
    }

    #[test]
    fn to_screen_never_panics_and_rejects_what_does_not_fit() {
        let cases = [
            (LogicalRect { x: 1e12, ..VALID }, (0, 0), 1.0),
            (LogicalRect { x: -1e12, ..VALID }, (0, 0), 1.0),
            (
                LogicalRect {
                    y: f64::MAX,
                    ..VALID
                },
                (0, 0),
                1.0,
            ),
            (
                LogicalRect {
                    width: f64::MAX,
                    ..VALID
                },
                (0, 0),
                3.0,
            ),
            (
                LogicalRect {
                    x: f64::NAN,
                    ..VALID
                },
                (0, 0),
                1.0,
            ),
            // Valid rect, origin near the i32 limits.
            (
                LogicalRect {
                    x: 100_000.0,
                    ..VALID
                },
                (i32::MAX - 10, 0),
                3.0,
            ),
            (
                LogicalRect {
                    y: -100_000.0,
                    ..VALID
                },
                (0, i32::MIN + 10),
                3.0,
            ),
            // Valid rect, unusable scale.
            (VALID, (0, 0), f64::NAN),
            (VALID, (0, 0), f64::INFINITY),
            (VALID, (0, 0), 0.0),
            (VALID, (0, 0), -1.0),
            (
                LogicalRect {
                    width: 100_000.0,
                    ..VALID
                },
                (0, 0),
                f64::MAX,
            ),
        ];
        for (rect, origin, scale_factor) in cases {
            assert_eq!(
                to_screen(rect, origin, scale_factor),
                None,
                "{rect:?} {origin:?} {scale_factor}"
            );
        }
    }

    #[test]
    fn to_screen_rejects_a_huge_finite_scale_without_panicking() {
        let right = LogicalRect { x: 1.0, ..VALID };
        let left = LogicalRect { x: -1.0, ..VALID };

        assert_eq!(to_screen(right, (1, 0), f64::MAX), None);
        assert_eq!(to_screen(left, (-1, 0), f64::MAX), None);
    }

    #[test]
    fn to_screen_converts_the_largest_valid_rect_at_scale_300() {
        let rect = LogicalRect {
            x: -100_000.0,
            y: 100_000.0,
            width: 100_000.0,
            height: 100_000.0,
        };

        assert_eq!(
            to_screen(rect, (i32::MAX - 300_000, 0), 3.0),
            Some(Rect {
                x: i32::MAX - 600_000,
                y: 300_000,
                width: 300_000,
                height: 300_000,
            })
        );
    }

    #[test]
    fn panel_rect_saturates_instead_of_overflowing() {
        let max = rect(i32::MAX, i32::MAX, u32::MAX, u32::MAX);
        let min = rect(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
        let cases = [
            (max, BAR, WORK_AREA, 3.0),
            (min, min, WORK_AREA, 3.0),
            (BAR, max, max, 1.0),
            (BAR, min, min, 1.0),
            (max, max, max, f64::MAX),
            (min, min, min, f64::NAN),
        ];
        for (anchor, bar, work_area, scale_factor) in cases {
            for edge in [Edge::Bottom, Edge::Top] {
                // Must not panic.
                let _ = panel_rect(
                    anchor,
                    bar,
                    work_area,
                    scale_factor,
                    size(360.0, 480.0),
                    edge,
                );
            }
        }
        let panel = panel_rect(
            BAR,
            BAR,
            rect(i32::MAX, 0, u32::MAX, 1032),
            1.0,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();
        assert_eq!(panel.x, i32::MAX);
    }

    #[test]
    fn bottom_panel_is_centered_on_the_anchor_8px_above_the_bar() {
        let anchor = rect(1200, 976, 100, 32);

        let panel = panel_rect(
            anchor,
            BAR,
            WORK_AREA,
            1.0,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();

        assert_eq!(panel, rect(1250 - 180, 960 - 8 - 480, 360, 480));
    }

    #[test]
    fn panel_near_the_left_edge_is_clamped_to_the_margin() {
        let anchor = rect(20, 976, 40, 32);

        let panel = panel_rect(
            anchor,
            BAR,
            WORK_AREA,
            1.0,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();

        assert_eq!(panel.x, 8);
    }

    #[test]
    fn panel_near_the_right_edge_is_clamped_to_the_margin() {
        let anchor = rect(2500, 976, 40, 32);

        let panel = panel_rect(
            anchor,
            BAR,
            WORK_AREA,
            1.0,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();

        assert_eq!(panel.x, 2560 - 8 - 360);
    }

    #[test]
    fn top_panel_sits_8px_below_the_bar() {
        let bar = rect(1160, 8, 240, 64);
        let anchor = rect(1200, 24, 100, 32);

        let panel = panel_rect(anchor, bar, WORK_AREA, 1.0, size(320.0, 240.0), Edge::Top).unwrap();

        assert_eq!(panel, rect(1250 - 160, 8 + 64 + 8, 320, 240));
    }

    #[test]
    fn scale_125_scales_size_gap_and_margin() {
        let work_area = rect(0, 0, 2048, 824);
        let bar = rect(874, 734, 300, 80);
        let anchor = rect(900, 754, 125, 40);

        let panel = panel_rect(
            anchor,
            bar,
            work_area,
            1.25,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();

        // 360 * 1.25 = 450, 480 * 1.25 = 600, 8 * 1.25 = 10.
        assert_eq!(panel, rect(962 - 225, 734 - 10 - 600, 450, 600));
    }

    #[test]
    fn scale_150_scales_size_gap_and_margin() {
        let work_area = rect(0, 0, 2560, 1368);
        let bar = rect(1100, 1260, 360, 96);
        let anchor = rect(1130, 1284, 150, 48);

        let panel = panel_rect(
            anchor,
            bar,
            work_area,
            1.5,
            size(320.0, 240.0),
            Edge::Bottom,
        )
        .unwrap();

        // 320 * 1.5 = 480, 240 * 1.5 = 360, 8 * 1.5 = 12.
        assert_eq!(panel, rect(1205 - 240, 1260 - 12 - 360, 480, 360));
    }

    #[test]
    fn scale_150_clamps_with_the_scaled_margin() {
        let work_area = rect(0, 0, 2560, 1368);
        let bar = rect(1100, 1260, 360, 96);
        let anchor = rect(0, 1284, 20, 48);

        let panel = panel_rect(
            anchor,
            bar,
            work_area,
            1.5,
            size(320.0, 240.0),
            Edge::Bottom,
        )
        .unwrap();

        assert_eq!(panel.x, 12);
    }

    #[test]
    fn card_wider_than_the_work_area_is_shrunk_to_it() {
        let work_area = rect(100, 0, 300, 1032);

        let panel = panel_rect(
            rect(200, 976, 40, 32),
            BAR,
            work_area,
            1.0,
            geometry(360.0, 480.0, 8.0),
            Edge::Bottom,
        )
        .unwrap();

        // The card spans 108..392; the inset goes 8 px beyond on each side.
        assert_eq!(panel.x, 100);
        assert_eq!(panel.width, 300 - 16 + 16);
        assert_eq!(panel.height, 480);
    }

    #[test]
    fn panel_taller_than_the_space_above_the_bar_is_shrunk_to_it() {
        let work_area = rect(0, 40, 2560, 600);
        let bar = rect(1160, 400, 240, 64);

        let panel = panel_rect(
            rect(1200, 416, 100, 32),
            bar,
            work_area,
            1.0,
            size(360.0, 480.0),
            Edge::Bottom,
        )
        .unwrap();

        // 400 - 8 - 480 = -88 would leave the work area: the card spans
        // 48..392, between the work area's margin and the bar's.
        assert_eq!((panel.y, panel.height), (48, 400 - 8 - 48));
    }

    #[test]
    fn bottom_card_sits_8px_above_the_bar_with_the_inset_around_it() {
        let anchor = rect(1200, 976, 100, 32);

        let panel = panel_rect(
            anchor,
            BAR,
            WORK_AREA,
            1.0,
            geometry(360.0, 480.0, 8.0),
            Edge::Bottom,
        )
        .unwrap();

        // The card is 344x464, centered on 1250, its bottom at 960 - 8; the
        // window's bottom is flush with the bar.
        assert_eq!(panel, rect(1250 - 172 - 8, 960 - 8 - 464 - 8, 360, 480));
        assert_eq!(panel.y + 480, 960);
    }

    #[test]
    fn top_card_sits_8px_below_the_bar_at_150_percent() {
        let work_area = rect(0, 0, 2560, 1368);
        let bar = rect(1100, 12, 360, 96);
        let anchor = rect(1130, 36, 150, 48);

        let panel = panel_rect(
            anchor,
            bar,
            work_area,
            1.5,
            geometry(320.0, 240.0, 8.0),
            Edge::Top,
        )
        .unwrap();

        // 320x240 at 1.5 = 480x360, inset 12, card 456x336, gap 12.
        assert_eq!(panel, rect(1205 - 228 - 12, 12 + 96 + 12 - 12, 480, 360));
    }

    #[test]
    fn inset_scales_and_rounds_like_the_size_at_125_percent() {
        let work_area = rect(0, 0, 2048, 824);
        let bar = rect(874, 734, 300, 80);
        let anchor = rect(900, 754, 125, 40);

        let panel = panel_rect(
            anchor,
            bar,
            work_area,
            1.25,
            geometry(360.0, 480.0, 4.4),
            Edge::Bottom,
        )
        .unwrap();

        // 450x600, inset 5.5 -> 6, card 438x588, gap 10.
        assert_eq!(panel, rect(962 - 219 - 6, 734 - 10 - 588 - 6, 450, 600));
    }

    #[test]
    fn card_is_clamped_inside_the_work_area_and_only_the_inset_goes_beyond() {
        let inset = geometry(360.0, 480.0, 8.0);

        let left = panel_rect(
            rect(20, 976, 40, 32),
            BAR,
            WORK_AREA,
            1.0,
            inset,
            Edge::Bottom,
        )
        .unwrap();
        let right = panel_rect(
            rect(2500, 976, 40, 32),
            BAR,
            WORK_AREA,
            1.0,
            inset,
            Edge::Bottom,
        )
        .unwrap();

        // Card at x = 8 and ending at 2560 - 8.
        assert_eq!(left.x, 8 - 8);
        assert_eq!(right.x + 360 - 8, 2560 - 8);
    }

    #[test]
    fn card_too_tall_for_the_space_is_shrunk_between_the_bar_and_the_far_edge() {
        let inset = geometry(360.0, 480.0, 8.0);
        let anchor = rect(1200, 416, 100, 32);

        let bottom = panel_rect(
            anchor,
            rect(1160, 400, 240, 64),
            rect(0, 40, 2560, 600),
            1.0,
            inset,
            Edge::Bottom,
        )
        .unwrap();
        let top = panel_rect(
            rect(1200, 24, 100, 32),
            rect(1160, 8, 240, 64),
            rect(0, 0, 2560, 500),
            1.0,
            inset,
            Edge::Top,
        )
        .unwrap();

        // Bottom: card from 40 + 8 down to 400 - 8, the window flush with
        // the bar. Top: card from 72 + 8 down to 500 - 8.
        assert_eq!((bottom.y + 8, bottom.height - 16), (48, 392 - 48));
        assert_eq!(bottom.y + bottom.height as i32, 400);
        assert_eq!((top.y + 8, top.height - 16), (80, 492 - 80));
        assert_eq!(top.y, 72);
    }

    #[test]
    fn no_room_for_the_card_gives_no_window() {
        // The bar fills the work area: nothing fits above or below it.
        let work_area = rect(0, 0, 1920, 80);
        let bar = rect(840, 8, 240, 64);

        for edge in [Edge::Bottom, Edge::Top] {
            assert_eq!(
                panel_rect(bar, bar, work_area, 1.0, size(360.0, 480.0), edge),
                None
            );
        }
    }

    #[test]
    fn inset_wider_than_half_the_window_never_underflows() {
        let odd = PanelGeometry {
            size: LogicalSize {
                width: 10.0,
                height: 4.0,
            },
            inset: 1e9,
        };
        for scale_factor in [1.0, 1.5, f64::MAX, f64::NAN] {
            for edge in [Edge::Bottom, Edge::Top] {
                // Must not panic.
                let _ = panel_rect(BAR, BAR, WORK_AREA, scale_factor, odd, edge);
            }
        }
    }

    /// `panel_rect`, as `None` when there is no room for the card.
    fn placed(
        anchor: Rect,
        bar: Rect,
        work_area: Rect,
        scale_factor: f64,
        geometry: PanelGeometry,
        edge: Edge,
    ) -> Option<Rect> {
        panel_rect(anchor, bar, work_area, scale_factor, geometry, edge)
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        let (a_right, b_right) = (
            i64::from(a.x) + i64::from(a.width),
            i64::from(b.x) + i64::from(b.width),
        );
        let (a_bottom, b_bottom) = (
            i64::from(a.y) + i64::from(a.height),
            i64::from(b.y) + i64::from(b.height),
        );
        i64::from(a.x) < b_right
            && i64::from(b.x) < a_right
            && i64::from(a.y) < b_bottom
            && i64::from(b.y) < a_bottom
    }

    fn within(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && i64::from(inner.x) + i64::from(inner.width)
                <= i64::from(outer.x) + i64::from(outer.width)
            && i64::from(inner.y) + i64::from(inner.height)
                <= i64::from(outer.y) + i64::from(outer.height)
    }

    #[test]
    fn panel_never_covers_the_bar_and_its_card_never_leaves_the_work_area() {
        let work_areas = [
            rect(0, 0, 1920, 1032),
            rect(0, 0, 2560, 1392),
            rect(0, 48, 1920, 1032),
            rect(62, 0, 1858, 1080),
            rect(-1920, -200, 1920, 1040),
            rect(0, 0, 800, 600),
            rect(0, 0, 400, 300),
        ];
        let sizes = [
            (17.0, 17.0),
            (40.0, 30.0),
            (336.0, 400.0),
            (360.0, 480.0),
            (1200.0, 900.0),
            (MAX_PANEL_SIZE, MAX_PANEL_SIZE),
        ];
        let mut checked = 0;
        for work_area in work_areas {
            for scale_factor in [1.0, 1.25, 1.5, 1.75] {
                for edge in [Edge::Bottom, Edge::Top] {
                    // The bar as the dock lays it: flush with the window
                    // side that faces the screen edge.
                    let dock = dock::dock_rect(work_area, scale_factor, (240.0, 140.0), edge);
                    let bar_height = (62.0 * scale_factor).round() as u32;
                    let bar = Rect {
                        x: dock.x + 20,
                        y: match edge {
                            Edge::Bottom => dock.y + dock.height as i32 - bar_height as i32,
                            Edge::Top => dock.y,
                        },
                        width: dock.width - 40,
                        height: bar_height,
                    };
                    let right = work_area.x + work_area.width as i32;
                    let anchors = [
                        rect(bar.x + 6, bar.y + 6, 50, 50),
                        rect(work_area.x - 500, bar.y, 50, 50),
                        rect(work_area.x, bar.y, 1, 1),
                        rect(right - 10, bar.y, 50, 50),
                        rect(right + 500, bar.y, 50, 50),
                    ];
                    for anchor in anchors {
                        for (width, height) in sizes {
                            for inset in 0..=8 {
                                let inset = f64::from(inset);
                                let geometry = reported(width, height, inset).unwrap();
                                let case = format!(
                                    "{work_area:?} {scale_factor} {edge:?} {anchor:?} {geometry:?}"
                                );
                                let Some(window) =
                                    placed(anchor, bar, work_area, scale_factor, geometry, edge)
                                else {
                                    panic!("no room: {case}");
                                };
                                let inset = (inset * scale_factor).round() as u32;
                                let card = Rect {
                                    x: window.x + inset as i32,
                                    y: window.y + inset as i32,
                                    width: window.width - 2 * inset,
                                    height: window.height - 2 * inset,
                                };
                                assert!(!overlaps(window, bar), "covers the bar: {case}");
                                assert!(within(card, work_area), "card leaves: {case}");
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 7 * 4 * 2 * 5 * 6 * 9);
    }

    #[test]
    fn the_inset_is_at_most_the_gap_to_the_bar() {
        assert_eq!(MAX_INSET, MARGIN);
    }

    /// The geometry the panel page reports, read as `panel_ready` reads it.
    fn reported(width: f64, height: f64, inset: f64) -> Result<PanelGeometry, String> {
        let size = serde_json::json!({ "width": width, "height": height });
        let size = serde_json::from_value::<PanelSize>(size).map_err(|e| e.to_string())?;
        let inset = serde_json::from_value::<PanelInset>(serde_json::json!(inset))
            .map_err(|e| e.to_string())?;
        Ok(PanelGeometry::new(size, inset))
    }

    #[test]
    fn geometry_within_the_limits_is_read() {
        for (width, height, inset) in [
            (360.0, 480.0, 0.0),
            (360.0, 480.0, 8.0),
            (360.0, 480.0, 4.4),
            (MIN_PANEL_SIZE, MIN_PANEL_SIZE, 0.0),
            (MAX_PANEL_SIZE, MAX_PANEL_SIZE, MAX_INSET),
            (17.0, 17.0, 8.0),
        ] {
            assert_eq!(
                reported(width, height, inset),
                Ok(geometry(width, height, inset)),
                "{width}x{height} {inset}"
            );
        }
        assert_eq!(MIN_PANEL_SIZE, 17.0);
    }

    #[test]
    fn geometry_out_of_bounds_is_rejected_when_deserialized() {
        let cases = [
            // `json!` writes NaN and infinities as null.
            (f64::NAN, 480.0, 0.0),
            (360.0, f64::INFINITY, 0.0),
            (0.0, 480.0, 0.0),
            (360.0, 0.5, 0.0),
            (360.0, -480.0, 0.0),
            (MAX_PANEL_SIZE + 1.0, 480.0, 0.0),
            (360.0, 1e12, 0.0),
            (360.0, 480.0, -1.0),
            (360.0, 480.0, f64::NAN),
            (360.0, 480.0, f64::INFINITY),
            // Wider than the gap to the dock bar: the window would cover it.
            (360.0, 480.0, MAX_INSET + 0.5),
            (360.0, 480.0, 32.0),
            // Smaller than a card can be with any inset.
            (16.0, 480.0, 0.0),
            (360.0, 10.0, 5.0),
        ];
        for (width, height, inset) in cases {
            assert!(
                reported(width, height, inset).is_err(),
                "{width}x{height} {inset}"
            );
        }
        for json in [
            r#"{"width":360,"height":480,"depth":1}"#,
            r#"{"width":"360","height":480}"#,
            r#"{"width":360}"#,
        ] {
            assert!(serde_json::from_str::<PanelSize>(json).is_err(), "{json}");
        }
        for json in [r#""8""#, "null", "-0.5", "1e400"] {
            assert!(serde_json::from_str::<PanelInset>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn rects_and_sizes_with_unknown_fields_are_rejected() {
        let rect = r#"{"x":0,"y":0,"width":1,"height":1,"top":0}"#;
        let size = r#"{"width":1,"height":1,"depth":1}"#;

        assert!(serde_json::from_str::<LogicalRect>(rect).is_err());
        assert!(serde_json::from_str::<LogicalSize>(size).is_err());
        assert!(serde_json::from_str::<LogicalSize>(r#"{"width":1}"#).is_err());
        assert_eq!(
            serde_json::from_str::<LogicalSize>(r#"{"width":1.5,"height":2}"#).unwrap(),
            LogicalSize {
                width: 1.5,
                height: 2.0
            }
        );
    }

    #[test]
    fn invalid_rects_and_sizes_are_rejected_when_deserialized() {
        let rects = [
            r#"{"x":0,"y":0,"width":-1,"height":1}"#,
            r#"{"x":0,"y":0,"width":1,"height":-0.5}"#,
            r#"{"x":1e12,"y":0,"width":1,"height":1}"#,
            r#"{"x":0,"y":-100000.5,"width":1,"height":1}"#,
            r#"{"x":0,"y":0,"width":1e300,"height":1}"#,
            r#"{"x":0,"y":0,"width":1e400,"height":1}"#,
            r#"{"x":null,"y":0,"width":1,"height":1}"#,
            r#"{"x":"NaN","y":0,"width":1,"height":1}"#,
            r#"{"x":0,"y":0,"width":1,"height":1,"top":0}"#,
        ];
        for json in rects {
            assert!(serde_json::from_str::<LogicalRect>(json).is_err(), "{json}");
        }
        let sizes = [
            r#"{"width":0,"height":1}"#,
            r#"{"width":1,"height":0.5}"#,
            r#"{"width":-3,"height":1}"#,
            r#"{"width":1,"height":100000.5}"#,
            r#"{"width":1e300,"height":1}"#,
            r#"{"width":"Infinity","height":1}"#,
            r#"{"width":1,"height":1,"depth":1}"#,
        ];
        for json in sizes {
            assert!(serde_json::from_str::<LogicalSize>(json).is_err(), "{json}");
        }
        assert_eq!(
            serde_json::from_str::<LogicalRect>(r#"{"x":-4,"y":-13.2,"width":140,"height":0}"#)
                .unwrap(),
            LogicalRect {
                x: -4.0,
                y: -13.2,
                width: 140.0,
                height: 0.0
            }
        );
    }

    #[test]
    fn payloads_use_the_names_the_pages_expect() {
        assert_eq!(
            serde_json::to_value(ShowPayload {
                panel_id: PanelId::About,
                wait: 7
            })
            .unwrap(),
            serde_json::json!({ "panelId": "about", "wait": 7 })
        );
        assert_eq!(SHOW_EVENT, "panel-show");
    }

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    const ANCHOR: LogicalRect = LogicalRect {
        x: 20.0,
        y: 40.0,
        width: 64.0,
        height: 64.0,
    };

    const RENDERED: PanelGeometry = PanelGeometry {
        size: LogicalSize {
            width: 392.0,
            height: 520.0,
        },
        inset: 8.0,
    };

    fn prepared(action: Action) -> (PanelId, u64) {
        match action {
            Action::Prepare(id, wait) => (id, wait),
            other => panic!("expected Prepare, got {other:?}"),
        }
    }

    /// Opens `id` and reveals it as its page confirms, at `RENDERED`.
    /// Returns the wait that showed it.
    fn show(state: &mut PanelState, panel: Panel, now: Instant) -> u64 {
        let id = panel.id();
        let (prepared_id, wait) = prepared(state.toggle(panel, ANCHOR, now));
        assert_eq!(prepared_id, id);
        assert_eq!(
            state.ready(id, wait, RENDERED),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: RENDERED
            }
        );
        wait
    }

    #[test]
    fn the_panel_is_at_normal_memory_to_render_and_low_once_hidden() {
        let mut state = PanelState::default();
        let prepare = state.toggle(Panel::Settings, ANCHOR, Instant::now());
        assert_eq!(memory_target(&prepare), Some(MemoryTarget::Normal));
        assert_eq!(memory_target(&state.hide()), Some(MemoryTarget::Low));
        // Placing an open panel again, or nothing, keeps the level.
        let reveal = Action::Reveal {
            anchor: ANCHOR,
            geometry: RENDERED,
        };
        assert_eq!(memory_target(&reveal), None);
        assert_eq!(memory_target(&Action::Nothing), None);
    }

    #[test]
    fn toggle_prepares_a_closed_panel_without_showing_it() {
        let mut state = PanelState::default();

        let (id, _) = prepared(state.toggle(Panel::Settings, ANCHOR, Instant::now()));

        assert_eq!(id, PanelId::Settings);
        assert_eq!(state.open(), Some(PanelId::Settings));
    }

    #[test]
    fn ready_for_the_waiting_panel_reveals_it_at_the_reported_geometry() {
        let mut state = PanelState::default();
        let (_, wait) = prepared(state.toggle(Panel::About, ANCHOR, Instant::now()));

        assert_eq!(
            state.ready(PanelId::About, wait, RENDERED),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn ready_for_a_stale_panel_is_ignored() {
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, first) = prepared(state.toggle(Panel::Settings, ANCHOR, t));
        let (_, wait) = prepared(state.toggle(Panel::About, ANCHOR, at(t, 50)));

        // The Settings page confirms late, after the switch to About.
        assert_eq!(
            state.ready(PanelId::Settings, first, RENDERED),
            Action::Nothing
        );
        // About still waits, and still falls back on its own size.
        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: PanelGeometry::fallback(&Panel::About)
            }
        );
    }

    #[test]
    fn ready_of_an_earlier_show_of_the_same_panel_is_ignored() {
        let mut state = PanelState::default();
        let t = Instant::now();
        // A, then B, then A again before either page confirmed.
        let (_, a) = prepared(state.toggle(Panel::Settings, ANCHOR, t));
        let (_, b) = prepared(state.toggle(Panel::About, ANCHOR, at(t, 30)));
        let moved = LogicalRect { x: 120.0, ..ANCHOR };
        let (_, again) = prepared(state.toggle(Panel::Settings, moved, at(t, 60)));

        // The answers to the first A and to B arrive late.
        assert_eq!(state.ready(PanelId::Settings, a, RENDERED), Action::Nothing);
        assert_eq!(state.ready(PanelId::About, b, RENDERED), Action::Nothing);
        // An answer to a show that never happened is ignored too.
        assert_eq!(
            state.ready(PanelId::Settings, again + 1, RENDERED),
            Action::Nothing
        );
        // Still waiting: only the answer to the latest show reveals it.
        assert_eq!(
            state.ready(PanelId::Settings, again, RENDERED),
            Action::Reveal {
                anchor: moved,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn ready_without_an_open_panel_is_ignored() {
        let mut state = PanelState::default();
        assert_eq!(state.ready(PanelId::Settings, 1, RENDERED), Action::Nothing);

        let (_, wait) = prepared(state.toggle(Panel::Settings, ANCHOR, Instant::now()));
        state.hide();
        assert_eq!(
            state.ready(PanelId::Settings, wait, RENDERED),
            Action::Nothing
        );
    }

    #[test]
    fn time_out_reveals_the_waiting_panel_at_its_fallback_size() {
        let mut state = PanelState::default();
        let (_, wait) = prepared(state.toggle(Panel::Settings, ANCHOR, Instant::now()));

        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: PanelGeometry::fallback(&Panel::Settings)
            }
        );
        assert_eq!(
            PanelGeometry::fallback(&Panel::Settings).size,
            LogicalSize {
                width: 336.0,
                height: 406.0
            }
        );
    }

    #[test]
    fn time_out_reveals_at_the_last_size_the_page_reported() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.hide();

        let (_, wait) = prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 1000)));

        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn time_out_after_ready_or_of_an_older_wait_does_nothing() {
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, first) = prepared(state.toggle(Panel::Settings, ANCHOR, t));
        state.ready(PanelId::Settings, first, RENDERED);
        assert_eq!(state.time_out(first), Action::Nothing);

        state.hide();
        let (_, second) = prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 1000)));
        assert_ne!(first, second);
        // The first wait's timer fires late: the second wait is untouched.
        assert_eq!(state.time_out(first), Action::Nothing);
        assert!(matches!(state.time_out(second), Action::Reveal { .. }));
    }

    #[test]
    fn toggle_of_the_waiting_panel_cancels_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, wait) = prepared(state.toggle(Panel::Settings, ANCHOR, t));

        assert_eq!(
            state.toggle(Panel::Settings, ANCHOR, at(t, 100)),
            Action::Hide
        );
        assert_eq!(state.time_out(wait), Action::Nothing);
        assert_eq!(
            state.ready(PanelId::Settings, wait, RENDERED),
            Action::Nothing
        );
        assert_eq!(state.open(), None);
    }

    #[test]
    fn toggle_of_another_panel_while_waiting_starts_a_new_wait() {
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, first) = prepared(state.toggle(Panel::Settings, ANCHOR, t));

        let (id, second) = prepared(state.toggle(Panel::About, ANCHOR, at(t, 100)));

        assert_eq!(id, PanelId::About);
        assert_eq!(state.time_out(first), Action::Nothing);
        assert!(matches!(state.time_out(second), Action::Reveal { .. }));
    }

    #[test]
    fn blur_while_waiting_is_ignored() {
        // Hiding the window for the wait takes its focus.
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, wait) = prepared(state.toggle(Panel::Settings, ANCHOR, t));

        assert_eq!(state.blur(at(t, 10)), Action::Nothing);
        assert_eq!(state.open(), Some(PanelId::Settings));
        assert!(matches!(
            state.ready(PanelId::Settings, wait, RENDERED),
            Action::Reveal { .. }
        ));
        // Revealed, a blur hides it again.
        assert_eq!(state.blur(at(t, 500)), Action::Hide);
    }

    #[test]
    fn hide_while_waiting_closes_and_ends_the_wait() {
        let mut state = PanelState::default();
        let (_, wait) = prepared(state.toggle(Panel::About, ANCHOR, Instant::now()));

        assert_eq!(state.hide(), Action::Hide);
        assert_eq!(state.time_out(wait), Action::Nothing);
        assert_eq!(state.ready(PanelId::About, wait, RENDERED), Action::Nothing);
    }

    #[test]
    fn ready_for_the_panel_on_screen_places_it_again() {
        let mut state = PanelState::default();
        let wait = show(&mut state, Panel::Settings, Instant::now());
        let taller = PanelGeometry {
            size: LogicalSize {
                width: 392.0,
                height: 600.0,
            },
            inset: 8.0,
        };

        assert_eq!(
            state.ready(PanelId::Settings, wait, taller),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: taller
            }
        );
        assert_eq!(state.ready(PanelId::About, wait, taller), Action::Nothing);
    }

    #[test]
    fn toggle_anchors_the_panel_to_the_clicked_item() {
        let mut state = PanelState::default();
        let item = LogicalRect { x: 90.0, ..ANCHOR };
        let (_, wait) = prepared(state.toggle(Panel::About, item, Instant::now()));

        assert_eq!(
            state.ready(PanelId::About, wait, RENDERED),
            Action::Reveal {
                anchor: item,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn toggle_of_the_open_panel_hides_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);

        assert_eq!(
            state.toggle(Panel::Settings, ANCHOR, at(t, 1000)),
            Action::Hide
        );
        assert_eq!(
            prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 1100))).0,
            PanelId::Settings
        );
    }

    #[test]
    fn toggle_of_another_panel_switches_content() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);

        assert_eq!(
            prepared(state.toggle(Panel::About, ANCHOR, at(t, 50))).0,
            PanelId::About
        );
        assert_eq!(state.toggle(Panel::About, ANCHOR, at(t, 100)), Action::Hide);
    }

    #[test]
    fn blur_hides_the_open_panel() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::About, t);

        assert_eq!(state.blur(at(t, 10)), Action::Hide);
        assert_eq!(state.blur(at(t, 20)), Action::Nothing);
    }

    #[test]
    fn blur_without_an_open_panel_does_nothing() {
        let mut state = PanelState::default();

        assert_eq!(state.blur(Instant::now()), Action::Nothing);
    }

    #[test]
    fn toggle_right_after_a_blur_hide_of_the_same_panel_keeps_it_closed() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            state.toggle(Panel::Settings, ANCHOR, at(t, 1299)),
            Action::Nothing
        );
        // The guard is consumed: the next click opens again.
        assert_eq!(
            prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 1350))).0,
            PanelId::Settings
        );
    }

    #[test]
    fn toggle_300ms_or_more_after_a_blur_hide_reopens() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 1300))).0,
            PanelId::Settings
        );
    }

    #[test]
    fn toggle_of_another_panel_right_after_a_blur_hide_opens_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.blur(at(t, 1000));

        assert_eq!(
            prepared(state.toggle(Panel::About, ANCHOR, at(t, 1050))).0,
            PanelId::About
        );
    }

    #[test]
    fn hide_closes_the_open_panel_without_arming_the_reopen_guard() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);

        assert_eq!(state.hide(), Action::Hide);
        // The focus loss caused by hiding must not arm the guard.
        assert_eq!(state.blur(at(t, 10)), Action::Nothing);
        assert_eq!(
            prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 50))).0,
            PanelId::Settings
        );
    }

    #[test]
    fn hide_after_a_blur_hide_keeps_the_reopen_guard() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.blur(at(t, 1000));

        // A late hide (Esc or the dock hiding) after the blur-hide.
        assert_eq!(state.hide(), Action::Nothing);
        // The click that closed the panel must not reopen it.
        assert_eq!(
            state.toggle(Panel::Settings, ANCHOR, at(t, 1100)),
            Action::Nothing
        );
    }

    #[test]
    fn hide_without_an_open_panel_does_nothing() {
        let mut state = PanelState::default();

        assert_eq!(state.hide(), Action::Nothing);
    }

    #[test]
    fn moving_the_dock_closes_the_open_panel_and_remembers_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);

        assert_eq!(state.close_for_move(t), Action::Hide);
        assert_eq!(state.open(), None);
        assert_eq!(
            state.dock_settled(false, at(t, 50)),
            Some(PanelId::Settings)
        );
    }

    #[test]
    fn moving_the_dock_closes_a_waiting_panel_and_remembers_it() {
        let mut state = PanelState::default();
        let t = Instant::now();
        let (_, wait) = prepared(state.toggle(Panel::About, ANCHOR, t));

        assert_eq!(state.close_for_move(t), Action::Hide);
        assert_eq!(state.time_out(wait), Action::Nothing);
        assert_eq!(state.dock_settled(false, at(t, 50)), Some(PanelId::About));
    }

    #[test]
    fn moving_the_dock_without_an_open_panel_reopens_nothing() {
        let mut state = PanelState::default();
        let t = Instant::now();

        assert_eq!(state.close_for_move(t), Action::Nothing);
        assert_eq!(state.dock_settled(false, t), None);
    }

    #[test]
    fn reopen_waits_while_the_layout_resizes_or_moves_the_dock() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        // The page measured the old window: its items are not where they
        // will be.
        assert_eq!(state.dock_settled(true, at(t, 20)), None);
        assert_eq!(state.dock_settled(true, at(t, 40)), None);
        assert_eq!(
            state.dock_settled(false, at(t, 60)),
            Some(PanelId::Settings)
        );
    }

    #[test]
    fn reopen_is_handed_out_once() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        assert_eq!(
            state.dock_settled(false, at(t, 20)),
            Some(PanelId::Settings)
        );
        assert_eq!(state.dock_settled(false, at(t, 40)), None);
        assert_eq!(state.dock_settled(true, at(t, 60)), None);
    }

    #[test]
    fn a_second_move_before_the_dock_settles_keeps_the_reopen() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::About, t);
        state.close_for_move(t);

        assert_eq!(state.close_for_move(at(t, 900)), Action::Nothing);
        // The second move restarts the wait.
        assert_eq!(state.dock_settled(false, at(t, 1500)), Some(PanelId::About));
    }

    #[test]
    fn a_reopen_the_dock_never_settles_for_expires() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        // Only layouts that resize the window, then nothing for a second.
        assert_eq!(state.dock_settled(true, at(t, 500)), None);
        assert_eq!(state.dock_settled(false, at(t, 1001)), None);
        // Dropped, not just withheld.
        assert_eq!(state.dock_settled(false, at(t, 1002)), None);
    }

    #[test]
    fn a_reopen_settled_just_in_time_is_handed_out() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        assert_eq!(REOPEN_TIMEOUT, Duration::from_secs(1));
        assert_eq!(
            state.dock_settled(false, at(t, 1000)),
            Some(PanelId::Settings)
        );
    }

    #[test]
    fn a_reopen_expired_while_closed_is_not_revived_by_a_later_move() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        // Long after, the size changes again with no panel open.
        assert_eq!(state.close_for_move(at(t, 5000)), Action::Nothing);
        assert_eq!(state.dock_settled(false, at(t, 5050)), None);
    }

    #[test]
    fn explicit_close_or_hide_drops_the_reopen() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        // Esc, Alt+F4 or the dock hiding.
        assert_eq!(state.hide(), Action::Nothing);
        assert_eq!(state.dock_settled(false, at(t, 50)), None);
    }

    #[test]
    fn a_click_on_an_item_replaces_the_reopen() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);

        let (id, _) = prepared(state.toggle(Panel::About, ANCHOR, at(t, 100)));

        assert_eq!(id, PanelId::About);
        assert_eq!(state.dock_settled(false, at(t, 150)), None);
    }

    #[test]
    fn the_reopened_panel_opens_like_a_click() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, Panel::Settings, t);
        state.close_for_move(t);
        // Hiding the window for the move takes its focus.
        assert_eq!(state.blur(at(t, 1)), Action::Nothing);
        let reopen = state.dock_settled(false, at(t, 10)).unwrap();

        // The dock answers `dock-request-panel` with a toggle at the item's
        // new place, right away: no reopen guard applies.
        let moved = LogicalRect { x: 120.0, ..ANCHOR };
        assert_eq!(reopen, PanelId::Settings);
        let (_, wait) = prepared(state.toggle(Panel::Settings, moved, at(t, 20)));
        assert_eq!(
            state.ready(reopen, wait, RENDERED),
            Action::Reveal {
                anchor: moved,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn toggle_prepares_a_widget_panel_by_its_instance() {
        let mut state = PanelState::default();
        let (id, wait) =
            prepared(state.toggle(widget(WIDGET, WidgetKind::Note), ANCHOR, Instant::now()));

        assert_eq!(id, widget_id(WIDGET));
        assert_eq!(state.open(), Some(widget_id(WIDGET)));
        assert_eq!(
            state.ready(widget_id(WIDGET), wait, RENDERED),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn two_instances_of_a_kind_are_two_panels() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, widget(WIDGET, WidgetKind::Clock), t);

        let (id, wait) =
            prepared(state.toggle(widget(OTHER_WIDGET, WidgetKind::Clock), ANCHOR, at(t, 50)));
        assert_eq!(id, widget_id(OTHER_WIDGET));
        // The first instance's answer is stale.
        assert_eq!(
            state.ready(widget_id(WIDGET), wait, RENDERED),
            Action::Nothing
        );
        assert_eq!(
            state.toggle(widget(OTHER_WIDGET, WidgetKind::Clock), ANCHOR, at(t, 100)),
            Action::Hide
        );
    }

    #[test]
    fn a_widget_and_a_system_panel_switch_like_any_two_panels() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, widget(WIDGET, WidgetKind::Note), t);

        assert_eq!(
            prepared(state.toggle(Panel::Settings, ANCHOR, at(t, 50))).0,
            PanelId::Settings
        );
        assert_eq!(
            prepared(state.toggle(widget(WIDGET, WidgetKind::Note), ANCHOR, at(t, 100))).0,
            widget_id(WIDGET)
        );
    }

    #[test]
    fn the_reopen_guard_is_per_instance() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, widget(WIDGET, WidgetKind::Clock), t);
        assert_eq!(state.blur(at(t, 1000)), Action::Hide);

        // The click that closed it keeps it closed...
        assert_eq!(
            state.toggle(widget(WIDGET, WidgetKind::Clock), ANCHOR, at(t, 1100)),
            Action::Nothing
        );
        show(&mut state, widget(WIDGET, WidgetKind::Clock), at(t, 1200));
        assert_eq!(state.blur(at(t, 2000)), Action::Hide);
        // ...but a click on another instance opens that one.
        assert_eq!(
            prepared(state.toggle(widget(OTHER_WIDGET, WidgetKind::Clock), ANCHOR, at(t, 2050))).0,
            widget_id(OTHER_WIDGET)
        );
    }

    #[test]
    fn time_out_reveals_a_widget_panel_at_the_size_of_its_kind() {
        let mut state = PanelState::default();
        let note = widget(WIDGET, WidgetKind::Note);
        let (_, wait) = prepared(state.toggle(note.clone(), ANCHOR, Instant::now()));

        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: PanelGeometry::fallback(&note)
            }
        );
        assert_eq!(PanelGeometry::fallback(&note).size.width, 356.0);
    }

    #[test]
    fn each_instance_keeps_the_size_its_page_reported() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, widget(WIDGET, WidgetKind::Clock), t);
        state.hide();

        // Another instance of the same kind never reported one.
        let other = widget(OTHER_WIDGET, WidgetKind::Clock);
        let (_, wait) = prepared(state.toggle(other.clone(), ANCHOR, at(t, 100)));
        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: PanelGeometry::fallback(&other)
            }
        );
        state.hide();

        let (_, wait) =
            prepared(state.toggle(widget(WIDGET, WidgetKind::Clock), ANCHOR, at(t, 200)));
        assert_eq!(
            state.time_out(wait),
            Action::Reveal {
                anchor: ANCHOR,
                geometry: RENDERED
            }
        );
    }

    #[test]
    fn moving_the_dock_reopens_the_widget_panel_by_its_id() {
        let mut state = PanelState::default();
        let t = Instant::now();
        show(&mut state, widget(WIDGET, WidgetKind::TimeProgress), t);

        assert_eq!(state.close_for_move(t), Action::Hide);
        // The dock toggles it again, so `toggle_panel` checks the instance
        // again before it reopens.
        assert_eq!(
            state.dock_settled(false, at(t, 50)),
            Some(widget_id(WIDGET))
        );
    }
}
