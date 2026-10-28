//! The live faces of the widgets' tiles (DESIGN.md "Ícones vivos"): what
//! each kind shows inside its glaze, at the time it is now.

use windows::Win32::Foundation::SYSTEMTIME;

use super::paint::{self, AgendaLine, Face, Graph, Meter};
use super::theme::Glaze;
use crate::domain::calendar::{self, When};
use crate::domain::dock_layout::DockSize;
use crate::domain::live_faces::{self, Wall};
use crate::domain::system_activity::{self, Reading};
use crate::domain::widgets::calendar::CalendarConfig;
use crate::domain::widgets::clock::{ClockConfig, ClockIconStyle};
use crate::domain::widgets::system::{ActivityConfig, ActivityShow, ActivityStyle};
use crate::domain::widgets::time_progress::{Period, TimeProgressConfig};
use crate::domain::widgets::{WidgetConfig, WidgetInstance};
use crate::windows::calendar::{Snapshot, Status};

/// The wall clock of the system's zone.
pub fn wall(now: &SYSTEMTIME) -> Wall {
    Wall {
        year: i32::from(now.wYear),
        month: now.wMonth as u8,
        day: now.wDay as u8,
        weekday: now.wDayOfWeek as u8,
        hour: now.wHour as u8,
        minute: now.wMinute as u8,
        second: now.wSecond as u8,
    }
}

/// Hours and minutes of the clock's zone: its own when it names one this
/// build knows, the system's otherwise.
fn clock_time(config: &ClockConfig, now: &SYSTEMTIME) -> (u8, u8) {
    let zoned = config.time_zone.as_deref().and_then(|name| {
        let zone = jiff::tz::TimeZoneDatabase::bundled().get(name).ok()?;
        let at = jiff::Timestamp::now().to_zoned(zone);
        Some((at.hour() as u8, at.minute() as u8))
    });
    zoned.unwrap_or((now.wHour as u8, now.wMinute as u8))
}

/// The first `count` periods of a time progress as meters, named.
fn periods(config: &TimeProgressConfig, count: usize, now: &SYSTEMTIME) -> Vec<Meter> {
    let names = &crate::i18n::messages().dock.periods;
    config
        .periods
        .iter()
        .take(count)
        .map(|period| {
            let fraction = live_faces::progress(*period, &wall(now), config);
            let name = names[Period::ALL
                .iter()
                .position(|each| each == period)
                .unwrap_or(0)];
            Meter {
                label: name,
                short: name,
                fraction: fraction.map(|fraction| fraction as f32),
                text: match fraction {
                    Some(fraction) => format!("{}%", live_faces::percent(fraction)),
                    None => crate::i18n::messages().dock.no_value.to_string(),
                },
            }
        })
        .collect()
}

/// The figure of a share, 0–100, as `42%`; the "no value" mark without one.
fn percent_text(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{}%", value.round() as u32),
        None => crate::i18n::messages().dock.no_value.to_string(),
    }
}

/// The Atividade's meters, from the reading (none yet: empty meters).
pub fn meters(config: &ActivityConfig, reading: Option<&Reading>) -> Vec<Meter> {
    let cpu = reading.and_then(|reading| reading.cpu);
    let memory = reading
        .and_then(|reading| reading.memory)
        .and_then(|memory| memory.percent());
    let meter = |label, short, value: Option<f64>| Meter {
        label,
        short,
        fraction: value.map(|value| (value / 100.0) as f32),
        text: percent_text(value),
    };
    let processor = || meter("CPU", "CPU", cpu);
    let ram = || meter("Memória", "RAM", memory);
    match config.show {
        ActivityShow::Both => vec![processor(), ram()],
        ActivityShow::Cpu => vec![processor()],
        ActivityShow::Memory => vec![ram()],
    }
}

/// The network's face: the rates, and on two icons the graph, scaled to
/// the busiest moment of the minute (at least 64 KB/s, so a quiet network
/// draws flat).
pub fn rates(width: u8, reading: Option<&Reading>) -> Face {
    let (down, up) = reading.map_or((0.0, 0.0), |reading| (reading.down, reading.up));
    let wide = width > 1;
    let text = if wide {
        system_activity::rate_text
    } else {
        system_activity::rate_short
    };
    let graph = wide.then(|| {
        let empty = Vec::new();
        let (downs, ups) = reading.map_or((&empty, &empty), |reading| {
            (&reading.down_history, &reading.up_history)
        });
        let peak = downs
            .iter()
            .chain(ups)
            .copied()
            .fold(64.0 * 1024.0, f64::max);
        let scale = |values: &Vec<f64>| values.iter().map(|value| (value / peak) as f32).collect();
        Graph {
            down: scale(downs),
            up: scale(ups),
        }
    });
    Face::Rates {
        glaze: Glaze::NETWORK,
        down: text(down),
        up: text(up),
        graph,
    }
}

/// The Calendário's face (spec 0017 F2a): the next meeting of the
/// calendars it shows, as wide as it is; `opened`, grown under the
/// pointer, with the one after it too.
pub fn agenda(
    config: &CalendarConfig,
    snapshot: Option<&Snapshot>,
    now: i64,
    opened: bool,
) -> Face {
    let text = &crate::i18n::messages().dock;
    let line = |text: String, size: f64, strong: bool| AgendaLine { text, size, strong };
    let width = if opened { 3 } else { config.width };
    let lines = match snapshot {
        None => vec![line(text.no_value.into(), 0.24, true)],
        Some(snapshot) if snapshot.status == Status::NoAccounts => {
            vec![line(text.calendar_connect.into(), 0.16, true)]
        }
        Some(snapshot) => {
            let shown: Vec<_> = snapshot
                .events
                .iter()
                .filter(|event| config.shows(&event.calendar))
                .cloned()
                .collect();
            // Only today's and tomorrow's: the reading looks a week ahead
            // for the card.
            let (_, tomorrow_ends) = calendar::window(now, snapshot.offset);
            let next: Vec<_> = calendar::upcoming(&shown, now)
                .into_iter()
                .filter(|event| event.start < tomorrow_ends)
                .collect();
            let say = |when: When, long: bool| match when {
                When::Now => text.calendar_now.to_owned(),
                When::In(minutes) if long => text.calendar_in.replace("{n}", &minutes.to_string()),
                When::In(minutes) => text.calendar_minutes.replace("{n}", &minutes.to_string()),
                When::At(time) => time,
                When::Tomorrow(time) if long => format!("{} · {time}", text.calendar_tomorrow),
                When::Tomorrow(_) => text.calendar_tomorrow.to_owned(),
            };
            let title = |event: &calendar::Event| {
                if !event.title.is_empty() {
                    event.title.clone()
                } else if event.busy {
                    text.calendar_busy.to_owned()
                } else {
                    text.calendar_untitled.to_owned()
                }
            };
            match next.first() {
                None => vec![line(text.calendar_free.into(), 0.2, true)],
                Some(first) => {
                    let when = calendar::when(first, now, snapshot.offset);
                    match width {
                        1 => vec![line(say(when, false), 0.22, true)],
                        2 => vec![
                            line(say(when, true), 0.2, true),
                            line(title(first), 0.15, false),
                        ],
                        // Grown: two lines, as large as two icons' — when
                        // and the title, then the one after.
                        _ => {
                            let mut lines = vec![line(
                                format!("{} {}", say(when, true), title(first)),
                                0.19,
                                true,
                            )];
                            if let Some(then) = next.get(1) {
                                lines.push(line(
                                    format!(
                                        "{} {} {}",
                                        text.calendar_then,
                                        calendar::clock(then.start, snapshot.offset),
                                        title(then)
                                    ),
                                    0.16,
                                    false,
                                ));
                            }
                            lines
                        }
                    }
                }
            }
        }
    };
    Face::Agenda {
        glaze: Glaze::CALENDAR,
        lines,
    }
}

/// The Ciclo's face (spec 0017 F2b), discreet: a ring of the cycle and a
/// number, the days to the next start, or `•2` on the period's second day.
/// No word on the tile: the panel says what it is.
pub fn cycle(config: &crate::domain::widgets::CycleConfig, size: DockSize) -> Face {
    use crate::domain::cycle::{self, Phase};
    let offset = crate::windows::calendar::offset();
    let today = cycle::day_of(crate::windows::calendar::now(), offset);
    let marks = crate::windows::calendar::cycle_marks(config.symbol.trim(), |calendar| {
        config.reads(calendar)
    });
    let status = marks.map(|marks| {
        cycle::status(
            &cycle::periods(&marks, offset),
            today,
            i64::from(config.sensitive),
        )
    });
    let no_value = crate::i18n::messages().dock.no_value.to_string();
    let text = match status.map(|status| status.phase) {
        Some(Phase::Period { day }) => format!("•{day}"),
        Some(Phase::Sensitive { days } | Phase::Between { days }) => days.max(0).to_string(),
        Some(Phase::Unknown) | None => no_value,
    };
    Face::Ring {
        glaze: Glaze::CYCLE,
        fraction: status.as_ref().and_then(cycle::fraction),
        // The small size shows only the ring.
        text: (size != DockSize::Small).then_some(text),
    }
}

/// The face of `widget` at `now`, with the system's reading for the
/// system widgets and the calendar's for the Calendário.
pub fn widget(
    widget: &WidgetInstance,
    size: DockSize,
    now: &SYSTEMTIME,
    system: Option<&Reading>,
    calendar: Option<&Snapshot>,
    opened: bool,
) -> Face {
    match &widget.config {
        WidgetConfig::Clock(config) => {
            let (hour, minute) = clock_time(config, now);
            match config.icon_style {
                ClockIconStyle::Analog => Face::Dial {
                    glaze: Glaze::CLOCK,
                    hours: f32::from(hour),
                    minutes: f32::from(minute),
                },
                ClockIconStyle::Digital => Face::Time {
                    glaze: Glaze::CLOCK,
                    text: live_faces::clock_text(hour, minute, config.hour_cycle),
                },
            }
        }
        WidgetConfig::TimeProgress(config) if config.width > 1 => Face::Meters {
            glaze: Glaze::PROGRESS,
            // Two icons: a ring per period; three: a bar per period, named.
            style: if config.width == 2 {
                ActivityStyle::Rings
            } else {
                ActivityStyle::Bars
            },
            meters: periods(config, usize::from(config.width), now),
            graph: None,
        },
        WidgetConfig::TimeProgress(config) => {
            let fraction = config
                .periods
                .first()
                .and_then(|period| live_faces::progress(*period, &wall(now), config));
            // The small size shows only the ring.
            let text = (size != DockSize::Small).then(|| match fraction {
                Some(fraction) => format!("{}%", live_faces::percent(fraction)),
                None => crate::i18n::messages().dock.no_value.to_string(),
            });
            Face::Ring {
                glaze: Glaze::PROGRESS,
                fraction: fraction.map(|fraction| fraction as f32),
                text,
            }
        }
        WidgetConfig::Note(_) => Face::Glyph {
            glaze: Glaze::NOTES,
            paths: paint::NOTE_GLYPH,
        },
        WidgetConfig::Activity(config) => Face::Meters {
            glaze: Glaze::ACTIVITY,
            style: config.style,
            meters: meters(config, system),
            // Three icons: the processor's last minute too.
            graph: (config.width >= 3).then(|| {
                system
                    .map(|reading| {
                        reading
                            .cpu_history
                            .iter()
                            .map(|value| (value / 100.0) as f32)
                            .collect()
                    })
                    .unwrap_or_default()
            }),
        },
        WidgetConfig::Network(config) => rates(config.width, system),
        WidgetConfig::Cycle(config) => cycle(config, size),
        WidgetConfig::Calendar(config) => {
            agenda(config, calendar, crate::windows::calendar::now(), opened)
        }
        // Never on the dock in this phase (`dock_items` leaves them out).
        WidgetConfig::Shortcut(_) | WidgetConfig::Files(_) => Face::Glyph {
            glaze: Glaze::FILES,
            paths: paint::NOTE_GLYPH,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::Event;

    const NOW: i64 = 1_793_188_800; // 2026-10-28 09:00 in Brasília.
    const BRT: i64 = -3 * 3600;

    fn event(title: &str, calendar: &str, start: i64) -> Event {
        Event {
            id: title.into(),
            calendar: calendar.into(),
            calendar_name: String::new(),
            color: None,
            title: title.into(),
            start,
            end: start + 1800,
            all_day: false,
            location: None,
            description: None,
            join: None,
            link: None,
            busy: false,
            key: title.into(),
        }
    }

    fn snapshot(events: Vec<Event>) -> Snapshot {
        Snapshot {
            status: Status::Ok,
            events,
            sources: Vec::new(),
            tasks: Vec::new(),
            offset: BRT,
        }
    }

    fn lines(width: u8, hidden: &[&str], snapshot: Option<&Snapshot>) -> Vec<String> {
        let config = CalendarConfig {
            width,
            hidden: hidden.iter().map(|id| id.to_string()).collect(),
            tasks: true,
            task_order: Vec::new(),
            ..CalendarConfig::default()
        };
        match agenda(&config, snapshot, NOW, width == 3) {
            Face::Agenda { lines, .. } => lines.into_iter().map(|line| line.text).collect(),
            _ => panic!("not the Calendário's face"),
        }
    }

    #[test]
    fn the_calendar_tile_says_more_the_wider_it_is() {
        let day = snapshot(vec![
            event("Daily", "max", NOW + 9 * 60),
            event("Revisão", "max", NOW + 2 * 3600),
        ]);
        assert_eq!(lines(1, &[], Some(&day)), ["9 min"]);
        assert_eq!(lines(2, &[], Some(&day)), ["em 9 min", "Daily"]);
        // Grown under the pointer: when and the title, then the one after.
        assert_eq!(
            lines(3, &[], Some(&day)),
            ["em 9 min Daily", "Depois 11:00 Revisão"]
        );
    }

    #[test]
    fn the_calendar_tile_leaves_hidden_calendars_out_and_says_when_free() {
        let day = snapshot(vec![event("Plantão", "noiva", NOW + 600)]);
        assert_eq!(lines(2, &["noiva"], Some(&day)), ["Livre"]);
        let tomorrow = snapshot(vec![event("Dentista", "max", NOW + 24 * 3600)]);
        assert_eq!(lines(1, &[], Some(&tomorrow)), ["amanhã"]);
        assert_eq!(
            lines(2, &[], Some(&tomorrow)),
            ["amanhã · 09:00", "Dentista"]
        );
        let none = Snapshot {
            status: Status::NoAccounts,
            ..snapshot(vec![])
        };
        assert_eq!(lines(2, &[], Some(&none)), ["Conectar"]);
        assert_eq!(lines(2, &[], None), ["–"]);
    }
}
