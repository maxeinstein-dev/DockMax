//! Test build only (feature `e2e`, spec 0003 §2.8). Keeps every run of the
//! E2E suite away from the user's DockMax: its own identifier (so its own
//! single-instance lock, `%APPDATA%` and `%LOCALAPPDATA%` folders and WebView2
//! profile) and a data folder chosen by the test runner for each run.
//!
//! The whole module is compiled out of the normal build, so no released
//! binary reads `DOCKMAX_E2E_DATA_DIR` or embeds the WebDriver server.

use std::ffi::OsString;
use std::path::PathBuf;

use tauri::{Context, Runtime, utils::config::AppDirectoriesOverride};

/// Folder that receives every app directory (config, data, local data, cache
/// and logs) of a test run. The runner creates a new one for each run.
const DATA_DIR_VAR: &str = "DOCKMAX_E2E_DATA_DIR";

/// Ending of the identifier `src-tauri/tauri.e2e.conf.json` sets.
const IDENTIFIER_SUFFIX: &str = ".e2e";

/// Points the app directories of `context` at the run's data folder. Panics,
/// before any window or file exists, when the build or the environment would
/// let a test touch the user's data.
pub fn isolate<R: Runtime>(context: &mut Context<R>) {
    let config = context.config_mut();
    let root = data_root(&config.identifier, std::env::var_os(DATA_DIR_VAR))
        .unwrap_or_else(|reason| panic!("e2e build: {reason}"));
    config.app.app_directories_override = Some(AppDirectoriesOverride::Root(root));
}

/// The data folder for a test build with `identifier`, read from
/// `data_dir` (the value of `DOCKMAX_E2E_DATA_DIR`).
fn data_root(identifier: &str, data_dir: Option<OsString>) -> Result<PathBuf, String> {
    // Built with `--features e2e` but without the e2e config, the test build
    // would share the identifier, and with it the single-instance lock and the
    // WebView2 profile, of the user's DockMax.
    if !identifier.ends_with(IDENTIFIER_SUFFIX) {
        return Err(format!(
            "the identifier `{identifier}` must end in `{IDENTIFIER_SUFFIX}`; build with `--config src-tauri/tauri.e2e.conf.json`"
        ));
    }
    let root = data_dir
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("`{DATA_DIR_VAR}` must name the run's data folder"))?;
    // Tauri resolves a relative override next to the executable, and one
    // starting with `$` as a base directory: neither is the folder the
    // runner created.
    if !root.is_absolute() {
        return Err(format!("`{DATA_DIR_VAR}` must be an absolute path"));
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IDENTIFIER: &str = "dev.maxsueleinstein.dockmax.e2e";

    fn absolute_dir() -> PathBuf {
        std::env::temp_dir().join("dockmax-e2e-test")
    }

    #[test]
    fn uses_the_folder_from_the_environment() {
        let dir = absolute_dir();
        assert_eq!(
            data_root(TEST_IDENTIFIER, Some(dir.clone().into_os_string())),
            Ok(dir)
        );
    }

    #[test]
    fn refuses_the_identifier_of_the_real_app() {
        let error = data_root(
            "dev.maxsueleinstein.dockmax",
            Some(absolute_dir().into_os_string()),
        )
        .unwrap_err();
        assert!(error.contains("tauri.e2e.conf.json"), "{error}");
    }

    #[test]
    fn refuses_a_missing_or_empty_folder() {
        for value in [None, Some(OsString::new())] {
            let error = data_root(TEST_IDENTIFIER, value).unwrap_err();
            assert!(error.contains(DATA_DIR_VAR), "{error}");
        }
    }

    #[test]
    fn refuses_a_relative_or_variable_folder() {
        for value in ["dockmax-e2e", "$TEMP/dockmax-e2e"] {
            let error = data_root(TEST_IDENTIFIER, Some(value.into())).unwrap_err();
            assert!(error.contains("absolute"), "{value}: {error}");
        }
    }

    #[test]
    fn the_e2e_config_sets_a_test_identifier() {
        // Pins the file the test build is made with to the check above.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.e2e.conf.json");
        let text = std::fs::read_to_string(&path).expect("tauri.e2e.conf.json must exist");
        let config: serde_json::Value =
            serde_json::from_str(&text).expect("tauri.e2e.conf.json must be valid JSON");
        assert_eq!(
            config.get("identifier").and_then(serde_json::Value::as_str),
            Some(TEST_IDENTIFIER)
        );
    }
}
