//! The launcher's files and folders (spec 0012 §5, L4): when a query looks
//! for them, the term each source is asked with, how the answers of the two
//! sources (Everything and the Windows Search index) are joined and ordered,
//! what a row shows, and the key its use is kept under. Pure: no Windows, no
//! clock, no database.
//!
//! From `MIN_CHARS` characters, the files come mixed under the apps, the
//! settings pages and the sum, in a band of their own after them, at most
//! `MIXED_LIMIT`. The keyword `f ` (f and a space) shows only files and
//! folders, at most `ONLY_LIMIT`.
//!
//! Nothing of the term becomes part of a command: it is cut into words of
//! the characters a file name can have, and each source gets them written in
//! its own syntax as literals (`everything_search`, `windows_search_sql`).

use std::collections::{HashMap, HashSet};

use super::launcher::{Band, band, normalize};

/// The keyword that shows only files and folders.
pub const KEYWORD: &str = "f";
/// How long a query must be (normalized) before the files are mixed in.
pub const MIN_CHARS: usize = 3;
/// Most files mixed under the other results, and with the keyword.
pub const MIXED_LIMIT: usize = 8;
pub const ONLY_LIMIT: usize = 30;
/// How long after the last key the files are looked for, in milliseconds.
pub const DEBOUNCE_MS: u32 = 120;
/// How long a source has to answer, in milliseconds.
pub const TIMEOUT_MS: u64 = 300;
/// How long an answer that missed `TIMEOUT_MS` is still waited for, in
/// milliseconds: the Windows Search cold, on the first search after the
/// launcher opens (the helper starting, COM and OLE DB loading).
pub const LATE_LIMIT_MS: u64 = 5_000;
/// How long the helper waits for that late answer at each ask, in
/// milliseconds: the requests of icons wait at most this much behind it.
pub const LATE_POLL_MS: u64 = 100;
/// The first line of an answer not yet whole: a source missed the time,
/// and what it finds may still come (`decode_answer`).
const LATE_MARK: &str = "+";
/// The longest path kept (a result, a key of the use).
pub const MAX_PATH_CHARS: usize = 1024;
/// Most words of a term, and the longest one.
const MAX_WORDS: usize = 8;
const MAX_WORD_CHARS: usize = 64;

/// The characters no file name has (plus the separators): dropped from the
/// term, so none of them reaches a source's syntax.
const NOT_IN_NAMES: [char; 9] = ['"', '*', '?', '<', '>', '|', ':', '\\', '/'];

/// How the files show up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Under the other results, at most `MIXED_LIMIT`.
    Mixed,
    /// Alone (`f term`), at most `ONLY_LIMIT`.
    Only,
}

/// A query for files: its words and how they show up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileQuery {
    pub words: Vec<String>,
    pub mode: Mode,
}

impl FileQuery {
    pub fn limit(&self) -> usize {
        match self.mode {
            Mode::Mixed => MIXED_LIMIT,
            Mode::Only => ONLY_LIMIT,
        }
    }
}

/// What `query` asks of the files: `f term` asks for files alone, unless
/// the user made `f` a web keyword (`keyword_taken`, which then wins); any
/// other query of at least `MIN_CHARS` characters gets them mixed in.
/// `None` when it asks for none, or nothing is left of the term.
pub fn parse(query: &str, keyword_taken: bool) -> Option<FileQuery> {
    let query = query.trim();
    if !keyword_taken
        && let Some((word, term)) = query.split_once(' ')
        && word.eq_ignore_ascii_case(KEYWORD)
    {
        let words = words(term);
        return (!words.is_empty()).then_some(FileQuery {
            words,
            mode: Mode::Only,
        });
    }
    if normalize(query).chars().count() < MIN_CHARS {
        return None;
    }
    let words = words(query);
    (!words.is_empty()).then_some(FileQuery {
        words,
        mode: Mode::Mixed,
    })
}

/// The words of `term` with only what a file name can have: no control
/// characters, none of `NOT_IN_NAMES`; at most `MAX_WORDS`, each at most
/// `MAX_WORD_CHARS`.
pub fn words(term: &str) -> Vec<String> {
    term.split(|c: char| c.is_whitespace() || NOT_IN_NAMES.contains(&c))
        .map(|word| {
            word.chars()
                .filter(|c| !c.is_control())
                .take(MAX_WORD_CHARS)
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .take(MAX_WORDS)
        .collect()
}

/// The search of Everything for `words`: each word in double quotes, which
/// Everything takes as a literal (its operators, wildcards and `regex:`-like
/// modifiers do nothing there); a space between them is AND. The words have
/// no double quote (`words`), so none can close the quotes.
pub fn everything_search(words: &[String]) -> String {
    words
        .iter()
        .map(|word| format!("\"{}\"", word.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The query of the Windows Search index for `words`, at most `limit`
/// rows: the path and the attributes (a folder's bit) of the files and
/// folders whose name has each word at a word's start.
///
/// `CONTAINS` and not `LIKE '%word%'`: `CONTAINS` uses the full-text index
/// (a `LIKE` with a leading `%` reads every name of the index), gives a rank
/// to order by, and ignores accents as the index does (`relatorio` finds
/// "Relatório"); a `LIKE` would not. What it misses is a word in the middle
/// of another (`orio`), which the launcher's apps find in their last bands
/// only.
///
/// Escaping: each word goes in double quotes inside a SQL string; the words
/// have no double quote nor `*` (`words`), and a single quote is doubled, as
/// SQL asks. `None` without words.
pub fn windows_search_sql(words: &[String], limit: usize) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let condition = words
        .iter()
        .map(|word| format!("\"{}*\"", word.replace(['"', '*'], "").replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(" AND ");
    Some(format!(
        "SELECT TOP {limit} System.ItemUrl, System.FileAttributes FROM SystemIndex \
         WHERE SCOPE='file:' AND CONTAINS(System.FileName, '{condition}') \
         ORDER BY System.Search.Rank DESC, System.DateAccessed DESC"
    ))
}

/// The path of an item of the Windows Search index from its
/// `System.ItemUrl` (`file:C:/Users/ana/a.txt`): `System.ItemPathDisplay`
/// is the path as Explorer shows it, with the folders' names translated
/// (`C:\Usuários`), which opens nothing. `None` when it is not a file on a
/// drive.
pub fn path_of_item_url(url: &str) -> Option<String> {
    let rest = url
        .get(..5)
        .filter(|scheme| scheme.eq_ignore_ascii_case("file:"))
        .map(|_| &url[5..])?;
    let path = rest.replace('/', "\\");
    is_valid_path(&path).then_some(path)
}

/// A file or a folder found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHit {
    /// Its full path (`C:\Users\ana\Documents\relatório.docx`).
    pub path: String,
    pub folder: bool,
}

impl FileHit {
    /// The name shown: the last part of the path (the drive, for a drive).
    pub fn name(&self) -> &str {
        let path = self.path.trim_end_matches('\\');
        match path.rsplit_once('\\') {
            Some((_, name)) if !name.is_empty() => name,
            _ => &self.path,
        }
    }

    /// The folder it is in; empty for a drive.
    pub fn parent(&self) -> &str {
        let path = self.path.trim_end_matches('\\');
        match path.rsplit_once('\\') {
            // `C:\file`: the parent is the drive itself, with its `\`.
            Some((parent, _)) if parent.len() == 2 => &self.path[..3],
            Some((parent, _)) => parent,
            None => "",
        }
    }
}

/// Whether `path` is one the launcher shows and opens: a full path on a
/// drive (`C:\...`, no network path, nothing relative), without control
/// characters nor those no path has, at most `MAX_PATH_CHARS`.
pub fn is_valid_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive =
        bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\';
    drive
        && path.chars().count() <= MAX_PATH_CHARS
        && !path[2..].contains(':')
        && !path
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '*' | '?' | '<' | '>' | '|' | '/'))
        && !path
            .split('\\')
            .skip(1)
            .any(|part| part == "." || part == "..")
}

/// Whether `path` is a full path on a share (`\\server\share\...`, L5): a
/// server and a share with a name (not the `\\.\` nor the `\\?\` of the
/// devices), and below them what `is_valid_path` allows.
pub fn is_valid_unc(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(r"\\") else {
        return false;
    };
    let mut parts = rest.split('\\');
    let named = |part: Option<&str>| {
        part.is_some_and(|part| !part.is_empty() && part != "." && part != "?")
    };
    named(parts.next())
        && named(parts.next())
        && path.chars().count() <= MAX_PATH_CHARS
        && !path.contains(':')
        && !path
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '*' | '?' | '<' | '>' | '|' | '/'))
        && !parts.any(|part| part == "." || part == "..")
}

/// Whether the launcher opens `path`: a full path on a drive, or on a share
/// typed by the user (L5).
pub fn is_openable(path: &str) -> bool {
    is_valid_path(path) || is_valid_unc(path)
}

/// The key the use of `hit` is kept under: `file:` and its path, with a
/// `\` at the end for a folder (so the recents know it without asking the
/// disk).
pub fn file_key(hit: &FileHit) -> String {
    let path = hit.path.trim_end_matches('\\');
    if hit.folder || path.len() == 2 {
        format!("file:{path}\\")
    } else {
        format!("file:{path}")
    }
}

/// The file or folder of a key `file_key` made; `None` for any other key.
pub fn hit_of_key(key: &str) -> Option<FileHit> {
    let path = key.strip_prefix("file:")?;
    let folder = path.ends_with('\\');
    let trimmed = path.trim_end_matches('\\');
    let path = if trimmed.len() == 2 { path } else { trimmed };
    is_openable(path).then(|| FileHit {
        path: path.to_owned(),
        folder,
    })
}

/// `folder` with the user's profile (`profile`) written `~`:
/// `C:\Users\ana\Documents` is `~\Documents`. Case does not matter; a folder
/// that only starts with the same letters (`C:\Users\anabela`) is left as is.
pub fn abbreviate(folder: &str, profile: &str) -> String {
    let profile = profile.trim_end_matches('\\');
    if profile.is_empty() || folder.len() < profile.len() {
        return folder.to_owned();
    }
    let Some((head, rest)) = folder.split_at_checked(profile.len()) else {
        return folder.to_owned();
    };
    if head.to_lowercase() == profile.to_lowercase() {
        if rest.is_empty() {
            return "~".to_owned();
        }
        if rest.starts_with('\\') {
            return format!("~{rest}");
        }
    }
    folder.to_owned()
}

/// The kinds of file whose icon is their own (a program's, a shortcut's):
/// read for each file. The others show their type's, read once.
const OWN_ICON: [&str; 8] = ["exe", "lnk", "ico", "url", "appref-ms", "msc", "cpl", "scr"];

/// What the icon of `hit` is kept under: its type's (`type:.pdf`), or its
/// own (`path:` and the path, lowercase) for a folder, a program, a shortcut
/// and a file without an extension.
pub fn icon_key(hit: &FileHit) -> String {
    let extension = hit
        .name()
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .filter(|extension| !extension.is_empty() && !extension.contains(' '));
    match extension {
        Some(extension) if !hit.folder && !OWN_ICON.contains(&extension.as_str()) => {
            format!("type:.{extension}")
        }
        _ => format!("path:{}", hit.path.to_lowercase()),
    }
}

/// The answers of the sources, in order (Everything's first), as one list:
/// each path once (compared without case, and without a `\` at the end),
/// the first answer kept; what is not a valid path is left out.
pub fn merge(sources: impl IntoIterator<Item = Vec<FileHit>>) -> Vec<FileHit> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for hit in sources.into_iter().flatten() {
        if !is_valid_path(&hit.path) {
            continue;
        }
        if seen.insert(hit.path.trim_end_matches('\\').to_lowercase()) {
            merged.push(hit);
        }
    }
    merged
}

/// The hits in the order the launcher shows them, at most `limit`: by how
/// the name, or a file's name without its extension, matches the words (the
/// bands of the apps, with the words joined by a space; a name that matches
/// in none after all of them), then by the
/// use (`usage`, each key's weight now), then in the order the sources gave.
pub fn rank(
    hits: Vec<FileHit>,
    words: &[String],
    usage: &HashMap<String, f64>,
    limit: usize,
) -> Vec<FileHit> {
    let query = normalize(&words.join(" "));
    let mut ranked: Vec<(Option<Band>, f64, usize, FileHit)> = hits
        .into_iter()
        .enumerate()
        .map(|(order, hit)| {
            let name = normalize(hit.name());
            // A file's name without its extension counts too:
            // `relatorio` is the exact name of "relatorio.txt".
            let stem = match name.rsplit_once('.') {
                Some((stem, _)) if !hit.folder && !stem.is_empty() => Some(stem),
                _ => None,
            };
            let found = [Some(name.as_str()), stem]
                .into_iter()
                .flatten()
                .filter_map(|name| band(&query, name, None))
                .min();
            let used = usage.get(&file_key(&hit)).copied().unwrap_or(0.0);
            (found, used, order, hit)
        })
        .collect();
    ranked.sort_by(|a, b| {
        let band = |found: Option<Band>| found.map_or(u8::MAX, |band| band as u8);
        band(a.0)
            .cmp(&band(b.0))
            .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.2.cmp(&b.2))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(.., hit)| hit)
        .collect()
}

/// Whether `hit` still matches `words`: each word (normalized) inside its
/// name. The rows of the last search that stay while a new one runs, so the
/// list does not blink at each key.
pub fn still_matches(hit: &FileHit, words: &[String]) -> bool {
    let name = normalize(hit.name());
    words
        .iter()
        .all(|word| name.contains(normalize(word).as_str()))
}

/// The hits as the helper sends them: one per line, `d` (a folder) or `f`
/// (a file), a tab and the path.
pub fn encode_hits(hits: &[FileHit]) -> String {
    let mut out = String::new();
    for hit in hits {
        out.push(if hit.folder { 'd' } else { 'f' });
        out.push('\t');
        out.push_str(&hit.path);
        out.push('\n');
    }
    out
}

/// The hits of `encode_hits`; a line that is not one, or with a path that
/// is not valid, is left out.
pub fn decode_hits(text: &str) -> Vec<FileHit> {
    text.lines()
        .filter_map(|line| {
            let (kind, path) = line.split_once('\t')?;
            let folder = match kind {
                "d" => true,
                "f" => false,
                _ => return None,
            };
            is_valid_path(path).then(|| FileHit {
                path: path.to_owned(),
                folder,
            })
        })
        .collect()
}

/// The answer of a search as the helper sends it: the hits
/// (`encode_hits`), after the line `LATE_MARK` when it is not whole (a
/// source missed `TIMEOUT_MS` and is still waited for).
pub fn encode_answer(hits: &[FileHit], whole: bool) -> String {
    let hits = encode_hits(hits);
    if whole {
        hits
    } else {
        format!("{LATE_MARK}\n{hits}")
    }
}

/// The hits of `encode_answer` and whether the answer is whole.
pub fn decode_answer(text: &str) -> (Vec<FileHit>, bool) {
    let whole = text.lines().next() != Some(LATE_MARK);
    (decode_hits(text), whole)
}

/// What the helper says when asked for the late answer of its last search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LateAnswer {
    /// Nothing is waited for any more (none was late, it came empty, it
    /// was too late or the source is gone): the launcher stops asking.
    Gone,
    /// Not yet: the launcher asks again.
    Waiting,
    /// It came: the hits of the search, the early ones and the late ones
    /// joined, for the launcher to show in place of the first answer.
    Came(Vec<FileHit>),
}

/// A `LateAnswer` as the helper sends it: nothing for `Gone` (and for an
/// empty `Came`, which changes nothing), `LATE_MARK` for `Waiting`, the
/// hits for `Came`.
pub fn encode_late(answer: &LateAnswer) -> String {
    match answer {
        LateAnswer::Gone => String::new(),
        LateAnswer::Waiting => format!("{LATE_MARK}\n"),
        LateAnswer::Came(hits) => encode_hits(hits),
    }
}

pub fn decode_late(text: &str) -> LateAnswer {
    if text.lines().next() == Some(LATE_MARK) {
        return LateAnswer::Waiting;
    }
    match decode_hits(text) {
        hits if hits.is_empty() => LateAnswer::Gone,
        hits => LateAnswer::Came(hits),
    }
}

/// A search of the helper waiting for the Windows Search index: its number
/// and what Everything gave, to be joined with the index's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexWait {
    number: u64,
    early: Vec<FileHit>,
}

/// What an answer of the index is to the search waited for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arrival {
    /// Of this search, in time: the joined hits go in its first answer.
    InTime(Vec<FileHit>),
    /// Of this search, after `TIMEOUT_MS` but within `LATE_LIMIT_MS`: the
    /// joined hits go after, and the launcher's list is redone with them.
    Late(Vec<FileHit>),
    /// Of an older search (the typing went on), or too late: dropped.
    Dropped,
}

impl IndexWait {
    pub fn new(number: u64, early: Vec<FileHit>) -> IndexWait {
        IndexWait { number, early }
    }

    /// What Everything gave (the first answer, when the index is late).
    pub fn early(&self) -> Vec<FileHit> {
        merge([self.early.clone()])
    }

    /// The answer `hits` of the index to the search `number`, `waited_ms`
    /// after this search was asked.
    pub fn arrival(&self, number: u64, hits: Vec<FileHit>, waited_ms: u64) -> Arrival {
        if number != self.number || self.expired(waited_ms) {
            return Arrival::Dropped;
        }
        let joined = merge([self.early.clone(), hits]);
        if waited_ms <= TIMEOUT_MS {
            Arrival::InTime(joined)
        } else {
            Arrival::Late(joined)
        }
    }

    /// Whether the index's answer is no longer waited for.
    pub fn expired(&self, waited_ms: u64) -> bool {
        waited_ms > LATE_LIMIT_MS
    }
}

/// Tells the searches of files apart: each one takes the next number, and
/// only the answer of the last one is shown. An answer of an older typing,
/// late, is dropped; a late answer of the typing still there is shown.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

impl Generation {
    /// A new search (or none any more): the ones before it are old.
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(1);
        self.0
    }

    /// Whether `generation` is the last search's.
    pub fn is_current(&self, generation: u64) -> bool {
        self.0 == generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> FileHit {
        FileHit {
            path: path.to_owned(),
            folder: false,
        }
    }

    fn folder(path: &str) -> FileHit {
        FileHit {
            path: path.to_owned(),
            folder: true,
        }
    }

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_files_come_from_three_characters() {
        assert_eq!(parse("re", false), None);
        assert_eq!(parse("  ré ", false), None);
        let query = parse("rel", false).unwrap();
        assert_eq!((query.mode, query.limit()), (Mode::Mixed, MIXED_LIMIT));
        assert_eq!(query.words, ["rel"]);
        assert_eq!(parse("relatorio mensal", false).unwrap().words.len(), 2);
    }

    #[test]
    fn the_keyword_shows_only_files() {
        let query = parse("f relatorio", false).unwrap();
        assert_eq!((query.mode, query.limit()), (Mode::Only, ONLY_LIMIT));
        assert_eq!(query.words, ["relatorio"]);
        // One letter is enough after the keyword; the case does not matter.
        assert_eq!(parse("F a", false).unwrap().mode, Mode::Only);
        // Without a term it is not the keyword: `f ` and `f` are typing.
        assert_eq!(parse("f ", false), None);
        assert_eq!(parse("f", false), None);
        // A word that only starts with f is a query like any other.
        assert_eq!(parse("foto", false).unwrap().mode, Mode::Mixed);
    }

    #[test]
    fn a_web_keyword_f_of_the_user_wins() {
        // `f relatorio` is then the user's web search: mixed in at most.
        assert_eq!(parse("f relatorio", true).unwrap().mode, Mode::Mixed);
    }

    #[test]
    fn the_default_web_keywords_do_not_take_f() {
        let defaults = crate::domain::launcher_web::defaults();
        assert!(
            !defaults
                .iter()
                .any(|keyword| keyword.keyword.eq_ignore_ascii_case(KEYWORD))
        );
    }

    #[test]
    fn the_term_keeps_only_what_a_name_has() {
        assert_eq!(
            words("a\"b* c?d|e<f>g:h"),
            strings(&["a", "b", "c", "d", "e", "f", "g", "h"])
        );
        assert_eq!(words(r"h\i/j"), ["h", "i", "j"]);
        assert_eq!(words("rel\u{7}at\u{0}orio"), ["relatorio"]);
        assert_eq!(words("   "), Vec::<String>::new());
        assert_eq!(words("a b c d e f g h i j").len(), MAX_WORDS);
        assert_eq!(
            words(&"x".repeat(MAX_WORD_CHARS + 5))[0].len(),
            MAX_WORD_CHARS
        );
        assert!(parse("f \"*?", false).is_none(), "nothing is left");
    }

    #[test]
    fn everything_gets_each_word_as_a_literal() {
        assert_eq!(
            everything_search(&strings(&["relatorio", "2024"])),
            "\"relatorio\" \"2024\""
        );
        // Everything's operators and modifiers stay inside the quotes.
        let query = parse("f regex:.* | !x <a>", false).unwrap();
        let search = everything_search(&query.words);
        assert_eq!(search, "\"regex\" \".\" \"!x\" \"a\"");
        assert_eq!(search.matches('"').count() % 2, 0);
    }

    #[test]
    fn windows_search_gets_each_word_escaped() {
        let sql = windows_search_sql(&strings(&["relatorio"]), 8).unwrap();
        assert!(sql.starts_with("SELECT TOP 8 System.ItemUrl"), "{sql}");
        assert!(
            sql.contains("CONTAINS(System.FileName, '\"relatorio*\"')"),
            "{sql}"
        );
        assert!(sql.contains("ORDER BY System.Search.Rank DESC"), "{sql}");
        // A quote of the term is doubled: the SQL string does not end.
        let sql = windows_search_sql(&strings(&["joe's", "x') OR 1=1 --"]), 30).unwrap();
        assert!(
            sql.contains("'\"joe''s*\" AND \"x'') OR 1=1 --*\"'"),
            "{sql}"
        );
        let condition = sql
            .split_once("CONTAINS(System.FileName, ")
            .unwrap()
            .1
            .rsplit_once(") ORDER BY")
            .unwrap()
            .0;
        // One SQL string: the quotes inside it are all doubled.
        let inner = &condition[1..condition.len() - 1];
        assert!(condition.starts_with('\'') && condition.ends_with('\''));
        assert!(!inner.replace("''", "").contains('\''), "{condition}");
        assert_eq!(windows_search_sql(&[], 8), None);
    }

    #[test]
    fn the_index_gives_the_path_in_its_url() {
        assert_eq!(
            path_of_item_url("file:C:/Users/Maxsuel Einstein/Área de Trabalho/a b.txt").as_deref(),
            Some(r"C:\Users\Maxsuel Einstein\Área de Trabalho\a b.txt")
        );
        assert_eq!(path_of_item_url("FILE:D:/x").as_deref(), Some(r"D:\x"));
        for bad in [
            "",
            "file:",
            "mapi://x/y",
            "file://server/share/a",
            "C:/a.txt",
            "fi",
        ] {
            assert_eq!(path_of_item_url(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_row_shows_the_name_and_its_folder() {
        let hit = file(r"C:\Users\ana\Documents\relatório.docx");
        assert_eq!(hit.name(), "relatório.docx");
        assert_eq!(hit.parent(), r"C:\Users\ana\Documents");
        let top = file(r"C:\pagefile.sys");
        assert_eq!((top.name(), top.parent()), ("pagefile.sys", r"C:\"));
        let drive = folder(r"D:\");
        assert_eq!((drive.name(), drive.parent()), (r"D:\", ""));
    }

    #[test]
    fn the_profile_is_written_as_a_tilde() {
        let profile = r"C:\Users\ana";
        assert_eq!(
            abbreviate(r"C:\Users\ana\Documents", profile),
            r"~\Documents"
        );
        assert_eq!(
            abbreviate(r"c:\users\ANA\Downloads", profile),
            r"~\Downloads"
        );
        assert_eq!(abbreviate(r"C:\Users\ana", profile), "~");
        assert_eq!(abbreviate(r"C:\Users\ana\", r"C:\Users\ana\"), r"~\");
        assert_eq!(
            abbreviate(r"C:\Users\anabela\Docs", profile),
            r"C:\Users\anabela\Docs"
        );
        assert_eq!(abbreviate(r"D:\Projetos", profile), r"D:\Projetos");
        assert_eq!(abbreviate(r"C:\Users", profile), r"C:\Users");
        assert_eq!(abbreviate(r"C:\x", ""), r"C:\x");
    }

    #[test]
    fn only_full_paths_on_a_drive_are_valid() {
        for good in [
            r"C:\a.txt",
            r"d:\Pasta\Sub",
            r"C:\",
            r"C:\Users\ana\relatório final.docx",
        ] {
            assert!(is_valid_path(good), "{good}");
        }
        for bad in [
            "",
            "C:",
            r"\\server\share\a.txt",
            r"a.txt",
            r"C:a.txt",
            r"C:\a\..\b",
            r"C:\a\.\b",
            r"C:\a:b",
            "C:\\a\nb",
            r"C:\a?.txt",
            r"C:/a.txt",
            "C:\\a\"b",
        ] {
            assert!(!is_valid_path(bad), "{bad:?}");
        }
        assert!(!is_valid_path(&format!(
            r"C:\{}",
            "a".repeat(MAX_PATH_CHARS)
        )));
    }

    #[test]
    fn the_key_tells_a_folder_by_its_last_backslash() {
        let doc = file(r"C:\Docs\a.txt");
        let docs = folder(r"C:\Docs");
        assert_eq!(file_key(&doc), r"file:C:\Docs\a.txt");
        assert_eq!(file_key(&docs), r"file:C:\Docs\");
        assert_eq!(hit_of_key(&file_key(&doc)), Some(doc));
        assert_eq!(hit_of_key(&file_key(&docs)), Some(docs));
        let drive = folder(r"C:\");
        assert_eq!(file_key(&drive), r"file:C:\");
        assert_eq!(hit_of_key(r"file:C:\"), Some(drive));
        for bad in [
            "app:x",
            "file:",
            "file:a.txt",
            r"file:\\s",
            r"file:\\.\pipe\x",
            "file:C:\\a\nb",
        ] {
            assert_eq!(hit_of_key(bad), None, "{bad:?}");
        }
        // A share typed in the launcher (L5) is kept too.
        let shared = file(r"\\srv\share\a.txt");
        assert_eq!(hit_of_key(&file_key(&shared)), Some(shared));
    }

    #[test]
    fn a_share_typed_is_opened_but_not_a_device() {
        for good in [r"\\srv\share\", r"\\srv\share\a b.txt", r"\\srv\share"] {
            assert!(is_valid_unc(good) && is_openable(good), "{good}");
        }
        for bad in [
            r"\\",
            r"\\srv",
            r"\\srv\",
            r"\\.\pipe\x",
            r"\\?\C:\x",
            r"\\srv\share\..\x",
            r"\\srv\share\a?.txt",
            r"\\srv\share\a:b",
            "\\\\srv\\share\\a\nb",
            r"C:\x",
        ] {
            assert!(!is_valid_unc(bad), "{bad:?}");
        }
        // The searches of files still take only the drives.
        assert!(!is_valid_path(r"\\srv\share\a.txt"));
    }

    #[test]
    fn a_file_shows_its_types_icon_and_a_program_its_own() {
        assert_eq!(icon_key(&file(r"C:\a\Relatório.PDF")), "type:.pdf");
        assert_eq!(icon_key(&file(r"C:\b\outro.pdf")), "type:.pdf");
        assert_eq!(
            icon_key(&file(r"C:\Tools\App.exe")),
            r"path:c:\tools\app.exe"
        );
        assert_eq!(icon_key(&file(r"C:\a\LEIAME")), r"path:c:\a\leiame");
        assert_eq!(icon_key(&folder(r"C:\a\b.old")), r"path:c:\a\b.old");
    }

    #[test]
    fn the_sources_are_joined_without_repeating_a_path() {
        let everything = vec![file(r"C:\Docs\Relatório.pdf"), folder(r"C:\Relatorios")];
        let windows = vec![
            file(r"c:\docs\relatório.PDF"),
            folder(r"C:\RELATORIOS\"),
            file(r"C:\Outro\relatorio.txt"),
            file("not a path"),
        ];
        let merged = merge([everything, windows]);
        assert_eq!(
            merged,
            [
                file(r"C:\Docs\Relatório.pdf"),
                folder(r"C:\Relatorios"),
                file(r"C:\Outro\relatorio.txt"),
            ]
        );
        // Without Everything, the Windows Search alone.
        assert_eq!(merge([Vec::new(), vec![file(r"C:\a.txt")]]).len(), 1);
    }

    #[test]
    fn the_band_comes_first_then_the_use_then_the_sources_order() {
        let hits = vec![
            file(r"C:\x\meu relatorio.txt"),
            file(r"C:\x\relatorio.txt"),
            file(r"C:\x\relatorios antigos.zip"),
            file(r"C:\x\outro.txt"),
        ];
        let words = strings(&["relatorio"]);
        let names = |hits: &[FileHit]| hits.iter().map(|h| h.name().to_owned()).collect::<Vec<_>>();
        let ranked = rank(hits.clone(), &words, &HashMap::new(), 10);
        assert_eq!(
            names(&ranked),
            [
                "relatorio.txt",
                "relatorios antigos.zip",
                "meu relatorio.txt",
                "outro.txt"
            ]
        );
        // Use orders within a band, never over a better one.
        let mut usage = HashMap::new();
        usage.insert(file_key(&hits[3]), 99.0);
        usage.insert(file_key(&hits[2]), 1.0);
        let ranked = rank(hits.clone(), &words, &usage, 10);
        assert_eq!(names(&ranked)[0], "relatorio.txt");
        assert_eq!(names(&ranked)[3], "outro.txt");
        let words = strings(&["rel"]);
        let ranked = rank(hits.clone(), &words, &usage, 10);
        assert_eq!(
            names(&ranked)[..2],
            ["relatorios antigos.zip", "relatorio.txt"]
        );
    }

    #[test]
    fn the_results_stop_at_the_limit() {
        let hits: Vec<FileHit> = (0..50)
            .map(|n| file(&format!(r"C:\r\rel {n}.txt")))
            .collect();
        let words = strings(&["rel"]);
        assert_eq!(
            rank(hits.clone(), &words, &HashMap::new(), MIXED_LIMIT).len(),
            MIXED_LIMIT
        );
        assert_eq!(
            rank(hits, &words, &HashMap::new(), ONLY_LIMIT).len(),
            ONLY_LIMIT
        );
    }

    #[test]
    fn the_rows_that_still_match_stay() {
        let hit = file(r"C:\x\Relatório Mensal.docx");
        assert!(still_matches(&hit, &strings(&["relat"])));
        assert!(still_matches(&hit, &strings(&["relatorio", "mens"])));
        assert!(!still_matches(&hit, &strings(&["relatorio", "anual"])));
    }

    #[test]
    fn the_hits_go_through_the_helper_whole() {
        let hits = vec![folder(r"C:\Docs"), file(r"C:\Docs\a b.txt")];
        let text = encode_hits(&hits);
        assert_eq!(text, "d\tC:\\Docs\nf\tC:\\Docs\\a b.txt\n");
        assert_eq!(decode_hits(&text), hits);
        assert_eq!(
            decode_hits("x\tC:\\a\nf\tnot a path\nf\nf\tC:\\ok.txt"),
            [file(r"C:\ok.txt")]
        );
    }

    #[test]
    fn an_answer_of_an_older_typing_is_dropped() {
        let mut generation = Generation::default();
        let first = generation.next();
        assert!(generation.is_current(first));
        let second = generation.next();
        assert!(!generation.is_current(first), "the late answer is dropped");
        assert!(generation.is_current(second));
    }

    #[test]
    fn an_answer_of_the_index_in_time_goes_in_the_first_answer() {
        let wait = IndexWait::new(4, vec![file(r"C:\a.txt")]);
        assert_eq!(
            wait.arrival(4, vec![file(r"C:\b.txt"), file(r"C:\A.TXT")], 120),
            Arrival::InTime(vec![file(r"C:\a.txt"), file(r"C:\b.txt")]),
            "joined, Everything's first, each path once"
        );
        assert_eq!(
            wait.arrival(4, Vec::new(), TIMEOUT_MS),
            Arrival::InTime(vec![file(r"C:\a.txt")])
        );
    }

    #[test]
    fn a_late_answer_of_the_same_search_comes_after() {
        // The first search after opening: the index cold, 900 ms.
        let wait = IndexWait::new(1, Vec::new());
        assert_eq!(wait.early(), []);
        assert_eq!(
            wait.arrival(1, vec![file(r"C:\relatorio.pdf")], 900),
            Arrival::Late(vec![file(r"C:\relatorio.pdf")])
        );
        assert_eq!(
            wait.arrival(1, vec![file(r"C:\relatorio.pdf")], LATE_LIMIT_MS),
            Arrival::Late(vec![file(r"C:\relatorio.pdf")])
        );
        // Too late: no longer waited for.
        assert!(wait.expired(LATE_LIMIT_MS + 1));
        assert_eq!(
            wait.arrival(1, vec![file(r"C:\relatorio.pdf")], LATE_LIMIT_MS + 1),
            Arrival::Dropped
        );
    }

    #[test]
    fn a_late_answer_of_an_older_search_is_dropped() {
        let wait = IndexWait::new(3, vec![file(r"C:\a.txt")]);
        assert_eq!(
            wait.arrival(2, vec![file(r"C:\b.txt")], 900),
            Arrival::Dropped
        );
        assert_eq!(
            wait.arrival(2, vec![file(r"C:\b.txt")], 50),
            Arrival::Dropped
        );
    }

    #[test]
    fn the_launcher_shows_a_late_answer_of_the_typing_still_there() {
        let mut generation = Generation::default();
        let asked = generation.next();
        // The first answer (not whole) and the late one carry the same
        // generation: both are shown while nothing more was typed.
        assert!(generation.is_current(asked), "in time");
        assert!(generation.is_current(asked), "late, same typing: shown");
        generation.next();
        assert!(!generation.is_current(asked), "late, older typing: dropped");
    }

    #[test]
    fn an_answer_says_whether_more_may_come() {
        let hits = vec![file(r"C:\a.txt")];
        assert_eq!(
            decode_answer(&encode_answer(&hits, true)),
            (hits.clone(), true)
        );
        assert_eq!(
            decode_answer(&encode_answer(&hits, false)),
            (hits.clone(), false)
        );
        assert_eq!(
            decode_answer(&encode_answer(&[], false)),
            (Vec::new(), false)
        );
        assert_eq!(decode_answer(""), (Vec::new(), true));
        // The mark is not a hit for an older reader of the hits.
        assert_eq!(decode_hits(&encode_answer(&hits, false)), hits);
    }

    #[test]
    fn a_late_answer_goes_through_the_helper() {
        let hits = vec![folder(r"C:\Docs")];
        for answer in [
            LateAnswer::Gone,
            LateAnswer::Waiting,
            LateAnswer::Came(hits.clone()),
        ] {
            assert_eq!(decode_late(&encode_late(&answer)), answer);
        }
        // Empty, it changes nothing: as gone.
        assert_eq!(
            decode_late(&encode_late(&LateAnswer::Came(Vec::new()))),
            LateAnswer::Gone
        );
    }
}
