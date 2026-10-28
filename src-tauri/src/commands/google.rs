//! Configurações › Contas (spec 0017 F2a, ADR 0012): the OAuth client, the
//! Google accounts and their calendars. Only the Configurações window calls
//! these; the network and the browser run off the async runtime's threads.

use serde::Serialize;
use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::domain::google::{CalendarEntry, Client, ClientError};
#[cfg(windows)]
use crate::windows::google::{self, GoogleError};

/// Event every window gets when an account comes or goes. Must match
/// `GOOGLE_ACCOUNTS_EVENT` in `src/core/google.ts`.
pub const ACCOUNTS_EVENT: &str = "google-accounts-changed";

/// The Configurações app's window.
const SETTINGS_LABEL: &str = "settings";

/// What the page gets when a call fails: only a code. Must match
/// `GoogleErrorCode` in `src/core/google.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GoogleErrorCode {
    NoClient,
    InvalidClientId,
    InvalidClientSecret,
    Refused,
    TimedOut,
    /// Cancelar, or a new sign-in started: not shown as an error.
    Cancelled,
    Offline,
    SignInAgain,
    NotConnected,
    Unexpected,
    Vault,
    NotAllowed,
}

#[cfg(windows)]
impl From<GoogleError> for GoogleErrorCode {
    fn from(error: GoogleError) -> Self {
        match error {
            GoogleError::NoClient => GoogleErrorCode::NoClient,
            GoogleError::Refused => GoogleErrorCode::Refused,
            GoogleError::TimedOut => GoogleErrorCode::TimedOut,
            GoogleError::Cancelled => GoogleErrorCode::Cancelled,
            GoogleError::Offline => GoogleErrorCode::Offline,
            GoogleError::SignInAgain => GoogleErrorCode::SignInAgain,
            GoogleError::Unexpected => GoogleErrorCode::Unexpected,
            GoogleError::Vault => GoogleErrorCode::Vault,
        }
    }
}

impl From<ClientError> for GoogleErrorCode {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::InvalidId => GoogleErrorCode::InvalidClientId,
            ClientError::InvalidSecret => GoogleErrorCode::InvalidClientSecret,
        }
    }
}

/// The client set (its id only, never the secret) and the accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleStatus {
    pub client_id: Option<String>,
    pub accounts: Vec<String>,
}

fn only_settings(window: &WebviewWindow) -> Result<(), GoogleErrorCode> {
    if window.label() == SETTINGS_LABEL {
        Ok(())
    } else {
        Err(GoogleErrorCode::NotAllowed)
    }
}

/// Runs `work` on a thread that may block, and logs a failure by its kind.
async fn blocking<T: Send + 'static>(
    what: &'static str,
    work: impl FnOnce() -> Result<T, GoogleErrorCode> + Send + 'static,
) -> Result<T, GoogleErrorCode> {
    let result = tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(GoogleErrorCode::Unexpected));
    if let Err(code) = &result {
        log::warn!("google: {what} failed: {code:?}");
    }
    result
}

#[cfg(windows)]
#[tauri::command]
pub fn google_status(window: WebviewWindow) -> Result<GoogleStatus, GoogleErrorCode> {
    only_settings(&window)?;
    Ok(GoogleStatus {
        client_id: google::client().map(|client| client.id),
        accounts: google::accounts(),
    })
}

/// Keeps the client pasted in Configurações, checked first.
#[cfg(windows)]
#[tauri::command]
pub fn set_google_client(
    window: WebviewWindow,
    id: String,
    secret: String,
) -> Result<(), GoogleErrorCode> {
    only_settings(&window)?;
    let client = Client::parse(&id, &secret)?;
    google::set_client(&client).map_err(GoogleErrorCode::from)
}

/// Signs an account in through the browser; its e-mail once it is kept.
#[cfg(windows)]
#[tauri::command]
pub async fn connect_google_account(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<String, GoogleErrorCode> {
    only_settings(&window)?;
    let email = blocking("connect", || {
        google::connect().map_err(GoogleErrorCode::from)
    })
    .await?;
    let _ = app.emit(ACCOUNTS_EVENT, ());
    Ok(email)
}

/// Ends the sign-in waiting for the browser (Cancelar): its
/// `connect_google_account` answers `cancelled`.
#[cfg(windows)]
#[tauri::command]
pub fn cancel_google_sign_in(window: WebviewWindow) -> Result<(), GoogleErrorCode> {
    only_settings(&window)?;
    google::cancel_sign_in();
    Ok(())
}

/// The calendars of a connected account.
#[cfg(windows)]
#[tauri::command]
pub async fn google_calendars(
    window: WebviewWindow,
    email: String,
) -> Result<Vec<CalendarEntry>, GoogleErrorCode> {
    only_settings(&window)?;
    if !google::accounts().contains(&email) {
        return Err(GoogleErrorCode::NotConnected);
    }
    blocking("calendars", move || {
        google::calendars(&email).map_err(GoogleErrorCode::from)
    })
    .await
}

/// Disconnects an account: its token is revoked and leaves the vault.
#[cfg(windows)]
#[tauri::command]
pub async fn disconnect_google_account(
    app: AppHandle,
    window: WebviewWindow,
    email: String,
) -> Result<(), GoogleErrorCode> {
    only_settings(&window)?;
    if !google::accounts().contains(&email) {
        return Err(GoogleErrorCode::NotConnected);
    }
    blocking("disconnect", move || {
        google::disconnect(&email).map_err(GoogleErrorCode::from)
    })
    .await?;
    let _ = app.emit(ACCOUNTS_EVENT, ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_reach_the_page_as_camel_case_codes() {
        let code = |code| serde_json::to_value(code).unwrap();
        assert_eq!(code(GoogleErrorCode::SignInAgain), "signInAgain");
        assert_eq!(code(GoogleErrorCode::InvalidClientId), "invalidClientId");
        assert_eq!(
            GoogleErrorCode::from(ClientError::InvalidSecret),
            GoogleErrorCode::InvalidClientSecret
        );
    }
}
