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
}

impl Default for CycleConfig {
    fn default() -> Self {
        CycleConfig {
            symbol: SYMBOL.to_owned(),
            calendar: None,
            sensitive: 5,
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
        if symbol_ok && calendar_ok && SENSITIVE.contains(&self.sensitive) {
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
        let hers = with("🌸", Some("noiva@gmail.com"), 5);
        assert!(hers.reads("noiva@gmail.com"));
        assert!(!hers.reads("max@gmail.com"));
        assert!(CycleConfig::default().reads("any"));
    }
}
