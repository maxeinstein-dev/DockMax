//! User settings: the `settings.json` v1 model, its defaults and its rules.
//!
//! Every field falls back to its default when missing, so partial or older
//! files still load, and unknown fields are ignored. Values that are present
//! but wrong are rejected by [`Settings::validate`].

use std::collections::BTreeMap;
use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::hotkey;
use super::icon_pack::is_valid_pack_id;
use super::launcher_web::{self, WebKeyword};

/// Format version written by this build. Files with any other version are
/// rejected.
pub const CURRENT_VERSION: u32 = 1;

/// Smallest dock icon size accepted, in logical pixels.
pub const MIN_ICON_SIZE: u16 = 32;

/// Largest dock icon size accepted, in logical pixels.
pub const MAX_ICON_SIZE: u16 = 96;

/// Smallest and largest growth of the hovered item, in percent.
pub const MIN_HOVER_SCALE: u16 = 100;
pub const MAX_HOVER_SCALE: u16 = 160;

/// Growth of the hovered item unless the user changes it, in percent.
pub const DEFAULT_HOVER_SCALE: u16 = 110;

/// Most a style's corner tweak can ask for, in percent.
pub const MAX_CORNERS: u8 = 100;

/// Largest distance between the dock and the screen edge, in logical
/// pixels.
pub const MAX_BOTTOM_SPACE: u8 = 64;

/// Most icons of the notification area the dock shows itself, beside the
/// battery, the network and the volume (as the PrimoDock).
pub const MAX_TRAY_IN_DOCK: usize = 6;

/// The longest name of a tray icon kept (`path:<program>#<id>`).
pub const MAX_TRAY_KEY: usize = 300;

/// How long the pointer rests on the screen edge before a dock hiding on its
/// own comes up, in milliseconds: the default and the most.
pub const DEFAULT_AUTO_HIDE_REVEAL: u16 = 300;
pub const MAX_AUTO_HIDE_REVEAL: u16 = 2000;
/// How long the pointer is away before it goes down, in milliseconds.
pub const DEFAULT_AUTO_HIDE_DELAY: u16 = 700;
pub const MAX_AUTO_HIDE_DELAY: u16 = 5000;
/// Milliseconds over an open program before its windows' previews show
/// (spec 0015).
pub const DEFAULT_PREVIEW_DELAY: u16 = 200;
pub const MAX_PREVIEW_DELAY: u16 = 1000;

/// The distance between the dock and the edge unless the user changes it:
/// the dock takes the place of the taskbar, so it sits close to the edge.
pub const DEFAULT_BOTTOM_SPACE: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub theme: Theme,
    pub dock: DockSettings,
    /// Whether the Dockyard opens when the user signs in to Windows (the entry
    /// in the user's `Run` key). Off unless the user turns it on.
    pub launch_at_login: bool,
    /// Whether Alt+Space opens the launcher (spec 0012). On unless the user
    /// turns it off; the menus open it either way.
    pub launcher: bool,
    /// The launcher's web search by keyword (spec 0012 L2); the first is
    /// the default search.
    pub launcher_keywords: Vec<WebKeyword>,
    /// What opens the launcher (spec 0012 L3), as `hotkey::Hotkey` writes
    /// it (`Alt+Space`).
    pub launcher_hotkey: String,
    /// The interface's language (spec 0019 G1).
    pub language: Language,
    /// The welcome tour was shown (spec 0019 G2). A new install starts
    /// without it, so the tour opens once; a file written before the tour
    /// existed counts as shown.
    #[serde(default = "shown")]
    pub tour_shown: bool,
}

fn shown() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: CURRENT_VERSION,
            theme: Theme::default(),
            dock: DockSettings::default(),
            launch_at_login: false,
            launcher: true,
            launcher_keywords: launcher_web::defaults(),
            launcher_hotkey: hotkey::DEFAULT.to_owned(),
            language: Language::default(),
            tour_shown: false,
        }
    }
}

impl Settings {
    /// Checks the rules serde can't express: the format version, the icon
    /// size range and the ranges of the look (spec 0005 §3.1).
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.version != CURRENT_VERSION {
            return Err(SettingsError::UnsupportedVersion);
        }
        if !(MIN_ICON_SIZE..=MAX_ICON_SIZE).contains(&self.dock.icon_size) {
            return Err(SettingsError::IconSizeOutOfRange);
        }
        if !launcher_web::is_valid_list(&self.launcher_keywords) {
            return Err(SettingsError::LauncherKeywordsInvalid);
        }
        if hotkey::check(&self.launcher_hotkey).is_err() {
            return Err(SettingsError::LauncherHotkeyInvalid);
        }
        self.dock.validate_look()
    }
}

impl DockSettings {
    fn validate_look(&self) -> Result<(), SettingsError> {
        if !(MIN_HOVER_SCALE..=MAX_HOVER_SCALE).contains(&self.hover_scale) {
            return Err(SettingsError::LookOutOfRange);
        }
        if self
            .tweaks
            .values()
            .any(|tweaks| tweaks.corners.is_some_and(|corners| corners > MAX_CORNERS))
        {
            return Err(SettingsError::LookOutOfRange);
        }
        if self.bottom_space > MAX_BOTTOM_SPACE
            || self.auto_hide_reveal > MAX_AUTO_HIDE_REVEAL
            || self.auto_hide_delay > MAX_AUTO_HIDE_DELAY
            || self.preview_delay > MAX_PREVIEW_DELAY
        {
            return Err(SettingsError::LookOutOfRange);
        }
        if self
            .indicator_color
            .as_deref()
            .is_some_and(|color| !is_hex_color(color))
        {
            return Err(SettingsError::LookOutOfRange);
        }
        if self
            .icon_pack
            .as_deref()
            .is_some_and(|id| !is_valid_pack_id(id))
        {
            return Err(SettingsError::LookOutOfRange);
        }
        if self.tray_in_dock.len() > MAX_TRAY_IN_DOCK
            || self.tray_in_dock.iter().any(|key| !is_tray_key(key))
        {
            return Err(SettingsError::LookOutOfRange);
        }
        Ok(())
    }
}

/// The name of a tray icon that survives a restart: `path:` and the
/// program's file, in lowercase, then `#` and the icon's number.
pub fn is_tray_key(key: &str) -> bool {
    key.len() <= MAX_TRAY_KEY
        && key
            .strip_prefix("path:")
            .and_then(|rest| rest.rsplit_once('#'))
            .is_some_and(|(path, number)| {
                !path.is_empty()
                    && !number.is_empty()
                    && number.bytes().all(|byte| byte.is_ascii_digit())
                    && !path.chars().any(char::is_control)
            })
}

/// `#RRGGBB`: a hash and six hexadecimal digits, nothing else.
fn is_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The interface's language (spec 0019 G1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// The Windows display language: Portuguese if it is one, English
    /// otherwise.
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en")]
    En,
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
    /// The look of the bar (spec 0005 §3.2).
    pub style: DockStyle,
    /// What the user changed in each style; a style without an entry, or a
    /// field left out, keeps the style's own look.
    #[serde(deserialize_with = "deserialize_tweaks")]
    pub tweaks: BTreeMap<DockStyle, StyleTweaks>,
    /// What an item does under the pointer (spec 0005 §3.3).
    pub hover: HoverEffect,
    /// How much the hovered item grows with [`HoverEffect::Scale`], in
    /// percent, within `MIN_HOVER_SCALE..=MAX_HOVER_SCALE`.
    pub hover_scale: u16,
    /// The mark under an open item (spec 0005 §3.4).
    pub indicator: Indicator,
    /// `#RRGGBB`, or `None` for the style's own color.
    pub indicator_color: Option<String>,
    /// How a pinned program's icon sits in its tile (spec 0006 B2).
    pub icon_shape: IconShape,
    /// The icon pack every pinned program wears (spec 0007): the id of a
    /// folder of `IconPacks`, or `None` for the icons Windows gives.
    pub icon_pack: Option<String>,
    /// What a program's icon does while it opens (spec 0006 B2).
    pub open_animation: OpenAnimation,
    /// Whether the open programs that are not pinned show on the dock, after
    /// the widgets (spec 0009 D2).
    pub show_open_apps: bool,
    /// Whether a program's icon wears the notification badge its button of
    /// the Windows taskbar has (spec 0016).
    pub show_badges: bool,
    /// The icons of the notification area the dock shows itself, at its end
    /// beside the battery, the network and the volume, by `is_tray_key`
    /// name; the others are behind the arrow. Left out of the file while
    /// empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tray_in_dock: Vec<String>,
    /// What a click with the middle button (the wheel) does on a program of the
    /// dock (spec 0009).
    pub middle_click: MiddleClick,
    /// What the dock does with the Windows taskbar (ADR 0009).
    pub taskbar: TaskbarMode,
    /// A dock on every monitor, each holding its own edge (spec 0010 D4b), or
    /// only on the primary one.
    pub every_monitor: bool,
    /// The dock goes off the screen when the pointer leaves it and comes
    /// back when the pointer reaches the screen edge, giving its band to the
    /// programs (like the taskbar's own auto-hide).
    pub auto_hide: bool,
    /// Its times, in milliseconds: on the edge before it comes up, within
    /// `0..=MAX_AUTO_HIDE_REVEAL`; away before it goes down, within
    /// `0..=MAX_AUTO_HIDE_DELAY`.
    pub auto_hide_reveal: u16,
    pub auto_hide_delay: u16,
    /// Distance between the dock and the screen edge, in logical pixels,
    /// within `0..=MAX_BOTTOM_SPACE`.
    pub bottom_space: u8,
    /// A switch of workspace goes to every monitor at once (spec 0013 C3),
    /// not only to the one of the dock that switched.
    pub switch_every_monitor: bool,
    /// The mark under an open item wears the color of the workspace shown
    /// (spec 0013 C3), over `indicator_color`.
    pub indicator_follows_workspace: bool,
    /// The pointer resting on an open program shows its windows' previews
    /// (spec 0015), after `preview_delay` milliseconds, within
    /// `0..=MAX_PREVIEW_DELAY`; with `previews_only_several`, only for a
    /// program with two windows or more.
    pub previews: bool,
    pub preview_delay: u16,
    pub previews_only_several: bool,
}

impl Default for DockSettings {
    fn default() -> Self {
        DockSettings {
            edge: DockEdge::default(),
            icon_size: SizePreset::Medium.icon_size(),
            opacity: DockOpacity::default(),
            style: DockStyle::default(),
            tweaks: BTreeMap::new(),
            hover: HoverEffect::default(),
            hover_scale: DEFAULT_HOVER_SCALE,
            indicator: Indicator::default(),
            indicator_color: None,
            icon_shape: IconShape::default(),
            icon_pack: None,
            open_animation: OpenAnimation::default(),
            show_open_apps: true,
            show_badges: true,
            tray_in_dock: Vec::new(),
            middle_click: MiddleClick::default(),
            taskbar: TaskbarMode::default(),
            every_monitor: true,
            auto_hide: false,
            auto_hide_reveal: DEFAULT_AUTO_HIDE_REVEAL,
            auto_hide_delay: DEFAULT_AUTO_HIDE_DELAY,
            bottom_space: DEFAULT_BOTTOM_SPACE,
            switch_every_monitor: false,
            indicator_follows_workspace: false,
            previews: true,
            preview_delay: DEFAULT_PREVIEW_DELAY,
            previews_only_several: false,
        }
    }
}

/// What the middle button does on a program of the dock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MiddleClick {
    /// Opens a new instance of the program, as the Windows taskbar does.
    #[default]
    NewInstance,
    /// Does nothing.
    None,
}

/// What the dock does with the Windows taskbar (ADR 0009).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskbarMode {
    /// Leaves the taskbar alone and sits above it.
    Visible,
    /// The dock takes the edge; the taskbar hides until the pointer reaches
    /// it.
    Autohide,
    /// The dock takes the edge and the taskbar is hidden altogether: what
    /// the dock does unless the user says otherwise (ADR 0009).
    #[default]
    Hidden,
}

impl TaskbarMode {
    /// Whether the dock reserves the edge of the screen (an app bar).
    pub fn reserves_the_edge(self) -> bool {
        self != TaskbarMode::Visible
    }
}

/// The look of the dock bar: eight ready-made styles (spec 0005 §3.2).
///
/// The Canarinho style (`"canary"`) was removed on 2026-10-30; a file that
/// still names it opens with the default style, see [`REMOVED_STYLES`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockStyle {
    #[default]
    #[serde(alias = "canary")]
    Native,
    LiquidGlass,
    Midnight,
    Rocket,
    AquaShelf,
    Neon,
    Paper,
    Floating,
}

/// Styles that no longer exist. As `style`, serde reads them as the default
/// one (the alias on [`DockStyle::Native`]); in `tweaks` their entry is
/// dropped, so it cannot overwrite the default style's own tweaks.
pub const REMOVED_STYLES: &[&str] = &["canary"];

/// Reads `tweaks`, dropping the entries of [`REMOVED_STYLES`]. Any other
/// unknown style is still an error.
pub fn deserialize_tweaks<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<DockStyle, StyleTweaks>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, IntoDeserializer};

    let raw = BTreeMap::<String, StyleTweaks>::deserialize(deserializer)?;
    let mut tweaks = BTreeMap::new();
    for (name, tweak) in raw {
        if REMOVED_STYLES.contains(&name.as_str()) {
            continue;
        }
        let style = DockStyle::deserialize(name.as_str().into_deserializer())
            .map_err(|error: serde::de::value::Error| D::Error::custom(error))?;
        tweaks.insert(style, tweak);
    }
    Ok(tweaks)
}

/// The user's changes to one style. `None` keeps what the style has.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StyleTweaks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<bool>,
    /// How round the bar is, 0–100 % of the most it can be.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corners: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflections: Option<bool>,
}

/// What an item does under the pointer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoverEffect {
    None,
    /// Only the item under the pointer grows.
    #[default]
    Scale,
    /// It grows and so do its neighbours, as in the macOS dock.
    Magnify,
}

/// The mark under an open item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Indicator {
    #[default]
    Dot,
    Dash,
}

/// How a pinned program's icon sits in its tile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconShape {
    /// On a rounded plate, with room around it.
    #[default]
    Plate,
    /// Without a plate, every icon filling the same square.
    Uniform,
    /// Without a plate, as the program drew it.
    Original,
}

/// What a program's icon does while the program opens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAnimation {
    /// It jumps, away from the screen edge.
    #[default]
    Bounce,
    /// It swells and shrinks.
    Pulse,
    /// It shakes from side to side.
    Shake,
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
    Small,
    Medium,
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
    /// A value of the dock's look is outside its range or malformed.
    #[error("dock look value out of range")]
    LookOutOfRange,
    /// A keyword of the launcher's web search is malformed or repeated.
    #[error("launcher keyword invalid")]
    LauncherKeywordsInvalid,
    /// The launcher's hotkey is malformed or one it can't take.
    #[error("launcher hotkey invalid")]
    LauncherHotkeyInvalid,
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
            SettingsError::LookOutOfRange => SettingsErrorCode::LookOutOfRange,
            SettingsError::LauncherKeywordsInvalid => SettingsErrorCode::LauncherKeywordsInvalid,
            SettingsError::LauncherHotkeyInvalid => SettingsErrorCode::LauncherHotkeyInvalid,
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
    /// A value of the dock's look is outside its range or malformed.
    LookOutOfRange,
    /// A keyword of the launcher's web search is malformed or repeated, or
    /// there are too many.
    LauncherKeywordsInvalid,
    /// The launcher's hotkey is malformed or one it can't take.
    LauncherHotkeyInvalid,
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
    fn a_hotkey_the_launcher_cannot_take_is_rejected() {
        for hotkey in ["", "Space", "Ctrl+C", "Win+L", "alt+space"] {
            let settings = Settings {
                launcher_hotkey: hotkey.into(),
                ..Settings::default()
            };
            assert_eq!(
                settings.validate(),
                Err(SettingsError::LauncherHotkeyInvalid),
                "{hotkey:?}"
            );
        }
        let settings = Settings {
            launcher_hotkey: "Ctrl+Shift+K".into(),
            ..Settings::default()
        };
        assert_eq!(settings.validate(), Ok(()));
    }

    #[test]
    fn a_bad_launcher_keyword_is_rejected() {
        let mut settings = Settings::default();
        settings.launcher_keywords[0].template = "http://example.com/?q=%s".into();
        assert_eq!(
            settings.validate(),
            Err(SettingsError::LauncherKeywordsInvalid)
        );
        settings.launcher_keywords.clear();
        assert_eq!(settings.validate(), Ok(()), "no keywords is fine");
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
        // Spec 0002 §2.3 item 3 plus the look fields of spec 0005 §3.1, which
        // are additive (and spec 0007's `iconPack`): the version stays 1.
        let spec: serde_json::Value = serde_json::from_str(
            r#"{
              "version": 1,
              "theme": "system",
              "dock": {
                "edge": "bottom", "iconSize": 48, "opacity": "translucent",
                "style": "native", "tweaks": {}, "hover": "scale", "hoverScale": 110,
                "indicator": "dot", "indicatorColor": null,
                "iconShape": "plate", "iconPack": null, "openAnimation": "bounce",
                "showOpenApps": true, "showBadges": true, "middleClick": "newInstance",
                "taskbar": "hidden", "everyMonitor": true, "autoHide": false, "autoHideReveal": 300, "autoHideDelay": 700, "bottomSpace": 2,
                "switchEveryMonitor": false, "indicatorFollowsWorkspace": false,
                "previews": true, "previewDelay": 200, "previewsOnlySeveral": false
              },
              "launchAtLogin": false,
              "launcher": true,
              "launcherKeywords": [
                {"keyword": "g", "name": "Google", "template": "https://www.google.com/search?q=%s"},
                {"keyword": "yt", "name": "YouTube", "template": "https://www.youtube.com/results?search_query=%s"},
                {"keyword": "w", "name": "Wikipédia", "template": "https://pt.wikipedia.org/w/index.php?search=%s"},
                {"keyword": "gh", "name": "GitHub", "template": "https://github.com/search?q=%s"}
              ],
              "launcherHotkey": "Alt+Space",
              "language": "system",
              "tourShown": false
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
    fn a_file_from_before_the_look_fields_loads_with_their_defaults() {
        let settings: Settings = serde_json::from_str(
            r#"{"version":1,"theme":"dark",
                "dock":{"edge":"top","iconSize":64,"opacity":"solid"}}"#,
        )
        .unwrap();

        assert_eq!(settings.dock.style, DockStyle::Native);
        assert!(settings.dock.tweaks.is_empty());
        assert_eq!(settings.dock.hover, HoverEffect::Scale);
        assert_eq!(settings.dock.hover_scale, 110);
        assert_eq!(settings.dock.indicator, Indicator::Dot);
        assert_eq!(settings.dock.indicator_color, None);
        assert_eq!(settings.dock.icon_shape, IconShape::Plate);
        assert_eq!(settings.dock.icon_pack, None);
        assert_eq!(settings.dock.open_animation, OpenAnimation::Bounce);
        assert_eq!(settings.validate(), Ok(()));
    }

    #[test]
    fn icon_shapes_and_open_animations_use_their_lowercase_names() {
        let settings: Settings =
            serde_json::from_str(r#"{"dock":{"iconShape":"original","openAnimation":"shake"}}"#)
                .unwrap();

        assert_eq!(settings.dock.icon_shape, IconShape::Original);
        assert_eq!(settings.dock.open_animation, OpenAnimation::Shake);
        for name in ["plate", "uniform", "original"] {
            let json = format!(r#"{{"dock":{{"iconShape":"{name}"}}}}"#);
            assert!(serde_json::from_str::<Settings>(&json).is_ok(), "{name}");
        }
        assert!(serde_json::from_str::<Settings>(r#"{"dock":{"iconShape":"Plate"}}"#).is_err());
        assert!(serde_json::from_str::<Settings>(r#"{"dock":{"openAnimation":"spin"}}"#).is_err());
    }

    fn with_dock(change: impl FnOnce(&mut DockSettings)) -> Settings {
        let mut settings = Settings::default();
        change(&mut settings.dock);
        settings
    }

    #[test]
    fn styles_hovers_and_indicators_use_camel_case_names() {
        let settings: Settings = serde_json::from_str(
            r#"{"dock":{"style":"aquaShelf","hover":"magnify","indicator":"dash",
                "tweaks":{"liquidGlass":{"corners":70,"reflections":true}}}}"#,
        )
        .unwrap();

        assert_eq!(settings.dock.style, DockStyle::AquaShelf);
        assert_eq!(settings.dock.hover, HoverEffect::Magnify);
        assert_eq!(settings.dock.indicator, Indicator::Dash);
        assert_eq!(
            settings.dock.tweaks[&DockStyle::LiquidGlass],
            StyleTweaks {
                border: None,
                corners: Some(70),
                reflections: Some(true),
            }
        );
        assert_eq!(settings.validate(), Ok(()));
        assert!(serde_json::from_str::<DockStyle>("\"liquid-glass\"").is_err());
        assert!(serde_json::from_str::<DockStyle>("\"Native\"").is_err());
    }

    #[test]
    fn a_tweak_left_out_is_not_written() {
        let tweaks = StyleTweaks {
            border: Some(false),
            ..StyleTweaks::default()
        };

        assert_eq!(
            serde_json::to_string(&tweaks).unwrap(),
            r#"{"border":false}"#
        );
    }

    #[test]
    fn every_style_round_trips_through_json() {
        let styles = [
            (DockStyle::Native, "native"),
            (DockStyle::LiquidGlass, "liquidGlass"),
            (DockStyle::Midnight, "midnight"),
            (DockStyle::Rocket, "rocket"),
            (DockStyle::AquaShelf, "aquaShelf"),
            (DockStyle::Neon, "neon"),
            (DockStyle::Paper, "paper"),
            (DockStyle::Floating, "floating"),
        ];
        for (style, name) in styles {
            assert_eq!(serde_json::to_value(style).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<DockStyle>(name.into()).unwrap(),
                style
            );
        }
    }

    #[test]
    fn a_file_with_the_removed_canarinho_style_opens_with_the_default_style() {
        let settings: Settings = serde_json::from_str(
            r#"{"version":1,"theme":"dark",
                "dock":{"edge":"top","iconSize":64,"style":"canary","hover":"magnify",
                    "tweaks":{"native":{"corners":30},"canary":{"corners":90,"border":false},
                        "neon":{"reflections":true}}},
                "launcher":false,"tourShown":true}"#,
        )
        .unwrap();

        assert_eq!(settings.dock.style, DockStyle::default());
        assert_eq!(settings.dock.style, DockStyle::Native);
        // The rest of the file is kept.
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.dock.edge, DockEdge::Top);
        assert_eq!(settings.dock.icon_size, 64);
        assert_eq!(settings.dock.hover, HoverEffect::Magnify);
        assert!(!settings.launcher);
        assert!(settings.tour_shown);
        // The Canarinho tweaks are dropped and do not overwrite Native's.
        assert_eq!(settings.dock.tweaks.len(), 2);
        assert_eq!(settings.dock.tweaks[&DockStyle::Native].corners, Some(30));
        assert_eq!(
            settings.dock.tweaks[&DockStyle::Neon].reflections,
            Some(true)
        );
        assert_eq!(settings.validate(), Ok(()));
        // Saved again, the file no longer names it.
        assert!(!serde_json::to_string(&settings).unwrap().contains("canary"));
        // Any other unknown style is still an error.
        assert!(serde_json::from_str::<Settings>(r#"{"dock":{"tweaks":{"banana":{}}}}"#).is_err());
    }

    #[test]
    fn hover_scale_outside_the_range_is_rejected() {
        for percent in [0, 99, 161, u16::MAX] {
            assert_eq!(
                with_dock(|dock| dock.hover_scale = percent).validate(),
                Err(SettingsError::LookOutOfRange),
                "{percent}"
            );
        }
        for percent in [100, 110, 160] {
            assert_eq!(
                with_dock(|dock| dock.hover_scale = percent).validate(),
                Ok(()),
                "{percent}"
            );
        }
    }

    #[test]
    fn corners_above_100_are_rejected_in_any_style() {
        let settings = with_dock(|dock| {
            dock.tweaks.insert(
                DockStyle::Neon,
                StyleTweaks {
                    corners: Some(101),
                    ..StyleTweaks::default()
                },
            );
        });

        assert_eq!(settings.validate(), Err(SettingsError::LookOutOfRange));
        let edge = with_dock(|dock| {
            dock.tweaks.insert(
                DockStyle::Neon,
                StyleTweaks {
                    corners: Some(100),
                    ..StyleTweaks::default()
                },
            );
        });
        assert_eq!(edge.validate(), Ok(()));
    }

    #[test]
    fn the_indicator_color_must_be_a_six_digit_hex_color() {
        for color in ["#ffffff", "#0A84FF", "#000000"] {
            assert_eq!(
                with_dock(|dock| dock.indicator_color = Some(color.into())).validate(),
                Ok(()),
                "{color}"
            );
        }
        for color in [
            "", "#", "fff", "#fff", "ffffff", "#fffffg", "#fffffff", "red", "#ffffff ", "#ffffÃ",
        ] {
            assert_eq!(
                with_dock(|dock| dock.indicator_color = Some(color.into())).validate(),
                Err(SettingsError::LookOutOfRange),
                "{color:?}"
            );
        }
    }

    #[test]
    fn the_middle_button_opens_a_new_instance_unless_it_is_turned_off() {
        assert_eq!(
            Settings::default().dock.middle_click,
            MiddleClick::NewInstance
        );
        for (name, expected) in [
            ("newInstance", MiddleClick::NewInstance),
            ("none", MiddleClick::None),
        ] {
            let json = format!(r#"{{"dock":{{"middleClick":"{name}"}}}}"#);
            let settings: Settings = serde_json::from_str(&json).unwrap();
            assert_eq!(settings.dock.middle_click, expected, "{name}");
        }
        assert!(serde_json::from_str::<Settings>(r#"{"dock":{"middleClick":"close"}}"#).is_err());
    }

    #[test]
    fn the_tray_icons_in_the_dock_are_up_to_six_names_of_tray_icons() {
        let key = |n: usize| format!("path:c:\\program files\\app\\app.exe#{n}");
        let six: Vec<String> = (0..6).map(key).collect();
        assert_eq!(
            with_dock(|dock| dock.tray_in_dock = six.clone()).validate(),
            Ok(())
        );
        let seven: Vec<String> = (0..7).map(key).collect();
        assert_eq!(
            with_dock(|dock| dock.tray_in_dock = seven).validate(),
            Err(SettingsError::LookOutOfRange)
        );
        for bad in [
            "c:\\app.exe#1",
            "path:#1",
            "path:app.exe#",
            "path:app.exe#x",
            "path:a\nb#1",
        ] {
            assert!(!is_tray_key(bad), "{bad}");
        }
        assert!(!is_tray_key(&format!("path:{}#1", "a".repeat(300))));
        // Left out of the file while empty: older files read the same.
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert!(json["dock"].get("trayInDock").is_none());
    }

    #[test]
    fn the_icon_pack_is_the_id_of_a_pack_folder_or_none() {
        for id in ["pixel-system", "a", "cores-2"] {
            assert_eq!(
                with_dock(|dock| dock.icon_pack = Some(id.into())).validate(),
                Ok(()),
                "{id}"
            );
        }
        for id in ["", "Pixel", "a b", "..", "a/b", r"a\b", r"C:\x"] {
            assert_eq!(
                with_dock(|dock| dock.icon_pack = Some(id.into())).validate(),
                Err(SettingsError::LookOutOfRange),
                "{id:?}"
            );
        }
        let settings: Settings =
            serde_json::from_str(r#"{"dock":{"iconPack":"pixel-system"}}"#).unwrap();
        assert_eq!(settings.dock.icon_pack.as_deref(), Some("pixel-system"));
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
        // Except the tour: a file from before it existed counts as shown.
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings {
                tour_shown: true,
                ..Settings::default()
            }
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
            SettingsError::LookOutOfRange,
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
                SettingsError::LookOutOfRange,
                SettingsErrorCode::LookOutOfRange,
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
            (SettingsErrorCode::LookOutOfRange, "lookOutOfRange"),
            (SettingsErrorCode::UnsupportedVersion, "unsupportedVersion"),
            (SettingsErrorCode::WriteFailed, "writeFailed"),
            (SettingsErrorCode::NotReady, "notReady"),
        ];
        for (code, name) in cases {
            assert_eq!(serde_json::to_value(code).unwrap(), name);
        }
    }
}
