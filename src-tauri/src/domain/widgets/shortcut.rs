//! Shortcut widget config (spec 0003 §2.5): what the shortcut opens. Only
//! in the domain in this phase: it can't be created, and opening it comes
//! in phase 3.

use serde::{Deserialize, Serialize};
use url::Url;

use super::WidgetError;
use super::path::is_absolute_windows_path;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutConfig {
    pub target: ShortcutTarget,
}

impl ShortcutConfig {
    /// Checks the target's text only, with no disk or network access: a
    /// path must be an absolute Windows path (see
    /// [`is_absolute_windows_path`]) and a URL an `http` or `https` one
    /// with a host (see [`is_web_url`]).
    pub fn validate(&self) -> Result<(), WidgetError> {
        let valid = match &self.target {
            ShortcutTarget::Path { path } => is_absolute_windows_path(path),
            ShortcutTarget::Url { url } => is_web_url(url),
        };
        if valid {
            Ok(())
        } else {
            Err(WidgetError::InvalidTarget)
        }
    }
}

/// Whether `text` is an `http` or `https` URL (scheme in any case) with a
/// host and no credentials (`user:password@`), which a shortcut must never
/// store.
///
/// The WHATWG parser is lenient where a reader is not, so the text itself
/// is checked first, and what is stored is what was checked:
///
/// - it silently drops leading and trailing spaces and control characters,
///   and tabs and line breaks anywhere: rejected;
/// - it reads `http:host`, `http:/host` and `\` as `http://host` and `/`:
///   the text must start with `http://` or `https://`, and a `\` anywhere
///   is rejected;
/// - it reads an empty userinfo (`https://@host`) as no credentials, and
///   anything before an `@` as userinfo (`https://example.com@evil.com`
///   opens `evil.com`): an `@` anywhere in the authority is rejected.
fn is_web_url(text: &str) -> bool {
    if text.trim() != text || text.chars().any(char::is_control) || text.contains('\\') {
        return false;
    }
    let Some(rest) = strip_prefix_ignore_case(text, "https://")
        .or_else(|| strip_prefix_ignore_case(text, "http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let Ok(url) = Url::parse(text) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
}

/// `text` after `prefix` (ASCII), compared without regard to ASCII case.
fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
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

#[cfg(test)]
mod tests {
    use super::*;

    fn path(path: &str) -> ShortcutConfig {
        ShortcutConfig {
            target: ShortcutTarget::Path {
                path: path.to_owned(),
            },
        }
    }

    fn url(url: &str) -> ShortcutConfig {
        ShortcutConfig {
            target: ShortcutTarget::Url {
                url: url.to_owned(),
            },
        }
    }

    #[test]
    fn default_is_the_system_drive_root_and_valid() {
        let config = ShortcutConfig::default();
        assert_eq!(config, path(r"C:\"));
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn absolute_paths_are_accepted() {
        for text in [
            r"C:\Users\Public\Documents",
            r"C:\Program Files\App\app.exe",
            "C:/Users/Public",
            r"\\server\share\relatório.xlsx",
        ] {
            assert_eq!(path(text).validate(), Ok(()), "{text:?}");
        }
    }

    #[test]
    fn paths_that_are_not_absolute_are_rejected() {
        for text in [
            "",
            "app.exe",
            r"..\app.exe",
            "C:",
            "C:app.exe",
            r"\Windows",
            r"\\server",
            r"\\?\C:\app.exe",
            r"\\.\PhysicalDrive0",
            "C:\\app\0.exe",
            "https://example.com/",
        ] {
            assert_eq!(
                path(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
        let too_long = format!(r"C:\{}", "a".repeat(32_765));
        assert_eq!(path(&too_long).validate(), Err(WidgetError::InvalidTarget));
    }

    #[test]
    fn http_and_https_urls_with_a_host_are_accepted() {
        for text in [
            "https://example.com/docs",
            "http://example.com",
            "HTTPS://Example.com/",
            "https://example.com:8443/a?b=c#d",
            "http://192.168.0.10/",
            "http://[::1]:8080/",
            "http://localhost/",
            "https://exemplo.com.br/relatório",
        ] {
            assert_eq!(url(text).validate(), Ok(()), "{text:?}");
        }
    }

    #[test]
    fn other_schemes_are_rejected() {
        for text in [
            "ftp://example.com/",
            "file:///C:/Windows",
            "javascript:alert(1)",
            "data:text/html,oi",
            "mailto:alguem@example.com",
            "ms-settings:display",
            "C:\\Windows",
            "example.com",
            "//example.com/",
            "",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
    }

    #[test]
    fn urls_without_a_host_or_with_credentials_are_rejected() {
        for text in [
            "http://",
            "https:///",
            "http://user@example.com/",
            "https://user:secret@example.com/",
            "https://:secret@example.com/",
            "http://exa mple.com/",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
    }

    #[test]
    fn text_the_parser_would_silently_change_is_rejected() {
        for text in [
            " https://example.com/",
            "https://example.com/ ",
            "https://exam\tple.com/",
            "https://example.com/\n",
            "\u{0}https://example.com/",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
    }

    #[test]
    fn urls_must_start_with_the_scheme_and_two_slashes() {
        for text in [
            "http:example.com",
            "https:example.com/",
            "http:/example.com/",
            "https:/example.com/",
            r"https:\\example.com",
            r"https:\\example.com/",
            r"https:/\example.com/",
            "ht\u{00AD}tps://example.com/",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
        for text in ["HTTP://example.com/", "hTtPs://example.com/"] {
            assert_eq!(url(text).validate(), Ok(()), "{text:?}");
        }
    }

    #[test]
    fn urls_with_a_backslash_anywhere_are_rejected() {
        for text in [
            r"https://example.com\@evil.com",
            r"https://example.com\path",
            r"https://example.com/a\b",
            r"https://example.com/?q=a\b",
            r"https://example.com/#a\b",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
    }

    #[test]
    fn urls_with_an_at_sign_in_the_authority_are_rejected() {
        for text in [
            "https://@example.com/",
            "https://:@example.com/",
            "http://@@example.com/",
            "https://example.com@evil.com/",
            "https://example.com%40x@evil.com/",
            "https://@example.com",
            "https://@example.com?a",
            "https://@example.com#a",
        ] {
            assert_eq!(
                url(text).validate(),
                Err(WidgetError::InvalidTarget),
                "{text:?}"
            );
        }
        // After the authority, `@` is ordinary text.
        for text in [
            "https://example.com/@user",
            "https://example.com/a?mail=a@b.com",
            "https://example.com/#@top",
        ] {
            assert_eq!(url(text).validate(), Ok(()), "{text:?}");
        }
    }

    #[test]
    fn the_error_message_never_names_the_target() {
        for config in [
            path("relatorio.pdf"),
            url("https://user:secret@example.com/"),
        ] {
            let message = config.validate().unwrap_err().to_string();
            for secret in ["relatorio", "secret", "example", "/", "\\"] {
                assert!(!message.contains(secret), "{message}");
            }
        }
    }
}
