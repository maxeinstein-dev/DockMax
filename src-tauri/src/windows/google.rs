//! The Google accounts (ADR 0012): the OAuth client and the refresh tokens
//! in the Windows vault, the sign-in in the default browser with the
//! answer on a loopback address, and the access tokens, kept in memory
//! only. Every call here blocks on the network: never on the dock's thread.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thiserror::Error;
use windows::Win32::Security::Cryptography::{
    BCRYPT_SHA256_ALG_HANDLE, BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom, BCryptHash,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR};

use super::credentials;
use super::http::{self, HttpError, Response};
use crate::domain::google::{
    self, CALENDAR_LIST_URL, CalendarEntry, CalendarListPage, Callback, Client, REVOKE_URL,
    TOKEN_URL, TokenError, Tokens,
};

/// The vault names: the client, and each account by its e-mail.
const CLIENT: &str = "DockMax/google-client";
const ACCOUNT: &str = "DockMax/google:";

/// How long the sign-in waits for the browser to come back.
const SIGN_IN_WAIT: Duration = Duration::from_secs(5 * 60);

/// Why an account call failed. No token, code or address in it: safe to
/// log and to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GoogleError {
    #[error("no OAuth client was set")]
    NoClient,
    #[error("the sign-in was refused")]
    Refused,
    #[error("the sign-in took too long")]
    TimedOut,
    #[error("the sign-in was cancelled in the DockMax")]
    Cancelled,
    #[error("no connection to Google")]
    Offline,
    #[error("Google no longer accepts this account: it must sign in again")]
    SignInAgain,
    #[error("Google gave an answer the DockMax can't read")]
    Unexpected,
    #[error("the vault could not be written")]
    Vault,
}

impl From<HttpError> for GoogleError {
    fn from(_: HttpError) -> Self {
        GoogleError::Offline
    }
}

/// The access tokens of this run, by e-mail, with when each ends.
static ACCESS: Mutex<Option<HashMap<String, (String, Instant)>>> = Mutex::new(None);

/// The flag of the sign-in waiting for the browser, if any: set, it ends
/// the wait. A new sign-in sets the one before it, so closing the browser
/// halfway never leaves Configurações stuck for the 5 minutes.
static SIGN_IN: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Starts a sign-in: the one waiting before it, if any, ends.
fn begin_sign_in() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let mut current = SIGN_IN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(before) = current.replace(flag.clone()) {
        before.store(true, Ordering::Relaxed);
    }
    flag
}

/// Ends the sign-in waiting for the browser, if any (Cancelar).
pub fn cancel_sign_in() {
    let current = SIGN_IN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(flag) = current.as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}

fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    let status = unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    assert!(status.is_ok(), "the system's random numbers failed");
    bytes
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let status = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, data, &mut out) };
    assert!(status.is_ok(), "the system's SHA-256 failed");
    out
}

/// The OAuth client, if one was set.
pub fn client() -> Option<Client> {
    serde_json::from_str(&credentials::read(CLIENT)?).ok()
}

/// Keeps the client pasted in Configurações. The accounts connected with
/// another client keep their tokens; one that Google refuses asks to sign
/// in again.
pub fn set_client(client: &Client) -> Result<(), GoogleError> {
    let json = serde_json::to_string(client).map_err(|_| GoogleError::Vault)?;
    credentials::write(CLIENT, &json).map_err(|_| GoogleError::Vault)
}

/// The connected accounts, by e-mail, sorted.
pub fn accounts() -> Vec<String> {
    credentials::names(ACCOUNT)
        .into_iter()
        .filter_map(|name| name.strip_prefix(ACCOUNT).map(str::to_owned))
        .collect()
}

/// Opens a meeting's address (Meet, Teams, Zoom) in the default browser,
/// or in the app that takes it. Any other address is refused.
pub fn open_meeting(url: &str) -> Result<(), GoogleError> {
    if crate::domain::calendar::is_meeting_address(url) {
        open_browser(url)
    } else {
        Err(GoogleError::Unexpected)
    }
}

/// Opens an event's page in Google Agenda, already checked by the caller.
pub fn open_page(url: &str) -> Result<(), GoogleError> {
    open_browser(url)
}

/// Opens `url` in the default browser.
fn open_browser(url: &str) -> Result<(), GoogleError> {
    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(url),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(GoogleError::Unexpected)
    }
}

/// The small page the browser shows once it came back.
fn answer(stream: &mut TcpStream, done: bool) {
    let text = if done {
        "Conta conectada ao DockMax. Pode fechar esta aba."
    } else {
        "A conta não foi conectada. Volte ao DockMax e tente de novo."
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>DockMax</title>\
         <body style=\"font:16px system-ui;margin:3em\">{text}</body>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Waits for the browser's request that answers this sign-in, until
/// `cancelled` is set or [`SIGN_IN_WAIT`] passes.
fn wait_for_code(
    listener: &TcpListener,
    state: &str,
    cancelled: &AtomicBool,
) -> Result<String, GoogleError> {
    listener
        .set_nonblocking(true)
        .map_err(|_| GoogleError::Unexpected)?;
    let deadline = Instant::now() + SIGN_IN_WAIT;
    while Instant::now() < deadline {
        if cancelled.load(Ordering::Relaxed) {
            return Err(GoogleError::Cancelled);
        }
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(_) => return Err(GoogleError::Unexpected),
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        // The request's first line, at most 8 KB of it.
        let mut line = String::new();
        let _ = BufReader::new(Read::take(&stream, 8 * 1024)).read_line(&mut line);
        match google::callback(&line, state) {
            Callback::Code(code) => {
                answer(&mut stream, true);
                return Ok(code);
            }
            Callback::Refused(_) => {
                answer(&mut stream, false);
                return Err(GoogleError::Refused);
            }
            // Something else (the browser's icon): not this sign-in's.
            Callback::Other => {
                let _ = write!(
                    stream,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"
                );
            }
        }
    }
    Err(GoogleError::TimedOut)
}

/// Reads a token answer.
fn tokens(response: Response) -> Result<Tokens, GoogleError> {
    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| GoogleError::Unexpected),
        400 | 401 => {
            let error: Option<TokenError> = serde_json::from_slice(&response.body).ok();
            match error.map(|error| error.error).as_deref() {
                Some("invalid_grant") | Some("invalid_client") | Some("unauthorized_client") => {
                    Err(GoogleError::SignInAgain)
                }
                _ => Err(GoogleError::Unexpected),
            }
        }
        _ => Err(GoogleError::Unexpected),
    }
}

fn remember(email: &str, tokens: &Tokens) {
    let ends = Instant::now()
        + Duration::from_secs(tokens.expires_in.saturating_sub(google::EXPIRY_MARGIN));
    let mut access = ACCESS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    access
        .get_or_insert_with(HashMap::new)
        .insert(email.to_owned(), (tokens.access_token.clone(), ends));
}

/// Signs an account in: the browser opens on Google's page, and the
/// account is kept once it comes back. Returns its e-mail.
pub fn connect() -> Result<String, GoogleError> {
    let client = client().ok_or(GoogleError::NoClient)?;
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| GoogleError::Unexpected)?;
    let port = listener
        .local_addr()
        .map_err(|_| GoogleError::Unexpected)?
        .port();
    let redirect = google::redirect_uri(port);
    let verifier = google::verifier(&random::<32>());
    let challenge = google::challenge(&sha256(verifier.as_bytes()));
    let state = google::state(&random::<16>());
    let cancelled = begin_sign_in();
    open_browser(&google::auth_url(&client, &redirect, &challenge, &state))?;
    let code = wait_for_code(&listener, &state, &cancelled)?;
    drop(listener);

    let response = http::post_form(
        TOKEN_URL,
        &google::code_form(&client, &code, &verifier, &redirect),
    )?;
    let tokens = tokens(response)?;
    let refresh = tokens
        .refresh_token
        .clone()
        .ok_or(GoogleError::Unexpected)?;
    let calendars = list_calendars(&tokens.access_token)?;
    let email = google::account_email(&calendars)
        .ok_or(GoogleError::Unexpected)?
        .to_owned();
    credentials::write(&format!("{ACCOUNT}{email}"), &refresh).map_err(|_| GoogleError::Vault)?;
    remember(&email, &tokens);
    log::info!("google: an account was connected");
    Ok(email)
}

/// A valid access token of `email`: the one in memory, or a new one.
pub fn access_token(email: &str) -> Result<String, GoogleError> {
    {
        let access = ACCESS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((token, ends)) = access.as_ref().and_then(|access| access.get(email))
            && Instant::now() < *ends
        {
            return Ok(token.clone());
        }
    }
    let client = client().ok_or(GoogleError::NoClient)?;
    let refresh =
        credentials::read(&format!("{ACCOUNT}{email}")).ok_or(GoogleError::SignInAgain)?;
    let tokens = tokens(http::post_form(
        TOKEN_URL,
        &google::refresh_form(&client, &refresh),
    )?)?;
    remember(email, &tokens);
    Ok(tokens.access_token)
}

/// `GET url` for `email`'s account, JSON back. A 401 means the token went
/// before its time (revoked): sign in again. Any other refusal (a calendar
/// no longer shared) is [`GoogleError::Unexpected`].
pub fn get_json<T: serde::de::DeserializeOwned>(email: &str, url: &str) -> Result<T, GoogleError> {
    let token = access_token(email)?;
    let response = http::get(url, Some(&token))?;
    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| GoogleError::Unexpected),
        401 => {
            forget_access(email);
            Err(GoogleError::SignInAgain)
        }
        _ => Err(GoogleError::Unexpected),
    }
}

/// `PATCH url` for `email`'s account with `json`: whether Google took it.
pub fn patch_json(email: &str, url: &str, json: &str) -> Result<(), GoogleError> {
    let token = access_token(email)?;
    let response = http::patch_json(url, &token, json)?;
    match response.status {
        200 => Ok(()),
        401 => {
            forget_access(email);
            Err(GoogleError::SignInAgain)
        }
        _ => Err(GoogleError::Unexpected),
    }
}

fn forget_access(email: &str) {
    let mut access = ACCESS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(access) = access.as_mut() {
        access.remove(email);
    }
}

fn list_calendars(token: &str) -> Result<Vec<CalendarEntry>, GoogleError> {
    let mut calendars = Vec::new();
    let mut page_token: Option<String> = None;
    // At most 10 pages of 250: far more calendars than anyone has.
    for _ in 0..10 {
        let mut url = url::Url::parse(CALENDAR_LIST_URL).expect("a valid constant");
        url.query_pairs_mut().append_pair("maxResults", "250");
        if let Some(page) = &page_token {
            url.query_pairs_mut().append_pair("pageToken", page);
        }
        let response = http::get(url.as_str(), Some(token))?;
        if response.status != 200 {
            return Err(if response.status == 401 {
                GoogleError::SignInAgain
            } else {
                GoogleError::Unexpected
            });
        }
        let page: CalendarListPage =
            serde_json::from_slice(&response.body).map_err(|_| GoogleError::Unexpected)?;
        calendars.extend(page.items);
        page_token = page.next_page_token;
        if page_token.is_none() {
            break;
        }
    }
    Ok(calendars)
}

/// The calendars of `email`'s account.
pub fn calendars(email: &str) -> Result<Vec<CalendarEntry>, GoogleError> {
    let token = access_token(email)?;
    list_calendars(&token).inspect_err(|error| {
        if *error == GoogleError::SignInAgain {
            forget_access(email);
        }
    })
}

/// Disconnects `email`: the token is revoked at Google and leaves the
/// vault. Without a connection, it leaves the vault all the same.
pub fn disconnect(email: &str) -> Result<(), GoogleError> {
    let name = format!("{ACCOUNT}{email}");
    if let Some(refresh) = credentials::read(&name) {
        match http::post_form(REVOKE_URL, &google::revoke_form(&refresh)) {
            Ok(response) if response.status == 200 => {}
            _ => log::warn!("google: the token could not be revoked; it leaves the vault anyway"),
        }
    }
    forget_access(email);
    credentials::delete(&name).map_err(|_| GoogleError::Vault)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_sha256_is_the_one_pkce_needs() {
        // RFC 7636, appendix B.
        let challenge = google::challenge(&sha256(b"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn cancelar_ends_the_wait_and_a_new_sign_in_ends_the_one_before() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let first = begin_sign_in();
        let second = begin_sign_in();
        assert!(
            first.load(Ordering::Relaxed),
            "the new sign-in ends the old"
        );
        assert!(!second.load(Ordering::Relaxed));

        cancel_sign_in();
        let started = Instant::now();
        assert_eq!(
            wait_for_code(&listener, "ST", &second),
            Err(GoogleError::Cancelled)
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn random_bytes_differ() {
        assert_ne!(random::<16>(), random::<16>());
    }

    #[test]
    fn a_token_answer_tells_when_to_sign_in_again() {
        let answer = |status, body: &str| Response {
            status,
            body: body.as_bytes().to_vec(),
        };
        assert_eq!(
            tokens(answer(400, r#"{"error":"invalid_grant"}"#)),
            Err(GoogleError::SignInAgain)
        );
        assert_eq!(tokens(answer(500, "")), Err(GoogleError::Unexpected));
        assert_eq!(
            tokens(answer(200, r#"{"access_token":"a","expires_in":10}"#)).map(|t| t.expires_in),
            Ok(10)
        );
    }
}
