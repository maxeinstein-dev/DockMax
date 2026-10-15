//! Logs of the Rust side and of both webviews, in one rotating file in the
//! app's log folder (`%LOCALAPPDATA%\<identifier>\logs` on Windows), plus
//! stdout in debug builds (spec 0002 §2.3 item 5).
//!
//! Messages name the capability affected and the kind of error only: never
//! a file path, settings content, argv or a command payload.

use log::LevelFilter;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_log::{
    Builder, RotationStrategy, Target, TargetKind, TimezoneStrategy, WEBVIEW_TARGET,
};

/// Log file name, without the `.log` extension.
const FILE_NAME: &str = "dockmax";

/// Size at which the log file is rotated, in bytes.
const MAX_FILE_SIZE: u128 = 1024 * 1024;

/// Log files kept, the active one included: at most ~5 MB on disk.
const KEPT_FILES: usize = 5;

/// Level of DockMax's own messages, from Rust and from the webviews.
const APP_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};

/// Level of the dependencies (Tauri, wry, tao...). Kept at `Warn`: their
/// messages are not reviewed for paths or payloads.
const DEPENDENCY_LEVEL: LevelFilter = LevelFilter::Warn;

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let mut builder = Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir {
            file_name: Some(FILE_NAME.into()),
        }))
        .max_file_size(MAX_FILE_SIZE)
        .rotation_strategy(RotationStrategy::KeepSome(KEPT_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(DEPENDENCY_LEVEL)
        .level_for(env!("CARGO_CRATE_NAME"), APP_LEVEL)
        .level_for(WEBVIEW_TARGET, APP_LEVEL);
    if cfg!(debug_assertions) {
        builder = builder.target(Target::new(TargetKind::Stdout));
    }
    builder.build()
}
