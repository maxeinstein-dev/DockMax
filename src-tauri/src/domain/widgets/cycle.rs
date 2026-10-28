//! The Ciclo's config (spec 0017 F2b): the symbol she marks the first day
//! with, the calendar it is in (all of them when none is chosen) and how
//! many days before the next start the sensitive window opens.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// The symbol by default.
pub const SYMBOL: &str = "🌸";

/// Longest symbol, in Unicode scalar values.
pub const MAX_SYMBOL_CHARS: usize = 16;

/// The sensitive window, in days before the next start.
pub const SENSITIVE: std::ops::RangeInclusive<u8> = 1..=10;

/// Most reminders, the longest text of one, and how far from its phase.
pub const MAX_REMINDERS: usize = 10;
pub const MAX_REMINDER_CHARS: usize = 120;
pub const REMINDER_DAYS: std::ops::RangeInclusive<i8> = -10..=10;

/// The Windows notices the author keeps (F2b-2): when the sensitive window
/// opens, the eve of the expected start, and the day itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Notices {
    pub sensitive: bool,
    pub eve: bool,
    pub day: bool,
}

impl Default for Notices {
    fn default() -> Self {
        Notices {
            sensitive: true,
            eve: true,
            day: true,
        }
    }
}

/// What a reminder counts from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Anchor {
    /// The day the sensitive window opens.
    #[default]
    Sensitive,
    /// The expected first day.
    Period,
}

closed_set!(Anchor { Sensitive, Period });

/// A reminder of the author's own: `text`, `days` before (negative) or
/// after its `anchor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub text: String,
    pub anchor: Anchor,
    pub days: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CycleConfig {
    /// What the title of each mark has: `🌸` by default.
    pub symbol: String,
    /// The calendar to look in, as Google names it; `None` looks in every
    /// calendar of the accounts.
    pub calendar: Option<String>,
    /// Days before the next start that the sensitive window opens; 5 by
    /// default, changed for the months ahead.
    pub sensitive: u8,
    /// The Windows notices kept (F2b-2).
    pub notices: Notices,
    /// The author's own reminders (F2b-2).
    pub reminders: Vec<Reminder>,
}

impl Default for CycleConfig {
    fn default() -> Self {
        CycleConfig {
            symbol: SYMBOL.to_owned(),
            calendar: None,
            sensitive: 5,
            notices: Notices::default(),
            reminders: Vec::new(),
        }
    }
}

impl CycleConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        let symbol = self.symbol.trim();
        let symbol_ok = !symbol.is_empty()
            && symbol.chars().count() <= MAX_SYMBOL_CHARS
            && !symbol.chars().any(char::is_control);
        let calendar_ok = self.calendar.as_deref().is_none_or(|id| {
            !id.is_empty()
                && id.len() <= super::calendar::MAX_ID_LEN
                && !id.chars().any(char::is_control)
        });
        let reminders_ok = self.reminders.len() <= MAX_REMINDERS
            && self.reminders.iter().all(|reminder| {
                let text = reminder.text.trim();
                !text.is_empty()
                    && text.chars().count() <= MAX_REMINDER_CHARS
                    && !text.chars().any(char::is_control)
                    && REMINDER_DAYS.contains(&reminder.days)
            });
        if symbol_ok && calendar_ok && reminders_ok && SENSITIVE.contains(&self.sensitive) {
            Ok(())
        } else {
            Err(WidgetError::InvalidCycle)
        }
    }

    /// Whether a mark in the calendar `id` counts.
    pub fn reads(&self, id: &str) -> bool {
        self.calendar
            .as_deref()
            .is_none_or(|calendar| calendar == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cycle_config_checks_its_symbol_calendar_and_window() {
        assert_eq!(CycleConfig::default().validate(), Ok(()));
        let with = |symbol: &str, calendar: Option<&str>, sensitive: u8| CycleConfig {
            symbol: symbol.into(),
            calendar: calendar.map(str::to_owned),
            sensitive,
            ..CycleConfig::default()
        };
        assert_eq!(
            with("Ciclo 🌸", Some("noiva@gmail.com"), 3).validate(),
            Ok(())
        );
        for bad in [
            with("", None, 5),
            with("  ", None, 5),
            with("a\nb", None, 5),
            with(&"x".repeat(17), None, 5),
            with("🌸", Some(""), 5),
            with("🌸", None, 0),
            with("🌸", None, 11),
        ] {
            assert_eq!(bad.validate(), Err(WidgetError::InvalidCycle), "{bad:?}");
        }
        let reminder = |text: &str, days: i8| CycleConfig {
            reminders: vec![Reminder {
                text: text.into(),
                anchor: Anchor::Sensitive,
                days,
            }],
            ..CycleConfig::default()
        };
        assert_eq!(reminder("Comprar chocolate", -1).validate(), Ok(()));
        for bad in [
            reminder("", 0),
            reminder("a\tb", 0),
            reminder("x", 11),
            reminder(&"x".repeat(121), 0),
        ] {
            assert_eq!(bad.validate(), Err(WidgetError::InvalidCycle));
        }
        let hers = with("🌸", Some("noiva@gmail.com"), 5);
        assert!(hers.reads("noiva@gmail.com"));
        assert!(!hers.reads("max@gmail.com"));
        assert!(CycleConfig::default().reads("any"));
    }
}
