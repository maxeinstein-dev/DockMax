//! The launcher's web search by keyword (spec 0012 §5, L2): `g dockyard`
//! gives one result, "Buscar “dockyard” no Google". The keywords come ready
//! and are edited in Configurações: a word, a name and an address with `%s`
//! where the term goes. The address must start with `https://`, and the term
//! is encoded for a URL. Pure.

use serde::{Deserialize, Serialize};

/// Most keywords kept.
pub const MAX_KEYWORDS: usize = 20;
/// The longest keyword, name and address, in characters.
pub const MAX_KEYWORD: usize = 16;
pub const MAX_NAME: usize = 40;
pub const MAX_TEMPLATE: usize = 500;

/// Where the term goes in an address.
pub const TERM: &str = "%s";

/// One keyword of the web search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebKeyword {
    /// What is typed before the term (`g`).
    pub keyword: String,
    /// The search's name (`Google`).
    pub name: String,
    /// The address, with `%s` for the term
    /// (`https://www.google.com/search?q=%s`).
    pub template: String,
}

/// The keywords that come ready; the first is the default search, used when
/// nothing else matches.
pub fn defaults() -> Vec<WebKeyword> {
    [
        ("g", "Google", "https://www.google.com/search?q=%s"),
        (
            "yt",
            "YouTube",
            "https://www.youtube.com/results?search_query=%s",
        ),
        (
            "w",
            "Wikipédia",
            "https://pt.wikipedia.org/w/index.php?search=%s",
        ),
        ("gh", "GitHub", "https://github.com/search?q=%s"),
    ]
    .into_iter()
    .map(|(keyword, name, template)| WebKeyword {
        keyword: keyword.to_owned(),
        name: name.to_owned(),
        template: template.to_owned(),
    })
    .collect()
}

/// Whether `keyword` can be kept: a word without spaces, a name and an
/// `https://` address with a host and `%s`, none of them too long or with
/// control characters.
pub fn is_valid(keyword: &WebKeyword) -> bool {
    let word = keyword.keyword.chars().count();
    let name = keyword.name.chars().count();
    (1..=MAX_KEYWORD).contains(&word)
        && !keyword
            .keyword
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        && (1..=MAX_NAME).contains(&name)
        && !keyword.name.trim().is_empty()
        && !keyword.name.chars().any(char::is_control)
        && is_valid_template(&keyword.template)
}

/// An `https://` address with a host, `%s` and nothing that is not part of
/// a URL (spaces, control characters).
pub fn is_valid_template(template: &str) -> bool {
    let Some(rest) = template.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    template.chars().count() <= MAX_TEMPLATE
        && !host.is_empty()
        && !host.contains(TERM)
        && template.contains(TERM)
        && !template
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
}

/// Whether the whole list can be kept: each keyword valid, at most
/// `MAX_KEYWORDS`, and no word twice (in any case).
pub fn is_valid_list(keywords: &[WebKeyword]) -> bool {
    let mut words: Vec<String> = keywords
        .iter()
        .map(|keyword| keyword.keyword.to_lowercase())
        .collect();
    words.sort();
    words.dedup();
    keywords.len() <= MAX_KEYWORDS && words.len() == keywords.len() && keywords.iter().all(is_valid)
}

/// The keyword `query` starts with, followed by a space, and the term after
/// it; `None` when it starts with none or the term is empty.
pub fn matching<'a>(
    keywords: &'a [WebKeyword],
    query: &'a str,
) -> Option<(&'a WebKeyword, &'a str)> {
    let query = query.trim_start();
    let (word, term) = query.split_once(' ')?;
    let term = term.trim();
    if term.is_empty() {
        return None;
    }
    let word = word.to_lowercase();
    keywords
        .iter()
        .find(|keyword| keyword.keyword.to_lowercase() == word)
        .map(|keyword| (keyword, term))
}

/// The address of `template` searching for `term`, encoded for a URL; `None`
/// when the template is not a valid one.
pub fn url(template: &str, term: &str) -> Option<String> {
    is_valid_template(template).then(|| template.replace(TERM, &encode(term)))
}

/// `text` with everything but the letters, digits and `-_.~` of ASCII
/// written as `%XX` of its UTF-8 bytes.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyword(word: &str, template: &str) -> WebKeyword {
        WebKeyword {
            keyword: word.to_owned(),
            name: "Busca".to_owned(),
            template: template.to_owned(),
        }
    }

    #[test]
    fn the_defaults_are_valid() {
        let defaults = defaults();
        assert!(is_valid_list(&defaults));
        assert_eq!(defaults[0].keyword, "g", "Google is the default search");
    }

    #[test]
    fn only_https_addresses_with_a_host_and_the_term_are_kept() {
        assert!(is_valid_template("https://example.com/?q=%s"));
        assert!(is_valid_template("https://example.com/%s"));
        for bad in [
            "http://example.com/?q=%s",
            "javascript:alert(1)//%s",
            "file:///C:/%s",
            "ms-settings:%s",
            "https://example.com/?q=",
            "https:///?q=%s",
            "https://%s.example.com/",
            "https://example.com/?q=%s x",
            " https://example.com/?q=%s",
            "HTTPS://example.com/?q=%s",
        ] {
            assert!(!is_valid_template(bad), "{bad:?}");
        }
        let long = format!("https://example.com/?q=%s&x={}", "a".repeat(MAX_TEMPLATE));
        assert!(!is_valid_template(&long));
    }

    #[test]
    fn a_keyword_needs_a_word_and_a_name() {
        let good = keyword("g", "https://example.com/?q=%s");
        assert!(is_valid(&good));
        assert!(!is_valid(&WebKeyword {
            keyword: String::new(),
            ..good.clone()
        }));
        assert!(!is_valid(&WebKeyword {
            keyword: "g g".into(),
            ..good.clone()
        }));
        assert!(!is_valid(&WebKeyword {
            keyword: "a".repeat(MAX_KEYWORD + 1),
            ..good.clone()
        }));
        assert!(!is_valid(&WebKeyword {
            name: "  ".into(),
            ..good.clone()
        }));
        assert!(!is_valid(&WebKeyword {
            name: "a\u{7}".into(),
            ..good.clone()
        }));
        assert!(!is_valid(&WebKeyword {
            name: "a".repeat(MAX_NAME + 1),
            ..good
        }));
    }

    #[test]
    fn a_word_is_kept_once_and_the_list_has_a_limit() {
        let one = keyword("g", "https://example.com/?q=%s");
        assert!(!is_valid_list(&[
            one.clone(),
            keyword("G", "https://other.com/?q=%s")
        ]));
        let many: Vec<WebKeyword> = (0..=MAX_KEYWORDS)
            .map(|n| keyword(&format!("k{n}"), "https://example.com/?q=%s"))
            .collect();
        assert!(!is_valid_list(&many));
        assert!(is_valid_list(&many[..MAX_KEYWORDS]));
        assert!(is_valid_list(&[]));
    }

    #[test]
    fn a_query_starting_with_a_keyword_and_a_space_searches() {
        let keywords = defaults();
        let (found, term) = matching(&keywords, "g dockyard").unwrap();
        assert_eq!((found.name.as_str(), term), ("Google", "dockyard"));
        let (found, term) = matching(&keywords, "YT  lo-fi beats ").unwrap();
        assert_eq!((found.name.as_str(), term), ("YouTube", "lo-fi beats"));
        assert!(matching(&keywords, "g").is_none(), "no space");
        assert!(matching(&keywords, "g   ").is_none(), "no term");
        assert!(
            matching(&keywords, "google chrome").is_none(),
            "not a keyword"
        );
        assert!(matching(&keywords, "gdockyard").is_none());
    }

    #[test]
    fn the_term_is_encoded_for_a_url() {
        assert_eq!(encode("dock max"), "dock%20max");
        assert_eq!(encode("a&b=c#d"), "a%26b%3Dc%23d");
        assert_eq!(encode("ação"), "a%C3%A7%C3%A3o");
        assert_eq!(encode("%s"), "%25s");
        assert_eq!(
            url("https://www.google.com/search?q=%s", "1+1").as_deref(),
            Some("https://www.google.com/search?q=1%2B1")
        );
        assert_eq!(url("http://example.com/?q=%s", "x"), None);
    }
}
