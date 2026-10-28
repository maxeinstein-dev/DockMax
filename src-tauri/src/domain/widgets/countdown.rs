//! The Contagem regressiva (spec 0017 F2c): the days to a date. Its name
//! is the widget's label ("Casamento", "Férias"), so the config holds only
//! the date and whether it comes back every year (a birthday).

use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::WidgetError;

/// The longest a date may be from today, either way: 100 years.
const MAX_DAYS: i64 = 100 * 366;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CountdownConfig {
    /// The day, as `2026-12-20`; `None` until the author picks one.
    pub date: Option<String>,
    /// It comes back every year: the count goes to the next one.
    pub yearly: bool,
}

impl CountdownConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        match &self.date {
            None => Ok(()),
            Some(text) if parse(text).is_some() => Ok(()),
            Some(_) => Err(WidgetError::InvalidCountdown),
        }
    }
}

/// `2026-12-20`, and only that form.
pub fn parse(text: &str) -> Option<Date> {
    let date: Date = text.parse().ok()?;
    // jiff takes other forms too; only the one the settings write counts.
    (date.to_string() == text).then_some(date)
}

fn epoch() -> Date {
    jiff::civil::date(1970, 1, 1)
}

/// A local day number (days since 1970-01-01) as a date.
fn date_of(day: i64) -> Option<Date> {
    epoch()
        .checked_add(jiff::Span::new().try_days(day).ok()?)
        .ok()
}

/// A date as a local day number.
fn day_of(date: Date) -> Option<i64> {
    date.since(epoch())
        .ok()
        .map(|span| i64::from(span.get_days()))
}

/// The day of `date` in `year`: the 29th of February is the 28th in the
/// years without one.
fn in_year(date: Date, year: i16) -> Option<Date> {
    Date::new(year, date.month(), date.day())
        .or_else(|_| Date::new(year, date.month(), date.day() - 1))
        .ok()
}

/// The day the count goes to, as a local day number, from `today`: the date,
/// or its next return when yearly (today included). `None` without a date.
pub fn target(config: &CountdownConfig, today: i64) -> Option<i64> {
    let date = parse(config.date.as_deref()?)?;
    if !config.yearly {
        return day_of(date);
    }
    let now = date_of(today)?;
    let this = in_year(date, now.year())?;
    let next = if this < now {
        in_year(date, now.year().checked_add(1)?)?
    } else {
        this
    };
    day_of(next)
}

/// Days from `today` to the date: 0 on the day, negative once it is gone.
/// `None` without a date, or one too far to count.
pub fn days_left(config: &CountdownConfig, today: i64) -> Option<i64> {
    let left = target(config, today)? - today;
    (left.abs() <= MAX_DAYS).then_some(left)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> i64 {
        day_of(parse(text).unwrap()).unwrap()
    }

    fn config(date: &str, yearly: bool) -> CountdownConfig {
        CountdownConfig {
            date: Some(date.into()),
            yearly,
        }
    }

    #[test]
    fn only_a_real_day_in_the_settings_form_is_taken() {
        assert_eq!(config("2026-12-20", false).validate(), Ok(()));
        assert_eq!(CountdownConfig::default().validate(), Ok(()));
        for bad in [
            "2026-02-30",
            "20/12/2026",
            "2026-12-20T00:00",
            "",
            "2026-1-5",
        ] {
            assert_eq!(
                config(bad, false).validate(),
                Err(WidgetError::InvalidCountdown),
                "{bad}"
            );
        }
    }

    #[test]
    fn it_counts_the_days_left_and_gone() {
        let today = day("2026-10-28");
        assert_eq!(days_left(&config("2026-12-20", false), today), Some(53));
        assert_eq!(days_left(&config("2026-10-28", false), today), Some(0));
        assert_eq!(days_left(&config("2026-10-25", false), today), Some(-3));
        assert_eq!(days_left(&CountdownConfig::default(), today), None);
        assert_eq!(days_left(&config("2200-01-01", false), today), None);
    }

    #[test]
    fn a_yearly_date_counts_to_its_next_return() {
        let today = day("2026-10-28");
        // Gone this year: next year's.
        assert_eq!(days_left(&config("1990-03-10", true), today), Some(133));
        // Still to come this year, and today itself.
        assert_eq!(days_left(&config("1990-11-02", true), today), Some(5));
        assert_eq!(days_left(&config("1990-10-28", true), today), Some(0));
        // The 29th of February on the 28th in 2027.
        assert_eq!(
            target(&config("2024-02-29", true), today),
            Some(day("2027-02-28"))
        );
        assert_eq!(
            target(&config("2024-02-29", true), day("2028-01-10")),
            Some(day("2028-02-29"))
        );
    }
}
