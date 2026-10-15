//! The settings as app state: loaded once at startup, read by both windows
//! and changed only through `update_settings`, which saves them before
//! telling every window (spec 0002 §2.3 items 2–4). Rust is the source of
//! truth; the windows apply what `settings-changed` carries.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::domain::settings::{
    DockEdge, DockOpacity, DockSettings, Settings, SettingsError, SettingsErrorCode, Theme,
};
use crate::persistence::settings_file::{self, FILE_NAME};

/// Event sent to every window with the new settings after a change.
pub const CHANGED_EVENT: &str = "settings-changed";

/// The settings `update_settings` receives: the full object, nothing more.
///
/// Stricter than [`Settings`], which fills missing fields with defaults and
/// ignores unknown ones so an older `settings.json` still loads. From the
/// interface, a missing field would silently reset that setting and an
/// unknown one would be dropped, so both are rejected here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsInput {
    version: u32,
    theme: Theme,
    dock: DockSettingsInput,
}

/// The `dock` part of [`SettingsInput`], as strict.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DockSettingsInput {
    edge: DockEdge,
    icon_size: u16,
    opacity: DockOpacity,
}

impl From<SettingsInput> for Settings {
    /// Only the shape is checked here; the values are validated when saved.
    fn from(input: SettingsInput) -> Self {
        let SettingsInput {
            version,
            theme,
            dock,
        } = input;
        Settings {
            version,
            theme,
            dock: DockSettings {
                edge: dock.edge,
                icon_size: dock.icon_size,
                opacity: dock.opacity,
            },
        }
    }
}

/// How `update_settings` fails: `{ "code": "writeFailed" }`, matching
/// `SettingsCommandError` in `src/core/settings.ts`. The interface turns the
/// code into a translated message; the error carries no path and no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsCommandError {
    code: SettingsErrorCode,
}

impl From<&SettingsError> for SettingsCommandError {
    fn from(error: &SettingsError) -> Self {
        SettingsCommandError { code: error.code() }
    }
}

/// The settings in use and where they are saved.
#[derive(Debug)]
pub struct SettingsState {
    settings: Settings,
    /// `None` when the config folder could not be resolved: the app runs on
    /// the defaults and every change is rejected.
    path: Option<PathBuf>,
}

impl SettingsState {
    /// Validates and saves `settings`, and only then makes them the current
    /// ones. On failure nothing changes, in memory or on disk.
    pub fn update(&mut self, settings: Settings) -> Result<&Settings, SettingsError> {
        let path = self.path.as_deref().ok_or(SettingsError::Unavailable)?;
        settings_file::save(path, &settings)?;
        self.settings = settings;
        Ok(&self.settings)
    }
}

/// Loads `settings.json` from the app config folder. Never fails: a
/// missing, corrupt or invalid file gives the defaults, with a warning in
/// the log for the last two.
pub fn setup(app: &AppHandle) {
    let state = match app.path().app_config_dir() {
        Ok(dir) => {
            let path = dir.join(FILE_NAME);
            let loaded = settings_file::load(&path);
            if let Some(issue) = loaded.issue {
                log::warn!("settings: using the defaults: {issue}");
            }
            SettingsState {
                settings: loaded.settings,
                path: Some(path),
            }
        }
        Err(error) => {
            log::warn!("settings: config folder unavailable, using the defaults: {error}");
            SettingsState {
                settings: Settings::default(),
                path: None,
            }
        }
    };
    app.manage(Mutex::new(state));
}

fn lock<'a>(state: &'a State<'_, Mutex<SettingsState>>) -> MutexGuard<'a, SettingsState> {
    // The state stays consistent even if a holder panicked: `update` only
    // replaces the settings after saving them.
    state
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Called by both windows at startup.
#[tauri::command]
pub fn get_settings(state: State<'_, Mutex<SettingsState>>) -> Settings {
    lock(&state).settings.clone()
}

/// Called by the panel on every change, with the full settings (a partial or
/// unknown payload is rejected by Tauri before this runs). Returns the
/// settings now in use; on failure, the error code (no path, no value) and
/// nothing changed.
///
/// Sync commands run on the main thread, one at a time, so the events go out
/// in the same order as the saves.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, Mutex<SettingsState>>,
    settings: SettingsInput,
) -> Result<Settings, SettingsCommandError> {
    let updated = lock(&state)
        .update(settings.into())
        .map_err(|error| {
            log::warn!("settings: change rejected: {error}");
            SettingsCommandError::from(&error)
        })?
        .clone();
    // The lock is released before emitting.
    if let Err(error) = app.emit(CHANGED_EVENT, &updated) {
        log::warn!("settings: saved, but failed to notify the windows: {error}");
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use tempfile::{TempDir, tempdir};

    use super::*;

    // The command must keep taking the strict DTO: with a `Settings`
    // parameter, serde defaults would silently accept partial payloads again.
    type UpdateCommand = fn(
        AppHandle,
        State<'static, Mutex<SettingsState>>,
        SettingsInput,
    ) -> Result<Settings, SettingsCommandError>;
    const _: UpdateCommand = update_settings;

    fn state_in(dir: &TempDir) -> SettingsState {
        SettingsState {
            settings: Settings::default(),
            path: Some(dir.path().join(FILE_NAME)),
        }
    }

    fn large_icons() -> Settings {
        let mut settings = Settings::default();
        settings.dock.icon_size = 64;
        settings
    }

    #[test]
    fn update_saves_then_becomes_the_current_settings() {
        let dir = tempdir().unwrap();
        let mut state = state_in(&dir);

        assert_eq!(state.update(large_icons()), Ok(&large_icons()));

        assert_eq!(state.settings, large_icons());
        let saved = settings_file::load(&dir.path().join(FILE_NAME));
        assert_eq!(saved.settings, large_icons());
        assert_eq!(saved.issue, None);
    }

    #[test]
    fn invalid_update_changes_neither_the_state_nor_the_file() {
        let dir = tempdir().unwrap();
        let mut state = state_in(&dir);
        state.update(large_icons()).unwrap();
        let before = fs::read(dir.path().join(FILE_NAME)).unwrap();

        let mut too_big = large_icons();
        too_big.dock.icon_size = 200;
        assert_eq!(
            state.update(too_big),
            Err(SettingsError::IconSizeOutOfRange)
        );

        assert_eq!(state.settings, large_icons());
        assert_eq!(fs::read(dir.path().join(FILE_NAME)).unwrap(), before);
    }

    #[test]
    fn failed_save_keeps_the_current_settings() {
        // A folder at the file path makes the save fail.
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(FILE_NAME)).unwrap();
        let mut state = state_in(&dir);

        let error = state.update(large_icons()).unwrap_err();

        assert!(matches!(error, SettingsError::Write(_)), "{error:?}");
        assert_eq!(state.settings, Settings::default());
    }

    fn input(json: &str) -> Result<SettingsInput, serde_json::Error> {
        serde_json::from_str(json)
    }

    const FULL: &str =
        r#"{"version":1,"theme":"dark","dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#;

    #[test]
    fn full_update_payload_converts_into_the_settings() {
        let settings = Settings::from(input(FULL).unwrap());

        assert_eq!(
            settings,
            Settings {
                version: 1,
                theme: Theme::Dark,
                dock: DockSettings {
                    edge: DockEdge::Top,
                    icon_size: 64,
                    opacity: DockOpacity::Solid,
                },
            }
        );
    }

    #[test]
    fn update_payload_with_missing_fields_is_rejected() {
        // Unlike the file, a missing field is never filled with its default:
        // it would silently reset that setting.
        let partial = [
            "{}",
            r#"{"version":1,"theme":"dark"}"#,
            r#"{"theme":"dark","dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
            r#"{"version":1,"dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
            r#"{"version":1,"theme":"dark","dock":{}}"#,
            r#"{"version":1,"theme":"dark","dock":{"edge":"top","iconSize":64}}"#,
            r#"{"version":1,"theme":"dark","dock":{"edge":"top","opacity":"solid"}}"#,
            r#"{"version":1,"theme":"dark","dock":{"iconSize":64,"opacity":"solid"}}"#,
        ];
        for json in partial {
            assert!(input(json).is_err(), "{json}");
        }
    }

    #[test]
    fn update_payload_with_unknown_fields_is_rejected() {
        let extra = [
            r#"{"version":1,"theme":"dark","locale":"en",
                "dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
            r#"{"version":1,"theme":"dark",
                "dock":{"edge":"top","iconSize":64,"opacity":"solid","offset":3}}"#,
            r#"{"version":1,"theme":"dark",
                "dock":{"edge":"top","icon_size":64,"opacity":"solid"}}"#,
        ];
        for json in extra {
            assert!(input(json).is_err(), "{json}");
        }
    }

    #[test]
    fn update_payload_of_the_wrong_shape_is_rejected() {
        for json in ["null", "[]", r#"{"version":1,"theme":"dark","dock":null}"#] {
            assert!(input(json).is_err(), "{json}");
        }
    }

    #[test]
    fn without_a_config_folder_every_update_is_rejected() {
        let mut state = SettingsState {
            settings: Settings::default(),
            path: None,
        };

        assert_eq!(state.update(large_icons()), Err(SettingsError::Unavailable));
        assert_eq!(state.settings, Settings::default());
    }

    #[test]
    fn command_error_is_only_the_code() {
        let cases = [
            (SettingsError::IconSizeOutOfRange, "iconSizeOutOfRange"),
            (SettingsError::UnsupportedVersion, "unsupportedVersion"),
            (
                SettingsError::Write(io::ErrorKind::PermissionDenied),
                "writeFailed",
            ),
            (SettingsError::Unavailable, "notReady"),
        ];
        for (error, code) in cases {
            assert_eq!(
                serde_json::to_value(SettingsCommandError::from(&error)).unwrap(),
                serde_json::json!({ "code": code }),
                "{error:?}"
            );
        }
    }
}
