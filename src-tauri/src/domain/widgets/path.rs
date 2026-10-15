//! Syntactic check of an absolute Windows path, shared by the Shortcut and
//! Files configs (spec 0003 §2.5). Looks at the text only: nothing here
//! touches the disk, so a path that passes may still not exist.

/// Longest path accepted, in UTF-16 code units: the limit of the Windows
/// wide-character APIs (`\\?\`-prefixed paths), which count `u16`s, not
/// characters. A character outside the Basic Multilingual Plane counts 2.
pub const MAX_PATH_UTF16_UNITS: usize = 32_767;

/// Whether `path` is an absolute Windows path in one of two forms:
///
/// - drive letter: `C:\...` (an ASCII letter, `:` and a separator);
/// - UNC: `\\server\share...`, with server and share not empty.
///
/// `\` and `/` are both separators, as for Windows itself. Rejected: an
/// empty path, a control character (`char::is_control`: NUL, tab, line
/// breaks, ESC, DEL, C1), more than [`MAX_PATH_UTF16_UNITS`], a relative
/// path (`foo`, `.\foo`), a drive-relative one (`C:`, `C:foo`), a
/// root-relative one (`\foo`) and the device and verbatim prefixes
/// (`\\?\`, `\\.\`), which skip the normalization the rest of the app
/// expects. Other characters that Windows forbids in names (`<>"|?*`) are
/// left to the opener: the spec asks only for the shape.
///
/// DOS device names as a component (`C:\pasta\CON`, `NUL.txt`) pass here:
/// they are checked in phase 3, where shortcuts and folders are opened.
pub fn is_absolute_windows_path(path: &str) -> bool {
    if path.is_empty() || path.chars().any(char::is_control) {
        return false;
    }
    if path.encode_utf16().count() > MAX_PATH_UTF16_UNITS {
        return false;
    }
    let mut chars = path.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(drive), Some(':'), Some(third)) => drive.is_ascii_alphabetic() && is_separator(third),
        (Some(first), Some(second), Some(_)) if is_separator(first) && is_separator(second) => {
            is_unc(&path[2..])
        }
        _ => false,
    }
}

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

/// `rest` is a UNC path after its two leading separators: `server\share`
/// and anything after it.
fn is_unc(rest: &str) -> bool {
    let mut parts = rest.split(is_separator);
    let server = parts.next().unwrap_or_default();
    let share = parts.next().unwrap_or_default();
    // `.` and `?` as the server are the device (`\\.\`) and verbatim
    // (`\\?\`) prefixes, not a server.
    !server.is_empty() && !share.is_empty() && server != "." && server != "?"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letter_paths_are_absolute() {
        let accepted = [
            r"C:\",
            r"C:\Users\Public\Documents",
            r"d:\x",
            "C:/Users/Public",
            r"Z:\pasta com espaço\relatório.pdf",
            r"C:\a\..\b",
        ];
        for path in accepted {
            assert!(is_absolute_windows_path(path), "{path:?}");
        }
    }

    #[test]
    fn unc_paths_with_server_and_share_are_absolute() {
        let accepted = [
            r"\\server\share",
            r"\\server\share\",
            r"\\server\share\Downloads",
            "//server/share/Downloads",
            r"\\192.168.0.10\público",
        ];
        for path in accepted {
            assert!(is_absolute_windows_path(path), "{path:?}");
        }
    }

    #[test]
    fn relative_and_incomplete_paths_are_rejected() {
        let rejected = [
            "",
            "C",
            "C:",
            "C:foo",
            r"C:foo\bar",
            "relatorio.pdf",
            r".\foo",
            r"..\foo",
            r"\foo",
            "/foo",
            r"1:\foo",
            r"ç:\foo",
            r"CC:\foo",
            r"\\",
            r"\\server",
            r"\\server\",
            r"\\\share",
            r"\\server\\share",
            " C:\\foo",
        ];
        for path in rejected {
            assert!(!is_absolute_windows_path(path), "{path:?}");
        }
    }

    #[test]
    fn device_and_verbatim_prefixes_are_rejected() {
        let rejected = [
            r"\\?\C:\foo",
            r"\\.\C:\foo",
            r"\\.\PhysicalDrive0",
            r"\\?\UNC\server\share",
            "//?/C:/foo",
            "//./COM1",
            r"\??\C:\foo",
        ];
        for path in rejected {
            assert!(!is_absolute_windows_path(path), "{path:?}");
        }
    }

    #[test]
    fn a_nul_character_is_rejected_anywhere() {
        assert!(!is_absolute_windows_path("C:\\foo\0"));
        assert!(!is_absolute_windows_path("C:\\fo\0o\\bar"));
        assert!(!is_absolute_windows_path("\\\\server\\sh\0are"));
    }

    #[test]
    fn a_control_character_is_rejected_anywhere() {
        let controls = [
            '\u{1}', '\t', '\n', '\r', '\u{1b}', '\u{7f}', '\u{85}', '\u{9f}',
        ];
        for c in controls {
            for path in [
                format!(r"C:\foo{c}"),
                format!(r"C:\fo{c}o\bar"),
                format!(r"\\server\sh{c}are"),
                format!(r"\\ser{c}ver\share"),
            ] {
                assert!(!is_absolute_windows_path(&path), "{path:?}");
            }
        }
    }

    #[test]
    fn length_limit_counts_utf16_units() {
        let at_limit = format!(r"C:\{}", "a".repeat(MAX_PATH_UTF16_UNITS - 3));
        assert_eq!(at_limit.encode_utf16().count(), MAX_PATH_UTF16_UNITS);
        assert!(is_absolute_windows_path(&at_limit));

        let over = format!("{at_limit}a");
        assert!(!is_absolute_windows_path(&over));

        // A character outside the BMP is one `char` but two UTF-16 units.
        let emoji = '\u{1F4C1}';
        let pairs = format!(r"C:\{}", emoji.to_string().repeat(16_382));
        assert_eq!(pairs.encode_utf16().count(), 32_767);
        assert!(is_absolute_windows_path(&pairs));
        let pairs_over = format!("{pairs}{emoji}");
        assert!(pairs_over.chars().count() < MAX_PATH_UTF16_UNITS);
        assert!(!is_absolute_windows_path(&pairs_over));
    }
}
