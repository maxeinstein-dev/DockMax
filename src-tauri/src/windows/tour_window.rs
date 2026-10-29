//! The welcome tour's window (spec 0019 G2): small, centered, made when it
//! opens and destroyed when it closes. Opening it again brings the open one
//! to the front.

use std::error::Error;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// The window's label: the capability `tour` is granted to it.
pub const LABEL: &str = "tour";

/// Opens the tour, or brings it to the front.
pub fn open(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    // Made off the main thread, as Configurações is: building a webview
    // window there waits on the main thread itself.
    let app = app.clone();
    std::thread::Builder::new()
        .name("tour-window".into())
        .spawn(move || {
            let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("tour.html".into()))
                .title(crate::i18n::messages().settings.tour_title)
                .inner_size(560.0, 480.0)
                .center()
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .additional_browser_args(super::BROWSER_ARGS)
                .build();
            if let Err(error) = built {
                log::warn!("tour: the window could not be made: {error}");
            }
        })?;
    Ok(())
}
