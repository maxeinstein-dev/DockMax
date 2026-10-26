//! The strip of small icons at the end of the dock while the Windows taskbar
//! is hidden, as the taskbar has them (and the PrimoDock): the arrow that
//! shows the other tray icons, the tray icons the user keeps in the dock (up
//! to six), the network, the volume and the battery. The glyphs are Windows'
//! own, from its icon font.

use crate::commands::tray::TrayIcon;
use crate::domain::settings::{DockEdge, TaskbarMode};
use crate::windows::system_status::{Battery, Network, Status, Volume};

/// The font of Windows 11's own icons (the taskbar's).
pub const ICON_FONT: &str = "Segoe Fluent Icons";

/// One small icon of the strip.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Shows the tray icons not kept in the dock (the tray panel).
    Arrow,
    /// A tray icon kept in the dock: its program's own image.
    Tray(TrayIcon),
    Network(Network),
    Volume(Volume),
    Battery(Battery),
}

impl Cell {
    /// A stable name for the cell, for assistive technology.
    pub fn key(&self) -> String {
        match self {
            Cell::Arrow => "strip:arrow".into(),
            Cell::Tray(icon) => format!("strip:tray:{}", icon.key),
            Cell::Network(_) => "strip:network".into(),
            Cell::Volume(_) => "strip:volume".into(),
            Cell::Battery(_) => "strip:battery".into(),
        }
    }

    /// The glyph of Windows' icon font, and how strongly it is drawn; `None`
    /// for a tray icon, which has its own image.
    pub fn glyph(&self, edge: DockEdge) -> Option<(char, f32)> {
        Some(match self {
            Cell::Arrow => match edge {
                // ChevronUp, or ChevronDown with the dock on top.
                DockEdge::Bottom => ('\u{E70E}', 1.0),
                DockEdge::Top => ('\u{E70D}', 1.0),
            },
            Cell::Tray(_) => return None,
            Cell::Network(Network::Wifi) => ('\u{E701}', 1.0),
            Cell::Network(Network::Wired) => ('\u{E839}', 1.0),
            Cell::Network(Network::Offline) => ('\u{E701}', 0.4),
            Cell::Volume(volume) if volume.muted => ('\u{E74F}', 1.0),
            Cell::Volume(volume) => (
                match volume.percent {
                    0 => '\u{E992}',
                    1..=33 => '\u{E993}',
                    34..=66 => '\u{E994}',
                    _ => '\u{E995}',
                },
                1.0,
            ),
            // Battery0 to Battery9, then Battery10.
            Cell::Battery(battery) => {
                let level = (u32::from(battery.percent) + 5) / 10;
                (
                    if level >= 10 {
                        '\u{E83F}'
                    } else {
                        char::from_u32(0xE850 + level).unwrap_or('\u{E850}')
                    },
                    1.0,
                )
            }
        })
    }

    /// What its label says.
    pub fn tip(&self) -> String {
        let text = &crate::i18n::messages().dock;
        match self {
            Cell::Arrow => text.show_hidden_icons.into(),
            Cell::Tray(icon) if icon.tip.trim().is_empty() => text.tray_icon.into(),
            Cell::Tray(icon) => icon.tip.clone(),
            Cell::Network(Network::Wifi) => text.wifi.into(),
            Cell::Network(Network::Wired) => text.wired.into(),
            Cell::Network(Network::Offline) => text.offline.into(),
            Cell::Volume(volume) if volume.muted => text.muted.into(),
            Cell::Volume(volume) => text.volume.replace("{n}", &volume.percent.to_string()),
            Cell::Battery(battery) => (if battery.charging {
                text.battery_charging
            } else {
                text.battery
            })
            .replace("{n}", &battery.percent.to_string()),
        }
    }
}

/// The strip for the taskbar `mode`: nothing while the taskbar is visible
/// (it has all of this); the arrow and the tray icons only while it is hidden
/// altogether (the dock holds the notification area then). `pinned` are the
/// tray icons kept in the dock, by their lasting name, in that order.
pub fn cells(
    mode: TaskbarMode,
    status: Option<&Status>,
    tray: &[TrayIcon],
    pinned: &[String],
) -> Vec<Cell> {
    if mode == TaskbarMode::Visible {
        return Vec::new();
    }
    let mut cells = Vec::new();
    if mode == TaskbarMode::Hidden {
        cells.push(Cell::Arrow);
        for id in pinned {
            if let Some(icon) = tray
                .iter()
                .find(|icon| icon.id.as_deref() == Some(id.as_str()) && !icon.hidden)
            {
                cells.push(Cell::Tray(icon.clone()));
            }
        }
    }
    if let Some(status) = status {
        cells.push(Cell::Network(status.network));
        if let Some(volume) = status.volume {
            cells.push(Cell::Volume(volume));
        }
        if let Some(battery) = status.battery {
            cells.push(Cell::Battery(battery));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(key: &str, id: &str) -> TrayIcon {
        TrayIcon {
            key: key.into(),
            id: Some(id.into()),
            tip: "Docker".into(),
            image: None,
            hidden: false,
            clickable: true,
        }
    }

    const STATUS: Status = Status {
        battery: Some(Battery {
            percent: 78,
            charging: false,
        }),
        network: Network::Wifi,
        volume: Some(Volume {
            percent: 40,
            muted: false,
        }),
    };

    #[test]
    fn hidden_taskbar_the_arrow_the_kept_icons_then_network_volume_battery() {
        let tray = [icon("1:1", "path:a.exe#1"), icon("2:1", "path:b.exe#1")];
        let pinned = ["path:b.exe#1".to_string(), "path:gone.exe#1".to_string()];

        let found = cells(TaskbarMode::Hidden, Some(&STATUS), &tray, &pinned);

        let keys: Vec<String> = found.iter().map(Cell::key).collect();
        assert_eq!(
            keys,
            [
                "strip:arrow",
                "strip:tray:2:1",
                "strip:network",
                "strip:volume",
                "strip:battery"
            ]
        );
    }

    #[test]
    fn an_auto_hiding_taskbar_keeps_its_tray_and_a_visible_one_everything() {
        let found = cells(TaskbarMode::Autohide, Some(&STATUS), &[], &[]);
        assert_eq!(found.len(), 3);
        assert!(!found.contains(&Cell::Arrow));
        assert!(cells(TaskbarMode::Visible, Some(&STATUS), &[], &[]).is_empty());
    }

    #[test]
    fn no_battery_no_battery_icon() {
        let desktop = Status {
            battery: None,
            ..STATUS
        };
        let found = cells(TaskbarMode::Autohide, Some(&desktop), &[], &[]);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn the_glyphs_follow_the_state() {
        let battery = |percent| {
            Cell::Battery(Battery {
                percent,
                charging: false,
            })
            .glyph(DockEdge::Bottom)
            .unwrap()
            .0
        };
        assert_eq!(battery(0), '\u{E850}');
        assert_eq!(battery(78), '\u{E858}');
        assert_eq!(battery(100), '\u{E83F}');
        let volume = |percent, muted| {
            Cell::Volume(Volume { percent, muted })
                .glyph(DockEdge::Bottom)
                .unwrap()
                .0
        };
        assert_eq!(volume(40, true), '\u{E74F}');
        assert_eq!(volume(0, false), '\u{E992}');
        assert_eq!(volume(100, false), '\u{E995}');
        assert_eq!(Cell::Arrow.glyph(DockEdge::Top).unwrap().0, '\u{E70D}');
        assert_eq!(
            Cell::Tray(icon("1:1", "path:a.exe#1")).glyph(DockEdge::Bottom),
            None
        );
    }

    #[test]
    fn the_labels_say_the_values() {
        assert_eq!(Cell::Battery(STATUS.battery.unwrap()).tip(), "Bateria: 78%");
        assert_eq!(Cell::Volume(STATUS.volume.unwrap()).tip(), "Volume: 40%");
        assert_eq!(Cell::Tray(icon("1:1", "path:a.exe#1")).tip(), "Docker");
    }
}
