//! The Calendário's events (spec 0017 F2a): read from every calendar of the
//! connected Google accounts, today's and tomorrow's, on a thread of their
//! own every minute while a Calendário is on a dock, and kept in memory
//! only. Nothing is read while there is none.
//!
//! Google's incremental `syncToken` can't be asked together with a time
//! window and an order, so each minute asks for the two days again: a few
//! small answers, one per calendar.

use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::{Condvar, Mutex, Once};
use std::time::{Duration, Instant};

use serde::Serialize;
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};

use super::google::{self, GoogleError};
use crate::domain::calendar::{self, Event, EventsPage, Source};
use crate::domain::cycle::Mark;
use crate::domain::tasks::{self, GoogleTaskList, GoogleTaskLists, GoogleTasks, Task};
use crate::domain::widgets::CycleConfig;

/// How often the events are read again.
const EVERY: Duration = Duration::from_secs(60);
/// How often the list of calendars is read again.
const SOURCES_EVERY: Duration = Duration::from_secs(30 * 60);

/// How the reading went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ok,
    /// No account is connected.
    NoAccounts,
    /// No connection: the events shown are the last ones read.
    Offline,
    /// An account must sign in again; the others were read.
    SignInAgain,
}

/// What the dock and the panel show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: Status,
    /// Today's and tomorrow's, in order, each meeting once.
    pub events: Vec<Event>,
    /// Every calendar of the accounts, for the widget's settings.
    pub sources: Vec<Source>,
    /// Google Tasks' tasks due today or late, not done (F2a-3).
    pub tasks: Vec<Task>,
    /// Seconds from UTC of the local time, when it was read.
    pub offset: i64,
}

static STATE: Mutex<Option<Snapshot>> = Mutex::new(None);
/// The event the panel should open on (a click on the card, F2a-4): taken
/// by the panel's next reading.
static OPEN: Mutex<Option<String>> = Mutex::new(None);

/// Event the panel gets when it is already open and an event of the card
/// was clicked: it reads the event to show. Must match
/// `CALENDAR_OPEN_EVENT` in `src/core/calendar.ts`.
pub const OPEN_EVENT: &str = "calendar-open-event";

/// The event the panel should show next (none: its list).
pub fn open_event(event: Option<String>) {
    *OPEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = event;
}

/// The event the panel should show, taken: it shows once.
pub fn take_open_event() -> Option<String> {
    OPEN.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}
static SOURCES: Mutex<Option<(Instant, Vec<Source>)>> = Mutex::new(None);
/// A task list with its account.
type AccountList = (String, GoogleTaskList);

/// Each account's task lists, read again with the calendars.
static LISTS: Mutex<Option<(Instant, Vec<AccountList>)>> = Mutex::new(None);
/// The docks (by window) that have a Calendário, and the symbols of their
/// Ciclos: read while any has one.
static WANTERS: Mutex<Vec<(isize, bool, Vec<CycleConfig>)>> = Mutex::new(Vec::new());

/// The Ciclo's notices already shown, by day, symbol, calendar and notice:
/// each once a day. In memory: one may show again after a restart.
static FIRED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The local hour from which the day's notices show.
const NOTICE_HOUR: i64 = 9;

/// The Ciclo's marks (spec 0017 F2b): each symbol's, by calendar, read
/// again every half hour. Only the events with the symbol are asked for
/// (Google's `q`), so nothing else of her calendar reaches the computer;
/// and they stay in memory only.
struct CycleMarks {
    at: Instant,
    symbols: Vec<String>,
    marks: Vec<(String, String, Mark)>,
}

static CYCLE: Mutex<Option<CycleMarks>> = Mutex::new(None);

/// How far back the Ciclo looks: a year and a month.
const CYCLE_BACK: i64 = 400 * 24 * 3600;
const CYCLE_EVERY: Duration = Duration::from_secs(30 * 60);
static WANTED: AtomicBool = AtomicBool::new(false);
static STARTED: Once = Once::new();
/// Set to read again now, with the thread woken.
static WAKE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

fn seconds(time: &SYSTEMTIME) -> i64 {
    jiff::civil::date(time.wYear as i16, time.wMonth as i8, time.wDay as i8)
        .at(time.wHour as i8, time.wMinute as i8, time.wSecond as i8, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .map(|at| at.timestamp().as_second())
        .unwrap_or(0)
}

/// The local time's distance from UTC now, in seconds, to the minute.
pub fn offset() -> i64 {
    let (local, utc) = unsafe { (GetLocalTime(), GetSystemTime()) };
    let difference = seconds(&local) - seconds(&utc);
    (difference as f64 / 60.0).round() as i64 * 60
}

pub fn now() -> i64 {
    jiff::Timestamp::now().as_second()
}

/// The last reading, if any.
pub fn snapshot() -> Option<Snapshot> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Whether the dock `dock` has a Calendário, and its Ciclos' symbols: the
/// reading starts (its thread, the first time) while any dock has one, and
/// stops when none has.
pub fn want(dock: isize, calendar: bool, cycles: Vec<CycleConfig>) {
    let on = {
        let mut wanters = WANTERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        wanters.retain(|(each, _, _)| *each != dock);
        if calendar || !cycles.is_empty() {
            wanters.push((dock, calendar, cycles));
        }
        !wanters.is_empty()
    };
    let was = WANTED.swap(on, Ordering::Relaxed);
    if on {
        STARTED.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("calendar".into())
                .spawn(run);
        });
        if !was {
            read_now();
        }
    }
}

/// Reads again now (the panel opened, the PC woke up).
pub fn read_now() {
    let (lock, wake) = &WAKE;
    *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    wake.notify_all();
}

/// Whether a dock has a Calendário, and every Ciclo's config.
fn wanted() -> (bool, Vec<CycleConfig>) {
    let wanters = WANTERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let calendar = wanters.iter().any(|(_, calendar, _)| *calendar);
    let mut cycles: Vec<CycleConfig> = wanters
        .iter()
        .flat_map(|(_, _, cycles)| cycles.iter().cloned())
        .collect();
    cycles.dedup();
    (calendar, cycles)
}

/// Every Ciclo's symbol, each once.
fn symbols_of(cycles: &[CycleConfig]) -> Vec<String> {
    let mut symbols: Vec<String> = cycles
        .iter()
        .map(|cycle| cycle.symbol.trim().to_owned())
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

/// Shows the day's notices of the Ciclos not shown yet, from 9h: one
/// neutral Windows notice for all of them.
fn notify(cycles: &[CycleConfig]) {
    use crate::domain::cycle;
    let offset = offset();
    let now = now();
    let today = cycle::day_of(now, offset);
    if (now + offset).rem_euclid(24 * 3600) < NOTICE_HOUR * 3600 {
        return;
    }
    let mut new = Vec::new();
    for config in cycles {
        let Some(marks) = cycle_marks(config.symbol.trim(), |id| config.reads(id)) else {
            continue;
        };
        let status = cycle::status(
            &cycle::periods(&marks, offset),
            today,
            i64::from(config.sensitive),
        );
        for notice in cycle::notices(&status, config, today) {
            new.push(format!(
                "{today}|{}|{}|{}",
                config.symbol.trim(),
                config.calendar.as_deref().unwrap_or(""),
                notice.key()
            ));
        }
    }
    let unseen: Vec<String> = {
        let mut fired = FIRED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Yesterday's are no use any more.
        fired.retain(|key| key.starts_with(&format!("{today}|")));
        let unseen: Vec<String> = new.into_iter().filter(|key| !fired.contains(key)).collect();
        fired.extend(unseen.iter().cloned());
        unseen
    };
    if !unseen.is_empty() {
        match super::toast::cycle_notice() {
            Ok(true) => {}
            // The panel still says them under "Hoje".
            Ok(false) => log::info!("cycle: Windows has the notices off"),
            Err(error) => log::warn!("cycle: the notice was not shown: {error}"),
        }
    }
}

/// The marks of `symbol` in the calendars `reads` takes, as the last
/// reading found them; `None` until the first.
pub fn cycle_marks(symbol: &str, reads: impl Fn(&str) -> bool) -> Option<Vec<Mark>> {
    let cycle = CYCLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let cycle = cycle.as_ref()?;
    cycle.symbols.iter().any(|each| each == symbol).then(|| {
        cycle
            .marks
            .iter()
            .filter(|(each, calendar, _)| each == symbol && reads(calendar))
            .map(|(_, _, mark)| *mark)
            .collect()
    })
}

/// Reads the Ciclo's marks again when half an hour passed or the symbols
/// changed. Whether they changed.
fn read_cycle(symbols: &[String]) -> bool {
    {
        let cycle = CYCLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(cycle) = cycle.as_ref()
            && cycle.symbols == symbols
            && cycle.at.elapsed() < CYCLE_EVERY
        {
            return false;
        }
    }
    let offset = offset();
    let accounts = google::accounts();
    let Ok(sources) = sources(&accounts) else {
        // No connection: the marks read before stay.
        return false;
    };
    let now = now();
    let mut marks = Vec::new();
    for source in &sources {
        for symbol in symbols {
            let url = events_url(source, now - CYCLE_BACK, now + 60 * 24 * 3600, Some(symbol));
            match google::get_json::<EventsPage>(&source.account, &url) {
                Ok(page) => marks.extend(
                    calendar::read_marks(&page.items, symbol, offset)
                        .into_iter()
                        .map(|mark| (symbol.clone(), source.id.clone(), mark)),
                ),
                Err(GoogleError::Unexpected) => {}
                Err(_) => return false,
            }
        }
    }
    let mut cycle = CYCLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let changed = cycle.as_ref().is_none_or(|before| before.marks != marks);
    *cycle = Some(CycleMarks {
        at: Instant::now(),
        symbols: symbols.to_vec(),
        marks,
    });
    changed
}

fn run() {
    loop {
        let (calendar, cycles) = wanted();
        let symbols = symbols_of(&cycles);
        if !symbols.is_empty() && read_cycle(&symbols) {
            super::native_dock::calendar_changed();
        }
        if !cycles.is_empty() {
            notify(&cycles);
        }
        if calendar {
            let read = read();
            let changed = {
                let mut state = STATE
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let changed = state.as_ref() != Some(&read);
                *state = Some(read);
                changed
            };
            if changed {
                super::native_dock::calendar_changed();
            }
        }
        let (lock, wake) = &WAKE;
        let mut asked = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !*asked {
            asked = wake
                .wait_timeout(asked, EVERY)
                .map(|(guard, _)| guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner().0);
        }
        *asked = false;
    }
}

/// Every calendar of the accounts, each once: a calendar two accounts see
/// keeps the first account's.
fn sources(accounts: &[String]) -> Result<Vec<Source>, GoogleError> {
    {
        let cached = SOURCES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((at, sources)) = cached.as_ref()
            && at.elapsed() < SOURCES_EVERY
        {
            return Ok(sources.clone());
        }
    }
    let mut sources: Vec<Source> = Vec::new();
    for account in accounts {
        for entry in google::calendars(account)? {
            calendar::keep_best(
                &mut sources,
                Source {
                    account: account.clone(),
                    access: calendar::access_rank(entry.access.as_deref()),
                    id: entry.id,
                    name: entry.name,
                    color: entry.color,
                },
            );
        }
    }
    *SOURCES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((Instant::now(), sources.clone()));
    Ok(sources)
}

/// The events of `source` between `from` and `to`; with `query`, only those
/// that have it (Google's `q`).
fn events_url(source: &Source, from: i64, to: i64, query: Option<&str>) -> String {
    let stamp = |at: i64| {
        jiff::Timestamp::from_second(at)
            .map(|at| at.to_string())
            .unwrap_or_default()
    };
    let mut url = url::Url::parse("https://www.googleapis.com/calendar/v3/calendars/")
        .expect("a valid constant");
    url.path_segments_mut()
        .expect("an https address")
        .pop_if_empty()
        .push(&source.id)
        .push("events");
    url.query_pairs_mut()
        .append_pair("timeMin", &stamp(from))
        .append_pair("timeMax", &stamp(to))
        .append_pair("singleEvents", "true")
        .append_pair("orderBy", "startTime")
        .append_pair("maxResults", "250");
    if let Some(query) = query {
        url.query_pairs_mut().append_pair("q", query);
    }
    url.into()
}

/// One reading of every calendar.
fn read() -> Snapshot {
    let offset = offset();
    let before = snapshot();
    let accounts = google::accounts();
    if accounts.is_empty() {
        return Snapshot {
            status: Status::NoAccounts,
            events: Vec::new(),
            sources: Vec::new(),
            tasks: Vec::new(),
            offset,
        };
    }
    let kept = |status| Snapshot {
        status,
        events: before
            .as_ref()
            .map(|before| before.events.clone())
            .unwrap_or_default(),
        sources: before
            .as_ref()
            .map(|before| before.sources.clone())
            .unwrap_or_default(),
        tasks: before
            .as_ref()
            .map(|before| before.tasks.clone())
            .unwrap_or_default(),
        offset,
    };
    let sources = match sources(&accounts) {
        Ok(sources) => sources,
        Err(GoogleError::SignInAgain) => return kept(Status::SignInAgain),
        Err(_) => return kept(Status::Offline),
    };
    // A week and a day ahead: the card's longest period (F2a-4); the tile
    // and the panel show only today and tomorrow of it.
    let (from, _) = calendar::window(now(), offset);
    let to = from + 8 * 24 * 3600;
    let mut events = Vec::new();
    let mut status = Status::Ok;
    for source in &sources {
        match google::get_json::<EventsPage>(&source.account, &events_url(source, from, to, None)) {
            Ok(page) => events.extend(calendar::read_events(page.items, source, offset)),
            Err(GoogleError::SignInAgain) => status = Status::SignInAgain,
            // A calendar that can't be read (no longer shared): the others
            // still show.
            Err(GoogleError::Unexpected) => {}
            Err(_) => return kept(Status::Offline),
        }
    }
    let tasks = match read_tasks(&accounts, offset) {
        Ok(tasks) => tasks,
        Err(GoogleError::SignInAgain) => {
            status = Status::SignInAgain;
            Vec::new()
        }
        Err(_) => before.map(|before| before.tasks).unwrap_or_default(),
    };
    Snapshot {
        status,
        events: calendar::merge(events),
        sources,
        tasks,
        offset,
    }
}

/// Every account's task lists, each with its account.
fn task_lists(accounts: &[String]) -> Result<Vec<AccountList>, GoogleError> {
    {
        let cached = LISTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((at, lists)) = cached.as_ref()
            && at.elapsed() < SOURCES_EVERY
        {
            return Ok(lists.clone());
        }
    }
    let mut lists = Vec::new();
    for account in accounts {
        let read: GoogleTaskLists = google::get_json(account, tasks::LISTS_URL)?;
        lists.extend(read.items.into_iter().map(|list| (account.clone(), list)));
    }
    *LISTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((Instant::now(), lists.clone()));
    Ok(lists)
}

/// The tasks due today or late, not done, of every list.
fn read_tasks(accounts: &[String], offset: i64) -> Result<Vec<Task>, GoogleError> {
    let Some(today) = tasks::today(now(), offset) else {
        return Ok(Vec::new());
    };
    let mut all = Vec::new();
    for (account, list) in task_lists(accounts)? {
        let read: GoogleTasks = google::get_json(&account, &tasks::tasks_url(&list.id, today))?;
        all.extend(tasks::read_tasks(read.items, &account, &list, today));
    }
    Ok(tasks::sort(all))
}

/// Why a task was not ticked off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteError {
    /// Not one of the tasks shown.
    Unknown,
    Google(GoogleError),
}

/// Ticks off a task the panel shows: Google keeps it done, and it leaves
/// the list at once.
pub fn complete(account: &str, list: &str, id: &str) -> Result<(), CompleteError> {
    let shown = snapshot().is_some_and(|snapshot| {
        snapshot
            .tasks
            .iter()
            .any(|task| task.account == account && task.list == list && task.id == id)
    });
    if !shown {
        return Err(CompleteError::Unknown);
    }
    google::patch_json(account, &tasks::task_url(list, id), tasks::COMPLETED)
        .map_err(CompleteError::Google)?;
    if let Some(snapshot) = STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_mut()
    {
        snapshot
            .tasks
            .retain(|task| !(task.account == account && task.list == list && task.id == id));
    }
    read_now();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_events_address_names_the_calendar_and_the_two_days() {
        let source = Source {
            account: "max@example.com".into(),
            id: "pt-br.brazilian#holiday@group.v.calendar.google.com".into(),
            name: "Feriados".into(),
            color: None,
            access: 1,
        };
        let url = url::Url::parse(&events_url(
            &source,
            1_793_156_400,
            1_793_329_200,
            Some("🌸"),
        ))
        .unwrap();
        assert_eq!(
            url.path(),
            "/calendar/v3/calendars/pt-br.brazilian%23holiday@group.v.calendar.google.com/events"
        );
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs["timeMin"], "2026-10-28T03:00:00Z");
        assert_eq!(pairs["timeMax"], "2026-10-30T03:00:00Z");
        assert_eq!(pairs["singleEvents"], "true");
        // The Ciclo asks only for the events with its symbol.
        assert_eq!(pairs["q"], "🌸");
    }

    #[test]
    fn the_offset_is_whole_minutes() {
        assert_eq!(offset() % 60, 0);
        assert!(offset().abs() <= 14 * 3600);
    }
}
