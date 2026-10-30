//! HTTPS through WinHTTP (ADR 0012): the system's TLS, certificates and
//! proxy, and no network library in the binary. Blocking calls, made off
//! the dock's thread. Only `https://`; a body of at most [`MAX_BODY`].

use std::ffi::c_void;

use thiserror::Error;
use url::Url;
use windows::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::core::{HSTRING, PCWSTR, w};

/// The largest answer read; Google's pages are far smaller.
pub const MAX_BODY: usize = 8 * 1024 * 1024;

/// Why a request got no answer. Never carries the address or the body,
/// which may hold a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HttpError {
    #[error("the address is not https")]
    NotHttps,
    #[error("no connection")]
    Offline,
    #[error("the answer is too large")]
    TooLarge,
}

/// An answer: its status and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// A WinHTTP handle, closed when dropped.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> Result<Handle, HttpError> {
        if raw.is_null() {
            Err(HttpError::Offline)
        } else {
            Ok(Handle(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// `GET url`, with `Authorization: Bearer <token>` when given.
pub fn get(url: &str, bearer: Option<&str>) -> Result<Response, HttpError> {
    request("GET", url, bearer, &[], None)
}

/// `GET url` with `Authorization: Bearer <token>` and the headers `extra`
/// (name and value each).
pub fn get_with(url: &str, bearer: &str, extra: &[(&str, &str)]) -> Result<Response, HttpError> {
    request("GET", url, Some(bearer), extra, None)
}

/// `POST url` with a form body (`application/x-www-form-urlencoded`).
pub fn post_form(url: &str, form: &str) -> Result<Response, HttpError> {
    request(
        "POST",
        url,
        None,
        &[],
        Some(("application/x-www-form-urlencoded", form)),
    )
}

/// `PATCH url` with a JSON body, with `Authorization: Bearer <token>`.
pub fn patch_json(url: &str, bearer: &str, json: &str) -> Result<Response, HttpError> {
    request(
        "PATCH",
        url,
        Some(bearer),
        &[],
        Some(("application/json", json)),
    )
}

/// `body`: its content type and the text.
fn request(
    method: &str,
    url: &str,
    bearer: Option<&str>,
    extra: &[(&str, &str)],
    body: Option<(&str, &str)>,
) -> Result<Response, HttpError> {
    let url = Url::parse(url).map_err(|_| HttpError::NotHttps)?;
    if url.scheme() != "https" {
        return Err(HttpError::NotHttps);
    }
    let host = url.host_str().ok_or(HttpError::NotHttps)?;
    let port = url.port_or_known_default().unwrap_or(443);
    let path = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };

    let session = Handle::new(unsafe {
        WinHttpOpen(
            w!("Dockyard"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        )
    })?;
    // Resolve, connect, send, receive: 10 s, 10 s, 15 s, 20 s.
    let _ = unsafe { WinHttpSetTimeouts(session.0, 10_000, 10_000, 15_000, 20_000) };
    let connection =
        Handle::new(unsafe { WinHttpConnect(session.0, &HSTRING::from(host), port, 0) })?;
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            &HSTRING::from(method),
            &HSTRING::from(path),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_OPEN_REQUEST_FLAGS(WINHTTP_FLAG_SECURE.0),
        )
    })?;

    let mut headers = String::from("Accept: application/json\r\n");
    if let Some(token) = bearer {
        headers.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some((content_type, _)) = body {
        headers.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    for (name, value) in extra {
        headers.push_str(&format!("{name}: {value}\r\n"));
    }
    let headers: Vec<u16> = headers.encode_utf16().collect();
    let body = body.map_or("", |(_, text)| text).as_bytes();
    let length = u32::try_from(body.len()).map_err(|_| HttpError::TooLarge)?;
    unsafe {
        WinHttpSendRequest(
            request.0,
            Some(&headers),
            (!body.is_empty()).then_some(body.as_ptr().cast()),
            length,
            length,
            0,
        )
        .map_err(|_| HttpError::Offline)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(|_| HttpError::Offline)?;
    }

    let mut status: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(std::ptr::from_mut(&mut status).cast()),
            &mut size,
            std::ptr::null_mut(),
        )
        .map_err(|_| HttpError::Offline)?;
    }

    let mut body = Vec::new();
    let mut chunk = vec![0u8; 16 * 1024];
    loop {
        let mut read: u32 = 0;
        unsafe {
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut read,
            )
            .map_err(|_| HttpError::Offline)?;
        }
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read as usize]);
        if body.len() > MAX_BODY {
            return Err(HttpError::TooLarge);
        }
    }
    Ok(Response {
        status: status as u16,
        body,
    })
}
