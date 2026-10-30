//! The Configurações app (spec 0011 H1): an ordinary window of its own, made
//! when the user opens the settings and destroyed when it is closed, so
//! nothing of it stays loaded the rest of the day. The dock's Configurações
//! item and the tray's "Configurações" open it; opening it again brings the
//! one that is open to the front, and so does a click while it is still
//! being made (`Opening`): one window, never two.
//!
//! The first WebView2 of the process starts cold (400-700 ms on the
//! author's machine, more on a busy one): the window shows at once in the
//! theme's surface color, takes the focus when it is done, and the pointer
//! resting on Configurações in the menu warms WebView2 (`warm`).

use std::error::Error;

use tauri::webview::{Color, PageLoadEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::launcher_pages::Section;
use crate::domain::settings::Theme;

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

/// The pointer rests on Configurações in the dock's (or the tray's) menu:
/// starts WebView2 now, so the click finds it warm (~110 ms to the page
/// instead of 400-700 ms cold, spec 0011 §2). It reuses the panel's warm-up
/// (H2): the hidden panel window, destroyed after `KEEP_WARM` unused, so
/// nothing stays loaded the rest of the day. Nothing to do when Configurações
/// is open or being made: WebView2 is already up.
pub fn warm(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() || opening().making {
        return;
    }
    super::panel_window::warm(app);
}

/// A window being made, and what was asked of it meanwhile (spec 0011 H1):
/// the window is made off the main thread and only exists for
/// `get_webview_window` once it is done, so a second click during a slow
/// first start would make a second window.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Opening {
    /// A window is being made.
    making: bool,
    /// The section the window starts on (its initialization script).
    start: Option<Section>,
    /// The last section asked for, by the first request or a later one.
    latest: Option<Section>,
    /// Asked again while it was being made.
    again: bool,
}

/// What a request to open Configurações does, when no window is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Makes the window, starting on this section.
    Make(Option<Section>),
    /// One is being made: it shows up for this request too when done.
    Wait,
}

/// What is left to do once the window is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ready {
    /// A section asked for while it was being made, other than the one it
    /// started on: sent to the page once it has loaded.
    section: Option<Section>,
    /// It was asked for again while being made.
    again: bool,
}

impl Opening {
    /// A request for the window, on `section` (`None`: as it is).
    fn ask(&mut self, section: Option<Section>) -> Step {
        if self.making {
            // A plain open keeps the section a page search asked for.
            if section.is_some() {
                self.latest = section;
            }
            self.again = true;
            return Step::Wait;
        }
        *self = Opening {
            making: true,
            start: section,
            latest: section,
            again: false,
        };
        Step::Make(section)
    }

    /// The making ended, made or failed: the next request makes a window
    /// again (the window, if made, is found by its label from now on).
    fn finish(&mut self) -> Ready {
        let ready = Ready {
            section: self.latest.filter(|&latest| Some(latest) != self.start),
            again: self.again,
        };
        *self = Opening::default();
        ready
    }
}

static OPENING: std::sync::Mutex<Opening> = std::sync::Mutex::new(Opening {
    making: false,
    start: None,
    latest: None,
    again: false,
});

fn opening() -> std::sync::MutexGuard<'static, Opening> {
    OPENING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The window's background before its page paints: the surface of the
/// theme (`--surface-rgb` in `src/themes/tokens.css`), so a cold start shows
/// the window at once in the app's colors instead of a white square.
fn background(theme: Theme, system_dark: bool) -> (u8, u8, u8) {
    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => system_dark,
    };
    if dark { (38, 39, 44) } else { (247, 247, 249) }
}

fn system_dark() -> bool {
    #[cfg(windows)]
    {
        super::native_dock::system_dark()
    }
    #[cfg(not(windows))]
    false
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
    let Step::Make(section) = opening().ask(section) else {
        return Ok(());
    };
    // Nothing left over from a window closed before its page loaded.
    *PENDING_SECTION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    // A new window starts on the section: its name, from a closed list, is
    // set before the page's scripts run.
    let start =
        section.map(|section| format!("window.__DOCKYARD_SECTION__ = \"{}\";", section.as_str()));
    let (red, green, blue) =
        background(crate::commands::settings::current(app).theme, system_dark());
    // Made off the main thread: the callers (a command, the tray menu) run on
    // it, and building a webview window there waits on the main thread
    // itself, which never answers.
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("settings-window".into())
        .spawn(move || {
            let asked = std::time::Instant::now();
            let title = crate::i18n::messages().settings.window_title;
            let mut builder =
                WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("settings.html".into()))
                    .title(title)
                    .inner_size(880.0, 640.0)
                    .min_inner_size(720.0, 480.0)
                    .center()
                    .resizable(true)
                    .background_color(Color(red, green, blue, 255))
                    .additional_browser_args(super::BROWSER_ARGS)
                    // A section asked for while the window was being made
                    // reaches the page once it has loaded and listens.
                    .on_page_load(|webview, payload| {
                        if payload.event() != PageLoadEvent::Finished {
                            return;
                        }
                        let Some(section) = PENDING_SECTION
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .take()
                        else {
                            return;
                        };
                        if let Err(error) = webview.emit_to(LABEL, SECTION_EVENT, section.as_str())
                        {
                            log::warn!(
                                "settings: the section asked for did not reach the page: {error}"
                            );
                        }
                    });
            if let Some(start) = &start {
                builder = builder.initialization_script(start);
            }
            let built = builder.build();
            let ready = opening().finish();
            match built {
                Ok(window) => {
                    log::info!(
                        "settings: window made in {} ms{}",
                        asked.elapsed().as_millis(),
                        if ready.again {
                            ", asked again meanwhile"
                        } else {
                            ""
                        }
                    );
                    if let Some(section) = ready.section {
                        *PENDING_SECTION
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(section);
                    }
                    // In front, even when making it took long enough for
                    // the click's right to the foreground to lapse.
                    if let Err(error) = window.set_focus() {
                        log::warn!("settings: the window could not take the focus: {error}");
                    }
                    // The subclass on the window's own thread.
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
        });
    if let Err(error) = spawned {
        opening().finish();
        return Err(error.into());
    }
    Ok(())
}

/// A section asked for while the window was being made, for its page once
/// loaded.
static PENDING_SECTION: std::sync::Mutex<Option<Section>> = std::sync::Mutex::new(None);

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
    use super::*;

    #[test]
    fn the_first_request_makes_the_window_on_its_section() {
        let mut opening = Opening::default();

        assert_eq!(
            opening.ask(Some(Section::Look)),
            Step::Make(Some(Section::Look))
        );
        assert!(opening.making);
    }

    #[test]
    fn a_second_click_while_it_is_being_made_makes_no_second_window() {
        let mut opening = Opening::default();
        assert_eq!(opening.ask(None), Step::Make(None));

        assert_eq!(opening.ask(None), Step::Wait);
        assert_eq!(opening.ask(None), Step::Wait);
        assert_eq!(
            opening.finish(),
            Ready {
                section: None,
                again: true
            }
        );
    }

    #[test]
    fn the_window_ends_on_the_last_section_asked_for() {
        let mut opening = Opening::default();
        opening.ask(None);
        opening.ask(Some(Section::Items));
        opening.ask(Some(Section::About));
        // A plain open (the dock's menu) keeps the page search's section.
        opening.ask(None);

        assert_eq!(opening.finish().section, Some(Section::About));
    }

    #[test]
    fn the_section_it_started_on_is_not_sent_again() {
        let mut opening = Opening::default();
        opening.ask(Some(Section::Apps));
        opening.ask(Some(Section::Apps));

        assert_eq!(opening.finish().section, None);
    }

    #[test]
    fn a_window_made_alone_needs_nothing_more() {
        let mut opening = Opening::default();
        opening.ask(Some(Section::General));

        assert_eq!(
            opening.finish(),
            Ready {
                section: None,
                again: false
            }
        );
    }

    #[test]
    fn after_the_making_ends_made_or_failed_the_next_request_makes_one_again() {
        let mut opening = Opening::default();
        opening.ask(None);
        opening.ask(Some(Section::Look));
        opening.finish();

        assert_eq!(opening, Opening::default());
        assert_eq!(opening.ask(None), Step::Make(None));
    }

    #[test]
    fn the_background_is_the_surface_of_the_theme() {
        let dark = (38, 39, 44);
        let light = (247, 247, 249);
        assert_eq!(background(Theme::Dark, false), dark);
        assert_eq!(background(Theme::Light, true), light);
        assert_eq!(background(Theme::System, true), dark);
        assert_eq!(background(Theme::System, false), light);
        // The same colors as the page's surface, light and dark.
        let tokens = include_str!("../../../src/themes/tokens.css");
        for (red, green, blue) in [dark, light] {
            let surface = format!("--surface-rgb: {red} {green} {blue};");
            assert!(tokens.contains(&surface), "{surface}");
        }
    }

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
