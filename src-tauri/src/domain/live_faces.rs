//! What the live tiles of the dock show (spec 0003 §2.2, DESIGN.md "Ícones
//! vivos"; the page's `formatDockTime` and `progressOf`): the clock's time
//! and how far the first period of a time progress is. Pure: the time comes
//! in.

use super::widgets::clock::HourCycle;
use super::widgets::time_progress::{Period, TimeProgressConfig, WeekStart};

/// A reading of the wall clock: the local calendar fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wall {
    pub year: i32,
    /// 1–12.
    pub month: u8,
    /// 1–31.
    pub day: u8,
    /// 0 is Sunday, as `Date.getDay()` counts.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// The clock tile's text: hours and minutes, never seconds; `HH:mm` in 24 h,
/// `h:mm` in 12 h without AM/PM (the period is in the panel).
pub fn clock_text(hour: u8, minute: u8, cycle: HourCycle) -> String {
    match cycle {
        HourCycle::H23 => format!("{hour:02}:{minute:02}"),
        HourCycle::H12 => {
            let twelve = match hour % 12 {
                0 => 12,
                other => other,
            };
            format!("{twelve}:{minute:02}")
        }
    }
}

fn leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u8) -> u32 {
    match month {
        2 if leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `HH:mm` → minutes after midnight, or `None` when malformed.
fn minutes_of(text: &str) -> Option<u32> {
    let (hours, minutes) = text.split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    (hours <= 23 && minutes <= 59).then_some(hours * 60 + minutes)
}

/// How far `wall` is into `period`, 0 to 1; `None` for the workday outside
/// its hours or on a weekend. Counted in wall-clock time: on a day with a
/// daylight-saving change the share is off by that hour (Brazil has none).
pub fn progress(period: Period, wall: &Wall, config: &TimeProgressConfig) -> Option<f64> {
    const DAY: f64 = 86_400.0;
    let seconds =
        f64::from(wall.hour) * 3600.0 + f64::from(wall.minute) * 60.0 + f64::from(wall.second);
    let fraction = match period {
        Period::Day => seconds / DAY,
        Period::Week => {
            let first = match config.week_start {
                WeekStart::Monday => 1,
                WeekStart::Sunday => 0,
            };
            let back = (u32::from(wall.weekday) + 7 - first) % 7;
            (f64::from(back) * DAY + seconds) / (7.0 * DAY)
        }
        Period::Month => {
            let days = days_in_month(wall.year, wall.month);
            (f64::from(u32::from(wall.day) - 1) * DAY + seconds) / (f64::from(days) * DAY)
        }
        Period::Year => {
            let before: u32 = (1..wall.month)
                .map(|month| days_in_month(wall.year, month))
                .sum::<u32>()
                + u32::from(wall.day)
                - 1;
            let days = if leap(wall.year) { 366.0 } else { 365.0 };
            (f64::from(before) * DAY + seconds) / (days * DAY)
        }
        Period::Workday => {
            if wall.weekday == 0 || wall.weekday == 6 {
                return None;
            }
            let from = f64::from(minutes_of(&config.workday.start)?) * 60.0;
            let to = f64::from(minutes_of(&config.workday.end)?) * 60.0;
            if from >= to || seconds < from || seconds > to {
                return None;
            }
            (seconds - from) / (to - from)
        }
    };
    Some(fraction.clamp(0.0, 1.0))
}

/// The percentage the tile shows: rounded down, so 100 % only at the end.
pub fn percent(fraction: f64) -> u32 {
    (fraction * 100.0).floor().clamp(0.0, 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::widgets::time_progress::Workday;

    fn wall(year: i32, month: u8, day: u8, weekday: u8, hour: u8, minute: u8) -> Wall {
        Wall {
            year,
            month,
            day,
            weekday,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn the_clock_pads_24_hours_and_not_12() {
        assert_eq!(clock_text(9, 5, HourCycle::H23), "09:05");
        assert_eq!(clock_text(0, 0, HourCycle::H23), "00:00");
        assert_eq!(clock_text(9, 5, HourCycle::H12), "9:05");
        assert_eq!(clock_text(0, 30, HourCycle::H12), "12:30");
        assert_eq!(clock_text(13, 0, HourCycle::H12), "1:00");
        assert_eq!(clock_text(12, 0, HourCycle::H12), "12:00");
    }

    #[test]
    fn the_day_is_half_gone_at_noon() {
        let config = TimeProgressConfig::default();
        let noon = wall(2026, 10, 15, 4, 12, 0);

        assert_eq!(progress(Period::Day, &noon, &config), Some(0.5));
    }

    #[test]
    fn the_week_starts_on_the_day_the_config_says() {
        let monday_start = TimeProgressConfig::default();
        let sunday_start = TimeProgressConfig {
            week_start: WeekStart::Sunday,
            ..TimeProgressConfig::default()
        };
        // Thursday at midnight.
        let thursday = wall(2026, 10, 15, 4, 0, 0);

        assert_eq!(
            progress(Period::Week, &thursday, &monday_start),
            Some(3.0 / 7.0)
        );
        assert_eq!(
            progress(Period::Week, &thursday, &sunday_start),
            Some(4.0 / 7.0)
        );
        // Sunday is the end of a week that starts on Monday.
        let sunday = wall(2026, 10, 18, 0, 0, 0);
        assert_eq!(
            progress(Period::Week, &sunday, &monday_start),
            Some(6.0 / 7.0)
        );
    }

    #[test]
    fn the_month_and_the_year_count_their_real_days() {
        let config = TimeProgressConfig::default();

        // February of a leap year: the 15th at midnight is 14 of 29 days.
        let february = wall(2028, 2, 15, 2, 0, 0);
        assert_eq!(
            progress(Period::Month, &february, &config),
            Some(14.0 / 29.0)
        );
        // At midnight on 2 July 2026, 182 of 365 days are gone.
        let july = wall(2026, 7, 2, 4, 0, 0);
        assert_eq!(progress(Period::Year, &july, &config), Some(182.0 / 365.0));
        let first = wall(2026, 1, 1, 4, 0, 0);
        assert_eq!(progress(Period::Year, &first, &config), Some(0.0));
    }

    #[test]
    fn the_workday_runs_on_weekdays_within_its_hours() {
        let config = TimeProgressConfig {
            workday: Workday {
                start: "09:00".into(),
                end: "17:00".into(),
            },
            ..TimeProgressConfig::default()
        };

        assert_eq!(
            progress(Period::Workday, &wall(2026, 10, 15, 4, 13, 0), &config),
            Some(0.5)
        );
        assert_eq!(
            progress(Period::Workday, &wall(2026, 10, 15, 4, 17, 0), &config),
            Some(1.0)
        );
        assert_eq!(
            progress(Period::Workday, &wall(2026, 10, 15, 4, 8, 59), &config),
            None
        );
        assert_eq!(
            progress(Period::Workday, &wall(2026, 10, 17, 6, 13, 0), &config),
            None
        );
        let broken = TimeProgressConfig {
            workday: Workday {
                start: "9h".into(),
                end: "17:00".into(),
            },
            ..TimeProgressConfig::default()
        };
        assert_eq!(
            progress(Period::Workday, &wall(2026, 10, 15, 4, 13, 0), &broken),
            None
        );
    }

    #[test]
    fn the_percentage_is_rounded_down() {
        assert_eq!(percent(0.999), 99);
        assert_eq!(percent(1.0), 100);
        assert_eq!(percent(0.0), 0);
        assert_eq!(percent(0.4567), 45);
    }
}
