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
