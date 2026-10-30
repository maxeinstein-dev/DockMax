//! The buffers of Everything's IPC (spec 0012 L4): the query sent to
//! Everything (`EVERYTHING_IPC_QUERY2`, in a `WM_COPYDATA`) and the list it
//! answers with (`EVERYTHING_IPC_LIST2`). Pure: the window messages are in
//! `windows::launcher::files`.
//!
//! The format is the one of voidtools' SDK, `ipc/everything_ipc.h` of
//! <https://www.voidtools.com/Everything-SDK.zip> (the page
//! <https://www.voidtools.com/support/everything/sdk/ipc/>), read on
//! 2026-10-30. The structures are packed (`#pragma pack(1)`), with `DWORD`s
//! in little endian and the texts in UTF-16 (the `W` query):
//!
//! ```text
//! EVERYTHING_IPC_QUERY2: reply_hwnd, reply_copydata_message, search_flags,
//!   offset, max_results, request_flags, sort_type (7 DWORDs), then the
//!   search, null terminated.
//! EVERYTHING_IPC_LIST2: totitems, numitems, offset, request_flags,
//!   sort_type (5 DWORDs), then numitems EVERYTHING_IPC_ITEM2 { flags,
//!   data_offset }, then each item's data at data_offset from the list's
//!   start: for each text asked (NAME, PATH, FULL_PATH_AND_NAME, in that
//!   order), a DWORD length in characters, without the null, then the text
//!   and its null.
//! ```

/// The class of Everything's window that takes the queries.
pub const WINDOW_CLASS: &str = "EVERYTHING_TASKBAR_NOTIFICATION";
/// `EVERYTHING_IPC_COPYDATA_QUERY2W`: the `dwData` of the query.
pub const COPYDATA_QUERY2W: usize = 18;

/// `EVERYTHING_IPC_QUERY2_REQUEST_*`.
pub const REQUEST_NAME: u32 = 0x1;
pub const REQUEST_PATH: u32 = 0x2;
pub const REQUEST_FULL_PATH_AND_NAME: u32 = 0x4;
/// `EVERYTHING_IPC_SORT_DATE_MODIFIED_DESCENDING`: the files changed last
/// first (the launcher orders them again by name and use).
pub const SORT_DATE_MODIFIED_DESCENDING: u32 = 14;
/// `EVERYTHING_IPC_FOLDER`: an item's flag.
pub const ITEM_FOLDER: u32 = 0x1;

/// The size of the query's header and of the list's, and of an item.
const QUERY_HEADER: usize = 7 * 4;
const LIST_HEADER: usize = 5 * 4;
const ITEM: usize = 2 * 4;

/// The most items read from one answer.
const MAX_ITEMS: usize = 1000;
/// The longest text read (characters).
const MAX_TEXT: usize = 32_768;

/// The query for `search`, answered to the window `reply_hwnd` with
/// `reply_id` as the `dwData` of the answer's `WM_COPYDATA`: at most
/// `max_results` items, only their full path asked, no flags (no case, no
/// whole word, no path, no regex: the search's words are literals).
pub fn query2(reply_hwnd: u32, reply_id: u32, search: &str, max_results: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(QUERY_HEADER + (search.len() + 1) * 2);
    for value in [
        reply_hwnd,
        reply_id,
        0, // search_flags
        0, // offset
        max_results,
        REQUEST_FULL_PATH_AND_NAME,
        SORT_DATE_MODIFIED_DESCENDING,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for unit in search.encode_utf16().chain(std::iter::once(0)) {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// One item of an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub path: String,
    pub folder: bool,
}

fn dword(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    let slice = bytes.get(at..end)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

/// The text at `at` (its length, the text, its null), and where the next
/// thing starts.
fn text(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    let length = dword(bytes, at)? as usize;
    if length > MAX_TEXT {
        return None;
    }
    let start = at + 4;
    let end = start.checked_add(length.checked_mul(2)?)?;
    let units: Vec<u16> = bytes
        .get(start..end)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    // The null after the text must be there too.
    let after = end.checked_add(2)?;
    if bytes.get(end..after)? != [0, 0] {
        return None;
    }
    Some((String::from_utf16(&units).ok()?, after))
}

/// The items of an answer (`EVERYTHING_IPC_LIST2`): each one's full path
/// and whether it is a folder. `None` when the buffer is not a list, or
/// does not carry the full paths; an item whose data runs out of the
/// buffer ends the list there.
pub fn parse_list2(bytes: &[u8]) -> Option<Vec<Item>> {
    let count = dword(bytes, 4)? as usize;
    let flags = dword(bytes, 12)?;
    if flags & REQUEST_FULL_PATH_AND_NAME == 0 {
        return None;
    }
    let count = count.min(MAX_ITEMS);
    if LIST_HEADER + count * ITEM > bytes.len() {
        return None;
    }
    let mut items = Vec::with_capacity(count);
    for index in 0..count {
        let at = LIST_HEADER + index * ITEM;
        let item_flags = dword(bytes, at)?;
        let mut data = dword(bytes, at + 4)? as usize;
        // The texts asked come in the order of their flags: the name and
        // the folder (if Everything sent them anyway) before the full path.
        let mut path = None;
        for flag in [REQUEST_NAME, REQUEST_PATH, REQUEST_FULL_PATH_AND_NAME] {
            if flags & flag == 0 {
                continue;
            }
            let Some((value, next)) = text(bytes, data) else {
                return Some(items);
            };
            data = next;
            if flag == REQUEST_FULL_PATH_AND_NAME {
                path = Some(value);
            }
        }
        if let Some(path) = path {
            items.push(Item {
                path,
                folder: item_flags & ITEM_FOLDER != 0,
            });
        }
    }
    Some(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// A list as Everything sends it, with `texts` per item in the order of
    /// `flags`.
    fn list(flags: u32, items: &[(u32, Vec<&str>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for value in [items.len() as u32 + 5, items.len() as u32, 0, flags, 14] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        let mut data = Vec::new();
        let data_start = LIST_HEADER + items.len() * ITEM;
        for (item_flags, texts) in items {
            out.extend_from_slice(&item_flags.to_le_bytes());
            out.extend_from_slice(&((data_start + data.len()) as u32).to_le_bytes());
            for text in texts {
                data.extend_from_slice(&(text.encode_utf16().count() as u32).to_le_bytes());
                data.extend_from_slice(&utf16(text));
                data.extend_from_slice(&[0, 0]);
            }
        }
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn the_query_is_the_sdk_header_and_the_search() {
        let query = query2(0x0012_3456, 7, "\"relatório\"", 30);
        let words: Vec<u32> = query[..QUERY_HEADER]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        assert_eq!(words, [0x0012_3456, 7, 0, 0, 30, 0x4, 14]);
        let mut search = utf16("\"relatório\"");
        search.extend_from_slice(&[0, 0]);
        assert_eq!(&query[QUERY_HEADER..], &search[..]);
        assert_eq!(
            query.len(),
            QUERY_HEADER + 2 * ("\"relatório\"".chars().count() + 1)
        );
    }

    #[test]
    fn the_answer_gives_the_paths_and_the_folders() {
        let bytes = list(
            REQUEST_FULL_PATH_AND_NAME,
            &[
                (0, vec![r"C:\Docs\relatório.pdf"]),
                (ITEM_FOLDER, vec![r"C:\Relatórios"]),
            ],
        );
        assert_eq!(
            parse_list2(&bytes).unwrap(),
            [
                Item {
                    path: r"C:\Docs\relatório.pdf".into(),
                    folder: false
                },
                Item {
                    path: r"C:\Relatórios".into(),
                    folder: true
                },
            ]
        );
    }

    #[test]
    fn the_name_and_the_folder_sent_anyway_are_skipped() {
        let flags = REQUEST_NAME | REQUEST_PATH | REQUEST_FULL_PATH_AND_NAME;
        let bytes = list(flags, &[(0, vec!["a.txt", r"C:\x", r"C:\x\a.txt"])]);
        assert_eq!(parse_list2(&bytes).unwrap()[0].path, r"C:\x\a.txt");
    }

    #[test]
    fn a_bad_answer_reads_nothing_out_of_the_buffer() {
        assert_eq!(parse_list2(&[]), None);
        assert_eq!(parse_list2(&[0; 19]), None);
        // Without the full paths.
        assert_eq!(parse_list2(&list(REQUEST_NAME, &[(0, vec!["a"])])), None);
        // More items than the buffer holds.
        let mut bytes = list(REQUEST_FULL_PATH_AND_NAME, &[]);
        bytes[4..8].copy_from_slice(&50u32.to_le_bytes());
        assert_eq!(parse_list2(&bytes), None);
        // A text longer than the buffer: the list ends before it.
        let mut bytes = list(
            REQUEST_FULL_PATH_AND_NAME,
            &[(0, vec![r"C:\a"]), (0, vec![r"C:\b"])],
        );
        let second_data = LIST_HEADER + 2 * ITEM + 4 + 2 * 4 + 2;
        bytes[second_data..second_data + 4].copy_from_slice(&9999u32.to_le_bytes());
        assert_eq!(parse_list2(&bytes).unwrap().len(), 1);
        // A data offset past the end.
        let mut bytes = list(REQUEST_FULL_PATH_AND_NAME, &[(0, vec![r"C:\a"])]);
        bytes[LIST_HEADER + 4..LIST_HEADER + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_list2(&bytes).unwrap().len(), 0);
        // A text without its null.
        let mut bytes = list(REQUEST_FULL_PATH_AND_NAME, &[(0, vec![r"C:\a"])]);
        let last = bytes.len() - 1;
        bytes[last] = 1;
        assert_eq!(parse_list2(&bytes).unwrap().len(), 0);
    }
}
