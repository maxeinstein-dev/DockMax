//! The live faces of the widgets' tiles (DESIGN.md "Ícones vivos"): what
//! each kind shows inside its glaze, at the time it is now.

use windows::Win32::Foundation::SYSTEMTIME;

use super::paint::{self, Face};
use super::theme::Glaze;
use crate::domain::dock_layout::DockSize;
use crate::domain::live_faces::{self, Wall};
use crate::domain::widgets::clock::{ClockConfig, ClockIconStyle};
use crate::domain::widgets::{WidgetConfig, WidgetInstance};

/// The wall clock of the system's zone.
pub fn wall(now: &SYSTEMTIME) -> Wall {
    Wall {
        year: i32::from(now.wYear),
        month: now.wMonth as u8,
        day: now.wDay as u8,
        weekday: now.wDayOfWeek as u8,
        hour: now.wHour as u8,
        minute: now.wMinute as u8,
        second: now.wSecond as u8,
    }
}

/// Hours and minutes of the clock's zone: its own when it names one this
/// build knows, the system's otherwise.
fn clock_time(config: &ClockConfig, now: &SYSTEMTIME) -> (u8, u8) {
    let zoned = config.time_zone.as_deref().and_then(|name| {
        let zone = jiff::tz::TimeZoneDatabase::bundled().get(name).ok()?;
        let at = jiff::Timestamp::now().to_zoned(zone);
        Some((at.hour() as u8, at.minute() as u8))
    });
    zoned.unwrap_or((now.wHour as u8, now.wMinute as u8))
}

/// The face of `widget` at `now`.
pub fn widget(widget: &WidgetInstance, size: DockSize, now: &SYSTEMTIME) -> Face {
    match &widget.config {
        WidgetConfig::Clock(config) => {
            let (hour, minute) = clock_time(config, now);
            match config.icon_style {
                ClockIconStyle::Analog => Face::Dial {
                    glaze: Glaze::CLOCK,
                    hours: f32::from(hour),
                    minutes: f32::from(minute),
                },
                ClockIconStyle::Digital => Face::Time {
                    glaze: Glaze::CLOCK,
                    text: live_faces::clock_text(hour, minute, config.hour_cycle),
                },
            }
        }
        WidgetConfig::TimeProgress(config) => {
            let fraction = config
                .periods
                .first()
                .and_then(|period| live_faces::progress(*period, &wall(now), config));
            // The small size shows only the ring.
            let text = (size != DockSize::Small).then(|| match fraction {
                Some(fraction) => format!("{}%", live_faces::percent(fraction)),
                None => crate::i18n::messages().dock.no_value.to_string(),
            });
            Face::Ring {
                glaze: Glaze::PROGRESS,
                fraction: fraction.map(|fraction| fraction as f32),
                text,
            }
        }
        WidgetConfig::Note(_) => Face::Glyph {
            glaze: Glaze::NOTES,
            paths: paint::NOTE_GLYPH,
        },
        // Never on the dock in this phase (`dock_items` leaves them out).
        WidgetConfig::Shortcut(_) | WidgetConfig::Files(_) => Face::Glyph {
            glaze: Glaze::FILES,
            paths: paint::NOTE_GLYPH,
        },
    }
}
