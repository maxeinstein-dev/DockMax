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
use ::windows::Win32::Foundation::{PROPERTYKEY, SIZE};
#[cfg(windows)]
use ::windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, DIB_RGB_COLORS, DeleteObject,
    GetDC, GetDIBits, GetObjectW, HBITMAP, HGDIOBJ, ReleaseDC,
};
#[cfg(windows)]
use ::windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
#[cfg(windows)]
use ::windows::Win32::UI::Shell::PropertiesSystem::{
    GPS_DEFAULT, IPropertyStore, SHGetPropertyStoreFromParsingName,
};
#[cfg(windows)]
use ::windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItemImageFactory,
    KF_FLAG_DEFAULT, SHCreateItemFromParsingName, SHGetKnownFolderItem, SIGDN_NORMALDISPLAY,
    SIGDN_PARENTRELATIVEPARSING, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY, ShellExecuteW,
};
#[cfg(windows)]
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, HCURSOR, ICONINFO, SW_SHOWNORMAL,
};
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
pub(super) fn init_com() {
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

/// The item of a program of the Apps folder, by the id the catalog gave.
#[cfg(windows)]
fn apps_folder_item(id: &str) -> Result<IShellItem, String> {
    if !is_valid_app_id(id) {
        return Err("the id is not one of the Apps folder".to_owned());
    }
    init_com();
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    unsafe { SHCreateItemFromParsingName(&name, None) }.map_err(|error| error.to_string())
}

/// The item of a program: the one in the Apps folder or, for a program that
/// was dropped on the dock and the Apps folder doesn't list, its file (the id
/// is then the path, see `domain::apps::is_pinnable_path`).
#[cfg(windows)]
pub(super) fn item_of(id: &str) -> Result<IShellItem, String> {
    apps_folder_item(id).or_else(|error| {
        let path = file_of(id).ok_or(error)?;
        let name = HSTRING::from(path.as_os_str());
        unsafe { SHCreateItemFromParsingName(&name, None) }.map_err(|error| error.to_string())
    })
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

/// `System.AppUserModel.ID`.
#[cfg(windows)]
const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: ::windows::core::GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};

/// The AppID the shell knows the program by (what its taskbar button, and
/// its Jump List, are filed under): the `System.AppUserModel.ID` of its item
/// in the Apps folder.
#[cfg(windows)]
pub fn app_user_model_id(id: &str) -> Option<String> {
    if !is_valid_app_id(id) {
        return None;
    }
    init_com();
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    let listed = unsafe {
        SHGetPropertyStoreFromParsingName::<_, _, IPropertyStore>(&name, None, GPS_DEFAULT)
    }
    .ok()
    .and_then(|store| unsafe { store.GetValue(&PKEY_APP_USER_MODEL_ID) }.ok())
    .map(|value| value.to_string())
    .filter(|text| !text.is_empty());
    // A program the Apps folder doesn't list has the AppID the shell gives
    // an executable that sets none: its path.
    listed.or_else(|| file_of(id).map(|path| path.to_string_lossy().into_owned()))
}

#[cfg(not(windows))]
pub fn app_user_model_id(_id: &str) -> Option<String> {
    None
}

/// `System.Link.TargetParsingPath`: the file a shortcut of the Apps folder
/// opens.
#[cfg(windows)]
const PKEY_LINK_TARGET_PARSING_PATH: PROPERTYKEY = PROPERTYKEY {
    fmtid: ::windows::core::GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25),
    pid: 2,
};

/// The program file the item `id` of the Apps folder runs, for a classic
/// program (a Store app has none). A program dropped on the dock is its own
/// file.
#[cfg(windows)]
pub fn target_of(id: &str) -> Option<std::path::PathBuf> {
    if !is_valid_app_id(id) {
        return None;
    }
    init_com();
    let name = match file_of(id) {
        // A program dropped on the dock is its own file; a shortcut dropped
        // on it runs what it points to.
        Some(file)
            if !file
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk")) =>
        {
            return Some(file);
        }
        Some(shortcut) => HSTRING::from(shortcut.as_os_str()),
        None => HSTRING::from(format!("shell:AppsFolder\\{id}")),
    };
    let target = unsafe {
        SHGetPropertyStoreFromParsingName::<_, _, IPropertyStore>(&name, None, GPS_DEFAULT)
    }
    .ok()
    .and_then(|store| unsafe { store.GetValue(&PKEY_LINK_TARGET_PARSING_PATH) }.ok())
    .map(|value| value.to_string())
    .filter(|text| !text.is_empty())?;
    Some(std::path::PathBuf::from(target))
}

#[cfg(not(windows))]
pub fn target_of(_id: &str) -> Option<std::path::PathBuf> {
    None
}

/// `System.Link.Arguments`: what a shortcut of the Apps folder adds to its
/// program's command line.
#[cfg(windows)]
const PKEY_LINK_ARGUMENTS: PROPERTYKEY = PROPERTYKEY {
    fmtid: ::windows::core::GUID::from_u128(0x436f2667_14e2_4feb_b30a_146c53b5b674),
    pid: 100,
};

/// The arguments the shortcut of the item `id` of the Apps folder gives its
/// program (a Chrome profile, say); `None` when it gives none.
#[cfg(windows)]
pub fn arguments_of(id: &str) -> Option<String> {
    if !is_valid_app_id(id) {
        return None;
    }
    init_com();
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    unsafe { SHGetPropertyStoreFromParsingName::<_, _, IPropertyStore>(&name, None, GPS_DEFAULT) }
        .ok()
        .and_then(|store| unsafe { store.GetValue(&PKEY_LINK_ARGUMENTS) }.ok())
        .map(|value| value.to_string())
        .filter(|text| !text.trim().is_empty())
}

/// The path of the program's file, when its id is that path (a classic
/// program the shell lists by its executable); `None` for a Store app, which
/// has no file to show, and for the ids that name a folder and a file.
pub fn file_of(id: &str) -> Option<std::path::PathBuf> {
    let bytes = id.as_bytes();
    let drive =
        bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\';
    let path = std::path::PathBuf::from(id);
    (is_valid_app_id(id) && drive && path.is_file()).then_some(path)
}

/// Shows `path` selected in a File Explorer window.
#[cfg(windows)]
pub fn reveal(path: &std::path::Path) -> Result<(), String> {
    let arguments = HSTRING::from(format!("/select,\"{}\"", path.display()));
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            &HSTRING::from("explorer.exe"),
            &arguments,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

#[cfg(not(windows))]
pub fn reveal(_path: &std::path::Path) -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Opens the folder `path` in File Explorer.
#[cfg(windows)]
pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(path.as_os_str()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

#[cfg(not(windows))]
pub fn open_folder(_path: &std::path::Path) -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Opens an address of the launcher's closed list (a Windows settings page,
/// a keyword's `https://` search) with the program Windows has for it.
/// Checked again here: nothing else is opened.
#[cfg(windows)]
pub fn open_uri(uri: &str) -> Result<(), String> {
    use crate::domain::launcher::{Action, is_allowed};
    if !is_allowed(&Action::OpenUri(uri.to_owned())) {
        return Err("the address is not one the launcher opens".to_owned());
    }
    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(uri),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

#[cfg(not(windows))]
pub fn open_uri(_uri: &str) -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Opens the Windows Task Manager, as the taskbar's menu does: through the
/// shell, which raises it to administrator when Windows asks for that.
#[cfg(windows)]
pub fn open_task_manager() -> Result<(), String> {
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            &HSTRING::from("taskmgr.exe"),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

#[cfg(not(windows))]
pub fn open_task_manager() -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Opens the program, the way the Start menu does.
#[cfg(windows)]
pub fn launch(id: &str) -> Result<(), String> {
    if !is_valid_app_id(id) {
        return Err("the id is not one of the Apps folder".to_owned());
    }
    init_com();
    // A program of the Apps folder opens as the Start menu opens it; a file
    // the Apps folder doesn't list (dropped on the dock) opens as itself.
    let target = match apps_folder_item(id) {
        Ok(_) => HSTRING::from(format!("shell:AppsFolder\\{id}")),
        Err(error) => HSTRING::from(file_of(id).ok_or(error)?.as_os_str()),
    };
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

/// A cursor of the program's icon, `size` pixels a side, its point in the
/// middle: what a tile dragged out of the dock shows (spec 0018). The
/// caller destroys it.
#[cfg(windows)]
pub fn drag_cursor(id: &str, size: i32) -> Option<HCURSOR> {
    let item = item_of(id).ok()?;
    let factory: IShellItemImageFactory = windows_core_cast(&item)?;
    let bitmap = unsafe { factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY) }.ok()?;
    let mask = unsafe { CreateBitmap(size, size, 1, 1, None) };
    let info = ICONINFO {
        fIcon: false.into(),
        xHotspot: (size / 2) as u32,
        yHotspot: (size / 2) as u32,
        hbmMask: mask,
        hbmColor: bitmap,
    };
    let icon = unsafe { CreateIconIndirect(&info) }.ok();
    unsafe {
        let _ = DeleteObject(HGDIOBJ(mask.0));
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
    }
    icon.map(|icon| HCURSOR(icon.0))
}

/// The program's icon as a PNG of `size` pixels a side, or `None` when
/// Windows has none.
#[cfg(windows)]
pub fn icon_png(id: &str, size: u32) -> Option<Vec<u8>> {
    item_icon_png(&item_of(id).ok()?, size)
}

/// The icon the shell shows for the file or folder `path` (a launcher's
/// result, L4), as a PNG of `size` pixels a side. Only in the launcher's
/// helper process: the shell stays out of the app's.
#[cfg(windows)]
pub fn path_icon_png(path: &str, size: u32) -> Option<Vec<u8>> {
    if !crate::domain::launcher_files::is_valid_path(path) {
        return None;
    }
    init_com();
    let item: IShellItem =
        unsafe { SHCreateItemFromParsingName(&HSTRING::from(path), None) }.ok()?;
    item_icon_png(&item, size)
}

/// Opens a launcher's file with the program Windows has for it, or its
/// folder in File Explorer (L4). Checked again here: nothing but a full path
/// on a drive, or on a share typed in the launcher (L5), is opened.
#[cfg(windows)]
pub fn open_path(path: &str) -> Result<(), String> {
    use crate::domain::launcher::{Action, is_allowed};
    if !is_allowed(&Action::OpenFile(path.to_owned())) {
        return Err("the path is not one the launcher opens".to_owned());
    }
    init_com();
    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(path),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

#[cfg(not(windows))]
pub fn open_path(_path: &str) -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Why something was not opened as administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elevation {
    /// The user said no to Windows' question (UAC): not an error.
    Cancelled,
    Failed(String),
}

/// Runs `file` (with `parameters`) as administrator, as Windows' Run does
/// with Ctrl+Shift+Enter: the shell's `runas` verb, which asks the user
/// (UAC). `parent` is the window the question belongs to. Blocks until
/// Windows answers: off the main thread.
#[cfg(windows)]
fn run_as_admin(file: &str, parameters: Option<&str>, parent: isize) -> Result<(), Elevation> {
    use ::windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
    use ::windows::Win32::UI::Shell::{
        SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    init_com();
    let verb = HSTRING::from("runas");
    let file = HSTRING::from(file);
    let parameters = parameters.map(HSTRING::from);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        hwnd: HWND(parent as _),
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: parameters
            .as_ref()
            .map_or(PCWSTR::null(), |parameters| PCWSTR(parameters.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    match unsafe { ShellExecuteExW(&mut info) } {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERROR_CANCELLED.to_hresult() => Err(Elevation::Cancelled),
        Err(error) => Err(Elevation::Failed(error.to_string())),
    }
}

/// Opens the program `id` of the Apps folder as administrator, as the Start
/// menu's "Executar como administrador" does; a classic program the Apps
/// folder can't raise is run from its file, with its shortcut's arguments.
#[cfg(windows)]
pub fn open_app_as_admin(id: &str, parent: isize) -> Result<(), Elevation> {
    use crate::domain::launcher::{Action, is_allowed};
    if !is_allowed(&Action::OpenAppAsAdmin(id.to_owned())) {
        return Err(Elevation::Failed("not a program of the Apps folder".into()));
    }
    match run_as_admin(&format!("shell:AppsFolder\\{id}"), None, parent) {
        Err(Elevation::Failed(error)) => {
            let Some(file) = target_of(id).filter(|file| file.is_file()) else {
                return Err(Elevation::Failed(error));
            };
            let arguments = arguments_of(id);
            run_as_admin(&file.to_string_lossy(), arguments.as_deref(), parent)
        }
        other => other,
    }
}

#[cfg(not(windows))]
pub fn open_app_as_admin(_id: &str, _parent: isize) -> Result<(), Elevation> {
    Err(Elevation::Failed("not Windows".to_owned()))
}

/// Runs a launcher's program, script or shortcut (`path`) as administrator.
/// Checked again here: only a path the launcher opens, of a kind Windows
/// raises.
#[cfg(windows)]
pub fn open_path_as_admin(path: &str, parent: isize) -> Result<(), Elevation> {
    use crate::domain::launcher::{Action, is_allowed};
    if !is_allowed(&Action::OpenFileAsAdmin(path.to_owned())) {
        return Err(Elevation::Failed("not a program the launcher runs".into()));
    }
    run_as_admin(path, None, parent)
}

#[cfg(not(windows))]
pub fn open_path_as_admin(_path: &str, _parent: isize) -> Result<(), Elevation> {
    Err(Elevation::Failed("not Windows".to_owned()))
}

/// Shows a launcher's file or folder (`path`) selected in File Explorer.
/// Checked again here, as `open_path`.
pub fn reveal_path(path: &str) -> Result<(), String> {
    use crate::domain::launcher::{Action, is_allowed};
    if !is_allowed(&Action::Reveal(path.to_owned())) {
        return Err("the path is not one the launcher shows".to_owned());
    }
    // A folder without its `\`, but a drive with it (`C:\`).
    let trimmed = path.trim_end_matches('\\');
    let shown = if trimmed.len() <= 2 { path } else { trimmed };
    reveal(std::path::Path::new(shown))
}

#[cfg(windows)]
fn item_icon_png(item: &IShellItem, size: u32) -> Option<Vec<u8>> {
    let factory: IShellItemImageFactory = windows_core_cast(item)?;
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

pub(super) fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
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
    fn only_an_id_that_is_a_path_to_a_file_has_a_file_to_show() {
        let exe = std::env::current_exe().unwrap();
        let id = exe.to_string_lossy().into_owned();

        assert_eq!(file_of(&id), Some(exe));
        assert_eq!(
            file_of("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            None
        );
        assert_eq!(
            file_of(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\notepad.exe"),
            None
        );
        assert_eq!(file_of(r"C:\no\such\file.exe"), None);
        assert_eq!(file_of(r"C:\"), None);
        assert_eq!(file_of(""), None);
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
