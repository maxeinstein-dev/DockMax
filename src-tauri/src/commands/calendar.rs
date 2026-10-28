//! The Calendário's panel (spec 0017 F2a): today's and tomorrow's events
//! and the calendars, as the dock read them; reading again when it opens;
//! and opening a meeting or an event in Google Agenda. Only the panel
//! calls these.

use serde::Serialize;
use tauri::WebviewWindow;

use crate::windows::panel;
#[cfg(windows)]
use crate::windows::{calendar, google};

/// What the panel shows: the last reading (none yet: `null`), and the time
/// it was asked, in Unix seconds.
#[cfg(windows)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarView {
    pub snapshot: Option<calendar::Snapshot>,
    pub now: i64,
    /// An event to open on (a click on the card, F2a-4), once.
    pub open: Option<String>,
}

fn only_panel(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == panel::LABEL {
        Ok(())
    } else {
        Err("only the panel reads the calendar".into())
    }
}

/// The events as the dock read them. `fresh`: read again now (the panel
/// just opened); the answer still is the last reading, and the next ask,
/// a moment later, gets the new one.
#[cfg(windows)]
#[tauri::command]
pub fn calendar_events(window: WebviewWindow, fresh: bool) -> Result<CalendarView, String> {
    only_panel(&window)?;
    if fresh {
        calendar::read_now();
    }
    Ok(CalendarView {
        snapshot: calendar::snapshot(),
        now: calendar::now(),
        open: calendar::take_open_event(),
    })
}

/// Ticks off a task the panel shows (F2a-3): Google keeps it done. Only
/// a task of the last reading is accepted.
#[cfg(windows)]
#[tauri::command]
pub async fn complete_calendar_task(
    window: WebviewWindow,
    account: String,
    list: String,
    id: String,
) -> Result<(), String> {
    only_panel(&window)?;
    tauri::async_runtime::spawn_blocking(move || calendar::complete(&account, &list, &id))
        .await
        .map_err(|_| "the task could not be ticked off".to_string())?
        .map_err(|error| {
            log::warn!("calendar: a task was not ticked off: {error:?}");
            "the task could not be ticked off".to_string()
        })
}

/// What the Ciclo's panel shows (spec 0017 F2b): where the cycle is, from
/// the marks the dock read; never the marks themselves.
#[cfg(windows)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleView {
    /// `None` until the marks were read.
    pub status: Option<crate::domain::cycle::Status>,
    /// Today, as a local day number (days since 1970-01-01).
    pub today: i64,
    pub accounts: bool,
    /// Today's notices and reminders (F2b-2): the panel says the
    /// reminders' text, which the Windows notice never shows.
    pub notices: Vec<crate::domain::cycle::Notice>,
}

/// The Ciclo of the widget `config` the panel shows.
#[cfg(windows)]
#[tauri::command]
pub fn cycle_status(
    window: WebviewWindow,
    config: crate::domain::widgets::CycleConfig,
) -> Result<CycleView, String> {
    use crate::domain::cycle;
    only_panel(&window)?;
    if config.validate().is_err() {
        return Err("invalid cycle settings".into());
    }
    let offset = calendar::offset();
    let today = cycle::day_of(calendar::now(), offset);
    let status = calendar::cycle_marks(config.symbol.trim(), |id| config.reads(id)).map(|marks| {
        cycle::status(
            &cycle::periods(&marks, offset),
            today,
            i64::from(config.sensitive),
        )
    });
    let notices = status
        .as_ref()
        .map(|status| cycle::notices(status, &config, today))
        .unwrap_or_default();
    Ok(CycleView {
        status,
        today,
        accounts: !google::accounts().is_empty(),
        notices,
    })
}

/// Sends a notice like the Ciclo's, to see that Windows shows it (a button
/// of the widget's settings).
#[cfg(windows)]
#[tauri::command]
pub fn cycle_test_notice(window: WebviewWindow) -> Result<(), String> {
    only_panel(&window)?;
    crate::windows::toast::cycle_notice().map_err(|error| {
        log::warn!("cycle: the test notice was not shown: {error}");
        "the notice could not be shown".to_string()
    })
}

/// Whether `url` is an event's page in Google Agenda.
fn is_calendar_page(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("www.google.com" | "calendar.google.com")
        )
        && url.path().starts_with("/calendar")
}

/// Opens a meeting (Meet, Teams, Zoom) or an event's page in Google
/// Agenda; any other address is refused.
#[cfg(windows)]
#[tauri::command]
pub fn open_calendar_link(window: WebviewWindow, url: String) -> Result<(), String> {
    only_panel(&window)?;
    let opened = if is_calendar_page(&url) {
        google::open_page(&url)
    } else {
        google::open_meeting(&url)
    };
    opened.map_err(|error| {
        log::warn!("calendar: a link was not opened: {error}");
        "the link could not be opened".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_google_agenda_pages_open_as_pages() {
        assert!(is_calendar_page(
            "https://www.google.com/calendar/event?eid=abc"
        ));
        assert!(is_calendar_page("https://calendar.google.com/calendar/r"));
        assert!(!is_calendar_page("http://www.google.com/calendar/event"));
        assert!(!is_calendar_page("https://www.google.com/search?q=x"));
        assert!(!is_calendar_page("https://evil.example/calendar"));
    }
}
