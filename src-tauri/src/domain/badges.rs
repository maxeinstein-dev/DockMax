//! The notification badges (spec 0014): the number a program puts on its
//! button of the Windows taskbar, read there and drawn on its icon of the
//! dock. Pure rules: the taskbar is read in `windows/badges.rs` and the
//! badge drawn in `windows/native_dock`.

use super::open_apps::Identity;

/// What a program's button of the taskbar carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// A number of things unread.
    Count(u32),
    /// A mark with no number (a glyph of the program's own).
    Dot,
}

impl Badge {
    /// The text drawn in the badge: the number, `99+` above 99, nothing for a
    /// mark with no number.
    pub fn label(self) -> Option<String> {
        match self {
            Badge::Count(count) if count > MAX_SHOWN => Some(format!("{MAX_SHOWN}+")),
            Badge::Count(count) => Some(count.to_string()),
            Badge::Dot => None,
        }
    }
}

/// The largest number a badge shows whole.
pub const MAX_SHOWN: u32 = 99;

/// The badge in the texts under a button of the taskbar, the first that is
/// one: a number (`"3"`, or `"99+"` as the taskbar may abbreviate it) is a
/// count; a short mark that is not a word (`"!"`, `"•"`) is a badge with no
/// number. Anything else is a label of the button (a window's title, when
/// the taskbar shows them), not a badge; a count of zero is no badge.
pub fn parse(texts: &[&str]) -> Option<Badge> {
    texts.iter().find_map(|text| {
        let text = text.trim();
        let digits = text.strip_suffix('+').unwrap_or(text);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            // A number too long for a u32 is still more than 99.
            let count = digits.parse().unwrap_or(u32::MAX);
            return (count > 0).then_some(Badge::Count(count));
        }
        let short = (1..=2).contains(&text.chars().count());
        (short && !text.chars().any(char::is_alphanumeric)).then_some(Badge::Dot)
    })
}

/// What `AutomationId` names the program of a button of the taskbar by:
/// `"Appid: <AppID>"`. The AppID, lowercased; `None` for anything else.
pub fn app_id_of(automation_id: &str) -> Option<String> {
    let id = automation_id.strip_prefix("Appid:")?.trim();
    (!id.is_empty()).then(|| id.to_lowercase())
}

/// The known folder and the rest of an AppID the shell made from a
/// program's file (`{6D809377-…}\FileZilla FTP Client\filezilla.exe`, for a
/// program that declares none): the folder's GUID, with its braces, and the
/// path inside it.
pub fn known_folder_path(app_id: &str) -> Option<(&str, &str)> {
    let end = app_id.find('}')?;
    if !app_id.starts_with('{') {
        return None;
    }
    let rest = app_id[end + 1..].strip_prefix('\\')?;
    (!rest.is_empty()).then(|| (&app_id[..=end], rest))
}

/// A button of the taskbar, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// Its name: the program's, then what the taskbar says of it
    /// (`"WhatsApp - 1 janela em execução fixado"`).
    pub name: String,
    /// The AppID it is for, lowercased (`app_id_of`).
    pub app_id: Option<String>,
    /// The program's file, lowercased, when the AppID was made from it
    /// (`known_folder_path`, the folder resolved).
    pub path: Option<String>,
    pub badge: Option<Badge>,
}

/// Whether the button's name is the program `name`'s: the name, alone or
/// followed by `" - "` and what the taskbar says. For a button that names no
/// AppID only.
fn named(button: &Button, name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && button
            .name
            .strip_prefix(name)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(" - "))
}

/// The badge of a pinned program of the dock: the button for one of its
/// AppIDs or its file, else (a button that names no AppID) the one with
/// its name.
pub fn for_pinned(buttons: &[Button], identity: Option<&Identity>, name: &str) -> Option<Badge> {
    let by_id = identity.and_then(|identity| {
        buttons.iter().find(|button| {
            button
                .app_id
                .as_ref()
                .is_some_and(|id| identity.ids.contains(id))
                || (button.path.is_some() && button.path == identity.target)
        })
    });
    by_id
        .or_else(|| {
            buttons
                .iter()
                .find(|button| button.app_id.is_none() && named(button, name))
        })?
        .badge
}

/// The badge of an open program that is not pinned, known by `key` (what its
/// windows are grouped by: their AppID, else their file, lowercased).
pub fn for_other(buttons: &[Button], key: Option<&str>, name: &str) -> Option<Badge> {
    let by_key = key.and_then(|key| {
        buttons.iter().find(|button| {
            button.app_id.as_deref() == Some(key) || button.path.as_deref() == Some(key)
        })
    });
    by_key
        .or_else(|| {
            buttons
                .iter()
                .find(|button| button.app_id.is_none() && named(button, name))
        })?
        .badge
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(name: &str, app_id: Option<&str>, badge: Option<Badge>) -> Button {
        Button {
            name: name.into(),
            app_id: app_id.map(str::to_lowercase),
            path: None,
            badge,
        }
    }

    const WHATSAPP: &str = "5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App";

    #[test]
    fn a_number_shows_whole_up_to_99() {
        assert_eq!(Badge::Count(1).label().as_deref(), Some("1"));
        assert_eq!(Badge::Count(99).label().as_deref(), Some("99"));
        assert_eq!(Badge::Count(100).label().as_deref(), Some("99+"));
        assert_eq!(Badge::Count(u32::MAX).label().as_deref(), Some("99+"));
        assert_eq!(Badge::Dot.label(), None);
    }

    #[test]
    fn the_badge_is_the_number_under_the_button() {
        assert_eq!(parse(&["1"]), Some(Badge::Count(1)));
        assert_eq!(parse(&[" 12 "]), Some(Badge::Count(12)));
        assert_eq!(parse(&["99+"]), Some(Badge::Count(99)));
        assert_eq!(parse(&["123456789012345"]), Some(Badge::Count(u32::MAX)));
        assert_eq!(parse(&["", "7"]), Some(Badge::Count(7)));
    }

    #[test]
    fn a_short_mark_is_a_badge_with_no_number() {
        assert_eq!(parse(&["!"]), Some(Badge::Dot));
        assert_eq!(parse(&["•"]), Some(Badge::Dot));
    }

    #[test]
    fn a_title_or_zero_is_no_badge() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[""]), None);
        assert_eq!(parse(&["0"]), None);
        // The taskbar's labels, when it shows them, are titles.
        assert_eq!(parse(&["Caixa de entrada - Outlook"]), None);
        assert_eq!(parse(&["A"]), None);
        assert_eq!(parse(&["Ok"]), None);
    }

    #[test]
    fn the_automation_id_names_the_app_id() {
        assert_eq!(
            app_id_of(&format!("Appid: {WHATSAPP}")).as_deref(),
            Some(WHATSAPP.to_lowercase().as_str())
        );
        assert_eq!(app_id_of("Appid: Chrome").as_deref(), Some("chrome"));
        assert_eq!(app_id_of("Appid: "), None);
        assert_eq!(app_id_of("StartButton"), None);
    }

    #[test]
    fn an_app_id_made_from_a_file_names_a_known_folder() {
        assert_eq!(
            known_folder_path(
                r"{6D809377-6AF0-444B-8957-A3773F02200E}\FileZilla FTP Client\filezilla.exe"
            ),
            Some((
                "{6D809377-6AF0-444B-8957-A3773F02200E}",
                r"FileZilla FTP Client\filezilla.exe"
            ))
        );
        assert_eq!(known_folder_path("Chrome"), None);
        assert_eq!(known_folder_path("{abc}"), None);
        assert_eq!(known_folder_path(r"{abc}\"), None);
        assert_eq!(known_folder_path(r"x{abc}\y"), None);
    }

    #[test]
    fn a_pinned_program_takes_the_badge_of_the_button_for_its_app_id() {
        let buttons = [
            button(
                "Google Chrome - 1 janela em execução fixado",
                Some("Chrome"),
                None,
            ),
            button(
                "WhatsApp - 1 janela em execução fixado",
                Some(WHATSAPP),
                Some(Badge::Count(3)),
            ),
        ];
        let whatsapp = Identity::new([WHATSAPP], None);
        let chrome = Identity::new(["Chrome"], None);

        assert_eq!(
            for_pinned(&buttons, Some(&whatsapp), "WhatsApp"),
            Some(Badge::Count(3))
        );
        assert_eq!(for_pinned(&buttons, Some(&chrome), "Google Chrome"), None);
        // The name alone does not take a button that names another AppID.
        assert_eq!(for_pinned(&buttons, None, "WhatsApp"), None);
    }

    #[test]
    fn a_pinned_program_takes_the_badge_of_the_button_for_its_file() {
        let exe = r"c:\program files\filezilla ftp client\filezilla.exe";
        let buttons = [Button {
            path: Some(exe.into()),
            ..button(
                "FileZilla - 1 janela em execução",
                Some(r"{6d809377-6af0-444b-8957-a3773f02200e}\filezilla ftp client\filezilla.exe"),
                Some(Badge::Dot),
            )
        }];
        let filezilla = Identity::new(["FileZilla"], Some(exe));

        assert_eq!(
            for_pinned(&buttons, Some(&filezilla), "FileZilla"),
            Some(Badge::Dot)
        );
    }

    #[test]
    fn a_button_that_names_no_app_id_is_matched_by_the_name() {
        let buttons = [
            button("Teams (trabalho) - 1 janela", None, Some(Badge::Count(9))),
            button("Teams - 2 janelas em execução", None, Some(Badge::Count(2))),
            button("Outlook", None, Some(Badge::Count(5))),
        ];

        assert_eq!(for_pinned(&buttons, None, "Teams"), Some(Badge::Count(2)));
        assert_eq!(for_pinned(&buttons, None, "Outlook"), Some(Badge::Count(5)));
        assert_eq!(for_pinned(&buttons, None, "Out"), None);
        assert_eq!(for_pinned(&buttons, None, ""), None);
    }

    #[test]
    fn an_open_program_takes_the_badge_of_the_button_for_its_key() {
        let buttons = [
            button("Google Chrome", Some("Chrome"), Some(Badge::Count(1))),
            button(
                "Max Alfama - 1 janela",
                Some("Chrome.UserData.Profile1"),
                Some(Badge::Count(4)),
            ),
        ];

        assert_eq!(
            for_other(
                &buttons,
                Some("chrome.userdata.profile1"),
                "Google Chrome (Max Alfama)"
            ),
            Some(Badge::Count(4))
        );
        assert_eq!(for_other(&buttons, Some("other"), "Other"), None);
    }
}
