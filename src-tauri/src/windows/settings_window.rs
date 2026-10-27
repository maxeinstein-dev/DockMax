//! The Configurações app (spec 0011 H1): an ordinary window of its own, made
//! when the user opens the settings and destroyed when it is closed, so
//! nothing of it stays loaded the rest of the day. The dock's Configurações
//! item and the tray's "Configurações" open it; opening it again brings the
//! one that is open to the front.

use std::error::Error;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::launcher_pages::Section;

/// The window's label: the capability `settings` is granted to it.
pub const LABEL: &str = "settings";

/// Event sent to the open window to show a section (the launcher's pages).
pub const SECTION_EVENT: &str = "settings-section";

/// Opens the Configurações window, or brings it to the front if it is open.
pub fn open(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    open_at(app, None)
}

/// Opens the Configurações window on `section` (a page the launcher found),
/// or brings it to the front showing it.
pub fn open_section(app: &AppHandle, section: Section) -> Result<(), Box<dyn Error>> {
    open_at(app, Some(section))
}

fn open_at(app: &AppHandle, section: Option<Section>) -> Result<(), Box<dyn Error>> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
        if let Some(section) = section {
            app.emit_to(LABEL, SECTION_EVENT, section.as_str())?;
        }
        return Ok(());
    }
    // A new window starts on the section: its name, from a closed list, is
    // set before the page's scripts run.
    let start =
        section.map(|section| format!("window.__DOCKMAX_SECTION__ = \"{}\";", section.as_str()));
    // Made off the main thread: the callers (a command, the tray menu) run on
    // it, and building a webview window there waits on the main thread
    // itself, which never answers.
    let app = app.clone();
    std::thread::Builder::new()
        .name("settings-window".into())
        .spawn(move || {
            let title = crate::i18n::messages().settings.window_title;
            let mut builder =
                WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("settings.html".into()))
                    .title(title)
                    .inner_size(880.0, 640.0)
                    .min_inner_size(720.0, 480.0)
                    .center()
                    .resizable(true)
                    .additional_browser_args(super::BROWSER_ARGS);
            if let Some(start) = &start {
                builder = builder.initialization_script(start);
            }
            let built = builder.build();
            if let Err(error) = built {
                log::warn!("settings: the window could not be made: {error}");
            }
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_settings_window_shares_the_browser_arguments_of_the_others() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        for window in config["app"]["windows"].as_array().unwrap() {
            assert_eq!(
                window["additionalBrowserArgs"].as_str(),
                Some(crate::windows::BROWSER_ARGS),
                "{}",
                window["label"]
            );
        }
    }
}
