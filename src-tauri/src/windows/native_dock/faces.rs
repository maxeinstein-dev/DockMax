//! The live faces of the widgets' tiles (DESIGN.md "Ícones vivos"): what
//! each kind shows inside its glaze.

use windows::Win32::Foundation::SYSTEMTIME;

use super::paint::{self, Face};
use super::theme::Glaze;
use crate::domain::dock_layout::DockSize;
use crate::domain::widgets::clock::ClockIconStyle;
use crate::domain::widgets::{WidgetConfig, WidgetInstance};

/// The face of `widget` at `now`.
pub fn widget(widget: &WidgetInstance, size: DockSize, now: &SYSTEMTIME) -> Face {
    let _ = size;
    match &widget.config {
        WidgetConfig::Clock(config) => match config.icon_style {
            ClockIconStyle::Analog => Face::Dial {
                glaze: Glaze::CLOCK,
                hours: f32::from(now.wHour),
                minutes: f32::from(now.wMinute),
            },
            ClockIconStyle::Digital => Face::Time {
                glaze: Glaze::CLOCK,
                text: format!("{:02}:{:02}", now.wHour, now.wMinute),
            },
        },
        WidgetConfig::TimeProgress(_) => Face::Ring {
            glaze: Glaze::PROGRESS,
            fraction: None,
            text: Some(crate::i18n::messages().dock.no_value.to_string()),
        },
        WidgetConfig::Note(_) => Face::Glyph {
            glaze: Glaze::NOTES,
            paths: paint::NOTE_GLYPH,
        },
        WidgetConfig::Shortcut(_) | WidgetConfig::Files(_) => Face::Glyph {
            glaze: Glaze::FILES,
            paths: paint::NOTE_GLYPH,
        },
    }
}
