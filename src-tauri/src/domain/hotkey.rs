//! The launcher's hotkey (spec 0012 L3): a combination of modifiers and one
//! key, kept in the settings as text (`Alt+Space`, `Ctrl+Shift+K`). Pure.
//!
//! The keys are the letters, the digits, F1–F24 and Space. A combination is
//! refused when it has no modifier, when its only modifier is Shift or Ctrl
//! with a letter or a digit (it would take typing or an editing shortcut,
//! such as Ctrl+C, from every program), and when Windows or the Dockyard keep
//! it for themselves (Win and one key, Alt+F4, Ctrl+Alt and a digit).

use std::fmt;

/// The hotkey unless the user changes it.
pub const DEFAULT: &str = "Alt+Space";

/// `RegisterHotKey`'s modifier flags.
pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

/// The key of a combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `A`–`Z`.
    Letter(char),
    /// `0`–`9`.
    Digit(char),
    /// F1–F24.
    Function(u8),
    Space,
}

impl Key {
    fn parse(text: &str) -> Option<Key> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c @ 'A'..='Z'), None) => return Some(Key::Letter(c)),
            (Some(c @ '0'..='9'), None) => return Some(Key::Digit(c)),
            _ => {}
        }
        if text == "Space" {
            return Some(Key::Space);
        }
        let number: u8 = text.strip_prefix('F')?.parse().ok()?;
        // "F01" is not a key's name.
        ((1..=24).contains(&number) && text == format!("F{number}"))
            .then_some(Key::Function(number))
    }

    /// The virtual-key code `RegisterHotKey` takes.
    pub fn vk(self) -> u32 {
        match self {
            Key::Letter(c) | Key::Digit(c) => c as u32,
            Key::Function(n) => 0x6F + u32::from(n),
            Key::Space => 0x20,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Letter(c) | Key::Digit(c) => write!(f, "{c}"),
            Key::Function(n) => write!(f, "F{n}"),
            Key::Space => f.write_str("Space"),
        }
    }
}

/// A combination of modifiers and one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: Key,
}

/// Why a combination can't be the launcher's hotkey; must match
/// `HotkeyProblem` in `src/core/launcher.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    /// Not a combination written as the settings keep it.
    Malformed,
    NoModifier,
    /// Shift or Ctrl alone with a letter or a digit.
    TakesTyping,
    /// Windows or the Dockyard keep it.
    Reserved,
}

impl Hotkey {
    /// `text` as the settings keep it: the modifiers in the order Ctrl, Alt,
    /// Shift, Win, each once, then the key, joined by `+`.
    pub fn parse(text: &str) -> Option<Hotkey> {
        let parts: Vec<&str> = text.split('+').collect();
        let (key, modifiers) = parts.split_last()?;
        let mut hotkey = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            key: Key::parse(key)?,
        };
        let mut last = 0;
        for modifier in modifiers {
            let (place, flag) = match *modifier {
                "Ctrl" => (1, &mut hotkey.ctrl),
                "Alt" => (2, &mut hotkey.alt),
                "Shift" => (3, &mut hotkey.shift),
                "Win" => (4, &mut hotkey.win),
                _ => return None,
            };
            if place <= last {
                return None;
            }
            last = place;
            *flag = true;
        }
        Some(hotkey)
    }

    /// `RegisterHotKey`'s modifier flags (without `MOD_NOREPEAT`).
    pub fn modifiers(self) -> u32 {
        [
            (self.ctrl, MOD_CONTROL),
            (self.alt, MOD_ALT),
            (self.shift, MOD_SHIFT),
            (self.win, MOD_WIN),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .fold(0, |flags, (_, flag)| flags | flag)
    }

    /// Why it can't be the launcher's hotkey, if it can't.
    pub fn problem(self) -> Option<Problem> {
        let Hotkey {
            ctrl,
            alt,
            shift,
            win,
            key,
        } = self;
        let typing = matches!(key, Key::Letter(_) | Key::Digit(_));
        if !(ctrl || alt || shift || win) {
            return Some(Problem::NoModifier);
        }
        if !alt && !win && (shift && !ctrl || ctrl && !shift && typing) {
            return Some(Problem::TakesTyping);
        }
        let only_win = win && !ctrl && !alt && !shift;
        let alt_f4 = alt && !ctrl && !shift && !win && key == Key::Function(4);
        let workspace = ctrl && alt && !shift && !win && matches!(key, Key::Digit(_));
        if only_win || alt_f4 || workspace {
            return Some(Problem::Reserved);
        }
        None
    }
}

/// Why `text` can't be the launcher's hotkey, if it can't.
pub fn check(text: &str) -> Result<Hotkey, Problem> {
    let hotkey = Hotkey::parse(text).ok_or(Problem::Malformed)?;
    match hotkey.problem() {
        Some(problem) => Err(problem),
        None => Ok(hotkey),
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.ctrl, "Ctrl+"),
            (self.alt, "Alt+"),
            (self.shift, "Shift+"),
            (self.win, "Win+"),
        ] {
            if on {
                f.write_str(name)?;
            }
        }
        write!(f, "{}", self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_alt_space() {
        let hotkey = check(DEFAULT).unwrap();
        assert_eq!(hotkey.modifiers(), MOD_ALT);
        assert_eq!(hotkey.key.vk(), 0x20);
        assert_eq!(hotkey.to_string(), DEFAULT);
    }

    #[test]
    fn a_combination_is_written_one_way() {
        for text in [
            "Ctrl+Shift+K",
            "Alt+Shift+Space",
            "Alt+Win+Space",
            "Ctrl+Alt+Shift+Win+F12",
            "Alt+0",
            "Ctrl+F24",
        ] {
            let parsed = Hotkey::parse(text).map(|hotkey| hotkey.to_string());
            assert_eq!(parsed.as_deref(), Some(text));
        }
        // The modifiers out of order or twice, other names, other keys.
        for bad in [
            "",
            "Alt+",
            "+Space",
            "alt+space",
            "Alt+Alt+Space",
            "Shift+Ctrl+K",
            "Win+Alt+Space",
            "Alt+F0",
            "Alt+F25",
            "Alt+F01",
            "Alt+Enter",
            "Alt+a",
            "Alt+KK",
            "Alt + Space",
            "Meta+Space",
        ] {
            assert_eq!(Hotkey::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_keys_have_their_windows_codes() {
        assert_eq!(Key::Letter('K').vk(), 0x4B);
        assert_eq!(Key::Digit('7').vk(), 0x37);
        assert_eq!(Key::Function(1).vk(), 0x70);
        assert_eq!(Key::Function(24).vk(), 0x87);
        let all = Hotkey::parse("Ctrl+Alt+Shift+Win+F1").unwrap();
        assert_eq!(all.modifiers(), MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN);
    }

    #[test]
    fn combinations_that_take_typing_or_editing_are_refused() {
        assert_eq!(check("Space"), Err(Problem::NoModifier));
        assert_eq!(check("Alt+Enter"), Err(Problem::Malformed));
        for text in ["Shift+K", "Shift+Space", "Shift+F2", "Ctrl+C", "Ctrl+5"] {
            assert_eq!(check(text), Err(Problem::TakesTyping), "{text}");
        }
        for text in [
            "Ctrl+Space",
            "Ctrl+F5",
            "Ctrl+Shift+K",
            "Alt+K",
            "Alt+Shift+Space",
            "Ctrl+Alt+Space",
            "Alt+Win+Space",
        ] {
            assert!(check(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn what_windows_and_the_dockyard_keep_is_refused() {
        for text in [
            "Win+L",
            "Win+Space",
            "Win+1",
            "Win+F1",
            "Alt+F4",
            "Ctrl+Alt+1",
        ] {
            assert_eq!(check(text), Err(Problem::Reserved), "{text}");
        }
        assert!(check("Ctrl+Alt+K").is_ok());
        assert!(check("Shift+Win+K").is_ok());
    }
}
