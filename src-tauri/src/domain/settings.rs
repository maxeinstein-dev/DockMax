//! User settings: the `settings.json` v1 model, its defaults and its rules.
//!
//! Every field falls back to its default when missing, so partial or older
//! files still load, and unknown fields are ignored. Values that are present
//! but wrong are rejected by [`Settings::validate`].

use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Format version written by this build. Files with any other version are
/// rejected.
pub const CURRENT_VERSION: u32 = 1;

/// Smallest dock icon size accepted, in logical pixels.
pub const MIN_ICON_SIZE: u16 = 32;

/// Largest dock icon size accepted, in logical pixels.
pub const MAX_ICON_SIZE: u16 = 96;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub theme: Theme,
    pub dock: DockSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: CURRENT_VERSION,
            theme: Theme::default(),
            dock: DockSettings::default(),
        }
    }
}

impl Settings {
    /// Checks the rules serde can't express: the format version and the
    /// icon size range.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.version != CURRENT_VERSION {
            return Err(SettingsError::UnsupportedVersion);
        }
        if !(MIN_ICON_SIZE..=MAX_ICON_SIZE).contains(&self.dock.icon_size) {
            return Err(SettingsError::IconSizeOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follows the Windows light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DockSettings {
    pub edge: DockEdge,
    /// Icon size in logical pixels, within `MIN_ICON_SIZE..=MAX_ICON_SIZE`.
    /// The size presets are values of this field, not a separate setting.
    pub icon_size: u16,
    pub opacity: DockOpacity,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            edge: DockEdge::default(),
            icon_size: SizePreset::Medium.icon_size(),
            opacity: DockOpacity::default(),
        }
    }
}

impl DockSettings {
    /// The preset matching the icon size, if any.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the dock in PR 3"))]
    pub fn preset(&self) -> Option<SizePreset> {
        SizePreset::from_icon_size(self.icon_size)
    }
}

/// Screen edge the dock sits on, as stored in the settings.
///
/// Kept apart from `windows::dock::Edge` so the domain doesn't depend on the
/// window layer; the dock settings (PR 3) map one to the other.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    #[default]
    Bottom,
    Top,
}

/// Dock background: simulated glass or opaque. Panels are always opaque.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockOpacity {
    #[default]
    Translucent,
    Solid,
}

/// Named icon sizes offered by the settings panel, from `DESIGN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizePreset {
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the dock in PR 3"))]
    Small,
    Medium,
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the dock in PR 3"))]
    Large,
}

impl SizePreset {
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the dock in PR 3"))]
    pub const ALL: [SizePreset; 3] = [SizePreset::Small, SizePreset::Medium, SizePreset::Large];

    /// Icon size of the preset, in logical pixels.
    pub const fn icon_size(self) -> u16 {
        match self {
            SizePreset::Small => 40,
            SizePreset::Medium => 48,
            SizePreset::Large => 64,
        }
    }

    /// The preset with exactly this icon size; `None` for sizes in between.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the dock in PR 3"))]
    pub fn from_icon_size(icon_size: u16) -> Option<SizePreset> {
        SizePreset::ALL
            .into_iter()
            .find(|preset| preset.icon_size() == icon_size)
    }
}

/// Why settings were rejected or could not be saved.
///
/// Messages name only the kind of failure, never a file path or a settings
/// value, so they are safe to log and to show in the interface.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettingsError {
    #[error("unsupported settings version")]
    UnsupportedVersion,
    #[error("icon size out of range")]
    IconSizeOutOfRange,
    #[error("settings file could not be written ({0})")]
    Write(io::ErrorKind),
    /// There is nowhere to save: the app config folder could not be
    /// resolved at startup, so the app runs on the defaults.
    #[error("config folder unavailable")]
    Unavailable,
}

impl SettingsError {
    /// The code the interface gets for this error, to show a translated
    /// message (`settings.errors` in `src/i18n/pt-BR.ts`).
    pub fn code(&self) -> SettingsErrorCode {
        match self {
            SettingsError::IconSizeOutOfRange => SettingsErrorCode::IconSizeOutOfRange,
            SettingsError::UnsupportedVersion => SettingsErrorCode::UnsupportedVersion,
            SettingsError::Write(_) => SettingsErrorCode::WriteFailed,
            SettingsError::Unavailable => SettingsErrorCode::NotReady,
        }
    }
}

/// What went wrong with a settings change, as the interface sees it. Must
/// match `SettingsErrorCode` in `src/core/settings.ts`; carries no path and
/// no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsErrorCode {
    /// The icon size is outside `MIN_ICON_SIZE..=MAX_ICON_SIZE`.
    IconSizeOutOfRange,
    /// The format version is not `CURRENT_VERSION`.
    UnsupportedVersion,
    /// The settings could not be saved; the previous ones stay.
    WriteFailed,
    /// The settings can't be changed in this run (no config folder).
    NotReady,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_icon_size(icon_size: u16) -> Settings {
        let mut settings = Settings::default();
        settings.dock.icon_size = icon_size;
        settings
    }

    #[test]
    fn defaults_are_v1_system_bottom_medium_translucent() {
        let settings = Settings::default();

        assert_eq!(settings.version, 1);
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.dock.edge, DockEdge::Bottom);
        assert_eq!(settings.dock.icon_size, 48);
        assert_eq!(settings.dock.opacity, DockOpacity::Translucent);
        assert_eq!(settings.validate(), Ok(()));
    }

    #[test]
    fn icon_size_below_the_range_is_rejected() {
        assert_eq!(
            with_icon_size(31).validate(),
            Err(SettingsError::IconSizeOutOfRange)
        );
        assert_eq!(
            with_icon_size(0).validate(),
            Err(SettingsError::IconSizeOutOfRange)
        );
    }

    #[test]
    fn icon_size_above_the_range_is_rejected() {
        assert_eq!(
            with_icon_size(97).validate(),
            Err(SettingsError::IconSizeOutOfRange)
        );
        assert_eq!(
            with_icon_size(u16::MAX).validate(),
            Err(SettingsError::IconSizeOutOfRange)
        );
    }

    #[test]
    fn icon_size_range_limits_are_accepted() {
        assert_eq!(with_icon_size(32).validate(), Ok(()));
        assert_eq!(with_icon_size(96).validate(), Ok(()));
    }

    #[test]
    fn any_version_other_than_1_is_rejected() {
        for version in [0, 2, u32::MAX] {
            let settings = Settings {
                version,
                ..Settings::default()
            };
            assert_eq!(
                settings.validate(),
                Err(SettingsError::UnsupportedVersion),
                "version {version}"
            );
        }
    }

    #[test]
    fn presets_round_trip_through_the_icon_size() {
        assert_eq!(SizePreset::Small.icon_size(), 40);
        assert_eq!(SizePreset::Medium.icon_size(), 48);
        assert_eq!(SizePreset::Large.icon_size(), 64);
        for preset in SizePreset::ALL {
            assert_eq!(SizePreset::from_icon_size(preset.icon_size()), Some(preset));
            assert_eq!(with_icon_size(preset.icon_size()).validate(), Ok(()));
        }
        assert_eq!(Settings::default().dock.preset(), Some(SizePreset::Medium));
    }

    #[test]
    fn sizes_between_presets_have_no_preset() {
        for icon_size in [32, 41, 47, 50, 63, 65, 96] {
            assert_eq!(SizePreset::from_icon_size(icon_size), None, "{icon_size}");
        }
        assert_eq!(with_icon_size(56).dock.preset(), None);
    }

    #[test]
    fn defaults_serialize_to_the_v1_example_of_the_spec() {
        // Spec 0002 §2.3 item 3, verbatim.
        let spec: serde_json::Value = serde_json::from_str(
            r#"{
              "version": 1,
              "theme": "system",
              "dock": { "edge": "bottom", "iconSize": 48, "opacity": "translucent" }
            }"#,
        )
        .unwrap();

        assert_eq!(serde_json::to_value(Settings::default()).unwrap(), spec);
        assert_eq!(
            serde_json::from_value::<Settings>(spec).unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn enums_use_lowercase_names_in_json() {
        let settings: Settings = serde_json::from_str(
            r#"{"version":1,"theme":"dark","dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
        )
        .unwrap();

        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.dock.edge, DockEdge::Top);
        assert_eq!(settings.dock.opacity, DockOpacity::Solid);
        assert_eq!(serde_json::to_string(&Theme::Light).unwrap(), "\"light\"");
        assert!(serde_json::from_str::<Theme>("\"Dark\"").is_err());
        assert!(serde_json::from_str::<DockEdge>("\"left\"").is_err());
    }

    #[test]
    fn missing_fields_fall_back_to_their_defaults() {
        let settings: Settings =
            serde_json::from_str(r#"{"theme":"light","dock":{"edge":"top","iconSize":40}}"#)
                .unwrap();

        assert_eq!(settings.version, CURRENT_VERSION);
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.dock.edge, DockEdge::Top);
        assert_eq!(settings.dock.icon_size, 40);
        assert_eq!(settings.dock.opacity, DockOpacity::Translucent);
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let settings: Settings = serde_json::from_str(
            r#"{"version":1,"locale":"pt-BR","theme":"dark",
                "dock":{"edge":"bottom","iconSize":48,"opacity":"solid","offset":12}}"#,
        )
        .unwrap();

        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.dock.opacity, DockOpacity::Solid);
    }

    #[test]
    fn error_messages_never_contain_a_path() {
        let errors = [
            SettingsError::UnsupportedVersion,
            SettingsError::IconSizeOutOfRange,
            SettingsError::Write(io::ErrorKind::PermissionDenied),
            SettingsError::Write(io::ErrorKind::NotFound),
            SettingsError::Write(io::ErrorKind::Other),
            SettingsError::Unavailable,
        ];
        for error in errors {
            let message = error.to_string();
            assert!(!message.is_empty());
            assert!(
                !message.contains('/') && !message.contains('\\'),
                "{message}"
            );
        }
    }

    #[test]
    fn each_error_maps_to_its_code() {
        let cases = [
            (
                SettingsError::IconSizeOutOfRange,
                SettingsErrorCode::IconSizeOutOfRange,
            ),
            (
                SettingsError::UnsupportedVersion,
                SettingsErrorCode::UnsupportedVersion,
            ),
            (
                SettingsError::Write(io::ErrorKind::PermissionDenied),
                SettingsErrorCode::WriteFailed,
            ),
            (
                SettingsError::Write(io::ErrorKind::NotFound),
                SettingsErrorCode::WriteFailed,
            ),
            (SettingsError::Unavailable, SettingsErrorCode::NotReady),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code, "{error:?}");
        }
    }

    #[test]
    fn error_codes_serialize_to_the_names_the_interface_expects() {
        // `SettingsErrorCode` in src/core/settings.ts.
        let cases = [
            (SettingsErrorCode::IconSizeOutOfRange, "iconSizeOutOfRange"),
            (SettingsErrorCode::UnsupportedVersion, "unsupportedVersion"),
            (SettingsErrorCode::WriteFailed, "writeFailed"),
            (SettingsErrorCode::NotReady, "notReady"),
        ];
        for (code, name) in cases {
            assert_eq!(serde_json::to_value(code).unwrap(), name);
        }
    }
}
