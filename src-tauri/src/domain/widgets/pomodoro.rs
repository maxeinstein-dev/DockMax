//! The Pomodoro (spec 0017 F2c-2): the times of focus and of the pauses.
//! The timer itself lives in `domain::pomodoro` and `windows::pomodoro`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// Minutes of focus, of a pause, of a long pause; and how many focuses
/// before a long one.
pub const FOCUS: std::ops::RangeInclusive<u8> = 1..=120;
pub const PAUSE: std::ops::RangeInclusive<u8> = 1..=60;
pub const EVERY: std::ops::RangeInclusive<u8> = 2..=8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PomodoroConfig {
    /// Minutes of each focus: 25 by default.
    pub focus: u8,
    /// Minutes of a pause: 5.
    pub short: u8,
    /// Minutes of the long pause: 15.
    pub long: u8,
    /// Focuses before the long pause: 4.
    pub every: u8,
    /// The next stage starts by itself; otherwise it waits for "Iniciar".
    pub auto_start: bool,
    /// A notice of Windows too at the end of a stage; the dock's bounce and
    /// badge always come. Off by default: the author keeps Windows' notices
    /// off.
    pub windows: bool,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        PomodoroConfig {
            focus: 25,
            short: 5,
            long: 15,
            every: 4,
            auto_start: false,
            windows: false,
        }
    }
}

impl PomodoroConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if FOCUS.contains(&self.focus)
            && PAUSE.contains(&self.short)
            && PAUSE.contains(&self.long)
            && EVERY.contains(&self.every)
        {
            Ok(())
        } else {
            Err(WidgetError::InvalidPomodoro)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_times_stay_in_their_ranges() {
        assert_eq!(PomodoroConfig::default().validate(), Ok(()));
        for bad in [
            PomodoroConfig {
                focus: 0,
                ..PomodoroConfig::default()
            },
            PomodoroConfig {
                focus: 121,
                ..PomodoroConfig::default()
            },
            PomodoroConfig {
                short: 0,
                ..PomodoroConfig::default()
            },
            PomodoroConfig {
                long: 61,
                ..PomodoroConfig::default()
            },
            PomodoroConfig {
                every: 1,
                ..PomodoroConfig::default()
            },
        ] {
            assert_eq!(bad.validate(), Err(WidgetError::InvalidPomodoro));
        }
    }
}
