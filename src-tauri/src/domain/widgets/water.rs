//! The Beber água (spec 0017 F3-2): the cups of the day against a goal,
//! back to zero at midnight, and a reminder after N minutes without one.
//! The day's count lives in the config, saved at each cup.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// The goal in cups, a cup in milliliters, the reminder in minutes (0: off).
pub const GOAL: std::ops::RangeInclusive<u8> = 1..=30;
pub const CUP: std::ops::RangeInclusive<u16> = 50..=1000;
pub const REMIND: std::ops::RangeInclusive<u16> = 15..=240;
const MAX_COUNT: u8 = 99;

/// The hours of the day the reminder may come: from 8h to 22h.
pub const AWAKE: std::ops::Range<i64> = 8..22;

const DAY: i64 = 24 * 3600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WaterConfig {
    /// Cups a day: 8.
    pub goal: u8,
    /// Milliliters in a cup: 250.
    pub cup: u16,
    /// Minutes without a cup before the reminder; 0 turns it off (the
    /// default).
    pub remind: u16,
    /// Cups on `day`, the local day number of the last change.
    pub count: u8,
    pub day: i64,
    /// The last cup, in Unix seconds.
    pub last: Option<i64>,
}

impl Default for WaterConfig {
    fn default() -> Self {
        WaterConfig {
            goal: 8,
            cup: 250,
            remind: 0,
            count: 0,
            day: 0,
            last: None,
        }
    }
}

impl WaterConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if GOAL.contains(&self.goal)
            && CUP.contains(&self.cup)
            && (self.remind == 0 || REMIND.contains(&self.remind))
            && self.count <= MAX_COUNT
        {
            Ok(())
        } else {
            Err(WidgetError::InvalidWater)
        }
    }

    /// The cups of `today` (a local day number): none on a new day.
    pub fn count_on(&self, today: i64) -> u8 {
        if self.day == today { self.count } else { 0 }
    }

    /// Whether the reminder is due at `now` (Unix seconds, local time
    /// `offset` seconds from UTC): on, in the awake hours, and `remind`
    /// minutes since the last cup of today or since 8h.
    pub fn due(&self, now: i64, offset: i64) -> bool {
        if self.remind == 0 {
            return false;
        }
        let local = now + offset;
        let today = local.div_euclid(DAY);
        let hour = local.rem_euclid(DAY) / 3600;
        if !AWAKE.contains(&hour) {
            return false;
        }
        let wake = today * DAY + AWAKE.start * 3600 - offset;
        let since = self
            .last
            .filter(|last| (last + offset).div_euclid(DAY) == today)
            .map_or(wake, |last| last.max(wake));
        now - since >= i64::from(self.remind) * 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-28 at `hour`:`minute`, in São Paulo (UTC-3).
    const OFFSET: i64 = -3 * 3600;
    fn at(hour: i64, minute: i64) -> i64 {
        20_389 * DAY + hour * 3600 + minute * 60 - OFFSET
    }

    fn reminding(minutes: u16, last: Option<i64>) -> WaterConfig {
        WaterConfig {
            remind: minutes,
            last,
            ..WaterConfig::default()
        }
    }

    #[test]
    fn the_values_stay_in_their_ranges() {
        assert_eq!(WaterConfig::default().validate(), Ok(()));
        for bad in [
            WaterConfig {
                goal: 0,
                ..WaterConfig::default()
            },
            WaterConfig {
                cup: 20,
                ..WaterConfig::default()
            },
            WaterConfig {
                remind: 5,
                ..WaterConfig::default()
            },
            WaterConfig {
                count: 100,
                ..WaterConfig::default()
            },
        ] {
            assert_eq!(bad.validate(), Err(WidgetError::InvalidWater));
        }
    }

    #[test]
    fn the_count_starts_over_each_day() {
        let config = WaterConfig {
            count: 5,
            day: 20_389,
            ..WaterConfig::default()
        };
        assert_eq!(config.count_on(20_389), 5);
        assert_eq!(config.count_on(20_390), 0);
    }

    #[test]
    fn the_reminder_comes_after_the_minutes_without_a_cup() {
        // Off by default.
        assert!(!WaterConfig::default().due(at(15, 0), OFFSET));
        // From 8h with no cup today.
        assert!(!reminding(60, None).due(at(8, 59), OFFSET));
        assert!(reminding(60, None).due(at(9, 0), OFFSET));
        // From the last cup.
        assert!(!reminding(60, Some(at(14, 30))).due(at(15, 29), OFFSET));
        assert!(reminding(60, Some(at(14, 30))).due(at(15, 30), OFFSET));
        // Yesterday's cup counts from 8h.
        assert!(reminding(60, Some(at(14, 30) - DAY)).due(at(9, 0), OFFSET));
        // Never at night.
        assert!(!reminding(60, Some(at(14, 30))).due(at(22, 30), OFFSET));
        assert!(!reminding(60, None).due(at(6, 0), OFFSET));
    }
}
