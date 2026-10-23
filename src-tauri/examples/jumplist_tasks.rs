//! Spike of the Jump Lists, second part (spec 0006, B3): the tasks and the
//! categories an app put in its list (`CustomDestinations`). For each program
//! of the Apps folder whose name has the given text: its AppID, the file the
//! shell keeps its list in (the name is a hash of the AppID), and every link
//! in it with the title the menu would show, the target and the arguments.
//! Read only: the links are read from copies in the temp folder; nothing is
//! run.
//!
//! `cargo run --example jumplist_tasks -- [name part]`

use std::path::PathBuf;

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    GPS_DEFAULT, IPropertyStore, SHGetPropertyStoreFromParsingName,
};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellLinkW, KF_FLAG_DEFAULT,
    SHGetKnownFolderItem, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING, ShellLink,
};
use windows::core::{GUID, HSTRING, Interface, PWSTR};

/// The shell's hash of an AppID: CRC-64 (polynomial 0x259C84CBA6426349,
/// reflected, starting at all ones, no final xor) of the upper-cased text as
/// UTF-16, written as 16 lower-case hexadecimal digits.
fn app_hash(app_id: &str) -> String {
    const POLY: u64 = 0x259C_84CB_A642_6349;
    let reflected = POLY.reverse_bits();
    let table: Vec<u64> = (0..256u64)
        .map(|i| {
            (0..8).fold(i, |c, _| {
                if c & 1 == 1 {
                    (c >> 1) ^ reflected
                } else {
                    c >> 1
                }
            })
        })
        .collect();
    let mut crc = u64::MAX;
    for unit in app_id.to_uppercase().encode_utf16() {
        for byte in unit.to_le_bytes() {
            crc = table[((crc ^ u64::from(byte)) & 0xFF) as usize] ^ (crc >> 8);
        }
    }
    format!("{crc:016x}")
}

fn text(value: PWSTR) -> String {
    let out = unsafe { value.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(value.0 as *const _)) };
    out
}

/// `System.AppUserModel.ID`.
const PKEY_APP_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};
/// `System.Title`.
const PKEY_TITLE: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xf29f85e0_4ff9_1068_ab91_08002b27b3d9),
    pid: 2,
};

fn app_id_of(id: &str) -> Option<String> {
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    let store: IPropertyStore =
        unsafe { SHGetPropertyStoreFromParsingName(&name, None, GPS_DEFAULT) }.ok()?;
    let value = unsafe { store.GetValue(&PKEY_APP_ID) }.ok()?;
    let text = value.to_string();
    (!text.is_empty()).then_some(text)
}

const LINK_HEADER: [u8; 20] = [
    0x4c, 0, 0, 0, 0x01, 0x14, 0x02, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46,
];

/// Where each link of the file starts.
fn link_starts(data: &[u8]) -> Vec<usize> {
    (0..data.len().saturating_sub(LINK_HEADER.len()))
        .filter(|&at| data[at..].starts_with(&LINK_HEADER))
        .collect()
}

fn describe(blob: &[u8], scratch: &PathBuf) -> String {
    std::fs::write(scratch, blob).unwrap();
    let link: IShellLinkW = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &ShellLink,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
    }
    .unwrap();
    let file: IPersistFile = link.cast().unwrap();
    let path = HSTRING::from(scratch.as_os_str());
    if let Err(error) = unsafe { file.Load(&path, STGM_READ) } {
        return format!("(could not load: {error})");
    }
    let title = link
        .cast::<IPropertyStore>()
        .ok()
        .and_then(|store| unsafe { store.GetValue(&PKEY_TITLE) }.ok())
        .map(|value| value.to_string())
        .unwrap_or_default();
    let mut target = [0u16; 512];
    let mut find = WIN32_FIND_DATAW::default();
    let _ = unsafe { link.GetPath(&mut target, &mut find, 0) };
    let mut args = [0u16; 512];
    let _ = unsafe { link.GetArguments(&mut args) };
    let own_id = link
        .cast::<IPropertyStore>()
        .ok()
        .and_then(|store| unsafe { store.GetValue(&PKEY_APP_ID) }.ok())
        .map(|value| value.to_string())
        .unwrap_or_default();
    let cut = |buffer: &[u16]| {
        String::from_utf16_lossy(
            &buffer[..buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len())],
        )
    };
    format!(
        "title {title:?} | target {:?} | args {:?} | appid {own_id:?}",
        cut(&target),
        cut(&args)
    )
}

fn main() {
    let wanted = std::env::args().nth(1).map(|t| t.to_lowercase());
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let recent = PathBuf::from(std::env::var("APPDATA").unwrap())
        .join("Microsoft/Windows/Recent/CustomDestinations");
    let scratch = std::env::temp_dir().join("jumplist-probe.lnk");
    let folder: IShellItem =
        unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) }.unwrap();
    let items: IEnumShellItems = unsafe { folder.BindToHandler(None, &BHID_EnumItems) }.unwrap();
    loop {
        let mut batch: [Option<IShellItem>; 16] = Default::default();
        let mut fetched = 0u32;
        unsafe { items.Next(&mut batch, Some(&mut fetched)) }.unwrap();
        if fetched == 0 {
            break;
        }
        for item in batch.iter().take(fetched as usize).flatten() {
            let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
                .map(text)
                .unwrap_or_default();
            let id = unsafe { item.GetDisplayName(SIGDN_PARENTRELATIVEPARSING) }
                .map(text)
                .unwrap_or_default();
            if wanted
                .as_ref()
                .is_some_and(|w| !name.to_lowercase().contains(w))
            {
                continue;
            }
            let Some(app_id) = app_id_of(&id) else {
                continue;
            };
            let hash = app_hash(&app_id);
            let file = recent.join(format!("{hash}.customDestinations-ms"));
            let Ok(data) = std::fs::read(&file) else {
                println!("== {name}: no custom list ({hash})");
                continue;
            };
            let starts = link_starts(&data);
            println!(
                "== {name}  appid {app_id:?}  file {hash}  {} bytes, {} link(s)",
                data.len(),
                starts.len()
            );
            for (n, &start) in starts.iter().enumerate() {
                let end = starts.get(n + 1).copied().unwrap_or(data.len());
                println!("   {n}: {}", describe(&data[start..end], &scratch));
            }
        }
    }
}
