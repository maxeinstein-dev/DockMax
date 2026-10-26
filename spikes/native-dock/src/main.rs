//! Spike (ADR 0011): a dock drawn natively, with no WebView2, to measure what
//! it costs. A layered, topmost, click-through-where-transparent window at the
//! bottom of the primary monitor, painted with Direct2D into a DIB and handed
//! to `UpdateLayeredWindow`: a translucent rounded bar, the real icons of
//! ~14 programs (from the shell), a clock (DirectWrite) and the hover
//! magnification animated at ~60 fps.
//!
//! `--animate` sweeps a virtual pointer across the bar forever, to measure the
//! CPU while animating without touching the real mouse. `--seconds N` quits
//! after N seconds.

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::time::Instant;

use windows::Win32::Foundation::{
    COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_DRAW_TEXT_OPTIONS_NONE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Bitmap, ID2D1DCRenderTarget, ID2D1Factory, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, GetMonitorInfoW, HBITMAP, HPALETTE,
    HDC, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, SelectObject,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, IWICImagingFactory, WICBitmapUsePremultipliedAlpha,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_RESIZETOFIT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
    KillTimer, MSG, PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE, SetTimer, ShowWindow,
    TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_DESTROY, WM_MOUSEMOVE,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{HSTRING, PCWSTR, w};

/// `WM_MOUSELEAVE`, which this crate's features do not name.
const WM_MOUSELEAVE: u32 = 0x02A3;
const ICON: f32 = 48.0;
const PAD: f32 = 8.0;
const GAP: f32 = 6.0;
const MAX_SCALE: f32 = 1.6;
const CLOCK_WIDTH: f32 = 90.0;
const ANIMATION_TIMER: usize = 1;
const CLOCK_TIMER: usize = 2;
const QUIT_TIMER: usize = 3;

/// Programs whose icons the bar shows: files that are on any Windows 11.
const PROGRAMS: [&str; 14] = [
    r"C:\Windows\explorer.exe",
    r"C:\Windows\System32\notepad.exe",
    r"C:\Windows\System32\cmd.exe",
    r"C:\Windows\System32\charmap.exe",
    r"C:\Windows\System32\Taskmgr.exe",
    r"C:\Windows\System32\control.exe",
    r"C:\Windows\regedit.exe",
    r"C:\Windows\System32\mmc.exe",
    r"C:\Windows\System32\magnify.exe",
    r"C:\Windows\System32\osk.exe",
    r"C:\Windows\System32\mspaint.exe",
    r"C:\Windows\System32\calc.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
];

struct Dock {
    hwnd: HWND,
    target: ID2D1DCRenderTarget,
    dc: HDC,
    _dib: HBITMAP,
    width: i32,
    height: i32,
    origin: POINT,
    icons: Vec<Option<ID2D1Bitmap>>,
    scales: Vec<f32>,
    pointer: Option<f32>,
    text: IDWriteTextFormat,
    bar: ID2D1SolidColorBrush,
    line: ID2D1SolidColorBrush,
    ink: ID2D1SolidColorBrush,
    animate: bool,
    started: Instant,
    animating: bool,
}

thread_local! {
    static DOCK: RefCell<Option<Dock>> = const { RefCell::new(None) };
}

fn color(r: f32, g: f32, b: f32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a }
}

fn icon_center(index: usize) -> f32 {
    PAD + ICON / 2.0 + index as f32 * (ICON + GAP)
}

/// How much the icon at `center` grows for a pointer at `x`: a bell around
/// the pointer, as the dock's magnification does.
fn wanted_scale(center: f32, x: Option<f32>) -> f32 {
    match x {
        None => 1.0,
        Some(x) => {
            let distance = (center - x) / (ICON * 1.4);
            1.0 + (MAX_SCALE - 1.0) * (-distance * distance).exp()
        }
    }
}

unsafe fn load_icon(
    wic: &IWICImagingFactory,
    target: &ID2D1DCRenderTarget,
    path: &str,
) -> Option<ID2D1Bitmap> {
    let item: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(&HSTRING::from(path), None) }.ok()?;
    let side = (ICON * MAX_SCALE) as i32;
    let bitmap = unsafe { item.GetImage(SIZE { cx: side, cy: side }, SIIGBF_RESIZETOFIT) }.ok()?;
    let source =
        unsafe { wic.CreateBitmapFromHBITMAP(bitmap, HPALETTE::default(), WICBitmapUsePremultipliedAlpha) }.ok();
    let _ = unsafe { DeleteObject(bitmap.into()) };
    unsafe { target.CreateBitmapFromWicBitmap(&source?, None) }.ok()
}

impl Dock {
    fn paint(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;
        let bar_top = height - PAD * 2.0 - ICON;
        unsafe {
            let rect = RECT {
                left: 0,
                top: 0,
                right: self.width,
                bottom: self.height,
            };
            if self.target.BindDC(self.dc, &rect).is_err() {
                return;
            }
            self.target.BeginDraw();
            self.target.Clear(Some(&color(0.0, 0.0, 0.0, 0.0)));
            let bar = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 0.5,
                    top: bar_top + 0.5,
                    right: width - 0.5,
                    bottom: height - 0.5,
                },
                radiusX: 16.0,
                radiusY: 16.0,
            };
            self.target.FillRoundedRectangle(&bar, &self.bar);
            self.target.DrawRoundedRectangle(&bar, &self.line, 1.0, None);
            for (index, icon) in self.icons.iter().enumerate() {
                let Some(icon) = icon else { continue };
                let size = ICON * self.scales[index];
                let center = icon_center(index);
                let bottom = height - PAD;
                let place = D2D_RECT_F {
                    left: center - size / 2.0,
                    top: bottom - size,
                    right: center + size / 2.0,
                    bottom,
                };
                self.target.DrawBitmap(
                    icon,
                    Some(&place),
                    1.0,
                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    None,
                );
            }
            let now = clock_text();
            let clock = D2D_RECT_F {
                left: width - CLOCK_WIDTH,
                top: bar_top,
                right: width - PAD,
                bottom: height,
            };
            self.target.DrawText(
                &now,
                &self.text,
                &clock,
                &self.ink,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
            let _ = self.target.EndDraw(None, None);
            let size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                self.hwnd,
                None,
                Some(&self.origin),
                Some(&size),
                Some(self.dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }

    /// One frame of the animation: each icon moves a third of the way to its
    /// wanted size. `false` once everything has settled.
    fn step(&mut self) -> bool {
        if self.animate {
            let t = self.started.elapsed().as_secs_f32();
            let span = icon_center(self.icons.len() - 1);
            self.pointer = Some(span / 2.0 + span / 2.0 * (t * 1.3).sin());
        }
        let mut moving = false;
        for index in 0..self.scales.len() {
            let wanted = wanted_scale(icon_center(index), self.pointer);
            let now = self.scales[index];
            let next = now + (wanted - now) * 0.35;
            if (next - wanted).abs() > 0.002 {
                moving = true;
                self.scales[index] = next;
            } else {
                self.scales[index] = wanted;
            }
        }
        self.paint();
        moving || self.animate
    }

    fn start_animation(&mut self) {
        if !self.animating {
            self.animating = true;
            unsafe { SetTimer(Some(self.hwnd), ANIMATION_TIMER, 16, None) };
        }
    }
}

fn clock_text() -> Vec<u16> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // UTC-3, enough for a spike.
    let local = (seconds as i64 - 3 * 3600).rem_euclid(86_400);
    format!("{:02}:{:02}", local / 3600, (local % 3600) / 60)
        .encode_utf16()
        .collect()
}

extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xffff) as i16 as f32;
            DOCK.with(|dock| {
                if let Some(dock) = dock.borrow_mut().as_mut() {
                    dock.pointer = Some(x);
                    dock.start_animation();
                }
            });
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = unsafe { TrackMouseEvent(&mut track) };
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            DOCK.with(|dock| {
                if let Some(dock) = dock.borrow_mut().as_mut() {
                    dock.pointer = None;
                    dock.start_animation();
                }
            });
            LRESULT(0)
        }
        WM_TIMER => {
            match wparam.0 {
                ANIMATION_TIMER => DOCK.with(|dock| {
                    if let Some(dock) = dock.borrow_mut().as_mut()
                        && !dock.step()
                    {
                        dock.animating = false;
                        let _ = unsafe { KillTimer(Some(hwnd), ANIMATION_TIMER) };
                    }
                }),
                CLOCK_TIMER => DOCK.with(|dock| {
                    if let Some(dock) = dock.borrow_mut().as_mut() {
                        dock.paint();
                    }
                }),
                QUIT_TIMER => unsafe { PostQuitMessage(0) },
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn main() -> windows::core::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let animate = args.iter().any(|arg| arg == "--animate");
    let seconds: Option<u32> = args
        .iter()
        .position(|arg| arg == "--seconds")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse().ok());
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let instance = GetModuleHandleW(None)?;
        let class = w!("NativeDockSpike");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let count = PROGRAMS.len();
        let width = (PAD * 2.0 + count as f32 * (ICON + GAP) + CLOCK_WIDTH) as i32;
        let height = (PAD * 2.0 + ICON * MAX_SCALE + 6.0) as i32;
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let area = info.rcMonitor;
        let origin = POINT {
            x: area.left + (area.right - area.left - width) / 2,
            y: area.bottom - height - 4,
        };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            PCWSTR(w!("Native dock spike").as_ptr()),
            WS_POPUP,
            origin.x,
            origin.y,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            None,
        )?;

        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
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
        let target = factory.CreateDCRenderTarget(&properties)?;
        let dc = CreateCompatibleDC(None);
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
        let dib = CreateDIBSection(Some(dc), &header, DIB_RGB_COLORS, &mut bits, None, 0)?;
        SelectObject(dc, dib.into());

        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let icons = PROGRAMS
            .iter()
            .map(|path| load_icon(&wic, &target, path))
            .collect::<Vec<_>>();
        drop(wic);

        let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let text = write.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            15.0,
            w!("pt-BR"),
        )?;
        text.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        text.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

        let bar = target.CreateSolidColorBrush(&color(0.10, 0.10, 0.12, 0.78), None)?;
        let line = target.CreateSolidColorBrush(&color(1.0, 1.0, 1.0, 0.14), None)?;
        let ink = target.CreateSolidColorBrush(&color(1.0, 1.0, 1.0, 0.95), None)?;

        let mut dock = Dock {
            hwnd,
            target,
            dc,
            _dib: dib,
            width,
            height,
            origin,
            scales: vec![1.0; icons.len()],
            icons,
            pointer: None,
            text,
            bar,
            line,
            ink,
            animate,
            started: Instant::now(),
            animating: false,
        };
        dock.paint();
        if animate {
            dock.start_animation();
        }
        DOCK.with(|slot| *slot.borrow_mut() = Some(dock));
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetTimer(Some(hwnd), CLOCK_TIMER, 60_000, None);
        if let Some(seconds) = seconds {
            SetTimer(Some(hwnd), QUIT_TIMER, seconds * 1000, None);
        }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}
