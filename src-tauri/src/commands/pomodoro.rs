//! The Pomodoro's panel (spec 0017 F2c-2): its timer, and starting,
//! pausing, skipping or starting it over. Only the panel calls these.

use tauri::WebviewWindow;

use crate::domain::widgets::{PomodoroConfig, WidgetId};
use crate::windows::panel;
#[cfg(windows)]
use crate::windows::pomodoro::{self, Action, View};

fn only_panel(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == panel::LABEL {
        Ok(())
    } else {
        Err("only the panel runs the Pomodoro".into())
    }
}

fn checked(widget: &str, config: &PomodoroConfig) -> Result<String, String> {
    let widget = WidgetId::parse(widget).map_err(|_| "unknown widget".to_string())?;
    config
        .validate()
        .map_err(|_| "invalid Pomodoro settings".to_string())?;
    Ok(widget.as_str().to_owned())
}

/// The timer of the widget `widget`, run by `config`.
#[cfg(windows)]
#[tauri::command]
pub fn pomodoro_status(
    window: WebviewWindow,
    widget: String,
    config: PomodoroConfig,
) -> Result<View, String> {
    only_panel(&window)?;
    let widget = checked(&widget, &config)?;
    Ok(pomodoro::view(&widget, &config))
}

/// `action` (`start`, `pause`, `skip` or `reset`) on the timer of the
/// widget `widget`.
#[cfg(windows)]
#[tauri::command]
pub fn pomodoro_act(
    window: WebviewWindow,
    widget: String,
    config: PomodoroConfig,
    action: String,
) -> Result<View, String> {
    only_panel(&window)?;
    let widget = checked(&widget, &config)?;
    let action = Action::parse(&action).ok_or("unknown action")?;
    Ok(pomodoro::act(&widget, &config, action))
}
