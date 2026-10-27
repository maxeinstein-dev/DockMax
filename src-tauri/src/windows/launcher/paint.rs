//! Paints the launcher's list of results with Direct2D, in the list's own
//! child window (spec 0012 §4). Everything is drawn in logical pixels: the
//! render target's DPI is the monitor's.
//!
//! Made when the launcher opens and dropped 45 s after it closes, with the
//! icons it holds.

use std::collections::HashMap;

use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Bitmap, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::core::{PCWSTR, w};

use super::layout::{self, ROW, Rgb};

/// Most icons kept while the launcher is open: they are dropped all at once
/// past it.
pub const MAX_ICONS: usize = 64;

/// One row as it is drawn.
pub struct Row<'a> {
    pub name: &'a str,
    pub subtitle: &'a str,
    pub icon: Option<&'a ID2D1Bitmap>,
    /// The glyph of Windows' icon font drawn instead of an icon (a settings
    /// page, the calculator, the web).
    pub glyph: Option<char>,
    /// The first letter, while there is no icon nor glyph.
    pub letter: char,
    pub selected: bool,
    /// What Alt and a number opens it with (`Alt+1`), for the first nine.
    pub shortcut: Option<String>,
}

/// What the list shows: rows, or one line of text when there are none
/// ("Lendo os apps…", "Nenhum app encontrado").
pub struct Scene<'a> {
    pub colors: layout::Colors,
    /// The title above the rows ("Recentes"), if any.
    pub title: Option<&'a str>,
    pub rows: Vec<Row<'a>>,
    pub message: Option<&'a str>,
}

/// Text styles, by size (hundredths of a pixel), weight, alignment and
/// font (Windows' icon font or not).
type FormatKey = (u32, i32, i32, bool);

/// A text's size in logical pixels, its weight, its alignment and whether
/// it is a glyph of Windows' icon font.
type TextStyle = (f64, DWRITE_FONT_WEIGHT, DWRITE_TEXT_ALIGNMENT, bool);

pub struct Painter {
    factory: ID2D1Factory,
    write: IDWriteFactory,
    wic: IWICImagingFactory,
    target: Option<ID2D1HwndRenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
    formats: HashMap<FormatKey, IDWriteTextFormat>,
    hwnd: HWND,
    scale: f64,
}

fn color(c: Rgb, alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: f32::from(c.0) / 255.0,
        g: f32::from(c.1) / 255.0,
        b: f32::from(c.2) / 255.0,
        a: alpha,
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> D2D_RECT_F {
    D2D_RECT_F {
        left: x as f32,
        top: y as f32,
        right: (x + width) as f32,
        bottom: (y + height) as f32,
    }
}

impl Painter {
    pub fn new(hwnd: HWND) -> windows::core::Result<Painter> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }?;
        let write: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?;
        let wic: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }?;
        Ok(Painter {
            factory,
            write,
            wic,
            target: None,
            brush: None,
            formats: HashMap::new(),
            hwnd,
            scale: 1.0,
        })
    }

    /// Draws at `scale` × 96 DPI from now on.
    pub fn set_scale(&mut self, scale: f64) {
        if (self.scale - scale).abs() > f64::EPSILON {
            self.scale = scale;
            self.formats.clear();
            if let Some(target) = &self.target {
                let dpi = (96.0 * scale) as f32;
                unsafe { target.SetDpi(dpi, dpi) };
            }
        }
    }

    /// The window changed size: the target follows.
    pub fn resize(&mut self) {
        if let Some(target) = &self.target {
            let size = self.client_size();
            if unsafe { target.Resize(&size) }.is_err() {
                self.drop_target();
            }
        }
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

    /// The render target, made the first time (and after Direct2D lost it).
    fn target(&mut self) -> Option<ID2D1HwndRenderTarget> {
        if self.target.is_none() {
            let dpi = (96.0 * self.scale) as f32;
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                // Software: a hardware target would load a Direct3D device
                // (tens of megabytes) for a list of eight rows.
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: dpi,
                dpiY: dpi,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let window = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: self.client_size(),
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            let target = unsafe { self.factory.CreateHwndRenderTarget(&properties, &window) }
                .inspect_err(|error| log::warn!("launcher: nothing to paint with: {error}"))
                .ok()?;
            unsafe { target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
            self.brush = unsafe { target.CreateSolidColorBrush(&color((0, 0, 0), 1.0), None) }.ok();
            self.target = Some(target);
        }
        self.target.clone()
    }

    /// Direct2D lost the device: the target, and the bitmaps made with it,
    /// are made again.
    fn drop_target(&mut self) {
        self.target = None;
        self.brush = None;
    }

    fn format(&mut self, (size, weight, align, glyph): TextStyle) -> Option<IDWriteTextFormat> {
        let key = ((size * 100.0).round() as u32, weight.0, align.0, glyph);
        if let Some(found) = self.formats.get(&key) {
            return Some(found.clone());
        }
        let family: PCWSTR = if glyph {
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
            // A long name ends in "…" instead of running under the shortcut.
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

    /// A PNG as a bitmap this painter draws; `None` when it can't be read,
    /// or when there is no target yet.
    pub fn bitmap(&mut self, png: &[u8]) -> Option<ID2D1Bitmap> {
        let target = self.target()?;
        unsafe {
            let stream = self.wic.CreateStream().ok()?;
            stream.InitializeFromMemory(png).ok()?;
            let decoder = self
                .wic
                .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
                .ok()?;
            let frame = decoder.GetFrame(0).ok()?;
            let converter = self.wic.CreateFormatConverter().ok()?;
            converter
                .Initialize(
                    &frame,
                    &GUID_WICPixelFormat32bppPBGRA,
                    WICBitmapDitherTypeNone,
                    None,
                    0.0,
                    WICBitmapPaletteTypeCustom,
                )
                .ok()?;
            target.CreateBitmapFromWicBitmap(&converter, None).ok()
        }
    }

    fn text(
        &mut self,
        target: &ID2D1HwndRenderTarget,
        text: &str,
        style: TextStyle,
        area: D2D_RECT_F,
        ink: D2D1_COLOR_F,
    ) {
        let Some(format) = self.format(style) else {
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
                &area,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    fn fill(
        &self,
        target: &ID2D1HwndRenderTarget,
        area: D2D_RECT_F,
        radius: f32,
        ink: D2D1_COLOR_F,
    ) {
        let Some(brush) = &self.brush else {
            return;
        };
        unsafe {
            brush.SetColor(&ink);
            if radius > 0.0 {
                target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: area,
                        radiusX: radius,
                        radiusY: radius,
                    },
                    brush,
                );
            } else {
                target.FillRectangle(&area, brush);
            }
        }
    }

    fn outline(
        &self,
        target: &ID2D1HwndRenderTarget,
        area: D2D_RECT_F,
        radius: f32,
        ink: D2D1_COLOR_F,
    ) {
        let Some(brush) = &self.brush else {
            return;
        };
        unsafe {
            brush.SetColor(&ink);
            target.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: area,
                    radiusX: radius,
                    radiusY: radius,
                },
                brush,
                2.0,
                None,
            );
        }
    }

    /// Draws `scene`, `width` logical pixels wide. Returns false when
    /// Direct2D lost the device: the caller drops the bitmaps it holds and
    /// paints again.
    pub fn paint(&mut self, scene: &Scene, width: f64) -> bool {
        let Some(target) = self.target() else {
            return true;
        };
        let c = scene.colors;
        unsafe {
            target.BeginDraw();
            target.Clear(Some(&color(c.background, 1.0)));
        }
        let mut y = layout::LIST_PAD;
        if let Some(title) = scene.title {
            self.text(
                &target,
                title,
                (
                    12.0,
                    DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    DWRITE_TEXT_ALIGNMENT_LEADING,
                    false,
                ),
                rect(
                    layout::SIDE + 4.0,
                    y,
                    width - 2.0 * layout::SIDE,
                    layout::TITLE,
                ),
                color(c.dim, 1.0),
            );
            y += layout::TITLE;
        }
        if let Some(message) = scene.message {
            self.text(
                &target,
                message,
                (
                    14.0,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_TEXT_ALIGNMENT_LEADING,
                    false,
                ),
                rect(layout::SIDE + 4.0, y, width - 2.0 * layout::SIDE, ROW),
                color(c.dim, 1.0),
            );
        }
        for row in &scene.rows {
            let band = rect(layout::SIDE, y, width - 2.0 * layout::SIDE, ROW);
            let (ink, dim) = if row.selected {
                self.fill(&target, band, 8.0, color(c.selection, 1.0));
                if c.contrast {
                    self.outline(&target, band, 8.0, color(c.selection_text, 1.0));
                }
                (color(c.selection_text, 1.0), color(c.selection_text, 0.78))
            } else {
                (color(c.text, 1.0), color(c.dim, 1.0))
            };
            let icon = layout::ICON;
            let icon_x = layout::SIDE + 10.0;
            let icon_y = y + (ROW - icon) / 2.0;
            match row.icon {
                Some(bitmap) => unsafe {
                    target.DrawBitmap(
                        bitmap,
                        Some(&rect(icon_x, icon_y, icon, icon)),
                        1.0,
                        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                        None,
                    );
                },
                None => {
                    self.fill(
                        &target,
                        rect(icon_x, icon_y, icon, icon),
                        8.0,
                        color(c.dim, 0.25),
                    );
                    let (letter, size, glyph) = match row.glyph {
                        Some(glyph) => (glyph.to_string(), 18.0, true),
                        None => (row.letter.to_string(), 15.0, false),
                    };
                    self.text(
                        &target,
                        &letter,
                        (
                            size,
                            DWRITE_FONT_WEIGHT_SEMI_BOLD,
                            windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_CENTER,
                            glyph,
                        ),
                        rect(icon_x, icon_y, icon, icon),
                        ink,
                    );
                }
            }
            let text_x = icon_x + icon + 12.0;
            let shortcut_width = if row.shortcut.is_some() { 56.0 } else { 0.0 };
            let text_width = (width - layout::SIDE - 12.0 - shortcut_width - text_x).max(0.0);
            self.text(
                &target,
                row.name,
                (
                    15.0,
                    DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    DWRITE_TEXT_ALIGNMENT_LEADING,
                    false,
                ),
                rect(text_x, y + 5.0, text_width, 22.0),
                ink,
            );
            self.text(
                &target,
                row.subtitle,
                (
                    12.0,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_TEXT_ALIGNMENT_LEADING,
                    false,
                ),
                rect(text_x, y + 25.0, text_width, 18.0),
                dim,
            );
            if let Some(shortcut) = &row.shortcut {
                self.text(
                    &target,
                    shortcut,
                    (
                        12.0,
                        DWRITE_FONT_WEIGHT_NORMAL,
                        DWRITE_TEXT_ALIGNMENT_TRAILING,
                        false,
                    ),
                    rect(
                        width - layout::SIDE - 12.0 - shortcut_width,
                        y,
                        shortcut_width,
                        ROW,
                    ),
                    dim,
                );
            }
            y += ROW;
        }
        match unsafe { target.EndDraw(None, None) } {
            Err(error) if error.code() == D2DERR_RECREATE_TARGET => {
                self.drop_target();
                false
            }
            _ => true,
        }
    }
}
