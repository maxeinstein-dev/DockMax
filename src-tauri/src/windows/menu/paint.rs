//! Paints a menu with Direct2D (spec 0014 R3): the card, a glyph or a
//! swatch on each item, the item under the pointer or the keyboard filled in
//! the accent, the shortcuts, the checks, the titles and the separators. In
//! logical pixels: the target's DPI is the monitor's.

use std::collections::HashMap;

use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TEXT_METRICS, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::core::{PCWSTR, w};
use windows_numerics::Vector2;

use crate::domain::menu::{self as model, Line, Rgb};
use crate::windows::launcher::layout::Colors;

/// The check of a checked item, from Windows' icon font.
const CHECK_GLYPH: char = '\u{E73E}';

/// Text sizes, in logical pixels.
pub const LABEL_SIZE: f64 = 14.0;
pub const SHORTCUT_SIZE: f64 = 12.0;
const TITLE_SIZE: f64 = 12.0;
const GLYPH_SIZE: f64 = 16.0;

/// Text styles, by size (hundredths of a pixel), weight, alignment and
/// whether it is Windows' icon font.
type FormatKey = (u32, i32, i32, bool);
type TextStyle = (f64, DWRITE_FONT_WEIGHT, DWRITE_TEXT_ALIGNMENT, bool);

/// What one paint draws.
pub struct Scene<'a> {
    pub lines: &'a [Line],
    pub hot: Option<usize>,
    pub colors: Colors,
    /// The menu's width, in logical pixels.
    pub width: f64,
}

pub struct Painter {
    factory: ID2D1Factory,
    write: IDWriteFactory,
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

fn v(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
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
    pub fn new(hwnd: HWND, scale: f64) -> windows::core::Result<Painter> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }?;
        let write: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?;
        Ok(Painter {
            factory,
            write,
            target: None,
            brush: None,
            formats: HashMap::new(),
            hwnd,
            scale,
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

    /// The render target, made the first time (and after Direct2D lost it).
    fn target(&mut self) -> Option<ID2D1HwndRenderTarget> {
        if self.target.is_none() {
            let dpi = (96.0 * self.scale) as f32;
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                // Software, as the launcher: a menu is a few lines of text.
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
                .inspect_err(|error| log::warn!("menu: nothing to paint with: {error}"))
                .ok()?;
            unsafe { target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
            self.brush = unsafe { target.CreateSolidColorBrush(&color((0, 0, 0), 1.0), None) }.ok();
            self.target = Some(target);
        }
        self.target.clone()
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
            // A label longer than the widest menu ends in "…".
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

    /// How wide `text` is at `size`, in logical pixels.
    pub fn measure(&mut self, text: &str, size: f64) -> f64 {
        let Some(format) = self.format((
            size,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_TEXT_ALIGNMENT_LEADING,
            false,
        )) else {
            return 0.0;
        };
        let wide: Vec<u16> = text.encode_utf16().collect();
        let Ok(layout) = (unsafe { self.write.CreateTextLayout(&wide, &format, 10_000.0, 100.0) })
        else {
            return 0.0;
        };
        let mut metrics = DWRITE_TEXT_METRICS::default();
        if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
            return 0.0;
        }
        f64::from(metrics.widthIncludingTrailingWhitespace)
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

    /// Draws `scene`. Returns false when Direct2D lost the device: the caller
    /// paints again.
    pub fn paint(&mut self, scene: &Scene) -> bool {
        let Some(target) = self.target() else {
            return true;
        };
        let Some(brush) = self.brush.clone() else {
            return true;
        };
        let c = scene.colors;
        let width = scene.width;
        unsafe {
            target.BeginDraw();
            target.Clear(Some(&color(c.background, 1.0)));
        }
        for ((line, top), index) in scene.lines.iter().zip(model::tops(scene.lines)).zip(0..) {
            match line {
                Line::Separator => unsafe {
                    brush.SetColor(&color(c.dim, 0.28));
                    let y = (top + model::SEPARATOR / 2.0) as f32;
                    target.DrawLine(
                        v((model::INSET + 8.0) as f32, y),
                        v((width - model::INSET - 8.0) as f32, y),
                        &brush,
                        1.0,
                        None,
                    );
                },
                Line::Title(title) => self.text(
                    &target,
                    title,
                    (
                        TITLE_SIZE,
                        DWRITE_FONT_WEIGHT_SEMI_BOLD,
                        DWRITE_TEXT_ALIGNMENT_LEADING,
                        false,
                    ),
                    rect(
                        model::INSET + 10.0,
                        top + 4.0,
                        width - 2.0 * model::INSET - 20.0,
                        model::TITLE - 4.0,
                    ),
                    color(c.dim, 1.0),
                ),
                Line::Item(item) => {
                    let hot = scene.hot == Some(index) && item.enabled;
                    let band = rect(
                        model::INSET,
                        top + 1.0,
                        width - 2.0 * model::INSET,
                        model::ITEM - 2.0,
                    );
                    if hot {
                        let shape = D2D1_ROUNDED_RECT {
                            rect: band,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        };
                        unsafe {
                            brush.SetColor(&color(c.selection, 1.0));
                            target.FillRoundedRectangle(&shape, &brush);
                            if c.contrast {
                                brush.SetColor(&color(c.selection_text, 1.0));
                                target.DrawRoundedRectangle(&shape, &brush, 2.0, None);
                            }
                        }
                    }
                    let (ink, dim) = if hot {
                        (color(c.selection_text, 1.0), color(c.selection_text, 0.8))
                    } else if item.enabled {
                        (color(c.text, 1.0), color(c.dim, 1.0))
                    } else {
                        (color(c.dim, 0.6), color(c.dim, 0.6))
                    };
                    let mark = rect(model::INSET + 8.0, top, 24.0, model::ITEM);
                    if let Some(swatch) = item.swatch {
                        let ellipse = D2D1_ELLIPSE {
                            point: v(
                                (model::INSET + 20.0) as f32,
                                (top + model::ITEM / 2.0) as f32,
                            ),
                            radiusX: 5.5,
                            radiusY: 5.5,
                        };
                        unsafe {
                            brush.SetColor(&color(swatch, 1.0));
                            target.FillEllipse(&ellipse, &brush);
                            if hot {
                                brush.SetColor(&color(c.selection_text, 0.9));
                                target.DrawEllipse(&ellipse, &brush, 1.5, None);
                            }
                        }
                    } else if let Some(glyph) = item.glyph {
                        self.text(
                            &target,
                            &glyph.to_string(),
                            (
                                GLYPH_SIZE,
                                DWRITE_FONT_WEIGHT_NORMAL,
                                DWRITE_TEXT_ALIGNMENT_CENTER,
                                true,
                            ),
                            mark,
                            dim,
                        );
                    }
                    let right = width - model::INSET - model::CHECK;
                    self.text(
                        &target,
                        &item.label,
                        (
                            LABEL_SIZE,
                            DWRITE_FONT_WEIGHT_NORMAL,
                            DWRITE_TEXT_ALIGNMENT_LEADING,
                            false,
                        ),
                        rect(
                            model::LABEL_X,
                            top,
                            (right - model::LABEL_X).max(0.0),
                            model::ITEM,
                        ),
                        ink,
                    );
                    if let Some(shortcut) = &item.shortcut {
                        self.text(
                            &target,
                            shortcut,
                            (
                                SHORTCUT_SIZE,
                                DWRITE_FONT_WEIGHT_NORMAL,
                                DWRITE_TEXT_ALIGNMENT_TRAILING,
                                false,
                            ),
                            rect(
                                model::LABEL_X,
                                top,
                                (right - model::LABEL_X).max(0.0),
                                model::ITEM,
                            ),
                            dim,
                        );
                    }
                    if item.checked {
                        let check = if hot { ink } else { color(c.selection, 1.0) };
                        self.text(
                            &target,
                            &CHECK_GLYPH.to_string(),
                            (
                                GLYPH_SIZE - 2.0,
                                DWRITE_FONT_WEIGHT_NORMAL,
                                DWRITE_TEXT_ALIGNMENT_CENTER,
                                true,
                            ),
                            rect(right, top, model::CHECK, model::ITEM),
                            check,
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
