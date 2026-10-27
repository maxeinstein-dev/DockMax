//! The card of the window previews (spec 0015): over a program's icon, a live
//! thumbnail of each of its windows, with its title and a close button.
//!
//! The thumbnails are the DWM's (`DwmRegisterThumbnail`): Windows draws the
//! window itself into the card, nothing is copied here. The DWM needs a plain
//! window to draw into, so the card is one of its own, not the dock's layered
//! one: a pop-up that never takes the focus, out of Alt+Tab, with Windows'
//! rounded corners and shadow. Its background, the titles and the buttons
//! are drawn with Direct2D, in physical pixels (the layout in
//! `domain/previews.rs` is in them too).
//!
//! Everything lives on the main thread, like the dock that asks for it. The
//! window is made the first time a card shows and kept, hidden, after.

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
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmSetWindowAttribute,
    DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DROPSHADOW, CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos,
    GetWindowTextLengthW, GetWindowTextW, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW,
    MA_NOACTIVATE, RegisterClassW, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetTimer, SetWindowPos,
    ShowWindow, WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_PAINT, WM_TIMER,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::domain::dock_layout::Area;
use crate::domain::previews::{self, Card, Hit};
use crate::domain::settings::DockEdge;

const CLASS: PCWSTR = w!("DockMaxPreview");

/// While the card shows, the pointer is watched: once it has been off the
/// icon and off the card for `LEAVE_GRACE`, the card goes.
const WATCH_TIMER: usize = 1;
const WATCH_EVERY: u32 = 50;
const LEAVE_GRACE: Duration = Duration::from_millis(300);

/// What a dock asks to show.
pub struct Request {
    /// The dock's window: the card belongs to it.
    pub owner: isize,
    /// The program's icon and the monitor, on the screen, in physical pixels.
    pub icon: Area,
    pub screen: Area,
    pub edge: DockEdge,
    pub scale: f64,
    pub dark: bool,
    /// The windows, the front one first.
    pub windows: Vec<isize>,
}

/// A card on the screen.
struct Shown {
    request: Request,
    titles: Vec<String>,
    /// The DWM's thumbnail of each window of `card.cells`, by index.
    thumbs: Vec<isize>,
    card: Card,
    hovered: Option<Hit>,
    away_since: Option<Instant>,
}

struct Preview {
    hwnd: HWND,
    painter: Painter,
    shown: Option<Shown>,
}

thread_local! {
    static PREVIEW: RefCell<Option<Preview>> = const { RefCell::new(None) };
}

/// The window, made the first time.
fn window() -> Option<HWND> {
    PREVIEW.with_borrow_mut(|preview| {
        if let Some(preview) = preview {
            return Some(preview.hwnd);
        }
        let hwnd = make_window()
            .inspect_err(|error| log::warn!("previews: the card can not be made: {error}"))
            .ok()?;
        let painter = Painter::new(hwnd)
            .inspect_err(|error| log::warn!("previews: nothing to paint with: {error}"))
            .ok()?;
        *preview = Some(Preview {
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

/// The title of the window `value`; empty when it has none.
fn title_of(value: isize) -> String {
    let window = HWND(value as _);
    let length = unsafe { GetWindowTextLengthW(window) };
    if length <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let read = unsafe { GetWindowTextW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..read.max(0) as usize])
}

fn unregister(thumbs: &[isize]) {
    for &thumb in thumbs {
        let _ = unsafe { DwmUnregisterThumbnail(thumb) };
    }
}

/// Shows the card `request` asks for, in place of any other; hides it when
/// none of the windows can be shown.
pub fn show(request: Request) {
    let Some(hwnd) = window() else {
        return;
    };
    let previous = PREVIEW.with_borrow_mut(|preview| preview.as_mut()?.shown.take());
    if let Some(previous) = previous {
        unregister(&previous.thumbs);
    }
    // A thumbnail for each window the DWM takes, and its size.
    let mut windows = Vec::new();
    let mut thumbs = Vec::new();
    let mut sizes = Vec::new();
    for &value in &request.windows {
        let Ok(thumb) = (unsafe { DwmRegisterThumbnail(hwnd, HWND(value as _)) }) else {
            continue;
        };
        let size = unsafe { DwmQueryThumbnailSourceSize(thumb) }.unwrap_or_default();
        windows.push(value);
        thumbs.push(thumb);
        sizes.push((f64::from(size.cx), f64::from(size.cy)));
    }
    let Some(card) = previews::card(
        request.icon,
        request.edge,
        request.screen,
        request.scale,
        &sizes,
    ) else {
        unregister(&thumbs);
        hide();
        return;
    };
    // The windows left out for want of room have no thumbnail; they stay
    // in the list, for the card shown again after a close.
    unregister(&thumbs[card.cells.len()..]);
    thumbs.truncate(card.cells.len());
    let titles = windows
        .iter()
        .take(card.cells.len())
        .map(|&value| title_of(value))
        .collect();
    let area = card.area;
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
    for (cell, &thumb) in card.cells.iter().zip(&thumbs) {
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION
                | DWM_TNP_VISIBLE
                | DWM_TNP_OPACITY
                | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: RECT {
                left: cell.thumb.x.round() as i32,
                top: cell.thumb.y.round() as i32,
                right: (cell.thumb.x + cell.thumb.width).round() as i32,
                bottom: (cell.thumb.y + cell.thumb.height).round() as i32,
            },
            opacity: 255,
            fVisible: true.into(),
            fSourceClientAreaOnly: false.into(),
            ..Default::default()
        };
        let _ = unsafe { DwmUpdateThumbnailProperties(thumb, &properties) };
    }
    PREVIEW.with_borrow_mut(|preview| {
        if let Some(preview) = preview {
            preview.painter.resize();
            preview.shown = Some(Shown {
                request: Request { windows, ..request },
                titles,
                thumbs,
                card,
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
    let hidden = PREVIEW.with_borrow_mut(|preview| {
        let preview = preview.as_mut()?;
        let shown = preview.shown.take()?;
        Some((preview.hwnd, shown))
    });
    if let Some((hwnd, shown)) = hidden {
        unregister(&shown.thumbs);
        unsafe {
            let _ = KillTimer(Some(hwnd), WATCH_TIMER);
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Whether a card shows, and for which dock.
pub fn shown_by() -> Option<isize> {
    PREVIEW.with_borrow(|preview| Some(preview.as_ref()?.shown.as_ref()?.request.owner))
}

/// Shows the card again for the same icon without the window `value` (it was
/// asked to close).
fn without(value: isize) {
    let request = PREVIEW.with_borrow_mut(|preview| {
        let shown = preview.as_mut()?.shown.as_mut()?;
        let windows = shown
            .request
            .windows
            .iter()
            .copied()
            .filter(|&other| other != value)
            .collect();
        Some(Request {
            owner: shown.request.owner,
            icon: shown.request.icon,
            screen: shown.request.screen,
            edge: shown.request.edge,
            scale: shown.request.scale,
            dark: shown.request.dark,
            windows,
        })
    });
    match request {
        Some(request) if !request.windows.is_empty() => show(request),
        _ => hide(),
    }
}

fn contains(area: Area, point: POINT) -> bool {
    let (x, y) = (f64::from(point.x), f64::from(point.y));
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

/// Every `WATCH_EVERY`: off the icon, the card and the gap between them for
/// `LEAVE_GRACE`, the card goes.
fn watch() {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let gone = PREVIEW.with_borrow_mut(|preview| {
        let shown = preview.as_mut()?.shown.as_mut()?;
        let icon = shown.request.icon;
        let card = shown.card.area;
        // The gap: as wide as the icon, from it to the card.
        let top = (icon.y + icon.height)
            .min(card.y + card.height)
            .min(icon.y.min(card.y));
        let bottom = (icon.y + icon.height).max(card.y + card.height);
        let bridge = Area {
            x: icon.x,
            y: top,
            width: icon.width,
            height: bottom - top,
        };
        if contains(card, point) || contains(icon, point) || contains(bridge, point) {
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

/// The pointer moved over the card, at `(x, y)` in its pixels.
fn hover(hwnd: HWND, x: f64, y: f64) {
    let changed = PREVIEW.with_borrow_mut(|preview| {
        let shown = preview.as_mut()?.shown.as_mut()?;
        let hovered = previews::hit(&shown.card, x, y);
        let changed = hovered != shown.hovered;
        shown.hovered = hovered;
        Some(changed)
    });
    if changed == Some(true) {
        let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
    }
}

/// A click at `(x, y)`: a cell brings its window to the front, a close
/// button asks its window to close.
fn click(x: f64, y: f64) {
    let target = PREVIEW.with_borrow(|preview| {
        let shown = preview.as_ref()?.shown.as_ref()?;
        let hit = previews::hit(&shown.card, x, y)?;
        let index = match hit {
            Hit::Cell(index) | Hit::Close(index) => index,
        };
        Some((hit, *shown.request.windows.get(index)?))
    });
    match target {
        Some((Hit::Cell(_), value)) => {
            hide();
            super::openwin::focus(value);
        }
        Some((Hit::Close(_), value)) => {
            super::openwin::close(value);
            without(value);
        }
        None => {}
    }
}

fn paint(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    unsafe { BeginPaint(hwnd, &mut paint) };
    PREVIEW.with_borrow_mut(|preview| {
        if let Some(Preview {
            painter,
            shown: Some(shown),
            ..
        }) = preview
            && !painter.paint(shown)
        {
            // Direct2D lost the device: once more with a new target.
            painter.paint(shown);
        }
    });
    let _ = unsafe { EndPaint(hwnd, &paint) };
}

fn point_of(lparam: LPARAM) -> (f64, f64) {
    let x = (lparam.0 & 0xFFFF) as u16 as i16;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16;
    (f64::from(x), f64::from(y))
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
            let (x, y) = point_of(lparam);
            hover(hwnd, x, y);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = point_of(lparam);
            click(x, y);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == WATCH_TIMER => {
            watch();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// The card's colors: the launcher's plate (spec 0012), light or dark.
struct Colors {
    background: D2D1_COLOR_F,
    text: D2D1_COLOR_F,
    dim: D2D1_COLOR_F,
    /// Under the cell the pointer is on, and under a thumbnail the DWM has
    /// nothing for (a window never drawn).
    hover: D2D1_COLOR_F,
    well: D2D1_COLOR_F,
}

const fn rgb(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

/// Windows' red for a close button under the pointer.
const CLOSE_RED: D2D1_COLOR_F = rgb(0xC4, 0x2B, 0x1C, 1.0);
const WHITE: D2D1_COLOR_F = rgb(0xFF, 0xFF, 0xFF, 1.0);

fn colors(dark: bool) -> Colors {
    if dark {
        Colors {
            background: rgb(0x24, 0x24, 0x26, 1.0),
            text: rgb(0xF5, 0xF5, 0xF7, 1.0),
            dim: rgb(0xA1, 0xA1, 0xA6, 1.0),
            hover: rgb(0xFF, 0xFF, 0xFF, 0.08),
            well: rgb(0xFF, 0xFF, 0xFF, 0.05),
        }
    } else {
        Colors {
            background: rgb(0xF6, 0xF6, 0xF8, 1.0),
            text: rgb(0x1C, 0x1C, 0x1E, 1.0),
            dim: rgb(0x6E, 0x6E, 0x73, 1.0),
            hover: rgb(0x00, 0x00, 0x00, 0.06),
            well: rgb(0x00, 0x00, 0x00, 0.04),
        }
    }
}

fn rect(area: Area) -> D2D_RECT_F {
    D2D_RECT_F {
        left: area.x as f32,
        top: area.y as f32,
        right: (area.x + area.width) as f32,
        bottom: (area.y + area.height) as f32,
    }
}

/// Text styles by size (hundredths of a pixel), weight, alignment and font.
type FormatKey = (u32, i32, i32, bool);

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
                // Software: a few rectangles and lines of text; a hardware
                // target would load a Direct3D device for them.
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
                .inspect_err(|error| log::warn!("previews: nothing to paint with: {error}"))
                .ok()?;
            unsafe { target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
            self.brush = unsafe { target.CreateSolidColorBrush(&WHITE, None) }.ok();
            self.target = Some(target);
        }
        self.target.clone()
    }

    fn format(
        &mut self,
        size: f64,
        weight: DWRITE_FONT_WEIGHT,
        align: DWRITE_TEXT_ALIGNMENT,
        glyph: bool,
    ) -> Option<IDWriteTextFormat> {
        let key = ((size * 100.0).round() as u32, weight.0, align.0, glyph);
        if let Some(found) = self.formats.get(&key) {
            return Some(found.clone());
        }
        let family = if glyph {
            w!("Segoe Fluent Icons")
        } else {
            w!("Segoe UI Variable Text")
        };
        let format = unsafe {
            self.write.CreateTextFormat(
                family,
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
            // A long title ends in "…" before the close button.
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
        glyph: bool,
        ink: D2D1_COLOR_F,
    ) {
        let Some(format) = self.format(size, weight, align, glyph) else {
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
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
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

    /// Draws the card; false when Direct2D lost the device (the target is
    /// made again at the next paint).
    fn paint(&mut self, shown: &Shown) -> bool {
        let Some(target) = self.target() else {
            return true;
        };
        let c = colors(shown.request.dark);
        let scale = shown.request.scale;
        unsafe {
            target.BeginDraw();
            target.Clear(Some(&c.background));
        }
        for (cell, title) in shown.card.cells.iter().zip(&shown.titles) {
            let over_cell =
                matches!(shown.hovered, Some(Hit::Cell(i) | Hit::Close(i)) if i == cell.index);
            if over_cell {
                self.fill(&target, grown(cell.area, 4.0 * scale), 6.0 * scale, c.hover);
            }
            // Under the thumbnail: seen only where the DWM draws nothing.
            self.fill(&target, cell.thumb, 4.0 * scale, c.well);
            let inset = 4.0 * scale;
            self.text(
                &target,
                title,
                Area {
                    x: cell.title.x + inset,
                    width: (cell.title.width - inset).max(0.0),
                    ..cell.title
                },
                12.0 * scale,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_LEADING,
                false,
                c.text,
            );
            let over_close = shown.hovered == Some(Hit::Close(cell.index));
            if over_close {
                self.fill(&target, cell.close, 4.0 * scale, CLOSE_RED);
            }
            // Segoe Fluent Icons' "ChromeClose".
            self.text(
                &target,
                "\u{E8BB}",
                cell.close,
                9.0 * scale,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                true,
                if over_close { WHITE } else { c.dim },
            );
        }
        if let Some(more) = shown.card.more_area {
            self.text(
                &target,
                &format!("+{}", shown.card.more),
                more,
                14.0 * scale,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_TEXT_ALIGNMENT_CENTER,
                false,
                c.dim,
            );
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

/// `area` grown by `by` on every side.
fn grown(area: Area, by: f64) -> Area {
    Area {
        x: area.x - by,
        y: area.y - by,
        width: area.width + 2.0 * by,
        height: area.height + 2.0 * by,
    }
}
