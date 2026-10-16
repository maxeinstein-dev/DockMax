//! Note widget config (spec 0003 §2.2, Note). A note has nothing to set:
//! its text lives in the `notes` table, not in the config, so it never
//! travels with the instance list.

use serde::{Deserialize, Serialize};

use super::{MAX_NOTE_BYTES, WidgetError};

/// Empty, and serialized as `{}` (not `null`), so every kind's config is an
/// object.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteConfig {}

impl NoteConfig {
    /// Always valid: the config has no field. The text's limit is a rule of
    /// the text, not of the config: see [`validate_note_body`].
    pub fn validate(&self) -> Result<(), WidgetError> {
        Ok(())
    }
}

/// Checks a note's text before it is saved: at most [`MAX_NOTE_BYTES`] in
/// UTF-8, the form it is stored in. Any text within the limit is accepted,
/// empty included. The error never carries the text.
pub fn validate_note_body(body: &str) -> Result<(), WidgetError> {
    if body.len() <= MAX_NOTE_BYTES {
        Ok(())
    } else {
        Err(WidgetError::NoteTooLarge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_has_nothing_to_check() {
        assert_eq!(NoteConfig::default().validate(), Ok(()));
    }

    #[test]
    fn text_up_to_64_kib_is_accepted() {
        assert_eq!(validate_note_body(""), Ok(()));
        assert_eq!(validate_note_body("lista do mercado"), Ok(()));
        assert_eq!(validate_note_body(&"a".repeat(MAX_NOTE_BYTES)), Ok(()));
    }

    #[test]
    fn text_over_64_kib_is_too_large() {
        assert_eq!(
            validate_note_body(&"a".repeat(MAX_NOTE_BYTES + 1)),
            Err(WidgetError::NoteTooLarge)
        );
    }

    #[test]
    fn the_limit_counts_utf8_bytes_not_characters() {
        // "ç" is 2 bytes: half as many characters fill the limit.
        let at_limit = "ç".repeat(MAX_NOTE_BYTES / 2);
        assert_eq!(validate_note_body(&at_limit), Ok(()));
        let over = format!("{at_limit}a");
        assert!(over.chars().count() < MAX_NOTE_BYTES);
        assert_eq!(validate_note_body(&over), Err(WidgetError::NoteTooLarge));
    }
}
