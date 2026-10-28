//! The Google sign-in of an installed app (ADR 0012, RFC 8252): the PKCE
//! pair, the address of the sign-in page, the request the browser sends
//! back to the loopback address, and the token answers. Pure:
//! `windows::google` does the network, the browser and the vault.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// The sign-in page and the token, revoke and API addresses.
pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
pub const CALENDAR_LIST_URL: &str = "https://www.googleapis.com/calendar/v3/users/me/calendarList";

/// Read only: the calendars and the tasks (spec 0017 §6c).
pub const SCOPES: &str = "https://www.googleapis.com/auth/calendar.readonly \
                          https://www.googleapis.com/auth/tasks.readonly";

/// The OAuth client the author made in Google Cloud (a "Desktop app"
/// client). Kept in the Windows vault, never in a file or a log.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: String,
    pub secret: String,
}

impl std::fmt::Debug for Client {
    // The secret never reaches a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Why a client pasted in Configurações is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ClientError {
    #[error("the client id is not a Google OAuth client id")]
    InvalidId,
    #[error("the client secret is empty or not one line")]
    InvalidSecret,
}

impl Client {
    /// Trims what was pasted and checks its shape: the id of a Google
    /// client ends in `.apps.googleusercontent.com`; the secret is one line
    /// of visible ASCII.
    pub fn parse(id: &str, secret: &str) -> Result<Client, ClientError> {
        let (id, secret) = (id.trim(), secret.trim());
        let id_ok = id.len() <= 200
            && id.ends_with(".apps.googleusercontent.com")
            && id.len() > ".apps.googleusercontent.com".len()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b));
        if !id_ok {
            return Err(ClientError::InvalidId);
        }
        let secret_ok = !secret.is_empty()
            && secret.len() <= 200
            && secret.bytes().all(|b| b.is_ascii_graphic());
        if !secret_ok {
            return Err(ClientError::InvalidSecret);
        }
        Ok(Client {
            id: id.to_owned(),
            secret: secret.to_owned(),
        })
    }
}

/// The PKCE verifier of 32 random bytes, as base64url (43 characters).
pub fn verifier(random: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(random)
}

/// The S256 challenge of `verifier`, given its SHA-256.
pub fn challenge(sha256_of_verifier: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(sha256_of_verifier)
}

/// The `state` of a sign-in: 16 random bytes, as base64url.
pub fn state(random: &[u8; 16]) -> String {
    URL_SAFE_NO_PAD.encode(random)
}

/// The sign-in page for `client`, coming back to `redirect`. Offline
/// access and the consent screen, so Google gives a refresh token every
/// time (a second sign-in of the same account would get none otherwise).
pub fn auth_url(client: &Client, redirect: &str, challenge: &str, state: &str) -> String {
    let mut url = Url::parse(AUTH_URL).expect("a valid constant");
    url.query_pairs_mut()
        .append_pair("client_id", &client.id)
        .append_pair("redirect_uri", redirect)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url.into()
}

/// The loopback address the browser comes back to.
pub fn redirect_uri(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// How the sign-in ended, read from the browser's request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    /// The code to exchange for the tokens.
    Code(String),
    /// The person refused, or Google said why not (`access_denied`...).
    Refused(String),
    /// Not the answer of this sign-in: another `state`, or a request for
    /// something else (the browser's `/favicon.ico`).
    Other,
}

/// Reads the first line of the browser's request (`GET /?code=...&state=...
/// HTTP/1.1`), accepting it only with the `state` this sign-in sent.
pub fn callback(request_line: &str, expected_state: &str) -> Callback {
    let mut parts = request_line.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Callback::Other;
    };
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Callback::Other;
    };
    if url.path() != "/" {
        return Callback::Other;
    }
    let find = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    if find("state").as_deref() != Some(expected_state) {
        return Callback::Other;
    }
    match (find("code"), find("error")) {
        (Some(code), None) if !code.is_empty() => Callback::Code(code),
        (_, Some(error)) => Callback::Refused(error),
        _ => Callback::Other,
    }
}

/// The form that exchanges the code for the tokens.
pub fn code_form(client: &Client, code: &str, verifier: &str, redirect: &str) -> String {
    form(&[
        ("client_id", &client.id),
        ("client_secret", &client.secret),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect),
    ])
}

/// The form that gets a new access token with the refresh token.
pub fn refresh_form(client: &Client, refresh_token: &str) -> String {
    form(&[
        ("client_id", &client.id),
        ("client_secret", &client.secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ])
}

/// The form that revokes a token.
pub fn revoke_form(token: &str) -> String {
    form(&[("token", token)])
}

fn form(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
}

/// Google's answer with the tokens. The refresh token comes only on the
/// first exchange.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Seconds the access token lasts.
    pub expires_in: u64,
}

impl std::fmt::Debug for Tokens {
    // No token reaches a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens")
            .field("expires_in", &self.expires_in)
            .finish_non_exhaustive()
    }
}

/// What a token answer says when it is not tokens: `invalid_grant` means
/// the account must sign in again (revoked, or the client changed).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TokenError {
    pub error: String,
}

/// Seconds before it ends that an access token is taken for ended, so a
/// request never starts with a token about to expire.
pub const EXPIRY_MARGIN: u64 = 60;

/// A calendar of an account, as Configurações lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEntry {
    pub id: String,
    #[serde(rename(deserialize = "summary"))]
    pub name: String,
    /// `#rrggbb`, the color the person chose in Google Agenda.
    #[serde(default, rename(deserialize = "backgroundColor"))]
    pub color: Option<String>,
    #[serde(default)]
    pub primary: bool,
}

/// A page of `calendarList`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListPage {
    #[serde(default)]
    pub items: Vec<CalendarEntry>,
    pub next_page_token: Option<String>,
}

/// The account's e-mail: the id of its primary calendar.
pub fn account_email(calendars: &[CalendarEntry]) -> Option<&str> {
    calendars
        .iter()
        .find(|calendar| calendar.primary)
        .map(|calendar| calendar.id.as_str())
        .filter(|id| id.contains('@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> Client {
        Client::parse("123-abc.apps.googleusercontent.com", "GOCSPX-test").unwrap()
    }

    #[test]
    fn the_pkce_challenge_is_the_one_of_rfc_7636() {
        // RFC 7636, appendix B.
        let random: [u8; 32] = [
            116, 24, 223, 180, 151, 153, 224, 37, 79, 250, 96, 125, 216, 173, 187, 186, 22, 212,
            37, 77, 105, 214, 191, 240, 91, 88, 5, 88, 83, 132, 141, 121,
        ];
        assert_eq!(
            verifier(&random),
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        );
        let sha: [u8; 32] = [
            19, 211, 30, 150, 26, 26, 216, 236, 47, 22, 177, 12, 76, 152, 46, 8, 118, 168, 120,
            173, 109, 241, 68, 86, 110, 225, 137, 74, 203, 112, 249, 195,
        ];
        assert_eq!(
            challenge(&sha),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_pasted_client_is_trimmed_and_checked() {
        assert_eq!(
            Client::parse("  123-abc.apps.googleusercontent.com\n", " GOCSPX-x "),
            Ok(Client {
                id: "123-abc.apps.googleusercontent.com".into(),
                secret: "GOCSPX-x".into()
            })
        );
        assert_eq!(Client::parse("123-abc", "x"), Err(ClientError::InvalidId));
        assert_eq!(
            Client::parse(".apps.googleusercontent.com", "x"),
            Err(ClientError::InvalidId)
        );
        assert_eq!(
            Client::parse("1.apps.googleusercontent.com", "two words"),
            Err(ClientError::InvalidSecret)
        );
        assert_eq!(
            Client::parse("1.apps.googleusercontent.com", ""),
            Err(ClientError::InvalidSecret)
        );
        // The secret never shows in a log.
        assert!(!format!("{:?}", client()).contains("GOCSPX"));
    }

    #[test]
    fn the_sign_in_page_asks_for_reading_only_with_pkce() {
        let url = Url::parse(&auth_url(&client(), &redirect_uri(5123), "CH", "ST")).unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        assert_eq!(pairs["client_id"], "123-abc.apps.googleusercontent.com");
        assert_eq!(pairs["redirect_uri"], "http://127.0.0.1:5123");
        assert_eq!(pairs["code_challenge"], "CH");
        assert_eq!(pairs["code_challenge_method"], "S256");
        assert_eq!(pairs["state"], "ST");
        assert_eq!(pairs["access_type"], "offline");
        assert!(
            pairs["scope"]
                .split(' ')
                .all(|scope| scope.ends_with(".readonly"))
        );
        // The secret never goes to the browser.
        assert!(!url.as_str().contains("GOCSPX"));
    }

    #[test]
    fn only_the_answer_with_this_state_counts() {
        assert_eq!(
            callback("GET /?state=ST&code=4%2Fabc&scope=x HTTP/1.1", "ST"),
            Callback::Code("4/abc".into())
        );
        assert_eq!(
            callback("GET /?error=access_denied&state=ST HTTP/1.1", "ST"),
            Callback::Refused("access_denied".into())
        );
        assert_eq!(
            callback("GET /?state=other&code=4 HTTP/1.1", "ST"),
            Callback::Other
        );
        assert_eq!(callback("GET /favicon.ico HTTP/1.1", "ST"), Callback::Other);
        assert_eq!(
            callback("POST /?state=ST&code=4 HTTP/1.1", "ST"),
            Callback::Other
        );
        assert_eq!(callback("", "ST"), Callback::Other);
    }

    #[test]
    fn the_forms_are_url_encoded() {
        let form = code_form(&client(), "4/a b", "VER", "http://127.0.0.1:1");
        assert!(form.contains("code=4%2Fa+b"));
        assert!(form.contains("grant_type=authorization_code"));
        assert!(form.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1"));
        assert!(refresh_form(&client(), "1//r").contains("refresh_token=1%2F%2Fr"));
    }

    #[test]
    fn the_token_answers_read_and_never_show_a_token() {
        let tokens: Tokens = serde_json::from_str(
            r#"{"access_token":"ya29.x","expires_in":3599,"refresh_token":"1//r","scope":"s","token_type":"Bearer"}"#,
        )
        .unwrap();
        assert_eq!(tokens.refresh_token.as_deref(), Some("1//r"));
        assert_eq!(tokens.expires_in, 3599);
        assert!(!format!("{tokens:?}").contains("ya29"));
        let error: TokenError =
            serde_json::from_str(r#"{"error":"invalid_grant","error_description":"x"}"#).unwrap();
        assert_eq!(error.error, "invalid_grant");
    }

    #[test]
    fn the_account_is_the_id_of_its_primary_calendar() {
        let page: CalendarListPage = serde_json::from_str(
            r##"{"items":[
                {"id":"pt-br.brazilian#holiday@group.v.calendar.google.com","summary":"Feriados","backgroundColor":"#16a765"},
                {"id":"max@example.com","summary":"Max","backgroundColor":"#9fc6e7","primary":true}
            ]}"##,
        )
        .unwrap();
        assert_eq!(account_email(&page.items), Some("max@example.com"));
        assert_eq!(page.items[0].name, "Feriados");
        assert_eq!(page.items[0].color.as_deref(), Some("#16a765"));
        assert_eq!(account_email(&page.items[..1]), None);
    }
}
