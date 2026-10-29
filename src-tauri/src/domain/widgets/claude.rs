//! The Uso do Claude (spec 0017 F5): the plan's limits, the five hours' and
//! the week's. The config holds only the width; the usage is read in
//! `windows::claude_usage`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// One icon: the five hours' ring; two: both limits.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClaudeConfig {
    pub width: u8,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        ClaudeConfig { width: 2 }
    }
}

impl ClaudeConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if WIDTHS.contains(&self.width) {
            Ok(())
        } else {
            Err(WidgetError::InvalidWidth)
        }
    }
}
