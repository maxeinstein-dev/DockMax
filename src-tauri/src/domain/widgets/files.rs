//! Files widget config (spec 0003 §2.5): the folder it lists. Only in the
//! domain in this phase: it can't be created, and listing comes in phase 3.

use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FilesConfig {
    /// Absolute Windows path of the folder: drive letter or UNC.
    pub folder: String,
}

impl Default for FilesConfig {
    /// The root of the system drive. It exists only so a stored config that
    /// can't be read still gives a valid instance (spec 0003 §2.4 item 6).
    fn default() -> Self {
        FilesConfig {
            folder: r"C:\".to_owned(),
        }
    }
}

impl FilesConfig {
    /// Checks the folder's text only, with no disk access.
    pub fn validate(&self) -> Result<(), WidgetError> {
        // L2: the validation lane implements this (absolute Windows path);
        // until then every config is accepted.
        Ok(())
    }
}
