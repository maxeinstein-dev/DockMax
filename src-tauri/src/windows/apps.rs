//! The programs installed on this PC, as Windows lists them: the Apps folder
//! of the shell (`shell:AppsFolder`), the one the Start menu's "All apps"
//! shows. Classic programs and Store apps are in it the same way, each with a
//! name, an icon and a *parsing name* that opens it.
//!
//! Touches nothing of Tauri: the commands call these functions from a worker
//! thread, because listing takes a while and an icon is decoded from disk.
//! The icon's pixels and the PNG are the only parts that also build off
//! Windows, so they are tested anywhere.

use std::collections::HashSet;

use crate::domain::apps::is_valid_app_id;

#[cfg(windows)]
use ::windows::Win32::Foundation::SIZE;
#[cfg(windows)]
use ::windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, HBITMAP, HGDIOBJ, ReleaseDC,
};
#[cfg(windows)]
use ::windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
#[cfg(windows)]
use ::windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItemImageFactory,
    KF_FLAG_DEFAULT, SHCreateItemFromParsingName, SHGetKnownFolderItem, SIGDN_NORMALDISPLAY,
    SIGDN_PARENTRELATIVEPARSING, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY, ShellExecuteW,
};
#[cfg(windows)]
use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
#[cfg(windows)]
use ::windows::core::{HSTRING, PCWSTR, PWSTR};

/// One program of the Apps folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogApp {
    /// The parsing name: what opens it (`shell:AppsFolder\{id}`) and what a
    /// pinned app remembers.
    pub id: String,
    /// The name the Start menu shows.
    pub name: String,
}

/// The list as the Start menu orders it: by name, without regard to case,
/// and without the same program twice (the Apps folder repeats some, such as
/// a program installed for the user and for everyone).
fn tidy(mut apps: Vec<CatalogApp>) -> Vec<CatalogApp> {
    apps.retain(|app| is_valid_app_id(&app.id) && !app.name.trim().is_empty());
    let mut seen = HashSet::new();
    apps.retain(|app| seen.insert(app.id.to_lowercase()));
    apps.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.to_lowercase()));
    apps
}

#[cfg(windows)]
fn init_com() {
    // Already initialized in another mode (the main thread's): fine either
    // way, the call only has to have been made on this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

#[cfg(windows)]
fn take_text(text: PWSTR) -> String {
    let value = unsafe { text.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(text.0 as *const _)) };
    value
}

/// Every program of the Apps folder, by name.
#[cfg(windows)]
pub fn catalog() -> Result<Vec<CatalogApp>, String> {
    init_com();
    let folder: IShellItem =
        unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) }
            .map_err(|error| error.to_string())?;
    let items: IEnumShellItems = unsafe { folder.BindToHandler(None, &BHID_EnumItems) }
        .map_err(|error| error.to_string())?;
    let mut apps = Vec::new();
    loop {
        let mut batch: [Option<IShellItem>; 32] = Default::default();
        let mut fetched = 0u32;
        unsafe { items.Next(&mut batch, Some(&mut fetched)) }.map_err(|error| error.to_string())?;
        if fetched == 0 {
            break;
        }
        for item in batch.iter().take(fetched as usize).flatten() {
            let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }.map(take_text);
            let id = unsafe { item.GetDisplayName(SIGDN_PARENTRELATIVEPARSING) }.map(take_text);
            if let (Ok(name), Ok(id)) = (name, id) {
                apps.push(CatalogApp { id, name });
            }
        }
    }
    Ok(tidy(apps))
}

/// The item of one program, by the id the catalog gave.
#[cfg(windows)]
fn item_of(id: &str) -> Result<IShellItem, String> {
    if !is_valid_app_id(id) {
        return Err("the id is not one of the Apps folder".to_owned());
    }
    init_com();
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    unsafe { SHCreateItemFromParsingName(&name, None) }.map_err(|error| error.to_string())
}

/// The name the shell shows for the program, or `None` when the id is not
/// one of the Apps folder.
#[cfg(windows)]
pub fn name_of(id: &str) -> Option<String> {
    let item = item_of(id).ok()?;
    let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
        .map(take_text)
        .ok()?;
    (!name.trim().is_empty()).then_some(name)
}

/// Opens the program, the way the Start menu does.
#[cfg(windows)]
pub fn launch(id: &str) -> Result<(), String> {
    if !is_valid_app_id(id) {
        return Err("the id is not one of the Apps folder".to_owned());
    }
    init_com();
    let target = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            &target,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute answers a number above 32 when it worked.
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

/// The program's icon as a PNG of `size` pixels a side, or `None` when
/// Windows has none.
#[cfg(windows)]
pub fn icon_png(id: &str, size: u32) -> Option<Vec<u8>> {
    let item = item_of(id).ok()?;
    let factory: IShellItemImageFactory = windows_core_cast(&item)?;
    let side = size.clamp(16, 256) as i32;
    let bitmap = unsafe {
        factory.GetImage(
            SIZE { cx: side, cy: side },
            SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
        )
    }
    .ok()?;
    let rgba = bitmap_rgba(bitmap);
    let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
    let (width, height, pixels) = rgba?;
    encode_png(width, height, &pixels)
}

#[cfg(windows)]
fn windows_core_cast<T: ::windows::core::Interface>(item: &IShellItem) -> Option<T> {
    ::windows::core::Interface::cast::<T>(item).ok()
}

/// The pixels of a 32-bit bitmap as straight RGBA, with its size. The shell
/// hands icons as premultiplied alpha: undone here.
#[cfg(windows)]
fn bitmap_rgba(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0),
            size_of::<BITMAP>() as i32,
            Some(&mut info as *mut _ as *mut _),
        )
    };
    if got == 0
        || info.bmWidth <= 0
        || info.bmHeight <= 0
        || info.bmWidth > 512
        || info.bmHeight > 512
    {
        return None;
    }
    let (width, height) = (info.bmWidth as u32, info.bmHeight as u32);
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let screen = unsafe { GetDC(None) };
    let lines = unsafe {
        GetDIBits(
            screen,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    unsafe { ReleaseDC(None, screen) };
    if lines == 0 {
        return None;
    }
    Some((width, height, unpremultiply(&pixels)))
}

/// Premultiplied BGRA to straight RGBA.
fn unpremultiply(bgra: &[u8]) -> Vec<u8> {
    bgra.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[b, g, r, a]| {
            if a == 0 || a == 255 {
                [r, g, b, a]
            } else {
                let scale = |channel: u8| (u32::from(channel) * 255 / u32::from(a)).min(255) as u8;
                [scale(r), scale(g), scale(b), a]
            }
        })
        .collect()
}

/// Windows alone has the Apps folder: elsewhere there are no programs.
#[cfg(not(windows))]
pub fn catalog() -> Result<Vec<CatalogApp>, String> {
    Ok(Vec::new())
}

#[cfg(not(windows))]
pub fn name_of(_id: &str) -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn launch(_id: &str) -> Result<(), String> {
    Err("not Windows".to_owned())
}

#[cfg(not(windows))]
pub fn icon_png(_id: &str, _size: u32) -> Option<Vec<u8>> {
    None
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str) -> CatalogApp {
        CatalogApp {
            id: id.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn the_catalog_is_by_name_without_regard_to_case() {
        let apps = tidy(vec![app("c", "zoom"), app("a", "Word"), app("b", "excel")]);

        let names: Vec<_> = apps.iter().map(|app| app.name.as_str()).collect();
        assert_eq!(names, ["excel", "Word", "zoom"]);
    }

    #[test]
    fn the_same_program_is_listed_once() {
        let apps = tidy(vec![app("X\\a.exe", "A"), app("x\\A.EXE", "A")]);

        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn an_entry_without_a_name_or_with_a_broken_id_is_left_out() {
        let apps = tidy(vec![
            app("ok", "Ok"),
            app("nameless", "  "),
            app("", "No id"),
        ]);

        assert_eq!(apps, vec![app("ok", "Ok")]);
    }

    #[test]
    fn a_premultiplied_pixel_is_made_straight_again() {
        // Half transparent pure red, premultiplied: (0, 0, 128, 128) in BGRA.
        let rgba = unpremultiply(&[0, 0, 128, 128, 1, 2, 3, 255, 9, 9, 9, 0]);

        assert_eq!(&rgba[0..4], &[255, 0, 0, 128]);
        assert_eq!(&rgba[4..8], &[3, 2, 1, 255], "opaque stays as it is");
        assert_eq!(&rgba[8..12], &[9, 9, 9, 0], "transparent stays as it is");
    }

    #[test]
    fn the_png_has_the_signature_and_the_size() {
        let png = encode_png(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).expect("a png");

        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
