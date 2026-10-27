//! What the system widgets' panels show (spec 0017 F1): the processor, the
//! memory and the network, read while the panel asks. The dock reads the
//! same sampler on its own timer.

use tauri::WebviewWindow;

use crate::domain::system_activity::Reading;
use crate::windows::panel;

/// The reading now, for the panel of an Atividade or an Atividade de rede.
/// The panel asks every 2 s while it is open; a second ask within 1,5 s
/// gets the same reading without reading again.
#[cfg(windows)]
#[tauri::command]
pub fn read_system_activity(window: WebviewWindow) -> Result<Reading, String> {
    if window.label() != panel::LABEL {
        return Err("only the panel reads the system activity".into());
    }
    Ok(crate::windows::system_activity::sample())
}
