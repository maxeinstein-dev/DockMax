//! The launcher's clipboard (spec 0012, "Área de transferência"): the texts of
//! Windows' own clipboard history (Win+V), found with the keyword `cb ` only.
//! Pure: no Windows, no clock; the history is read in
//! `windows/launcher/clipboard.rs`.
//!
//! The Dockyard keeps nothing of it: no file, no cache between openings. What
//! was read lives while the launcher shows `cb `, and goes when it closes or
//! the query leaves the keyword.
//!
//! **Privacy:** a text of the history is a `ClipText`, which has no `Display`
//! and whose `Debug` writes only its length: no `log::` line can print it by
//! mistake (`format!("{}", text)` does not compile, `{:?}` writes
//! `ClipText(12 chars)`).

use super::launcher::{Action, MAX_QUERY_CHARS, is_allowed, normalize};
use super::launcher_files::Generation;

/// The keyword that shows the clipboard's history (`cb`, of "clipboard").
pub const KEYWORD: &str = "cb";
/// Most items listed: as many as Windows keeps in the history.
pub const LIMIT: usize = 25;
/// The longest first line shown, in characters.
pub const LINE_CHARS: usize = 80;
/// What is kept of each text to filter by, in characters.
pub const MAX_TEXT_CHARS: usize = 4096;
/// How long the history has to answer, in milliseconds.
pub const TIMEOUT_MS: u32 = 1500;
/// The longest id of a history item (Windows' are GUIDs).
pub const MAX_ID_CHARS: usize = 128;
/// The page of Windows' Configurações that turns the history on.
pub const SETTINGS_URI: &str = "ms-settings:clipboard";

/// A text of the clipboard's history. No `Display`, and a `Debug` that
/// writes only its length: what the user copied never reaches the log.
#[derive(Clone, PartialEq, Eq)]
pub struct ClipText(String);

impl ClipText {
    /// At most `MAX_TEXT_CHARS` of `text`.
    pub fn new(text: &str) -> ClipText {
        ClipText(text.chars().take(MAX_TEXT_CHARS).collect())
    }

    /// The text itself: only to show it and to filter it, never to log it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ClipText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClipText({} chars)", self.0.chars().count())
    }
}

/// A text item of the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    /// Windows' id of the item, what Enter puts back by.
    pub id: String,
    pub text: ClipText,
    /// When it was copied, in Unix milliseconds.
    pub at_ms: i64,
}

/// A query of the history: what the texts must have (empty: every one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipQuery {
    pub term: String,
}

/// What `query` (as typed) asks of the history: only `cb ` and what follows,
/// unless the user made `cb` a web keyword (`keyword_taken`, which then
/// wins). `cb` alone, and any other query, ask nothing: the history never
/// shows up mixed with the apps.
pub fn parse(query: &str, keyword_taken: bool) -> Option<ClipQuery> {
    if keyword_taken {
        return None;
    }
    let (word, term) = query.trim_start().split_once(' ')?;
    word.eq_ignore_ascii_case(KEYWORD).then(|| ClipQuery {
        term: term.trim().to_owned(),
    })
}

/// The first line of `text` with something in it, its spaces and tabs made
/// one space, cut at `LINE_CHARS` characters (the last one an ellipsis when
/// cut). Empty when the text is only blanks.
pub fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let mut flat = String::with_capacity(line.len().min(LINE_CHARS * 4));
    let mut space = false;
    for c in line.chars() {
        if c.is_whitespace() || c.is_control() {
            space = !flat.is_empty();
            continue;
        }
        if space {
            flat.push(' ');
            space = false;
        }
        flat.push(c);
    }
    if flat.chars().count() <= LINE_CHARS {
        return flat;
    }
    let mut cut: String = flat.chars().take(LINE_CHARS - 1).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

/// How long ago something was copied, as the row says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ago {
    /// Less than a minute (or a clock set back).
    Now,
    Minutes(i64),
    Hours(i64),
    Days(i64),
}

/// How long before `now_ms` the time `at_ms` was.
pub fn ago(at_ms: i64, now_ms: i64) -> Ago {
    const MINUTE: i64 = 60 * 1000;
    let elapsed = now_ms.saturating_sub(at_ms).max(0);
    match elapsed {
        e if e < MINUTE => Ago::Now,
        e if e < 60 * MINUTE => Ago::Minutes(e / MINUTE),
        e if e < 24 * 60 * MINUTE => Ago::Hours(e / (60 * MINUTE)),
        e => Ago::Days(e / (24 * 60 * MINUTE)),
    }
}

/// A WinRT `DateTime` (100 ns ticks since 1601-01-01, UTC) in Unix
/// milliseconds.
pub fn unix_ms_of_ticks(ticks: i64) -> i64 {
    const FROM_1601_TO_1970_MS: i64 = 11_644_473_600_000;
    ticks / 10_000 - FROM_1601_TO_1970_MS
}

/// The items `term` finds, as indices into `clips`, the last copied first, at
/// most `LIMIT`: every word of the term (normalized: no case, no accents)
/// somewhere in the text. With no term, the last copied.
pub fn rank(clips: &[Clip], term: &str) -> Vec<usize> {
    let term: String = normalize(term).chars().take(MAX_QUERY_CHARS).collect();
    let words: Vec<&str> = term.split(' ').filter(|word| !word.is_empty()).collect();
    let mut found: Vec<usize> = (0..clips.len())
        .filter(|&index| {
            words.is_empty() || {
                let text = normalize(clips[index].text.expose());
                words.iter().all(|word| text.contains(word))
            }
        })
        .collect();
    // Stable: Windows' own order among the same time.
    found.sort_by_key(|&index| std::cmp::Reverse(clips[index].at_ms));
    found.truncate(LIMIT);
    found
}

/// The address `text` is, when it is one `https://` link and nothing else
/// (spaces around it aside), one the launcher opens: "Abrir como link".
pub fn link(text: &str) -> Option<String> {
    let text = text.trim();
    let starts = text
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    if !starts {
        return None;
    }
    let uri = format!("https://{}", &text[8..]);
    is_allowed(&Action::OpenUri(uri.clone())).then_some(uri)
}

/// Whether `id` can be a history item's id: what `Action::CopyClip` holds.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= MAX_ID_CHARS && !id.chars().any(char::is_control)
}

/// What reading the history answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Its text items, in Windows' order.
    Ready(Vec<Clip>),
    /// The history is off in Windows' Configurações.
    Disabled,
    /// Windows refused (another program has the foreground) or failed.
    Failed,
}

/// Where the reading of the history is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    Idle,
    Waiting(u64),
    Done(Answer),
    /// It did not answer in `TIMEOUT_MS`.
    Late,
}

/// The history as the launcher has it now: read once each time `cb ` is
/// typed, dropped when the launcher closes or the query leaves `cb `.
#[derive(Debug)]
pub struct History {
    generation: Generation,
    pub reading: Reading,
}

impl Default for History {
    fn default() -> History {
        History {
            generation: Generation::default(),
            reading: Reading::Idle,
        }
    }
}

impl History {
    /// `cb ` is typed: the generation of a reading to start, or `None` when
    /// one was started already.
    pub fn want(&mut self) -> Option<u64> {
        if self.reading != Reading::Idle {
            return None;
        }
        let generation = self.generation.next();
        self.reading = Reading::Waiting(generation);
        Some(generation)
    }

    /// The reading `generation` answered; whether it was taken (it is the
    /// last one, still waited for).
    pub fn answered(&mut self, generation: u64, answer: Answer) -> bool {
        if self.reading != Reading::Waiting(generation) || !self.generation.is_current(generation) {
            return false;
        }
        self.reading = Reading::Done(answer);
        true
    }

    /// The reading `generation` ran out of time; whether that changed what is
    /// shown. Its answer, if it ever comes, is dropped.
    pub fn timed_out(&mut self, generation: u64) -> bool {
        if self.reading != Reading::Waiting(generation) {
            return false;
        }
        self.generation.next();
        self.reading = Reading::Late;
        true
    }

    /// The texts read, if any.
    pub fn clips(&self) -> &[Clip] {
        match &self.reading {
            Reading::Done(Answer::Ready(clips)) => clips,
            _ => &[],
        }
    }

    /// Forgets what was read (the launcher closed, the query left `cb `): a
    /// reading still running answers nothing.
    pub fn clear(&mut self) {
        self.generation.next();
        self.reading = Reading::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: &str, text: &str, at_ms: i64) -> Clip {
        Clip {
            id: id.to_owned(),
            text: ClipText::new(text),
            at_ms,
        }
    }

    fn history() -> Vec<Clip> {
        vec![
            clip("a", "Relatório de vendas\nsegunda linha", 1_000),
            clip("b", "https://github.com/maxeinstein-dev/Dockyard", 3_000),
            clip("c", "senha do wifi: abacaxi", 2_000),
        ]
    }

    #[test]
    fn only_cb_and_a_space_asks_for_the_history() {
        assert_eq!(parse("cb ", false), Some(ClipQuery { term: "".into() }));
        assert_eq!(
            parse("  CB  vendas ", false),
            Some(ClipQuery {
                term: "vendas".into()
            })
        );
        // Typing like any other: never mixed with the apps.
        for query in ["cb", "cbx", "c b", "vendas", "", "f cb", "j cb"] {
            assert_eq!(parse(query, false), None, "{query:?}");
        }
        // The user's web keyword `cb` wins.
        assert_eq!(parse("cb vendas", true), None);
    }

    #[test]
    fn cb_does_not_collide_with_the_other_keywords() {
        let defaults = crate::domain::launcher_web::defaults();
        assert!(
            !defaults
                .iter()
                .any(|keyword| keyword.keyword.eq_ignore_ascii_case(KEYWORD))
        );
        for other in [
            crate::domain::launcher_files::KEYWORD,
            crate::domain::launcher_windows::KEYWORD,
        ] {
            assert!(!KEYWORD.eq_ignore_ascii_case(other), "{other}");
        }
        // `f cb` asks for files, `j cb` for windows: not the history.
        assert!(crate::domain::launcher_files::parse("f cb", false).is_some());
        assert_eq!(parse("f cb", false), None);
    }

    #[test]
    fn with_no_term_the_last_copied_come_first() {
        let clips = history();
        assert_eq!(rank(&clips, ""), [1, 2, 0]);
        let many: Vec<Clip> = (0..40).map(|i| clip("x", "texto", i)).collect();
        assert_eq!(rank(&many, "").len(), LIMIT);
        assert_eq!(rank(&many, "")[0], 39);
    }

    #[test]
    fn a_term_filters_by_the_text_without_case_nor_accents() {
        let clips = history();
        assert_eq!(rank(&clips, "relatorio"), [0]);
        assert_eq!(rank(&clips, "VENDAS segunda"), [0]);
        // Every word, anywhere.
        assert_eq!(rank(&clips, "wifi abacaxi"), [2]);
        assert_eq!(rank(&clips, "wifi vendas"), Vec::<usize>::new());
        assert_eq!(rank(&clips, "dockyard"), [1]);
        assert_eq!(rank(&clips, "zqxj"), Vec::<usize>::new());
    }

    #[test]
    fn the_row_shows_the_first_line_cut() {
        assert_eq!(first_line("Relatório\nsegunda"), "Relatório");
        assert_eq!(first_line("\n\n   \n  meio  \t texto \nfim"), "meio texto");
        assert_eq!(first_line("   \n\t"), "");
        assert_eq!(first_line("a\r\nb"), "a");
        let long = "x".repeat(200);
        let cut = first_line(&long);
        assert_eq!(cut.chars().count(), LINE_CHARS);
        assert!(cut.ends_with('…'));
        // Exactly the limit: not cut.
        assert_eq!(first_line(&"y".repeat(LINE_CHARS)), "y".repeat(LINE_CHARS));
        // Cut in the middle of accents: by characters, not bytes.
        let accents = "ção ".repeat(40);
        assert!(first_line(&accents).chars().count() <= LINE_CHARS);
    }

    #[test]
    fn how_long_ago() {
        let minute = 60_000;
        assert_eq!(ago(0, 59_999), Ago::Now);
        assert_eq!(ago(0, 5 * minute + 10), Ago::Minutes(5));
        assert_eq!(ago(0, 59 * minute), Ago::Minutes(59));
        assert_eq!(ago(0, 60 * minute), Ago::Hours(1));
        assert_eq!(ago(0, 23 * 60 * minute), Ago::Hours(23));
        assert_eq!(ago(0, 24 * 60 * minute), Ago::Days(1));
        assert_eq!(ago(0, 3 * 24 * 60 * minute + 5), Ago::Days(3));
        // A clock set back: now.
        assert_eq!(ago(10 * minute, 0), Ago::Now);
    }

    #[test]
    fn winrt_ticks_become_unix_milliseconds() {
        // 1970-01-01 in ticks since 1601.
        assert_eq!(unix_ms_of_ticks(116_444_736_000_000_000), 0);
        // 2026-10-30T00:00:00Z.
        assert_eq!(
            unix_ms_of_ticks(116_444_736_000_000_000 + 1_793_318_400_000 * 10_000),
            1_793_318_400_000
        );
    }

    #[test]
    fn only_a_lone_https_link_opens_as_a_link() {
        assert_eq!(
            link("  https://github.com/a?b=1 \n").as_deref(),
            Some("https://github.com/a?b=1")
        );
        assert_eq!(
            link("HTTPS://example.com").as_deref(),
            Some("https://example.com")
        );
        for text in [
            "http://example.com",
            "https://",
            "veja https://example.com",
            "https://example.com e mais",
            "https://a.com\nhttps://b.com",
            "ftp://example.com",
            "javascript:alert(1)",
            "file:///C:/Windows/System32/cmd.exe",
            "",
        ] {
            assert_eq!(link(text), None, "{text:?}");
        }
        let long = format!("https://example.com/{}", "a".repeat(3000));
        assert_eq!(link(&long), None);
    }

    #[test]
    fn the_history_off_and_a_late_answer() {
        let mut history = History::default();
        let first = history.want().unwrap();
        // Asked once per `cb `.
        assert_eq!(history.want(), None);
        assert!(history.answered(first, Answer::Disabled));
        assert_eq!(history.reading, Reading::Done(Answer::Disabled));
        assert!(history.clips().is_empty());
        // Cleared (closed, or the query left `cb `): read again next time.
        history.clear();
        let second = history.want().unwrap();
        assert!(history.timed_out(second));
        assert_eq!(history.reading, Reading::Late);
        // The late answer is dropped.
        assert!(!history.answered(second, Answer::Ready(self::history())));
        assert!(history.clips().is_empty());
        // An answer to a reading before a clear is dropped too.
        history.clear();
        let third = history.want().unwrap();
        history.clear();
        assert!(!history.answered(third, Answer::Ready(self::history())));
        let fourth = history.want().unwrap();
        assert!(history.answered(fourth, Answer::Ready(self::history())));
        assert_eq!(history.clips().len(), 3);
    }

    #[test]
    fn a_text_never_shows_in_debug() {
        let secret = "senha-super-secreta-123";
        let item = clip("id-1", secret, 0);
        for written in [
            format!("{item:?}"),
            format!("{:?}", item.text),
            format!("{:?}", Answer::Ready(vec![item.clone()])),
            format!(
                "{:#?}",
                History {
                    generation: Generation::default(),
                    reading: Reading::Done(Answer::Ready(vec![item.clone()])),
                }
            ),
        ] {
            assert!(!written.contains(secret), "{written}");
            assert!(!written.contains("senha"), "{written}");
        }
        assert_eq!(format!("{:?}", item.text), "ClipText(23 chars)");
        // What is kept to filter by is cut.
        let long = ClipText::new(&"z".repeat(MAX_TEXT_CHARS + 10));
        assert_eq!(long.expose().chars().count(), MAX_TEXT_CHARS);
    }

    #[test]
    fn an_item_id_is_checked() {
        assert!(is_valid_id("{6F1B1A6A-3B2E-4C55-9C12-6F6B0E4A1B2C}"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("a\nb"));
        assert!(!is_valid_id(&"a".repeat(MAX_ID_CHARS + 1)));
    }
}
