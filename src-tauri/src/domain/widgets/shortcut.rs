//! Shortcut widget config (spec 0003 §2.5): what the shortcut opens. Only
//! in the domain in this phase: it can't be created, and opening it comes
//! in phase 3.

use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutConfig {
    pub target: ShortcutTarget,
}

impl ShortcutConfig {
    /// Checks the target's text only, with no disk or network access.
    pub fn validate(&self) -> Result<(), WidgetError> {
        // L2: the validation lane implements this (absolute Windows path or
        // http/https URL with a host); until then every config is accepted.
        Ok(())
    }
}

/// A file, folder or program path, or a web address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShortcutTarget {
    /// Absolute Windows path: drive letter or UNC.
    Path { path: String },
    /// `http` or `https` URL with a host.
    Url { url: String },
}

impl Default for ShortcutTarget {
    /// The root of the system drive. A shortcut is never created with it:
    /// it exists only so a stored config that can't be read still gives a
    /// valid instance (spec 0003 §2.4 item 6), and opening a folder is the
    /// harmless choice.
    fn default() -> Self {
        ShortcutTarget::Path {
            path: r"C:\".to_owned(),
        }
    }
}
