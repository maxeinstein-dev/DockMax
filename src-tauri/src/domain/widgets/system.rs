//! The system widgets' configs (spec 0017 F1): Atividade (the processor and
//! the memory) and Atividade de rede (the rates down and up). Both can be
//! one or two icons wide.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// The widths, in icons, the system widgets accept.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=2;

fn check_width(width: u8) -> Result<(), WidgetError> {
    if WIDTHS.contains(&width) {
        Ok(())
    } else {
        Err(WidgetError::InvalidWidth)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ActivityConfig {
    pub show: ActivityShow,
    pub style: ActivityStyle,
    /// In icons: 1 or 2.
    pub width: u8,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        ActivityConfig {
            show: ActivityShow::Both,
            style: ActivityStyle::Bars,
            width: 1,
        }
    }
}

impl ActivityConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        check_width(self.width)
    }
}

/// What the Atividade shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityShow {
    #[default]
    Both,
    Cpu,
    Memory,
}

closed_set!(ActivityShow { Both, Cpu, Memory });

/// How the Atividade draws its values in the dock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityStyle {
    #[default]
    Bars,
    Rings,
    Numbers,
}

closed_set!(ActivityStyle {
    Bars,
    Rings,
    Numbers
});

/// Atividade de rede: one icon shows the two rates; two icons add the graph
/// of the last minute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NetworkConfig {
    /// In icons: 1 or 2.
    pub width: u8,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig { width: 2 }
    }
}

impl NetworkConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        check_width(self.width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_widgets_are_one_or_two_icons_wide() {
        for width in [1, 2] {
            let activity = ActivityConfig {
                width,
                ..ActivityConfig::default()
            };
            assert_eq!(activity.validate(), Ok(()));
            assert_eq!(NetworkConfig { width }.validate(), Ok(()));
        }
        for width in [0, 3, 4] {
            let activity = ActivityConfig {
                width,
                ..ActivityConfig::default()
            };
            assert_eq!(activity.validate(), Err(WidgetError::InvalidWidth));
            assert_eq!(
                NetworkConfig { width }.validate(),
                Err(WidgetError::InvalidWidth)
            );
        }
    }
}
