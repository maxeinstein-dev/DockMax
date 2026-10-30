//! The launcher's open windows (spec 0012 L7): which windows are listed,
//! what each is called, when a query looks for them and in what order they
//! come. Pure: no Windows; the windows are read in `windows/openwin.rs`, by
//! the dock's own rule of which window counts (`open_apps::has_button`).
//!
//! The windows match by their title and by their program's name, in the
//! bands of the apps (`launcher::band`), and come mixed after the apps and
//! the settings pages, before the files, at most `MIXED_LIMIT`. The keyword
//! `j ` (janelas) shows only windows, at most `ONLY_LIMIT`; with nothing
//! after it, every window, in the stacking order (the one used last first).

use super::apps::is_pinnable_path;
use super::launcher::{Band, MAX_QUERY_CHARS, band, normalize};

/// The keyword that shows only the open windows (`j`, of "janelas").
pub const KEYWORD: &str = "j";
/// Most windows mixed under the apps, and with the keyword.
pub const MIXED_LIMIT: usize = 8;
pub const ONLY_LIMIT: usize = 30;

/// An open window, as the launcher shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub hwnd: isize,
    pub title: String,
    /// Its program's name, as the dock shows it ("" when unknown).
    pub app: String,
    /// What its program's icon is read by (`apps::icon_png`): the program's
    /// id in the Apps folder, or its file; `None` for none (a glyph).
    pub icon: Option<String>,
    /// Its program's file, shown in File Explorer.
    pub exe: Option<String>,
}

/// How the windows show up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Under the apps and the pages, at most `MIXED_LIMIT`.
    Mixed,
    /// Alone (`j term`, or `j ` for all), at most `ONLY_LIMIT`.
    Only,
}

/// A query for windows: what they must match (empty: every one) and how
/// they show up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowQuery {
    pub term: String,
    pub mode: Mode,
}

impl WindowQuery {
    pub fn limit(&self) -> usize {
        match self.mode {
            Mode::Mixed => MIXED_LIMIT,
            Mode::Only => ONLY_LIMIT,
        }
    }
}

/// What `query` (as typed, spaces and all) asks of the windows: `j ` and
/// what follows asks for windows alone, every one when nothing follows,
/// unless the user made `j` a web keyword (`keyword_taken`, which then
/// wins); any other query that is not empty gets them mixed in. `j` alone
/// is typing like any other.
pub fn parse(query: &str, keyword_taken: bool) -> Option<WindowQuery> {
    let query = query.trim_start();
    if !keyword_taken
        && let Some((word, term)) = query.split_once(' ')
        && word.eq_ignore_ascii_case(KEYWORD)
    {
        return Some(WindowQuery {
            term: term.trim().to_owned(),
            mode: Mode::Only,
        });
    }
    let term = query.trim();
    (!term.is_empty()).then(|| WindowQuery {
        term: term.to_owned(),
        mode: Mode::Mixed,
    })
}

/// The best band the window matches `term` (normalized) in, by its title or
/// its program's name.
fn band_of(window: &Window, term: &str) -> Option<Band> {
    let title = normalize(&window.title);
    let app = normalize(&window.app);
    [band(term, &title, None), band(term, &app, None)]
        .into_iter()
        .flatten()
        .min()
}

/// The windows `query` finds, as indices into `windows` (in the stacking
/// order, the top first): by the band, and within a band in the stacking
/// order (the one used last first); every one, in that order, for `j `
/// alone. At most the query's limit.
pub fn rank(windows: &[Window], query: &WindowQuery) -> Vec<usize> {
    let term: String = normalize(&query.term)
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect();
    if term.is_empty() {
        return match query.mode {
            Mode::Only => (0..windows.len()).take(query.limit()).collect(),
            Mode::Mixed => Vec::new(),
        };
    }
    let mut found: Vec<(Band, usize)> = windows
        .iter()
        .enumerate()
        .filter_map(|(index, window)| Some((band_of(window, &term)?, index)))
        .collect();
    // Stable: the stacking order within a band.
    found.sort_by_key(|(band, _)| *band);
    found
        .into_iter()
        .take(query.limit())
        .map(|(_, index)| index)
        .collect()
}

/// The mixed list (no keyword): `first` (the sum, then the apps and the
/// settings pages, as `launcher::search` ordered them), then the open
/// windows in a band of their own, then the files (L4) after everything
/// else. The web search with the first keyword (`fallback`) only when
/// nothing at all matched.
pub fn mix<T>(first: Vec<T>, windows: Vec<T>, files: Vec<T>, fallback: Option<T>) -> Vec<T> {
    if first.is_empty() && windows.is_empty() && files.is_empty() {
        return fallback.into_iter().collect();
    }
    let mut mixed = first;
    mixed.extend(windows);
    mixed.extend(files);
    mixed
}

/// What tells a window's row from the others. Never kept in the use: a
/// window lives a short while, and its handle is reused by Windows.
pub fn key(hwnd: isize) -> String {
    format!("window:{hwnd}")
}

/// Whether a window is the Dockyard's own, left out of the list: of this
/// process (the dock, the launcher, Configurações), or of another one of the
/// same program (the helper, another copy). Paths compare without case.
pub fn is_ours(pid: u32, exe: Option<&str>, own_pid: u32, own_exe: Option<&str>) -> bool {
    pid == own_pid
        || matches!((exe, own_exe), (Some(exe), Some(own)) if !own.is_empty() && exe.eq_ignore_ascii_case(own))
}

/// What a window's program is shown as: its icon's id and its name. The
/// dock's own description of it first (`dock`, the program pinned or shown
/// open), then the Apps folder's program its AppID is (`catalog`), then the
/// program's file (`exe`) and its name without the extension. `None` for
/// the icon when nothing gives one.
pub fn describe(
    dock: Option<(String, String)>,
    catalog: Option<(String, String)>,
    exe: Option<&str>,
) -> (Option<String>, String) {
    if let Some((icon, name)) = dock.or(catalog) {
        return (Some(icon), name);
    }
    let Some(exe) = exe else {
        return (None, String::new());
    };
    let name = exe.rsplit('\\').next().unwrap_or(exe);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let icon = is_pinnable_path(exe).then(|| exe.to_owned());
    (icon, stem.to_owned())
}

/// Whether "Mostrar no Explorador" shows the program's file `exe`: a path
/// the launcher opens, and not a Store app's (its folder, `WindowsApps`, is
/// closed to the user) nor the frame Windows draws around one
/// (`ApplicationFrameHost.exe`).
pub fn is_revealable(exe: &str) -> bool {
    let lower = exe.to_lowercase();
    super::launcher_files::is_openable(exe)
        && !lower.contains(r"\windowsapps\")
        && !lower.ends_with(r"\applicationframehost.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(hwnd: isize, title: &str, app: &str) -> Window {
        Window {
            hwnd,
            title: title.to_owned(),
            app: app.to_owned(),
            icon: None,
            exe: None,
        }
    }

    /// In the stacking order, the top first.
    fn open() -> Vec<Window> {
        vec![
            window(1, "Relatório.docx - Word", "Word"),
            window(2, "Caixa de entrada - Outlook", "Outlook"),
            window(3, "dockmax - Visual Studio Code", "Visual Studio Code"),
            window(4, "Sem título - Bloco de Notas", "Bloco de Notas"),
            window(5, "Notas da reunião - Word", "Word"),
        ]
    }

    fn titles(windows: &[Window], found: &[usize]) -> Vec<String> {
        found.iter().map(|&i| windows[i].title.clone()).collect()
    }

    fn find(query: &str) -> Vec<String> {
        let windows = open();
        titles(&windows, &rank(&windows, &parse(query, false).unwrap()))
    }

    #[test]
    fn a_window_is_found_by_its_title_and_by_its_program() {
        // By the program's name: every Word window, the top one first.
        assert_eq!(
            find("word"),
            ["Relatório.docx - Word", "Notas da reunião - Word"]
        );
        // By a word of the title, without accents.
        assert_eq!(find("relatorio"), ["Relatório.docx - Word"]);
        // The initials of the program.
        assert_eq!(find("vsc"), ["dockmax - Visual Studio Code"]);
        assert!(find("zqxj").is_empty());
    }

    #[test]
    fn a_better_band_comes_before_the_stacking_order() {
        // "Notas da reunião" starts with "notas" (prefix); the Bloco de Notas
        // window, higher in the stack, only has it as a word.
        assert_eq!(
            find("notas"),
            ["Notas da reunião - Word", "Sem título - Bloco de Notas"]
        );
    }

    #[test]
    fn j_alone_lists_every_window_in_the_stacking_order() {
        let query = parse("j ", false).unwrap();
        assert_eq!(query.mode, Mode::Only);
        assert_eq!(query.term, "");
        assert_eq!(rank(&open(), &query), [0, 1, 2, 3, 4]);
        // `j term`: only the windows, by the term.
        let query = parse("  J  word ", false).unwrap();
        assert_eq!(query.mode, Mode::Only);
        assert_eq!(query.term, "word");
        assert_eq!(rank(&open(), &query), [0, 4]);
        // Up to `ONLY_LIMIT`.
        let many: Vec<Window> = (0..40).map(|i| window(i, "Janela", "App")).collect();
        assert_eq!(rank(&many, &parse("j ", false).unwrap()).len(), ONLY_LIMIT);
        assert_eq!(
            rank(&many, &parse("janela", false).unwrap()).len(),
            MIXED_LIMIT
        );
    }

    #[test]
    fn j_without_a_space_is_typing() {
        let query = parse("j", false).unwrap();
        assert_eq!(query.mode, Mode::Mixed);
        assert_eq!(parse("jogo", false).unwrap().mode, Mode::Mixed);
        assert_eq!(parse("", false), None);
        assert_eq!(parse("   ", false), None);
    }

    #[test]
    fn a_web_keyword_j_of_the_user_wins() {
        assert_eq!(parse("j word", true).unwrap().mode, Mode::Mixed);
    }

    #[test]
    fn the_default_keywords_do_not_take_j() {
        let defaults = crate::domain::launcher_web::defaults();
        // `w` is the Wikipédia's: the windows' keyword could not be it.
        assert!(defaults.iter().any(|keyword| keyword.keyword == "w"));
        assert!(
            !defaults
                .iter()
                .any(|keyword| keyword.keyword.eq_ignore_ascii_case(KEYWORD))
        );
        assert!(!KEYWORD.eq_ignore_ascii_case(crate::domain::launcher_files::KEYWORD));
    }

    #[test]
    fn the_dockyard_is_left_out() {
        let own = Some(r"C:\Program Files\Dockyard\dockyard.exe");
        // Its own process.
        assert!(is_ours(10, None, 10, own));
        // Another copy of it (the helper, a second one), whatever the case.
        assert!(is_ours(
            11,
            Some(r"c:\program files\dockyard\DOCKYARD.EXE"),
            10,
            own
        ));
        // Another program.
        assert!(!is_ours(
            12,
            Some(r"C:\Windows\System32\notepad.exe"),
            10,
            own
        ));
        // Its own file unknown: only the process.
        assert!(!is_ours(12, Some(r"C:\a.exe"), 10, None));
        assert!(!is_ours(12, Some(""), 10, Some("")));
    }

    #[test]
    fn a_window_is_described_by_the_dock_then_the_catalog_then_its_file() {
        let dock = Some(("Chrome".to_owned(), "Google Chrome (Ana)".to_owned()));
        let catalog = Some(("Chrome".to_owned(), "Google Chrome".to_owned()));
        let exe = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        assert_eq!(
            describe(dock, catalog.clone(), Some(exe)),
            (Some("Chrome".to_owned()), "Google Chrome (Ana)".to_owned())
        );
        assert_eq!(
            describe(None, catalog, Some(exe)),
            (Some("Chrome".to_owned()), "Google Chrome".to_owned())
        );
        assert_eq!(
            describe(None, None, Some(exe)),
            (Some(exe.to_owned()), "chrome".to_owned())
        );
        assert_eq!(describe(None, None, None), (None, String::new()));
    }

    #[test]
    fn a_store_app_s_file_is_not_shown() {
        assert!(is_revealable(r"C:\Windows\System32\notepad.exe"));
        assert!(!is_revealable(
            r"C:\Program Files\WindowsApps\Microsoft.WindowsCalculator_11.0_x64__8wekyb3d8bbwe\CalculatorApp.exe"
        ));
        assert!(!is_revealable(
            r"C:\Windows\System32\ApplicationFrameHost.exe"
        ));
        assert!(!is_revealable("notepad.exe"));
    }

    #[test]
    fn the_windows_come_after_the_apps_and_before_the_files() {
        let mixed = mix(
            vec!["= 4", "Word (app)", "Configurações"],
            vec!["Relatório.docx - Word"],
            vec!["relatorio.docx"],
            Some("Buscar na web"),
        );
        assert_eq!(
            mixed,
            [
                "= 4",
                "Word (app)",
                "Configurações",
                "Relatório.docx - Word",
                "relatorio.docx"
            ]
        );
        // A window alone keeps the web search away; nothing at all brings it.
        assert_eq!(
            mix(Vec::new(), vec!["Notas - Word"], Vec::new(), Some("web")),
            ["Notas - Word"]
        );
        assert_eq!(
            mix(Vec::<&str>::new(), Vec::new(), Vec::new(), Some("web")),
            ["web"]
        );
        assert!(mix(Vec::<&str>::new(), Vec::new(), Vec::new(), None).is_empty());
    }

    #[test]
    fn the_web_search_leaves_when_late_files_come() {
        // `relatorio` with the index cold: nothing yet, the web search.
        let before = mix(Vec::<&str>::new(), Vec::new(), Vec::new(), Some("web"));
        assert_eq!(before, ["web"]);
        // The late answer of the same typing comes: the files, no web.
        let after = mix(
            Vec::new(),
            Vec::new(),
            vec!["relatorio.pdf", "relatorio.docx"],
            Some("web"),
        );
        assert_eq!(after, ["relatorio.pdf", "relatorio.docx"]);
    }

    #[test]
    fn the_windows_mixed_in_follow_the_apps_bands() {
        // Mixed in, a query finds the windows by the same bands as the apps,
        // at most `MIXED_LIMIT`; the order among them is the band's, then
        // the stacking order.
        let windows = open();
        let names = titles(&windows, &rank(&windows, &parse("word", false).unwrap()));
        let mixed = mix(
            vec!["Word".to_owned()],
            names,
            vec!["word.txt".to_owned()],
            None,
        );
        assert_eq!(
            mixed,
            [
                "Word",
                "Relatório.docx - Word",
                "Notas da reunião - Word",
                "word.txt"
            ]
        );
    }

    #[test]
    fn a_window_s_key_is_its_handle() {
        assert_eq!(key(0x1234), "window:4660");
    }
}
