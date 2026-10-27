//! What Configurações asks of the launcher (spec 0012 L1): whether another
//! program holds Alt+Space, and forgetting what it opened.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::State;

use super::widgets::WidgetCommandError;
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
