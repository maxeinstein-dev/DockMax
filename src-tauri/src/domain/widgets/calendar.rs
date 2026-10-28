//! The Calendário's config (spec 0017 F2a): its width, and the calendars it
//! leaves out. Every calendar of the connected accounts shows unless it is
//! listed here, so a calendar shared later shows by itself.

use serde::{Deserialize, Serialize};

use super::WidgetError;
/// The widths the Calendário takes, in icons: three was too small to
/// read (the author, 2026-10-28); two grow to three under the pointer.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=2;

/// Most calendars a config can hide.
pub const MAX_HIDDEN: usize = 200;

/// Longest calendar id kept (Google's are far shorter).
pub const MAX_ID_LEN: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CalendarConfig {
    /// In icons, 1 or 2: the time of the next meeting; the time and the
    /// title. Two icons grow to three under the pointer, with the meeting
    /// after it.
    pub width: u8,
    /// The ids of the calendars not shown, as Google names them.
    pub hidden: Vec<String>,
    /// Whether the panel shows Google Tasks' tasks (F2a-3). A config
    /// stored before it reads as `true`.
    pub tasks: bool,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        CalendarConfig {
            width: 2,
            hidden: Vec::new(),
            tasks: true,
        }
    }
}

impl CalendarConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if !WIDTHS.contains(&self.width) {
            return Err(WidgetError::InvalidWidth);
        }
        let ids_ok = self.hidden.len() <= MAX_HIDDEN
            && self.hidden.iter().all(|id| {
                !id.is_empty() && id.len() <= MAX_ID_LEN && !id.chars().any(char::is_control)
            });
        if ids_ok {
            Ok(())
        } else {
            Err(WidgetError::InvalidCalendars)
        }
    }

    /// Whether the calendar `id` shows.
    pub fn shows(&self, id: &str) -> bool {
        !self.hidden.iter().any(|hidden| hidden == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_calendar_config_takes_a_width_and_hidden_calendars() {
        let config = CalendarConfig {
            width: 2,
            hidden: vec!["pt-br.brazilian#holiday@group.v.calendar.google.com".into()],
            tasks: false,
        };
        assert_eq!(config.validate(), Ok(()));
        assert!(!config.shows("pt-br.brazilian#holiday@group.v.calendar.google.com"));
        assert!(config.shows("max@example.com"));

        let wide = CalendarConfig {
            width: 3,
            ..CalendarConfig::default()
        };
        assert_eq!(wide.validate(), Err(WidgetError::InvalidWidth));
        for bad in ["", "a\nb"] {
            let config = CalendarConfig {
                hidden: vec![bad.into()],
                ..CalendarConfig::default()
            };
            assert_eq!(config.validate(), Err(WidgetError::InvalidCalendars));
        }
        let many = CalendarConfig {
            hidden: (0..=MAX_HIDDEN).map(|n| n.to_string()).collect(),
            ..CalendarConfig::default()
        };
        assert_eq!(many.validate(), Err(WidgetError::InvalidCalendars));
    }
}
