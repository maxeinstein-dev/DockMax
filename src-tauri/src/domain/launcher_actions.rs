//! The menu of actions of a result of the launcher (spec 0012 L6): which
//! actions each kind of result has, in what order, what each one does (an
//! action of the closed list, `launcher::Action`) and when Ctrl+C copies
//! the result instead of the text. Pure: no Windows, no database.

use super::apps::is_valid_app_id;
use super::launcher::Action;
use super::launcher_files::is_openable;

/// What can be done to a result, in the order the menu shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// What Enter does.
    Open,
    /// Opens it as administrator (Ctrl+Shift+Enter, as in Windows' Run).
    OpenAsAdmin,
    /// Pins the program to the dock, or takes it out of it.
    Pin,
    Unpin,
    /// Shows it selected in a File Explorer window.
    Reveal,
    /// Copies its full path.
    CopyPath,
    /// Copies a sum's result.
    CopyResult,
    /// Brings an open window to the front (L7: what Enter does to one).
    Switch,
    /// Asks an open window to close (`WM_CLOSE`).
    CloseWindow,
    /// Puts an item of the clipboard's history back as what is copied
    /// (what Enter does to one).
    CopyClip,
    /// Opens an item of the history that is one `https://` link.
    OpenLink,
}

/// What a result is, as the menu sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject<'a> {
    /// A program of the Apps folder: its id, whether the workspace in use
    /// has it pinned, and whether it has a file of its own to show (a
    /// classic program; a Store app has none).
    App {
        id: &'a str,
        pinned: bool,
        has_file: bool,
    },
    /// A file, by its path.
    File {
        path: &'a str,
    },
    Folder,
    /// A settings page (of Windows or of the Dockyard) and a web search:
    /// they only open.
    Page,
    Web,
    /// A sum's result: it is copied.
    Calculator,
    /// An open window (L7), and whether its program's file can be shown in
    /// File Explorer (`launcher_windows::is_revealable`).
    Window {
        revealable: bool,
    },
    /// An item of the clipboard's history, and whether its text is one
    /// `https://` link (`launcher_clipboard::link`).
    Clip {
        link: bool,
    },
}

/// The kinds of file Windows runs as administrator: programs, scripts,
/// installers, consoles and shortcuts to them.
const ELEVATED_EXTENSIONS: [&str; 7] = ["exe", "com", "bat", "cmd", "msi", "msc", "lnk"];

/// Whether `path` is a file Windows can run as administrator (by its
/// extension).
pub fn runs_as_admin(path: &str) -> bool {
    let name = path.rsplit('\\').next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && ELEVATED_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
    })
}

/// Whether the program `id` of the Apps folder is a packaged app (the Store's
/// and the like): its id is `PackageFamilyName!App`. A classic program's id
/// is a path, a known folder's or an AppID without `!`.
pub fn is_packaged(id: &str) -> bool {
    id.contains('!')
}

/// Whether the program `id` may have a file to show in File Explorer: a
/// classic one. Whether it really has one is asked of the shell.
pub fn may_have_file(id: &str) -> bool {
    is_valid_app_id(id) && !is_packaged(id)
}

/// The actions of `subject`, in the menu's order.
pub fn verbs(subject: Subject<'_>) -> Vec<Verb> {
    match subject {
        Subject::App {
            pinned, has_file, ..
        } => {
            let mut verbs = vec![
                Verb::Open,
                Verb::OpenAsAdmin,
                if pinned { Verb::Unpin } else { Verb::Pin },
            ];
            if has_file {
                verbs.push(Verb::Reveal);
            }
            verbs
        }
        Subject::File { path } => {
            let mut verbs = vec![Verb::Open];
            if runs_as_admin(path) {
                verbs.push(Verb::OpenAsAdmin);
            }
            verbs.extend([Verb::Reveal, Verb::CopyPath]);
            verbs
        }
        Subject::Folder => vec![Verb::Open, Verb::Reveal, Verb::CopyPath],
        Subject::Page | Subject::Web => vec![Verb::Open],
        Subject::Calculator => vec![Verb::CopyResult],
        Subject::Window { revealable } => {
            let mut verbs = vec![Verb::Switch, Verb::CloseWindow];
            if revealable {
                verbs.push(Verb::Reveal);
            }
            verbs
        }
        Subject::Clip { link } => {
            let mut verbs = vec![Verb::CopyClip];
            if link {
                verbs.push(Verb::OpenLink);
            }
            verbs
        }
    }
}

/// Whether the actions make a menu: with one action only (a page, a web
/// search, a sum), Shift+Enter does it at once and the menu does not open.
pub fn opens_menu(verbs: &[Verb]) -> bool {
    verbs.len() > 1
}

/// What Ctrl+C copies of a result with these actions, if anything: its path,
/// or a sum's result.
pub fn copy_verb(verbs: &[Verb]) -> Option<Verb> {
    verbs
        .iter()
        .copied()
        .find(|verb| matches!(verb, Verb::CopyPath | Verb::CopyResult | Verb::CopyClip))
}

/// Whether Ctrl+C, with the text field's selection from `start` to `end`,
/// copies the result instead of the text: only with nothing selected.
pub fn ctrl_c_copies_result(start: u32, end: u32, verbs: &[Verb]) -> bool {
    start == end && copy_verb(verbs).is_some()
}

/// The action of the closed list that `verb` does on a result whose own
/// action (Enter's) is `open`; `file` is the program's file, for a program
/// (or a window's program, L7) shown in File Explorer, or the link an item
/// of the clipboard's history is. `None` when `verb` does not apply to it.
pub fn action(verb: Verb, open: &Action, file: Option<&str>) -> Option<Action> {
    let action = match (verb, open) {
        (Verb::Open, _) => open.clone(),
        (Verb::OpenAsAdmin, Action::OpenApp(id)) => Action::OpenAppAsAdmin(id.clone()),
        (Verb::OpenAsAdmin, Action::OpenFile(path)) => Action::OpenFileAsAdmin(path.clone()),
        (Verb::Pin, Action::OpenApp(id)) => Action::PinApp(id.clone()),
        (Verb::Unpin, Action::OpenApp(id)) => Action::UnpinApp(id.clone()),
        (Verb::Reveal, Action::OpenApp(_)) => Action::Reveal(file?.to_owned()),
        (Verb::Reveal, Action::OpenFile(path)) => Action::Reveal(path.clone()),
        (Verb::CopyPath, Action::OpenFile(path)) => Action::CopyPath(path.clone()),
        (Verb::CopyResult, Action::Copy(text)) => Action::Copy(text.clone()),
        (Verb::Switch, Action::FocusWindow(_)) => open.clone(),
        (Verb::CloseWindow, Action::FocusWindow(hwnd)) => Action::CloseWindow(*hwnd),
        (Verb::Reveal, Action::FocusWindow(_)) => Action::Reveal(file?.to_owned()),
        (Verb::CopyClip, Action::CopyClip(_)) => open.clone(),
        (Verb::OpenLink, Action::CopyClip(_)) => Action::OpenUri(file?.to_owned()),
        _ => return None,
    };
    super::launcher::is_allowed(&action).then_some(action)
}

/// Whether `path` can be run as administrator from the launcher: a path the
/// launcher opens, of a kind Windows elevates.
pub fn is_elevatable(path: &str) -> bool {
    is_openable(path) && runs_as_admin(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORE: &str = "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App";
    const CLASSIC: &str = r"{6D809377-6AF0-444B-8957-A3773F02200E}\Notepad++\notepad++.exe";

    #[test]
    fn a_store_app_has_no_reveal_and_a_classic_one_with_a_file_has() {
        assert!(is_packaged(STORE));
        assert!(!may_have_file(STORE));
        assert!(may_have_file(CLASSIC));
        assert!(may_have_file("Microsoft.Office.WINWORD.EXE.15"));
        let store = verbs(Subject::App {
            id: STORE,
            pinned: false,
            has_file: false,
        });
        assert_eq!(store, [Verb::Open, Verb::OpenAsAdmin, Verb::Pin]);
        let classic = verbs(Subject::App {
            id: CLASSIC,
            pinned: false,
            has_file: true,
        });
        assert_eq!(
            classic,
            [Verb::Open, Verb::OpenAsAdmin, Verb::Pin, Verb::Reveal]
        );
        // A program has no path to copy: Ctrl+C copies the text.
        assert_eq!(copy_verb(&classic), None);
    }

    #[test]
    fn pin_turns_into_unpin_once_the_program_is_pinned() {
        let of = |pinned| {
            verbs(Subject::App {
                id: STORE,
                pinned,
                has_file: false,
            })
        };
        assert!(of(false).contains(&Verb::Pin));
        assert!(!of(false).contains(&Verb::Unpin));
        assert!(of(true).contains(&Verb::Unpin));
        assert!(!of(true).contains(&Verb::Pin));
        let open = Action::OpenApp(STORE.into());
        assert_eq!(
            action(Verb::Pin, &open, None),
            Some(Action::PinApp(STORE.into()))
        );
        assert_eq!(
            action(Verb::Unpin, &open, None),
            Some(Action::UnpinApp(STORE.into()))
        );
    }

    #[test]
    fn only_programs_and_shortcuts_open_as_administrator() {
        let common = verbs(Subject::File {
            path: r"C:\Users\ana\relatorio.pdf",
        });
        assert_eq!(common, [Verb::Open, Verb::Reveal, Verb::CopyPath]);
        for path in [
            r"C:\Tools\setup.EXE",
            r"C:\Users\ana\Desktop\Code.lnk",
            r"C:\scripts\build.cmd",
            r"\\srv\share\install.msi",
        ] {
            assert_eq!(
                verbs(Subject::File { path }),
                [Verb::Open, Verb::OpenAsAdmin, Verb::Reveal, Verb::CopyPath],
                "{path}"
            );
            assert!(is_elevatable(path), "{path}");
        }
        // The extension of the name, not of a folder; no name, no kind.
        for path in [r"C:\a.exe\notes", r"C:\Tools\.exe", r"C:\Tools\exe"] {
            assert!(!runs_as_admin(path), "{path}");
        }
        let open = Action::OpenFile(r"C:\Users\ana\relatorio.pdf".into());
        assert_eq!(action(Verb::OpenAsAdmin, &open, None), None);
        assert_eq!(
            action(Verb::CopyPath, &open, None),
            Some(Action::CopyPath(r"C:\Users\ana\relatorio.pdf".into()))
        );
        assert_eq!(
            action(Verb::Reveal, &open, None),
            Some(Action::Reveal(r"C:\Users\ana\relatorio.pdf".into()))
        );
    }

    #[test]
    fn a_folder_opens_shows_and_copies() {
        assert_eq!(
            verbs(Subject::Folder),
            [Verb::Open, Verb::Reveal, Verb::CopyPath]
        );
    }

    #[test]
    fn a_page_a_search_and_a_sum_have_no_menu() {
        for subject in [Subject::Page, Subject::Web] {
            let verbs = verbs(subject);
            assert_eq!(verbs, [Verb::Open]);
            assert!(!opens_menu(&verbs));
            assert_eq!(copy_verb(&verbs), None);
        }
        let sum = verbs(Subject::Calculator);
        assert_eq!(sum, [Verb::CopyResult]);
        assert!(!opens_menu(&sum));
        assert_eq!(
            action(Verb::CopyResult, &Action::Copy("3,5".into()), None),
            Some(Action::Copy("3,5".into()))
        );
        assert!(opens_menu(&verbs(Subject::Folder)));
    }

    #[test]
    fn ctrl_c_copies_the_result_only_with_nothing_selected() {
        let file = verbs(Subject::File {
            path: r"C:\a\b.txt",
        });
        // The cursor anywhere, nothing selected: the path.
        assert!(ctrl_c_copies_result(0, 0, &file));
        assert!(ctrl_c_copies_result(4, 4, &file));
        // Some text selected: the text, as always.
        assert!(!ctrl_c_copies_result(0, 3, &file));
        assert!(!ctrl_c_copies_result(2, 5, &file));
        // A result with nothing to copy: the text field's Ctrl+C.
        assert!(!ctrl_c_copies_result(0, 0, &verbs(Subject::Page)));
        assert!(ctrl_c_copies_result(0, 0, &verbs(Subject::Calculator)));
    }

    #[test]
    fn a_program_is_shown_only_with_its_file() {
        let open = Action::OpenApp(CLASSIC.into());
        assert_eq!(action(Verb::Reveal, &open, None), None);
        assert_eq!(
            action(
                Verb::Reveal,
                &open,
                Some(r"C:\Program Files\Notepad++\notepad++.exe")
            ),
            Some(Action::Reveal(
                r"C:\Program Files\Notepad++\notepad++.exe".into()
            ))
        );
        // A file the launcher would not open is not shown either.
        assert_eq!(action(Verb::Reveal, &open, Some("notepad++.exe")), None);
    }

    #[test]
    fn a_window_switches_closes_and_shows_its_program() {
        let shown = verbs(Subject::Window { revealable: true });
        assert_eq!(shown, [Verb::Switch, Verb::CloseWindow, Verb::Reveal]);
        assert!(opens_menu(&shown));
        // A Store app's window: its file is not shown.
        assert_eq!(
            verbs(Subject::Window { revealable: false }),
            [Verb::Switch, Verb::CloseWindow]
        );
        // A window has no path to copy: Ctrl+C copies the text.
        assert_eq!(copy_verb(&shown), None);
        assert!(!ctrl_c_copies_result(0, 0, &shown));
        let open = Action::FocusWindow(0x42);
        assert_eq!(action(Verb::Switch, &open, None), Some(open.clone()));
        assert_eq!(
            action(Verb::CloseWindow, &open, None),
            Some(Action::CloseWindow(0x42))
        );
        let exe = r"C:\Windows\System32\notepad.exe";
        assert_eq!(
            action(Verb::Reveal, &open, Some(exe)),
            Some(Action::Reveal(exe.into()))
        );
        assert_eq!(action(Verb::Reveal, &open, None), None);
        // The others' verbs do nothing new to a window.
        for verb in [
            Verb::OpenAsAdmin,
            Verb::Pin,
            Verb::Unpin,
            Verb::CopyPath,
            Verb::CopyResult,
        ] {
            assert_eq!(action(verb, &open, Some(exe)), None, "{verb:?}");
        }
        // Nor the window's verbs to the others.
        let app = Action::OpenApp(STORE.into());
        assert_eq!(action(Verb::CloseWindow, &app, None), None);
        assert_eq!(action(Verb::Switch, &app, None), None);
        // A handle 0 is no window.
        assert_eq!(
            action(Verb::CloseWindow, &Action::FocusWindow(0), None),
            None
        );
    }

    #[test]
    fn a_clip_copies_and_a_link_also_opens() {
        let plain = verbs(Subject::Clip { link: false });
        assert_eq!(plain, [Verb::CopyClip]);
        // One action only: Shift+Enter copies, no menu.
        assert!(!opens_menu(&plain));
        let link = verbs(Subject::Clip { link: true });
        assert_eq!(link, [Verb::CopyClip, Verb::OpenLink]);
        assert!(opens_menu(&link));
        // Ctrl+C with nothing selected copies the item, as Enter does.
        assert_eq!(copy_verb(&link), Some(Verb::CopyClip));
        assert!(ctrl_c_copies_result(0, 0, &plain));
        let open = Action::CopyClip("{id}".into());
        assert_eq!(action(Verb::CopyClip, &open, None), Some(open.clone()));
        assert_eq!(
            action(Verb::OpenLink, &open, Some("https://example.com/a")),
            Some(Action::OpenUri("https://example.com/a".into()))
        );
        // No link, or one the launcher does not open: nothing.
        assert_eq!(action(Verb::OpenLink, &open, None), None);
        assert_eq!(
            action(Verb::OpenLink, &open, Some("http://example.com")),
            None
        );
        // The others' verbs do nothing to an item.
        for verb in [
            Verb::OpenAsAdmin,
            Verb::Pin,
            Verb::Reveal,
            Verb::CopyPath,
            Verb::CopyResult,
            Verb::Switch,
            Verb::CloseWindow,
        ] {
            assert_eq!(
                action(verb, &open, Some("https://example.com")),
                None,
                "{verb:?}"
            );
        }
    }

    #[test]
    fn a_verb_that_does_not_apply_gives_no_action() {
        let page = Action::OpenUri("ms-settings:network-wifi".into());
        for verb in [
            Verb::OpenAsAdmin,
            Verb::Pin,
            Verb::Unpin,
            Verb::Reveal,
            Verb::CopyPath,
            Verb::CopyResult,
            Verb::Switch,
            Verb::CloseWindow,
            Verb::CopyClip,
            Verb::OpenLink,
        ] {
            assert_eq!(action(verb, &page, None), None, "{verb:?}");
        }
        assert_eq!(action(Verb::Open, &page, None), Some(page));
    }
}
