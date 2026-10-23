//! The programs pinned to the dock (spec 0006): what is kept about each, the
//! limits, and why a change to the list is refused. Pure rules: the list is
//! stored in `persistence/apps_repo.rs` and the programs themselves are read
//! from Windows in `windows/apps.rs`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::widgets::{StorageStatus, WidgetError, WidgetId};

/// The most programs the dock holds pinned.
pub const MAX_APPS: usize = 40;

/// The longest id of a program the dock accepts, in characters. The shell's
/// parsing names are short (an AUMID, or a folder and a path).
pub const MAX_APP_ID_CHARS: usize = 512;

/// The longest name kept for a program, in characters.
pub const MAX_APP_NAME_CHARS: usize = 200;

/// Whether `id` can be the parsing name of a program of the shell's Apps
/// folder: one line of text with no control character, no space at either
/// end and a bounded length. Checked before an id from the interface is used
/// for anything; the `shell:AppsFolder\` prefix then keeps the name inside
/// that folder.
pub fn is_valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= MAX_APP_ID_CHARS
        && id.trim() == id
        && !id.chars().any(char::is_control)
}

/// Whether `name` can be kept as the name of a program.
pub fn is_valid_app_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= MAX_APP_NAME_CHARS
        && !name.chars().any(char::is_control)
}

/// One pinned program, as the interface sees it:
/// `{"id": "...", "appId": "...", "name": "..."}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedApp {
    /// The dock's own id for the item (UUID v7), what the commands take.
    pub id: WidgetId,
    /// The parsing name of the program in the Apps folder: what opens it.
    pub app_id: String,
    /// The name shown for the program.
    pub name: String,
}

/// What `list_apps` returns and `apps-changed` carries: the storage status
/// (the one of the widgets, in the same database) and the pinned programs in
/// dock order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppsSnapshot {
    pub storage: StorageStatus,
    pub apps: Vec<PinnedApp>,
}

/// Why a change to the pinned programs was refused or could not be saved.
/// Messages name only the kind of failure, never an id or a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AppError {
    #[error("pinned app limit reached")]
    LimitReached,
    #[error("pinned app not found")]
    NotFound,
    #[error("app already pinned")]
    AlreadyPinned,
    #[error("not a program of the Apps folder")]
    InvalidApp,
    #[error("app could not be opened")]
    LaunchFailed,
    #[error("app storage unavailable")]
    Unavailable,
    #[error("app storage could not be written")]
    WriteFailed,
}

impl From<WidgetError> for AppError {
    /// The storage errors of the shared database keep their meaning; any
    /// other (a widget rule) can't come from a change to the apps.
    fn from(error: WidgetError) -> Self {
        match error {
            WidgetError::NotFound => AppError::NotFound,
            WidgetError::Unavailable => AppError::Unavailable,
            _ => AppError::WriteFailed,
        }
    }
}

impl AppError {
    /// The code the interface gets, to show a translated message.
    pub fn code(self) -> AppErrorCode {
        match self {
            AppError::LimitReached => AppErrorCode::LimitReached,
            AppError::NotFound => AppErrorCode::NotFound,
            AppError::AlreadyPinned => AppErrorCode::AlreadyPinned,
            AppError::InvalidApp => AppErrorCode::InvalidApp,
            AppError::LaunchFailed => AppErrorCode::LaunchFailed,
            AppError::Unavailable => AppErrorCode::Unavailable,
            AppError::WriteFailed => AppErrorCode::WriteFailed,
        }
    }
}

/// An [`AppError`] as the interface sees it; must match `AppErrorCode` in
/// `src/core/apps.ts` (which adds `notReady`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppErrorCode {
    LimitReached,
    NotFound,
    AlreadyPinned,
    InvalidApp,
    LaunchFailed,
    Unavailable,
    WriteFailed,
}

impl AppErrorCode {
    /// Every code, in the order of the contract file.
    #[cfg(test)]
    pub const ALL: [AppErrorCode; 7] = [
        AppErrorCode::LimitReached,
        AppErrorCode::NotFound,
        AppErrorCode::AlreadyPinned,
        AppErrorCode::InvalidApp,
        AppErrorCode::LaunchFailed,
        AppErrorCode::Unavailable,
        AppErrorCode::WriteFailed,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_one_line_of_text_of_a_bounded_length() {
        assert!(is_valid_app_id(
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"
        ));
        assert!(is_valid_app_id(
            r"{6D809377-6AF0-444B-8957-A3773F02200E}\Microsoft VS Code\Code.exe"
        ));
        assert!(!is_valid_app_id(""));
        assert!(!is_valid_app_id(" notepad"));
        assert!(!is_valid_app_id("notepad "));
        assert!(!is_valid_app_id("note\npad"));
        assert!(!is_valid_app_id("note\0pad"));
        assert!(!is_valid_app_id(&"a".repeat(MAX_APP_ID_CHARS + 1)));
        assert!(is_valid_app_id(&"a".repeat(MAX_APP_ID_CHARS)));
    }

    #[test]
    fn a_name_shows_something_and_is_one_line() {
        assert!(is_valid_app_name("Calculadora"));
        assert!(!is_valid_app_name("   "));
        assert!(!is_valid_app_name("Calc\nulator"));
        assert!(!is_valid_app_name(&"a".repeat(MAX_APP_NAME_CHARS + 1)));
    }

    #[test]
    fn the_storage_errors_keep_their_meaning() {
        assert_eq!(
            AppError::from(WidgetError::Unavailable),
            AppError::Unavailable
        );
        assert_eq!(AppError::from(WidgetError::NotFound), AppError::NotFound);
        assert_eq!(
            AppError::from(WidgetError::WriteFailed),
            AppError::WriteFailed
        );
        assert_eq!(
            AppError::from(WidgetError::InvalidLabel),
            AppError::WriteFailed
        );
    }

    #[test]
    fn error_messages_never_carry_an_id_or_a_name() {
        for error in [
            AppError::LimitReached,
            AppError::NotFound,
            AppError::AlreadyPinned,
            AppError::InvalidApp,
            AppError::LaunchFailed,
            AppError::Unavailable,
            AppError::WriteFailed,
        ] {
            let message = error.to_string();
            assert!(
                !message.contains('\\') && !message.contains('!'),
                "{message}"
            );
        }
    }

    /// The files the TypeScript side checks against its own types
    /// (`src/core/apps.test.ts`): changing one side alone breaks one of the
    /// two tests.
    fn fixture(name: &str) -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/apps")
            .join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    #[test]
    fn error_codes_serialize_to_the_names_in_the_contract_file() {
        let codes: Vec<serde_json::Value> = AppErrorCode::ALL
            .iter()
            .map(|code| serde_json::to_value(code).unwrap())
            .collect();

        assert_eq!(serde_json::Value::Array(codes), fixture("error-codes.json"));
    }

    #[test]
    fn the_snapshot_sample_reads_and_writes_back_the_same() {
        let expected = fixture("snapshot-sample.json");

        let snapshot: AppsSnapshot = serde_json::from_value(expected.clone()).unwrap();

        assert_eq!(snapshot.apps.len(), 2);
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);
        for app in &snapshot.apps {
            assert!(is_valid_app_id(&app.app_id) && is_valid_app_name(&app.name));
        }
    }
}
