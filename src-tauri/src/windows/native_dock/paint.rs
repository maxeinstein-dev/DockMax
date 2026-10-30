//! Paints the dock with Direct2D into a bitmap the layered window shows
//! (`UpdateLayeredWindow`, spec 0011 H3): what is transparent there lets the
//! mouse through to the windows behind, so the dock takes the mouse only
//! where it paints.
//!
//! Everything is drawn in logical pixels: the render target's DPI is the
//! monitor's.

use std::collections::HashMap;

use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F,
    D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED,
    D2D1_FIGURE_END_OPEN, D2D1_GRADIENT_STOP, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_LARGE, D2D1_ARC_SIZE_SMALL,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
    D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_SOLID, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE,
    D2D1_EXTEND_MODE_CLAMP, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_GAMMA_2_2, D2D1_LAYER_OPTIONS_NONE, D2D1_LAYER_PARAMETERS, D2D1_LINE_JOIN_ROUND,
    D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT, D2D1_STROKE_STYLE_PROPERTIES, D2D1_SWEEP_DIRECTION_CLOCKWISE,
    D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory,
    ID2D1Bitmap, ID2D1Brush, ID2D1DCRenderTarget, ID2D1Factory, ID2D1Geometry,
    ID2D1LinearGradientBrush, ID2D1PathGeometry, ID2D1RadialGradientBrush, ID2D1SolidColorBrush,
    ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
    HGDIOBJ, SelectObject,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Interface, PCWSTR, w};
use windows_numerics::{Matrix3x2, Vector2};

use super::look::Look;
use super::svg::{self, Segment};
use super::theme::{APP_INK, Color, Glaze, Palette};
use crate::domain::badges::Badge;
use crate::domain::dock_layout::{self, Area};
use crate::domain::settings::{DockEdge, IconShape, Indicator};
use crate::domain::system_activity::HISTORY;
use crate::domain::widgets::system::ActivityStyle;

/// The superellipse every tile is cut to, in a 100 × 100 box.
const SQUIRCLE: &str = "M50 0C90 0 100 10 100 50S90 100 50 100 0 90 0 50 10 0 50 0Z";

/// The glyphs drawn for Dockyard itself (`src/dock/Glyph.tsx`, the note's
/// tile): a 24 px view box, a 1.75 stroke with round ends.
pub const SETTINGS_GLYPH: &[&str] = &[
    "M4 7.5h9.5M17.5 7.5H20M4 16.5h3.5M11.5 16.5H20",
    "M13.5 7.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
    "M7.5 16.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0",
];
pub const NOTE_GLYPH: &[&str] = &[
    "M7.5 3.5h9a2.5 2.5 0 0 1 2.5 2.5v12a2.5 2.5 0 0 1-2.5 2.5h-9a2.5 2.5 0 0 1-2.5-2.5v-12a2.5 2.5 0 0 1 2.5-2.5z",
    "M8.5 8.5h7M8.5 12h7M8.5 15.5h4",
];

/// A note, the Tocando agora without a cover (24 px box).
const NOTE: &[&str] = &[
    "M9 17.5V5.5l10 -2v12M9 17.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0M19 15.5a2.5 2.5 0 1 1 -5 0a2.5 2.5 0 1 1 5 0",
];
/// Two bars: paused.
const PAUSE: &[&str] = &["M9 6v12M15 6v12"];
/// The Tocando agora's buttons, three icons wide.
const PLAY: &[&str] = &["M8 5.5l11 6.5 -11 6.5z"];
const PREVIOUS: &[&str] = &["M6 6v12M19 6l-9 6 9 6z"];
const NEXT: &[&str] = &["M18 6v12M5 6l9 6 -9 6z"];

/// The Clima's skies (24 px box, after Lucide's).
pub const SUN: &[&str] = &[
    "M12 8a4 4 0 1 1 0 8a4 4 0 1 1 0 -8",
    "M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4 -1.4M17.3 6.7l1.4 -1.4",
];
const MOON: &[&str] = &["M12 3a6 6 0 0 0 9 9a9 9 0 1 1 -9 -9z"];
const PARTLY: &[&str] = &[
    "M12 2v2M4.93 4.93l1.41 1.41M20 12h2M19.07 4.93l-1.41 1.41",
    "M15.947 12.65a4 4 0 0 0 -5.925 -4.128",
    "M13 22H7a5 5 0 1 1 4.9 -6H13a3 3 0 0 1 0 6z",
];
const CLOUD: &[&str] = &["M17.5 19H9a7 7 0 1 1 6.71 -9h1.79a4.5 4.5 0 1 1 0 9z"];
/// A cloud's upper part, with room under it.
const CLOUD_TOP: &str = "M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242";
const FOG: &[&str] = &[CLOUD_TOP, "M16 17H7M17 21H9"];
const DRIZZLE: &[&str] = &[CLOUD_TOP, "M8 19v1M8 14v1M16 19v1M16 14v1M12 21v1M12 16v1"];
const RAIN: &[&str] = &[CLOUD_TOP, "M16 14v6M8 14v6M12 16v6"];
const SNOW: &[&str] = &[
    CLOUD_TOP,
    "M8 15h0.01M8 19h0.01M12 17h0.01M12 21h0.01M16 15h0.01M16 19h0.01",
];
const STORM: &[&str] = &[
    "M6 16.326A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 0.5 8.973",
    "M13 12l-3 5h4l-3 5",
];

/// The sky of `condition`; a clear night has the moon.
pub fn sky(condition: crate::domain::weather::Condition, day: bool) -> &'static [&'static str] {
    use crate::domain::weather::Condition;
    match condition {
        Condition::Clear if day => SUN,
        Condition::Clear => MOON,
        Condition::PartlyCloudy => PARTLY,
        Condition::Cloudy => CLOUD,
        Condition::Fog => FOG,
        Condition::Drizzle => DRIZZLE,
        Condition::Rain => RAIN,
        Condition::Snow => SNOW,
        Condition::Storm => STORM,
    }
}

/// What a tile shows inside its glaze.
pub enum Face {
    /// A line glyph on a glaze.
    Glyph {
        glaze: Glaze,
        paths: &'static [&'static str],
    },
    /// A glyph of Windows' icon font (`strip::ICON_FONT`) on a glaze: the
    /// Lançador's search, as the menus show it.
    Symbol { glaze: Glaze, glyph: char },
    /// A program's icon; its first letter while there is none.
    App {
        icon: Option<ID2D1Bitmap>,
        letter: String,
        shape: IconShape,
        badge: Option<ID2D1Bitmap>,
    },
    /// The digital clock: the time, white on the glaze.
    Time { glaze: Glaze, text: String },
    /// The analog clock: hands at the time.
    Dial {
        glaze: Glaze,
        hours: f32,
        minutes: f32,
    },
    /// The time progress: the ring and the percentage (none on the small
    /// size).
    Ring {
        glaze: Glaze,
        fraction: Option<f32>,
        text: Option<String>,
    },
    /// The Atividade (and the time progress when wide): one to three
    /// meters as bars, rings or figures; with `graph`, the processor's last
    /// minute (0–1) beside them.
    Meters {
        glaze: Glaze,
        style: ActivityStyle,
        meters: Vec<Meter>,
        graph: Option<Vec<f32>>,
    },
    /// The Calendário (spec 0017 F2a): lines of text, the first one the
    /// strongest, left-aligned on a wide tile and centered on a square one.
    Agenda {
        glaze: Glaze,
        lines: Vec<AgendaLine>,
    },
    /// The Tocando agora (spec 0017 F4): the cover (a note without one),
    /// dimmed while paused; on a wide tile, the title and the artist beside
    /// it.
    Media {
        glaze: Glaze,
        art: Option<ID2D1Bitmap>,
        playing: bool,
        title: Option<String>,
        artist: Option<String>,
        /// Nothing plays: a note in the middle of the tile, no text.
        idle: bool,
        /// The program that plays, a small mark on the cover.
        badge: Option<ID2D1Bitmap>,
        /// Three icons wide: previous, play or pause and next under the
        /// title (`dock_layout::media_buttons`).
        controls: bool,
        /// Seconds since the track showed: where a long title rolls to.
        since: f64,
    },
    /// The Clima (spec 0017 F5): the sky and, once read, the temperature
    /// under it; on a wide tile, beside it with the day's highest and
    /// lowest.
    Weather {
        glaze: Glaze,
        sky: &'static [&'static str],
        temperature: Option<String>,
        range: Option<String>,
    },
    /// The Atividade de rede: the rates and, on a wide tile, the graph.
    Rates {
        glaze: Glaze,
        down: String,
        up: String,
        graph: Option<Graph>,
    },
}

/// One value of the Atividade: the processor's or the memory's.
#[derive(Debug, Clone, PartialEq)]
pub struct Meter {
    /// Its name, and the short one a narrow tile has room for.
    pub label: &'static str,
    pub short: &'static str,
    /// 0–1; `None` while there is no reading.
    pub fraction: Option<f32>,
    /// The figure, as `42%`.
    pub text: String,
}

/// A line of the Calendário's tile.
#[derive(Debug, Clone, PartialEq)]
pub struct AgendaLine {
    pub text: String,
    /// The text's size, as a share of the tile's height.
    pub size: f64,
    /// White and bold; the others are fainter.
    pub strong: bool,
}

/// The network's last minute, each value 0–1 of the graph's height.
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub down: Vec<f32>,
    pub up: Vec<f32>,
}

/// One item as it is drawn now.
pub struct Tile {
    /// The tile, grown and moved as it is this frame.
    pub area: Area,
    pub face: Face,
    /// Held down by the mouse.
    pub pressed: bool,
    /// Configurações while the storage needs attention.
    pub attention: bool,
    /// Being dragged: drawn above the others.
    pub dragging: bool,
    /// Risen above the bar (a Calendário under the pointer): drawn above
    /// the others, with no reflection.
    pub raised: bool,
    /// The mark under it: its centre and opacity (0 for none).
    pub dot: Option<(f64, f32)>,
    /// The item's box (for the focus ring).
    pub item: Area,
    /// The program's notification badge (spec 0016).
    pub notice: Option<Badge>,
}

/// The clock and the date at the end of the bar.
pub struct Status {
    pub area: Area,
    pub time: String,
    pub date: String,
    pub hovered: bool,
}

/// A small icon of the strip at the end: a glyph of Windows' icon font
/// (with how strongly it is drawn) or a tray icon's own image.
pub struct StripCell {
    pub area: Area,
    pub glyph: Option<(char, f32)>,
    pub image: Option<ID2D1Bitmap>,
    pub hovered: bool,
}

/// The label pill: anchored at the middle of its edge nearest the tile.
pub struct Tip {
    pub anchor: (f64, f64),
    pub text: String,
}

/// Everything one frame shows.
/// A workspace dot of the switcher (spec 0013): its centre, its color,
/// whether it is the workspace shown and whether the pointer is on it.
pub struct Dot {
    pub center: (f64, f64),
    pub color: Color,
    pub active: bool,
    pub hovered: bool,
}

pub struct Scene {
    pub palette: Palette,
    pub look: Look,
    pub edge: DockEdge,
    pub pad: f64,
    pub icon: f64,
    pub bar: Area,
    pub separators: Vec<f64>,
    pub dots: Vec<Dot>,
    pub tiles: Vec<Tile>,
    pub status: Option<Status>,
    pub strip: Vec<StripCell>,
    pub tip: Option<Tip>,
    pub indicator: Indicator,
    pub indicator_color: Color,
}

fn v(x: f64, y: f64) -> Vector2 {
    Vector2 {
        X: x as f32,
        Y: y as f32,
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

fn color(c: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Scales by `s` around the origin, then moves by `(x, y)`.
fn place(s: f64, x: f64, y: f64) -> Matrix3x2 {
    Matrix3x2 {
        M11: s as f32,
        M12: 0.0,
        M21: 0.0,
        M22: s as f32,
        M31: x as f32,
        M32: y as f32,
    }
}

const IDENTITY: Matrix3x2 = Matrix3x2 {
    M11: 1.0,
    M12: 0.0,
    M21: 0.0,
    M22: 1.0,
    M31: 0.0,
    M32: 0.0,
};

/// `a`, then `b` (Direct2D maps row vectors: `p × a × b`).
fn then(a: &Matrix3x2, b: &Matrix3x2) -> Matrix3x2 {
    Matrix3x2 {
        M11: a.M11 * b.M11 + a.M12 * b.M21,
        M12: a.M11 * b.M12 + a.M12 * b.M22,
        M21: a.M21 * b.M11 + a.M22 * b.M21,
        M22: a.M21 * b.M12 + a.M22 * b.M22,
        M31: a.M31 * b.M11 + a.M32 * b.M21 + b.M31,
        M32: a.M31 * b.M12 + a.M32 * b.M22 + b.M32,
    }
}

/// The mirror about the horizontal line at `y`.
fn mirror(y: f64) -> Matrix3x2 {
    Matrix3x2 {
        M11: 1.0,
        M12: 0.0,
        M21: 0.0,
        M22: -1.0,
        M31: 0.0,
        M32: (2.0 * y) as f32,
    }
}

/// A text style: its size in logical pixels, its weight and whether it is
/// the display face (the tiles) or the text face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TextKey {
    size: u32,
    weight: i32,
    display: bool,
}

/// The bitmap the dock is drawn in, as large as the window.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: i32,
    height: i32,
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

impl Surface {
    fn new(width: i32, height: i32) -> windows::core::Result<Surface> {
        let dc = unsafe { CreateCompatibleDC(None) };
        let header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match unsafe {
            CreateDIBSection(Some(dc), &header, DIB_RGB_COLORS, &mut bits, None, 0)
        } {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = unsafe { DeleteDC(dc) };
                return Err(error);
            }
        };
        let previous = unsafe { SelectObject(dc, bitmap.into()) };
        Ok(Surface {
            dc,
            bitmap,
            previous,
            width,
            height,
        })
    }
}

pub struct Painter {
    factory: ID2D1Factory,
    write: IDWriteFactory,
    wic: IWICImagingFactory,
    target: ID2D1DCRenderTarget,
    surface: Option<Surface>,
    brush: ID2D1SolidColorBrush,
    round: ID2D1StrokeStyle,
    squircle: ID2D1PathGeometry,
    geometries: HashMap<&'static str, ID2D1PathGeometry>,
    /// The bitmap of each skin drawn (spec 0020 §3), made once.
    skins: HashMap<&'static str, ID2D1Bitmap>,
    formats: HashMap<TextKey, IDWriteTextFormat>,
    /// Windows' icon font, by size (hundredths of a pixel).
    icon_formats: HashMap<u32, IDWriteTextFormat>,
    glazes: HashMap<(u32, u32, u32, bool), ID2D1LinearGradientBrush>,
    sheen: ID2D1RadialGradientBrush,
    rim: ID2D1LinearGradientBrush,
    /// The DPI the target draws at: 96 × the monitor's scale.
    dpi: f32,
    /// Applied after every transform: the mirror while a reflection is
    /// drawn, nothing otherwise.
    base: Matrix3x2,
    /// Seconds until a long text drawn in the last frame moves again: the
    /// dock paints again then. `None` when nothing rolls.
    pub marquee: Option<f64>,
}

impl Painter {
    pub fn new() -> windows::core::Result<Painter> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }?;
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let target = unsafe { factory.CreateDCRenderTarget(&properties) }?;
        unsafe {
            target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            // ClearType needs an opaque background: the dock has none.
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        }
        let write: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?;
        let wic: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }?;
        let brush = unsafe { target.CreateSolidColorBrush(&color(Color::WHITE), None) }?;
        let round = unsafe {
            factory.CreateStrokeStyle(
                &D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    dashCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    miterLimit: 10.0,
                    dashStyle: D2D1_DASH_STYLE_SOLID,
                    dashOffset: 0.0,
                },
                None,
            )
        }?;
        let squircle = path(&factory, SQUIRCLE)?;
        // The white sheen at the top of every glaze: radial-gradient(130% 70%
        // at 50% -12%, white 0.26, transparent 58%), in the 100 × 100 box.
        let sheen = unsafe {
            let stops = target.CreateGradientStopCollection(
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(Color::WHITE.with_alpha(0.26)),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 0.58,
                        color: color(Color::WHITE.with_alpha(0.0)),
                    },
                ],
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            target.CreateRadialGradientBrush(
                &D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                    center: v(50.0, -12.0),
                    gradientOriginOffset: v(0.0, 0.0),
                    radiusX: 130.0,
                    radiusY: 70.0,
                },
                None,
                &stops,
            )?
        };
        // The inner edges: light along the top, dark along the bottom.
        let rim = unsafe {
            let stops = target.CreateGradientStopCollection(
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(Color::WHITE.with_alpha(0.32)),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 0.05,
                        color: color(Color::WHITE.with_alpha(0.0)),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 0.95,
                        color: color(Color::BLACK.with_alpha(0.0)),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 1.0,
                        color: color(Color::BLACK.with_alpha(0.16)),
                    },
                ],
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            target.CreateLinearGradientBrush(
                &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: v(0.0, 0.0),
                    endPoint: v(0.0, 100.0),
                },
                None,
                &stops,
            )?
        };
        Ok(Painter {
            factory,
            write,
            wic,
            target,
            surface: None,
            brush,
            round,
            squircle,
            geometries: HashMap::new(),
            formats: HashMap::new(),
            icon_formats: HashMap::new(),
            glazes: HashMap::new(),
            skins: HashMap::new(),
            sheen,
            rim,
            dpi: 96.0,
            base: IDENTITY,
            marquee: None,
        })
    }

    /// Draws at `scale` × 96 DPI.
    pub fn set_scale(&mut self, scale: f64) {
        self.dpi = (96.0 * scale) as f32;
        unsafe { self.target.SetDpi(self.dpi, self.dpi) };
    }

    fn format(&mut self, size: f64, weight: i32, display: bool) -> Option<IDWriteTextFormat> {
        let key = TextKey {
            size: (size * 100.0).round() as u32,
            weight,
            display,
        };
        if let Some(format) = self.formats.get(&key) {
            return Some(format.clone());
        }
        let family: PCWSTR = if display {
            w!("Segoe UI Variable Display")
        } else {
            w!("Segoe UI Variable Text")
        };
        let format = unsafe {
            self.write.CreateTextFormat(
                family,
                None,
                DWRITE_FONT_WEIGHT(weight),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size as f32,
                w!("pt-BR"),
            )
        }
        .ok()?;
        unsafe {
            let _ = format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
        }
        self.formats.insert(key, format.clone());
        Some(format)
    }

    /// The width of `text` in the label pill's style, in logical pixels.
    pub fn label_width(&mut self, text: &str) -> f64 {
        self.text_width(text, 12.5, 600, false) + 20.0
    }

    fn text_width(&mut self, text: &str, size: f64, weight: i32, display: bool) -> f64 {
        let Some(format) = self.format(size, weight, display) else {
            return 0.0;
        };
        let wide: Vec<u16> = text.encode_utf16().collect();
        let Ok(layout) = (unsafe {
            self.write
                .CreateTextLayout(&wide, &format, 10_000.0, 1_000.0)
        }) else {
            return 0.0;
        };
        let mut metrics = DWRITE_TEXT_METRICS::default();
        if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
            return 0.0;
        }
        f64::from(metrics.widthIncludingTrailingWhitespace)
    }

    /// A PNG as a bitmap this painter can draw; `None` when it can't be read.
    pub fn bitmap(&self, png: &[u8]) -> Option<ID2D1Bitmap> {
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
            self.target.CreateBitmapFromWicBitmap(&converter, None).ok()
        }
    }

    fn glyph(&mut self, text: &'static str) -> Option<ID2D1PathGeometry> {
        if let Some(found) = self.geometries.get(text) {
            return Some(found.clone());
        }
        let made = path(&self.factory, text).ok()?;
        self.geometries.insert(text, made.clone());
        Some(made)
    }

    fn glaze_brush(&mut self, glaze: Glaze, pressed: bool) -> Option<ID2D1LinearGradientBrush> {
        let key = (
            glaze.h.to_bits(),
            glaze.s.to_bits(),
            glaze.l.to_bits(),
            pressed,
        );
        if let Some(found) = self.glazes.get(&key) {
            return Some(found.clone());
        }
        let (top, bottom) = glaze.tones();
        let dim = |c: Color| {
            if pressed {
                Color {
                    r: c.r * 0.84,
                    g: c.g * 0.84,
                    b: c.b * 0.84,
                    a: c.a,
                }
            } else {
                c
            }
        };
        let brush = unsafe {
            let stops = self
                .target
                .CreateGradientStopCollection(
                    &[
                        D2D1_GRADIENT_STOP {
                            position: 0.0,
                            color: color(dim(top)),
                        },
                        D2D1_GRADIENT_STOP {
                            position: 1.0,
                            color: color(dim(bottom)),
                        },
                    ],
                    D2D1_GAMMA_2_2,
                    D2D1_EXTEND_MODE_CLAMP,
                )
                .ok()?;
            self.target
                .CreateLinearGradientBrush(
                    &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                        startPoint: v(0.0, 0.0),
                        endPoint: v(0.0, 100.0),
                    },
                    None,
                    &stops,
                )
                .ok()?
        };
        self.glazes.insert(key, brush.clone());
        Some(brush)
    }

    /// Sets `matrix`, then the base.
    fn transform(&self, matrix: &Matrix3x2) {
        unsafe { self.target.SetTransform(&then(matrix, &self.base)) };
    }

    fn solid(&self, c: Color) -> &ID2D1SolidColorBrush {
        unsafe { self.brush.SetColor(&color(c)) };
        &self.brush
    }

    /// Draws `scene` in a window of `width` × `height` physical pixels at
    /// `origin` on the screen, and hands it to the layered window.
    pub fn present(
        &mut self,
        hwnd: HWND,
        origin: POINT,
        width: i32,
        height: i32,
        scene: &Scene,
    ) -> windows::core::Result<()> {
        self.marquee = None;
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let fits = self
            .surface
            .as_ref()
            .is_some_and(|surface| surface.width == width && surface.height == height);
        if !fits {
            self.surface = None;
            self.surface = Some(Surface::new(width, height)?);
        }
        let dc = self
            .surface
            .as_ref()
            .map(|surface| surface.dc)
            .unwrap_or_default();
        let bounds = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        unsafe {
            self.target.BindDC(dc, &bounds)?;
            self.target.BeginDraw();
            self.target.Clear(Some(&color(Color::CLEAR)));
            self.transform(&IDENTITY);
        }
        self.draw(scene);
        unsafe { self.target.EndDraw(None, None) }?;
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        unsafe {
            UpdateLayeredWindow(
                hwnd,
                None,
                Some(&origin),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                Some(dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        }
    }

    fn draw(&mut self, scene: &Scene) {
        self.bar(scene);
        let separator_top = scene.bar.y + scene.pad + 4.0;
        let separator_bottom = scene.bar.y + scene.pad + scene.icon - 4.0;
        for &x in &scene.separators {
            let brush = self
                .solid(scene.look.separator.unwrap_or(scene.palette.hair))
                .clone();
            unsafe {
                self.target.FillRectangle(
                    &rect(Area {
                        x,
                        y: separator_top,
                        width: 1.0,
                        height: separator_bottom - separator_top,
                    }),
                    &brush,
                )
            };
        }
        for dot in &scene.dots {
            self.dot(dot);
        }
        if scene.look.reflections {
            for tile in scene
                .tiles
                .iter()
                .filter(|tile| !tile.dragging && !tile.raised)
            {
                self.reflection(scene, tile);
            }
        }
        // The risen and the dragged tiles last, above the others.
        let order = scene
            .tiles
            .iter()
            .filter(|tile| !tile.dragging && !tile.raised)
            .chain(
                scene
                    .tiles
                    .iter()
                    .filter(|tile| tile.raised && !tile.dragging),
            )
            .chain(scene.tiles.iter().filter(|tile| tile.dragging));
        for tile in order {
            self.tile(scene, tile);
        }
        for tile in &scene.tiles {
            self.marks(scene, tile);
        }
        for cell in &scene.strip {
            self.strip_cell(scene, cell);
        }
        if let Some(status) = &scene.status {
            self.status(scene, status);
        }
        if let Some(tip) = &scene.tip {
            self.tip(scene, tip);
        }
    }

    /// A workspace dot: full for the one shown, a ring for the others; a
    /// little bigger under the pointer.
    fn dot(&mut self, dot: &Dot) {
        use crate::domain::dock_layout::{ACTIVE_DOT_RADIUS, DOT_RADIUS};
        let grow = if dot.hovered { 1.0 } else { 0.0 };
        let brush = self.solid(dot.color).clone();
        let center = v(dot.center.0, dot.center.1);
        unsafe {
            if dot.active {
                let radius = (ACTIVE_DOT_RADIUS + grow) as f32;
                self.target.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: radius,
                        radiusY: radius,
                    },
                    &brush,
                );
            } else {
                let radius = (DOT_RADIUS + grow) as f32;
                self.target.DrawEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: radius - 0.75,
                        radiusY: radius - 0.75,
                    },
                    &brush,
                    1.5,
                    None,
                );
            }
        }
    }

    /// A vertical (or, `across`, horizontal) gradient from `from` to `to`.
    fn gradient(
        &self,
        stops: &[(f32, Color)],
        from: Vector2,
        to: Vector2,
    ) -> Option<ID2D1LinearGradientBrush> {
        let stops: Vec<D2D1_GRADIENT_STOP> = stops
            .iter()
            .map(|&(position, c)| D2D1_GRADIENT_STOP {
                position,
                color: color(c),
            })
            .collect();
        unsafe {
            let collection = self
                .target
                .CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)
                .ok()?;
            self.target
                .CreateLinearGradientBrush(
                    &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                        startPoint: from,
                        endPoint: to,
                    },
                    None,
                    &collection,
                )
                .ok()
        }
    }

    fn bar(&mut self, scene: &Scene) {
        let bar = scene.bar;
        if bar.width <= 0.0 {
            return;
        }
        let look = &scene.look;
        if look.skin {
            self.skin_bar(scene);
            return;
        }
        let radius = (look.corners * bar.height / 2.0) as f32;
        let rounded = |grow: f64| D2D1_ROUNDED_RECT {
            rect: rect(Area {
                x: bar.x + 0.5 - grow,
                y: bar.y + 0.5 - grow,
                width: bar.width - 1.0 + 2.0 * grow,
                height: bar.height - 1.0 + 2.0 * grow,
            }),
            radiusX: radius + grow as f32,
            radiusY: radius + grow as f32,
        };
        let shape = rounded(0.0);
        // The glow: rings fading outward, never under the bar.
        for glow in &look.glows {
            for step in 0..4 {
                let grow = f64::from(glow.reach) * (f64::from(step) + 0.5) / 4.0;
                let alpha = glow.color.a * (1.0 - step as f32 / 4.0) * 0.45;
                let ring = self.solid(glow.color.with_alpha(alpha)).clone();
                unsafe {
                    self.target.DrawRoundedRectangle(
                        &rounded(grow),
                        &ring,
                        (f64::from(glow.reach) / 4.0) as f32,
                        None::<&ID2D1StrokeStyle>,
                    )
                };
            }
        }
        let (top, bottom) = (v(0.0, bar.y), v(0.0, bar.y + bar.height));
        let fill = look.fill(&scene.palette);
        let clear = fill.is_clear();
        match fill {
            super::look::Fill::Solid(c) => {
                // A see-through bar keeps a breath of alpha: the mouse stays
                // on it between the tiles.
                let c = if clear { c.with_alpha(0.004) } else { c };
                let fill = self.solid(c).clone();
                unsafe { self.target.FillRoundedRectangle(&shape, &fill) };
            }
            super::look::Fill::Gradient { base, stops, over } => {
                if let Some(base) = base {
                    let fill = self.solid(base).clone();
                    unsafe { self.target.FillRoundedRectangle(&shape, &fill) };
                }
                for layer in [&stops, &over] {
                    if layer.is_empty() {
                        continue;
                    }
                    if let Some(fill) = self.gradient(layer, top, bottom) {
                        unsafe { self.target.FillRoundedRectangle(&shape, &fill) };
                    }
                }
            }
        }
        if look.shelf {
            self.shelf(scene);
        }
        let hi = look.hi.unwrap_or(scene.palette.hi);
        if hi.a > 0.0 {
            self.light_edge(scene, radius, hi);
        }
        if let Some((from, to)) = look.outline {
            if let Some(line) = self.gradient(
                &[(0.0, from), (1.0, to)],
                v(bar.x, 0.0),
                v(bar.x + bar.width, 0.0),
            ) {
                unsafe {
                    self.target
                        .DrawRoundedRectangle(&shape, &line, 1.0, None::<&ID2D1StrokeStyle>)
                };
            }
        } else if look.border {
            let line = self.solid(scene.palette.hair).clone();
            unsafe {
                self.target
                    .DrawRoundedRectangle(&shape, &line, 1.0, None::<&ID2D1StrokeStyle>)
            };
        }
    }

    /// The bar as its style's skin (spec 0020 §3): the ends of the image as
    /// they are and the middle in tiles, at the bar's height, without
    /// smoothing for pixel art.
    fn skin_bar(&mut self, scene: &Scene) {
        let Some(skin) = crate::domain::skin::skin_of(scene.look.style) else {
            return;
        };
        if !self.skins.contains_key(skin.id) {
            let Some(bitmap) = self.bitmap(skin.png) else {
                return;
            };
            self.skins.insert(skin.id, bitmap);
        }
        let Some(bitmap) = self.skins.get(skin.id) else {
            return;
        };
        let meta = skin.meta();
        let bar = scene.bar;
        let mode = if meta.pixelated {
            D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR
        } else {
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR
        };
        let to_rect = |r: crate::domain::skin::Rect| D2D_RECT_F {
            left: r.x as f32,
            top: r.y as f32,
            right: (r.x + r.width) as f32,
            bottom: (r.y + r.height) as f32,
        };
        let area = crate::domain::skin::Rect {
            x: bar.x,
            y: bar.y,
            width: bar.width,
            height: bar.height,
        };
        for piece in crate::domain::skin::pieces(meta, area) {
            unsafe {
                self.target.DrawBitmap(
                    bitmap,
                    Some(&to_rect(piece.to)),
                    1.0,
                    mode,
                    Some(&to_rect(piece.from)),
                )
            };
        }
    }

    /// The silver shelf under the tiles (Prateleira Aqua): narrower on the
    /// side away from the screen edge.
    fn shelf(&mut self, scene: &Scene) {
        let bar = scene.bar;
        let height = scene.pad + 9.0;
        let inset = bar.width * 0.03;
        let (near, far) = match scene.edge {
            DockEdge::Bottom => (bar.y + bar.height, bar.y + bar.height - height),
            DockEdge::Top => (bar.y, bar.y + height),
        };
        let Ok(geometry) = (unsafe { self.factory.CreatePathGeometry() }) else {
            return;
        };
        let built = unsafe {
            geometry.Open().and_then(|sink| {
                sink.BeginFigure(v(bar.x, near), D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(v(bar.x + bar.width, near));
                sink.AddLine(v(bar.x + bar.width - inset, far));
                sink.AddLine(v(bar.x + inset, far));
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close()
            })
        };
        if built.is_err() {
            return;
        }
        let stops = [
            (0.0, Color::rgb(0xf6, 0xf8, 0xfc)),
            (0.55, Color::rgb(0xb4, 0xba, 0xc6)),
            (1.0, Color::rgb(0x85, 0x8c, 0x9b)),
        ];
        if let Some(fill) = self.gradient(&stops, v(0.0, far), v(0.0, near)) {
            unsafe {
                self.target
                    .FillGeometry(&geometry, &fill, None::<&ID2D1Brush>)
            };
        }
    }

    /// The light edge, away from the screen edge (The Light Edge Rule).
    fn light_edge(&mut self, scene: &Scene, radius: f32, hi: Color) {
        let bar = scene.bar;
        let inner = D2D1_ROUNDED_RECT {
            rect: rect(Area {
                x: bar.x + 1.5,
                y: bar.y + 1.5,
                width: bar.width - 3.0,
                height: bar.height - 3.0,
            }),
            radiusX: (radius - 1.0).max(0.0),
            radiusY: (radius - 1.0).max(0.0),
        };
        let lit = 2.0 / bar.height as f32;
        let (start, end) = match scene.edge {
            DockEdge::Bottom => (v(0.0, bar.y), v(0.0, bar.y + bar.height)),
            DockEdge::Top => (v(0.0, bar.y + bar.height), v(0.0, bar.y)),
        };
        if let Some(edge) = self.gradient(&[(0.0, hi), (lit, hi.with_alpha(0.0))], start, end) {
            unsafe {
                self.target
                    .DrawRoundedRectangle(&inner, &edge, 1.0, None::<&ID2D1StrokeStyle>)
            };
        }
    }

    /// The tile's shadow and glaze, cut to the superellipse.
    fn glaze(&mut self, area: Area, glaze: Glaze, pressed: bool) {
        if area.width > area.height * 1.05 {
            self.wide_glaze(area, glaze, pressed);
            return;
        }
        let s = area.width / 100.0;
        // The drop shadow: two soft copies below the tile.
        for (dy, alpha, grow) in [(1.0, 0.12, 0.0), (3.5, 0.07, 1.5)] {
            let shadow = self.solid(Color::BLACK.with_alpha(alpha)).clone();
            let side = area.width + 2.0 * grow;
            unsafe {
                self.transform(&place(side / 100.0, area.x - grow, area.y - grow + dy));
                self.target
                    .FillGeometry(&self.squircle, &shadow, None::<&ID2D1Brush>);
            }
        }
        let Some(fill) = self.glaze_brush(glaze, pressed) else {
            return;
        };
        unsafe {
            self.transform(&place(s, area.x, area.y));
            self.target
                .FillGeometry(&self.squircle, &fill, None::<&ID2D1Brush>);
            self.target
                .FillGeometry(&self.squircle, &self.sheen, None::<&ID2D1Brush>);
            self.target.DrawGeometry(
                &self.squircle,
                &self.rim,
                (1.0 / s) as f32,
                None::<&ID2D1StrokeStyle>,
            );
            self.transform(&IDENTITY);
        }
    }

    /// Strokes the paths of a 24-unit view box inside `area` at `ratio` of
    /// its side.
    fn strokes(
        &mut self,
        area: Area,
        ratio: f64,
        paths: &'static [&'static str],
        ink: Color,
        width: f64,
    ) {
        let side = area.width * ratio;
        let k = side / 24.0;
        let x = area.x + (area.width - side) / 2.0;
        let y = area.y + (area.height - side) / 2.0;
        let brush = self.solid(ink).clone();
        for &text in paths {
            let Some(geometry) = self.glyph(text) else {
                continue;
            };
            unsafe {
                self.transform(&place(k, x, y));
                self.target
                    .DrawGeometry(&geometry, &brush, width as f32, &self.round);
            }
        }
        self.transform(&IDENTITY);
    }

    fn text(&mut self, text: &str, area: Area, size: f64, weight: i32, display: bool, ink: Color) {
        let Some(format) = self.format(size, weight, display) else {
            return;
        };
        let wide: Vec<u16> = text.encode_utf16().collect();
        let brush = self.solid(ink).clone();
        unsafe {
            self.target.DrawText(
                &wide,
                &format,
                &rect(area),
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            )
        };
    }

    /// The tile mirrored beyond its screen-side edge, fading out within the
    /// bar's padding (`-webkit-box-reflect` with a mask of 60 % to nothing
    /// over 15 % of the tile).
    fn reflection(&mut self, scene: &Scene, tile: &Tile) {
        let area = tile.area;
        let reach = area.height * 0.15;
        let (edge, start, end, clip) = match scene.edge {
            DockEdge::Bottom => {
                let edge = area.y + area.height;
                (
                    edge,
                    v(0.0, edge),
                    v(0.0, edge + reach),
                    Area {
                        y: edge,
                        height: reach,
                        ..area
                    },
                )
            }
            DockEdge::Top => (
                area.y,
                v(0.0, area.y),
                v(0.0, area.y - reach),
                Area {
                    y: area.y - reach,
                    height: reach,
                    ..area
                },
            ),
        };
        let Some(fade) = self.gradient(
            &[
                (0.0, Color::WHITE.with_alpha(0.6)),
                (1.0, Color::WHITE.with_alpha(0.0)),
            ],
            start,
            end,
        ) else {
            return;
        };
        let Ok(brush) = fade.cast::<ID2D1Brush>() else {
            return;
        };
        self.transform(&IDENTITY);
        unsafe {
            let parameters = D2D1_LAYER_PARAMETERS {
                contentBounds: rect(clip),
                geometricMask: std::mem::ManuallyDrop::new(None),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: IDENTITY,
                opacity: 1.0,
                opacityBrush: std::mem::ManuallyDrop::new(Some(brush)),
                layerOptions: D2D1_LAYER_OPTIONS_NONE,
            };
            self.target.PushLayer(
                &parameters,
                None::<&windows::Win32::Graphics::Direct2D::ID2D1Layer>,
            );
            let mut parameters = parameters;
            std::mem::ManuallyDrop::drop(&mut parameters.opacityBrush);
        }
        self.base = mirror(edge);
        let plain = Tile {
            area,
            face: match &tile.face {
                Face::App {
                    icon,
                    letter,
                    shape,
                    ..
                } => Face::App {
                    icon: icon.clone(),
                    letter: letter.clone(),
                    shape: *shape,
                    badge: None,
                },
                Face::Glyph { glaze, paths } => Face::Glyph {
                    glaze: *glaze,
                    paths,
                },
                Face::Symbol { glaze, glyph } => Face::Symbol {
                    glaze: *glaze,
                    glyph: *glyph,
                },
                Face::Time { glaze, text } => Face::Time {
                    glaze: *glaze,
                    text: text.clone(),
                },
                Face::Dial {
                    glaze,
                    hours,
                    minutes,
                } => Face::Dial {
                    glaze: *glaze,
                    hours: *hours,
                    minutes: *minutes,
                },
                Face::Ring {
                    glaze,
                    fraction,
                    text,
                } => Face::Ring {
                    glaze: *glaze,
                    fraction: *fraction,
                    text: text.clone(),
                },
                Face::Meters {
                    glaze,
                    style,
                    meters,
                    graph,
                } => Face::Meters {
                    glaze: *glaze,
                    style: *style,
                    meters: meters.clone(),
                    graph: graph.clone(),
                },
                Face::Agenda { glaze, lines } => Face::Agenda {
                    glaze: *glaze,
                    lines: lines.clone(),
                },
                Face::Media {
                    glaze,
                    art,
                    playing,
                    title,
                    artist,
                    idle,
                    badge,
                    controls,
                    since,
                } => Face::Media {
                    glaze: *glaze,
                    art: art.clone(),
                    playing: *playing,
                    title: title.clone(),
                    artist: artist.clone(),
                    idle: *idle,
                    badge: badge.clone(),
                    controls: *controls,
                    since: *since,
                },
                Face::Weather {
                    glaze,
                    sky,
                    temperature,
                    range,
                } => Face::Weather {
                    glaze: *glaze,
                    sky,
                    temperature: temperature.clone(),
                    range: range.clone(),
                },
                Face::Rates {
                    glaze,
                    down,
                    up,
                    graph,
                } => Face::Rates {
                    glaze: *glaze,
                    down: down.clone(),
                    up: up.clone(),
                    graph: graph.clone(),
                },
            },
            pressed: false,
            attention: false,
            dragging: false,
            raised: false,
            dot: None,
            item: tile.item,
            notice: None,
        };
        self.tile(scene, &plain);
        self.base = IDENTITY;
        self.transform(&IDENTITY);
        unsafe { self.target.PopLayer() };
    }

    fn tile(&mut self, scene: &Scene, tile: &Tile) {
        let area = tile.area;
        // Whatever the tile draws without a transform of its own goes through
        // the base too.
        self.transform(&IDENTITY);
        match &tile.face {
            Face::Glyph { glaze, paths } => {
                self.glaze(area, *glaze, tile.pressed);
                self.strokes(area, 0.54, paths, Color::WHITE, 1.75);
            }
            Face::Symbol { glaze, glyph } => {
                self.glaze(area, *glaze, tile.pressed);
                // About the size of the line glyphs (`strokes` at 0.54).
                self.icon_glyph(*glyph, area, area.width * 0.46, Color::WHITE);
            }
            Face::App {
                icon,
                letter,
                shape,
                badge,
            } => {
                let plate = icon.is_none() || *shape == IconShape::Plate;
                if plate {
                    self.glaze(area, Glaze::APP, tile.pressed);
                }
                match icon {
                    Some(icon) => {
                        let ratio = match shape {
                            IconShape::Plate => 0.78,
                            IconShape::Uniform => 1.0,
                            IconShape::Original => 0.9,
                        };
                        let side = area.width * ratio;
                        let place_at = Area {
                            x: area.x + (area.width - side) / 2.0,
                            y: area.y + (area.height - side) / 2.0,
                            width: side,
                            height: side,
                        };
                        unsafe {
                            self.target.DrawBitmap(
                                icon,
                                Some(&rect(place_at)),
                                if tile.pressed { 0.84 } else { 1.0 },
                                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                                None,
                            )
                        };
                    }
                    None => {
                        self.text(letter, area, area.width * 0.42, 600, false, APP_INK);
                    }
                }
                if let Some(badge) = badge {
                    self.badge(scene, area, badge);
                }
            }
            Face::Time { glaze, text } => {
                self.glaze(area, *glaze, tile.pressed);
                let size = area.width * 0.28;
                let shadow = Area {
                    y: area.y + 1.0,
                    ..area
                };
                self.text(text, shadow, size, 600, true, Color::BLACK.with_alpha(0.18));
                self.text(text, area, size, 600, true, Color::WHITE);
            }
            Face::Dial {
                glaze,
                hours,
                minutes,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                self.dial(area, *hours, *minutes);
            }
            Face::Ring {
                glaze,
                fraction,
                text,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                self.ring(area, *fraction);
                if let Some(text) = text {
                    self.percent(text, area);
                }
            }
            Face::Meters {
                glaze,
                style,
                meters,
                graph,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                match graph {
                    // Three icons: the meters on the left, the graph on the
                    // right.
                    Some(graph) => {
                        let u = area.height;
                        let left = Area {
                            width: area.width * 0.56,
                            ..area
                        };
                        self.meters(left, *style, meters);
                        let plot = Area {
                            x: area.x + area.width * 0.56,
                            y: area.y + u * 0.2,
                            width: area.width * 0.44 - u * 0.16,
                            height: u * 0.6,
                        };
                        self.graph_line(plot, graph, true);
                        self.graph_line(plot, graph, false);
                    }
                    None => self.meters(area, *style, meters),
                }
            }
            Face::Agenda { glaze, lines } => {
                self.glaze(area, *glaze, tile.pressed);
                self.agenda(area, lines);
            }
            Face::Rates {
                glaze,
                down,
                up,
                graph,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                self.rates(area, down, up, graph.as_ref());
            }
            Face::Weather {
                glaze,
                sky,
                temperature,
                range,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                self.weather(area, sky, temperature.as_deref(), range.as_deref());
            }
            Face::Media {
                glaze,
                art,
                playing,
                title,
                artist,
                idle,
                badge,
                controls,
                since,
            } => {
                self.glaze(area, *glaze, tile.pressed);
                if *idle {
                    let u = area.height;
                    let middle = Area {
                        x: area.x + (area.width - u) / 2.0,
                        width: u,
                        ..area
                    };
                    self.strokes(middle, 0.5, NOTE, Color::WHITE, 1.75);
                } else {
                    self.media(
                        scene,
                        area,
                        art.as_ref(),
                        badge.as_ref(),
                        *controls,
                        *since,
                        *playing,
                        title.as_deref(),
                        artist.as_deref(),
                    );
                }
            }
        }
    }

    /// The Tocando agora: the cover in a rounded square at the left of the
    /// tile (all of it on a square tile), and the title and the artist to
    /// its right on a wide one.
    #[allow(clippy::too_many_arguments)]
    fn media(
        &mut self,
        scene: &Scene,
        area: Area,
        art: Option<&ID2D1Bitmap>,
        badge: Option<&ID2D1Bitmap>,
        controls: bool,
        since: f64,
        playing: bool,
        title: Option<&str>,
        artist: Option<&str>,
    ) {
        let u = area.height;
        let inset = u * 0.12;
        let side = u - 2.0 * inset;
        let cover = Area {
            x: area.x + inset,
            y: area.y + inset,
            width: side,
            height: side,
        };
        match art {
            Some(art) => {
                self.rounded_bitmap(cover, side * 0.2, art);
                if !playing {
                    let shade = self.solid(Color::BLACK.with_alpha(0.38)).clone();
                    unsafe {
                        self.target.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: rect(cover),
                                radiusX: (side * 0.2) as f32,
                                radiusY: (side * 0.2) as f32,
                            },
                            &shade,
                        );
                    }
                    self.strokes(cover, 0.4, PAUSE, Color::WHITE, 2.0);
                }
                if let Some(badge) = badge {
                    self.badge(scene, cover, badge);
                }
            }
            None => {
                let paths = if playing { NOTE } else { PAUSE };
                self.strokes(cover, 0.62, paths, Color::WHITE, 1.75);
                if let Some(badge) = badge {
                    self.badge(scene, cover, badge);
                }
            }
        }
        let Some(title) = title else {
            return;
        };
        if controls {
            let toggle = if playing { PAUSE } else { PLAY };
            for (button, paths) in dock_layout::media_buttons(area)
                .into_iter()
                .zip([PREVIOUS, toggle, NEXT])
            {
                self.strokes(button, 1.0, paths, Color::WHITE, 1.6);
            }
        }
        let x = cover.x + side + inset;
        let room = area.x + area.width - inset - x;
        // On a small dock the text keeps a readable size.
        let t = if u < 44.0 { u * 1.25 } else { u };
        if room <= 0.0 {
            return;
        }
        if controls {
            // The upper line: the title, and the artist after it, fainter.
            let size = t * 0.17;
            let mut runs = vec![(title.to_owned(), size, 600, Color::WHITE)];
            if let Some(artist) = artist.filter(|artist| !artist.is_empty()) {
                runs.push((
                    format!("  ·  {artist}"),
                    size * 0.9,
                    500,
                    Color::WHITE.with_alpha(0.78),
                ));
            }
            self.rolling(&runs, x, area.y + u * 0.33, room, since);
            return;
        }
        let rows: Vec<(String, f64, i32, Color)> = match artist.filter(|artist| !artist.is_empty())
        {
            Some(artist) => vec![
                (title.to_owned(), t * 0.19, 600, Color::WHITE),
                (
                    artist.to_owned(),
                    t * 0.16,
                    500,
                    Color::WHITE.with_alpha(0.78),
                ),
            ],
            None => vec![(title.to_owned(), t * 0.19, 600, Color::WHITE)],
        };
        let heights: Vec<f64> = rows.iter().map(|(_, size, _, _)| size * 1.3).collect();
        let mut y = area.y + (u - heights.iter().sum::<f64>()) / 2.0;
        for (row, height) in rows.into_iter().zip(heights) {
            self.rolling(std::slice::from_ref(&row), x, y + height / 2.0, room, since);
            y += height;
        }
    }

    /// Texts side by side (`runs`: each with its size, weight and ink), from
    /// `x`, centered on `cy`, in `room`: when they don't fit, they roll in it
    /// (`dock_layout::marquee`), `since` seconds after they showed.
    fn rolling(
        &mut self,
        runs: &[(String, f64, i32, Color)],
        x: f64,
        cy: f64,
        room: f64,
        since: f64,
    ) {
        let widths: Vec<f64> = runs
            .iter()
            .map(|(text, size, weight, _)| self.text_width(text, *size, *weight, true))
            .collect();
        let total: f64 = widths.iter().sum();
        let (offset, next) = dock_layout::marquee(since, total - room);
        let tall = runs.iter().map(|(_, size, _, _)| *size).fold(0.0, f64::max) * 1.3;
        let clip = Area {
            x,
            y: cy - tall / 2.0,
            width: room,
            height: tall,
        };
        unsafe {
            self.target
                .PushAxisAlignedClip(&rect(clip), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }
        let mut from = x - offset;
        for ((text, size, weight, ink), width) in runs.iter().zip(widths) {
            let height = size * 1.3;
            self.text(
                text,
                Area {
                    x: from,
                    y: cy - height / 2.0,
                    width,
                    height,
                },
                *size,
                *weight,
                true,
                *ink,
            );
            from += width;
        }
        unsafe { self.target.PopAxisAlignedClip() };
        if next.is_finite() {
            self.marquee = Some(self.marquee.map_or(next, |due| due.min(next)));
        }
    }

    /// `bitmap` drawn over `area`, its corners rounded by `radius`.
    fn rounded_bitmap(&mut self, area: Area, radius: f64, bitmap: &ID2D1Bitmap) {
        let bounds = rect(area);
        let Ok(round) = (unsafe {
            self.factory
                .CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                    rect: bounds,
                    radiusX: radius as f32,
                    radiusY: radius as f32,
                })
        }) else {
            return;
        };
        let Ok(mask) = round.cast::<ID2D1Geometry>() else {
            return;
        };
        unsafe {
            let parameters = D2D1_LAYER_PARAMETERS {
                contentBounds: bounds,
                geometricMask: std::mem::ManuallyDrop::new(Some(mask)),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: IDENTITY,
                opacity: 1.0,
                opacityBrush: std::mem::ManuallyDrop::new(None),
                layerOptions: D2D1_LAYER_OPTIONS_NONE,
            };
            self.target.PushLayer(
                &parameters,
                None::<&windows::Win32::Graphics::Direct2D::ID2D1Layer>,
            );
            self.target.DrawBitmap(
                bitmap,
                Some(&bounds),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            self.target.PopLayer();
            let mut parameters = parameters;
            std::mem::ManuallyDrop::drop(&mut parameters.geometricMask);
        }
    }

    /// `text` with its left edge at `x`, centered on `cy`; its width.
    fn text_from(&mut self, text: &str, x: f64, cy: f64, size: f64, ink: Color) -> f64 {
        let width = self.text_width(text, size, 600, true);
        let area = Area {
            x,
            y: cy - size,
            width,
            height: size * 2.0,
        };
        self.text(text, area, size, 600, true, ink);
        width
    }

    /// `text` with its right edge at `right`, centered on `cy`.
    fn text_to(&mut self, text: &str, right: f64, cy: f64, size: f64, ink: Color) {
        let width = self.text_width(text, size, 600, true);
        self.text_from(text, right - width, cy, size, ink);
    }

    /// A rounded bar: the faint track and, over it, `fraction` of it in
    /// white, from the bottom when it stands, from the left when it lies.
    fn meter_bar(&mut self, bar: Area, fraction: Option<f32>) {
        let radius = (bar.width.min(bar.height) / 2.0) as f32;
        let rounded = |area: Area| D2D1_ROUNDED_RECT {
            rect: rect(area),
            radiusX: radius,
            radiusY: radius,
        };
        let track = self.solid(Color::WHITE.with_alpha(0.28)).clone();
        unsafe { self.target.FillRoundedRectangle(&rounded(bar), &track) };
        let Some(fraction) = fraction
            .map(|f| f64::from(f.clamp(0.0, 1.0)))
            .filter(|f| *f > 0.0)
        else {
            return;
        };
        let filled = if bar.height > bar.width {
            let height = (bar.height * fraction).max(bar.width);
            Area {
                y: bar.y + bar.height - height,
                height,
                ..bar
            }
        } else {
            Area {
                width: (bar.width * fraction).max(bar.height),
                ..bar
            }
        };
        let white = self.solid(Color::WHITE).clone();
        unsafe { self.target.FillRoundedRectangle(&rounded(filled), &white) };
    }

    /// The Atividade's meters (spec 0017 F1), in a tile one or two icons
    /// wide.
    fn meters(&mut self, area: Area, style: ActivityStyle, meters: &[Meter]) {
        let u = area.height;
        let wide = area.width > u * 1.2;
        let faint = Color::WHITE.with_alpha(0.75);
        let count = meters.len().max(1) as f64;
        match style {
            ActivityStyle::Bars if wide => {
                // A row per meter: the name, the bar, the figure.
                let size = u * 0.15;
                let row = u * 0.3;
                let top = area.y + (u - row * count) / 2.0;
                let left = area.x + u * 0.14;
                let right = area.x + area.width - u * 0.14;
                // The bars start after the longest name and end before the
                // widest figure, so no text runs under a bar.
                let widest = |painter: &mut Self, texts: &mut dyn Iterator<Item = &str>| {
                    texts.fold(0.0_f64, |most, text| {
                        most.max(painter.text_width(text, size, 600, true))
                    })
                };
                // Less than two icons of room (beside a graph): the short names.
                let name = |meter: &Meter| {
                    if area.width < u * 2.0 {
                        meter.short
                    } else {
                        meter.label
                    }
                };
                let names = widest(self, &mut meters.iter().map(name));
                let figures = widest(self, &mut meters.iter().map(|meter| meter.text.as_str()));
                let start = left + names + u * 0.1;
                let end = right - figures - u * 0.1;
                for (index, meter) in meters.iter().enumerate() {
                    let cy = top + row * (index as f64 + 0.5);
                    self.text_from(name(meter), left, cy, size, faint);
                    self.text_to(&meter.text, right, cy, size, Color::WHITE);
                    let bar = Area {
                        x: start,
                        y: cy - u * 0.04,
                        width: (end - start).max(u * 0.1),
                        height: u * 0.08,
                    };
                    self.meter_bar(bar, meter.fraction);
                }
            }
            ActivityStyle::Bars => {
                // Standing bars, side by side, the short name under each.
                let bar_width = u * 0.13;
                let gap = u * 0.16;
                let total = bar_width * count + gap * (count - 1.0);
                let height = u * 0.48;
                let top = area.y + u * 0.14;
                for (index, meter) in meters.iter().enumerate() {
                    let x = area.x + (area.width - total) / 2.0 + (bar_width + gap) * index as f64;
                    let bar = Area {
                        x,
                        y: top,
                        width: bar_width,
                        height,
                    };
                    self.meter_bar(bar, meter.fraction);
                    let label = Area {
                        x: x + bar_width / 2.0 - u * 0.25,
                        y: top + height + u * 0.04,
                        width: u * 0.5,
                        height: u * 0.18,
                    };
                    self.text(meter.short, label, u * 0.13, 600, true, faint);
                }
            }
            ActivityStyle::Rings if wide || meters.len() == 1 => {
                // A ring per meter, each in an icon's square, its figure in it.
                let side = area.width / count;
                for (index, meter) in meters.iter().enumerate() {
                    let square = Area {
                        x: area.x + side * index as f64 + (side - u) / 2.0,
                        y: area.y,
                        width: u,
                        height: u,
                    };
                    self.ring(square, meter.fraction);
                    self.percent(&meter.text, square);
                }
            }
            ActivityStyle::Rings => {
                // Two rings, one inside the other: the processor outside.
                for (index, meter) in meters.iter().enumerate() {
                    let side = u * (1.0 - 0.32 * index as f64);
                    let square = Area {
                        x: area.x + (area.width - side) / 2.0,
                        y: area.y + (u - side) / 2.0,
                        width: side,
                        height: side,
                    };
                    self.ring(square, meter.fraction);
                }
            }
            ActivityStyle::Numbers if meters.len() == 1 && !wide => {
                // The name above, the figure large.
                let meter = &meters[0];
                let label = Area {
                    y: area.y + u * 0.1,
                    height: u * 0.22,
                    ..area
                };
                self.text(meter.label, label, u * 0.15, 600, true, faint);
                let figure = Area {
                    y: area.y + u * 0.12,
                    height: u * 0.88,
                    ..area
                };
                self.percent(&meter.text, figure);
            }
            ActivityStyle::Numbers => {
                // A row per meter: the name, then the figure.
                let size = if wide { u * 0.2 } else { u * 0.16 };
                let row = u * 0.3;
                let top = area.y + (u - row * count) / 2.0;
                let inset = u * if wide { 0.18 } else { 0.1 };
                for (index, meter) in meters.iter().enumerate() {
                    let cy = top + row * (index as f64 + 0.5);
                    let label = if wide { meter.label } else { meter.short };
                    self.text_from(label, area.x + inset, cy, size * 0.85, faint);
                    self.text_to(
                        &meter.text,
                        area.x + area.width - inset,
                        cy,
                        size,
                        Color::WHITE,
                    );
                }
            }
        }
    }

    /// `text`, cut with an ellipsis until it is at most `room` wide.
    fn fit(&mut self, text: &str, room: f64, size: f64, weight: i32) -> String {
        if self.text_width(text, size, weight, true) <= room {
            return text.to_owned();
        }
        let mut chars: Vec<char> = text.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let cut: String = chars.iter().collect::<String>().trim_end().to_owned() + "…";
            if self.text_width(&cut, size, weight, true) <= room {
                return cut;
            }
        }
        String::new()
    }

    /// The Calendário's lines, one under the other, the block centered on
    /// the tile's height.
    fn agenda(&mut self, area: Area, lines: &[AgendaLine]) {
        let u = area.height;
        let wide = area.width > u * 1.2;
        let inset = u * if wide { 0.16 } else { 0.08 };
        let room = area.width - 2.0 * inset;
        let heights: Vec<f64> = lines.iter().map(|line| u * line.size * 1.3).collect();
        let mut y = area.y + (u - heights.iter().sum::<f64>()) / 2.0;
        for (line, height) in lines.iter().zip(heights) {
            let size = u * line.size;
            let weight = if line.strong { 600 } else { 500 };
            let text = self.fit(&line.text, room, size, weight);
            let ink = if line.strong {
                Color::WHITE
            } else {
                Color::WHITE.with_alpha(0.78)
            };
            if wide {
                let width = self.text_width(&text, size, weight, true);
                let place = Area {
                    x: area.x + inset,
                    y,
                    width,
                    height,
                };
                self.text(&text, place, size, weight, true, ink);
            } else {
                let place = Area { y, height, ..area };
                self.text(&text, place, size, weight, true, ink);
            }
            y += height;
        }
    }

    /// The Clima: the sky alone until the weather is read; on a square
    /// tile, the temperature under it; on a wide one, the sky on the left
    /// and the temperature with the day's range beside it.
    fn weather(
        &mut self,
        area: Area,
        sky: &'static [&'static str],
        temperature: Option<&str>,
        range: Option<&str>,
    ) {
        let u = area.height;
        let square = Area { width: u, ..area };
        let Some(temperature) = temperature else {
            let middle = Area {
                x: area.x + (area.width - u) / 2.0,
                ..square
            };
            self.strokes(middle, 0.54, sky, Color::WHITE, 1.75);
            return;
        };
        if area.width > u * 1.5 {
            self.strokes(square, 0.56, sky, Color::WHITE, 1.75);
            let x = area.x + u * 0.92;
            match range {
                Some(range) => {
                    self.text_from(temperature, x, area.y + u * 0.4, u * 0.3, Color::WHITE);
                    let faint = Color::WHITE.with_alpha(0.8);
                    self.text_from(range, x, area.y + u * 0.7, u * 0.15, faint);
                }
                None => {
                    self.text_from(temperature, x, area.y + u * 0.5, u * 0.3, Color::WHITE);
                }
            }
            return;
        }
        let top = Area {
            y: area.y + u * 0.06,
            height: u * 0.56,
            ..square
        };
        self.strokes(top, 0.42, sky, Color::WHITE, 1.9);
        let under = Area {
            y: area.y + u * 0.56,
            height: u * 0.34,
            ..square
        };
        self.text(temperature, under, u * 0.26, 600, true, Color::WHITE);
    }

    /// The Atividade de rede: the two rates and, on a wide tile, the graph
    /// of the last minute under them.
    fn rates(&mut self, area: Area, down: &str, up: &str, graph: Option<&Graph>) {
        let u = area.height;
        let faint = Color::WHITE.with_alpha(0.75);
        let rows = [("\u{2193}", down), ("\u{2191}", up)];
        let Some(graph) = graph else {
            // One icon: a row per rate, the arrow on the left.
            let size = u * 0.16;
            for (index, (arrow, rate)) in rows.into_iter().enumerate() {
                let cy = area.y + u * (0.33 + 0.34 * index as f64);
                self.text_from(arrow, area.x + u * 0.1, cy, size, faint);
                self.text_to(rate, area.x + area.width - u * 0.1, cy, size, Color::WHITE);
            }
            return;
        };
        if area.width > u * 2.5 {
            // Three icons: the rates one over the other on the left, the
            // graph as tall as the tile beside them.
            let size = u * 0.18;
            for (index, (arrow, rate)) in rows.into_iter().enumerate() {
                let cy = area.y + u * (0.33 + 0.34 * index as f64);
                let x = area.x + u * 0.16;
                let width = self.text_from(arrow, x, cy, size, faint);
                self.text_from(rate, x + width + u * 0.04, cy, size, Color::WHITE);
            }
            let plot = Area {
                x: area.x + area.width * 0.48,
                y: area.y + u * 0.18,
                width: area.width * 0.52 - u * 0.16,
                height: u * 0.64,
            };
            self.graph_line(plot, &graph.down, true);
            self.graph_line(plot, &graph.up, false);
            return;
        }
        let plot = Area {
            x: area.x + u * 0.12,
            y: area.y + u * 0.46,
            width: area.width - u * 0.24,
            height: u * 0.38,
        };
        self.graph_line(plot, &graph.down, true);
        self.graph_line(plot, &graph.up, false);
        // The two rates side by side above it, down on the left.
        let size = u * 0.17;
        let cy = area.y + u * 0.26;
        for (index, (arrow, rate)) in rows.into_iter().enumerate() {
            let x = area.x + area.width / 2.0 * index as f64 + u * 0.12;
            let width = self.text_from(arrow, x, cy, size, faint);
            self.text_from(rate, x + width + u * 0.03, cy, size, Color::WHITE);
        }
    }

    /// One series of the network graph in `plot`: values 0–1, the newest
    /// on the right; the rate down filled, the rate up as a line.
    fn graph_line(&mut self, plot: Area, values: &[f32], filled: bool) {
        if values.len() < 2 {
            return;
        }
        let step = plot.width / (HISTORY - 1) as f64;
        let start = plot.x + plot.width - step * (values.len() - 1) as f64;
        let bottom = plot.y + plot.height;
        let at = |index: usize, value: f32| {
            v(
                start + step * index as f64,
                bottom - plot.height * f64::from(value.clamp(0.0, 1.0)),
            )
        };
        let Ok(path) = (unsafe { self.factory.CreatePathGeometry() }) else {
            return;
        };
        let built = unsafe {
            path.Open().and_then(|sink| {
                if filled {
                    sink.BeginFigure(v(start, bottom), D2D1_FIGURE_BEGIN_FILLED);
                    for (index, value) in values.iter().enumerate() {
                        sink.AddLine(at(index, *value));
                    }
                    sink.AddLine(v(plot.x + plot.width, bottom));
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                } else {
                    sink.BeginFigure(at(0, values[0]), D2D1_FIGURE_BEGIN_HOLLOW);
                    for (index, value) in values.iter().enumerate().skip(1) {
                        sink.AddLine(at(index, *value));
                    }
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                }
                sink.Close()
            })
        };
        if built.is_err() {
            return;
        }
        if filled {
            let fill = self.solid(Color::WHITE.with_alpha(0.32)).clone();
            unsafe { self.target.FillGeometry(&path, &fill, None::<&ID2D1Brush>) };
        } else {
            let line = self.solid(Color::WHITE.with_alpha(0.9)).clone();
            unsafe { self.target.DrawGeometry(&path, &line, 1.4, &self.round) };
        }
    }

    /// The glaze of a tile wider than tall (spec 0017): a rounded
    /// rectangle with corners about as round as the squircle's, the same
    /// fill, sheen and rim stretched across it.
    fn wide_glaze(&mut self, area: Area, glaze: Glaze, pressed: bool) {
        let radius = area.height * 0.3;
        // Drawn in a 100 × 100 box stretched over the tile, so the brushes
        // made for that box fit; the corners are set back to round.
        let shape = |width: f64, height: f64| D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 100.0,
            },
            radiusX: (radius * 100.0 / width) as f32,
            radiusY: (radius * 100.0 / height) as f32,
        };
        let stretch = |x: f64, y: f64, width: f64, height: f64| Matrix3x2 {
            M11: (width / 100.0) as f32,
            M12: 0.0,
            M21: 0.0,
            M22: (height / 100.0) as f32,
            M31: x as f32,
            M32: y as f32,
        };
        for (dy, alpha, grow) in [(1.0, 0.12, 0.0), (3.5, 0.07, 1.5)] {
            let shadow = self.solid(Color::BLACK.with_alpha(alpha)).clone();
            let (width, height) = (area.width + 2.0 * grow, area.height + 2.0 * grow);
            self.transform(&stretch(area.x - grow, area.y - grow + dy, width, height));
            unsafe {
                self.target
                    .FillRoundedRectangle(&shape(width, height), &shadow)
            };
        }
        let Some(fill) = self.glaze_brush(glaze, pressed) else {
            self.transform(&IDENTITY);
            return;
        };
        let tile = shape(area.width, area.height);
        self.transform(&stretch(area.x, area.y, area.width, area.height));
        unsafe {
            self.target.FillRoundedRectangle(&tile, &fill);
            self.target.FillRoundedRectangle(&tile, &self.sheen);
            self.target.DrawRoundedRectangle(
                &tile,
                &self.rim,
                (100.0 / area.height) as f32,
                None::<&ID2D1StrokeStyle>,
            );
        }
        self.transform(&IDENTITY);
    }

    /// The progress in the ring: the number, and a smaller, fainter `%`.
    fn percent(&mut self, text: &str, area: Area) {
        let size = area.width * 0.23;
        let Some(number) = text.strip_suffix('%') else {
            self.text(text, area, size, 600, true, Color::WHITE);
            return;
        };
        let small = size * 0.7;
        let number_width = self.text_width(number, size, 600, true);
        let sign_width = self.text_width("%", small, 600, true);
        let left = area.x + (area.width - number_width - sign_width) / 2.0;
        self.text(
            number,
            Area {
                x: left,
                width: number_width,
                ..area
            },
            size,
            600,
            true,
            Color::WHITE,
        );
        // On the number's baseline, about: a little lower than centered.
        self.text(
            "%",
            Area {
                x: left + number_width,
                y: area.y + (size - small) * 0.35,
                width: sign_width,
                ..area
            },
            small,
            600,
            true,
            Color::WHITE.with_alpha(0.85),
        );
    }

    /// The photo of a profile on the corner of an open program's icon, in a
    /// ring of the bar's color.
    fn badge(&mut self, scene: &Scene, area: Area, badge: &ID2D1Bitmap) {
        let side = area.width * 0.44;
        let center = v(
            area.x + area.width + 2.0 - side / 2.0,
            area.y + area.height + 2.0 - side / 2.0,
        );
        let radius = (side / 2.0) as f32;
        let ring = self.solid(scene.palette.surface.with_alpha(1.0)).clone();
        unsafe {
            self.target.FillEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: radius + 1.5,
                    radiusY: radius + 1.5,
                },
                &ring,
            );
        }
        let Ok(circle) = (unsafe {
            self.factory.CreateEllipseGeometry(&D2D1_ELLIPSE {
                point: center,
                radiusX: radius,
                radiusY: radius,
            })
        }) else {
            return;
        };
        let Ok(mask) = circle.cast::<ID2D1Geometry>() else {
            return;
        };
        let bounds = D2D_RECT_F {
            left: center.X - radius,
            top: center.Y - radius,
            right: center.X + radius,
            bottom: center.Y + radius,
        };
        unsafe {
            let parameters = D2D1_LAYER_PARAMETERS {
                contentBounds: bounds,
                geometricMask: std::mem::ManuallyDrop::new(Some(mask)),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: IDENTITY,
                opacity: 1.0,
                opacityBrush: std::mem::ManuallyDrop::new(None),
                layerOptions: D2D1_LAYER_OPTIONS_NONE,
            };
            self.target.PushLayer(
                &parameters,
                None::<&windows::Win32::Graphics::Direct2D::ID2D1Layer>,
            );
            self.target.DrawBitmap(
                badge,
                Some(&bounds),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            self.target.PopLayer();
            let mut parameters = parameters;
            std::mem::ManuallyDrop::drop(&mut parameters.geometricMask);
        }
    }

    fn dial(&mut self, area: Area, hours: f32, minutes: f32) {
        let side = area.width * 0.76;
        let k = side / 24.0;
        let x0 = area.x + (area.width - side) / 2.0;
        let y0 = area.y + (area.height - side) / 2.0;
        let at = |x: f64, y: f64| v(x0 + x * k, y0 + y * k);
        let mark = self.solid(Color::WHITE.with_alpha(0.75)).clone();
        for (x, y) in [(12.0, 2.6), (21.4, 12.0), (12.0, 21.4), (2.6, 12.0)] {
            unsafe {
                self.target.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: at(x, y),
                        radiusX: (0.75 * k) as f32,
                        radiusY: (0.75 * k) as f32,
                    },
                    &mark,
                )
            };
        }
        let white = self.solid(Color::WHITE).clone();
        let hand = |degrees: f64, length: f64| {
            let radians = degrees.to_radians();
            at(12.0 + length * radians.sin(), 12.0 - length * radians.cos())
        };
        let hour = ((f64::from(hours) % 12.0) + f64::from(minutes) / 60.0) * 30.0;
        unsafe {
            self.target.DrawLine(
                at(12.0, 12.0),
                hand(hour, 5.0),
                &white,
                (2.1 * k) as f32,
                &self.round,
            );
            self.target.DrawLine(
                at(12.0, 12.0),
                hand(f64::from(minutes) * 6.0, 7.6),
                &white,
                (1.5 * k) as f32,
                &self.round,
            );
            self.target.FillEllipse(
                &D2D1_ELLIPSE {
                    point: at(12.0, 12.0),
                    radiusX: (1.25 * k) as f32,
                    radiusY: (1.25 * k) as f32,
                },
                &white,
            );
        }
    }

    fn ring(&mut self, area: Area, fraction: Option<f32>) {
        let side = area.width * 0.78;
        let k = side / 24.0;
        let center = v(area.x + area.width / 2.0, area.y + area.height / 2.0);
        let radius = 9.0 * k;
        let width = (2.2 * k) as f32;
        let track = self.solid(Color::WHITE.with_alpha(0.28)).clone();
        unsafe {
            self.target.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: radius as f32,
                    radiusY: radius as f32,
                },
                &track,
                width,
                &self.round,
            )
        };
        let Some(fraction) = fraction.filter(|f| *f > 0.0) else {
            return;
        };
        let white = self.solid(Color::WHITE).clone();
        if fraction >= 1.0 {
            unsafe {
                self.target.DrawEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: radius as f32,
                        radiusY: radius as f32,
                    },
                    &white,
                    width,
                    &self.round,
                )
            };
            return;
        }
        let angle = f64::from(fraction) * std::f64::consts::TAU;
        let start = v(f64::from(center.X), f64::from(center.Y) - radius);
        let end = v(
            f64::from(center.X) + radius * angle.sin(),
            f64::from(center.Y) - radius * angle.cos(),
        );
        let Ok(arc) = (unsafe { self.factory.CreatePathGeometry() }) else {
            return;
        };
        let built = unsafe {
            arc.Open().and_then(|sink| {
                sink.BeginFigure(start, D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: end,
                    size: D2D_SIZE_F {
                        width: radius as f32,
                        height: radius as f32,
                    },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: if fraction > 0.5 {
                        D2D1_ARC_SIZE_LARGE
                    } else {
                        D2D1_ARC_SIZE_SMALL
                    },
                });
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
                sink.Close()
            })
        };
        if built.is_ok() {
            unsafe { self.target.DrawGeometry(&arc, &white, width, &self.round) };
        }
    }

    /// The marks that sit on the bar, not on the tile: the dot, the badge of
    /// attention and the mark of a drop.
    fn marks(&mut self, scene: &Scene, tile: &Tile) {
        if let Some((x, opacity)) = tile.dot.filter(|(_, opacity)| *opacity > 0.0) {
            let ink = self
                .solid(scene.indicator_color.with_alpha(opacity))
                .clone();
            let edge_y = match scene.edge {
                DockEdge::Bottom => scene.bar.y + scene.bar.height - 1.0,
                DockEdge::Top => scene.bar.y + 1.0,
            };
            unsafe {
                match scene.indicator {
                    Indicator::Dot => {
                        let cy = match scene.edge {
                            DockEdge::Bottom => edge_y - 2.0,
                            DockEdge::Top => edge_y + 2.0,
                        };
                        self.target.FillEllipse(
                            &D2D1_ELLIPSE {
                                point: v(x, cy),
                                radiusX: 2.0,
                                radiusY: 2.0,
                            },
                            &ink,
                        );
                    }
                    Indicator::Dash => {
                        let top = match scene.edge {
                            DockEdge::Bottom => edge_y - 3.0,
                            DockEdge::Top => edge_y,
                        };
                        self.target.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: rect(Area {
                                    x: x - 7.0,
                                    y: top,
                                    width: 14.0,
                                    height: 3.0,
                                }),
                                radiusX: 1.5,
                                radiusY: 1.5,
                            },
                            &ink,
                        );
                    }
                }
            }
        }
        if let Some(notice) = tile.notice {
            self.notice(scene, tile.area, notice);
        }
        if tile.attention {
            let center = v(
                tile.area.x + tile.area.width + 2.0 - 5.0,
                tile.area.y - 2.0 + 5.0,
            );
            let ring = self.solid(scene.palette.surface.with_alpha(1.0)).clone();
            unsafe {
                self.target.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: 7.0,
                        radiusY: 7.0,
                    },
                    &ring,
                );
            }
            let danger = self.solid(scene.palette.danger).clone();
            unsafe {
                self.target.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: 5.0,
                        radiusY: 5.0,
                    },
                    &danger,
                );
            }
        }
    }

    /// The notification badge on a program's tile (spec 0016): the number
    /// in white on the danger color, in a ring of the bar's, or a dot for a
    /// badge with no number.
    fn notice(&mut self, scene: &Scene, tile: Area, notice: Badge) {
        let label = notice.label();
        let probe = dock_layout::badge_area(tile, 1.0);
        let size = dock_layout::badge_text_size(probe.height);
        let text_width = label
            .as_deref()
            .map_or(0.0, |text| self.text_width(text, size, 700, true));
        let area = dock_layout::badge_area(tile, text_width);
        let radius = (area.height / 2.0) as f32;
        let ring = self.solid(scene.palette.surface.with_alpha(1.0)).clone();
        let fill = self.solid(scene.palette.danger).clone();
        unsafe {
            self.target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rect(Area {
                        x: area.x - 1.5,
                        y: area.y - 1.5,
                        width: area.width + 3.0,
                        height: area.height + 3.0,
                    }),
                    radiusX: radius + 1.5,
                    radiusY: radius + 1.5,
                },
                &ring,
            );
            self.target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rect(area),
                    radiusX: radius,
                    radiusY: radius,
                },
                &fill,
            );
        }
        if let Some(text) = label {
            self.text(&text, area, size, 700, true, Color::WHITE);
        }
    }

    fn status(&mut self, scene: &Scene, status: &Status) {
        let area = status.area;
        if status.hovered {
            let ink = scene.look.ink.unwrap_or(scene.palette.ink);
            let hover = self.solid(ink.with_alpha(0.12)).clone();
            unsafe {
                self.target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: rect(area),
                        radiusX: 4.0,
                        radiusY: 4.0,
                    },
                    &hover,
                )
            };
        }
        // Two lines, 16 and 14 px, 1 px apart, centered.
        let top = area.y + (area.height - 31.0) / 2.0;
        let ink = scene.look.ink.unwrap_or(scene.palette.ink);
        // Without a bar behind it, a shadow keeps it legible.
        let shadows: &[f64] = if scene.look.flat { &[1.0] } else { &[] };
        for (dy, ink) in shadows
            .iter()
            .map(|&dy| (dy, Color::BLACK.with_alpha(0.7)))
            .chain(std::iter::once((0.0, ink)))
        {
            self.text(
                &status.time,
                Area {
                    y: top + dy,
                    height: 16.0,
                    ..area
                },
                14.0,
                600,
                false,
                ink,
            );
            self.text(
                &status.date,
                Area {
                    y: top + 17.0 + dy,
                    height: 14.0,
                    ..area
                },
                11.0,
                400,
                false,
                ink.with_alpha(ink.a * 0.85),
            );
        }
    }

    /// A glyph of Windows' icon font, centered in `area`.
    fn icon_glyph(&mut self, glyph: char, area: Area, size: f64, ink: Color) {
        let key = (size * 100.0).round() as u32;
        let format = match self.icon_formats.get(&key) {
            Some(format) => format.clone(),
            None => {
                let family = windows::core::HSTRING::from(super::strip::ICON_FONT);
                let Ok(format) = (unsafe {
                    self.write.CreateTextFormat(
                        &family,
                        None,
                        DWRITE_FONT_WEIGHT(400),
                        DWRITE_FONT_STYLE_NORMAL,
                        DWRITE_FONT_STRETCH_NORMAL,
                        size as f32,
                        w!("pt-BR"),
                    )
                }) else {
                    return;
                };
                unsafe {
                    let _ = format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                    let _ = format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                    let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                }
                self.icon_formats.insert(key, format.clone());
                format
            }
        };
        let mut buffer = [0u16; 2];
        let text = glyph.encode_utf16(&mut buffer);
        let brush = self.solid(ink).clone();
        unsafe {
            self.target.DrawText(
                text,
                &format,
                &rect(area),
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            )
        };
    }

    fn strip_cell(&mut self, scene: &Scene, cell: &StripCell) {
        let ink = scene.look.ink.unwrap_or(scene.palette.ink);
        let area = cell.area;
        if cell.hovered {
            let hover = self.solid(ink.with_alpha(0.12)).clone();
            unsafe {
                self.target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: rect(Area {
                            x: area.x + 1.0,
                            y: area.y + 1.0,
                            width: area.width - 2.0,
                            height: area.height - 2.0,
                        }),
                        radiusX: 4.0,
                        radiusY: 4.0,
                    },
                    &hover,
                )
            };
        }
        if let Some((glyph, alpha)) = cell.glyph {
            if scene.look.flat {
                // Without a bar behind it, a shadow keeps it legible.
                let shadow = Area {
                    y: area.y + 1.0,
                    ..area
                };
                self.icon_glyph(
                    glyph,
                    shadow,
                    area.width * 0.6,
                    Color::BLACK.with_alpha(0.6),
                );
            }
            self.icon_glyph(glyph, area, area.width * 0.6, ink.with_alpha(ink.a * alpha));
        }
        if let Some(image) = &cell.image {
            let side = (area.width * 0.66).round();
            let at = Area {
                x: area.x + (area.width - side) / 2.0,
                y: area.y + (area.height - side) / 2.0,
                width: side,
                height: side,
            };
            unsafe {
                self.target.DrawBitmap(
                    image,
                    Some(&rect(at)),
                    1.0,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                )
            };
        }
    }

    fn tip(&mut self, scene: &Scene, tip: &Tip) {
        let width = self.label_width(&tip.text);
        let (x, y) = tip.anchor;
        let top = match scene.edge {
            DockEdge::Bottom => y - 24.0,
            DockEdge::Top => y,
        };
        let pill = Area {
            x: x - width / 2.0,
            y: top,
            width,
            height: 24.0,
        };
        let shape = D2D1_ROUNDED_RECT {
            rect: rect(pill),
            radiusX: 12.0,
            radiusY: 12.0,
        };
        // A soft shadow under the pill.
        let shadow = self.solid(Color::BLACK.with_alpha(0.12)).clone();
        unsafe {
            self.target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rect(Area {
                        y: pill.y + 2.0,
                        ..pill
                    }),
                    radiusX: 12.0,
                    radiusY: 12.0,
                },
                &shadow,
            )
        };
        let fill = self.solid(scene.palette.tip_bg).clone();
        unsafe { self.target.FillRoundedRectangle(&shape, &fill) };
        let ring = self.solid(scene.palette.hair).clone();
        unsafe {
            self.target
                .DrawRoundedRectangle(&shape, &ring, 1.0, None::<&ID2D1StrokeStyle>)
        };
        self.text(&tip.text, pill, 12.5, 600, false, scene.palette.tip_ink);
    }
}

/// A path geometry from an SVG path.
fn path(factory: &ID2D1Factory, text: &str) -> windows::core::Result<ID2D1PathGeometry> {
    let segments = svg::parse(text).unwrap_or_default();
    let geometry = unsafe { factory.CreatePathGeometry() }?;
    let sink = unsafe { geometry.Open() }?;
    let mut open = false;
    let mut at = (0.0f32, 0.0f32);
    for segment in segments {
        unsafe {
            match segment {
                Segment::Move(x, y) => {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    }
                    sink.BeginFigure(Vector2 { X: x, Y: y }, D2D1_FIGURE_BEGIN_FILLED);
                    open = true;
                    at = (x, y);
                }
                Segment::Line(x, y) => {
                    sink.AddLine(Vector2 { X: x, Y: y });
                    at = (x, y);
                }
                Segment::Cubic([x1, y1, x2, y2, x, y]) => {
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: Vector2 { X: x1, Y: y1 },
                        point2: Vector2 { X: x2, Y: y2 },
                        point3: Vector2 { X: x, Y: y },
                    });
                    at = (x, y);
                }
                Segment::Arc {
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                    x,
                    y,
                } => {
                    // Direct2D draws nothing for an arc to where it starts.
                    if (x, y) != at {
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: Vector2 { X: x, Y: y },
                            size: D2D_SIZE_F {
                                width: rx,
                                height: ry,
                            },
                            rotationAngle: rotation,
                            sweepDirection: if sweep {
                                D2D1_SWEEP_DIRECTION_CLOCKWISE
                            } else {
                                D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                            },
                            arcSize: if large {
                                D2D1_ARC_SIZE_LARGE
                            } else {
                                D2D1_ARC_SIZE_SMALL
                            },
                        });
                    }
                    at = (x, y);
                }
                Segment::Close => {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                        open = false;
                    }
                }
            }
        }
    }
    unsafe {
        if open {
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }
        sink.Close()?;
    }
    Ok(geometry)
}
