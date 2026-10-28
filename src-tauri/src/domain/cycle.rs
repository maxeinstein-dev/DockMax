//! The Ciclo (spec 0017 F2b): the menstrual cycle of the author's fiancée,
//! read from her calendar, where she marks each first day with an all-day
//! event (`🌸` by default). From the marks of the last year, the average
//! cycle and period, the day of the cycle, and the next start. Pure, and
//! kept in memory only: nothing of hers goes to the disk.

use serde::Serialize;

const DAY: i64 = 24 * 3600;

/// Without enough marks, a cycle of 28 days and a period of 5.
pub const DEFAULT_CYCLE: f64 = 28.0;
pub const DEFAULT_PERIOD: f64 = 5.0;
/// How many of the last cycles the average takes.
pub const AVERAGE_OF: usize = 6;
/// Gaps outside this many days are not cycles (a mark missed, or two marks
/// of one period) and are left out of the average.
const CYCLE_RANGE: std::ops::RangeInclusive<i64> = 15..=60;
/// A period is 1 to 10 days long.
const PERIOD_RANGE: std::ops::RangeInclusive<i64> = 1..=10;

/// A mark she made: the event's start and end, Unix seconds (all-day: local
/// midnights).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    pub start: i64,
    pub end: i64,
}

/// A period, as local day numbers (days since 1970-01-01, local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub first: i64,
    /// Days long.
    pub days: i64,
}

/// The local day number of `at`.
pub fn day_of(at: i64, offset: i64) -> i64 {
    (at + offset).div_euclid(DAY)
}

/// The marks as periods, oldest first: marks that start within the same
/// period (the event in two calendars, or one per day) are one period.
pub fn periods(marks: &[Mark], offset: i64) -> Vec<Period> {
    let mut days: Vec<(i64, i64)> = marks
        .iter()
        .map(|mark| {
            let first = day_of(mark.start, offset);
            // An all-day event ends at the next midnight.
            let last = day_of(mark.end - 1, offset).max(first);
            (
                first,
                (last - first + 1).clamp(*PERIOD_RANGE.start(), *PERIOD_RANGE.end()),
            )
        })
        .collect();
    days.sort();
    let mut periods: Vec<Period> = Vec::new();
    for (first, length) in days {
        match periods.last_mut() {
            Some(last) if first < last.first + last.days.max(*CYCLE_RANGE.start() / 2) => {
                // The same period, marked again: the longest stretch counts.
                last.days = last.days.max(first + length - last.first);
            }
            _ => periods.push(Period {
                first,
                days: length,
            }),
        }
    }
    periods
}

/// The averages, from the last [`AVERAGE_OF`] cycles.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Averages {
    pub cycle: f64,
    pub period: f64,
    /// How many cycles the average took (0: the defaults).
    pub cycles: usize,
}

pub fn averages(periods: &[Period]) -> Averages {
    let gaps: Vec<i64> = periods
        .windows(2)
        .map(|pair| pair[1].first - pair[0].first)
        .filter(|gap| CYCLE_RANGE.contains(gap))
        .collect();
    let recent = &gaps[gaps.len().saturating_sub(AVERAGE_OF)..];
    let cycle = if recent.is_empty() {
        DEFAULT_CYCLE
    } else {
        recent.iter().sum::<i64>() as f64 / recent.len() as f64
    };
    // A period marked as one day only says when it started.
    let lengths: Vec<i64> = periods
        .iter()
        .rev()
        .take(AVERAGE_OF)
        .map(|period| period.days)
        .filter(|days| *days > 1)
        .collect();
    let period = if lengths.is_empty() {
        DEFAULT_PERIOD
    } else {
        lengths.iter().sum::<i64>() as f64 / lengths.len() as f64
    };
    Averages {
        cycle,
        period,
        cycles: recent.len(),
    }
}

/// Where the cycle is today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum Phase {
    /// No mark yet.
    Unknown,
    /// The period: its `day` (1 is the first).
    Period { day: i64 },
    /// The days before the next start, within the sensitive window: `in`
    /// days to it.
    Sensitive {
        #[serde(rename = "in")]
        days: i64,
    },
    /// The rest of the cycle: `in` days to the next start (0 or less: due,
    /// not marked yet).
    Between {
        #[serde(rename = "in")]
        days: i64,
    },
}

/// The cycle seen today.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    pub averages: Averages,
    /// Today's day of the cycle (1 on a first day); 0 without marks.
    pub day: i64,
    /// The last first day and the next one expected, local day numbers.
    pub last: Option<i64>,
    pub next: Option<i64>,
    /// How many periods were read.
    pub marks: usize,
}

/// The cycle on `today` (a local day number), with the sensitive window
/// `sensitive` days before the next start.
pub fn status(periods: &[Period], today: i64, sensitive: i64) -> Status {
    let averages = averages(periods);
    // A mark in the future (made ahead) doesn't start the cycle yet.
    let Some(last) = periods.iter().rev().find(|period| period.first <= today) else {
        return Status {
            phase: Phase::Unknown,
            averages,
            day: 0,
            last: None,
            next: None,
            marks: periods.len(),
        };
    };
    let day = today - last.first + 1;
    let next = last.first + averages.cycle.round() as i64;
    let to_next = next - today;
    let length = if last.days > 1 {
        last.days
    } else {
        averages.period.round() as i64
    };
    let phase = if day <= length {
        Phase::Period { day }
    } else if to_next <= sensitive && to_next > 0 {
        Phase::Sensitive { days: to_next }
    } else {
        Phase::Between { days: to_next }
    };
    Status {
        phase,
        averages,
        day,
        last: Some(last.first),
        next: Some(next),
        marks: periods.len(),
    }
}

/// A notice of the Ciclo (F2b-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    /// The sensitive window opens today.
    Sensitive,
    /// The expected start is tomorrow.
    Eve,
    /// The expected start is today.
    Day,
    /// One of the author's reminders: its text.
    Reminder { text: String },
}

impl Notice {
    /// What tells this notice from the others of the day, for firing it
    /// once.
    pub fn key(&self) -> String {
        match self {
            Notice::Sensitive => "sensitive".into(),
            Notice::Eve => "eve".into(),
            Notice::Day => "day".into(),
            Notice::Reminder { text } => format!("reminder:{text}"),
        }
    }
}

/// The notices of `today` for a cycle seen as `status`, with the widget's
/// config: none without a prediction.
pub fn notices(
    status: &Status,
    config: &crate::domain::widgets::CycleConfig,
    today: i64,
) -> Vec<Notice> {
    use crate::domain::widgets::cycle::Anchor;
    let Some(next) = status.next.filter(|_| status.phase != Phase::Unknown) else {
        return Vec::new();
    };
    let opens = next - i64::from(config.sensitive);
    let mut notices = Vec::new();
    if config.notices.sensitive && today == opens {
        notices.push(Notice::Sensitive);
    }
    if config.notices.eve && today == next - 1 {
        notices.push(Notice::Eve);
    }
    if config.notices.day && today == next {
        notices.push(Notice::Day);
    }
    for reminder in &config.reminders {
        let anchor = match reminder.anchor {
            Anchor::Sensitive => opens,
            Anchor::Period => next,
        };
        if today == anchor + i64::from(reminder.days) {
            notices.push(Notice::Reminder {
                text: reminder.text.trim().to_owned(),
            });
        }
    }
    notices
}

/// The share of the cycle gone today, 0–1, for the tile's ring.
pub fn fraction(status: &Status) -> Option<f32> {
    (status.day > 0).then(|| (status.day as f64 / status.averages.cycle).clamp(0.0, 1.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRT: i64 = -3 * 3600;

    /// An all-day mark of `days` days starting on local day `first`.
    fn mark(first: i64, days: i64) -> Mark {
        Mark {
            start: first * DAY - BRT,
            end: (first + days) * DAY - BRT,
        }
    }

    #[test]
    fn marks_become_periods_and_repeats_count_once() {
        let marks = [
            mark(100, 5),
            mark(128, 1),
            mark(100, 1),
            mark(129, 1),
            mark(157, 4),
        ];
        assert_eq!(
            periods(&marks, BRT),
            [
                Period {
                    first: 100,
                    days: 5
                },
                Period {
                    first: 128,
                    days: 2
                },
                Period {
                    first: 157,
                    days: 4
                },
            ]
        );
    }

    #[test]
    fn the_averages_take_the_last_six_cycles_or_the_defaults() {
        assert_eq!(
            averages(&[]),
            Averages {
                cycle: 28.0,
                period: 5.0,
                cycles: 0
            }
        );
        let firsts = [0, 30, 59, 89, 118, 148, 177, 207];
        let periods: Vec<Period> = firsts
            .iter()
            .map(|&first| Period { first, days: 4 })
            .collect();
        let averages = averages(&periods);
        // The last six gaps: 29, 30, 29, 30, 29, 30.
        assert_eq!(averages.cycles, 6);
        assert!((averages.cycle - 29.5).abs() < 1e-9);
        assert_eq!(averages.period, 4.0);
        // A missed mark (a 60+ day gap) is not a cycle.
        let missed = [Period { first: 0, days: 5 }, Period { first: 70, days: 5 }];
        assert_eq!(averages_of(&missed).cycles, 0);
    }

    fn averages_of(periods: &[Period]) -> Averages {
        averages(periods)
    }

    #[test]
    fn the_phase_follows_the_day_of_the_cycle() {
        let periods = [
            Period {
                first: 100,
                days: 5,
            },
            Period {
                first: 128,
                days: 5,
            },
        ];
        // Cycle of 28; period of 5; sensitive window of 5 days.
        let on = |today| status(&periods, today, 5).phase;
        assert_eq!(on(128), Phase::Period { day: 1 });
        assert_eq!(on(132), Phase::Period { day: 5 });
        assert_eq!(on(133), Phase::Between { days: 23 });
        assert_eq!(on(151), Phase::Sensitive { days: 5 });
        assert_eq!(on(155), Phase::Sensitive { days: 1 });
        // Due and not marked: no warning, only the count.
        assert_eq!(on(156), Phase::Between { days: 0 });
        assert_eq!(on(158), Phase::Between { days: -2 });
        let today = status(&periods, 140, 5);
        assert_eq!(today.day, 13);
        assert_eq!(today.next, Some(156));
        assert_eq!(fraction(&today), Some(13.0 / 28.0));
    }

    #[test]
    fn the_notices_fall_on_their_days() {
        use crate::domain::widgets::CycleConfig;
        use crate::domain::widgets::cycle::{Anchor, Reminder};
        let periods = [
            Period {
                first: 100,
                days: 5,
            },
            Period {
                first: 128,
                days: 5,
            },
        ];
        // Next start: 156; the window of 5 days opens on 151.
        let config = CycleConfig {
            reminders: vec![
                Reminder {
                    text: " Comprar chocolate ".into(),
                    anchor: Anchor::Sensitive,
                    days: -1,
                },
                Reminder {
                    text: "Paciência".into(),
                    anchor: Anchor::Period,
                    days: 1,
                },
            ],
            ..CycleConfig::default()
        };
        let on = |today| notices(&status(&periods, today, 5), &config, today);
        assert_eq!(
            on(150),
            [Notice::Reminder {
                text: "Comprar chocolate".into()
            }]
        );
        assert_eq!(on(151), [Notice::Sensitive]);
        assert_eq!(on(155), [Notice::Eve]);
        assert_eq!(on(156), [Notice::Day]);
        assert!(on(140).is_empty());
        // Turned off: only the reminders.
        let quiet = CycleConfig {
            notices: crate::domain::widgets::cycle::Notices {
                sensitive: false,
                eve: false,
                day: false,
                windows: true,
            },
            ..config.clone()
        };
        assert!(notices(&status(&periods, 151, 5), &quiet, 151).is_empty());
        // Without marks, nothing.
        assert!(notices(&status(&[], 151, 5), &config, 151).is_empty());
    }

    #[test]
    fn without_marks_nothing_is_guessed() {
        let none = status(&[], 100, 5);
        assert_eq!(none.phase, Phase::Unknown);
        assert_eq!(fraction(&none), None);
        // A mark ahead of today doesn't count yet.
        let ahead = status(
            &[Period {
                first: 120,
                days: 5,
            }],
            100,
            5,
        );
        assert_eq!(ahead.phase, Phase::Unknown);
    }

    #[test]
    fn a_one_day_mark_lasts_the_average_period() {
        let periods = [
            Period {
                first: 100,
                days: 6,
            },
            Period {
                first: 128,
                days: 1,
            },
        ];
        // Only the 6-day period says how long they last.
        assert_eq!(status(&periods, 133, 5).phase, Phase::Period { day: 6 });
        assert_eq!(status(&periods, 134, 5).phase, Phase::Between { days: 22 });
    }
}
