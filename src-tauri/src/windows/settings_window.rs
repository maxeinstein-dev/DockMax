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

/// Event sent to the window when Alt+Space (with any other modifiers) is
/// pressed while the launcher's hotkey is captured: Windows turns it into the
/// window's menu before the page sees it (spec 0012 L3).
pub const HOTKEY_KEY_EVENT: &str = "launcher-hotkey-pressed";

/// The subclass that hands Alt+Space to the capture.
const SUBCLASS_ID: usize = 0x4C33;

static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

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
    let _ = APP.set(app.clone());
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
        section.map(|section| format!("window.__DOCKYARD_SECTION__ = \"{}\";", section.as_str()));
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
            match builder.build() {
                // The subclass on the window's own thread.
                Ok(window) => {
                    let installed = app.run_on_main_thread(move || {
                        if let Err(error) = install(&window) {
                            log::warn!("settings: Alt+Space is not handed to the capture: {error}");
                        }
                    });
                    if let Err(error) = installed {
                        log::warn!("settings: the window setup was not scheduled: {error}");
                    }
                }
                Err(error) => log::warn!("settings: the window could not be made: {error}"),
            }
        })?;
    Ok(())
}

/// Hands Alt+Space to the hotkey's capture instead of opening the window's
/// menu, while the capture runs.
#[cfg(windows)]
fn install(window: &tauri::WebviewWindow) -> Result<(), Box<dyn Error>> {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let hwnd = window.hwnd()?;
    if unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0) }.as_bool() {
        Ok(())
    } else {
        Err("failed to subclass the window".into())
    }
}

#[cfg(not(windows))]
fn install(_window: &tauri::WebviewWindow) -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Alt+Space as the capture writes it, with the other modifiers down now.
#[cfg(windows)]
fn alt_space_now() -> String {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_RWIN, VK_SHIFT,
    };
    let down = |key: VIRTUAL_KEY| unsafe { GetKeyState(i32::from(key.0)) } < 0;
    let mut text = String::new();
    if down(VK_CONTROL) {
        text.push_str("Ctrl+");
    }
    text.push_str("Alt+");
    if down(VK_SHIFT) {
        text.push_str("Shift+");
    }
    if down(VK_LWIN) || down(VK_RWIN) {
        text.push_str("Win+");
    }
    text.push_str("Space");
    text
}

#[cfg(windows)]
unsafe extern "system" fn subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{SC_KEYMENU, WM_NCDESTROY, WM_SYSCOMMAND};
    // The window's menu by the keyboard: Alt+Space, its character a space.
    let menu_by_space = message == WM_SYSCOMMAND
        && (wparam.0 & 0xFFF0) == SC_KEYMENU as usize
        && lparam.0 == isize::from(b' ');
    if menu_by_space && super::launcher::capturing() {
        if let Some(app) = APP.get()
            && let Err(error) = app.emit_to(LABEL, HOTKEY_KEY_EVENT, alt_space_now())
        {
            log::warn!("settings: Alt+Space did not reach the capture: {error}");
        }
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        let _ = unsafe { RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID) };
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
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
