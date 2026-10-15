//! Time progress widget config (spec 0003 §2.2, Time progress): which
//! periods to show and in what order, when the week starts and the workday
//! hours.

use serde::{Deserialize, Serialize};

use super::WidgetError;

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
    /// Checks the periods (at least one, no repeats) and the workday hours.
    pub fn validate(&self) -> Result<(), WidgetError> {
        // L2: the validation lane implements this (periods and workday);
        // until then every config is accepted.
        Ok(())
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
