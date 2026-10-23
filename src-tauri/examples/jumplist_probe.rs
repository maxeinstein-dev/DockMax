//! Spike of the Jump Lists (spec 0006 §3 B3): for each program of the Apps
//! folder whose name has the given text, which AppID the shell knows it by
//! and what the documented API gives for it: the recent and the frequent
//! destinations. Read only: nothing is opened and no list is changed.
//!
//! `cargo run --example jumplist_probe -- [name part]`

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::IObjectArray;
use windows::Win32::UI::Shell::PropertiesSystem::{
    GPS_DEFAULT, IPropertyStore, SHGetPropertyStoreFromParsingName,
};
use windows::Win32::UI::Shell::{
    ADLT_FREQUENT, ADLT_RECENT, APPDOCLISTTYPE, ApplicationDocumentLists, BHID_EnumItems,
    FOLDERID_AppsFolder, IApplicationDocumentLists, IEnumShellItems, IShellItem, KF_FLAG_DEFAULT,
    SHGetKnownFolderItem, SIGDN_FILESYSPATH, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};
use windows::core::{GUID, HSTRING, PWSTR};

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

fn app_id_of(id: &str) -> Option<String> {
    let name = HSTRING::from(format!("shell:AppsFolder\\{id}"));
    let store: IPropertyStore =
        unsafe { SHGetPropertyStoreFromParsingName(&name, None, GPS_DEFAULT) }.ok()?;
    let value = unsafe { store.GetValue(&PKEY_APP_ID) }.ok()?;
    let text = value.to_string();
    (!text.is_empty()).then_some(text)
}

fn destinations(app_id: &str, kind: APPDOCLISTTYPE) -> Result<Vec<String>, String> {
    let lists: IApplicationDocumentLists = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &ApplicationDocumentLists,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
    }
    .map_err(|e| e.to_string())?;
    unsafe { lists.SetAppID(&HSTRING::from(app_id)) }.map_err(|e| e.to_string())?;
    let array: IObjectArray = unsafe { lists.GetList(kind, 20) }.map_err(|e| e.to_string())?;
    let count = unsafe { array.GetCount() }.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for index in 0..count {
        let item: Result<IShellItem, _> = unsafe { array.GetAt(index) };
        match item {
            Ok(item) => {
                let name = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
                    .map(text)
                    .unwrap_or_default();
                let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
                    .map(text)
                    .unwrap_or_default();
                out.push(format!("{name}  [{path}]"));
            }
            Err(_) => {
                let unknown: Result<windows::core::IUnknown, _> = unsafe { array.GetAt(index) };
                out.push(format!("(not a shell item: {})", unknown.is_ok()));
            }
        }
    }
    Ok(out)
}

fn main() {
    let wanted = std::env::args().nth(1).map(|t| t.to_lowercase());
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
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
            let property = app_id_of(&id);
            println!("== {name}\n   id: {id}\n   AppUserModel.ID: {property:?}");
            let candidates: Vec<String> = property
                .iter()
                .cloned()
                .chain(std::iter::once(id.clone()))
                .collect();
            for candidate in candidates {
                for (label, kind) in [("recent", ADLT_RECENT), ("frequent", ADLT_FREQUENT)] {
                    match destinations(&candidate, kind) {
                        Ok(list) => println!(
                            "   {label} with {candidate:?}: {} item(s) {:?}",
                            list.len(),
                            list.iter().take(4).collect::<Vec<_>>()
                        ),
                        Err(e) => println!("   {label} with {candidate:?}: error {e}"),
                    }
                }
            }
        }
    }
}
