//! Time progress widget config (spec 0003 §2.2, Time progress): which
//! periods to show and in what order, when the week starts and the workday
//! hours.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// Most periods a config lists: each [`Period`] at most once.
pub const MAX_PERIODS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimeProgressConfig {
    /// Periods in the order the panel shows them; the dock icon's ring shows
    /// the first one.
    pub periods: Vec<Period>,
    pub week_start: WeekStart,
    /// Used only when `periods` has [`Period::Workday`].
    pub workday: Workday,
}

impl Default for TimeProgressConfig {
    fn default() -> Self {
        TimeProgressConfig {
            periods: vec![Period::Day, Period::Week, Period::Month, Period::Year],
            week_start: WeekStart::default(),
            workday: Workday::default(),
        }
    }
}

impl TimeProgressConfig {
    /// Checks the periods (1 to [`MAX_PERIODS`], no repeats), then the
    /// workday hours. The workday is checked even when `periods` doesn't
    /// list [`Period::Workday`], so adding it later never meets bad hours.
    /// `week_start` is closed by its type.
    pub fn validate(&self) -> Result<(), WidgetError> {
        let count = self.periods.len();
        let mut seen = HashSet::with_capacity(count);
        let repeats = !self.periods.iter().all(|period| seen.insert(*period));
        if !(1..=MAX_PERIODS).contains(&count) || repeats {
            return Err(WidgetError::InvalidPeriods);
        }
        self.workday.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Day,
    Week,
    Month,
    Year,
    /// The workday hours, Monday to Friday.
    Workday,
}

/// First day of the week for [`Period::Week`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

/// Workday hours, both on the same day, as local `HH:mm` text. Kept as text
/// so a config edited by hand still reads; the format is a validation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workday {
    pub start: String,
    pub end: String,
}

impl Default for Workday {
    fn default() -> Self {
        Workday {
            start: "09:00".to_owned(),
            end: "18:00".to_owned(),
        }
    }
}

impl Workday {
    /// Both times in strict `HH:mm` (two ASCII digits each, `00:00` to
    /// `23:59`) and the start before the end, on the same day.
    pub fn validate(&self) -> Result<(), WidgetError> {
        match (minutes_of_day(&self.start), minutes_of_day(&self.end)) {
            (Some(start), Some(end)) if start < end => Ok(()),
            _ => Err(WidgetError::InvalidWorkday),
        }
    }
}

/// Minutes since midnight of a strict `HH:mm` time; `None` for any other
/// text (`9:00`, `09:00:00`, `24:00`, `09:60`, full-width digits...).
fn minutes_of_day(text: &str) -> Option<u16> {
    let [h1, h2, b':', m1, m2] = *text.as_bytes() else {
        return None;
    };
    let digit = |byte: u8| byte.is_ascii_digit().then(|| u16::from(byte - b'0'));
    let hours = digit(h1)? * 10 + digit(h2)?;
    let minutes = digit(m1)? * 10 + digit(m2)?;
    (hours < 24 && minutes < 60).then_some(hours * 60 + minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_periods(periods: &[Period]) -> TimeProgressConfig {
        TimeProgressConfig {
            periods: periods.to_vec(),
            ..TimeProgressConfig::default()
        }
    }

    fn with_workday(start: &str, end: &str) -> TimeProgressConfig {
        TimeProgressConfig {
            workday: Workday {
                start: start.to_owned(),
                end: end.to_owned(),
            },
            ..TimeProgressConfig::default()
        }
    }

    #[test]
    fn default_is_valid() {
        assert_eq!(TimeProgressConfig::default().validate(), Ok(()));
    }

    #[test]
    fn one_to_five_distinct_periods_in_any_order_are_accepted() {
        let all = [
            Period::Workday,
            Period::Year,
            Period::Month,
            Period::Week,
            Period::Day,
        ];
        assert_eq!(all.len(), MAX_PERIODS);
        for count in 1..=MAX_PERIODS {
            assert_eq!(with_periods(&all[..count]).validate(), Ok(()), "{count}");
        }
    }

    #[test]
    fn no_period_is_rejected() {
        assert_eq!(
            with_periods(&[]).validate(),
            Err(WidgetError::InvalidPeriods)
        );
    }

    #[test]
    fn a_repeated_period_is_rejected() {
        let repeated: [&[Period]; 3] = [
            &[Period::Day, Period::Day],
            &[Period::Day, Period::Week, Period::Day],
            // Six items: over the limit and a repeat.
            &[
                Period::Day,
                Period::Week,
                Period::Month,
                Period::Year,
                Period::Workday,
                Period::Day,
            ],
        ];
        for periods in repeated {
            assert_eq!(
                with_periods(periods).validate(),
                Err(WidgetError::InvalidPeriods),
                "{periods:?}"
            );
        }
    }

    #[test]
    fn workday_limits_are_accepted() {
        let accepted = [
            ("00:00", "23:59"),
            ("09:00", "09:01"),
            ("23:58", "23:59"),
            ("08:30", "17:15"),
        ];
        for (start, end) in accepted {
            assert_eq!(with_workday(start, end).validate(), Ok(()), "{start}-{end}");
        }
    }

    #[test]
    fn workday_not_in_strict_hh_mm_is_rejected() {
        let malformed = [
            "",
            "9:00",
            "09:0",
            "0900",
            "09.00",
            "09h00",
            "09:00:00",
            "24:00",
            "25:00",
            "99:99",
            "09:60",
            "-1:00",
            " 9:00",
            "09:00 ",
            "+9:00",
            "ab:cd",
            "\u{FF10}\u{FF19}:\u{FF10}\u{FF10}", // full-width digits
            "09\u{FF1A}00",                      // full-width colon
        ];
        for text in malformed {
            assert_eq!(
                with_workday(text, "23:59").validate(),
                Err(WidgetError::InvalidWorkday),
                "start {text:?}"
            );
            assert_eq!(
                with_workday("00:00", text).validate(),
                Err(WidgetError::InvalidWorkday),
                "end {text:?}"
            );
        }
    }

    #[test]
    fn workday_start_must_be_before_the_end() {
        let rejected = [("09:00", "09:00"), ("18:00", "09:00"), ("23:59", "00:00")];
        for (start, end) in rejected {
            assert_eq!(
                with_workday(start, end).validate(),
                Err(WidgetError::InvalidWorkday),
                "{start}-{end}"
            );
        }
    }

    #[test]
    fn workday_is_checked_even_when_not_listed() {
        let config = TimeProgressConfig {
            periods: vec![Period::Day],
            ..with_workday("18:00", "09:00")
        };
        assert_eq!(config.validate(), Err(WidgetError::InvalidWorkday));
    }

    #[test]
    fn periods_are_checked_before_the_workday() {
        let config = TimeProgressConfig {
            periods: vec![],
            ..with_workday("24:00", "09:00")
        };
        assert_eq!(config.validate(), Err(WidgetError::InvalidPeriods));
    }
}
