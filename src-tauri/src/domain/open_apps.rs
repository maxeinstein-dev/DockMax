//! Which pinned programs are open (spec 0009): matching a window to a program
//! of the dock, summing the windows of each, and deciding what a click does.
//! Pure rules: the windows are read in `windows/openwin.rs` and acted on in
//! `windows/open_apps.rs`.

use serde::Serialize;

/// What a pinned program is known by to the shell, lowercased: the names a
/// window may declare for it and the file it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    /// The AppIDs of the program: the id in the Apps folder and the
    /// `System.AppUserModel.ID` of its item.
    pub ids: Vec<String>,
    /// The program file, for a classic program (a Store app has none).
    pub target: Option<String>,
}

impl Identity {
    /// An identity from the texts the shell gave for the program. Empty ones
    /// are left out; case does not matter.
    pub fn new<'a>(ids: impl IntoIterator<Item = &'a str>, target: Option<&str>) -> Identity {
        Identity {
            ids: ids
                .into_iter()
                .filter(|id| !id.is_empty())
                .map(str::to_lowercase)
                .collect(),
            target: target
                .filter(|path| !path.is_empty())
                .map(str::to_lowercase),
        }
    }
}

/// What is known of an open window, for matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowFacts {
    pub hwnd: isize,
    pub aumid: Option<String>,
    pub exe: Option<String>,
}

/// The AppID the shell gives File Explorer: the id of its item in the Apps
/// folder, the one the dock pins.
pub const EXPLORER_APP_ID: &str = "Microsoft.Windows.Explorer";

/// The AppID the shell groups the window by: the one it `declared`, else, for
/// a folder window of File Explorer (the `explorer.exe` in `windows_dir`,
/// which sets none), Explorer's own. Another window of that process that
/// declares an AppID keeps it, and an `explorer.exe` elsewhere is another
/// program.
pub fn shell_aumid(
    declared: Option<String>,
    exe: Option<&str>,
    windows_dir: &str,
) -> Option<String> {
    if declared.as_deref().is_some_and(|aumid| !aumid.is_empty()) {
        return declared;
    }
    let explorer = format!("{}\\explorer.exe", windows_dir.trim_end_matches('\\'));
    exe.filter(|exe| exe.eq_ignore_ascii_case(&explorer))
        .map(|_| EXPLORER_APP_ID.to_owned())
        .or(declared)
}

/// Whether the window belongs to the program: it declares one of the
/// program's AppIDs, or it runs the program's file.
pub fn matches(window: &WindowFacts, identity: &Identity) -> bool {
    if let Some(aumid) = window.aumid.as_deref() {
        let aumid = aumid.to_lowercase();
        if identity.ids.contains(&aumid) {
            return true;
        }
        // An AppID that extends one of the program's (`Chrome.UserData.
        // Profile1` of `Chrome`) is another instance of it that the shell keeps
        // apart, as Windows keeps the profiles of a browser apart: it is not
        // the program's by the file alone.
        if identity.ids.iter().any(|id| {
            aumid
                .strip_prefix(id.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
        }) {
            return false;
        }
    }
    match (&window.exe, &identity.target) {
        (Some(exe), Some(target)) => exe.to_lowercase() == *target,
        _ => false,
    }
}

/// The windows of one pinned program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The pinned program's id (the dock's own).
    pub id: String,
    /// Its windows, the topmost first.
    pub hwnds: Vec<isize>,
    /// One of them is the window the user is working in.
    pub active: bool,
}

/// The pinned programs that have windows. `windows` is in stacking order
/// (topmost first) and `foreground` is the window in front, or its owner
/// root. A program with no window is left out.
pub fn summarize(
    programs: &[(String, Identity)],
    windows: &[WindowFacts],
    foreground: Option<isize>,
) -> Vec<Found> {
    programs
        .iter()
        .filter_map(|(id, identity)| {
            let hwnds: Vec<isize> = windows
                .iter()
                .filter(|window| matches(window, identity))
                .map(|window| window.hwnd)
                .collect();
            (!hwnds.is_empty()).then(|| Found {
                id: id.clone(),
                active: foreground.is_some_and(|front| hwnds.contains(&front)),
                hwnds,
            })
        })
        .collect()
}

/// The windows that belong to no pinned program, grouped by program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// What the program is known by, lowercased: the window's AppID, else the
    /// program's file.
    pub key: String,
    /// Its windows, the topmost first.
    pub hwnds: Vec<isize>,
}

/// What a window is grouped by: the AppID it declares, else the file it runs;
/// `None` when it tells neither (nothing to show it as).
pub fn group_key(window: &WindowFacts) -> Option<String> {
    window
        .aumid
        .as_deref()
        .filter(|aumid| !aumid.is_empty())
        .or(window.exe.as_deref().filter(|exe| !exe.is_empty()))
        .map(str::to_lowercase)
}

/// The open programs that are not pinned: the windows no program of `found`
/// claims, grouped by `group_key`, in the order the groups first appeared in
/// `windows` (the topmost first). `order` is the order the dock showed them
/// last: groups it names keep their place and the new ones go at the end, so
/// the items do not shuffle each time the stacking changes.
pub fn others(windows: &[WindowFacts], found: &[Found], order: &[String]) -> Vec<Group> {
    let claimed: Vec<isize> = found
        .iter()
        .flat_map(|found| found.hwnds.iter().copied())
        .collect();
    let mut groups: Vec<Group> = Vec::new();
    for window in windows.iter().filter(|w| !claimed.contains(&w.hwnd)) {
        let Some(key) = group_key(window) else {
            continue;
        };
        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => group.hwnds.push(window.hwnd),
            None => groups.push(Group {
                key,
                hwnds: vec![window.hwnd],
            }),
        }
    }
    keep_order(&mut groups, |group| &group.key, order);
    groups
}

/// Puts `items` in the order they were shown in (`order` names them by key):
/// the ones it names keep their place and the others follow, in the order
/// they were given.
pub fn keep_order<T>(items: &mut [T], key: impl Fn(&T) -> &str, order: &[String]) {
    items.sort_by_key(|item| {
        order
            .iter()
            .position(|known| known == key(item))
            .unwrap_or(usize::MAX)
    });
}

/// An opaque name for a group the dock can send back, so the page never holds
/// a path: 16 hexadecimal digits of the FNV-1a hash of the key.
pub fn short_id(key: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// An open program that is not pinned, as the dock sees it: `{"key": "...",
/// "appId": "...", "name": "...", "count": 1, "active": false}`. `app_id` is
/// what `app_icon` takes (the Apps folder's id, or the program's file for one
/// the folder does not list); `key` is what `activate_open` takes. Carries no
/// window handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOther {
    pub key: String,
    pub app_id: String,
    pub name: String,
    pub count: usize,
    pub active: bool,
    /// The program has a badge to draw on its icon (the photo of a Chrome
    /// profile): `open_badge` gives it.
    pub badge: bool,
}

/// What the dock gets: the pinned programs that are open and the others.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OpenSnapshot {
    pub apps: Vec<OpenApp>,
    pub others: Vec<OpenOther>,
}

/// An open program as the dock sees it: `{"id": "...", "count": 2,
/// "active": true}`. Carries no window handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApp {
    pub id: String,
    pub count: usize,
    pub active: bool,
}

impl From<&Found> for OpenApp {
    fn from(found: &Found) -> Self {
        OpenApp {
            id: found.id.clone(),
            count: found.hwnds.len(),
            active: found.active,
        }
    }
}

/// What a click on an open program does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Put the window away: it is the only one and the user is in it.
    Minimize(isize),
    /// Bring the window to the front, restoring it if it was minimized.
    Focus(isize),
}

/// The click on a program with `hwnds` (topmost first) when the window in
/// front is `foreground`: minimize the only window the user is in; from one of
/// several, pass to the next in a fixed order (by handle, so the turn comes
/// round whatever the stacking does); otherwise take the topmost.
pub fn click(hwnds: &[isize], foreground: Option<isize>) -> Option<Click> {
    let topmost = *hwnds.first()?;
    let Some(front) = foreground.filter(|front| hwnds.contains(front)) else {
        return Some(Click::Focus(topmost));
    };
    if hwnds.len() == 1 {
        return Some(Click::Minimize(front));
    }
    let mut ordered = hwnds.to_vec();
    ordered.sort_unstable();
    let at = ordered.iter().position(|hwnd| *hwnd == front)?;
    Some(Click::Focus(ordered[(at + 1) % ordered.len()]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(hwnd: isize, aumid: Option<&str>, exe: Option<&str>) -> WindowFacts {
        WindowFacts {
            hwnd,
            aumid: aumid.map(str::to_owned),
            exe: exe.map(str::to_owned),
        }
    }

    const CHROME_EXE: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

    #[test]
    fn a_window_that_declares_an_appid_of_the_program_belongs_to_it() {
        let calculator = Identity::new(
            [
                "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
                "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            ],
            None,
        );

        assert!(matches(
            &window(
                1,
                Some("microsoft.windowscalculator_8wekyb3d8bbwe!APP"),
                Some(r"C:\Windows\System32\ApplicationFrameHost.exe")
            ),
            &calculator
        ));
        assert!(!matches(
            &window(2, Some("Other.App!App"), None),
            &calculator
        ));
    }

    #[test]
    fn a_window_that_runs_the_file_of_the_program_belongs_to_it() {
        let chrome = Identity::new(["Chrome"], Some(CHROME_EXE));

        assert!(matches(
            &window(
                1,
                Some("Some.Other.AppId"),
                Some(&CHROME_EXE.to_uppercase())
            ),
            &chrome
        ));
        assert!(matches(&window(2, None, Some(CHROME_EXE)), &chrome));
        assert!(!matches(
            &window(3, None, Some(r"C:\Windows\notepad.exe")),
            &chrome
        ));
    }

    #[test]
    fn a_window_whose_appid_extends_the_programs_is_another_instance_of_it() {
        // The pinned Chrome is the default profile's; the other profiles are
        // each their own item, as in the Windows taskbar.
        let chrome = Identity::new(["Chrome"], Some(CHROME_EXE));

        assert!(matches(
            &window(1, Some("Chrome"), Some(CHROME_EXE)),
            &chrome
        ));
        assert!(!matches(
            &window(2, Some("Chrome.UserData.Profile1"), Some(CHROME_EXE)),
            &chrome
        ));
        // A name that only begins the same way is not an extension of it.
        assert!(matches(
            &window(3, Some("Chromatic"), Some(CHROME_EXE)),
            &chrome
        ));
        // And a window that declares nothing still goes by the file.
        assert!(matches(&window(4, None, Some(CHROME_EXE)), &chrome));
    }

    #[test]
    fn a_program_with_no_file_and_no_appid_match_matches_nothing() {
        let store = Identity::new(["Claude_x!Claude"], None);

        assert!(!matches(&window(1, None, Some(CHROME_EXE)), &store));
        assert!(!matches(&window(1, None, None), &store));
        assert!(!matches(
            &window(1, None, None),
            &Identity::new([""], Some(""))
        ));
    }

    #[test]
    fn the_windows_of_each_program_are_counted_and_the_active_one_found() {
        let programs = vec![
            ("calc".to_owned(), Identity::new(["Calc!App"], None)),
            (
                "chrome".to_owned(),
                Identity::new(["Chrome"], Some(CHROME_EXE)),
            ),
            ("none".to_owned(), Identity::new(["Gone!App"], None)),
        ];
        let windows = vec![
            window(30, None, Some(CHROME_EXE)),
            window(20, Some("Calc!App"), None),
            window(10, None, Some(CHROME_EXE)),
        ];

        let found = summarize(&programs, &windows, Some(10));

        assert_eq!(
            found,
            vec![
                Found {
                    id: "calc".into(),
                    hwnds: vec![20],
                    active: false
                },
                Found {
                    id: "chrome".into(),
                    hwnds: vec![30, 10],
                    active: true
                },
            ]
        );
        assert_eq!(
            OpenApp::from(&found[1]),
            OpenApp {
                id: "chrome".into(),
                count: 2,
                active: true
            }
        );
        assert!(
            summarize(&programs, &windows, None)
                .iter()
                .all(|f| !f.active)
        );
        assert!(summarize(&programs, &[], None).is_empty());
    }

    #[test]
    fn windows_no_pinned_program_claims_are_grouped_by_program() {
        let windows = vec![
            window(1, None, Some(r"C:\Apps\code.exe")),
            window(2, Some("Pinned!App"), None),
            window(
                3,
                Some("Store.App!App"),
                Some(r"C:\Windows\ApplicationFrameHost.exe"),
            ),
            window(4, None, Some(r"C:\APPS\Code.exe")),
            window(5, None, None),
        ];
        let found = vec![Found {
            id: "pinned".into(),
            hwnds: vec![2],
            active: false,
        }];

        let groups = others(&windows, &found, &[]);

        assert_eq!(
            groups,
            vec![
                Group {
                    key: r"c:\apps\code.exe".into(),
                    hwnds: vec![1, 4]
                },
                Group {
                    key: "store.app!app".into(),
                    hwnds: vec![3]
                },
            ]
        );
    }

    const EXPLORER_EXE: &str = r"C:\Windows\explorer.exe";

    #[test]
    fn a_folder_window_of_file_explorer_has_explorers_appid() {
        let windows_dir = r"C:\WINDOWS\";

        assert_eq!(
            shell_aumid(None, Some(EXPLORER_EXE), windows_dir).as_deref(),
            Some(EXPLORER_APP_ID)
        );
        assert_eq!(
            shell_aumid(Some(String::new()), Some(EXPLORER_EXE), windows_dir).as_deref(),
            Some(EXPLORER_APP_ID)
        );
        // A window of Explorer's process that declares an AppID keeps it.
        assert_eq!(
            shell_aumid(
                Some("Microsoft.Windows.ControlPanel".into()),
                Some(EXPLORER_EXE),
                windows_dir
            )
            .as_deref(),
            Some("Microsoft.Windows.ControlPanel")
        );
        // An `explorer.exe` outside Windows is another program.
        assert_eq!(
            shell_aumid(None, Some(r"C:\Apps\explorer.exe"), windows_dir),
            None
        );
        assert_eq!(shell_aumid(None, Some(CHROME_EXE), windows_dir), None);
        assert_eq!(shell_aumid(None, None, windows_dir), None);
    }

    #[test]
    fn a_pinned_file_explorer_claims_its_folder_windows() {
        // As the shell gives them: the pinned item has no file, and the folder
        // window declares no AppID of its own.
        let programs = vec![(
            "explorer".to_owned(),
            Identity::new([EXPLORER_APP_ID, EXPLORER_APP_ID], None),
        )];
        let windows: Vec<WindowFacts> = [(1, None, EXPLORER_EXE), (2, None, CHROME_EXE)]
            .into_iter()
            .map(|(hwnd, declared, exe)| WindowFacts {
                hwnd,
                aumid: shell_aumid(declared, Some(exe), r"C:\Windows"),
                exe: Some(exe.to_owned()),
            })
            .collect();

        let found = summarize(&programs, &windows, Some(1));

        assert_eq!(
            found,
            vec![Found {
                id: "explorer".into(),
                hwnds: vec![1],
                active: true,
            }]
        );
        assert_eq!(
            others(&windows, &found, &[]),
            vec![Group {
                key: CHROME_EXE.to_lowercase(),
                hwnds: vec![2]
            }]
        );
    }

    #[test]
    fn the_groups_keep_the_order_they_were_shown_in() {
        let windows = vec![
            window(1, Some("A!App"), None),
            window(2, Some("B!App"), None),
            window(3, Some("C!App"), None),
        ];
        let order = vec!["c!app".to_owned(), "a!app".to_owned()];

        let keys: Vec<String> = others(&windows, &[], &order)
            .into_iter()
            .map(|group| group.key)
            .collect();

        // C and A where they were, B (new) at the end.
        assert_eq!(keys, ["c!app", "a!app", "b!app"]);
    }

    #[test]
    fn keeping_the_order_puts_the_known_first_and_the_new_at_the_end_as_given() {
        let mut items = vec!["new1", "b", "new2", "a"];
        let order = vec!["a".to_owned(), "b".to_owned()];

        keep_order(&mut items, |item| item, &order);

        assert_eq!(items, ["a", "b", "new1", "new2"]);
    }

    #[test]
    fn a_group_has_an_opaque_name_that_is_the_same_each_time() {
        let id = short_id(r"c:\apps\code.exe");

        assert_eq!(id, short_id(r"c:\apps\code.exe"));
        assert_ne!(id, short_id(r"c:\apps\other.exe"));
        assert_eq!(id.len(), 16);
        assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!id.contains("code"));
    }

    #[test]
    fn a_click_on_a_program_not_in_front_takes_its_topmost_window() {
        assert_eq!(click(&[7, 3], None), Some(Click::Focus(7)));
        assert_eq!(click(&[7, 3], Some(99)), Some(Click::Focus(7)));
    }

    #[test]
    fn a_click_on_the_only_window_in_front_minimizes_it() {
        assert_eq!(click(&[7], Some(7)), Some(Click::Minimize(7)));
    }

    #[test]
    fn a_click_with_several_windows_passes_to_the_next_in_a_fixed_order() {
        // Stacking changes between clicks; the turn goes by handle.
        assert_eq!(click(&[30, 10, 20], Some(30)), Some(Click::Focus(10)));
        assert_eq!(click(&[10, 30, 20], Some(10)), Some(Click::Focus(20)));
        assert_eq!(click(&[20, 10, 30], Some(20)), Some(Click::Focus(30)));
    }

    #[test]
    fn a_click_on_a_program_with_no_window_does_nothing() {
        assert_eq!(click(&[], Some(1)), None);
    }
}
