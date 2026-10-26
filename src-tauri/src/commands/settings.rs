//! The settings as app state: loaded once at startup, read by both windows
//! and changed only through `update_settings`, which saves them before
//! telling every window (spec 0002 §2.3 items 2–4). Rust is the source of
//! truth; the windows apply what `settings-changed` carries.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use std::collections::BTreeMap;

use crate::domain::settings::{
    DockEdge, DockOpacity, DockSettings, DockStyle, HoverEffect, IconShape, Indicator, MiddleClick,
    OpenAnimation, Settings, SettingsError, SettingsErrorCode, StyleTweaks, TaskbarMode, Theme,
};
use crate::persistence::settings_file::{self, FILE_NAME};
use crate::windows::dock;

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
    launch_at_login: bool,
}

/// The `dock` part of [`SettingsInput`], as strict.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DockSettingsInput {
    edge: DockEdge,
    icon_size: u16,
    opacity: DockOpacity,
    style: DockStyle,
    tweaks: BTreeMap<DockStyle, StyleTweaks>,
    hover: HoverEffect,
    hover_scale: u16,
    indicator: Indicator,
    /// An `Option` that serde fills with `None` when missing: the interface
    /// always sends it, as `null` for the style's own color.
    indicator_color: Option<String>,
    icon_shape: IconShape,
    /// Like `indicator_color`: always sent, `null` for the Windows icons.
    icon_pack: Option<String>,
    open_animation: OpenAnimation,
    show_open_apps: bool,
    middle_click: MiddleClick,
    taskbar: TaskbarMode,
    bottom_space: u8,
}

impl From<SettingsInput> for Settings {
    /// Only the shape is checked here; the values are validated when saved.
    fn from(input: SettingsInput) -> Self {
        let SettingsInput {
            version,
            theme,
            dock,
            launch_at_login,
        } = input;
        Settings {
            version,
            theme,
            launch_at_login,
            dock: DockSettings {
                edge: dock.edge,
                icon_size: dock.icon_size,
                opacity: dock.opacity,
                style: dock.style,
                tweaks: dock.tweaks,
                hover: dock.hover,
                hover_scale: dock.hover_scale,
                indicator: dock.indicator,
                indicator_color: dock.indicator_color,
                icon_shape: dock.icon_shape,
                icon_pack: dock.icon_pack,
                open_animation: dock.open_animation,
                show_open_apps: dock.show_open_apps,
                middle_click: dock.middle_click,
                taskbar: dock.taskbar,
                bottom_space: dock.bottom_space,
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
    let state = load_state(app.path().app_config_dir());
    app.manage(Mutex::new(state));
}

/// The settings state from `config_dir`, the app config folder as Tauri
/// resolved it. Without a folder, the defaults and a warning in the log.
///
/// The error is not logged, and `E` has no bound so it can't be: its text
/// may hold the folder, which an override can point anywhere.
fn load_state<E>(config_dir: Result<PathBuf, E>) -> SettingsState {
    match config_dir {
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
        Err(_) => {
            log::warn!("settings: config folder unavailable, using the defaults");
            SettingsState {
                settings: Settings::default(),
                path: None,
            }
        }
    }
}

fn lock<'a>(state: &'a State<'_, Mutex<SettingsState>>) -> MutexGuard<'a, SettingsState> {
    // The state stays consistent even if a holder panicked: `update` only
    // replaces the settings after saving them.
    state
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The settings in use, for the Rust side (the dock's edge). The defaults
/// if the state is not loaded yet, which `startup` rules out.
pub fn current(app: &AppHandle) -> Settings {
    app.try_state::<Mutex<SettingsState>>()
        .map(|state| lock(&state).settings.clone())
        .unwrap_or_default()
}

/// Called by both windows at startup.
#[tauri::command]
pub fn get_settings(state: State<'_, Mutex<SettingsState>>) -> Settings {
    lock(&state).settings.clone()
}

/// Puts the sign-in entry as the setting says. The build the end-to-end tests
/// drive never touches the real `Run` key of whoever runs them. A failure is
/// logged: the setting stays as the user chose it, and the next run of a
/// release build writes the entry again (`sync_launch_at_login`).
fn apply_launch_at_login(enabled: bool) {
    if cfg!(feature = "e2e") {
        return;
    }
    if let Err(error) = crate::windows::autostart::apply(enabled) {
        log::warn!("settings: starting with Windows could not be changed: {error}");
    }
}

/// At startup of a release build: a program that moved (an update, a new
/// folder) is found at sign-in again. The debug builds of development never
/// register themselves.
pub fn sync_launch_at_login(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    if current(app).launch_at_login {
        apply_launch_at_login(true);
    }
}

/// Called by the panel on every change, with the full settings (a partial or
/// unknown payload is rejected by Tauri before this runs). Returns the
/// settings now in use; on failure, the error code (no path, no value) and
/// nothing changed.
///
/// Sync commands run on the main thread, one at a time, so the events go out
/// in the same order as the saves. A change of edge or size places the dock
/// again and closes the open panel (its anchor moved).
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, Mutex<SettingsState>>,
    settings: SettingsInput,
) -> Result<Settings, SettingsCommandError> {
    let (before, updated, launch_before) = {
        let mut state = lock(&state);
        let before = state.settings.dock.clone();
        let launch_before = state.settings.launch_at_login;
        let updated = state
            .update(settings.into())
            .map_err(|error| {
                log::warn!("settings: change rejected: {error}");
                SettingsCommandError::from(&error)
            })?
            .clone();
        (before, updated, launch_before)
    };
    // The lock is released before emitting and placing the dock, which
    // reads the settings.
    if let Err(error) = app.emit(CHANGED_EVENT, &updated) {
        log::warn!("settings: saved, but failed to notify the windows: {error}");
    }
    dock::settings_changed(&app, &before, &updated.dock);
    if updated.launch_at_login != launch_before {
        apply_launch_at_login(updated.launch_at_login);
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
    fn without_a_config_folder_the_defaults_are_used_and_never_saved() {
        let mut state = load_state(Err::<PathBuf, _>(Unprintable));

        assert_eq!(state.settings, Settings::default());
        assert_eq!(state.path, None);
        assert!(matches!(
            state.update(large_icons()),
            Err(SettingsError::Unavailable)
        ));
    }

    #[test]
    fn with_a_config_folder_the_settings_file_is_loaded() {
        let dir = tempdir().unwrap();
        state_in(&dir).update(large_icons()).unwrap();

        let state = load_state::<Unprintable>(Ok(dir.path().to_path_buf()));

        assert_eq!(state.settings, large_icons());
        assert_eq!(state.path, Some(dir.path().join(FILE_NAME)));
    }

    /// Neither `Display` nor `Debug`: the tests above only compile while the
    /// error of the config folder, whose text may hold the redirected
    /// folder, is never logged.
    struct Unprintable;

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

    /// The look fields of a full update payload.
    const LOOK: &str = r##""style":"neon","tweaks":{"neon":{"corners":30}},"hover":"magnify","hoverScale":120,"indicator":"dash","indicatorColor":"#0A84FF","iconShape":"uniform","iconPack":"pixel-system","openAnimation":"pulse","showOpenApps":false,"middleClick":"none","taskbar":"hidden","bottomSpace":2"##;

    fn full() -> String {
        format!(
            r#"{{"version":1,"theme":"dark","dock":{{"edge":"top","iconSize":64,"opacity":"solid",{LOOK}}},"launchAtLogin":true}}"#
        )
    }

    #[test]
    fn full_update_payload_converts_into_the_settings() {
        let settings = Settings::from(input(&full()).unwrap());

        assert_eq!(
            settings,
            Settings {
                version: 1,
                theme: Theme::Dark,
                launch_at_login: true,
                dock: DockSettings {
                    edge: DockEdge::Top,
                    icon_size: 64,
                    opacity: DockOpacity::Solid,
                    style: DockStyle::Neon,
                    tweaks: BTreeMap::from([(
                        DockStyle::Neon,
                        StyleTweaks {
                            corners: Some(30),
                            ..StyleTweaks::default()
                        },
                    )]),
                    hover: HoverEffect::Magnify,
                    hover_scale: 120,
                    indicator: Indicator::Dash,
                    indicator_color: Some("#0A84FF".into()),
                    icon_shape: IconShape::Uniform,
                    icon_pack: Some("pixel-system".into()),
                    open_animation: OpenAnimation::Pulse,
                    show_open_apps: false,
                    middle_click: MiddleClick::None,
                    taskbar: TaskbarMode::Hidden,
                    bottom_space: 2,
                },
            }
        );
    }

    #[test]
    fn update_payload_with_missing_fields_is_rejected() {
        // Unlike the file, a missing field is never filled with its default:
        // it would silently reset that setting.
        let partial = [
            "{}".to_string(),
            r#"{"version":1,"theme":"dark"}"#.to_string(),
            format!(
                r#"{{"theme":"dark","dock":{{"edge":"top","iconSize":64,"opacity":"solid",{LOOK}}}}}"#
            ),
            format!(
                r#"{{"version":1,"dock":{{"edge":"top","iconSize":64,"opacity":"solid",{LOOK}}}}}"#
            ),
            r#"{"version":1,"theme":"dark","dock":{}}"#.to_string(),
            format!(
                r#"{{"version":1,"theme":"dark","dock":{{"edge":"top","iconSize":64,{LOOK}}}}}"#
            ),
            format!(
                r#"{{"version":1,"theme":"dark","dock":{{"edge":"top","opacity":"solid",{LOOK}}}}}"#
            ),
            format!(
                r#"{{"version":1,"theme":"dark","dock":{{"iconSize":64,"opacity":"solid",{LOOK}}}}}"#
            ),
            // A payload from before the look fields: every one of them is
            // required, none is filled with its default.
            r#"{"version":1,"theme":"dark","dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#
                .to_string(),
        ];
        for json in partial {
            assert!(input(&json).is_err(), "{json}");
        }

        // Each look field, taken out of an otherwise full payload (the
        // optional color is the exception: it is `null` for the default).
        for field in [
            "style",
            "tweaks",
            "hover",
            "hoverScale",
            "indicator",
            "taskbar",
            "bottomSpace",
        ] {
            let mut value: serde_json::Value = serde_json::from_str(&full()).unwrap();
            value["dock"].as_object_mut().unwrap().remove(field);
            assert!(input(&value.to_string()).is_err(), "{field}");
        }
    }

    #[test]
    fn update_payload_with_unknown_fields_is_rejected() {
        let extra = [
            format!(
                r#"{{"version":1,"theme":"dark","locale":"en",
                "dock":{{"edge":"top","iconSize":64,"opacity":"solid",{LOOK}}}}}"#
            ),
            format!(
                r#"{{"version":1,"theme":"dark",
                "dock":{{"edge":"top","iconSize":64,"opacity":"solid","offset":3,{LOOK}}}}}"#
            ),
            format!(
                r#"{{"version":1,"theme":"dark",
                "dock":{{"edge":"top","icon_size":64,"opacity":"solid",{LOOK}}}}}"#
            ),
        ];
        for json in extra {
            assert!(input(&json).is_err(), "{json}");
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
            (SettingsError::LookOutOfRange, "lookOutOfRange"),
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
