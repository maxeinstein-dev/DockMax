//! The name a program goes by when it does not say one itself: a tray icon
//! with no tooltip is named after the program that owns it
//! (`windows/program_name.rs` reads the file).

use std::path::Path;

/// The name of the program in the file `path`: the description in its
/// version resource (`FileDescription`) when it has one, else the file's
/// name without `.exe`. `None` only when neither has a letter to show.
pub fn from_file(description: Option<&str>, path: &Path) -> Option<String> {
    if let Some(description) = description.map(str::trim).filter(|text| !text.is_empty()) {
        return Some(description.to_string());
    }
    let file = path.file_name()?.to_str()?;
    let stem = match file.len().checked_sub(4) {
        Some(at) if file.is_char_boundary(at) && file[at..].eq_ignore_ascii_case(".exe") => {
            &file[..at]
        }
        _ => file,
    };
    let stem = stem.trim();
    (!stem.is_empty()).then(|| stem.to_string())
}

/// The tooltip of a tray icon, or, when it has none (blank counts as none),
/// the name of its program from `fallback`; empty when that has none either.
pub fn tip_or(tip: String, fallback: impl FnOnce() -> Option<String>) -> String {
    if !tip.trim().is_empty() {
        return tip;
    }
    fallback().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_description_of_the_file_is_the_name() {
        let path = Path::new(r"C:\Program Files\Fortinet\FortiClient\FortiTray.exe");

        assert_eq!(
            from_file(Some("FortiClient System Tray Controller"), path).as_deref(),
            Some("FortiClient System Tray Controller")
        );
        // Spaces around it are not part of the name.
        assert_eq!(
            from_file(Some("  Docker Desktop "), path).as_deref(),
            Some("Docker Desktop")
        );
    }

    #[test]
    fn without_a_description_the_file_name_without_exe_is_the_name() {
        let path = Path::new(r"C:\Apps\SyncTool.EXE");

        assert_eq!(from_file(None, path).as_deref(), Some("SyncTool"));
        assert_eq!(from_file(Some("   "), path).as_deref(), Some("SyncTool"));
        // Only a final `.exe` goes.
        assert_eq!(
            from_file(None, Path::new(r"C:\a\my.exe.tool")).as_deref(),
            Some("my.exe.tool")
        );
        assert_eq!(
            from_file(None, Path::new(r"C:\a\Ação.exe")).as_deref(),
            Some("Ação")
        );
    }

    #[test]
    fn a_file_with_nothing_to_show_has_no_name() {
        assert_eq!(from_file(None, Path::new(r"C:\a\.exe")), None);
        assert_eq!(from_file(None, Path::new("")), None);
    }

    #[test]
    fn the_tooltip_wins_and_only_a_blank_one_asks_for_the_program() {
        let asked = std::cell::Cell::new(false);
        let program = || {
            asked.set(true);
            Some("Programa".to_string())
        };

        assert_eq!(tip_or("WhatsApp".into(), program), "WhatsApp");
        assert!(!asked.get(), "a tooltip never reads the program's file");

        assert_eq!(
            tip_or(String::new(), || Some("Programa".into())),
            "Programa"
        );
        assert_eq!(tip_or(" \n".into(), || Some("Programa".into())), "Programa");
        assert_eq!(tip_or(String::new(), || None), "");
    }
}
