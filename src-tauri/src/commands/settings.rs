//! The settings as app state: loaded once at startup, read by both windows
//! and changed only through `update_settings`, which saves them before
//! telling every window (spec 0002 §2.3 items 2–4). Rust is the source of
//! truth; the windows apply what `settings-changed` carries.

use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::domain::settings::{Settings, SettingsError};
use crate::persistence::settings_file::{self, FILE_NAME};

/// Event sent to every window with the new settings after a change.
pub const CHANGED_EVENT: &str = "settings-changed";

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
        let path = self
            .path
            .as_deref()
            .ok_or(SettingsError::Write(io::ErrorKind::NotFound))?;
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

/// Called by the panel on every change. Returns the settings now in use; on
/// failure, a message safe to show (no path, no value) and nothing changed.
///
/// Sync commands run on the main thread, one at a time, so the events go out
/// in the same order as the saves.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, Mutex<SettingsState>>,
    settings: Settings,
) -> Result<Settings, String> {
    let updated = lock(&state)
        .update(settings)
        .map_err(|error| {
            log::warn!("settings: change rejected: {error}");
            error.to_string()
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
    use std::fs;

    use tempfile::{TempDir, tempdir};

    use super::*;

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

    #[test]
    fn without_a_config_folder_every_update_is_rejected() {
        let mut state = SettingsState {
            settings: Settings::default(),
            path: None,
        };

        assert_eq!(
            state.update(large_icons()),
            Err(SettingsError::Write(io::ErrorKind::NotFound))
        );
        assert_eq!(state.settings, Settings::default());
    }
}
