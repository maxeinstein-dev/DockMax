//! The Calendário's rules (spec 0017 F2a): Google's events read into the
//! ones the widget shows, today's and tomorrow's, with the same meeting in
//! two calendars shown once, the declined, cancelled and cycle ("🌸") ones
//! left out, the link to join, and when the next one is. Pure: times are
//! Unix seconds, and the local time is the UTC offset the caller gives.

use serde::{Deserialize, Serialize};

/// The symbol of the cycle's events (spec 0017 F2b): never shown here.
pub const CYCLE_SYMBOL: &str = "🌸";

/// Minutes before a meeting that the tile's middle click joins it.
pub const JOIN_AHEAD: i64 = 10 * 60;

/// A calendar the events come from, as the accounts list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub account: String,
    pub id: String,
    pub name: String,
    /// `#rrggbb`.
    pub color: Option<String>,
    /// How much the account sees of it ([`access_rank`]).
    #[serde(skip)]
    pub access: u8,
}

/// How much an `accessRole` sees: the owner most, the free/busy reader
/// (no titles) least.
pub fn access_rank(role: Option<&str>) -> u8 {
    match role {
        Some("owner") => 3,
        Some("writer") => 2,
        Some("reader") => 1,
        _ => 0,
    }
}

/// Adds `source` to `sources`, or puts it in place of the same calendar
/// seen through another account that sees less of it: the Alfama's
/// calendar, shared as free/busy with the personal account, is read
/// through the Alfama's account, with its titles.
pub fn keep_best(sources: &mut Vec<Source>, source: Source) {
    match sources.iter_mut().find(|known| known.id == source.id) {
        Some(known) if known.access < source.access => *known = source,
        Some(_) => {}
        None => sources.push(source),
    }
}

/// An event as the widget shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub calendar: String,
    pub calendar_name: String,
    pub color: Option<String>,
    pub title: String,
    /// Unix seconds; an all-day event starts at local midnight.
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
    /// The meeting's address (Meet, Teams or Zoom).
    pub join: Option<String>,
    /// The event in Google Agenda.
    pub link: Option<String>,
    /// Shown only as busy, with no title (a free/busy calendar).
    pub busy: bool,
    /// The same event in another calendar has this key too.
    #[serde(skip)]
    pub key: String,
}

/// Google's time of an event: an instant, or a day for all-day events.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTime {
    pub date_time: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAttendee {
    #[serde(default, rename = "self")]
    pub is_self: bool,
    pub response_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleEntryPoint {
    pub entry_point_type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleConference {
    #[serde(default)]
    pub entry_points: Vec<GoogleEntryPoint>,
}

/// An item of `events.list`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleEvent {
    pub id: String,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub html_link: Option<String>,
    pub hangout_link: Option<String>,
    #[serde(rename = "iCalUID")]
    pub ical_uid: Option<String>,
    #[serde(default)]
    pub conference_data: Option<GoogleConference>,
    #[serde(default)]
    pub start: GoogleTime,
    #[serde(default)]
    pub end: GoogleTime,
    #[serde(default)]
    pub attendees: Vec<GoogleAttendee>,
}

/// A page of `events.list`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    #[serde(default)]
    pub items: Vec<GoogleEvent>,
}

/// Unix seconds of a `dateTime` (RFC 3339, with its offset).
fn instant(text: &str) -> Option<i64> {
    text.parse::<jiff::Timestamp>()
        .ok()
        .map(|at| at.as_second())
}

/// Unix seconds of local midnight of a `date` (`2026-10-28`), at
/// `offset` seconds from UTC.
fn midnight(text: &str, offset: i64) -> Option<i64> {
    let date: jiff::civil::Date = text.parse().ok()?;
    let utc = date
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?
        .timestamp()
        .as_second();
    Some(utc - offset)
}

/// The address to join a meeting: Meet's, the conference's video entry,
/// or a Teams or Zoom address written in the place or the description.
pub fn join_link(event: &GoogleEvent) -> Option<String> {
    if let Some(link) = &event.hangout_link {
        return Some(link.clone());
    }
    let video = event.conference_data.as_ref().and_then(|conference| {
        conference
            .entry_points
            .iter()
            .find(|entry| entry.entry_point_type.as_deref() == Some("video"))
            .and_then(|entry| entry.uri.clone())
    });
    if video.is_some() {
        return video;
    }
    [&event.location, &event.description]
        .into_iter()
        .flatten()
        .flat_map(|text| text.split(|c: char| c.is_whitespace() || "\"'<>()".contains(c)))
        .find(|word| is_meeting_address(word))
        .map(str::to_owned)
}

/// Whether `word` is the address of a Teams, Zoom or Meet meeting.
pub fn is_meeting_address(word: &str) -> bool {
    let Ok(url) = url::Url::parse(word) else {
        return false;
    };
    let host = url.host_str().unwrap_or("");
    url.scheme() == "https"
        && (host == "meet.google.com"
            || host == "teams.microsoft.com"
            || host == "teams.live.com"
            || host == "zoom.us"
            || host.ends_with(".zoom.us"))
}

/// Whether the person declined it.
fn declined(event: &GoogleEvent) -> bool {
    event
        .attendees
        .iter()
        .any(|attendee| attendee.is_self && attendee.response_status.as_deref() == Some("declined"))
}

/// Google's events of `source` as the widget's, at `offset` seconds from
/// UTC. Cancelled, declined and cycle events, and the ones without a time
/// it can read, are left out.
pub fn read_events(items: Vec<GoogleEvent>, source: &Source, offset: i64) -> Vec<Event> {
    items
        .into_iter()
        .filter(|event| event.status.as_deref() != Some("cancelled") && !declined(event))
        .filter(|event| {
            !event
                .summary
                .as_deref()
                .is_some_and(|title| title.contains(CYCLE_SYMBOL))
        })
        .filter_map(|event| {
            let (start, end, all_day) = match (&event.start.date_time, &event.start.date) {
                (Some(start), _) => (
                    instant(start)?,
                    event.end.date_time.as_deref().and_then(instant)?,
                    false,
                ),
                (None, Some(day)) => (
                    midnight(day, offset)?,
                    event
                        .end
                        .date
                        .as_deref()
                        .and_then(|day| midnight(day, offset))?,
                    true,
                ),
                (None, None) => return None,
            };
            let join = join_link(&event);
            let title = event
                .summary
                .filter(|title| !title.trim().is_empty())
                .unwrap_or_default();
            let key = format!(
                "{}|{start}",
                event.ical_uid.clone().unwrap_or_else(|| event.id.clone())
            );
            Some(Event {
                id: event.id,
                calendar: source.id.clone(),
                calendar_name: source.name.clone(),
                color: source.color.clone(),
                busy: title.is_empty() && source.access == 0,
                title,
                start,
                end,
                all_day,
                location: event.location,
                description: event.description,
                join,
                link: event.html_link,
                key,
            })
        })
        .collect()
}

/// The events in order of start, each meeting once: the same one in two
/// calendars (the author's and the fiancée's, or two accounts) keeps the
/// copy with a title, else the first calendar's.
pub fn merge(mut events: Vec<Event>) -> Vec<Event> {
    events.sort_by(|a, b| {
        (a.start, a.all_day, a.title.is_empty(), &a.title).cmp(&(
            b.start,
            b.all_day,
            b.title.is_empty(),
            &b.title,
        ))
    });
    let mut seen = std::collections::HashSet::new();
    events.retain(|event| seen.insert(event.key.clone()));
    events
}

/// The first second of today and the first of the day after tomorrow, at
/// `offset` seconds from UTC: what the widget asks Google for.
pub fn window(now: i64, offset: i64) -> (i64, i64) {
    let day = 24 * 3600;
    let local = now + offset;
    let today = local - local.rem_euclid(day) - offset;
    (today, today + 2 * day)
}

/// The meetings still to come or under way, in order: not all-day ones.
pub fn upcoming(events: &[Event], now: i64) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| !event.all_day && event.end > now)
        .collect()
}

/// `14:30`: the local time of `at`.
pub fn clock(at: i64, offset: i64) -> String {
    let minutes = (at + offset).rem_euclid(24 * 3600) / 60;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// When a meeting is, for the tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    /// It started.
    Now,
    /// It starts within the hour: in this many minutes.
    In(i64),
    /// Later today, at this time (`14:30`).
    At(String),
    /// Tomorrow, at this time.
    Tomorrow(String),
}

/// When `event` is, seen at `now`.
pub fn when(event: &Event, now: i64, offset: i64) -> When {
    let day = |at: i64| (at + offset).div_euclid(24 * 3600);
    if event.start <= now {
        When::Now
    } else if event.start - now < 3600 {
        When::In((event.start - now + 59) / 60)
    } else if day(event.start) > day(now) {
        When::Tomorrow(clock(event.start, offset))
    } else {
        When::At(clock(event.start, offset))
    }
}

/// The meeting the tile's middle click joins: the first with an address
/// that started or starts within [`JOIN_AHEAD`].
pub fn joinable(events: &[Event], now: i64) -> Option<&Event> {
    upcoming(events, now)
        .into_iter()
        .find(|event| event.join.is_some() && event.start - now <= JOIN_AHEAD)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRT: i64 = -3 * 3600;

    fn source() -> Source {
        Source {
            account: "max@example.com".into(),
            id: "max@example.com".into(),
            name: "Max".into(),
            color: Some("#9fc6e7".into()),
            access: 3,
        }
    }

    fn items(json: &str) -> Vec<GoogleEvent> {
        serde_json::from_str::<EventsPage>(json).unwrap().items
    }

    #[test]
    fn events_read_with_their_times_and_links() {
        let events = read_events(
            items(
                r#"{"items":[
                {"id":"a","summary":"Daily","start":{"dateTime":"2026-10-28T09:00:00-03:00"},
                 "end":{"dateTime":"2026-10-28T09:15:00-03:00"},"hangoutLink":"https://meet.google.com/abc-defg-hij",
                 "htmlLink":"https://www.google.com/calendar/event?eid=x","iCalUID":"u1"},
                {"id":"b","summary":"Feriado","start":{"date":"2026-11-02"},"end":{"date":"2026-11-03"}}
            ]}"#,
            ),
            &source(),
            BRT,
        );
        assert_eq!(events.len(), 2);
        let daily = &events[0];
        assert_eq!(daily.start, 1_793_188_800);
        assert_eq!(daily.end - daily.start, 15 * 60);
        assert_eq!(
            daily.join.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
        assert!(!daily.all_day);
        let holiday = &events[1];
        assert!(holiday.all_day);
        assert_eq!(clock(holiday.start, BRT), "00:00");
        assert_eq!(holiday.end - holiday.start, 24 * 3600);
    }

    #[test]
    fn declined_cancelled_and_cycle_events_are_left_out() {
        let events = read_events(
            items(
                r#"{"items":[
                {"id":"a","summary":"Recusada","start":{"dateTime":"2026-10-28T10:00:00Z"},"end":{"dateTime":"2026-10-28T11:00:00Z"},
                 "attendees":[{"self":true,"responseStatus":"declined"},{"responseStatus":"accepted"}]},
                {"id":"b","status":"cancelled","summary":"Cancelada","start":{"dateTime":"2026-10-28T10:00:00Z"},"end":{"dateTime":"2026-10-28T11:00:00Z"}},
                {"id":"c","summary":"🌸","start":{"date":"2026-10-28"},"end":{"date":"2026-10-29"}},
                {"id":"d","summary":"Aceita","start":{"dateTime":"2026-10-28T10:00:00Z"},"end":{"dateTime":"2026-10-28T11:00:00Z"},
                 "attendees":[{"self":true,"responseStatus":"accepted"}]}
            ]}"#,
            ),
            &source(),
            BRT,
        );
        let titles: Vec<_> = events.iter().map(|event| event.title.as_str()).collect();
        assert_eq!(titles, ["Aceita"]);
    }

    #[test]
    fn teams_and_zoom_addresses_are_found_in_the_text() {
        let event = |location: &str| GoogleEvent {
            id: "x".into(),
            status: None,
            summary: None,
            location: Some(location.into()),
            description: None,
            html_link: None,
            hangout_link: None,
            ical_uid: None,
            conference_data: None,
            start: GoogleTime::default(),
            end: GoogleTime::default(),
            attendees: vec![],
        };
        assert_eq!(
            join_link(&event("Sala 2 (https://alfama.zoom.us/j/123?pwd=x)")).as_deref(),
            Some("https://alfama.zoom.us/j/123?pwd=x")
        );
        assert_eq!(
            join_link(&event("https://teams.microsoft.com/l/meetup-join/abc")).as_deref(),
            Some("https://teams.microsoft.com/l/meetup-join/abc")
        );
        assert_eq!(join_link(&event("http://zoom.us/j/1 Sala 3")), None);
        assert_eq!(join_link(&event("https://evil.example/zoom.us")), None);
    }

    #[test]
    fn the_same_meeting_in_two_calendars_shows_once() {
        let json = r#"{"items":[{"id":"a","summary":"Sync","iCalUID":"same",
            "start":{"dateTime":"2026-10-28T13:00:00Z"},"end":{"dateTime":"2026-10-28T14:00:00Z"}}]}"#;
        let other = Source {
            id: "noiva@example.com".into(),
            ..source()
        };
        let mut all = read_events(items(json), &source(), BRT);
        all.extend(read_events(items(json), &other, BRT));
        let merged = merge(all);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].calendar, "max@example.com");
    }

    #[test]
    fn a_calendar_two_accounts_see_is_read_through_the_one_that_sees_more() {
        let through = |account: &str, role: &str| Source {
            account: account.into(),
            id: "maxsuelm@alfamaweb.com.br".into(),
            name: "Alfama".into(),
            color: None,
            access: access_rank(Some(role)),
        };
        let mut sources = Vec::new();
        keep_best(&mut sources, through("pessoal@gmail.com", "freeBusyReader"));
        keep_best(&mut sources, through("maxsuelm@alfamaweb.com.br", "owner"));
        keep_best(&mut sources, through("outra@gmail.com", "reader"));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].account, "maxsuelm@alfamaweb.com.br");
        assert_eq!(access_rank(None), 0);
    }

    #[test]
    fn a_free_busy_event_is_busy_and_the_titled_copy_wins() {
        let json = r#"{"items":[{"id":"a","iCalUID":"same",
            "start":{"dateTime":"2026-10-28T18:00:00Z"},"end":{"dateTime":"2026-10-28T19:00:00Z"}}]}"#;
        let busy_only = Source {
            access: 0,
            ..source()
        };
        let hidden = read_events(items(json), &busy_only, BRT);
        assert!(hidden[0].busy);
        assert_eq!(hidden[0].title, "");

        let titled_json = json.replace(r#""id":"a","#, r#""id":"a","summary":"Sprint review","#);
        let titled = read_events(items(&titled_json), &source(), BRT);
        assert!(!titled[0].busy);
        let mut all = hidden;
        all.extend(titled);
        let merged = merge(all);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "Sprint review");
    }

    #[test]
    fn the_window_is_today_and_tomorrow_in_local_time() {
        // 2026-10-28 01:30 in Brasília is still 04:30 UTC of the same day.
        let now = 1_793_163_000;
        let (start, end) = window(now, BRT);
        assert_eq!(clock(start, BRT), "00:00");
        assert_eq!(end - start, 48 * 3600);
        assert!(start <= now && now < end);
    }

    #[test]
    fn the_tile_says_when_the_next_meeting_is() {
        let at = |start: i64| Event {
            id: String::new(),
            calendar: String::new(),
            calendar_name: String::new(),
            color: None,
            title: "Daily".into(),
            start,
            end: start + 900,
            all_day: false,
            location: None,
            description: None,
            join: Some("https://meet.google.com/x".into()),
            link: None,
            busy: false,
            key: String::new(),
        };
        let now = 1_793_188_800;
        assert_eq!(when(&at(now - 60), now, BRT), When::Now);
        assert_eq!(when(&at(now + 9 * 60), now, BRT), When::In(9));
        assert_eq!(when(&at(now + 61), now, BRT), When::In(2));
        assert_eq!(
            when(&at(now + 5 * 3600), now, BRT),
            When::At("14:00".into())
        );
        assert_eq!(
            when(&at(now + 24 * 3600), now, BRT),
            When::Tomorrow("09:00".into())
        );

        let soon = [at(now + 20 * 60), at(now + 5 * 60)];
        let merged = merge(soon.to_vec());
        assert_eq!(
            joinable(&merged, now).map(|event| event.start),
            Some(now + 5 * 60)
        );
        assert_eq!(joinable(&merged[1..], now), None);
    }
}
