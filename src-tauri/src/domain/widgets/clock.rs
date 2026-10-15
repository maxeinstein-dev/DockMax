//! Clock widget config (spec 0003 §2.2, Clock): which time zone, 24 h or
//! 12 h, and whether the panel shows seconds.

use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClockConfig {
    /// IANA time zone name, such as `America/Sao_Paulo`; `None` follows the
    /// system time zone. Serialized as `null`, never left out, so the
    /// interface always gets the field.
    pub time_zone: Option<String>,
    pub hour_cycle: HourCycle,
    /// Seconds in the panel only; the dock icon never shows them.
    pub show_seconds: bool,
}

impl ClockConfig {
    /// Checks the time zone name against the IANA database.
    pub fn validate(&self) -> Result<(), WidgetError> {
        // L2: the validation lane implements this (time zone known to the
        // bundled IANA database); until then every config is accepted.
        Ok(())
    }
}

/// How hours are counted, named as in `Intl.DateTimeFormat`'s `hourCycle`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HourCycle {
    /// 0–23.
    #[default]
    H23,
    /// 1–12 with AM/PM.
    H12,
}
