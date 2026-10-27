//! The live faces of the widgets' tiles (DESIGN.md "Ícones vivos"): what
//! each kind shows inside its glaze, at the time it is now.

use windows::Win32::Foundation::SYSTEMTIME;

use super::paint::{self, Face, Graph, Meter};
use super::theme::Glaze;
use crate::domain::dock_layout::DockSize;
use crate::domain::live_faces::{self, Wall};
use crate::domain::system_activity::{self, Reading};
use crate::domain::widgets::clock::{ClockConfig, ClockIconStyle};
use crate::domain::widgets::system::{ActivityConfig, ActivityShow};
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

/// The figure of a share, 0–100, as `42%`; the "no value" mark without one.
fn percent_text(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{}%", value.round() as u32),
        None => crate::i18n::messages().dock.no_value.to_string(),
    }
}

/// The Atividade's meters, from the reading (none yet: empty meters).
pub fn meters(config: &ActivityConfig, reading: Option<&Reading>) -> Vec<Meter> {
    let cpu = reading.and_then(|reading| reading.cpu);
    let memory = reading
        .and_then(|reading| reading.memory)
        .and_then(|memory| memory.percent());
    let meter = |label, short, value: Option<f64>| Meter {
        label,
        short,
        fraction: value.map(|value| (value / 100.0) as f32),
        text: percent_text(value),
    };
    let processor = || meter("CPU", "CPU", cpu);
    let ram = || meter("Memória", "RAM", memory);
    match config.show {
        ActivityShow::Both => vec![processor(), ram()],
        ActivityShow::Cpu => vec![processor()],
        ActivityShow::Memory => vec![ram()],
    }
}

/// The network's face: the rates, and on two icons the graph, scaled to
/// the busiest moment of the minute (at least 64 KB/s, so a quiet network
/// draws flat).
pub fn rates(width: u8, reading: Option<&Reading>) -> Face {
    let (down, up) = reading.map_or((0.0, 0.0), |reading| (reading.down, reading.up));
    let wide = width > 1;
    let text = if wide {
        system_activity::rate_text
    } else {
        system_activity::rate_short
    };
    let graph = wide.then(|| {
        let empty = Vec::new();
        let (downs, ups) = reading.map_or((&empty, &empty), |reading| {
            (&reading.down_history, &reading.up_history)
        });
        let peak = downs
            .iter()
            .chain(ups)
            .copied()
            .fold(64.0 * 1024.0, f64::max);
        let scale = |values: &Vec<f64>| values.iter().map(|value| (value / peak) as f32).collect();
        Graph {
            down: scale(downs),
            up: scale(ups),
        }
    });
    Face::Rates {
        glaze: Glaze::NETWORK,
        down: text(down),
        up: text(up),
        graph,
    }
}

/// The face of `widget` at `now`, with the system's reading for the
/// system widgets.
pub fn widget(
    widget: &WidgetInstance,
    size: DockSize,
    now: &SYSTEMTIME,
    system: Option<&Reading>,
) -> Face {
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
        WidgetConfig::Activity(config) => Face::Meters {
            glaze: Glaze::ACTIVITY,
            style: config.style,
            meters: meters(config, system),
        },
        WidgetConfig::Network(config) => rates(config.width, system),
        // Never on the dock in this phase (`dock_items` leaves them out).
        WidgetConfig::Shortcut(_) | WidgetConfig::Files(_) => Face::Glyph {
            glaze: Glaze::FILES,
            paths: paint::NOTE_GLYPH,
        },
    }
}
