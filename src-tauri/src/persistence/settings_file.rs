//! Reads and writes `settings.json`.
//!
//! Loading never fails: a missing, unreadable, corrupt or invalid file gives
//! the default settings, plus the issue for the caller to log. Saving is
//! atomic: the JSON goes to a temporary file in the same folder, which then
//! replaces the target, so the target is never left half written.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use thiserror::Error;

use crate::domain::settings::{Settings, SettingsError};

/// Name of the settings file inside the app config folder.
pub const FILE_NAME: &str = "settings.json";

/// Settings read at startup, and why the defaults were used instead of the
/// file, if they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub settings: Settings,
    pub issue: Option<LoadIssue>,
}

/// Why an existing settings file was ignored.
///
/// Messages name only the kind of problem, never a file path or the file
/// content, so they are safe to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LoadIssue {
    #[error("settings file is not valid JSON")]
    Corrupt,
    #[error("settings file has invalid values or an unsupported version")]
    Invalid,
    #[error("settings file could not be read ({0})")]
    Unreadable(io::ErrorKind),
}

/// Loads the settings at `path`. A missing file gives the defaults with no
/// issue; any other problem gives the defaults and the issue.
pub fn load(path: &Path) -> Loaded {
    let issue = match fs::read(path) {
        Ok(bytes) => match parse(&bytes) {
            Ok(settings) => {
                return Loaded {
                    settings,
                    issue: None,
                };
            }
            Err(issue) => Some(issue),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => Some(LoadIssue::Unreadable(error.kind())),
    };
    Loaded {
        settings: Settings::default(),
        issue,
    }
}

/// Parses and validates the file content. Serde's messages are dropped on
/// purpose: they can quote settings values.
fn parse(bytes: &[u8]) -> Result<Settings, LoadIssue> {
    // Notepad and PowerShell may add a UTF-8 BOM, which serde_json rejects.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| LoadIssue::Corrupt)?;
    // Serde also builds a struct from an array, which would turn `[]` into
    // the defaults without an issue.
    if !value.is_object() {
        return Err(LoadIssue::Invalid);
    }
    let settings = Settings::deserialize(value).map_err(|_| LoadIssue::Invalid)?;
    settings.validate().map_err(|_| LoadIssue::Invalid)?;
    Ok(settings)
}

/// Validates `settings` and writes them to `path` as pretty JSON, creating
/// the parent folder if needed. Invalid settings are rejected before anything
/// is written, leaving the previous file as it was.
pub fn save(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    save_with(path, settings, |_| Ok(()))
}

/// [`save`], running `before_rename` once the temporary file is complete and
/// before it replaces `path`. A seam for the tests: it lets them look at both
/// files mid-save and make the save fail at that point.
fn save_with(
    path: &Path,
    settings: &Settings,
    before_rename: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), SettingsError> {
    settings.validate()?;
    let mut json = serde_json::to_vec_pretty(settings)
        .map_err(|error| SettingsError::Write(io::Error::from(error).kind()))?;
    json.push(b'\n');
    write_atomically(path, &json, before_rename).map_err(|error| SettingsError::Write(error.kind()))
}

/// Writes `contents` to a temporary file next to `path`, flushes it to disk,
/// runs `before_rename` on it and renames it over `path` (`MoveFileExW` with
/// replace on Windows). On failure the temporary file is removed and `path`
/// keeps its old content.
fn write_atomically(
    path: &Path,
    contents: &[u8],
    before_rename: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let temp = temp_path(path)?;
    let result = write_temp(&temp, contents)
        .and_then(|()| before_rename(&temp))
        .and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        // Best effort: the file may not exist if creating it failed.
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Creates the temporary file and makes its content durable. The file is
/// closed when this returns, so it can be renamed on Windows.
fn write_temp(temp: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(temp)?;
    file.write_all(contents)?;
    file.flush()?;
    file.sync_all()
}

/// A temporary file name next to `path`, unique within this process and
/// across processes, so concurrent saves never write the same file.
fn temp_path(path: &Path) -> io::Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let Some(name) = path.file_name() else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(
        ".{}-{}.tmp",
        process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(path.with_file_name(temp_name))
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::domain::settings::{DockEdge, DockOpacity, DockSettings, Theme};

    fn settings_path(dir: &TempDir) -> PathBuf {
        dir.path().join(FILE_NAME)
    }

    fn custom_settings() -> Settings {
        Settings {
            version: 1,
            theme: Theme::Dark,
            launch_at_login: false,
            dock: DockSettings {
                edge: DockEdge::Top,
                icon_size: 64,
                opacity: DockOpacity::Solid,
                ..DockSettings::default()
            },
        }
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn load_str(content: &str) -> Loaded {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        fs::write(&path, content).unwrap();
        load(&path)
    }

    #[test]
    fn missing_file_loads_the_defaults_without_an_issue() {
        let dir = tempdir().unwrap();

        assert_eq!(
            load(&settings_path(&dir)),
            Loaded {
                settings: Settings::default(),
                issue: None,
            }
        );
    }

    #[test]
    fn starting_with_windows_is_off_unless_the_file_says_so() {
        assert!(!load_str(r#"{"version":1}"#).settings.launch_at_login);
        let loaded = load_str(r#"{"version":1,"launchAtLogin":true}"#);

        assert!(loaded.settings.launch_at_login);
        assert_eq!(loaded.issue, None);
    }

    #[test]
    fn valid_file_loads_its_values() {
        let loaded = load_str(
            r#"{"version":1,"theme":"dark","dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
        );

        assert_eq!(loaded.settings, custom_settings());
        assert_eq!(loaded.issue, None);
    }

    #[test]
    fn corrupt_file_loads_the_defaults_and_reports_corrupt() {
        for content in ["", "{", "not json", r#"{"version":1,}"#] {
            assert_eq!(
                load_str(content),
                Loaded {
                    settings: Settings::default(),
                    issue: Some(LoadIssue::Corrupt),
                },
                "{content:?}"
            );
        }
    }

    #[test]
    fn invalid_file_loads_the_defaults_and_reports_invalid() {
        let invalid = [
            r#"{"version":1,"dock":{"iconSize":31}}"#,
            r#"{"version":1,"dock":{"iconSize":97}}"#,
            r#"{"version":1,"dock":{"iconSize":-1}}"#,
            r#"{"version":1,"dock":{"iconSize":70000}}"#,
            r#"{"version":1,"dock":{"iconSize":"48"}}"#,
            r#"{"version":1,"theme":"blue"}"#,
            r#"{"version":1,"dock":{"edge":"left"}}"#,
            r#"{"version":1,"dock":{"opacity":"clear"}}"#,
            r#"{"version":1,"dock":null}"#,
            "[]",
            "null",
        ];
        for content in invalid {
            assert_eq!(
                load_str(content),
                Loaded {
                    settings: Settings::default(),
                    issue: Some(LoadIssue::Invalid),
                },
                "{content}"
            );
        }
    }

    #[test]
    fn unknown_version_is_invalid() {
        for content in [
            r#"{"version":2,"theme":"dark"}"#,
            r#"{"version":0,"theme":"dark"}"#,
        ] {
            let loaded = load_str(content);
            assert_eq!(loaded.settings, Settings::default(), "{content}");
            assert_eq!(loaded.issue, Some(LoadIssue::Invalid), "{content}");
        }
    }

    #[test]
    fn partial_file_keeps_its_values_and_fills_in_the_rest() {
        let loaded =
            load_str(r#"{"version":1,"theme":"light","dock":{"edge":"top","iconSize":40}}"#);

        assert_eq!(loaded.issue, None);
        assert_eq!(loaded.settings.theme, Theme::Light);
        assert_eq!(loaded.settings.dock.edge, DockEdge::Top);
        assert_eq!(loaded.settings.dock.icon_size, 40);
        assert_eq!(loaded.settings.dock.opacity, DockOpacity::Translucent);
    }

    #[test]
    fn unknown_fields_in_the_file_are_ignored() {
        let loaded = load_str(
            r#"{"version":1,"locale":"en","theme":"dark",
                "dock":{"edge":"top","iconSize":64,"opacity":"solid","offset":3}}"#,
        );

        assert_eq!(loaded.issue, None);
        assert_eq!(loaded.settings, custom_settings());
    }

    #[test]
    fn file_with_a_utf8_bom_loads() {
        let loaded = load_str("\u{FEFF}{\"version\":1,\"theme\":\"dark\"}");

        assert_eq!(loaded.issue, None);
        assert_eq!(loaded.settings.theme, Theme::Dark);
    }

    #[test]
    fn unreadable_path_loads_the_defaults_and_reports_unreadable() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir(&path).unwrap();

        let loaded = load(&path);

        assert_eq!(loaded.settings, Settings::default());
        assert!(
            matches!(loaded.issue, Some(LoadIssue::Unreadable(kind)) if kind != io::ErrorKind::NotFound),
            "{:?}",
            loaded.issue
        );
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);

        save(&path, &custom_settings()).unwrap();

        assert_eq!(
            load(&path),
            Loaded {
                settings: custom_settings(),
                issue: None,
            }
        );
    }

    #[test]
    fn save_writes_pretty_json_in_the_v1_format() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);

        save(&path, &Settings::default()).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("\n  \"version\": 1,\n"), "{written}");
        assert!(written.ends_with("}\n"), "{written}");
        let value: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "version": 1,
                "theme": "system",
                "dock": {
                    "edge": "bottom", "iconSize": 48, "opacity": "translucent",
                    "style": "native", "tweaks": {}, "hover": "scale", "hoverScale": 110,
                    "indicator": "dot", "indicatorColor": null,
                    "iconShape": "plate", "iconPack": null, "openAnimation": "bounce",
                    "taskbar": "hidden", "bottomSpace": 2
                },
                "launchAtLogin": false
            })
        );
    }

    #[test]
    fn save_creates_the_missing_config_folder() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("DockMax").join("config").join(FILE_NAME);

        save(&path, &Settings::default()).unwrap();

        assert_eq!(load(&path).issue, None);
    }

    #[test]
    fn save_replaces_an_existing_file() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        save(&path, &Settings::default()).unwrap();

        save(&path, &custom_settings()).unwrap();

        assert_eq!(load(&path).settings, custom_settings());
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn save_rejects_invalid_settings_and_keeps_the_previous_file() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        save(&path, &custom_settings()).unwrap();
        let before = fs::read(&path).unwrap();

        let mut too_big = custom_settings();
        too_big.dock.icon_size = 97;
        assert_eq!(
            save(&path, &too_big),
            Err(SettingsError::IconSizeOutOfRange)
        );
        let future = Settings {
            version: 2,
            ..custom_settings()
        };
        assert_eq!(save(&path, &future), Err(SettingsError::UnsupportedVersion));

        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);

        for _ in 0..3 {
            save(&path, &custom_settings()).unwrap();
            save(&path, &Settings::default()).unwrap();
        }

        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn failed_rename_removes_the_temp_file_and_keeps_the_target() {
        // A folder (with content) at the target path can't be replaced by a
        // file, so the write to the temp file succeeds and the rename fails.
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep.txt"), "kept").unwrap();

        let error = save(&path, &Settings::default()).unwrap_err();

        assert!(matches!(error, SettingsError::Write(_)), "{error:?}");
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
        assert!(path.is_dir());
        assert_eq!(fs::read_to_string(path.join("keep.txt")).unwrap(), "kept");
    }

    /// Saves `custom_settings()` over a file holding the defaults, with
    /// `before_rename` run between writing the temp file and renaming it.
    fn save_over_defaults(
        dir: &TempDir,
        before_rename: impl FnOnce(&Path) -> io::Result<()>,
    ) -> (Vec<u8>, Result<(), SettingsError>) {
        let path = settings_path(dir);
        save(&path, &Settings::default()).unwrap();
        let before = fs::read(&path).unwrap();
        (before, save_with(&path, &custom_settings(), before_rename))
    }

    #[test]
    fn target_keeps_the_old_content_until_the_temp_file_is_complete() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        let mut seen = None;

        let (before, result) = save_over_defaults(&dir, |temp| {
            seen = Some((fs::read(&path)?, fs::read(temp)?, temp.to_path_buf()));
            Ok(())
        });

        assert_eq!(result, Ok(()));
        let (target_then, temp_then, temp) = seen.expect("the hook runs before the rename");
        // Written in place, the target would already hold the new content.
        assert_eq!(target_then, before);
        assert_eq!(temp_then, fs::read(&path).unwrap());
        assert_eq!(load(&path).settings, custom_settings());
        assert_eq!(temp.parent(), path.parent());
        assert!(!temp.exists());
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn failure_before_the_rename_keeps_the_previous_file_byte_for_byte() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);

        let (before, result) =
            save_over_defaults(&dir, |_| Err(io::Error::from(io::ErrorKind::StorageFull)));

        assert_eq!(
            result,
            Err(SettingsError::Write(io::ErrorKind::StorageFull))
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn write_error_message_has_no_path() {
        let dir = tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir(&path).unwrap();

        let message = save(&path, &Settings::default()).unwrap_err().to_string();

        assert!(message.starts_with("settings file could not be written"));
        assert!(
            !message.contains('/') && !message.contains('\\'),
            "{message}"
        );
        assert!(!message.contains(FILE_NAME), "{message}");
    }

    #[test]
    fn load_issue_messages_have_no_path() {
        let issues = [
            LoadIssue::Corrupt,
            LoadIssue::Invalid,
            LoadIssue::Unreadable(io::ErrorKind::PermissionDenied),
        ];
        for issue in issues {
            let message = issue.to_string();
            assert!(
                !message.contains('/') && !message.contains('\\'),
                "{message}"
            );
        }
    }

    #[test]
    fn temp_file_sits_next_to_the_target_with_a_unique_name() {
        let path = Path::new("config").join(FILE_NAME);

        let first = temp_path(&path).unwrap();
        let second = temp_path(&path).unwrap();

        assert_eq!(first.parent(), path.parent());
        assert_ne!(first, path);
        assert_ne!(first, second);
        let name = first.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("settings.json.") && name.ends_with(".tmp"),
            "{name}"
        );
        assert_eq!(
            temp_path(Path::new("")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
