//! The launcher's paths (spec 0012 L5), as in the Ulauncher: a query that
//! starts as a path (`C:\`, `~\`, `%APPDATA%\`, `\\server\share\`) lists the
//! folder typed, filtered by what comes after its last `\`. Tab completes
//! the selected row into the field and Backspace on a `\` goes up a folder.
//! Pure: the folder is read by `windows::launcher::paths`, with
//! `std::fs::read_dir`, off the launcher's thread.
//!
//! In this mode only the paths answer: no apps, no web search.

use super::launcher::{Band, band, normalize};
use super::launcher_files::{Generation, MAX_PATH_CHARS, is_openable};

/// Most rows of a folder shown.
pub const LIMIT: usize = 50;
/// How long a folder has to be read, in milliseconds: past it the row says
/// the folder did not answer, and a late answer is dropped.
pub const TIMEOUT_MS: u32 = 2_000;
/// Most entries of a folder read: a huge folder stops there.
pub const MAX_ENTRIES: usize = 20_000;
/// The longest name of an environment variable taken (`%NAME%\`).
const MAX_VARIABLE: usize = 64;

/// A query that is a path: the folder typed and the filter after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathQuery {
    /// The folder as typed, up to its last `\` (`~\Docs\`): what Tab
    /// completes on.
    pub typed: String,
    /// The folder to read, expanded and ending in `\` (`C:\Users\ana\Docs\`);
    /// `None` when it cannot be (a variable that does not exist, a profile
    /// that is not a path): the row says the folder was not found.
    pub folder: Option<String>,
    /// What comes after the last `\`.
    pub filter: String,
}

/// How long the root of `query` is, its `\` included, when it starts as a
/// path: a drive (`C:\`), the profile (`~\`), a variable (`%APPDATA%\`) or a
/// share (`\\server\share\`). `None` when it is not a path.
pub fn root_len(query: &str) -> Option<usize> {
    let bytes = query.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\' {
        return Some(3);
    }
    if query.starts_with("~\\") {
        return Some(2);
    }
    if let Some(rest) = query.strip_prefix('%') {
        let (name, after) = rest.split_once('%')?;
        return (is_variable_name(name) && after.starts_with('\\')).then_some(name.len() + 3);
    }
    if let Some(rest) = query.strip_prefix("\\\\") {
        let mut parts = rest.splitn(3, '\\');
        let server = parts.next()?;
        let share = parts.next()?;
        // The share ends with its `\`: `\\server\share` is still typing.
        parts.next()?;
        let named = |part: &str| !part.is_empty() && part != "." && part != "?";
        return (named(server) && named(share) && !server.contains(':') && !share.contains(':'))
            .then_some(2 + server.len() + 1 + share.len() + 1);
    }
    None
}

/// The name of an environment variable as `%NAME%` takes it: letters,
/// digits, `_` and the parentheses of `ProgramFiles(x86)`.
fn is_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_VARIABLE
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')'))
}

/// Whether `query` is a path, so only the paths answer it.
pub fn is_path(query: &str) -> bool {
    root_len(query).is_some()
}

/// What `query` asks when it is a path; `None` when it is not one. `~` is
/// `profile` (`USERPROFILE`) and `%NAME%` is `variable(NAME)`, the user's
/// environment; each must be a full path. `.` and `..` are resolved, and a
/// `\` repeated counts once.
pub fn parse(
    query: &str,
    profile: &str,
    variable: impl Fn(&str) -> Option<String>,
) -> Option<PathQuery> {
    let root = root_len(query)?;
    // `root` ends in a `\`, so there is one.
    let last = query.rfind('\\')?;
    let typed = &query[..=last];
    let filter = &query[last + 1..];
    let base = match &query[..root] {
        drive if drive.as_bytes()[1] == b':' => Some(drive.to_owned()),
        "~\\" => Some(profile.to_owned()),
        variable_root if variable_root.starts_with('%') => {
            variable(&variable_root[1..variable_root.len() - 2])
        }
        share => Some(share.to_owned()),
    };
    let folder = base.and_then(|base| join(&base, &typed[root..]));
    Some(PathQuery {
        typed: typed.to_owned(),
        folder,
        filter: filter.to_owned(),
    })
}

/// `base` (a full path) with the folders of `rest` after it, `.` and `..`
/// resolved (never above the drive or the share), ending in `\`. `None`
/// when `base` is not a full path, or what comes out is not one the
/// launcher opens.
fn join(base: &str, rest: &str) -> Option<String> {
    let base = base.trim();
    let (anchor, below) = if let Some(share) = base.strip_prefix("\\\\") {
        let mut parts = share.splitn(3, '\\');
        let server = parts.next()?;
        let name = parts.next()?;
        (format!("\\\\{server}\\{name}"), parts.next().unwrap_or(""))
    } else {
        let bytes = base.as_bytes();
        if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
            return None;
        }
        if bytes.len() > 2 && bytes[2] != b'\\' {
            return None;
        }
        (base[..2].to_owned(), base.get(3..).unwrap_or(""))
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in below.split('\\').chain(rest.split('\\')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    let mut folder = anchor;
    folder.push('\\');
    for part in &parts {
        folder.push_str(part);
        folder.push('\\');
    }
    (folder.chars().count() <= MAX_PATH_CHARS && is_openable(&folder)).then_some(folder)
}

/// One entry of a folder, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub folder: bool,
    /// Hidden or of the system (the attributes), or a name starting with
    /// `.`: shown only when the filter starts with `.`.
    pub hidden: bool,
    /// The name normalized, made once for every key typed.
    normalized: String,
}

impl Entry {
    pub fn new(name: &str, folder: bool, hidden: bool) -> Entry {
        Entry {
            name: name.to_owned(),
            folder,
            hidden: hidden || name.starts_with('.'),
            normalized: normalize(name),
        }
    }
}

/// The entries the launcher shows for `filter`, at most `limit`: the
/// folders first, then the files. With a filter, only the names it matches
/// in the bands of the apps (a file's name without its extension counts
/// too), the better band first; then in alphabetical order, without case
/// nor accents. The hidden ones only when the filter starts with `.`.
pub fn list<'a>(entries: &'a [Entry], filter: &str, limit: usize) -> Vec<&'a Entry> {
    let query = normalize(filter);
    let hidden = filter.starts_with('.');
    let mut shown: Vec<(u8, &Entry)> = entries
        .iter()
        .filter(|entry| hidden || !entry.hidden)
        .filter_map(|entry| {
            if query.is_empty() {
                return Some((0, entry));
            }
            let stem = match entry.normalized.rsplit_once('.') {
                Some((stem, _)) if !entry.folder && !stem.is_empty() => Some(stem),
                _ => None,
            };
            [Some(entry.normalized.as_str()), stem]
                .into_iter()
                .flatten()
                .filter_map(|name| band(&query, name, None))
                .min()
                .map(|found: Band| (found as u8, entry))
        })
        .collect();
    shown.sort_by(|a, b| {
        b.1.folder
            .cmp(&a.1.folder)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.normalized.cmp(&b.1.normalized))
            .then_with(|| a.1.name.cmp(&b.1.name))
    });
    shown
        .into_iter()
        .take(limit)
        .map(|(_, entry)| entry)
        .collect()
}

/// What Tab leaves in the field for the row `name` of the folder typed
/// `typed`: the name after it, and a `\` for a folder, which enters it.
pub fn complete(typed: &str, name: &str, folder: bool) -> String {
    let mut text = format!("{typed}{name}");
    if folder {
        text.push('\\');
    }
    text
}

/// What Backspace leaves in the field `text` when it ends in `\`: the
/// folder above, as in the Ulauncher (`C:\Users\ana\` is `C:\Users\`).
/// `None` when it does not end in `\`, or above it there is no path left
/// (`C:\`, `~\`, `\\server\share\`): then Backspace erases a character, as
/// always.
pub fn up(text: &str) -> Option<String> {
    if !text.ends_with('\\') || !is_path(text) {
        return None;
    }
    let (above, _) = text.trim_end_matches('\\').rsplit_once('\\')?;
    let above = format!("{above}\\");
    is_path(&above).then_some(above)
}

/// Why a folder shows no rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// It does not exist, is a file, or cannot be expanded.
    NotFound,
    /// Windows refused to read it.
    Denied,
    /// It did not answer in `TIMEOUT_MS`.
    Slow,
}

/// Where the reading of the folder typed is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// No folder.
    Idle,
    /// The read of this generation is running.
    Waiting(u64),
    Ready(Vec<Entry>),
    Failed(Problem),
}

/// The folder being browsed: read once while its entries are filtered at
/// each key, read again when the folder typed changes. Each read takes the
/// next generation; an answer of an older one (the typing went on to
/// another folder, the launcher closed) is dropped.
#[derive(Debug, Clone)]
pub struct Browse {
    generation: Generation,
    folder: Option<String>,
    pub reading: Reading,
}

impl Default for Browse {
    fn default() -> Browse {
        Browse {
            generation: Generation::default(),
            folder: None,
            reading: Reading::Idle,
        }
    }
}

impl Browse {
    /// The folder typed is `folder` (`None`: it cannot be expanded): the
    /// generation of a read to start, or `None` when it is the folder
    /// already read (compared without case) or there is nothing to read.
    pub fn want(&mut self, folder: Option<&str>) -> Option<u64> {
        let Some(folder) = folder else {
            self.generation.next();
            self.folder = None;
            self.reading = Reading::Failed(Problem::NotFound);
            return None;
        };
        if self
            .folder
            .as_deref()
            .is_some_and(|known| known.eq_ignore_ascii_case(folder))
        {
            return None;
        }
        let generation = self.generation.next();
        self.folder = Some(folder.to_owned());
        self.reading = Reading::Waiting(generation);
        Some(generation)
    }

    /// The read `generation` answered; whether it was taken (it is the
    /// last one, still waited for).
    pub fn answered(&mut self, generation: u64, read: Result<Vec<Entry>, Problem>) -> bool {
        if self.reading != Reading::Waiting(generation) || !self.generation.is_current(generation) {
            return false;
        }
        self.reading = match read {
            Ok(entries) => Reading::Ready(entries),
            Err(problem) => Reading::Failed(problem),
        };
        true
    }

    /// The read `generation` ran out of time; whether that changed what is
    /// shown. Its answer, if it ever comes, is dropped.
    pub fn timed_out(&mut self, generation: u64) -> bool {
        if self.reading != Reading::Waiting(generation) {
            return false;
        }
        self.generation.next();
        self.reading = Reading::Failed(Problem::Slow);
        true
    }

    /// No folder any more (the query is not a path, the launcher closed):
    /// a read still running answers nothing.
    pub fn clear(&mut self) {
        self.generation.next();
        self.folder = None;
        self.reading = Reading::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r"C:\Users\ana";

    fn variable(name: &str) -> Option<String> {
        match name.to_ascii_uppercase().as_str() {
            "APPDATA" => Some(r"C:\Users\ana\AppData\Roaming".to_owned()),
            "PROGRAMFILES(X86)" => Some(r"C:\Program Files (x86)".to_owned()),
            "HOMESHARE" => Some(r"\\srv\home\ana".to_owned()),
            "PATHEXT" => Some(".COM;.EXE".to_owned()),
            _ => None,
        }
    }

    fn parsed(query: &str) -> Option<PathQuery> {
        parse(query, PROFILE, variable)
    }

    fn folder_of(query: &str) -> Option<String> {
        parsed(query).and_then(|query| query.folder)
    }

    #[test]
    fn the_mode_starts_with_a_drive_the_profile_a_variable_or_a_share() {
        for path in [
            r"C:\",
            r"d:\",
            r"C:\Users\ana\Doc",
            r"~\",
            r"~\Documents\",
            r"%APPDATA%\",
            r"%appdata%\Micro",
            r"%ProgramFiles(x86)%\",
            r"\\servidor\compartilhamento\",
            r"\\srv\share\pasta\a",
        ] {
            assert!(is_path(path), "{path}");
        }
        for not in [
            "",
            "c",
            "C:",
            "C:/",
            "C:a",
            "1:\\",
            "~",
            "~/",
            "~ana",
            "%APPDATA%",
            "%APPDATA",
            "%%\\",
            "%A B%\\",
            "%A;B%\\",
            r"\\",
            r"\\srv",
            r"\\srv\",
            r"\\srv\share",
            r"\\.\pipe\x",
            r"\\?\C:\x",
            r"\\srv\\x\",
            "g dockmax",
            "2+2",
            "relatorio",
        ] {
            assert!(!is_path(not), "{not:?}");
        }
    }

    #[test]
    fn the_profile_and_the_variables_are_expanded() {
        assert_eq!(folder_of(r"~\").as_deref(), Some(r"C:\Users\ana\"));
        assert_eq!(
            folder_of(r"~\Documents\Rel").as_deref(),
            Some(r"C:\Users\ana\Documents\")
        );
        assert_eq!(
            folder_of(r"%APPDATA%\Microsoft\").as_deref(),
            Some(r"C:\Users\ana\AppData\Roaming\Microsoft\")
        );
        assert_eq!(
            folder_of(r"%appdata%\").as_deref(),
            Some(r"C:\Users\ana\AppData\Roaming\")
        );
        assert_eq!(
            folder_of(r"%ProgramFiles(x86)%\").as_deref(),
            Some(r"C:\Program Files (x86)\")
        );
        assert_eq!(
            folder_of(r"%HOMESHARE%\Docs\").as_deref(),
            Some(r"\\srv\home\ana\Docs\")
        );
        // A path mode all the same, with no folder: "Pasta não encontrada".
        let unknown = parsed(r"%NOPE%\x").unwrap();
        assert_eq!((unknown.folder, unknown.filter.as_str()), (None, "x"));
        // A variable that is not a path, and a profile that is not one.
        assert_eq!(folder_of(r"%PATHEXT%\"), None);
        assert_eq!(parse(r"~\", "", variable).unwrap().folder, None);
        assert_eq!(parse(r"~\", "ana", variable).unwrap().folder, None);
    }

    #[test]
    fn the_folder_and_the_filter_split_at_the_last_backslash() {
        let query = parsed(r"C:\Users\ana\Docu").unwrap();
        assert_eq!(query.typed, r"C:\Users\ana\");
        assert_eq!(query.folder.as_deref(), Some(r"C:\Users\ana\"));
        assert_eq!(query.filter, "Docu");
        let query = parsed(r"d:\").unwrap();
        assert_eq!(
            (query.folder.as_deref(), query.filter.as_str()),
            (Some(r"d:\"), "")
        );
        let query = parsed(r"\\srv\share\Pasta\rel").unwrap();
        assert_eq!(query.folder.as_deref(), Some(r"\\srv\share\Pasta\"));
        assert_eq!(query.filter, "rel");
        // `.`, `..` and a repeated `\` are resolved, never above the root.
        assert_eq!(
            folder_of(r"C:\Users\.\ana\..\\bob\").as_deref(),
            Some(r"C:\Users\bob\")
        );
        assert_eq!(folder_of(r"C:\..\..\").as_deref(), Some(r"C:\"));
        assert_eq!(
            folder_of(r"\\srv\share\..\x\").as_deref(),
            Some(r"\\srv\share\x\")
        );
        assert_eq!(
            folder_of(r"~\..\").as_deref(),
            Some(r"C:\Users\"),
            "the profile's parent"
        );
        // What no path has makes no folder.
        assert_eq!(folder_of(r"C:\a|b\"), None);
    }

    fn entries() -> Vec<Entry> {
        vec![
            Entry::new("zeta.txt", false, false),
            Entry::new("Área de Trabalho", true, false),
            Entry::new("documentos", true, false),
            Entry::new("Relatório.docx", false, false),
            Entry::new("alfa.txt", false, false),
            Entry::new("Bravo", true, false),
            Entry::new("desktop.ini", false, true),
            Entry::new(".git", true, false),
            Entry::new("NTUSER.DAT", false, true),
        ]
    }

    fn names(listed: &[&Entry]) -> Vec<String> {
        listed.iter().map(|entry| entry.name.clone()).collect()
    }

    #[test]
    fn the_folders_come_first_then_the_files_in_alphabetical_order() {
        let entries = entries();
        assert_eq!(
            names(&list(&entries, "", LIMIT)),
            [
                "Área de Trabalho",
                "Bravo",
                "documentos",
                "alfa.txt",
                "Relatório.docx",
                "zeta.txt"
            ]
        );
        assert_eq!(list(&entries, "", 2).len(), 2);
    }

    #[test]
    fn the_filter_matches_in_the_bands_of_the_apps() {
        let entries = vec![
            Entry::new("relatorio antigo.zip", false, false),
            Entry::new("Relatório.docx", false, false),
            Entry::new("meu relatorio.txt", false, false),
            Entry::new("Relatorios", true, false),
            Entry::new("outro.txt", false, false),
        ];
        // Without case nor accents; the folders first, then the better band.
        assert_eq!(
            names(&list(&entries, "RELATORIO", LIMIT)),
            [
                "Relatorios",
                "Relatório.docx",
                "relatorio antigo.zip",
                "meu relatorio.txt"
            ]
        );
        assert_eq!(names(&list(&entries, "ou", LIMIT)), ["outro.txt"]);
        assert!(list(&entries, "xyz", LIMIT).is_empty());
    }

    #[test]
    fn the_hidden_ones_show_only_with_a_dot() {
        let entries = entries();
        let shown = names(&list(&entries, "", LIMIT));
        for hidden in ["desktop.ini", ".git", "NTUSER.DAT"] {
            assert!(!shown.contains(&hidden.to_owned()), "{hidden}");
        }
        assert_eq!(
            names(&list(&entries, "desktop", LIMIT)),
            Vec::<String>::new()
        );
        assert_eq!(names(&list(&entries, ".g", LIMIT)), [".git"]);
        // With a dot, the hidden ones by their attributes too.
        let all = names(&list(&entries, ".", LIMIT));
        assert!(all.contains(&".git".to_owned()), "{all:?}");
    }

    #[test]
    fn tab_completes_the_row_and_enters_a_folder() {
        assert_eq!(complete(r"~\", "Documents", true), r"~\Documents\");
        assert_eq!(
            complete(r"C:\Users\ana\", "relatório.docx", false),
            r"C:\Users\ana\relatório.docx"
        );
        assert_eq!(complete(r"%APPDATA%\", "Code", true), r"%APPDATA%\Code\");
        // What Tab leaves is the folder typed, and lists it.
        let entered = parsed(&complete(r"C:\Users\", "ana", true)).unwrap();
        assert_eq!(entered.folder.as_deref(), Some(r"C:\Users\ana\"));
        assert_eq!(entered.filter, "");
    }

    #[test]
    fn backspace_on_a_backslash_goes_up_a_folder() {
        assert_eq!(up(r"C:\Users\ana\").as_deref(), Some(r"C:\Users\"));
        assert_eq!(up(r"C:\Users\").as_deref(), Some(r"C:\"));
        assert_eq!(
            up(r"~\Documents\Contratos\").as_deref(),
            Some(r"~\Documents\")
        );
        assert_eq!(up(r"~\Documents\").as_deref(), Some(r"~\"));
        assert_eq!(up(r"%APPDATA%\Microsoft\").as_deref(), Some(r"%APPDATA%\"));
        assert_eq!(up(r"\\srv\share\x\").as_deref(), Some(r"\\srv\share\"));
        assert_eq!(up(r"C:\a\\").as_deref(), Some(r"C:\"));
        // At the root, or not on a `\`: a character, as always.
        for typing in [
            r"C:\",
            r"~\",
            r"%APPDATA%\",
            r"\\srv\share\",
            r"C:\Users\an",
            "relatorio",
            r"a\b\",
        ] {
            assert_eq!(up(typing), None, "{typing:?}");
        }
    }

    #[test]
    fn a_read_of_an_older_folder_is_dropped() {
        let mut browse = Browse::default();
        let first = browse.want(Some(r"C:\Users\")).unwrap();
        // The same folder, filtered: not read again.
        assert_eq!(browse.want(Some(r"c:\users\")), None);
        let second = browse.want(Some(r"C:\Windows\")).unwrap();
        assert!(!browse.answered(first, Ok(vec![Entry::new("ana", true, false)])));
        assert_eq!(browse.reading, Reading::Waiting(second));
        assert!(browse.answered(second, Err(Problem::Denied)));
        assert_eq!(browse.reading, Reading::Failed(Problem::Denied));
        // The answer once taken is not taken again.
        assert!(!browse.answered(second, Ok(Vec::new())));
        // Out of time: "the folder did not answer", and the late answer is
        // dropped.
        let third = browse.want(Some(r"\\srv\share\")).unwrap();
        assert!(browse.timed_out(third));
        assert!(!browse.answered(third, Ok(Vec::new())));
        assert_eq!(browse.reading, Reading::Failed(Problem::Slow));
        // The launcher closed: a read still running answers nothing.
        let fourth = browse.want(Some(r"D:\")).unwrap();
        browse.clear();
        assert!(!browse.answered(fourth, Ok(Vec::new())));
        assert_eq!(browse.reading, Reading::Idle);
        // A folder that cannot be expanded is not read.
        assert_eq!(browse.want(None), None);
        assert_eq!(browse.reading, Reading::Failed(Problem::NotFound));
    }
}
