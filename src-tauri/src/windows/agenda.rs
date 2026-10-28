//! The Calendário's card (spec 0017 F2a-4): over the dock while the pointer
//! is on the Calendário, the meetings of a period and the day's tasks, in
//! rows that scroll with the wheel; a click on a meeting opens its details
//! in the panel. It is its own window, as the previews' card (spec 0015):
//! the dock's is no taller than the bar. A pop-up that never takes the
//! focus, out of Alt+Tab, with Windows' rounded corners and shadow, drawn
//! with Direct2D in physical pixels. On the main thread, like the dock.
//!
//! A mouse convenience: the panel has the same meetings and tasks for the
//! keyboard and assistive technology.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    D2DERR_RECREATE_TARGET, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_FEATURE_LEVEL_DEFAULT, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT, D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget,
    ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DROPSHADOW, CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos, HWND_TOPMOST,
    IDC_ARROW, KillTimer, LoadCursorW, MA_NOACTIVATE, RegisterClassW, SW_HIDE, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetTimer, SetWindowPos, ShowWindow, WM_ERASEBKGND, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::domain::agenda::{self, Row};
use crate::domain::dock_layout::Area;
use crate::domain::settings::DockEdge;
use crate::domain::widgets::WidgetId;

const CLASS: PCWSTR = w!("DockMaxAgenda");

/// While the card shows, the pointer is watched: once it has been off the
/// Calendário and off the card for `LEAVE_GRACE`, the card goes.
const WATCH_TIMER: usize = 1;
const WATCH_EVERY: u32 = 50;
const LEAVE_GRACE: Duration = Duration::from_millis(300);

/// Logical pixels between the Calendário and the card.
const GAP: f64 = 10.0;

/// What a dock asks to show.
#[derive(Clone)]
pub struct Request {
    /// The dock's window: the card belongs to it.
    pub owner: isize,
    /// The Calendário whose card it is.
    pub widget: WidgetId,
    /// The Calendário and the monitor, on the screen, in physical pixels.
    pub tile: Area,
    pub screen: Area,
    pub edge: DockEdge,
    pub scale: f64,
    pub dark: bool,
    pub rows: Vec<Row>,
}

/// A card on the screen.
struct Shown {
    request: Request,
    /// The card, on the screen, in physical pixels.
    area: Area,
    /// The content's height and the scroll, in logical pixels.
    content: f64,
    scroll: f64,
    hovered: Option<usize>,
    away_since: Option<Instant>,
}

struct Card {
    hwnd: HWND,
    painter: Painter,
    shown: Option<Shown>,
}

thread_local! {
    static CARD: RefCell<Option<Card>> = const { RefCell::new(None) };
}

/// The window, made the first time.
fn window() -> Option<HWND> {
    CARD.with_borrow_mut(|card| {
        if let Some(card) = card {
            return Some(card.hwnd);
        }
        let hwnd = make_window()
            .inspect_err(|error| log::warn!("agenda: the card can not be made: {error}"))
            .ok()?;
        let painter = Painter::new(hwnd)
            .inspect_err(|error| log::warn!("agenda: nothing to paint with: {error}"))
            .ok()?;
        *card = Some(Card {
            hwnd,
            painter,
            shown: None,
        });
        Some(hwnd)
    })
}

fn make_window() -> windows::core::Result<HWND> {
    let instance = unsafe { GetModuleHandleW(None) }?;
    let class_data = WNDCLASSW {
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(procedure),
        hInstance: instance.into(),
        lpszClassName: CLASS,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class_data) };
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            CLASS,
            w!(""),
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
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&DWMWCP_ROUND).cast(),
            size_of_val(&DWMWCP_ROUND) as u32,
        );
    }
    Ok(hwnd)
}

/// Where the card goes: centered over the Calendário, away from the screen
/// edge the dock is on, kept on the monitor.
pub fn place(tile: Area, screen: Area, edge: DockEdge, scale: f64, height: f64) -> Area {
    let width = agenda::WIDTH * scale;
    let height = height * scale;
    let x = (tile.x + tile.width / 2.0 - width / 2.0)
        .clamp(screen.x, (screen.x + screen.width - width).max(screen.x));
    let y = match edge {
        DockEdge::Bottom => tile.y - GAP * scale - height,
        DockEdge::Top => tile.y + tile.height + GAP * scale,
    };
    Area {
        x: x.round(),
        y: y.round(),
        width: width.round(),
        height: height.round(),
    }
}

/// Shows the card `request` asks for, in place of any other.
pub fn show(request: Request) {
    let Some(hwnd) = window() else {
        return;
    };
    let (_, content) = agenda::tops(&request.rows);
    let area = place(
        request.tile,
        request.screen,
        request.edge,
        request.scale,
        agenda::height(content),
    );
    // The same Calendário again (its rows read again): the scroll stays.
    let scroll = CARD.with_borrow(|card| {
        let shown = card.as_ref()?.shown.as_ref()?;
        (shown.request.widget == request.widget).then_some(shown.scroll)
    });
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            area.x as i32,
            area.y as i32,
            area.width as i32,
            area.height as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    CARD.with_borrow_mut(|card| {
        if let Some(card) = card {
            card.painter.resize();
            card.shown = Some(Shown {
                request,
                area,
                content,
                scroll: agenda::clamp_scroll(scroll.unwrap_or(0.0), content),
                hovered: None,
                away_since: None,
            });
        }
    });
    unsafe {
        SetTimer(Some(hwnd), WATCH_TIMER, WATCH_EVERY, None);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

/// Hides the card, if one shows.
pub fn hide() {
    let hwnd = CARD.with_borrow_mut(|card| {
        let card = card.as_mut()?;
        card.shown.take()?;
        Some(card.hwnd)
    });
    if let Some(hwnd) = hwnd {
        unsafe {
            let _ = KillTimer(Some(hwnd), WATCH_TIMER);
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Whether a card shows, and for which dock.
pub fn shown_by() -> Option<isize> {
    CARD.with_borrow(|card| Some(card.as_ref()?.shown.as_ref()?.request.owner))
}

fn contains(area: Area, point: POINT) -> bool {
    let (x, y) = (f64::from(point.x), f64::from(point.y));
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

/// Every `WATCH_EVERY`: off the Calendário, the card and the gap between
/// them for `LEAVE_GRACE`, the card goes.
fn watch() {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let gone = CARD.with_borrow_mut(|card| {
        let shown = card.as_mut()?.shown.as_mut()?;
        let tile = shown.request.tile;
        let card = shown.area;
        let top = tile.y.min(card.y);
        let bottom = (tile.y + tile.height).max(card.y + card.height);
        let bridge = Area {
            x: tile.x,
            y: top,
            width: tile.width,
            height: bottom - top,
        };
        if contains(card, point) || contains(tile, point) || contains(bridge, point) {
            shown.away_since = None;
            return Some(false);
        }
        let since = *shown.away_since.get_or_insert_with(Instant::now);
        Some(since.elapsed() >= LEAVE_GRACE)
    });
    if gone == Some(true) {
        hide();
    }
}

/// The row under `(x, y)`, in the card's physical pixels.
fn row_at(shown: &Shown, y: f64) -> Option<usize> {
    agenda::hit(&shown.request.rows, y / shown.request.scale, shown.scroll)
}

fn hover(hwnd: HWND, y: f64) {
    let changed = CARD.with_borrow_mut(|card| {
        let shown = card.as_mut()?.shown.as_mut()?;
        let hovered = row_at(shown, y).filter(|&index| {
            matches!(
                shown.request.rows.get(index),
                Some(Row::Event { .. } | Row::Task { .. })
            )
        });
        let changed = hovered != shown.hovered;
        shown.hovered = hovered;
        Some(changed)
    });
    if changed == Some(true) {
        let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
    }
}

/// The wheel: `delta` in Windows' notches of 120, negative toward the user.
fn wheel(hwnd: HWND, delta: i16) {
    CARD.with_borrow_mut(|card| {
        if let Some(shown) = card.as_mut().and_then(|card| card.shown.as_mut()) {
            let by = -f64::from(delta) / 120.0 * agenda::LINE * 1.5;
            shown.scroll = agenda::clamp_scroll(shown.scroll + by, shown.content);
        }
    });
    let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
}

/// A click: a meeting opens its details in the panel; a task, the panel.
fn click(y: f64) {
    let target = CARD.with_borrow(|card| {
        let shown = card.as_ref()?.shown.as_ref()?;
        let index = row_at(shown, y)?;
        let event = match shown.request.rows.get(index)? {
            Row::Event { id, .. } => Some(id.clone()),
            Row::Task { .. } => None,
            _ => return None,
        };
        Some((shown.request.owner, shown.request.widget.clone(), event))
    });
    if let Some((owner, widget, event)) = target {
        hide();
        super::native_dock::open_calendar(owner, widget, event);
    }
}

fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    unsafe { BeginPaint(hwnd, &mut paint) };
    CARD.with_borrow_mut(|card| {
        if let Some(Card {
            painter,
            shown: Some(shown),
            ..
        }) = card
            && !painter.paint(shown)
        {
            painter.paint(shown);
        }
    });
    let _ = unsafe { EndPaint(hwnd, &paint) };
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // A click never takes the focus from the window in front.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            hover(hwnd, f64::from(((lparam.0 >> 16) & 0xFFFF) as u16 as i16));
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            wheel(hwnd, ((wparam.0 >> 16) & 0xFFFF) as u16 as i16);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            click(f64::from(((lparam.0 >> 16) & 0xFFFF) as u16 as i16));
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == WATCH_TIMER => {
            watch();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// The card's colors: the previews' (spec 0015), light or dark.
struct Colors {
    background: D2D1_COLOR_F,
    text: D2D1_COLOR_F,
    dim: D2D1_COLOR_F,
    hover: D2D1_COLOR_F,
    late: D2D1_COLOR_F,
    accent: D2D1_COLOR_F,
}

const fn rgb(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

fn colors(dark: bool) -> Colors {
    if dark {
        Colors {
            background: rgb(0x24, 0x24, 0x26, 1.0),
            text: rgb(0xF5, 0xF5, 0xF7, 1.0),
            dim: rgb(0xA1, 0xA1, 0xA6, 1.0),
            hover: rgb(0xFF, 0xFF, 0xFF, 0.08),
            late: rgb(0xFF, 0x8A, 0x80, 1.0),
            accent: rgb(0x7C, 0x9C, 0xFF, 1.0),
        }
    } else {
        Colors {
            background: rgb(0xF6, 0xF6, 0xF8, 1.0),
            text: rgb(0x1C, 0x1C, 0x1E, 1.0),
            dim: rgb(0x6E, 0x6E, 0x73, 1.0),
            hover: rgb(0x00, 0x00, 0x00, 0.06),
            late: rgb(0xB4, 0x23, 0x18, 1.0),
            accent: rgb(0x2F, 0x5B, 0xD3, 1.0),
        }
    }
}

/// `#rrggbb` as a color.
fn hex(text: &str) -> Option<D2D1_COLOR_F> {
    let digits = text.strip_prefix('#').filter(|digits| digits.len() == 6)?;
    let part = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some(rgb(part(0)?, part(2)?, part(4)?, 1.0))
}

fn rect(area: Area) -> D2D_RECT_F {
    D2D_RECT_F {
        left: area.x as f32,
        top: area.y as f32,
        right: (area.x + area.width) as f32,
        bottom: (area.y + area.height) as f32,
    }
}

/// Text styles by size (hundredths of a pixel), weight and alignment.
type FormatKey = (u32, i32, i32);

/// Draws the card in physical pixels: the target stays at 96 DPI.
struct Painter {
    factory: ID2D1Factory,
    write: IDWriteFactory,
    target: Option<ID2D1HwndRenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
    formats: HashMap<FormatKey, IDWriteTextFormat>,
    hwnd: HWND,
}

impl Painter {
    fn new(hwnd: HWND) -> windows::core::Result<Painter> {
        Ok(Painter {
            factory: unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }?,
            write: unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?,
            target: None,
            brush: None,
            formats: HashMap::new(),
            hwnd,
        })
    }

    fn client_size(&self) -> D2D_SIZE_U {
        let mut client = RECT::default();
        let _ = unsafe { GetClientRect(self.hwnd, &mut client) };
        D2D_SIZE_U {
            width: u32::try_from(client.right - client.left)
                .unwrap_or(0)
                .max(1),
            height: u32::try_from(client.bottom - client.top)
                .unwrap_or(0)
                .max(1),
        }
    }

    fn resize(&mut self) {
        if let Some(target) = &self.target {
            let size = self.client_size();
            if unsafe { target.Resize(&size) }.is_err() {
                self.target = None;
                self.brush = None;
            }
        }
    }

    fn target(&mut self) -> Option<ID2D1HwndRenderTarget> {
        if self.target.is_none() {
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                // Software: a few lines of text, as the previews' card.
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let window = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: self.client_size(),
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            let target = unsafe { self.factory.CreateHwndRenderTarget(&properties, &window) }
                .inspect_err(|error| log::warn!("agenda: nothing to paint with: {error}"))
                .ok()?;
            unsafe { target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
            self.brush = unsafe { target.CreateSolidColorBrush(&rgb(0, 0, 0, 1.0), None) }.ok();
            self.target = Some(target);
        }
        self.target.clone()
    }

    fn format(
        &mut self,
        size: f64,
        weight: DWRITE_FONT_WEIGHT,
        align: DWRITE_TEXT_ALIGNMENT,
    ) -> Option<IDWriteTextFormat> {
        let key = ((size * 100.0).round() as u32, weight.0, align.0);
        if let Some(found) = self.formats.get(&key) {
            return Some(found.clone());
        }
        let format = unsafe {
            self.write.CreateTextFormat(
                w!("Segoe UI Variable Text"),
                None,
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size as f32,
                w!("pt-BR"),
            )
        }
        .ok()?;
        unsafe {
            let _ = format.SetTextAlignment(align);
            let _ = format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            // A long title ends in "…".
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            if let Ok(sign) = self.write.CreateEllipsisTrimmingSign(&format) {
                let _ = format.SetTrimming(&trimming, &sign);
            }
        }
        self.formats.insert(key, format.clone());
        Some(format)
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        target: &ID2D1HwndRenderTarget,
        text: &str,
        area: Area,
        size: f64,
        weight: DWRITE_FONT_WEIGHT,
        align: DWRITE_TEXT_ALIGNMENT,
        ink: D2D1_COLOR_F,
    ) {
        let Some(format) = self.format(size, weight, align) else {
            return;
        };
        let Some(brush) = &self.brush else {
            return;
        };
        let wide: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            brush.SetColor(&ink);
            target.DrawText(
                &wide,
                &format,
                &rect(area),
                brush,
                // The tasks' emoji in their colors.
                D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    fn fill(&self, target: &ID2D1HwndRenderTarget, area: Area, radius: f64, ink: D2D1_COLOR_F) {
        let Some(brush) = &self.brush else {
            return;
        };
        unsafe {
            brush.SetColor(&ink);
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rect(area),
                    radiusX: radius as f32,
                    radiusY: radius as f32,
                },
                brush,
            );
        }
    }

    fn dot(&self, target: &ID2D1HwndRenderTarget, x: f64, y: f64, radius: f64, ink: D2D1_COLOR_F) {
        let Some(brush) = &self.brush else {
            return;
        };
        unsafe {
            brush.SetColor(&ink);
            target.FillEllipse(
                &D2D1_ELLIPSE {
                    point: windows_numerics::Vector2 {
                        X: x as f32,
                        Y: y as f32,
                    },
                    radiusX: radius as f32,
                    radiusY: radius as f32,
                },
                brush,
            );
        }
    }

    /// Draws the card; false when Direct2D lost the device (the target is
    /// made again at the next paint).
    fn paint(&mut self, shown: &Shown) -> bool {
        let Some(target) = self.target() else {
            return true;
        };
        let c = colors(shown.request.dark);
        let s = shown.request.scale;
        let width = shown.area.width;
        unsafe {
            target.BeginDraw();
            target.Clear(Some(&c.background));
        }
        let (tops, _) = agenda::tops(&shown.request.rows);
        let inset = 14.0 * s;
        for (index, (row, top)) in shown.request.rows.iter().zip(tops).enumerate() {
            let y = (top - shown.scroll) * s;
            let tall = if row.is_heading() {
                agenda::HEADING
            } else {
                agenda::LINE
            } * s;
            if y + tall < 0.0 || y > shown.area.height {
                continue;
            }
            let line = Area {
                x: inset,
                y,
                width: width - 2.0 * inset,
                height: tall,
            };
            if shown.hovered == Some(index) {
                let back = Area {
                    x: 6.0 * s,
                    width: width - 12.0 * s,
                    ..line
                };
                self.fill(&target, back, 6.0 * s, c.hover);
            }
            match row {
                Row::Day(name) | Row::Tasks(name) => {
                    self.text(
                        &target,
                        &name.to_uppercase(),
                        Area {
                            y: y + 4.0 * s,
                            height: tall - 4.0 * s,
                            ..line
                        },
                        10.5 * s,
                        DWRITE_FONT_WEIGHT_SEMI_BOLD,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        c.dim,
                    );
                }
                Row::Free(text) => {
                    self.text(
                        &target,
                        text,
                        line,
                        12.5 * s,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        c.dim,
                    );
                }
                Row::Event {
                    color, when, title, ..
                } => {
                    let ink = color.as_deref().and_then(hex).unwrap_or(c.accent);
                    self.dot(&target, inset + 4.0 * s, y + tall / 2.0, 4.0 * s, ink);
                    let time = 74.0 * s;
                    self.text(
                        &target,
                        when,
                        Area {
                            x: inset + 14.0 * s,
                            width: time - 14.0 * s,
                            ..line
                        },
                        12.0 * s,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        c.dim,
                    );
                    self.text(
                        &target,
                        title,
                        Area {
                            x: inset + time,
                            width: (line.width - time).max(0.0),
                            ..line
                        },
                        12.5 * s,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        c.text,
                    );
                }
                Row::Task { title, late } => {
                    // An empty box, as the panel's.
                    let side = 12.0 * s;
                    let boxed = Area {
                        x: inset,
                        y: y + (tall - side) / 2.0,
                        width: side,
                        height: side,
                    };
                    self.fill(&target, boxed, 3.0 * s, c.dim);
                    self.fill(
                        &target,
                        Area {
                            x: boxed.x + 1.5 * s,
                            y: boxed.y + 1.5 * s,
                            width: side - 3.0 * s,
                            height: side - 3.0 * s,
                        },
                        2.0 * s,
                        c.background,
                    );
                    let mark = if *late { 64.0 * s } else { 0.0 };
                    self.text(
                        &target,
                        title,
                        Area {
                            x: inset + 22.0 * s,
                            width: (line.width - 22.0 * s - mark).max(0.0),
                            ..line
                        },
                        12.5 * s,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        c.text,
                    );
                    if *late {
                        self.text(
                            &target,
                            crate::i18n::messages().dock.calendar_late,
                            line,
                            11.0 * s,
                            DWRITE_FONT_WEIGHT_NORMAL,
                            DWRITE_TEXT_ALIGNMENT_TRAILING,
                            c.late,
                        );
                    }
                }
            }
        }
        match unsafe { target.EndDraw(None, None) } {
            Err(error) if error.code() == D2DERR_RECREATE_TARGET => {
                self.target = None;
                self.brush = None;
                false
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: f64, y: f64, width: f64, height: f64) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn the_card_rises_over_the_calendario_and_stays_on_the_monitor() {
        let screen = area(0.0, 0.0, 1920.0, 1080.0);
        let tile = area(900.0, 1000.0, 120.0, 56.0);
        let card = place(tile, screen, DockEdge::Bottom, 1.0, 200.0);
        assert_eq!(card.width, agenda::WIDTH);
        assert_eq!(card.x, 960.0 - agenda::WIDTH / 2.0);
        assert_eq!(card.y + card.height, 1000.0 - GAP);
        // At the screen's left edge: moved in.
        let left = place(
            area(0.0, 1000.0, 56.0, 56.0),
            screen,
            DockEdge::Bottom,
            1.0,
            200.0,
        );
        assert_eq!(left.x, 0.0);
        // A dock at the top: under it, at 150 %.
        let top = place(
            area(900.0, 0.0, 84.0, 84.0),
            screen,
            DockEdge::Top,
            1.5,
            200.0,
        );
        assert_eq!(top.y, 84.0 + GAP * 1.5);
        assert_eq!(top.height, 300.0);
    }

    #[test]
    fn a_calendars_color_reads_from_hex() {
        assert_eq!(hex("#ff0000").map(|c| c.r), Some(1.0));
        assert_eq!(hex("red"), None);
        assert_eq!(hex("#fff"), None);
    }
}
