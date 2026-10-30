//! The logger the app attaches writes Dockyard's lines to the log file in
//! the app's log folder, at the app's level, and the dependencies' only from
//! `Warn` (spec 0002 §2.3 item 5). Built on the mock runtime, with the app's
//! folders in a temporary one: no window, no WebView, nothing of the user's.
//!
//! One test in this file: the logger is global to the process.

use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::utils::config::AppDirectoriesOverride;

/// The crate's name as `log::` targets start with it.
const APP: &str = "dockyard_lib";

#[test]
fn dockyard_lines_reach_the_log_file() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let mut context = mock_context(noop_assets());
    context.config_mut().app.app_directories_override =
        Some(AppDirectoriesOverride::Root(folder.path().to_path_buf()));
    let app = mock_builder().build(context).expect("the mock app builds");

    dockyard_lib::setup_logging(app.handle());
    log::info!(target: &format!("{APP}::windows::settings_window"), "from the app");
    log::info!(target: "tauri::manager", "a dependency, below its level");
    log::warn!(target: "tauri::manager", "a dependency, at its level");
    log::logger().flush();

    let file = folder.path().join("logs").join("dockmax.log");
    let written = std::fs::read_to_string(&file).expect("the log file");
    assert!(written.contains("logging: Dockyard"), "{written}");
    assert!(
        written.contains(&format!(
            "[INFO][{APP}::windows::settings_window] from the app"
        )),
        "{written}"
    );
    assert!(written.contains("a dependency, at its level"), "{written}");
    assert!(!written.contains("below its level"), "{written}");
}
