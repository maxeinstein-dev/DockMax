#[cfg(windows)]
pub mod appbar;
pub mod apps;
pub mod autostart;
#[cfg(windows)]
pub mod badges;
pub mod dock;
#[cfg(windows)]
pub mod drop_target;
#[cfg(windows)]
pub mod flyout;
#[cfg(windows)]
pub mod fullscreen;
pub mod jumplist;
#[cfg(windows)]
pub mod launcher;
pub mod memory;
#[cfg(windows)]
pub mod menu;
#[cfg(windows)]
pub mod monitors;
#[cfg(windows)]
mod native;
#[cfg(windows)]
pub mod native_dock;
#[cfg(windows)]
pub mod open_apps;
#[cfg(windows)]
pub mod openwin;
pub mod panel;
pub mod panel_window;
#[cfg(windows)]
pub mod popup_menu;
#[cfg(windows)]
pub mod previews;
#[cfg(windows)]
pub mod profiles;
#[cfg(windows)]
pub mod recovery;
pub mod settings_window;
#[cfg(windows)]
pub mod system_status;
#[cfg(windows)]
pub mod taskbar;
#[cfg(windows)]
pub mod tray_host;
#[cfg(windows)]
pub mod window_spaces;

/// The WebView2 arguments every window of the app shares (they must be the
/// same in all of them, or the windows can't share the browser process):
/// the panel's and the settings', both made on demand (ADR 0006, ADR 0010,
/// spec 0011). The dock has no WebView2.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --process-per-site --in-process-gpu";
