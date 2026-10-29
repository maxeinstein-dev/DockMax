//! The Clima's panel (spec 0017 F5): the last forecast of a city, and the
//! search for one by its name. Only the panel calls these.

use tauri::WebviewWindow;

use crate::domain::weather::Place;
use crate::windows::panel;

fn only_panel(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == panel::LABEL {
        Ok(())
    } else {
        Err("only the panel reads the weather".into())
    }
}

/// The last forecast at the city (`null` before the first); `fresh`: read
/// it again if it is not recent (the panel just opened), the answer coming
/// a moment later.
#[cfg(windows)]
#[tauri::command]
pub fn weather_reading(
    window: WebviewWindow,
    latitude: f64,
    longitude: f64,
    fresh: bool,
) -> Result<Option<crate::windows::weather::Reading>, String> {
    only_panel(&window)?;
    if fresh {
        crate::windows::weather::read_now();
    }
    Ok(crate::windows::weather::reading(
        crate::windows::weather::key(latitude, longitude),
    ))
}

/// The cities named like `query`.
#[cfg(windows)]
#[tauri::command]
pub async fn weather_search(window: WebviewWindow, query: String) -> Result<Vec<Place>, String> {
    only_panel(&window)?;
    tauri::async_runtime::spawn_blocking(move || crate::windows::weather::search(&query))
        .await
        .map_err(|_| "the search stopped".to_string())?
        .map_err(|()| "no answer".to_string())
}
