//! The launcher's clipboard (spec 0012, "Área de transferência"): Windows'
//! own clipboard history (Win+V), read through WinRT
//! (`Windows.ApplicationModel.DataTransfer.Clipboard`) on a thread of its
//! own, and its rows. What the rows are and how they are found is
//! `domain::launcher_clipboard`, pure.
//!
//! Nothing of what was copied goes to the log: only how many items, of
//! which kind, and how long it took. The texts are `ClipText`, which cannot
//! be printed (`launcher_clipboard`).

use std::time::Instant;

use windows::ApplicationModel::DataTransfer::{
    Clipboard, ClipboardHistoryItem, ClipboardHistoryItemsResultStatus,
    SetHistoryItemAsContentStatus, StandardDataFormats,
};
use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use super::{FILES_TIMER, FOLDER_TIMER, Found, Kind, Launcher, Note, is_open, post};
use crate::domain::launcher::Action;
use crate::domain::launcher_clipboard::{self as clipboard, Ago, Answer, Clip, ClipText, Reading};
use crate::domain::launcher_pages;

/// How long the history has to answer.
pub(super) const CLIP_TIMER: usize = 6;

/// Reads the history on a thread of its own and hands `done` what it
/// answered.
fn read(done: impl FnOnce(Answer) + Send + 'static) -> bool {
    std::thread::Builder::new()
        .name("launcher-clipboard".into())
        .spawn(move || done(single_threaded(read_history)))
        .is_ok()
}

/// Runs `work` with the thread in a single-threaded apartment (STA).
/// `Clipboard` is a single-threaded WinRT class: from a thread without COM
/// (the multithreaded apartment, implicit) each call fails with
/// `RO_E_UNSUPPORTED_FROM_MTA` (0x8000001D), and the history read as
/// "could not be read" in 2 ms, in front or not.
fn single_threaded<T>(work: impl FnOnce() -> T) -> T {
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
    let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    if !entered {
        log::warn!("launcher: the clipboard thread did not enter a single-threaded apartment");
    }
    // What `work` returns holds no WinRT object: it outlives the apartment.
    let done = work();
    if entered {
        unsafe { CoUninitialize() };
    }
    done
}

/// The text items of the history, in Windows' order (the last copied
/// first); `Disabled` with the history off. Blocks on WinRT.
pub(super) fn read_history() -> Answer {
    let started = Instant::now();
    match Clipboard::IsHistoryEnabled() {
        Ok(true) => {}
        Ok(false) => {
            log::info!("launcher: the clipboard history is off");
            return Answer::Disabled;
        }
        Err(error) => {
            log::warn!("launcher: the clipboard history did not say whether it is on: {error}");
            return Answer::Failed;
        }
    }
    let result = match Clipboard::GetHistoryItemsAsync().and_then(|operation| operation.join()) {
        Ok(result) => result,
        Err(error) => {
            log::warn!("launcher: the clipboard history was not read: {error}");
            return Answer::Failed;
        }
    };
    match result.Status() {
        Ok(ClipboardHistoryItemsResultStatus::Success) => {}
        Ok(ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled) => {
            return Answer::Disabled;
        }
        Ok(status) => {
            log::warn!("launcher: the clipboard history refused ({})", status.0);
            return Answer::Failed;
        }
        Err(error) => {
            log::warn!("launcher: the clipboard history did not answer: {error}");
            return Answer::Failed;
        }
    }
    let Ok(items) = result.Items() else {
        return Answer::Failed;
    };
    let Ok(text_format) = StandardDataFormats::Text() else {
        return Answer::Failed;
    };
    let count = items.Size().unwrap_or(0);
    let mut clips = Vec::new();
    // Images, files and the like: left out in this version.
    let mut others = 0;
    for index in 0..count {
        let Ok(item) = items.GetAt(index) else {
            continue;
        };
        match clip_of(&item, &text_format) {
            Some(clip) => clips.push(clip),
            None => others += 1,
        }
    }
    // Counts only: never what was copied.
    log::info!(
        "launcher: clipboard history read: {} texts, {others} other items, in {} ms",
        clips.len(),
        started.elapsed().as_millis()
    );
    Answer::Ready(clips)
}

/// A text item of the history; `None` for an image, files or anything
/// without text.
fn clip_of(item: &ClipboardHistoryItem, text_format: &windows::core::HSTRING) -> Option<Clip> {
    let content = item.Content().ok()?;
    if !content.Contains(text_format).ok()? {
        return None;
    }
    let text = content.GetTextAsync().ok()?.join().ok()?;
    let id = item.Id().ok()?.to_string_lossy();
    let at_ms = item
        .Timestamp()
        .map(|time| clipboard::unix_ms_of_ticks(time.UniversalTime))
        .unwrap_or(0);
    clipboard::is_valid_id(&id).then(|| Clip {
        id,
        text: ClipText::new(&text.to_string_lossy()),
        at_ms,
    })
}

/// Puts the item `id` of the history back as what is copied now
/// (`SetHistoryItemAsContent`). Blocks on WinRT. Whether it did.
fn restore(id: &str) -> bool {
    let items = Clipboard::GetHistoryItemsAsync()
        .and_then(|operation| operation.join())
        .and_then(|result| result.Items());
    let Ok(items) = items else {
        log::warn!("launcher: the clipboard history was not read again");
        return false;
    };
    let count = items.Size().unwrap_or(0);
    let item = (0..count)
        .filter_map(|index| items.GetAt(index).ok())
        .find(|item| item.Id().is_ok_and(|item_id| item_id == id));
    let Some(item) = item else {
        log::info!("launcher: the clipboard item was gone");
        return false;
    };
    match Clipboard::SetHistoryItemAsContent(&item) {
        Ok(SetHistoryItemAsContentStatus::Success) => true,
        Ok(status) => {
            log::warn!(
                "launcher: the clipboard item was not put back ({})",
                status.0
            );
            false
        }
        Err(error) => {
            log::warn!("launcher: the clipboard item was not put back: {error}");
            false
        }
    }
}

/// "há 5 min", in the language in use.
fn ago_text(ago: Ago) -> String {
    let text = &crate::i18n::messages().launcher;
    match ago {
        Ago::Now => text.ago_now.to_owned(),
        Ago::Minutes(n) => text.ago_minutes.replace("{n}", &n.to_string()),
        Ago::Hours(n) => text.ago_hours.replace("{n}", &n.to_string()),
        Ago::Days(1) => text.ago_one_day.to_owned(),
        Ago::Days(n) => text.ago_days.replace("{n}", &n.to_string()),
    }
}

impl Found {
    /// An item of the history: its first line, and under it "Área de
    /// transferência · há 5 min".
    fn clip(clip: &Clip, now_ms: i64) -> Found {
        let text = &crate::i18n::messages().launcher;
        let line = clipboard::first_line(clip.text.expose());
        Found {
            key: format!("clip:{}", clip.id),
            kind: Kind::Clip,
            name: if line.is_empty() {
                text.clip_blank.to_owned()
            } else {
                line
            },
            subtitle: Some(
                text.clip_of
                    .replace("{ago}", &ago_text(clipboard::ago(clip.at_ms, now_ms))),
            ),
            action: Action::CopyClip(clip.id.clone()),
            window: None,
        }
    }

    /// The one row with Windows' history off: Enter opens the page of
    /// Windows' Configurações that turns it on (and counts as that page).
    fn clipboard_off() -> Found {
        let text = &crate::i18n::messages().launcher;
        Found {
            key: launcher_pages::page_key("clipboard"),
            kind: Kind::WindowsPage,
            name: text.clipboard_off.to_owned(),
            subtitle: Some(text.clipboard_off_hint.to_owned()),
            action: Action::OpenUri(clipboard::SETTINGS_URI.to_owned()),
            window: None,
        }
    }
}

impl Launcher {
    /// The text changed: whether it asks for the history (`cb `), which
    /// then answers alone (no apps, windows, files nor web), read once off
    /// the launcher's thread. Leaving `cb ` drops what was read.
    pub(super) fn clip_changed(&mut self) -> bool {
        let taken = self
            .keywords
            .iter()
            .any(|keyword| keyword.keyword.eq_ignore_ascii_case(clipboard::KEYWORD));
        let Some(wanted) = clipboard::parse(&self.query, taken) else {
            if self.clip_query.take().is_some() {
                self.history.clear();
                let _ = unsafe { KillTimer(Some(self.frame), CLIP_TIMER) };
            }
            return false;
        };
        if self.path_query.take().is_some() {
            self.browse.clear();
            let _ = unsafe { KillTimer(Some(self.frame), FOLDER_TIMER) };
        }
        self.file_query = None;
        self.files.clear();
        self.window_query = None;
        let _ = unsafe { KillTimer(Some(self.frame), FILES_TIMER) };
        self.clip_query = Some(wanted);
        if let Some(generation) = self.history.want() {
            unsafe { SetTimer(Some(self.frame), CLIP_TIMER, clipboard::TIMEOUT_MS, None) };
            let started = read(move |answer| {
                post(move |launcher| launcher.clip_read(generation, answer));
            });
            if !started {
                self.history.timed_out(generation);
            }
        }
        self.refresh();
        self.after_change();
        self.announce_soon();
        true
    }

    /// The rows of `cb `, alone: the texts the term finds, the last copied
    /// first; the one row of the history off; or why there are none.
    pub(super) fn refresh_clip(&mut self) {
        let Some(wanted) = &self.clip_query else {
            return;
        };
        let (results, note) = match &self.history.reading {
            // Being read: a moment without rows.
            Reading::Idle | Reading::Waiting(_) => (Vec::new(), Note::None),
            Reading::Done(Answer::Disabled) => (vec![Found::clipboard_off()], Note::None),
            Reading::Done(Answer::Failed) | Reading::Late => (Vec::new(), Note::ClipboardFailed),
            Reading::Done(Answer::Ready(clips)) => {
                let now = crate::persistence::database::now_ms();
                let rows: Vec<Found> = clipboard::rank(clips, &wanted.term)
                    .into_iter()
                    .map(|index| Found::clip(&clips[index], now))
                    .collect();
                let note = if rows.is_empty() {
                    Note::NothingFound
                } else {
                    Note::None
                };
                (rows, note)
            }
        };
        self.recent = false;
        self.results = results;
        self.note = note;
        self.selected = 0;
        self.first = 0;
    }

    /// The history of the reading `generation` came: shown when it is still
    /// waited for, dropped when the launcher closed or `cb ` was left.
    fn clip_read(&mut self, generation: u64, answer: Answer) {
        if !is_open() || !self.history.answered(generation, answer) {
            return;
        }
        let _ = unsafe { KillTimer(Some(self.frame), CLIP_TIMER) };
        self.refresh();
        self.after_change();
        self.announce_soon();
    }

    /// The history did not answer in time: the list says so.
    pub(super) fn clip_late(&mut self) {
        let _ = unsafe { KillTimer(Some(self.frame), CLIP_TIMER) };
        if let Reading::Waiting(generation) = self.history.reading
            && self.history.timed_out(generation)
        {
            log::info!("launcher: the clipboard history did not answer in time");
            self.refresh();
            self.after_change();
            self.announce_soon();
        }
    }

    /// Forgets the history read (the launcher closes).
    pub(super) fn clip_forget(&mut self) {
        self.clip_query = None;
        self.history.clear();
        let _ = unsafe { KillTimer(Some(self.frame), CLIP_TIMER) };
    }

    /// The link the item of `action` is, when its text is one `https://`
    /// link: "Abrir como link".
    pub(super) fn clip_link(&self, action: &Action) -> Option<String> {
        let Action::CopyClip(id) = action else {
            return None;
        };
        let clip = self.history.clips().iter().find(|clip| &clip.id == id)?;
        clipboard::link(clip.text.expose())
    }

    /// Enter: the item `id` is what is copied again, off the launcher's
    /// thread; the launcher closes saying "Copiado". It does not paste: that
    /// would take sending Ctrl+V to the program, and the Dockyard does not
    /// send keys.
    pub(super) fn copy_clip(&mut self, key: String, id: String) {
        self.launching = Some(key.clone());
        let thread_key = key.clone();
        let spawned = std::thread::Builder::new()
            .name("launcher-clipboard".into())
            .spawn(move || {
                let copied = single_threaded(|| restore(&id));
                post(move |launcher| {
                    if copied {
                        super::access::announce(
                            launcher.list.0 as isize,
                            crate::i18n::messages().launcher.clip_copied,
                        );
                    }
                    launcher.launched(&thread_key, false, copied);
                });
            });
        if spawned.is_err() {
            self.launched(&key, false, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads this machine's clipboard history for real and prints only how
    /// many items there are and how long each text is, never what they
    /// say, with the time and the memory it took. Not run by `cargo test`:
    /// `cargo test clipboard_history_answers -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the clipboard history of this machine"]
    fn clipboard_history_answers() {
        let before = private_bytes();
        let started = Instant::now();
        let answer = single_threaded(read_history);
        let took = started.elapsed();
        let after = private_bytes();
        match &answer {
            Answer::Ready(clips) => {
                let sizes: Vec<usize> = clips
                    .iter()
                    .map(|clip| clip.text.expose().chars().count())
                    .collect();
                println!("{} text items, sizes in characters: {sizes:?}", clips.len());
                let with_link = clips
                    .iter()
                    .filter(|clip| clipboard::link(clip.text.expose()).is_some())
                    .count();
                println!("{with_link} of them are one https:// link");
            }
            Answer::Disabled => println!("the history is off"),
            Answer::Failed => println!("the history could not be read"),
        }
        println!("read in {took:?}");
        let started = Instant::now();
        let again = single_threaded(read_history);
        println!(
            "read again in {:?} ({})",
            started.elapsed(),
            match again {
                Answer::Ready(clips) => format!("{} texts", clips.len()),
                Answer::Disabled => "off".into(),
                Answer::Failed => "failed".into(),
            }
        );
        println!(
            "private memory: {:.1} MB before, {:.1} MB after (+{:.1} MB)",
            before as f64 / 1e6,
            after as f64 / 1e6,
            (after as f64 - before as f64) / 1e6
        );
    }

    /// The history's status and how many items it has, never what they
    /// are: "Success 12", "AccessDenied 0".
    fn status_and_count() -> String {
        let enabled = Clipboard::IsHistoryEnabled();
        let result = Clipboard::GetHistoryItemsAsync().and_then(|operation| operation.join());
        describe(enabled, result)
    }

    fn describe(
        enabled: windows::core::Result<bool>,
        result: windows::core::Result<
            windows::ApplicationModel::DataTransfer::ClipboardHistoryItemsResult,
        >,
    ) -> String {
        let result = match result {
            Ok(result) => result,
            Err(error) => return format!("enabled {enabled:?}, error {:?}", error.code()),
        };
        let status = match result.Status() {
            Ok(ClipboardHistoryItemsResultStatus::Success) => "Success".to_owned(),
            Ok(ClipboardHistoryItemsResultStatus::AccessDenied) => "AccessDenied".to_owned(),
            Ok(ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled) => {
                "ClipboardHistoryDisabled".to_owned()
            }
            Ok(other) => format!("status {}", other.0),
            Err(error) => format!("no status {:?}", error.code()),
        };
        let count = result
            .Items()
            .and_then(|items| items.Size())
            .map_or(-1, i64::from);
        format!("enabled {enabled:?}, {status}, {count} items")
    }

    /// Runs the window's messages for `long`.
    fn pump(long: std::time::Duration) {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        };
        let started = Instant::now();
        while started.elapsed() < long {
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Reads the history the ways the launcher could, with a small window
    /// of this process in front (as the launcher is), and prints only the
    /// status and how many items, never what they are. The window closes
    /// itself in a few seconds. Not run by `cargo test`:
    /// `cargo test clipboard_history_in_front -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the clipboard history of this machine, with a window in front"]
    fn clipboard_history_in_front() {
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetForegroundWindow, GetWindowThreadProcessId,
            SW_SHOWNORMAL, SetForegroundWindow, ShowWindow, WS_CAPTION, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST,
        };
        use windows::core::w;

        let time = |label: &str, run: &dyn Fn() -> String| {
            let started = Instant::now();
            let said = run();
            println!("{label}: {said} ({:?})", started.elapsed());
        };
        // Without COM (the multithreaded apartment, implicit), as before the
        // fix, and in a single-threaded one, as the launcher reads now.
        let without_com = || std::thread::spawn(status_and_count).join().unwrap();
        let single = || {
            std::thread::spawn(|| single_threaded(status_and_count))
                .join()
                .unwrap()
        };
        time("not in front, worker thread (MTA)", &without_com);
        time("not in front, worker thread (STA)", &single);

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                w!("STATIC"),
                w!("tipocb"),
                WS_CAPTION,
                40,
                40,
                160,
                60,
                None,
                None,
                None,
                None,
            )
        }
        .expect("window");
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            // The input of the window in front, for a moment: Windows then
            // lets this thread take the foreground.
            let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let mine = GetCurrentThreadId();
            let _ = AttachThreadInput(mine, front, true);
            let _ = SetForegroundWindow(hwnd);
            let _ = AttachThreadInput(mine, front, false);
        }
        pump(std::time::Duration::from_millis(300));
        println!("in front: {}", unsafe { GetForegroundWindow() } == hwnd);

        time("in front, worker thread (MTA)", &without_com);
        time("in front, worker thread (STA)", &single);
        println!("in front: {}", unsafe { GetForegroundWindow() } == hwnd);
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
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

    /// No `log::` line of this file writes what was copied: the texts are
    /// `ClipText` (no `Display`), and the lines here name only counts,
    /// statuses and errors.
    #[test]
    fn no_log_line_names_a_text() {
        let source = include_str!("clipboard.rs");
        // The code, not these tests.
        let code = source.split("#[cfg(test)]").next().unwrap_or("");
        let mut lines = 0;
        for statement in code.split("log::").skip(1) {
            let call = statement.split(';').next().unwrap_or("");
            lines += 1;
            // Neither inlined in the format string nor as an argument after
            // it (the format strings may say "texts", in words).
            let arguments = call.split_once("\",").map_or("", |(_, rest)| rest);
            for word in ["{text", "{clip", "{line", "{name", "{id", "{content"] {
                assert!(!call.contains(word), "a log line takes `{word}`: {call}");
            }
            for word in ["text", "expose", "clip.", "line", "name", "id", "content"] {
                assert!(
                    !arguments.contains(word),
                    "a log line takes `{word}`: {call}"
                );
            }
        }
        assert!(lines >= 5, "the guard reads the log lines ({lines})");
    }
}
