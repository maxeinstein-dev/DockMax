//! The notification area in the dock (ADR 0009, Fase E): the icons the host
//! (`windows/tray_host.rs`) collected, for the tray panel, and the clicks on
//! them. The host runs only while the dock holds the Windows taskbar hidden.

use serde::Serialize;
use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::windows::panel;
#[cfg(windows)]
use crate::windows::popup_menu;
#[cfg(windows)]
use crate::windows::tray_host::{self, Button};

/// Event every window receives with the full list of icons when it
/// changes. Must match `TRAY_CHANGED_EVENT` in `src/core/tray.ts`.
pub const CHANGED_EVENT: &str = "tray-changed";

/// An icon of the notification area, as the pages see it. Never the handle
/// of the program's window: a click names the icon by `key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayIcon {
    pub key: String,
    /// The name that survives a restart (`path:<program>#<number>`, as the
    /// settings keep the icons shown in the dock); null when the program's
    /// file can't be read.
    pub id: Option<String>,
    pub tip: String,
    /// A PNG as a `data:` URL, or null when the image could not be read.
    pub image: Option<String>,
    /// The program asked not to show it.
    pub hidden: bool,
    /// Whether it takes clicks (it said which message to send).
    pub clickable: bool,
}

/// The longest `key` a click may name: `hwnd:uid` is far shorter.
const MAX_KEY: usize = 64;

#[cfg(windows)]
fn snapshot() -> Vec<TrayIcon> {
    tray_host::icons()
        .into_iter()
        .map(|icon| TrayIcon {
            clickable: icon.callback != 0,
            id: stable_key(icon.hwnd, icon.uid),
            key: icon.key,
            tip: icon.tip,
            image: icon.png,
            hidden: icon.hidden,
        })
        .collect()
}

#[cfg(not(windows))]
fn snapshot() -> Vec<TrayIcon> {
    Vec::new()
}

/// The icons now, for the dock.
pub fn icons() -> Vec<TrayIcon> {
    snapshot()
}

/// `path:` and the program's file in lowercase, `#` and the icon's number:
/// what names a tray icon across restarts (the PrimoDock's key).
pub fn key_of(path: &str, uid: u32) -> String {
    format!("path:{}#{uid}", path.to_lowercase())
}

#[cfg(windows)]
fn stable_key(hwnd: isize, uid: u32) -> Option<String> {
    let path = crate::windows::openwin::exe_of_window(hwnd)?;
    Some(key_of(&path.to_string_lossy(), uid))
}

/// Starts the host and has every change sent to the pages. Does nothing
/// when it is running.
#[cfg(windows)]
pub fn start(app: &AppHandle) {
    if std::env::var_os("DOCKMAX_TRAY_TRACE").is_some() {
        tray_host::trace(true);
    }
    {
        // When a program's menu, opened from the tray panel, is gone.
        let app = app.clone();
        popup_menu::set_handler(move || {
            let again = app.clone();
            let _ = app.run_on_main_thread(move || panel::after_tray_menu(&again));
        });
    }
    let app = app.clone();
    let started = tray_host::start(move || {
        if let Err(error) = app.emit(CHANGED_EVENT, snapshot()) {
            log::warn!("tray: could not tell the windows: {error}");
        }
        crate::windows::dock::refresh();
    });
    if !started {
        log::warn!("tray: the host did not start");
    }
}

/// Stops the host: programs find the Windows taskbar's tray again.
#[cfg(windows)]
pub fn stop(app: &AppHandle) {
    tray_host::stop();
    let _ = app.emit(CHANGED_EVENT, Vec::<TrayIcon>::new());
}

/// The icons now, for the tray panel when it opens, and for Configurações,
/// which chooses those the dock shows.
#[tauri::command]
pub fn list_tray(window: WebviewWindow) -> Result<Vec<TrayIcon>, String> {
    if window.label() != panel::LABEL && window.label() != SETTINGS_LABEL {
        return Err("only the panel and Configurações list the tray".into());
    }
    Ok(snapshot())
}

/// The Configurações app's window.
const SETTINGS_LABEL: &str = "settings";

/// A click on an icon of the tray panel: sends the program the messages the
/// taskbar would. `key` is one `list_tray` gave; anything else is refused.
#[cfg(windows)]
#[tauri::command]
pub fn tray_click(window: WebviewWindow, key: String, button: Button) -> Result<(), String> {
    if window.label() != panel::LABEL {
        return Err("only the panel clicks the tray".into());
    }
    if key.len() > MAX_KEY {
        return Err("no such icon".into());
    }
    if button == Button::Right {
        // The program opens its own menu, which takes the focus: the panel
        // stays open for it.
        popup_menu::expect_menu();
    }
    tray_host::click(&key, button).map_err(|error| {
        log::warn!("tray: click refused: {error}");
        "could not click the icon".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pages_get_camel_case_and_never_a_window_handle() {
        let icon = TrayIcon {
            key: "1:2".into(),
            id: Some(key_of("C:\\Docker\\Docker.exe", 3)),
            tip: "Docker".into(),
            image: None,
            hidden: false,
            clickable: true,
        };

        let json = serde_json::to_value(&icon).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "key": "1:2",
                "id": "path:c:\\docker\\docker.exe#3",
                "tip": "Docker",
                "image": null,
                "hidden": false,
                "clickable": true,
            })
        );
    }
}
