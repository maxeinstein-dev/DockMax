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

/// Whether the window belongs to the program: it declares one of the
/// program's AppIDs, or it runs the program's file (so every window of one
/// executable, the profiles of a browser, belongs to the one program).
pub fn matches(window: &WindowFacts, identity: &Identity) -> bool {
    if let Some(aumid) = window.aumid.as_deref() {
        let aumid = aumid.to_lowercase();
        if identity.ids.contains(&aumid) {
            return true;
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
                Some("Chrome.UserData.Profile1"),
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
