//! What Configurações asks of the launcher (spec 0012): whether another
//! program holds the hotkey, forgetting what it opened (L1), and a new
//! hotkey (L3).

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, State};

use super::widgets::WidgetCommandError;
use crate::domain::hotkey::{self, Problem};
use crate::domain::settings::Settings;
use crate::persistence::database::Storage;
use crate::persistence::launcher_repo;

/// What Configurações shows about the launcher; matches `LauncherStatus` in
/// `src/core/launcher.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    /// Another program holds Alt+Space: the launcher opens only from the
    /// menus.
    pub hotkey_taken: bool,
}

#[tauri::command]
pub fn launcher_status() -> LauncherStatus {
    #[cfg(windows)]
    let hotkey_taken = crate::windows::launcher::status().hotkey_taken;
    #[cfg(not(windows))]
    let hotkey_taken = false;
    LauncherStatus { hotkey_taken }
}

/// Forgets everything the launcher opened ("Limpar o histórico").
#[tauri::command]
pub fn clear_launcher_history(state: State<'_, Mutex<Storage>>) -> Result<(), WidgetCommandError> {
    let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
    let conn = storage.connection()?;
    launcher_repo::clear(conn)?;
    log::info!("launcher: history cleared");
    Ok(())
}

/// Why a new hotkey was refused, as the interface sees it; matches
/// `HotkeyErrorCode` in `src/core/launcher.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyErrorCode {
    Malformed,
    NoModifier,
    TakesTyping,
    Reserved,
    /// Another program holds it.
    Taken,
    /// It could not be saved; the hotkey before it stays.
    NotSaved,
}

impl From<Problem> for HotkeyErrorCode {
    fn from(problem: Problem) -> Self {
        match problem {
            Problem::Malformed => HotkeyErrorCode::Malformed,
            Problem::NoModifier => HotkeyErrorCode::NoModifier,
            Problem::TakesTyping => HotkeyErrorCode::TakesTyping,
            Problem::Reserved => HotkeyErrorCode::Reserved,
        }
    }
}

/// `{ "code": "taken" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HotkeyCommandError {
    code: HotkeyErrorCode,
}

impl From<HotkeyErrorCode> for HotkeyCommandError {
    fn from(code: HotkeyErrorCode) -> Self {
        HotkeyCommandError { code }
    }
}

/// Makes `hotkey` (as `hotkey::Hotkey` writes it) the launcher's: refused
/// when the launcher can't take it; with the launcher on, tried for real
/// first, and refused when another program holds it. Async: the launcher
/// answers on the main thread, which this must not hold.
#[tauri::command]
pub async fn set_launcher_hotkey(
    app: AppHandle,
    hotkey: String,
) -> Result<Settings, HotkeyCommandError> {
    let parsed = hotkey::check(&hotkey).map_err(HotkeyErrorCode::from)?;
    let on = super::settings::current(&app).launcher;
    #[cfg(windows)]
    if on {
        use crate::windows::launcher::{TryError, try_hotkey};
        try_hotkey(parsed).map_err(|error| match error {
            TryError::Taken => HotkeyErrorCode::Taken,
            TryError::NotReady => HotkeyErrorCode::NotSaved,
        })?;
    }
    #[cfg(not(windows))]
    let _ = (on, parsed);
    let saved = super::settings::change(&app, |settings| {
        settings.launcher_hotkey = parsed.to_string();
    });
    match saved {
        Ok(settings) => {
            log::info!("launcher: the hotkey is now {parsed}");
            Ok(settings)
        }
        Err(error) => {
            log::warn!("launcher: the hotkey could not be saved: {error}");
            // The one tried goes; the one saved comes back.
            #[cfg(windows)]
            crate::windows::launcher::set_enabled(on);
            Err(HotkeyErrorCode::NotSaved.into())
        }
    }
}

/// Configurações captures a new hotkey (`on`) or is done: while it does,
/// the current one is let go, or pressing it would open the launcher.
#[tauri::command]
pub fn capture_launcher_hotkey(on: bool) {
    #[cfg(windows)]
    crate::windows::launcher::capture(on);
    #[cfg(not(windows))]
    let _ = on;
}
