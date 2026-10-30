//! The launcher's search (spec 0012 §5): which entries match what the user
//! typed, and in what order. Pure: no Windows, no clock, no database.
//!
//! A name matches in one of six bands, best first; within a band, the apps
//! come before the settings pages, then what the user opened more often (and
//! more lately) through the launcher, then the shorter name, then the
//! alphabet. Use never lifts an entry over a better band: typing an app's
//! exact name never loses to an app used a lot.
//!
//! What a result does is one of a closed list of actions (`Action`), each
//! checked by `is_allowed` before it runs: nothing found by the launcher runs
//! anything else.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::apps::is_valid_app_id;
use super::launcher_pages::{self, Section};

/// How many results the launcher shows.
pub const MAX_RESULTS: usize = 8;

/// How long it takes an opening to weigh half as much, in milliseconds
/// (14 days).
pub const HALF_LIFE_MS: i64 = 14 * 24 * 60 * 60 * 1000;

/// The longest query looked at; the rest is ignored.
pub const MAX_QUERY_CHARS: usize = 64;

/// Most name characters between two letters of the query in the last band.
const MAX_GAP: usize = 3;

/// The key under which the use of the app `id` is kept.
pub fn app_key(id: &str) -> String {
    format!("app:{id}")
}

/// How a name matches the query, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    /// The name is the query.
    Exact,
    /// The name starts with the query.
    Prefix,
    /// A word of the name starts with the query (`code` finds "Visual
    /// Studio Code").
    Word,
    /// The initials of the words start with the query (`vsc`).
    Initials,
    /// The query is inside the name, or the program's file name starts with
    /// it (`msedge` finds "Microsoft Edge").
    Inside,
    /// The query's letters are in the name in order, at most `MAX_GAP`
    /// apart (`ntpd` finds "Notepad++").
    Scattered,
}

/// Something the launcher can find.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Its key: what its use is kept under (`app_key`).
    pub key: String,
    /// The name shown.
    pub name: String,
    /// The program's file name without its extension, for the classic
    /// programs (whose id is a path); `None` for the others.
    pub file: Option<String>,
    /// Other words that find it, normalized (a settings page's).
    pub aliases: Vec<&'static str>,
    /// Its kind's place within a band: the apps (0) before the settings
    /// pages (1).
    pub rank: u8,
}

impl Candidate {
    /// A program of the Apps folder.
    pub fn app(id: &str, name: &str) -> Candidate {
        Candidate {
            key: app_key(id),
            name: name.to_owned(),
            file: file_of(id),
            aliases: Vec::new(),
            rank: 0,
        }
    }

    /// A settings page of the fixed list.
    pub fn page(page: &launcher_pages::Page) -> Candidate {
        Candidate {
            key: launcher_pages::page_key(page.id),
            name: page.name.to_owned(),
            file: None,
            aliases: page.aliases.to_vec(),
            rank: 1,
        }
    }
}

/// Lowercase, without accents, the runs of spaces made one, and no spaces
/// at the ends: `"  Configurações  do  Windows"` is
/// `"configuracoes do windows"`.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(fold(c));
    }
    out
}

/// A lowercase letter without its accent, for the accented letters of
/// Portuguese and of the other Latin languages names show up in.
fn fold(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' => 'i',
        'ł' => 'l',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
        'ř' => 'r',
        'ś' | 'š' | 'ş' => 's',
        'ť' | 'ţ' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        other => other,
    }
}

/// The words of a normalized name: split at anything that is not a letter
/// or a digit.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// The band `name` matches `query` in (both normalized), with the program's
/// file name `file` (normalized) when there is one; `None` when it does not
/// match. An empty query matches nothing.
pub fn band(query: &str, name: &str, file: Option<&str>) -> Option<Band> {
    if query.is_empty() {
        return None;
    }
    if name == query {
        return Some(Band::Exact);
    }
    if name.starts_with(query) {
        return Some(Band::Prefix);
    }
    if words(name).any(|word| word.starts_with(query)) {
        return Some(Band::Word);
    }
    let long = query.chars().count() >= 2;
    if long {
        let initials: String = words(name).filter_map(|word| word.chars().next()).collect();
        if initials.starts_with(query) {
            return Some(Band::Initials);
        }
    }
    if long && (name.contains(query) || file.is_some_and(|file| file.starts_with(query))) {
        return Some(Band::Inside);
    }
    if long && scattered(query, name) {
        return Some(Band::Scattered);
    }
    None
}

/// Whether the letters of `query` are in `name` in order, each at most
/// `MAX_GAP` characters after the one before (the first anywhere). Spaces
/// in the query are not looked for.
fn scattered(query: &str, name: &str) -> bool {
    let wanted: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).collect();
    let name: Vec<char> = name.chars().collect();
    if wanted.is_empty() || wanted.len() > name.len() {
        return false;
    }
    // `ends[j]`: the query so far can end at `name[j]`.
    let mut ends: Vec<bool> = name.iter().map(|c| *c == wanted[0]).collect();
    for letter in &wanted[1..] {
        let mut next = vec![false; name.len()];
        for (j, reached) in ends.iter().enumerate() {
            if !reached {
                continue;
            }
            for k in j + 1..name.len().min(j + 2 + MAX_GAP) {
                if name[k] == *letter {
                    next[k] = true;
                }
            }
        }
        if !next.contains(&true) {
            return false;
        }
        ends = next;
    }
    true
}

/// The use of `score` kept at `from_ms`, as it weighs at `now_ms`: half each
/// `HALF_LIFE_MS`. A time in the past of `from_ms` (a clock set back) weighs
/// it whole.
pub fn decay(score: f64, from_ms: i64, now_ms: i64) -> f64 {
    let elapsed = now_ms.saturating_sub(from_ms).max(0) as f64;
    score * 0.5_f64.powf(elapsed / HALF_LIFE_MS as f64)
}

/// What the use of an entry weighs at `now_ms` after one more opening.
pub fn opened(score: f64, from_ms: i64, now_ms: i64) -> f64 {
    decay(score, from_ms, now_ms) + 1.0
}

/// The candidates that match `query`, best first, at most `limit`, as
/// indices into `candidates`. `usage` is each key's weight now (`decay`).
pub fn search(
    candidates: &[Candidate],
    usage: &HashMap<String, f64>,
    query: &str,
    limit: usize,
) -> Vec<usize> {
    let query: String = normalize(query).chars().take(MAX_QUERY_CHARS).collect();
    let mut found: Vec<(Band, f64, usize, String, usize)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let name = normalize(&candidate.name);
            let file = candidate.file.as_deref().map(normalize);
            let band = std::iter::once(band(&query, &name, file.as_deref()))
                .chain(
                    candidate
                        .aliases
                        .iter()
                        .map(|alias| band(&query, alias, None)),
                )
                .flatten()
                .min()?;
            let used = usage.get(&candidate.key).copied().unwrap_or(0.0);
            Some((band, used, name.chars().count(), name, index))
        })
        .collect();
    found.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| candidates[a.4].rank.cmp(&candidates[b.4].rank))
            .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
            .then_with(|| candidates[a.4].key.cmp(&candidates[b.4].key))
    });
    found
        .into_iter()
        .take(limit)
        .map(|(.., index)| index)
        .collect()
}

/// What the launcher shows before anything is typed: the candidates used
/// most (by `usage`, each key's weight now), at most `limit`, as indices
/// into `candidates`. Only those used at all.
pub fn recents(candidates: &[Candidate], usage: &HashMap<String, f64>, limit: usize) -> Vec<usize> {
    let mut used: Vec<(f64, usize)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            usage
                .get(&candidate.key)
                .copied()
                .filter(|weight| *weight > 0.0)
                .map(|weight| (weight, index))
        })
        .collect();
    used.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| candidates[a.1].name.cmp(&candidates[b.1].name))
    });
    used.into_iter()
        .take(limit)
        .map(|(_, index)| index)
        .collect()
}

/// The file name of a classic program's id (a path to an `.exe` or a
/// `.lnk`) without its extension; `None` for any other id (a Store app's).
pub fn file_of(id: &str) -> Option<String> {
    let looks_like_path = id.len() > 3 && id.as_bytes()[1] == b':' && id.contains('\\');
    if !looks_like_path {
        return None;
    }
    let name = id.rsplit('\\').next()?;
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    (!stem.is_empty()).then(|| stem.to_owned())
}

/// The longest text a result copies (a calculator's result).
pub const MAX_COPY: usize = 64;
/// The longest address a result opens.
pub const MAX_URI: usize = 2048;

/// What a result does: the closed list of spec 0012 §5. A new kind of
/// action is added here, and to `is_allowed`, or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Opens a program of the Apps folder, by its id.
    OpenApp(String),
    /// Opens a section of the Dockyard's Configurações.
    OpenSection(Section),
    /// Opens an address: a Windows settings page of the fixed list, or an
    /// `https://` search of a keyword.
    OpenUri(String),
    /// Copies a text (a calculator's result).
    Copy(String),
    /// Opens a file with the program Windows has for it, or a folder in
    /// File Explorer (L4): a full path on a drive, found by the search of
    /// files.
    OpenFile(String),
}

/// Whether `action` may run: every result's action is checked here first.
pub fn is_allowed(action: &Action) -> bool {
    match action {
        Action::OpenApp(id) => is_valid_app_id(id),
        Action::OpenSection(_) => true,
        Action::OpenUri(uri) => launcher_pages::is_listed_uri(uri) || is_web_uri(uri),
        Action::Copy(text) => {
            !text.is_empty()
                && text.chars().count() <= MAX_COPY
                && !text.chars().any(char::is_control)
        }
        Action::OpenFile(path) => super::launcher_files::is_valid_path(path),
    }
}

/// An `https://` address with a host, without spaces or control characters:
/// what a keyword's search makes (`launcher_web::url`).
fn is_web_uri(uri: &str) -> bool {
    let Some(rest) = uri.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    uri.len() <= MAX_URI
        && !host.is_empty()
        && !uri.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str) -> Candidate {
        Candidate {
            key: app_key(name),
            name: name.to_owned(),
            file: None,
            aliases: Vec::new(),
            rank: 0,
        }
    }

    /// Names from the author's Apps folder.
    fn catalog() -> Vec<Candidate> {
        let mut apps: Vec<Candidate> = [
            "Bloco de Notas",
            "Calculadora",
            "Configurações",
            "Conexão de Área de Trabalho Remota",
            "Gerenciador de Tarefas",
            "Google Chrome",
            "Notepad++",
            "Paint",
            "Prompt de Comando",
            "Visual Studio Code",
            "Visual Studio Installer",
            "Windows PowerShell",
        ]
        .into_iter()
        .map(app)
        .collect();
        apps.push(Candidate::app(
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            "Microsoft Edge",
        ));
        apps
    }

    fn names(candidates: &[Candidate], found: &[usize]) -> Vec<String> {
        found.iter().map(|&i| candidates[i].name.clone()).collect()
    }

    fn find(query: &str) -> Vec<String> {
        let apps = catalog();
        names(&apps, &search(&apps, &HashMap::new(), query, MAX_RESULTS))
    }

    #[test]
    fn normalizing_drops_case_accents_and_extra_spaces() {
        assert_eq!(
            normalize("  Configurações   do  Windows "),
            "configuracoes do windows"
        );
        assert_eq!(normalize("ÁREA de Trabalho"), "area de trabalho");
        assert_eq!(normalize("Notepad++"), "notepad++");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn each_band_is_found() {
        let band_of = |query: &str, name: &str| band(&normalize(query), &normalize(name), None);
        assert_eq!(band_of("paint", "Paint"), Some(Band::Exact));
        assert_eq!(band_of("bloco", "Bloco de Notas"), Some(Band::Prefix));
        assert_eq!(band_of("code", "Visual Studio Code"), Some(Band::Word));
        assert_eq!(band_of("vsc", "Visual Studio Code"), Some(Band::Initials));
        assert_eq!(
            band_of("gdt", "Gerenciador de Tarefas"),
            Some(Band::Initials)
        );
        assert_eq!(band_of("tudi", "Visual Studio Code"), Some(Band::Inside));
        assert_eq!(band_of("ntpd", "Notepad++"), Some(Band::Scattered));
        assert_eq!(band_of("xyz", "Notepad++"), None);
        assert_eq!(band_of("", "Paint"), None);
    }

    #[test]
    fn the_program_file_finds_a_classic_program() {
        let file = file_of(r"C:\Program Files\Microsoft\msedge.exe");
        assert_eq!(file.as_deref(), Some("msedge"));
        assert_eq!(
            band("msedge", "microsoft edge", file.as_deref()),
            Some(Band::Inside)
        );
        assert_eq!(find("msedge"), ["Microsoft Edge"]);
    }

    #[test]
    fn a_store_app_has_no_file() {
        assert_eq!(
            file_of("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            None
        );
        assert_eq!(
            file_of(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\cmd.exe"),
            None
        );
    }

    #[test]
    fn one_letter_finds_only_by_the_start_of_a_word() {
        // No initials, no inside, no scattered letters with one letter.
        assert_eq!(
            find("n"),
            ["Notepad++", "Bloco de Notas"],
            "the name's start first, then a word's"
        );
    }

    #[test]
    fn the_scattered_letters_keep_to_the_gap() {
        assert!(scattered("ntpd", "notepad++"));
        // A later start finds what an earlier one can't.
        assert!(scattered("ab", "axxxxab"));
        // A later middle letter reaches what an earlier one can't.
        assert!(scattered("abc", "abxbxxxc"));
        assert!(!scattered("ac", "axxxxc"));
        assert!(!scattered("abc", "ab"));
    }

    #[test]
    fn the_spec_examples_come_first() {
        assert_eq!(find("bloco")[0], "Bloco de Notas");
        assert_eq!(find("vsc")[0], "Visual Studio Code");
        assert_eq!(find("configuracoes")[0], "Configurações");
        assert_eq!(find("area")[0], "Conexão de Área de Trabalho Remota");
        assert_eq!(find("ntpd")[0], "Notepad++");
    }

    #[test]
    fn a_better_band_beats_use() {
        let apps = catalog();
        let mut usage = HashMap::new();
        // "Visual Studio Installer" used a lot: still after the exact name.
        usage.insert(app_key("Visual Studio Installer"), 50.0);
        let found = search(&apps, &usage, "visual studio code", MAX_RESULTS);
        assert_eq!(names(&apps, &found)[0], "Visual Studio Code");
    }

    #[test]
    fn within_a_band_use_comes_first_then_the_shorter_name_then_the_alphabet() {
        let apps = catalog();
        let mut usage = HashMap::new();
        let none = search(&apps, &usage, "visual", MAX_RESULTS);
        // Same band, same length: the alphabet.
        assert_eq!(
            names(&apps, &none),
            ["Visual Studio Code", "Visual Studio Installer"]
        );
        usage.insert(app_key("Visual Studio Installer"), 1.0);
        let used = search(&apps, &usage, "visual", MAX_RESULTS);
        assert_eq!(
            names(&apps, &used),
            ["Visual Studio Installer", "Visual Studio Code"]
        );
        // The shorter name before the longer, with no use.
        let shorter = search(&apps, &HashMap::new(), "p", MAX_RESULTS);
        assert_eq!(names(&apps, &shorter)[..2], ["Paint", "Prompt de Comando"]);
    }

    #[test]
    fn the_results_stop_at_the_limit() {
        let apps = catalog();
        assert_eq!(search(&apps, &HashMap::new(), "c", 2).len(), 2);
    }

    #[test]
    fn a_long_query_is_cut() {
        let long = "a".repeat(MAX_QUERY_CHARS + 10);
        let apps = vec![app(&"a".repeat(MAX_QUERY_CHARS))];
        assert_eq!(search(&apps, &HashMap::new(), &long, MAX_RESULTS), [0]);
    }

    #[test]
    fn use_weighs_half_every_fourteen_days() {
        assert_eq!(decay(4.0, 0, 0), 4.0);
        assert!((decay(4.0, 0, HALF_LIFE_MS) - 2.0).abs() < 1e-9);
        assert!((decay(4.0, 0, 2 * HALF_LIFE_MS) - 1.0).abs() < 1e-9);
        // A clock set back weighs the use whole.
        assert_eq!(decay(4.0, 10, 0), 4.0);
        assert!((opened(2.0, 0, HALF_LIFE_MS) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_recents_are_the_most_used_ones_only() {
        let apps = catalog();
        let mut usage = HashMap::new();
        assert!(recents(&apps, &usage, MAX_RESULTS).is_empty());
        usage.insert(app_key("Paint"), 1.0);
        usage.insert(app_key("Calculadora"), 3.0);
        usage.insert(app_key("Gone"), 9.0);
        usage.insert(app_key("Notepad++"), 0.0);
        assert_eq!(
            names(&apps, &recents(&apps, &usage, MAX_RESULTS)),
            ["Calculadora", "Paint"]
        );
        assert_eq!(recents(&apps, &usage, 1).len(), 1);
    }

    fn with_pages() -> Vec<Candidate> {
        let mut all = catalog();
        all.extend(launcher_pages::PAGES.iter().map(Candidate::page));
        all
    }

    fn find_all(query: &str) -> Vec<String> {
        let all = with_pages();
        names(&all, &search(&all, &HashMap::new(), query, MAX_RESULTS))
    }

    #[test]
    fn a_page_is_found_by_its_other_words() {
        for query in ["wifi", "internet", "rede sem fio", "wi-fi"] {
            assert!(find_all(query).contains(&"Wi-Fi".to_owned()), "{query}");
        }
        assert_eq!(find_all("papel de parede")[0], "Plano de fundo");
        assert_eq!(find_all("bateria")[0], "Energia e bateria");
    }

    #[test]
    fn in_the_same_band_the_apps_come_before_the_pages() {
        // The app "Configurações" is the exact name; the page "Configurações
        // do Dockyard" only starts with it.
        assert_eq!(
            find_all("configuracoes")[..2],
            ["Configurações", "Configurações do Dockyard"]
        );
        // "Paint" (an app) and "Plano de fundo" (a page) both start with p:
        // the app first, even with the page used a lot.
        let all = with_pages();
        let mut usage = HashMap::new();
        usage.insert(launcher_pages::page_key("background"), 99.0);
        let found = names(&all, &search(&all, &usage, "p", 50));
        let paint = found.iter().position(|name| name == "Paint").unwrap();
        let page = found
            .iter()
            .position(|name| name == "Plano de fundo")
            .unwrap();
        assert!(paint < page, "{found:?}");
    }

    #[test]
    fn every_action_is_checked() {
        assert!(is_allowed(&Action::OpenApp(
            "Microsoft.Paint_8wekyb3d8bbwe!App".into()
        )));
        assert!(!is_allowed(&Action::OpenApp(String::new())));
        assert!(is_allowed(&Action::OpenSection(Section::Look)));
        assert!(is_allowed(&Action::OpenUri(
            "ms-settings:network-wifi".into()
        )));
        assert!(is_allowed(&Action::OpenUri(
            "https://www.google.com/search?q=dock%20max".into()
        )));
        for uri in [
            "ms-settings:developers",
            "http://example.com/",
            "https://",
            "https://example.com/a b",
            "file:///C:/Windows/System32/cmd.exe",
            "shell:AppsFolder",
            r"C:\Windows\System32\cmd.exe",
        ] {
            assert!(!is_allowed(&Action::OpenUri(uri.into())), "{uri}");
        }
        let long = format!("https://example.com/?q={}", "a".repeat(MAX_URI));
        assert!(!is_allowed(&Action::OpenUri(long)));
        assert!(is_allowed(&Action::Copy("3,5".into())));
        assert!(!is_allowed(&Action::Copy(String::new())));
        assert!(!is_allowed(&Action::Copy("a\nb".into())));
        assert!(!is_allowed(&Action::Copy("1".repeat(MAX_COPY + 1))));
        assert!(is_allowed(&Action::OpenFile(r"C:\Users\ana\a.pdf".into())));
        for path in [
            "",
            r"\\server\share\a.pdf",
            "a.pdf",
            r"C:\a\..\b",
            "C:\\a\nb",
        ] {
            assert!(!is_allowed(&Action::OpenFile(path.into())), "{path:?}");
        }
    }
}
