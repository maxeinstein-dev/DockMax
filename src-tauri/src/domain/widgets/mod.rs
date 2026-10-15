//! Widget instances: the closed set of kinds, each kind's config with its
//! default and rules, the instance the interface gets, and the errors it
//! sees (spec 0003 §2.1, §2.5–§2.7).
//!
//! Must match `src/core/widgets.ts`; the JSON in `tests/fixtures/widgets/`
//! pins the shapes on both sides. Nothing here touches the database, Tauri
//! or Windows.

pub mod clock;
pub mod files;
pub mod note;
pub mod shortcut;
pub mod time_progress;

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use clock::ClockConfig;
pub use files::FilesConfig;
pub use note::NoteConfig;
pub use shortcut::ShortcutConfig;
pub use time_progress::TimeProgressConfig;

/// Most widget instances the dock holds (spec 0003 §2.1 item 3).
pub const MAX_WIDGETS: usize = 12;

/// Largest note text accepted, in UTF-8 bytes (spec 0003 §2.2, Note).
pub const MAX_NOTE_BYTES: usize = 64 * 1024;

/// Longest instance label accepted, in characters (spec 0003 §2.1 item 6).
pub const MAX_LABEL_CHARS: usize = 40;

/// Key in the `meta` table of the recovery notice: set to
/// [`RECOVERY_NOTICE_PENDING`] when a corrupt database was set aside, and
/// removed when the user dismisses the notice (spec 0003 §2.4 item 7).
pub const RECOVERY_NOTICE_KEY: &str = "recovery_notice";

/// Value of [`RECOVERY_NOTICE_KEY`] while the notice is shown.
pub const RECOVERY_NOTICE_PENDING: &str = "pending";

/// The kinds of widget. A closed set: there are no third-party widgets.
/// Stored in the `kind` column with the same kebab-case names as in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WidgetKind {
    Clock,
    TimeProgress,
    Note,
    Shortcut,
    Files,
}

impl WidgetKind {
    pub const ALL: [WidgetKind; 5] = [
        WidgetKind::Clock,
        WidgetKind::TimeProgress,
        WidgetKind::Note,
        WidgetKind::Shortcut,
        WidgetKind::Files,
    ];

    /// Whether `add_widget` may create this kind in this phase. Shortcut and
    /// Files exist only in the domain until phase 3 (spec 0003 §2.5).
    pub const fn creatable(self) -> bool {
        match self {
            WidgetKind::Clock | WidgetKind::TimeProgress | WidgetKind::Note => true,
            WidgetKind::Shortcut | WidgetKind::Files => false,
        }
    }

    /// Format version of this kind's config, written next to it in
    /// `config_version`. A stored config with another version is not read.
    pub const fn config_version(self) -> u32 {
        match self {
            WidgetKind::Clock
            | WidgetKind::TimeProgress
            | WidgetKind::Note
            | WidgetKind::Shortcut
            | WidgetKind::Files => 1,
        }
    }

    /// The kebab-case name, as in JSON and in the `kind` column.
    pub const fn as_str(self) -> &'static str {
        match self {
            WidgetKind::Clock => "clock",
            WidgetKind::TimeProgress => "time-progress",
            WidgetKind::Note => "note",
            WidgetKind::Shortcut => "shortcut",
            WidgetKind::Files => "files",
        }
    }

    /// The kind named `name` (kebab-case, exact); `None` for a kind this
    /// build doesn't know, such as one from a newer version.
    pub fn from_name(name: &str) -> Option<WidgetKind> {
        WidgetKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == name)
    }
}

impl fmt::Display for WidgetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable ID of a widget instance: a UUID v7 in lowercase hyphenated form,
/// made in Rust when the instance is created and never reused or changed
/// (spec 0003 §2.4 item 5).
///
/// Text from the interface only becomes a `WidgetId` through
/// [`WidgetId::parse`], so a malformed ID never reaches the database.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WidgetId(String);

impl WidgetId {
    /// A new ID. UUID v7 starts with the creation time, so IDs made by this
    /// process sort in creation order.
    ///
    /// Deliberately no `Default`: a fresh random ID is not a default value.
    pub fn new() -> WidgetId {
        WidgetId(Uuid::now_v7().hyphenated().to_string())
    }

    /// Accepts only the exact form [`WidgetId::new`] makes: a v7 UUID,
    /// lowercase and hyphenated. Anything else can't name an instance, so it
    /// is [`WidgetError::NotFound`].
    pub fn parse(text: &str) -> Result<WidgetId, WidgetError> {
        let uuid = Uuid::try_parse(text).map_err(|_| WidgetError::NotFound)?;
        let canonical = uuid.hyphenated().to_string();
        if uuid.get_version_num() != 7 || canonical != text {
            return Err(WidgetError::NotFound);
        }
        Ok(WidgetId(canonical))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for WidgetId {
    type Error = WidgetError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        WidgetId::parse(&text)
    }
}

impl From<WidgetId> for String {
    fn from(id: WidgetId) -> Self {
        id.0
    }
}

/// A kind's config, tagged with the kind: `{"kind": "clock", "config":
/// {...}}`. The tag makes a config of the wrong kind impossible to build,
/// and lets `update_widget` check it against the instance's kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "config", rename_all = "kebab-case")]
pub enum WidgetConfig {
    Clock(ClockConfig),
    TimeProgress(TimeProgressConfig),
    Note(NoteConfig),
    Shortcut(ShortcutConfig),
    Files(FilesConfig),
}

impl WidgetConfig {
    pub fn kind(&self) -> WidgetKind {
        match self {
            WidgetConfig::Clock(_) => WidgetKind::Clock,
            WidgetConfig::TimeProgress(_) => WidgetKind::TimeProgress,
            WidgetConfig::Note(_) => WidgetKind::Note,
            WidgetConfig::Shortcut(_) => WidgetKind::Shortcut,
            WidgetConfig::Files(_) => WidgetKind::Files,
        }
    }

    /// The config a new instance of `kind` gets, and the one an instance
    /// falls back to when its stored config can't be read.
    pub fn default_for(kind: WidgetKind) -> WidgetConfig {
        match kind {
            WidgetKind::Clock => WidgetConfig::Clock(ClockConfig::default()),
            WidgetKind::TimeProgress => WidgetConfig::TimeProgress(TimeProgressConfig::default()),
            WidgetKind::Note => WidgetConfig::Note(NoteConfig::default()),
            WidgetKind::Shortcut => WidgetConfig::Shortcut(ShortcutConfig::default()),
            WidgetKind::Files => WidgetConfig::Files(FilesConfig::default()),
        }
    }

    /// Checks the rules serde can't express, by kind.
    pub fn validate(&self) -> Result<(), WidgetError> {
        match self {
            WidgetConfig::Clock(config) => config.validate(),
            WidgetConfig::TimeProgress(config) => config.validate(),
            WidgetConfig::Note(config) => config.validate(),
            WidgetConfig::Shortcut(config) => config.validate(),
            WidgetConfig::Files(config) => config.validate(),
        }
    }
}

/// Checks an instance label: `None` (the kind's name) or up to
/// [`MAX_LABEL_CHARS`] characters.
pub fn validate_label(label: Option<&str>) -> Result<(), WidgetError> {
    // L2: the validation lane implements this (length, no control
    // characters); until then every label is accepted.
    let _ = label;
    Ok(())
}

/// The `config` column for `config`: the JSON of the kind's config alone,
/// without the kind tag, which has its own column. Read back with
/// [`decode_config`].
pub fn encode_config(config: &WidgetConfig) -> String {
    let json = match config {
        WidgetConfig::Clock(config) => serde_json::to_string(config),
        WidgetConfig::TimeProgress(config) => serde_json::to_string(config),
        WidgetConfig::Note(config) => serde_json::to_string(config),
        WidgetConfig::Shortcut(config) => serde_json::to_string(config),
        WidgetConfig::Files(config) => serde_json::to_string(config),
    };
    // Plain structs with string keys: serializing them can't fail.
    json.expect("widget configs always serialize")
}

/// Reads a stored config: `json` from the `config` column, written with
/// `config_version` for `kind`.
///
/// Missing fields take their defaults and unknown fields are ignored, as in
/// `settings.json`, so a config from an older build of the same version
/// still reads. A different version or unreadable JSON is
/// [`WidgetError::InvalidConfig`]; values that read but break a rule give
/// that rule's error. Either way the caller falls back to
/// [`WidgetConfig::default_for`] (spec 0003 §2.4 item 6).
pub fn decode_config(
    kind: WidgetKind,
    config_version: u32,
    json: &str,
) -> Result<WidgetConfig, WidgetError> {
    if config_version != kind.config_version() {
        return Err(WidgetError::InvalidConfig);
    }
    let config = match kind {
        WidgetKind::Clock => serde_json::from_str(json).map(WidgetConfig::Clock),
        WidgetKind::TimeProgress => serde_json::from_str(json).map(WidgetConfig::TimeProgress),
        WidgetKind::Note => serde_json::from_str(json).map(WidgetConfig::Note),
        WidgetKind::Shortcut => serde_json::from_str(json).map(WidgetConfig::Shortcut),
        WidgetKind::Files => serde_json::from_str(json).map(WidgetConfig::Files),
    }
    .map_err(|_| WidgetError::InvalidConfig)?;
    config.validate()?;
    Ok(config)
}

/// A widget in the dock, as the interface gets it:
/// `{"id": "...", "label": null, "kind": "clock", "config": {...}}`.
///
/// There is no position: a list of instances is always in dock order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetInstance {
    pub id: WidgetId,
    /// Shown instead of the kind's name; `None` (serialized as `null`) uses
    /// the kind's name.
    pub label: Option<String>,
    #[serde(flatten)]
    pub config: WidgetConfig,
}

impl WidgetInstance {
    pub fn kind(&self) -> WidgetKind {
        self.config.kind()
    }
}

/// Whether the widget database can be used in this run (spec 0003 §2.4
/// item 7): `{"state": "ok"}`, `{"state": "recovered"}` or
/// `{"state": "unavailable", "reason": "newerVersion"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StorageStatus {
    Ok,
    /// A corrupt database was set aside and a new one made; the notice
    /// stays until the user dismisses it.
    Recovered,
    /// The widgets can't be used in this run; the database file is left
    /// untouched. The dock shows only Settings and About.
    Unavailable {
        reason: UnavailableReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UnavailableReason {
    /// The database was written by a newer DockMax.
    NewerVersion,
    /// A migration failed; the file was left as it was.
    MigrationFailed,
    /// The database could not be opened or created.
    OpenFailed,
}

/// What `list_widgets` returns and `widgets-changed` carries: the storage
/// status and every instance, in dock order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetsSnapshot {
    pub storage: StorageStatus,
    pub widgets: Vec<WidgetInstance>,
}

/// Where `move_widget` moves an instance: one place towards the start
/// (`up`) or the end (`down`) of the dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveDirection {
    Up,
    Down,
}

/// Why a widget change was rejected or could not be saved.
///
/// Messages name only the kind of failure, never a path, a label, a config
/// value or note text, so they are safe to log and to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WidgetError {
    #[error("widget limit reached")]
    LimitReached,
    #[error("widget kind cannot be created")]
    NotCreatable,
    #[error("widget instance not found")]
    NotFound,
    #[error("config kind does not match the widget")]
    KindMismatch,
    #[error("invalid widget label")]
    InvalidLabel,
    #[error("unknown time zone")]
    InvalidTimeZone,
    #[error("invalid progress periods")]
    InvalidPeriods,
    #[error("invalid workday hours")]
    InvalidWorkday,
    #[error("invalid shortcut target")]
    InvalidTarget,
    #[error("invalid folder")]
    InvalidFolder,
    #[error("note too large")]
    NoteTooLarge,
    #[error("widget config could not be read")]
    InvalidConfig,
    #[error("widget storage unavailable")]
    Unavailable,
    #[error("widget storage could not be written")]
    WriteFailed,
}

impl WidgetError {
    /// The code the interface gets for this error, to show a translated
    /// message.
    pub fn code(&self) -> WidgetErrorCode {
        match self {
            WidgetError::LimitReached => WidgetErrorCode::LimitReached,
            WidgetError::NotCreatable => WidgetErrorCode::NotCreatable,
            WidgetError::NotFound => WidgetErrorCode::NotFound,
            WidgetError::KindMismatch => WidgetErrorCode::KindMismatch,
            WidgetError::InvalidLabel => WidgetErrorCode::InvalidLabel,
            WidgetError::InvalidTimeZone => WidgetErrorCode::InvalidTimeZone,
            WidgetError::InvalidPeriods => WidgetErrorCode::InvalidPeriods,
            WidgetError::InvalidWorkday => WidgetErrorCode::InvalidWorkday,
            WidgetError::InvalidTarget => WidgetErrorCode::InvalidTarget,
            WidgetError::InvalidFolder => WidgetErrorCode::InvalidFolder,
            WidgetError::NoteTooLarge => WidgetErrorCode::NoteTooLarge,
            WidgetError::InvalidConfig => WidgetErrorCode::InvalidConfig,
            WidgetError::Unavailable => WidgetErrorCode::Unavailable,
            WidgetError::WriteFailed => WidgetErrorCode::WriteFailed,
        }
    }
}

/// What went wrong with a widget command, as the interface sees it. Must
/// match `WidgetErrorCode` in `src/core/widgets.ts` (which adds `notReady`
/// for failures before a command runs); carries no path and no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetErrorCode {
    /// The dock already has [`MAX_WIDGETS`] instances.
    LimitReached,
    /// The kind can't be created in this phase.
    NotCreatable,
    /// No instance has this ID.
    NotFound,
    /// The config is for another kind than the instance.
    KindMismatch,
    /// The label is too long or has control characters.
    InvalidLabel,
    /// The time zone is not in the IANA database.
    InvalidTimeZone,
    /// No period, or a period listed twice.
    InvalidPeriods,
    /// Workday hours not `HH:mm`, or the start not before the end.
    InvalidWorkday,
    /// The shortcut target is not an absolute path or an http(s) URL.
    InvalidTarget,
    /// The folder is not an absolute path.
    InvalidFolder,
    /// The note is over [`MAX_NOTE_BYTES`].
    NoteTooLarge,
    /// A stored config has another version or is not valid JSON.
    InvalidConfig,
    /// The widget database can't be used in this run.
    Unavailable,
    /// The change could not be saved; nothing changed.
    WriteFailed,
}

impl WidgetErrorCode {
    pub const ALL: [WidgetErrorCode; 14] = [
        WidgetErrorCode::LimitReached,
        WidgetErrorCode::NotCreatable,
        WidgetErrorCode::NotFound,
        WidgetErrorCode::KindMismatch,
        WidgetErrorCode::InvalidLabel,
        WidgetErrorCode::InvalidTimeZone,
        WidgetErrorCode::InvalidPeriods,
        WidgetErrorCode::InvalidWorkday,
        WidgetErrorCode::InvalidTarget,
        WidgetErrorCode::InvalidFolder,
        WidgetErrorCode::NoteTooLarge,
        WidgetErrorCode::InvalidConfig,
        WidgetErrorCode::Unavailable,
        WidgetErrorCode::WriteFailed,
    ];
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::clock::HourCycle;
    use super::shortcut::ShortcutTarget;
    use super::time_progress::{Period, WeekStart, Workday};
    use super::*;

    const ID: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071";

    fn to_json<T: Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    fn instance(config: WidgetConfig) -> WidgetInstance {
        WidgetInstance {
            id: WidgetId::parse(ID).unwrap(),
            label: None,
            config,
        }
    }

    #[test]
    fn only_clock_time_progress_and_note_are_creatable() {
        let creatable: Vec<_> = WidgetKind::ALL
            .into_iter()
            .filter(|kind| kind.creatable())
            .collect();

        assert_eq!(
            creatable,
            [
                WidgetKind::Clock,
                WidgetKind::TimeProgress,
                WidgetKind::Note
            ]
        );
    }

    #[test]
    fn every_config_is_at_version_1() {
        for kind in WidgetKind::ALL {
            assert_eq!(kind.config_version(), 1, "{kind}");
        }
    }

    #[test]
    fn kind_names_are_kebab_case_in_json_and_in_the_column() {
        let names = ["clock", "time-progress", "note", "shortcut", "files"];
        for (kind, name) in WidgetKind::ALL.into_iter().zip(names) {
            assert_eq!(to_json(&kind), name);
            assert_eq!(kind.as_str(), name);
            assert_eq!(WidgetKind::from_name(name), Some(kind));
            assert_eq!(
                serde_json::from_value::<WidgetKind>(json!(name)).unwrap(),
                kind
            );
        }
        assert_eq!(WidgetKind::from_name("timeProgress"), None);
        assert_eq!(WidgetKind::from_name("Clock"), None);
        assert_eq!(WidgetKind::from_name("weather"), None);
    }

    #[test]
    fn default_config_matches_its_kind() {
        for kind in WidgetKind::ALL {
            let config = WidgetConfig::default_for(kind);
            assert_eq!(config.kind(), kind);
            assert_eq!(config.validate(), Ok(()), "{kind}");
        }
    }

    #[test]
    fn new_ids_are_lowercase_v7_and_parse_back() {
        let first = WidgetId::new();
        let second = WidgetId::new();

        assert_ne!(first, second);
        assert_eq!(WidgetId::parse(first.as_str()), Ok(first.clone()));
        assert_eq!(
            Uuid::parse_str(first.as_str()).unwrap().get_version_num(),
            7
        );
        assert!(first < second, "v7 IDs sort in creation order");
    }

    #[test]
    fn ids_other_than_canonical_v7_are_not_found() {
        let rejected = [
            "",
            "clock",
            "019A1B2C-3D4E-7F60-8A1B-2C3D4E5F6071", // uppercase
            "019a1b2c3d4e7f608a1b2c3d4e5f6071",     // no hyphens
            "{019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071}",
            "urn:uuid:019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
            "019a1b2c-3d4e-4f60-8a1b-2c3d4e5f6071", // v4
            " 019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
        ];
        for text in rejected {
            assert_eq!(
                WidgetId::parse(text),
                Err(WidgetError::NotFound),
                "{text:?}"
            );
            assert!(serde_json::from_value::<WidgetId>(json!(text)).is_err());
        }
    }

    #[test]
    fn instances_serialize_flat_with_the_kind_and_a_null_label() {
        let clock = instance(WidgetConfig::Clock(ClockConfig::default()));

        assert_eq!(
            to_json(&clock),
            json!({
                "id": ID,
                "label": null,
                "kind": "clock",
                "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false }
            })
        );
        assert_eq!(
            serde_json::from_value::<WidgetInstance>(to_json(&clock)).unwrap(),
            clock
        );
    }

    #[test]
    fn default_configs_serialize_to_the_contract() {
        let cases = [
            (
                WidgetKind::TimeProgress,
                json!({
                    "periods": ["day", "week", "month", "year"],
                    "weekStart": "monday",
                    "workday": { "start": "09:00", "end": "18:00" }
                }),
            ),
            (WidgetKind::Note, json!({})),
            (
                WidgetKind::Shortcut,
                json!({ "target": { "type": "path", "path": "C:\\" } }),
            ),
            (WidgetKind::Files, json!({ "folder": "C:\\" })),
        ];
        for (kind, config) in cases {
            let instance = instance(WidgetConfig::default_for(kind));
            assert_eq!(
                to_json(&instance),
                json!({ "id": ID, "label": null, "kind": kind.as_str(), "config": config }),
                "{kind}"
            );
        }
    }

    #[test]
    fn non_default_values_use_the_contract_names() {
        let clock = ClockConfig {
            time_zone: Some("America/Sao_Paulo".to_owned()),
            hour_cycle: HourCycle::H12,
            show_seconds: true,
        };
        assert_eq!(
            to_json(&clock),
            json!({ "timeZone": "America/Sao_Paulo", "hourCycle": "h12", "showSeconds": true })
        );

        let progress = TimeProgressConfig {
            periods: vec![Period::Workday, Period::Day],
            week_start: WeekStart::Sunday,
            workday: Workday::default(),
        };
        assert_eq!(to_json(&progress)["periods"], json!(["workday", "day"]));
        assert_eq!(to_json(&progress)["weekStart"], "sunday");

        let url = ShortcutTarget::Url {
            url: "https://example.com/".to_owned(),
        };
        assert_eq!(
            to_json(&url),
            json!({ "type": "url", "url": "https://example.com/" })
        );
    }

    #[test]
    fn storage_status_is_tagged_by_state() {
        let cases = [
            (StorageStatus::Ok, json!({ "state": "ok" })),
            (StorageStatus::Recovered, json!({ "state": "recovered" })),
            (
                StorageStatus::Unavailable {
                    reason: UnavailableReason::NewerVersion,
                },
                json!({ "state": "unavailable", "reason": "newerVersion" }),
            ),
            (
                StorageStatus::Unavailable {
                    reason: UnavailableReason::MigrationFailed,
                },
                json!({ "state": "unavailable", "reason": "migrationFailed" }),
            ),
            (
                StorageStatus::Unavailable {
                    reason: UnavailableReason::OpenFailed,
                },
                json!({ "state": "unavailable", "reason": "openFailed" }),
            ),
        ];
        for (status, expected) in cases {
            assert_eq!(to_json(&status), expected);
            assert_eq!(
                serde_json::from_value::<StorageStatus>(expected).unwrap(),
                status
            );
        }
    }

    #[test]
    fn snapshot_and_move_direction_use_the_contract_names() {
        let snapshot = WidgetsSnapshot {
            storage: StorageStatus::Ok,
            widgets: vec![],
        };
        assert_eq!(
            to_json(&snapshot),
            json!({ "storage": { "state": "ok" }, "widgets": [] })
        );
        assert_eq!(to_json(&MoveDirection::Up), "up");
        assert_eq!(to_json(&MoveDirection::Down), "down");
        assert!(serde_json::from_value::<MoveDirection>(json!("Up")).is_err());
    }

    #[test]
    fn tagged_config_round_trips_and_rejects_an_unknown_kind() {
        let config = WidgetConfig::Note(NoteConfig {});
        assert_eq!(to_json(&config), json!({ "kind": "note", "config": {} }));
        assert_eq!(
            serde_json::from_value::<WidgetConfig>(to_json(&config)).unwrap(),
            config
        );
        assert!(
            serde_json::from_value::<WidgetConfig>(json!({ "kind": "weather", "config": {} }))
                .is_err()
        );
    }

    #[test]
    fn encoded_configs_decode_back() {
        for kind in WidgetKind::ALL {
            let config = WidgetConfig::default_for(kind);
            let json = encode_config(&config);
            assert!(!json.contains("\"kind\""), "{json}");
            assert_eq!(
                decode_config(kind, kind.config_version(), &json),
                Ok(config)
            );
        }
    }

    #[test]
    fn decoding_another_version_is_invalid_config() {
        for kind in WidgetKind::ALL {
            let json = encode_config(&WidgetConfig::default_for(kind));
            for version in [0, 2, u32::MAX] {
                assert_eq!(
                    decode_config(kind, version, &json),
                    Err(WidgetError::InvalidConfig),
                    "{kind} v{version}"
                );
            }
        }
    }

    #[test]
    fn decoding_unreadable_json_is_invalid_config() {
        let unreadable = [
            "",
            "not json",
            "null",
            "\"clock\"",
            "42",
            "{\"timeZone\":",
            "{\"hourCycle\":\"h24\"}",
        ];
        for json in unreadable {
            assert_eq!(
                decode_config(WidgetKind::Clock, 1, json),
                Err(WidgetError::InvalidConfig),
                "{json:?}"
            );
        }
        assert_eq!(
            decode_config(
                WidgetKind::Shortcut,
                1,
                r#"{"target":{"type":"ftp","url":"x"}}"#
            ),
            Err(WidgetError::InvalidConfig)
        );
    }

    #[test]
    fn decoding_fills_missing_fields_and_ignores_unknown_ones() {
        assert_eq!(
            decode_config(
                WidgetKind::Clock,
                1,
                r#"{"showSeconds":true,"color":"red"}"#
            ),
            Ok(WidgetConfig::Clock(ClockConfig {
                show_seconds: true,
                ..ClockConfig::default()
            }))
        );
        assert_eq!(
            decode_config(
                WidgetKind::TimeProgress,
                1,
                r#"{"workday":{"end":"17:00"}}"#
            ),
            Ok(WidgetConfig::TimeProgress(TimeProgressConfig {
                workday: Workday {
                    start: "09:00".to_owned(),
                    end: "17:00".to_owned(),
                },
                ..TimeProgressConfig::default()
            }))
        );
        for kind in WidgetKind::ALL {
            assert_eq!(
                decode_config(kind, 1, "{}"),
                Ok(WidgetConfig::default_for(kind)),
                "{kind}"
            );
        }
    }

    #[test]
    fn error_messages_carry_no_path_or_value() {
        let errors = [
            WidgetError::LimitReached,
            WidgetError::NotCreatable,
            WidgetError::NotFound,
            WidgetError::KindMismatch,
            WidgetError::InvalidLabel,
            WidgetError::InvalidTimeZone,
            WidgetError::InvalidPeriods,
            WidgetError::InvalidWorkday,
            WidgetError::InvalidTarget,
            WidgetError::InvalidFolder,
            WidgetError::NoteTooLarge,
            WidgetError::InvalidConfig,
            WidgetError::Unavailable,
            WidgetError::WriteFailed,
        ];
        let codes: Vec<_> = errors.iter().map(WidgetError::code).collect();
        assert_eq!(codes, WidgetErrorCode::ALL, "one code per error, in order");
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
    fn error_codes_serialize_to_the_names_the_interface_expects() {
        // `WidgetErrorCode` in src/core/widgets.ts, minus the TS-only `notReady`.
        let names = [
            "limitReached",
            "notCreatable",
            "notFound",
            "kindMismatch",
            "invalidLabel",
            "invalidTimeZone",
            "invalidPeriods",
            "invalidWorkday",
            "invalidTarget",
            "invalidFolder",
            "noteTooLarge",
            "invalidConfig",
            "unavailable",
            "writeFailed",
        ];
        let serialized: Vec<_> = WidgetErrorCode::ALL.iter().map(to_json).collect();
        assert_eq!(serialized, names);
    }
}
