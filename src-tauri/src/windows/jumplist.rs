//! The menu a program has on the Windows taskbar, its Jump List (spec 0006
//! §6): the tasks and categories the program put in its list, and the recent
//! and frequent destinations Windows keeps for it.
//!
//! - **Recent and frequent** come from the documented API
//!   `IApplicationDocumentLists`, asked with the program's AppID.
//! - **Tasks and categories** are read from the program's
//!   `CustomDestinations` file, whose name is a hash of the AppID
//!   ([`app_hash`]). The file is a sequence of shell links between category
//!   headers; each link is read with `IShellLinkW` and run by opening a copy
//!   of it with `ShellExecuteW`, as the shell does, so the rules of
//!   activation (Store apps, arguments) stay the shell's.
//!
//! Read only: no list is ever changed (`BeginList` on another program's
//! AppID would rewrite its list). The lists hold recent documents and
//! addresses, so nothing here logs a title, a path or an address.
//!
//! Touches nothing of Tauri: the commands call these functions from a worker
//! thread.

use serde::Serialize;

#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use ::windows::Win32::Foundation::PROPERTYKEY;
#[cfg(windows)]
use ::windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
#[cfg(windows)]
use ::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile, STGM_READ,
};
#[cfg(windows)]
use ::windows::Win32::UI::Shell::Common::IObjectArray;
#[cfg(windows)]
use ::windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
#[cfg(windows)]
use ::windows::Win32::UI::Shell::{
    ADLT_FREQUENT, ADLT_RECENT, APPDOCLISTTYPE, ApplicationDocumentLists,
    IApplicationDocumentLists, IShellItem, IShellLinkW, SHLoadIndirectString, SIGDN_FILESYSPATH,
    SIGDN_NORMALDISPLAY, ShellExecuteW, ShellLink,
};
#[cfg(windows)]
use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
#[cfg(windows)]
use ::windows::core::{GUID, HSTRING, Interface, PCWSTR, PWSTR};

/// The most entries of a list the menu shows: the number Windows shows by
/// default for recent items.
pub const MAX_LIST_ENTRIES: usize = 10;

/// The most tasks the menu shows.
pub const MAX_TASKS: usize = 15;

/// What a section of the menu is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionKind {
    /// What the program offers to do: "Nova janela".
    Tasks,
    /// The program's own category, with its own name.
    Category,
    Recent,
    Frequent,
}

/// What choosing an entry runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runnable {
    /// A shell link, as stored in the list: opened from a copy of it.
    Link(Vec<u8>),
    /// A file, a folder or an address: opened by the shell.
    Open(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub run: Runnable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: SectionKind,
    /// The name of a [`SectionKind::Category`].
    pub title: Option<String>,
    pub entries: Vec<Entry>,
}

/// The shell's hash of an AppID: CRC-64 (polynomial `0x259C84CBA6426349`,
/// reflected, starting at all ones, no final xor) of the upper-cased text as
/// UTF-16 little endian, as 16 lower-case hexadecimal digits. It names the
/// list's files.
pub fn app_hash(app_id: &str) -> String {
    const POLY: u64 = 0x259C_84CB_A642_6349;
    let reflected = POLY.reverse_bits();
    let table: Vec<u64> = (0..256u64)
        .map(|i| {
            (0..8).fold(i, |crc, _| {
                if crc & 1 == 1 {
                    (crc >> 1) ^ reflected
                } else {
                    crc >> 1
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

/// What a category of the `CustomDestinations` file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryKind {
    /// The program's own category, named.
    Custom(String),
    /// The program's tasks.
    Tasks,
    /// A category the shell fills itself (recent, frequent): no links here.
    Known,
}

/// One category of the file, with the shell links in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCategory {
    pub kind: CategoryKind,
    pub links: Vec<Vec<u8>>,
}

/// Closes every category.
const FOOTER: [u8; 4] = [0xAB, 0xFB, 0xBF, 0xBA];

/// The class a shell link starts with, after its 4-byte size: it begins
/// every link, and the file puts it once more before each entry.
const LINK_CLASS: [u8; 16] = [
    0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// The file's version this reader knows.
const FILE_VERSION: u32 = 2;

const KIND_CUSTOM: u32 = 0;
const KIND_TASKS: u32 = 2;

/// The most categories and entries read from one file: a file with more is
/// not one a program wrote.
const MAX_CATEGORIES: u32 = 64;
const MAX_LINKS: usize = 512;

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u32(&mut self) -> Option<u32> {
        let bytes = self.data.get(self.at..self.at + 4)?;
        self.at += 4;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.data.get(self.at..self.at + 2)?;
        self.at += 2;
        Some(u16::from_le_bytes(bytes.try_into().ok()?))
    }

    /// The text of `chars` UTF-16 units.
    fn text(&mut self, chars: usize) -> Option<String> {
        let bytes = self.data.get(self.at..self.at + chars * 2)?;
        self.at += chars * 2;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        Some(String::from_utf16_lossy(&units))
    }

    /// The position of the next footer, from here.
    fn footer(&self) -> Option<usize> {
        self.data
            .get(self.at..)?
            .windows(FOOTER.len())
            .position(|window| window == FOOTER)
            .map(|offset| self.at + offset)
    }
}

/// Where each entry's link starts inside `region`: the entry is the link's
/// class, the link's size (`0x4C`, the header) and the class again.
fn link_starts(region: &[u8]) -> Vec<usize> {
    let mut pattern = Vec::with_capacity(LINK_CLASS.len() * 2 + 4);
    pattern.extend_from_slice(&LINK_CLASS);
    pattern.extend_from_slice(&[0x4C, 0, 0, 0]);
    pattern.extend_from_slice(&LINK_CLASS);
    region
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| *window == pattern.as_slice())
        .map(|(at, _)| at + LINK_CLASS.len())
        .collect()
}

/// The categories of a `CustomDestinations` file, in order. Tolerant: a file
/// of another version, or one that stops making sense, gives the categories
/// read so far.
pub fn parse_custom(data: &[u8]) -> Vec<CustomCategory> {
    let mut found = Vec::new();
    let mut reader = Reader { data, at: 0 };
    let (Some(version), Some(categories), Some(_)) = (reader.u32(), reader.u32(), reader.u32())
    else {
        return found;
    };
    if version != FILE_VERSION || categories > MAX_CATEGORIES {
        return found;
    }
    for _ in 0..categories {
        let Some(kind) = reader.u32() else { break };
        let kind = match kind {
            KIND_CUSTOM => {
                let Some(length) = reader.u16() else { break };
                let Some(name) = reader.text(usize::from(length)) else {
                    break;
                };
                CategoryKind::Custom(name)
            }
            KIND_TASKS => CategoryKind::Tasks,
            _ => CategoryKind::Known,
        };
        // How many entries the header says is not trusted: the entries are
        // what lies before the footer.
        if reader.u32().is_none() {
            break;
        }
        let Some(footer) = reader.footer() else { break };
        let region = &data[reader.at..footer];
        let starts = link_starts(region);
        let links = starts
            .iter()
            .enumerate()
            .take(MAX_LINKS)
            .map(|(n, &start)| {
                // The next entry starts with the class, 16 bytes before its
                // link.
                let end = starts
                    .get(n + 1)
                    .map_or(region.len(), |next| next - LINK_CLASS.len());
                region[start..end].to_vec()
            })
            .collect();
        found.push(CustomCategory { kind, links });
        reader.at = footer + FOOTER.len();
    }
    found
}

/// The menu's sections for what the file holds: the program's categories
/// first, then the tasks; the known categories are the API's.
pub fn custom_sections(
    categories: Vec<CustomCategory>,
    describe: impl Fn(&[u8]) -> Option<String>,
) -> Vec<Section> {
    let mut named = Vec::new();
    let mut tasks = Vec::new();
    for category in categories {
        let limit = match category.kind {
            CategoryKind::Tasks => MAX_TASKS,
            CategoryKind::Custom(_) => MAX_LIST_ENTRIES,
            CategoryKind::Known => continue,
        };
        let entries: Vec<Entry> = category
            .links
            .into_iter()
            .filter_map(|link| {
                describe(&link).map(|title| Entry {
                    title,
                    run: Runnable::Link(link),
                })
            })
            .take(limit)
            .collect();
        if entries.is_empty() {
            continue;
        }
        match category.kind {
            CategoryKind::Tasks => tasks.push(Section {
                kind: SectionKind::Tasks,
                title: None,
                entries,
            }),
            CategoryKind::Custom(name) => named.push(Section {
                kind: SectionKind::Category,
                title: Some(name).filter(|name| !name.trim().is_empty()),
                entries,
            }),
            CategoryKind::Known => {}
        }
    }
    named.extend(tasks);
    named
}

#[cfg(windows)]
fn take_text(text: PWSTR) -> String {
    let value = unsafe { text.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(text.0 as *const _)) };
    value
}

/// `System.Title`.
#[cfg(windows)]
const PKEY_TITLE: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xf29f85e0_4ff9_1068_ab91_08002b27b3d9),
    pid: 2,
};

/// A string a program wrote as a resource (`@{package?ms-resource:///...}`)
/// as the text it stands for; any other string as it is.
#[cfg(windows)]
fn resolved(text: &str) -> String {
    if !text.starts_with('@') {
        return text.to_owned();
    }
    let mut buffer = [0u16; 256];
    match unsafe { SHLoadIndirectString(&HSTRING::from(text), &mut buffer, None) } {
        Ok(()) => from_buffer(&buffer),
        Err(_) => String::new(),
    }
}

#[cfg(windows)]
fn from_buffer(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// A scratch file name in the temp folder, apart from every other.
#[cfg(windows)]
fn scratch_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "dockmax-jumplist-{}.lnk",
        uuid::Uuid::now_v7().simple()
    ))
}

/// How long a copy of a link stays in the temp folder: the shell reads it
/// after `ShellExecuteW` answers.
#[cfg(windows)]
const SCRATCH_LIFE: std::time::Duration = std::time::Duration::from_secs(600);

/// Removes the copies old enough to be done with.
#[cfg(windows)]
fn sweep_scratch() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("dockmax-jumplist-") && name.ends_with(".lnk")) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > SCRATCH_LIFE);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// A shell link loaded from its bytes.
#[cfg(windows)]
fn load_link(blob: &[u8]) -> Option<IShellLinkW> {
    let path = scratch_path();
    std::fs::write(&path, blob).ok()?;
    let link: Option<IShellLinkW> = (|| {
        let link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.ok()?;
        let file: IPersistFile = link.cast().ok()?;
        unsafe { file.Load(&HSTRING::from(path.as_os_str()), STGM_READ) }.ok()?;
        Some(link)
    })();
    let _ = std::fs::remove_file(&path);
    link
}

/// The title the menu shows for a link: its `System.Title`, else the name of
/// its target, else its description.
#[cfg(windows)]
fn link_title(blob: &[u8]) -> Option<String> {
    let link = load_link(blob)?;
    let title = link
        .cast::<IPropertyStore>()
        .ok()
        .and_then(|store| unsafe { store.GetValue(&PKEY_TITLE) }.ok())
        .map(|value| value.to_string())
        .unwrap_or_default();
    let mut title = resolved(title.trim());
    if title.is_empty() {
        let mut target = [0u16; 520];
        let mut find = WIN32_FIND_DATAW::default();
        if unsafe { link.GetPath(&mut target, &mut find, 0) }.is_ok() {
            let path = from_buffer(&target);
            title = path
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or_default()
                .to_owned();
        }
    }
    if title.is_empty() {
        let mut text = [0u16; 260];
        if unsafe { link.GetDescription(&mut text) }.is_ok() {
            title = from_buffer(&text);
        }
    }
    let title = title.trim().to_owned();
    (!title.is_empty()).then_some(title)
}

/// The recent or frequent destinations Windows keeps for `app_id`.
#[cfg(windows)]
fn listed(app_id: &str, which: APPDOCLISTTYPE) -> Vec<Entry> {
    let Ok(lists) = (unsafe {
        CoCreateInstance::<_, IApplicationDocumentLists>(
            &ApplicationDocumentLists,
            None,
            CLSCTX_INPROC_SERVER,
        )
    }) else {
        return Vec::new();
    };
    if unsafe { lists.SetAppID(&HSTRING::from(app_id)) }.is_err() {
        return Vec::new();
    }
    let Ok(array) = (unsafe { lists.GetList::<IObjectArray>(which, MAX_LIST_ENTRIES as u32) })
    else {
        return Vec::new();
    };
    let count = unsafe { array.GetCount() }.unwrap_or(0);
    let mut entries = Vec::new();
    for index in 0..count.min(MAX_LIST_ENTRIES as u32) {
        if let Ok(item) = unsafe { array.GetAt::<IShellItem>(index) } {
            let title = unsafe { item.GetDisplayName(SIGDN_NORMALDISPLAY) }
                .map(take_text)
                .unwrap_or_default();
            // A file or folder opens by its path; an address by its name.
            let target = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
                .map(take_text)
                .unwrap_or_else(|_| title.clone());
            if !title.trim().is_empty() && !target.is_empty() {
                entries.push(Entry {
                    title,
                    run: Runnable::Open(target),
                });
            }
        } else if let Ok(link) = unsafe { array.GetAt::<IShellLinkW>(index) } {
            // A link: kept as bytes, like the ones of the file.
            let path = scratch_path();
            let saved = link.cast::<IPersistFile>().ok().and_then(|file| {
                unsafe { file.Save(&HSTRING::from(path.as_os_str()), false) }.ok()
            });
            let blob = saved.and_then(|()| std::fs::read(&path).ok());
            let _ = std::fs::remove_file(&path);
            if let Some(blob) = blob
                && let Some(title) = link_title(&blob)
            {
                entries.push(Entry {
                    title,
                    run: Runnable::Link(blob),
                });
            }
        }
    }
    entries
}

/// The AppID of a program: the `System.AppUserModel.ID` of its item in the
/// Apps folder.
#[cfg(windows)]
fn app_id_of(parsing_name: &str) -> Option<String> {
    super::apps::app_user_model_id(parsing_name)
}

/// The path of a program's `CustomDestinations` file, by its AppID.
#[cfg(windows)]
fn custom_file(app_id: &str) -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Recent")
            .join("CustomDestinations")
            .join(format!("{}.customDestinations-ms", app_hash(app_id))),
    )
}

/// The menu of a program of the Apps folder, by its parsing name: the
/// program's categories, or else its recent (or frequent) destinations, and
/// its tasks, in the order the taskbar shows them. Empty sections are left
/// out.
#[cfg(windows)]
pub fn menu(parsing_name: &str) -> Vec<Section> {
    let Some(app_id) = app_id_of(parsing_name) else {
        return Vec::new();
    };
    let file = custom_file(&app_id).and_then(|path| std::fs::read(path).ok());
    let mut custom = file.map_or_else(Vec::new, |data| {
        custom_sections(parse_custom(&data), link_title)
    });
    let tasks = custom
        .iter()
        .position(|section| section.kind == SectionKind::Tasks)
        .map(|at| custom.remove(at));
    let mut sections = custom;
    // A program that made its own categories keeps its recent items there;
    // the taskbar shows those and not the automatic lists. Otherwise it
    // shows the recent ones, or the frequent ones when there are no recent.
    if sections.is_empty() {
        let recent = listed(&app_id, ADLT_RECENT);
        let (kind, entries) = if recent.is_empty() {
            (SectionKind::Frequent, listed(&app_id, ADLT_FREQUENT))
        } else {
            (SectionKind::Recent, recent)
        };
        if !entries.is_empty() {
            sections.push(Section {
                kind,
                title: None,
                entries,
            });
        }
    }
    sections.extend(tasks);
    sections
}

/// Opens an entry of a menu, as the taskbar would.
#[cfg(windows)]
pub fn run(runnable: &Runnable) -> Result<(), String> {
    sweep_scratch();
    let target = match runnable {
        Runnable::Open(target) => HSTRING::from(target.as_str()),
        Runnable::Link(blob) => {
            let path = scratch_path();
            std::fs::write(&path, blob).map_err(|_| "the link could not be copied".to_owned())?;
            HSTRING::from(path.as_os_str())
        }
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

/// Windows alone has the lists.
#[cfg(not(windows))]
pub fn menu(_parsing_name: &str) -> Vec<Section> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn run(_runnable: &Runnable) -> Result<(), String> {
    Err("not Windows".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class that opens each entry, the link's size and the class again,
    /// then `body`: an entry as the file writes it.
    fn entry(body: &[u8]) -> Vec<u8> {
        let mut bytes = LINK_CLASS.to_vec();
        bytes.extend_from_slice(&[0x4C, 0, 0, 0]);
        bytes.extend_from_slice(&LINK_CLASS);
        bytes.extend_from_slice(body);
        bytes
    }

    fn link(body: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x4C, 0, 0, 0];
        bytes.extend_from_slice(&LINK_CLASS);
        bytes.extend_from_slice(body);
        bytes
    }

    fn u32s(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// A file like the ones Windows writes: a known category with nothing in
    /// it, the tasks and a category with a name.
    fn sample() -> Vec<u8> {
        let mut file = u32s(&[2, 3, 0]);
        // Known (frequent), empty: the count is all ones.
        file.extend(u32s(&[1, u32::MAX]));
        file.extend(FOOTER);
        // Tasks: two entries.
        file.extend(u32s(&[2, 2]));
        file.extend(entry(b"first task"));
        file.extend(entry(b"second task"));
        file.extend(FOOTER);
        // A category with a name: one entry.
        file.extend(u32s(&[0]));
        file.extend(7u16.to_le_bytes());
        file.extend(utf16("Recents"));
        file.extend(u32s(&[1]));
        file.extend(entry(b"a recent thing"));
        file.extend(FOOTER);
        file
    }

    #[test]
    fn the_hash_is_the_one_the_shell_names_the_files_with() {
        // Measured on this machine: the files of these AppIDs exist under
        // exactly these names (spec 0006 §6).
        assert_eq!(app_hash("MSEdge"), "ccba5a5986c77e43");
        assert_eq!(app_hash("Chrome"), "5d696d521de238c3");
        assert_eq!(
            app_hash("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            "ff99ba2fb2e34b73"
        );
    }

    #[test]
    fn the_hash_does_not_care_about_case() {
        assert_eq!(app_hash("msedge"), app_hash("MSEdge"));
    }

    #[test]
    fn a_file_is_read_category_by_category() {
        let categories = parse_custom(&sample());

        assert_eq!(categories.len(), 3);
        assert_eq!(categories[0].kind, CategoryKind::Known);
        assert!(categories[0].links.is_empty());
        assert_eq!(categories[1].kind, CategoryKind::Tasks);
        assert_eq!(
            categories[1].links,
            vec![link(b"first task"), link(b"second task")]
        );
        assert_eq!(categories[2].kind, CategoryKind::Custom("Recents".into()));
        assert_eq!(categories[2].links, vec![link(b"a recent thing")]);
    }

    #[test]
    fn a_file_of_another_version_is_not_read() {
        let mut file = sample();
        file[0] = 3;

        assert!(parse_custom(&file).is_empty());
    }

    #[test]
    fn a_file_that_stops_makes_the_categories_read_so_far() {
        let file = sample();
        // Cut inside the last category: no footer after it.
        let cut = &file[..file.len() - 12];

        let categories = parse_custom(cut);

        assert_eq!(categories.len(), 2);
    }

    #[test]
    fn nonsense_gives_nothing_and_does_not_panic() {
        assert!(parse_custom(&[]).is_empty());
        assert!(parse_custom(&[1, 2, 3]).is_empty());
        assert!(parse_custom(&[0xFF; 200]).is_empty());
        assert!(parse_custom(&u32s(&[2, 1_000_000, 0])).is_empty());
        // A category count that the file does not have.
        assert!(parse_custom(&u32s(&[2, 5, 0, 2, 1])).is_empty());
    }

    #[test]
    fn the_sections_are_the_categories_first_and_the_tasks_last() {
        let sections = custom_sections(parse_custom(&sample()), |link| {
            Some(String::from_utf8_lossy(&link[20..]).into_owned())
        });

        let shape: Vec<_> = sections
            .iter()
            .map(|s| (s.kind, s.title.clone(), s.entries.len()))
            .collect();
        assert_eq!(
            shape,
            [
                (SectionKind::Category, Some("Recents".to_owned()), 1),
                (SectionKind::Tasks, None, 2),
            ]
        );
        assert_eq!(sections[1].entries[0].title, "first task");
        assert_eq!(
            sections[1].entries[0].run,
            Runnable::Link(link(b"first task"))
        );
    }

    #[test]
    fn an_entry_with_no_title_is_left_out_and_so_is_an_empty_section() {
        let sections = custom_sections(parse_custom(&sample()), |link| {
            link.ends_with(b"second task").then(|| "second".to_owned())
        });

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].kind, SectionKind::Tasks);
        assert_eq!(sections[0].entries.len(), 1);
    }

    #[test]
    fn a_section_is_cut_at_its_limit() {
        let mut file = u32s(&[2, 1, 0, 2, 20]);
        for n in 0..20u8 {
            file.extend(entry(&[n]));
        }
        file.extend(FOOTER);

        let sections = custom_sections(parse_custom(&file), |_| Some("task".to_owned()));

        assert_eq!(sections[0].entries.len(), MAX_TASKS);
    }

    /// Reads the lists of this PC for every program of the Apps folder and
    /// says what shape each menu has: titles are not printed. For looking:
    /// `cargo test --lib real_lists -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore = "reads the lists of the PC it runs on"]
    fn real_lists() {
        let mut with_menu = 0;
        let apps = crate::windows::apps::catalog().unwrap();
        for app in &apps {
            let sections = menu(&app.id);
            if sections.is_empty() {
                continue;
            }
            with_menu += 1;
            let shape: Vec<String> = sections
                .iter()
                .map(|section| format!("{:?}x{}", section.kind, section.entries.len()))
                .collect();
            println!("{}: {}", app.name, shape.join(", "));
        }
        println!("{with_menu} of {} programs have a menu", apps.len());
    }

    #[test]
    fn a_category_without_a_name_has_no_title() {
        let mut file = u32s(&[2, 1, 0, 0]);
        file.extend(1u16.to_le_bytes());
        file.extend(utf16(" "));
        file.extend(u32s(&[1]));
        file.extend(entry(b"x"));
        file.extend(FOOTER);

        let sections = custom_sections(parse_custom(&file), |_| Some("x".to_owned()));

        assert_eq!(sections[0].kind, SectionKind::Category);
        assert_eq!(sections[0].title, None);
    }
}
