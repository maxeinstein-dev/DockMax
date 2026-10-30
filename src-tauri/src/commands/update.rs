//! Sobre's updates (spec 0019 G3), in the panel and in Configurações: what
//! there is, looking now, and Atualizar. The pages never reach the updater
//! plugin itself: no capability grants its commands.

use tauri::AppHandle;

use crate::domain::update::UpdateState;
use crate::windows::update;

/// What Sobre shows: nothing new, a new version, or its download.
#[tauri::command]
pub fn update_status() -> UpdateState {
    update::state()
}

/// Procurar atualizações: looks now and answers what it found, or why it
/// could not look.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateState, String> {
    update::check(&app, false).await
}

/// Atualizar: downloads and installs the new version. The app closes as
/// Sair closes it and the installer opens it again; this answers only when
/// something failed before that.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}
