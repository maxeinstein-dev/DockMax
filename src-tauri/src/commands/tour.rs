//! The welcome tour (spec 0019 G2): it opens by itself on the first run,
//! and again from "Ver o tour" in Sobre.

use tauri::{AppHandle, WebviewWindow};

use crate::windows::{settings_window, tour_window};

/// Opens the tour if it was never shown, and marks it shown: closing it
/// half way doesn't bring it back.
pub fn first_run(app: &AppHandle) {
    if super::settings::current(app).tour_shown {
        return;
    }
    if let Err(error) = tour_window::open(app) {
        log::warn!("tour: the window could not be opened: {error}");
        return;
    }
    if let Err(error) = super::settings::change(app, |settings| settings.tour_shown = true) {
        log::warn!("tour: shown, but not marked: {error}");
    }
}

/// "Ver o tour" in Sobre, from the panel or Configurações.
#[tauri::command]
pub fn open_tour(app: AppHandle) -> Result<(), String> {
    tour_window::open(&app).map_err(|error| {
        log::warn!("tour: the window could not be opened: {error}");
        "the tour could not be opened".to_string()
    })
}

/// The tour's last button: closes it and, with `settings`, opens
/// Configurações. Only the tour's window calls this.
#[tauri::command]
pub fn finish_tour(app: AppHandle, window: WebviewWindow, settings: bool) -> Result<(), String> {
    if window.label() != tour_window::LABEL {
        return Err("only the tour finishes itself".into());
    }
    if settings && let Err(error) = settings_window::open(&app) {
        log::warn!("tour: Configurações could not be opened: {error}");
    }
    window
        .close()
        .map_err(|_| "the tour could not be closed".to_string())
}
