//! The Tocando agora's panel (spec 0017 F4): what plays now, and play or
//! pause, the previous and the next. Only the panel calls these.

use serde::Serialize;
use tauri::WebviewWindow;

use crate::windows::panel;

fn only_panel(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == panel::LABEL {
        Ok(())
    } else {
        Err("only the panel reads what plays".into())
    }
}

/// What the panel shows; `None` fields when nothing plays.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaView {
    pub title: String,
    pub artist: String,
    /// The program's name, as the Start menu shows it.
    pub app: String,
    pub playing: bool,
    pub can_toggle: bool,
    pub can_previous: bool,
    pub can_next: bool,
    /// The cover as a `data:` URL.
    pub art: Option<String>,
}

/// The name of the program `aumid`: the Start menu's, or the file's.
#[cfg(windows)]
fn app_name(aumid: &str) -> String {
    crate::windows::apps::name_of(aumid).unwrap_or_else(|| {
        let file = aumid.rsplit(['\\', '/', '!']).next().unwrap_or(aumid);
        file.trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .to_owned()
    })
}

/// What plays now; `null` when nothing does.
#[cfg(windows)]
#[tauri::command]
pub fn media_status(window: WebviewWindow) -> Result<Option<MediaView>, String> {
    use base64::Engine;
    only_panel(&window)?;
    Ok(crate::windows::media::snapshot().map(|now| MediaView {
        app: app_name(&now.app),
        art: now.art.as_ref().map(|art| {
            format!(
                "data:{};base64,{}",
                art.0,
                base64::engine::general_purpose::STANDARD.encode(&art.1)
            )
        }),
        title: now.title,
        artist: now.artist,
        playing: now.playing,
        can_toggle: now.can_toggle,
        can_previous: now.can_previous,
        can_next: now.can_next,
    }))
}

/// `action` (`toggle`, `previous` or `next`) on what plays now.
#[cfg(windows)]
#[tauri::command]
pub fn media_control(window: WebviewWindow, action: String) -> Result<(), String> {
    only_panel(&window)?;
    let action = crate::windows::media::Action::parse(&action).ok_or("unknown action")?;
    crate::windows::media::control(action);
    Ok(())
}
