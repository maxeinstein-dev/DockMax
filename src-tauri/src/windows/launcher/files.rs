//! The launcher's search of files and folders (spec 0012 L4), in the
//! launcher's helper process (`icons::serve`): Everything, when it runs, and
//! the Windows Search index, joined without repeating a path
//! (`launcher_files::merge`).
//!
//! - **Everything** (voidtools) through its IPC: a `WM_COPYDATA` with an
//!   `EVERYTHING_IPC_QUERY2` to its window of class
//!   `EVERYTHING_TASKBAR_NOTIFICATION`, answered with another `WM_COPYDATA`
//!   to a message-only window of this thread (`domain::everything_ipc` has
//!   the buffers and the SDK's reference). Without that window Everything is
//!   not running and the source is left out, without an error. The Dockyard
//!   neither installs nor downloads it, nor loads its DLL.
//! - **Windows Search** through OLE DB (`Search.CollatorDSO`, SQL on
//!   `SystemIndex`), on a thread of its own that keeps the session between
//!   queries: COM and OLE DB stay in the helper, out of the app's process.
//!
//! Each source has `TIMEOUT_MS` to answer; a late answer of Windows Search
//! is dropped by its number when it comes.

use std::cell::RefCell;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CLSIDFromProgID, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Search::{
    DBACCESSOR_ROWDATA, DBBINDING, DBMEMOWNER_CLIENTOWNED, DBPARAMIO_NOTPARAM, DBPART_LENGTH,
    DBPART_STATUS, DBPART_VALUE, DBSTATUS_S_OK, DBTYPE_UI4, DBTYPE_WSTR, HACCESSOR, IAccessor,
    ICommandText, IDBCreateCommand, IDBCreateSession, IDBInitialize, IRowset,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, HWND_MESSAGE, MSG,
    MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT, RegisterClassW,
    SMTO_ABORTIFHUNG, SendMessageTimeoutW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_COPYDATA, WNDCLASSW,
};
use windows::core::{GUID, HSTRING, IUnknown, Interface, PCWSTR, w};

use crate::domain::everything_ipc;
use crate::domain::launcher_files::{self, FileHit, TIMEOUT_MS};

/// `DBGUID_DEFAULT`: the SQL dialect of the provider (oledb.h).
const DBGUID_DEFAULT: GUID = GUID::from_u128(0xc8b521fb_5cf3_11ce_ade5_00aa0044773d);
/// `FILE_ATTRIBUTE_DIRECTORY`.
const DIRECTORY: u32 = 0x10;
/// The longest path read from the index, in UTF-16 units.
const PATH_UNITS: usize = launcher_files::MAX_PATH_CHARS + 16;
/// Rows asked of the index at a time.
const BATCH: usize = 16;

/// The class of the window Everything answers to.
const REPLY_CLASS: PCWSTR = w!("DockyardLauncherEverythingReply");

/// The search, as the helper keeps it between queries.
pub struct Searcher {
    windows: Option<mpsc::Sender<(u64, String)>>,
    answers: mpsc::Receiver<(u64, Vec<FileHit>)>,
    sent: u64,
    reply: Option<HWND>,
}

impl Searcher {
    pub fn new() -> Searcher {
        let (requests, received) = mpsc::channel::<(u64, String)>();
        let (answer, answers) = mpsc::channel();
        let windows = std::thread::Builder::new()
            .name("launcher-windows-search".into())
            .spawn(move || windows_search_thread(&received, &answer))
            .ok()
            .map(|_| requests);
        Searcher {
            windows,
            answers,
            sent: 0,
            reply: None,
        }
    }

    /// The files and folders whose name has `words`, at most `limit` of
    /// each source, joined: Everything's first.
    pub fn search(&mut self, words: &[String], limit: usize) -> Vec<FileHit> {
        let started = Instant::now();
        let deadline = started + Duration::from_millis(TIMEOUT_MS);
        self.sent += 1;
        let number = self.sent;
        if let (Some(windows), Some(sql)) = (
            &self.windows,
            launcher_files::windows_search_sql(words, limit),
        ) && windows.send((number, sql)).is_err()
        {
            self.windows = None;
        }
        let everything = self.everything(words, limit, deadline);
        let mut indexed = Vec::new();
        if self.windows.is_some() {
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                match self.answers.recv_timeout(left) {
                    // An answer of an earlier query, late: dropped.
                    Ok((answered, _)) if answered != number => continue,
                    Ok((_, hits)) => {
                        indexed = hits;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        launcher_files::merge([everything, indexed])
    }

    /// Everything's answer; nothing when it is not running, does not take
    /// the query or does not answer before `deadline`.
    fn everything(&mut self, words: &[String], limit: usize, deadline: Instant) -> Vec<FileHit> {
        let Ok(target) =
            (unsafe { FindWindowW(&HSTRING::from(everything_ipc::WINDOW_CLASS), PCWSTR::null()) })
        else {
            return Vec::new();
        };
        let Some(reply) = self.reply_window() else {
            return Vec::new();
        };
        let id = self.sent as u32;
        // Only 32 bits of a window handle count, even on x64 (the SDK).
        let query = everything_ipc::query2(
            reply.0 as usize as u32,
            id,
            &launcher_files::everything_search(words),
            limit as u32,
        );
        let data = COPYDATASTRUCT {
            dwData: everything_ipc::COPYDATA_QUERY2W,
            cbData: query.len() as u32,
            lpData: query.as_ptr() as *mut _,
        };
        REPLY.with(|slot| *slot.borrow_mut() = (id, None));
        let mut taken = 0usize;
        let left = deadline.saturating_duration_since(Instant::now());
        let sent = unsafe {
            SendMessageTimeoutW(
                target,
                WM_COPYDATA,
                WPARAM(reply.0 as usize),
                LPARAM(std::ptr::from_ref(&data) as isize),
                SMTO_ABORTIFHUNG,
                left.as_millis().max(1) as u32,
                Some(&mut taken),
            )
        };
        if sent.0 == 0 || taken == 0 {
            return Vec::new();
        }
        // Everything answers with a message of its own: wait for it.
        loop {
            if let Some(bytes) = REPLY.with(|slot| slot.borrow_mut().1.take()) {
                return everything_ipc::parse_list2(&bytes)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| FileHit {
                        path: item.path,
                        folder: item.folder,
                    })
                    .collect();
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Vec::new();
            }
            unsafe {
                MsgWaitForMultipleObjects(None, false, left.as_millis().max(1) as u32, QS_ALLINPUT);
                let mut message = MSG::default();
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }

    /// The message-only window Everything answers to, made once.
    fn reply_window(&mut self) -> Option<HWND> {
        if self.reply.is_none() {
            let instance = unsafe { GetModuleHandleW(None) }.ok()?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(reply_procedure),
                hInstance: instance.into(),
                lpszClassName: REPLY_CLASS,
                ..Default::default()
            };
            unsafe { RegisterClassW(&class) };
            self.reply = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    REPLY_CLASS,
                    PCWSTR::null(),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    Some(instance.into()),
                    None,
                )
            }
            .ok();
        }
        self.reply
    }
}

thread_local! {
    /// The answer waited for: its id, and its bytes once it comes.
    static REPLY: RefCell<(u32, Option<Vec<u8>>)> = const { RefCell::new((0, None)) };
}

/// The most bytes taken from Everything's answer.
const MAX_REPLY: usize = 4 * 1024 * 1024;

extern "system" fn reply_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_COPYDATA && lparam.0 != 0 {
        let data = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
        let length = data.cbData as usize;
        REPLY.with(|slot| {
            let mut slot = slot.borrow_mut();
            // Only the answer of the last query.
            if data.dwData == slot.0 as usize && !data.lpData.is_null() && length <= MAX_REPLY {
                let bytes = unsafe { std::slice::from_raw_parts(data.lpData.cast::<u8>(), length) };
                slot.1 = Some(bytes.to_vec());
            }
        });
        return LRESULT(1);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Answers the queries of the Windows Search index, the last one first: one
/// asked while another ran is the only one done after it.
fn windows_search_thread(
    received: &mpsc::Receiver<(u64, String)>,
    answer: &mpsc::Sender<(u64, Vec<FileHit>)>,
) {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let mut session: Option<IDBCreateCommand> = None;
    while let Ok(mut request) = received.recv() {
        while let Ok(newer) = received.try_recv() {
            request = newer;
        }
        let (number, sql) = request;
        if session.is_none() {
            session = connect()
                .inspect_err(|error| log::warn!("launcher: Windows Search: {error}"))
                .ok();
        }
        let hits = match session.as_ref().map(|factory| query(factory, &sql)) {
            Some(Ok(hits)) => hits,
            Some(Err(error)) => {
                log::warn!("launcher: Windows Search: {error}");
                // Made again next time (the service may have restarted).
                session = None;
                Vec::new()
            }
            None => Vec::new(),
        };
        if answer.send((number, hits)).is_err() {
            return;
        }
    }
}

/// A session of the Windows Search provider.
pub fn connect() -> windows::core::Result<IDBCreateCommand> {
    unsafe {
        let class = CLSIDFromProgID(w!("Search.CollatorDSO"))?;
        let source: IDBInitialize = CoCreateInstance(&class, None, CLSCTX_INPROC_SERVER)?;
        source.Initialize()?;
        let session: IDBCreateSession = source.cast()?;
        session
            .CreateSession(None::<&IUnknown>, &IDBCreateCommand::IID)?
            .cast()
    }
}

/// A row as the accessor writes it.
#[repr(C)]
struct Row {
    path_status: u32,
    path_length: usize,
    path: [u16; PATH_UNITS],
    attributes_status: u32,
    attributes: u32,
}

/// The files and folders the index answers `sql` with
/// (`launcher_files::windows_search_sql`).
pub fn query(factory: &IDBCreateCommand, sql: &str) -> windows::core::Result<Vec<FileHit>> {
    let mut hits = Vec::new();
    unsafe {
        let command: ICommandText = factory
            .CreateCommand(None::<&IUnknown>, &ICommandText::IID)?
            .cast()?;
        command.SetCommandText(&DBGUID_DEFAULT, &HSTRING::from(sql))?;
        let mut opened: Option<IUnknown> = None;
        command.Execute(
            None::<&IUnknown>,
            &IRowset::IID,
            None,
            None,
            Some(&mut opened),
        )?;
        let Some(opened) = opened else {
            return Ok(hits);
        };
        let rowset: IRowset = opened.cast()?;
        let accessor: IAccessor = rowset.cast()?;
        let part = (DBPART_VALUE.0 | DBPART_LENGTH.0 | DBPART_STATUS.0) as u32;
        let bindings = [
            DBBINDING {
                iOrdinal: 1,
                obValue: std::mem::offset_of!(Row, path),
                obLength: std::mem::offset_of!(Row, path_length),
                obStatus: std::mem::offset_of!(Row, path_status),
                dwPart: part,
                dwMemOwner: DBMEMOWNER_CLIENTOWNED.0 as u32,
                eParamIO: DBPARAMIO_NOTPARAM.0 as u32,
                cbMaxLen: PATH_UNITS * 2,
                wType: DBTYPE_WSTR.0 as u16,
                ..Default::default()
            },
            DBBINDING {
                iOrdinal: 2,
                obValue: std::mem::offset_of!(Row, attributes),
                obStatus: std::mem::offset_of!(Row, attributes_status),
                dwPart: (DBPART_VALUE.0 | DBPART_STATUS.0) as u32,
                dwMemOwner: DBMEMOWNER_CLIENTOWNED.0 as u32,
                eParamIO: DBPARAMIO_NOTPARAM.0 as u32,
                cbMaxLen: 4,
                wType: DBTYPE_UI4.0 as u16,
                ..Default::default()
            },
        ];
        let mut handle = HACCESSOR::default();
        accessor.CreateAccessor(
            DBACCESSOR_ROWDATA.0 as u32,
            bindings.len(),
            bindings.as_ptr(),
            size_of::<Row>(),
            &mut handle,
            None,
        )?;
        let mut row: Box<Row> = Box::new(std::mem::zeroed());
        let read = (|| -> windows::core::Result<()> {
            loop {
                let mut rows = [0usize; BATCH];
                let mut pointer = rows.as_mut_ptr();
                let mut obtained = 0usize;
                // Called through the table: the wrapper takes a slice of
                // pointers where OLE DB wants a pointer to our array.
                (Interface::vtable(&rowset).GetNextRows)(
                    Interface::as_raw(&rowset),
                    0,
                    0,
                    BATCH as isize,
                    &mut obtained,
                    &mut pointer,
                )
                .ok()?;
                let obtained = obtained.min(BATCH);
                for &hrow in &rows[..obtained] {
                    *row = std::mem::zeroed();
                    if rowset
                        .GetData(hrow, handle, std::ptr::from_mut(&mut *row).cast())
                        .is_err()
                        || row.path_status != DBSTATUS_S_OK.0 as u32
                    {
                        continue;
                    }
                    let units = (row.path_length / 2).min(PATH_UNITS);
                    let Some(path) = String::from_utf16(&row.path[..units])
                        .ok()
                        .and_then(|url| launcher_files::path_of_item_url(&url))
                    else {
                        continue;
                    };
                    let folder = row.attributes_status == DBSTATUS_S_OK.0 as u32
                        && row.attributes & DIRECTORY != 0;
                    hits.push(FileHit { path, folder });
                }
                let _ = rowset.ReleaseRows(
                    obtained,
                    rows.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if obtained < BATCH {
                    return Ok(());
                }
            }
        })();
        let _ = accessor.ReleaseAccessor(handle, None);
        read?;
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asks the Windows Search index of this machine for real and prints
    /// what it answers, with the time and the memory it took. Not run by
    /// `cargo test`: `cargo test windows_search_answers -- --ignored
    /// --nocapture` (the word in `LAUNCHER_FILES_WORD`, `relatorio` when
    /// unset).
    #[test]
    #[ignore = "asks the Windows Search of this machine"]
    fn windows_search_answers() {
        let word = std::env::var("LAUNCHER_FILES_WORD").unwrap_or_else(|_| "relatorio".into());
        let words = launcher_files::words(&word);
        let before = private_bytes();
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let started = Instant::now();
        let factory = connect().expect("the Windows Search provider");
        let connected = started.elapsed();
        let sql = launcher_files::windows_search_sql(&words, 30).unwrap();
        println!("SQL: {sql}");
        for round in 0..3 {
            let started = Instant::now();
            let hits = query(&factory, &sql).expect("the query");
            println!(
                "round {round}: {} hits in {:?} (connected in {connected:?})",
                hits.len(),
                started.elapsed()
            );
            if round == 0 {
                for hit in hits.iter().take(10) {
                    let kind = if hit.folder { "Pasta  " } else { "Arquivo" };
                    println!("  {kind} {}", hit.path);
                }
            }
        }
        // The same with LIKE, for comparison (spec 0012, "Como o L4 ficou").
        let like = format!(
            "SELECT TOP 30 System.ItemUrl, System.FileAttributes FROM SystemIndex \
             WHERE SCOPE='file:' AND System.FileName LIKE '%{}%'",
            words[0].replace('\'', "''")
        );
        let started = Instant::now();
        let hits = query(&factory, &like).expect("the LIKE query");
        println!("LIKE: {} hits in {:?}", hits.len(), started.elapsed());
        let after = private_bytes();
        println!(
            "private memory: {:.1} MB before, {:.1} MB after (+{:.1} MB)",
            before as f64 / 1e6,
            after as f64 / 1e6,
            (after as f64 - before as f64) / 1e6
        );
        let mut searcher = Searcher::new();
        let started = Instant::now();
        let joined = searcher.search(&words, 30);
        println!(
            "Searcher (Everything {}): {} hits in {:?}",
            if unsafe { FindWindowW(&HSTRING::from(everything_ipc::WINDOW_CLASS), PCWSTR::null()) }
                .is_ok()
            {
                "running"
            } else {
                "not running"
            },
            joined.len(),
            started.elapsed()
        );
    }

    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged_pool: usize,
        quota_paged_pool: usize,
        quota_peak_non_paged_pool: usize,
        quota_non_paged_pool: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, size: u32) -> i32;
    }

    fn private_bytes() -> usize {
        let mut counters = Counters {
            cb: size_of::<Counters>() as u32,
            ..Default::default()
        };
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
        counters.private_usage
    }
}
