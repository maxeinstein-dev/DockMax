//! The monitors the dock goes on (spec 0010 D4b: one dock per monitor).

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::core::BOOL;

use super::dock::Rect;

/// `MONITORINFOF_PRIMARY`.
const PRIMARY: u32 = 1;

/// A monitor as the dock sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// What names it across changes of the layout: the device's name
    /// (`\\.\DISPLAY1`).
    pub key: String,
    /// Its handle, as Windows gives it now (an `HMONITOR` as a number).
    pub handle: isize,
    /// Its rectangle and its work area, in physical pixels.
    pub rect: Rect,
    pub work: Rect,
    /// Its scale: 1 at 96 DPI.
    pub scale: f64,
    pub primary: bool,
}

fn rect(r: RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        width: u32::try_from(r.right - r.left).unwrap_or(0),
        height: u32::try_from(r.bottom - r.top).unwrap_or(0),
    }
}

/// The monitor `handle`; `None` when it can't be read.
fn describe(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let read =
        unsafe { GetMonitorInfoW(handle, std::ptr::from_mut(&mut info).cast::<MONITORINFO>()) };
    if !read.as_bool() {
        return None;
    }
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    let _ = unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let length = info
        .szDevice
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(info.szDevice.len());
    Some(Monitor {
        key: String::from_utf16_lossy(&info.szDevice[..length]),
        handle: handle.0 as isize,
        rect: rect(info.monitorInfo.rcMonitor),
        work: rect(info.monitorInfo.rcWork),
        scale: f64::from(dpi_x.max(1)) / 96.0,
        primary: info.monitorInfo.dwFlags & PRIMARY != 0,
    })
}

/// Every monitor, the primary first.
pub fn all() -> Vec<Monitor> {
    unsafe extern "system" fn collect(
        handle: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        state: LPARAM,
    ) -> BOOL {
        let found = unsafe { &mut *(state.0 as *mut Vec<HMONITOR>) };
        found.push(handle);
        true.into()
    }
    let mut handles: Vec<HMONITOR> = Vec::new();
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(std::ptr::from_mut(&mut handles) as isize),
        )
    };
    let mut monitors: Vec<Monitor> = handles.into_iter().filter_map(describe).collect();
    monitors.sort_by_key(|monitor| !monitor.primary);
    monitors
}
