//! Files widget config (spec 0003 §2.5): the folder it lists. Only in the
//! domain in this phase: it can't be created, and listing comes in phase 3.

use serde::{Deserialize, Serialize};

use super::WidgetError;
use super::path::is_absolute_windows_path;

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
    /// Checks the folder's text only, with no disk access: an absolute
    /// Windows path, the same rule as a shortcut's path (see
    /// [`is_absolute_windows_path`]). Whether it exists and is a folder is
    /// for the listing, in phase 3.
    pub fn validate(&self) -> Result<(), WidgetError> {
        if is_absolute_windows_path(&self.folder) {
            Ok(())
        } else {
            Err(WidgetError::InvalidFolder)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_folder(folder: &str) -> FilesConfig {
        FilesConfig {
            folder: folder.to_owned(),
        }
    }

    #[test]
    fn default_is_the_system_drive_root_and_valid() {
        assert_eq!(FilesConfig::default().folder, r"C:\");
        assert_eq!(FilesConfig::default().validate(), Ok(()));
    }

    #[test]
    fn absolute_folders_are_accepted() {
        for folder in [
            r"C:\Users\Public\Downloads",
            "D:/Fotos",
            r"\\server\share\Downloads",
        ] {
            assert_eq!(with_folder(folder).validate(), Ok(()), "{folder:?}");
        }
    }

    #[test]
    fn folders_that_are_not_absolute_windows_paths_are_rejected() {
        for folder in [
            "",
            "Downloads",
            r".\Downloads",
            "C:",
            "C:Downloads",
            r"\Downloads",
            r"\\server",
            r"\\?\C:\Downloads",
            r"\\.\C:\Downloads",
            "C:\\Down\0loads",
            "https://example.com/",
            "file:///C:/Downloads",
        ] {
            assert_eq!(
                with_folder(folder).validate(),
                Err(WidgetError::InvalidFolder),
                "{folder:?}"
            );
        }
        let too_long = format!(r"C:\{}", "a".repeat(32_765));
        assert_eq!(
            with_folder(&too_long).validate(),
            Err(WidgetError::InvalidFolder)
        );
    }

    #[test]
    fn the_error_message_never_names_the_folder() {
        let message = with_folder("Downloads").validate().unwrap_err().to_string();
        assert!(!message.contains("Downloads"), "{message}");
    }
}
