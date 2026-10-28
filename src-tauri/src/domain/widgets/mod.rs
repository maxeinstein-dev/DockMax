//! Widget instances: the closed set of kinds, each kind's config with its
//! default and rules, the instance the interface gets, and the errors it
//! sees (spec 0003 §2.1, §2.5–§2.7).
//!
//! Must match `src/core/widgets.ts`; the JSON in `tests/fixtures/widgets/`
//! pins the shapes on both sides. Nothing here touches the database, Tauri
//! or Windows.

/// Gives a fieldless enum `ALL`: every variant, in the order listed, which
/// is the order of `tests/fixtures/widgets/enums.json` (spec 0003 §2.6).
/// The list is matched exhaustively, so a variant added to the enum but not
/// to the list doesn't compile; the contract tests then compare `ALL` with
/// `enums.json`, and the TS tests compare that file with the TS types.
macro_rules! closed_set {
    ($ty:ident { $($variant:ident),+ $(,)? }) => {
        impl $ty {
            /// Every value, in the order of `enums.json`.
            #[allow(dead_code, reason = "most are read only by the contract tests")]
            pub const ALL: [$ty; [$(stringify!($variant)),+].len()] = [$($ty::$variant),+];
        }

        // Never called: it only has to compile.
        const _: fn($ty) = |value| match value {
            $($ty::$variant => {})+
        };
    };
}

pub mod calendar;
pub mod clock;
#[cfg(test)]
mod contract_tests;
pub mod files;
pub mod note;
mod path;
pub mod shortcut;
pub mod system;
pub mod time_progress;

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use calendar::CalendarConfig;
pub use clock::ClockConfig;
pub use files::FilesConfig;
pub use note::NoteConfig;
pub use shortcut::ShortcutConfig;
pub use system::{ActivityConfig, NetworkConfig};
pub use time_progress::TimeProgressConfig;

/// Most widget instances the dock holds (spec 0003 §2.1 item 3).
pub const MAX_WIDGETS: usize = 12;

/// Largest note text accepted, in UTF-8 bytes (spec 0003 §2.2, Note).
pub const MAX_NOTE_BYTES: usize = 64 * 1024;

/// Longest instance label accepted, in Unicode scalar values (code points,
/// as `chars()` counts them), not bytes or graphemes (spec 0003 §2.1 item
/// 6). JavaScript's `.length` counts UTF-16 units instead: an emoji outside
/// the BMP is 1 here and 2 there.
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
    Activity,
    Network,
    Calendar,
}

closed_set!(WidgetKind {
    Clock,
    TimeProgress,
    Note,
    Shortcut,
    Files,
    Activity,
    Network,
    Calendar,
});

impl WidgetKind {
    /// Whether `add_widget` may create this kind in this phase. Shortcut and
    /// Files exist only in the domain until phase 3 (spec 0003 §2.5).
    pub const fn creatable(self) -> bool {
        match self {
            WidgetKind::Clock
            | WidgetKind::TimeProgress
            | WidgetKind::Note
            | WidgetKind::Activity
            | WidgetKind::Network
            | WidgetKind::Calendar => true,
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
            | WidgetKind::Files
            | WidgetKind::Activity
            | WidgetKind::Network
            | WidgetKind::Calendar => 1,
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
            WidgetKind::Activity => "activity",
            WidgetKind::Network => "network",
            WidgetKind::Calendar => "calendar",
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
    Activity(ActivityConfig),
    Network(NetworkConfig),
    Calendar(CalendarConfig),
}

impl WidgetConfig {
    pub fn kind(&self) -> WidgetKind {
        match self {
            WidgetConfig::Clock(_) => WidgetKind::Clock,
            WidgetConfig::TimeProgress(_) => WidgetKind::TimeProgress,
            WidgetConfig::Note(_) => WidgetKind::Note,
            WidgetConfig::Shortcut(_) => WidgetKind::Shortcut,
            WidgetConfig::Files(_) => WidgetKind::Files,
            WidgetConfig::Activity(_) => WidgetKind::Activity,
            WidgetConfig::Network(_) => WidgetKind::Network,
            WidgetConfig::Calendar(_) => WidgetKind::Calendar,
        }
    }

    /// How many icons wide the widget is in the dock (spec 0017: 1 to 4).
    pub fn span(&self) -> usize {
        usize::from(self.width())
    }

    /// The width in icons, as the config keeps it; 1 for the kinds that
    /// have only one.
    pub fn width(&self) -> u8 {
        match self {
            WidgetConfig::Clock(_)
            | WidgetConfig::Note(_)
            | WidgetConfig::Shortcut(_)
            | WidgetConfig::Files(_) => 1,
            WidgetConfig::TimeProgress(config) => config.width,
            WidgetConfig::Activity(config) => config.width,
            WidgetConfig::Network(config) => config.width,
            WidgetConfig::Calendar(config) => config.width,
        }
    }

    /// The widths the kind takes (spec 0017 F1b): only the kinds that show
    /// something more when wider take more than one.
    pub fn widths(&self) -> std::ops::RangeInclusive<u8> {
        match self {
            WidgetConfig::Clock(_)
            | WidgetConfig::Note(_)
            | WidgetConfig::Shortcut(_)
            | WidgetConfig::Files(_) => 1..=1,
            WidgetConfig::TimeProgress(_)
            | WidgetConfig::Activity(_)
            | WidgetConfig::Network(_) => system::WIDTHS,
            WidgetConfig::Calendar(_) => calendar::WIDTHS,
        }
    }

    /// The same config at `width` icons; `None` for a width the kind
    /// doesn't take.
    pub fn with_width(&self, width: u8) -> Option<WidgetConfig> {
        if !self.widths().contains(&width) {
            return None;
        }
        let mut config = self.clone();
        match &mut config {
            WidgetConfig::TimeProgress(config) => config.width = width,
            WidgetConfig::Activity(config) => config.width = width,
            WidgetConfig::Network(config) => config.width = width,
            WidgetConfig::Calendar(config) => config.width = width,
            WidgetConfig::Clock(_)
            | WidgetConfig::Note(_)
            | WidgetConfig::Shortcut(_)
            | WidgetConfig::Files(_) => {}
        }
        Some(config)
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
            WidgetKind::Activity => WidgetConfig::Activity(ActivityConfig::default()),
            WidgetKind::Network => WidgetConfig::Network(NetworkConfig::default()),
            WidgetKind::Calendar => WidgetConfig::Calendar(CalendarConfig::default()),
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
            WidgetConfig::Activity(config) => config.validate(),
            WidgetConfig::Network(config) => config.validate(),
            WidgetConfig::Calendar(config) => config.validate(),
        }
    }
}

/// Checks an instance label: `None` (the kind's name) or text of 1 to
/// [`MAX_LABEL_CHARS`] Unicode scalar values (as `chars()` counts them)
/// with at least one visible character and none of these anywhere:
///
/// - a control character (`char::is_control`), or a line or paragraph
///   separator (U+2028, U+2029), which break the line;
/// - a bidirectional formatting character (see [`is_bidi_formatting`]),
///   which can make the label read differently from what was stored;
/// - a noncharacter (see [`is_noncharacter`]), which Unicode reserves for
///   internal use and never assigns.
///
/// Leading and trailing spaces are kept as given.
pub fn validate_label(label: Option<&str>) -> Result<(), WidgetError> {
    let Some(label) = label else {
        return Ok(());
    };
    let length = label.chars().count();
    let valid = (1..=MAX_LABEL_CHARS).contains(&length)
        && !label.chars().any(|c| {
            c.is_control()
                || matches!(c, '\u{2028}' | '\u{2029}')
                || is_bidi_formatting(c)
                || is_noncharacter(c)
        })
        && !label.chars().all(is_invisible);
    if valid {
        Ok(())
    } else {
        Err(WidgetError::InvalidLabel)
    }
}

/// The bidirectional formatting characters: the marks (U+061C ALM, U+200E
/// LRM, U+200F RLM), the embeddings and overrides (U+202A–U+202E) and the
/// isolates (U+2066–U+2069). They reorder the text around them, so a label
/// with one can show something other than its characters in order.
fn is_bidi_formatting(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// The 66 noncharacters: U+FDD0–U+FDEF and the last two code points of
/// every plane (U+xFFFE and U+xFFFF).
fn is_noncharacter(c: char) -> bool {
    matches!(c, '\u{FDD0}'..='\u{FDEF}') || u32::from(c) & 0xFFFE == 0xFFFE
}

/// Whether `c` shows nothing on its own: a label made only of these looks
/// empty. Next to visible characters they are kept, since some are needed
/// there (the zero-width joiner in emoji sequences, variation selectors).
///
/// The list: whitespace (`char::is_whitespace`, which covers U+00A0, U+3000
/// and the other spaces); U+00AD soft hyphen; U+034F combining grapheme
/// joiner; U+115F, U+1160, U+3164 and U+FFA0, the hangul fillers; U+17B4
/// and U+17B5, the khmer inherent vowels; U+180B–U+180F, the mongolian
/// variation selectors and vowel separator; U+200B–U+200D, zero-width
/// space, non-joiner and joiner; U+2060–U+206F, word joiner, invisible
/// operators and deprecated format characters; U+2800 braille pattern
/// blank; U+FE00–U+FE0F, variation selectors; U+FEFF, byte order mark;
/// U+1D173–U+1D17A, musical format characters; U+E0000–U+E0FFF, tags and
/// supplementary variation selectors. The bidi marks are rejected anywhere
/// by [`validate_label`], so they need not be here.
fn is_invisible(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{034F}'
                | '\u{115F}'
                | '\u{1160}'
                | '\u{17B4}'
                | '\u{17B5}'
                | '\u{180B}'..='\u{180F}'
                | '\u{200B}'..='\u{200D}'
                | '\u{2060}'..='\u{206F}'
                | '\u{2800}'
                | '\u{3164}'
                | '\u{FE00}'..='\u{FE0F}'
                | '\u{FEFF}'
                | '\u{FFA0}'
                | '\u{1D173}'..='\u{1D17A}'
                | '\u{E0000}'..='\u{E0FFF}'
        )
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
        WidgetConfig::Activity(config) => serde_json::to_string(config),
        WidgetConfig::Network(config) => serde_json::to_string(config),
        WidgetConfig::Calendar(config) => serde_json::to_string(config),
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
        WidgetKind::Activity => serde_json::from_str(json).map(WidgetConfig::Activity),
        WidgetKind::Network => serde_json::from_str(json).map(WidgetConfig::Network),
        WidgetKind::Calendar => serde_json::from_str(json).map(WidgetConfig::Calendar),
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
    /// Only the tests need it: the app reads the kind from the config.
    #[cfg(test)]
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

closed_set!(UnavailableReason {
    NewerVersion,
    MigrationFailed,
    OpenFailed,
});

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

closed_set!(MoveDirection { Up, Down });

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
    #[error("invalid widget width")]
    InvalidWidth,
    #[error("invalid hidden calendars")]
    InvalidCalendars,
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
            WidgetError::InvalidWidth => WidgetErrorCode::InvalidWidth,
            WidgetError::InvalidCalendars => WidgetErrorCode::InvalidCalendars,
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
    /// The label is empty, too long, shows nothing, or has a control,
    /// bidi formatting or noncharacter code point (see [`validate_label`]).
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
    /// A width the kind doesn't accept.
    InvalidWidth,
    /// A hidden calendar empty, too long or with a control character, or
    /// too many of them.
    InvalidCalendars,
    /// The note is over [`MAX_NOTE_BYTES`].
    NoteTooLarge,
    /// A stored config has another version or is not valid JSON, or the
    /// config `update_widget` got is not exactly the canonical one (a
    /// missing, unknown or mistyped field).
    InvalidConfig,
    /// The widget database can't be used in this run.
    Unavailable,
    /// The change could not be saved; nothing changed.
    WriteFailed,
}

closed_set!(WidgetErrorCode {
    LimitReached,
    NotCreatable,
    NotFound,
    KindMismatch,
    InvalidLabel,
    InvalidTimeZone,
    InvalidPeriods,
    InvalidWorkday,
    InvalidTarget,
    InvalidFolder,
    InvalidWidth,
    InvalidCalendars,
    NoteTooLarge,
    InvalidConfig,
    Unavailable,
    WriteFailed,
});

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::clock::{ClockIconStyle, HourCycle};
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
    fn only_shortcut_and_files_are_not_creatable() {
        let creatable: Vec<_> = WidgetKind::ALL
            .into_iter()
            .filter(|kind| kind.creatable())
            .collect();

        assert_eq!(
            creatable,
            [
                WidgetKind::Clock,
                WidgetKind::TimeProgress,
                WidgetKind::Note,
                WidgetKind::Activity,
                WidgetKind::Network,
                WidgetKind::Calendar
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
        let names = [
            "clock",
            "time-progress",
            "note",
            "shortcut",
            "files",
            "activity",
            "network",
            "calendar",
        ];
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
                "config": {
                    "timeZone": null,
                    "hourCycle": "h23",
                    "showSeconds": false,
                    "iconStyle": "digital"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<WidgetInstance>(to_json(&clock)).unwrap(),
            clock
        );
    }

    #[test]
    fn only_the_kinds_that_show_more_take_another_width() {
        for kind in WidgetKind::ALL {
            let config = WidgetConfig::default_for(kind);
            let most = match kind {
                WidgetKind::TimeProgress | WidgetKind::Activity | WidgetKind::Network => 3,
                // Three was too small to read: two rise to three under
                // the pointer instead.
                WidgetKind::Calendar => 2,
                _ => 1,
            };
            assert_eq!(config.widths(), 1..=most, "{kind}");
            assert_eq!(config.with_width(most + 1), None, "{kind}");
            assert_eq!(config.with_width(0), None, "{kind}");
            let widest = config.with_width(most).unwrap();
            assert_eq!(widest.width(), most);
            assert_eq!(widest.span(), usize::from(most));
            assert_eq!(widest.validate(), Ok(()));
        }
    }

    #[test]
    fn a_progress_stored_before_its_width_reads_one_icon_wide() {
        let read = decode_config(
            WidgetKind::TimeProgress,
            1,
            r#"{"periods":["day"],"weekStart":"monday","workday":{"start":"09:00","end":"18:00"}}"#,
        )
        .unwrap();
        assert_eq!(read.width(), 1);
        assert_eq!(
            decode_config(WidgetKind::TimeProgress, 1, r#"{"width":4}"#),
            Err(WidgetError::InvalidWidth)
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
                    "workday": { "start": "09:00", "end": "18:00" },
                    "width": 1
                }),
            ),
            (WidgetKind::Note, json!({})),
            (
                WidgetKind::Shortcut,
                json!({ "target": { "type": "path", "path": "C:\\" } }),
            ),
            (WidgetKind::Files, json!({ "folder": "C:\\" })),
            (
                WidgetKind::Activity,
                json!({ "show": "both", "style": "bars", "width": 1 }),
            ),
            (WidgetKind::Network, json!({ "width": 2 })),
            (
                WidgetKind::Calendar,
                json!({ "width": 2, "hidden": [], "tasks": true }),
            ),
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
            icon_style: ClockIconStyle::Analog,
        };
        assert_eq!(
            to_json(&clock),
            json!({
                "timeZone": "America/Sao_Paulo",
                "hourCycle": "h12",
                "showSeconds": true,
                "iconStyle": "analog"
            })
        );

        let progress = TimeProgressConfig {
            periods: vec![Period::Workday, Period::Day],
            week_start: WeekStart::Sunday,
            workday: Workday::default(),
            width: 3,
        };
        assert_eq!(to_json(&progress)["width"], 3);
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
        // A clock stored before the icon style existed reads as digital.
        assert_eq!(
            decode_config(
                WidgetKind::Clock,
                1,
                r#"{"timeZone":"Europe/Lisbon","hourCycle":"h12","showSeconds":false}"#
            ),
            Ok(WidgetConfig::Clock(ClockConfig {
                time_zone: Some("Europe/Lisbon".to_owned()),
                hour_cycle: HourCycle::H12,
                icon_style: ClockIconStyle::Digital,
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
            WidgetError::InvalidWidth,
            WidgetError::InvalidCalendars,
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
                !message.contains('/') && !message.contains('\\') && !message.contains(':'),
                "{message}"
            );
        }
    }

    #[test]
    fn decoding_a_readable_config_that_breaks_a_rule_gives_the_rule_error() {
        let cases = [
            (
                WidgetKind::Clock,
                r#"{"timeZone":"Mars/Olympus_Mons"}"#,
                WidgetError::InvalidTimeZone,
            ),
            (
                WidgetKind::Clock,
                r#"{"timeZone":"america/sao_paulo"}"#,
                WidgetError::InvalidTimeZone,
            ),
            (
                WidgetKind::TimeProgress,
                r#"{"periods":[]}"#,
                WidgetError::InvalidPeriods,
            ),
            (
                WidgetKind::TimeProgress,
                r#"{"periods":["day","day"]}"#,
                WidgetError::InvalidPeriods,
            ),
            (
                WidgetKind::TimeProgress,
                r#"{"workday":{"start":"24:00"}}"#,
                WidgetError::InvalidWorkday,
            ),
            (
                WidgetKind::TimeProgress,
                r#"{"workday":{"start":"18:00","end":"09:00"}}"#,
                WidgetError::InvalidWorkday,
            ),
            (
                WidgetKind::Shortcut,
                r#"{"target":{"type":"path","path":"relatorio.pdf"}}"#,
                WidgetError::InvalidTarget,
            ),
            (
                WidgetKind::Shortcut,
                r#"{"target":{"type":"url","url":"file:///C:/Windows"}}"#,
                WidgetError::InvalidTarget,
            ),
            (
                WidgetKind::Files,
                r#"{"folder":"Downloads"}"#,
                WidgetError::InvalidFolder,
            ),
            (
                WidgetKind::Files,
                r#"{"folder":""}"#,
                WidgetError::InvalidFolder,
            ),
        ];
        for (kind, json, error) in cases {
            assert_eq!(decode_config(kind, 1, json), Err(error), "{kind} {json}");
        }
    }

    #[test]
    fn labels_up_to_40_characters_are_accepted() {
        let accepted = [
            "São Paulo",
            "a",
            " Lisboa ",
            "relógio 🕐",
            &"a".repeat(MAX_LABEL_CHARS),
            // 40 characters, 80 bytes: the limit counts characters.
            &"ç".repeat(MAX_LABEL_CHARS),
        ];
        assert_eq!(validate_label(None), Ok(()));
        for label in accepted {
            assert_eq!(validate_label(Some(label)), Ok(()), "{label:?}");
        }
    }

    #[test]
    fn labels_empty_blank_too_long_or_with_control_characters_are_rejected() {
        let rejected = [
            "",
            " ",
            "   ",
            "\u{3000}", // ideographic space
            &"a".repeat(MAX_LABEL_CHARS + 1),
            &"ç".repeat(MAX_LABEL_CHARS + 1),
            "São\nPaulo",
            "São\tPaulo",
            "São Paulo\0",
            "\u{1b}[31mvermelho",
            "fim\u{7f}",
            "c1\u{85}",
        ];
        for label in rejected {
            assert_eq!(
                validate_label(Some(label)),
                Err(WidgetError::InvalidLabel),
                "{label:?}"
            );
        }
    }

    #[test]
    fn labels_with_a_bidi_formatting_character_are_rejected() {
        let bidi = [
            '\u{061C}', // arabic letter mark
            '\u{200E}', // left-to-right mark
            '\u{200F}', // right-to-left mark
            '\u{202A}', // left-to-right embedding
            '\u{202B}', // right-to-left embedding
            '\u{202C}', // pop directional formatting
            '\u{202D}', // left-to-right override
            '\u{202E}', // right-to-left override
            '\u{2066}', // left-to-right isolate
            '\u{2067}', // right-to-left isolate
            '\u{2068}', // first strong isolate
            '\u{2069}', // pop directional isolate
        ];
        for c in bidi {
            for label in [
                format!("São{c} Paulo"),
                format!("{c}abc"),
                format!("abc{c}"),
            ] {
                assert_eq!(
                    validate_label(Some(&label)),
                    Err(WidgetError::InvalidLabel),
                    "{label:?}"
                );
            }
        }
    }

    #[test]
    fn labels_with_a_noncharacter_are_rejected() {
        let nonchars = [
            '\u{FDD0}',
            '\u{FDE7}',
            '\u{FDEF}',
            '\u{FFFE}',
            '\u{FFFF}',
            '\u{1FFFE}',
            '\u{1FFFF}',
            '\u{AFFFE}',
            '\u{10FFFE}',
            '\u{10FFFF}',
        ];
        for c in nonchars {
            let label = format!("Lisboa{c}");
            assert_eq!(
                validate_label(Some(&label)),
                Err(WidgetError::InvalidLabel),
                "{label:?}"
            );
        }
        // Their neighbours are ordinary characters.
        for c in ['\u{FDCF}', '\u{FDF0}', '\u{FFFD}', '\u{1FFFD}'] {
            let label = format!("Lisboa{c}");
            assert_eq!(validate_label(Some(&label)), Ok(()), "{label:?}");
        }
    }

    #[test]
    fn labels_with_nothing_visible_are_rejected() {
        let invisible = [
            '\u{00A0}',  // no-break space
            '\u{00AD}',  // soft hyphen
            '\u{034F}',  // combining grapheme joiner
            '\u{115F}',  // hangul choseong filler
            '\u{1160}',  // hangul jungseong filler
            '\u{17B4}',  // khmer vowel inherent aq
            '\u{17B5}',  // khmer vowel inherent aa
            '\u{180E}',  // mongolian vowel separator
            '\u{200B}',  // zero width space
            '\u{200C}',  // zero width non-joiner
            '\u{200D}',  // zero width joiner
            '\u{2060}',  // word joiner
            '\u{2063}',  // invisible separator
            '\u{2800}',  // braille pattern blank
            '\u{3164}',  // hangul filler
            '\u{FE0F}',  // variation selector 16
            '\u{FEFF}',  // byte order mark
            '\u{FFA0}',  // halfwidth hangul filler
            '\u{1D173}', // musical symbol begin beam
            '\u{E0020}', // tag space
            '\u{E0100}', // variation selector 17
        ];
        for c in invisible {
            for label in [c.to_string(), format!("{c}{c}{c}"), format!(" {c} ")] {
                assert_eq!(
                    validate_label(Some(&label)),
                    Err(WidgetError::InvalidLabel),
                    "{label:?}"
                );
            }
        }
        let all: String = invisible.iter().collect();
        assert_eq!(validate_label(Some(&all)), Err(WidgetError::InvalidLabel));
    }

    #[test]
    fn invisible_characters_next_to_visible_ones_are_kept() {
        let accepted = [
            "👨\u{200D}👩\u{200D}👧", // family emoji, joined by ZWJ
            "❤\u{FE0F}",              // emoji presentation
            "a\u{200B}b",
            "co\u{00AD}operar",
            "\u{2800}Lisboa",
        ];
        for label in accepted {
            assert_eq!(validate_label(Some(label)), Ok(()), "{label:?}");
        }
    }

    #[test]
    fn labels_with_a_line_or_paragraph_separator_are_rejected() {
        for label in ["São\u{2028}Paulo", "São\u{2029}Paulo"] {
            assert_eq!(
                validate_label(Some(label)),
                Err(WidgetError::InvalidLabel),
                "{label:?}"
            );
        }
    }

    #[test]
    fn the_label_error_never_carries_the_label() {
        let label = "Rótulo secreto\n";
        let message = validate_label(Some(label)).unwrap_err().to_string();
        assert!(
            !message.contains("Rótulo") && !message.contains("secreto"),
            "{message}"
        );
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
            "invalidWidth",
            "invalidCalendars",
            "noteTooLarge",
            "invalidConfig",
            "unavailable",
            "writeFailed",
        ];
        let serialized: Vec<_> = WidgetErrorCode::ALL.iter().map(to_json).collect();
        assert_eq!(serialized, names);
    }
}
