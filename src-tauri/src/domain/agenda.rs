//! The Calendário's card (spec 0017 F2a-4): what shows over the dock while
//! the pointer is on the Calendário, the meetings of a period and the day's
//! tasks, in rows that scroll; and where each row is. Pure: the card's
//! window (`windows/agenda.rs`) draws these rows.

use super::calendar::{self, Event};
use super::tasks::Task;
pub use super::widgets::calendar::CardPeriod;

const DAY: i64 = 24 * 3600;

/// The hour after which `Auto` looks at the next morning.
pub const EVENING: i64 = 18;

/// The span the card shows, from `now`: Unix seconds, at `offset` seconds
/// from UTC.
pub fn range(period: CardPeriod, now: i64, offset: i64) -> (i64, i64) {
    let (today, _) = calendar::window(now, offset);
    let end_of = |days: i64| today + days * DAY;
    let to = match period {
        CardPeriod::Auto => {
            if now - today >= EVENING * 3600 {
                end_of(1) + 12 * 3600
            } else {
                end_of(1)
            }
        }
        CardPeriod::Day => end_of(1),
        CardPeriod::TwoDays => end_of(2),
        CardPeriod::ThreeDays => end_of(3),
        CardPeriod::Week => end_of(7),
    };
    (now, to)
}

/// The words the rows use, from `i18n`.
#[derive(Debug, Clone, Copy)]
pub struct Words {
    pub today: &'static str,
    pub tomorrow: &'static str,
    /// Sunday first.
    pub weekdays: [&'static str; 7],
    pub all_day: &'static str,
    pub now: &'static str,
    pub busy: &'static str,
    pub untitled: &'static str,
    pub tasks: &'static str,
    pub free: &'static str,
}

/// One row of the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A day's name: `Hoje`, `Amanhã`, `qui., 30/10`.
    Day(String),
    Event {
        /// The event's id, to open its details.
        id: String,
        color: Option<String>,
        when: String,
        title: String,
    },
    /// Nothing in the span.
    Free(String),
    Tasks(String),
    Task {
        title: String,
        late: bool,
    },
}

impl Row {
    /// A heading (a day, Tarefas) is lower than a line.
    pub fn is_heading(&self) -> bool {
        matches!(self, Row::Day(_) | Row::Tasks(_))
    }
}

fn day_name(day: i64, today: i64, words: &Words) -> String {
    match day - today {
        0 => words.today.to_owned(),
        1 => words.tomorrow.to_owned(),
        _ => {
            let date = jiff::Timestamp::from_second(day * DAY)
                .map(|at| at.to_zoned(jiff::tz::TimeZone::UTC).date())
                .ok();
            match date {
                Some(date) => format!(
                    "{}, {:02}/{:02}",
                    words.weekdays[date.weekday().to_sunday_zero_offset() as usize],
                    date.day(),
                    date.month()
                ),
                None => String::new(),
            }
        }
    }
}

/// The card's rows: the events of `range` by day (the all-day ones first
/// in each day), then, with `with_tasks`, the day's tasks in `tasks`'s
/// order.
pub fn rows(
    events: &[&Event],
    tasks: &[&Task],
    with_tasks: bool,
    range: (i64, i64),
    now: i64,
    offset: i64,
    words: &Words,
) -> Vec<Row> {
    let (from, to) = range;
    let local_day = |at: i64| (at + offset).div_euclid(DAY);
    let today = local_day(now);
    let mut rows = Vec::new();
    let mut shown: Vec<&&Event> = events
        .iter()
        .filter(|event| event.end > from && event.start < to)
        .collect();
    // Each day: all-day first, then by time.
    shown.sort_by_key(|event| {
        let day = local_day(event.start.max(from));
        (day, !event.all_day, event.start)
    });
    let mut current = None;
    for event in shown {
        let day = if event.all_day {
            local_day(event.start.max(now))
        } else {
            local_day(event.start)
        };
        if current != Some(day) {
            current = Some(day);
            rows.push(Row::Day(day_name(day, today, words)));
        }
        let when = if event.all_day {
            words.all_day.to_owned()
        } else if event.start <= now {
            words.now.to_owned()
        } else {
            calendar::clock(event.start, offset)
        };
        let title = if !event.title.is_empty() {
            event.title.clone()
        } else if event.busy {
            words.busy.to_owned()
        } else {
            words.untitled.to_owned()
        };
        rows.push(Row::Event {
            id: event.id.clone(),
            color: event.color.clone(),
            when,
            title,
        });
    }
    if rows.is_empty() {
        rows.push(Row::Free(words.free.to_owned()));
    }
    if with_tasks && !tasks.is_empty() {
        rows.push(Row::Tasks(words.tasks.to_owned()));
        for task in tasks {
            rows.push(Row::Task {
                title: if task.title.is_empty() {
                    words.untitled.to_owned()
                } else {
                    task.title.clone()
                },
                late: task.late,
            });
        }
    }
    rows
}

/// The tasks in the order given in the panel (by title), then the others
/// in Google's order, as the panel shows them.
pub fn ordered<'a>(tasks: &'a [Task], order: &[String]) -> Vec<&'a Task> {
    let rank = |task: &Task| {
        order
            .iter()
            .position(|title| *title == task.title)
            .unwrap_or(order.len())
    };
    let mut ordered: Vec<&Task> = tasks.iter().collect();
    ordered.sort_by_key(|task| rank(task));
    ordered
}

/// The card's size and its rows' places, in logical pixels.
pub const WIDTH: f64 = 300.0;
pub const PAD: f64 = 10.0;
pub const LINE: f64 = 30.0;
pub const HEADING: f64 = 26.0;
/// The tallest the card gets; past it, the rows scroll.
pub const MAX_HEIGHT: f64 = 380.0;

/// The top of each row in the content, and the content's height.
pub fn tops(rows: &[Row]) -> (Vec<f64>, f64) {
    let mut y = PAD;
    let mut tops = Vec::with_capacity(rows.len());
    for row in rows {
        tops.push(y);
        y += if row.is_heading() { HEADING } else { LINE };
    }
    (tops, y + PAD)
}

/// The card's height for a content `content` tall.
pub fn height(content: f64) -> f64 {
    content.min(MAX_HEIGHT)
}

/// The scroll kept within the content.
pub fn clamp_scroll(scroll: f64, content: f64) -> f64 {
    scroll.clamp(0.0, (content - MAX_HEIGHT).max(0.0))
}

/// The row under `y` (in the card, scrolled by `scroll`).
pub fn hit(rows: &[Row], y: f64, scroll: f64) -> Option<usize> {
    let (tops, _) = tops(rows);
    let y = y + scroll;
    tops.iter().enumerate().position(|(index, top)| {
        let tall = if rows[index].is_heading() {
            HEADING
        } else {
            LINE
        };
        y >= *top && y < top + tall
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRT: i64 = -3 * 3600;
    // 2026-10-28 09:00 in Brasília (a Wednesday).
    const MORNING: i64 = 1_793_188_800;

    fn words() -> Words {
        Words {
            today: "Hoje",
            tomorrow: "Amanhã",
            weekdays: ["dom.", "seg.", "ter.", "qua.", "qui.", "sex.", "sáb."],
            all_day: "Dia inteiro",
            now: "agora",
            busy: "Ocupado",
            untitled: "(sem título)",
            tasks: "Tarefas",
            free: "Nada marcado",
        }
    }

    fn event(title: &str, start: i64, all_day: bool) -> Event {
        Event {
            id: title.into(),
            calendar: "c".into(),
            calendar_name: String::new(),
            color: None,
            title: title.into(),
            start,
            end: start + if all_day { DAY } else { 1800 },
            all_day,
            location: None,
            description: None,
            join: None,
            link: None,
            busy: false,
            key: title.into(),
        }
    }

    fn task(title: &str) -> Task {
        Task {
            account: "a".into(),
            list: "L".into(),
            list_name: String::new(),
            id: title.into(),
            title: title.into(),
            notes: None,
            due: "2026-10-28".into(),
            late: false,
            link: None,
            position: String::new(),
        }
    }

    #[test]
    fn auto_shows_the_day_and_after_six_the_next_morning() {
        let (from, to) = range(CardPeriod::Auto, MORNING, BRT);
        assert_eq!(from, MORNING);
        assert_eq!(calendar::clock(to, BRT), "00:00");
        assert_eq!(to - MORNING, 15 * 3600);
        let evening = MORNING + 10 * 3600; // 19:00
        let (_, to) = range(CardPeriod::Auto, evening, BRT);
        assert_eq!(calendar::clock(to, BRT), "12:00");
        assert_eq!(to - evening, 17 * 3600);
        let (_, week) = range(CardPeriod::Week, MORNING, BRT);
        assert_eq!(week - MORNING, 15 * 3600 + 6 * DAY);
    }

    #[test]
    fn the_rows_go_by_day_with_the_all_day_ones_first_then_the_tasks() {
        let events = [
            event("Daily", MORNING + 3600, false),
            event("Feriado", MORNING - 9 * 3600, true),
            event("Workout", MORNING + 21 * 3600, false),
            event("Revisão", MORNING + 2 * DAY + 3600, false),
            event("Passou", MORNING - 3600, false),
        ];
        let refs: Vec<&Event> = events.iter().collect();
        let tasks = [task("Ferro"), task("Própolis")];
        let task_refs: Vec<&Task> = tasks.iter().collect();
        let rows = rows(
            &refs,
            &task_refs,
            true,
            range(CardPeriod::ThreeDays, MORNING, BRT),
            MORNING,
            BRT,
            &words(),
        );
        let text: Vec<String> = rows
            .iter()
            .map(|row| match row {
                Row::Day(name) | Row::Tasks(name) | Row::Free(name) => format!("# {name}"),
                Row::Event { when, title, .. } => format!("{when} {title}"),
                Row::Task { title, .. } => format!("- {title}"),
            })
            .collect();
        assert_eq!(
            text,
            [
                "# Hoje",
                "Dia inteiro Feriado",
                "10:00 Daily",
                "# Amanhã",
                "06:00 Workout",
                "# sex., 30/10",
                "10:00 Revisão",
                "# Tarefas",
                "- Ferro",
                "- Própolis",
            ]
        );
        // Só reuniões: no tasks.
        let meetings = rows_without_tasks(&refs);
        assert!(!meetings.iter().any(|row| matches!(row, Row::Tasks(_))));
    }

    fn rows_without_tasks(events: &[&Event]) -> Vec<Row> {
        rows(
            events,
            &[],
            false,
            range(CardPeriod::Day, MORNING, BRT),
            MORNING,
            BRT,
            &words(),
        )
    }

    #[test]
    fn an_empty_span_says_so() {
        assert_eq!(rows_without_tasks(&[]), [Row::Free("Nada marcado".into())]);
    }

    #[test]
    fn the_tasks_keep_the_order_given_in_the_panel() {
        let tasks = [task("Regar"), task("Ferro"), task("Própolis")];
        let order = vec!["Própolis".to_owned()];
        let titles: Vec<_> = ordered(&tasks, &order)
            .iter()
            .map(|task| task.title.as_str())
            .collect();
        assert_eq!(titles, ["Própolis", "Regar", "Ferro"]);
    }

    #[test]
    fn the_card_scrolls_past_its_height_and_finds_its_rows() {
        let rows: Vec<Row> = (0..20)
            .map(|n| Row::Task {
                title: n.to_string(),
                late: false,
            })
            .collect();
        let (tops, content) = tops(&rows);
        assert_eq!(tops[1], PAD + LINE);
        assert_eq!(content, PAD * 2.0 + 20.0 * LINE);
        assert_eq!(height(content), MAX_HEIGHT);
        assert_eq!(clamp_scroll(-5.0, content), 0.0);
        assert_eq!(clamp_scroll(10_000.0, content), content - MAX_HEIGHT);
        assert_eq!(hit(&rows, PAD + 1.0, 0.0), Some(0));
        assert_eq!(hit(&rows, PAD + 1.0, LINE), Some(1));
        assert_eq!(hit(&rows, 1.0, 0.0), None);
    }
}
