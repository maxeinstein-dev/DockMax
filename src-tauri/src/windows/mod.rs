#[cfg(windows)]
pub mod appbar;
pub mod apps;
pub mod autostart;
pub mod dock;
#[cfg(windows)]
pub mod drop_target;
#[cfg(windows)]
pub mod flyout;
pub mod hit_test;
pub mod jumplist;
pub mod memory;
#[cfg(windows)]
mod native;
#[cfg(windows)]
pub mod open_apps;
#[cfg(windows)]
pub mod openwin;
pub mod panel;
#[cfg(windows)]
pub mod popup_menu;
#[cfg(windows)]
pub mod profiles;
#[cfg(windows)]
pub mod taskbar;
#[cfg(windows)]
pub mod tray_host;
