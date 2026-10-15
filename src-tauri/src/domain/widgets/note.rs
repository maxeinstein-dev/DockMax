//! Note widget config (spec 0003 §2.2, Note). A note has nothing to set:
//! its text lives in the `notes` table, not in the config, so it never
//! travels with the instance list.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// Empty, and serialized as `{}` (not `null`), so every kind's config is an
/// object.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteConfig {}

impl NoteConfig {
    /// Nothing to check today; kept so every kind has the same shape.
    pub fn validate(&self) -> Result<(), WidgetError> {
        // L2: the validation lane owns this; an empty config has no rule.
        Ok(())
    }
}
