//! The Calendário's config (spec 0017 F2a): its width, and the calendars it
//! leaves out. Every calendar of the connected accounts shows unless it is
//! listed here, so a calendar shared later shows by itself.

use serde::{Deserialize, Serialize};

use super::WidgetError;
/// The widths the Calendário takes, in icons: three was too small to
/// read (the author, 2026-10-28); two grow to three under the pointer.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=2;

/// How far ahead the card looks (the widget's settings).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CardPeriod {
    /// The day; after 18h, tomorrow until noon.
    #[default]
    Auto,
    Day,
    TwoDays,
    ThreeDays,
    Week,
}

closed_set!(CardPeriod {
    Auto,
    Day,
    TwoDays,
    ThreeDays,
    Week,
});

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
    /// The tasks' order the author gave in the panel, by title (a task
    /// that repeats every day keeps its title): these first, in this
    /// order, then the others in Google's.
    pub task_order: Vec<String>,
    /// The card over the dock (F2a-4): how far ahead it looks, and
    /// whether it lists the day's tasks under the meetings.
    pub card_period: CardPeriod,
    pub card_tasks: bool,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        CalendarConfig {
            width: 2,
            hidden: Vec::new(),
            tasks: true,
            task_order: Vec::new(),
            card_period: CardPeriod::Auto,
            card_tasks: true,
        }
    }
}

impl CalendarConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if !WIDTHS.contains(&self.width) {
            return Err(WidgetError::InvalidWidth);
        }
        let fits = |list: &[String]| {
            list.len() <= MAX_HIDDEN
                && list.iter().all(|text| {
                    !text.is_empty()
                        && text.len() <= MAX_ID_LEN
                        && !text.chars().any(char::is_control)
                })
        };
        if fits(&self.hidden) && fits(&self.task_order) {
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
            task_order: vec!["🪴 Regar Mini Samambaia".into(), "😷 Própolis".into()],
            card_period: CardPeriod::Week,
            card_tasks: false,
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
        let broken_order = CalendarConfig {
            task_order: vec!["a\tb".into()],
            ..CalendarConfig::default()
        };
        assert_eq!(broken_order.validate(), Err(WidgetError::InvalidCalendars));
    }
}
