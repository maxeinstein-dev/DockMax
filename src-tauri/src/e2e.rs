//! Test build only (feature `e2e`, spec 0003 §2.8). Keeps every run of the
//! E2E suite away from the user's Dockyard: its own identifier (so its own
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
    // WebView2 profile, of the user's Dockyard.
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

/// One item of the dock as the suite sees it.
#[derive(Debug, serde::Serialize)]
pub struct DockItemView {
    pub name: String,
    /// What its face draws: `app`, `glyph`, `time`, `dial` or `ring`.
    pub face: &'static str,
}

/// The native dock as the suite sees it (spec 0011 H3): it has no page to
/// look into, so the hidden `e2e` window asks for this.
#[derive(Debug, serde::Serialize)]
pub struct DockView {
    pub items: Vec<DockItemView>,
    pub dark: bool,
}

/// What the dock shows now.
#[tauri::command]
pub async fn e2e_dock_view() -> Result<DockView, String> {
    let view = crate::windows::native_dock::view().ok_or("the dock did not answer")?;
    Ok(DockView {
        items: view
            .items
            .into_iter()
            .map(|(name, face)| DockItemView { name, face })
            .collect(),
        dark: view.dark,
    })
}

/// Presses the dock's item named `name`, as a click would.
#[tauri::command]
pub async fn e2e_press_dock(name: String) -> Result<(), String> {
    if crate::windows::native_dock::press_named(name) {
        Ok(())
    } else {
        Err("no such item on the dock".into())
    }
}

/// What the launcher shows now (spec 0012 L1).
#[tauri::command]
pub async fn e2e_launcher_view() -> Result<crate::windows::launcher::View, String> {
    crate::windows::launcher::view().ok_or_else(|| "the launcher did not answer".into())
}

/// Opens the launcher, or closes it when open, as Alt+Space does.
#[tauri::command]
pub async fn e2e_launcher_toggle() {
    crate::windows::launcher::toggle();
}

/// Types `text` into the launcher's box.
#[tauri::command]
pub async fn e2e_launcher_type(text: String) {
    crate::windows::launcher::type_text(text);
}

/// Presses `up`, `down`, `escape`, `tab` or `backspace` in the launcher's
/// box.
#[tauri::command]
pub async fn e2e_launcher_key(key: String) -> Result<(), String> {
    if crate::windows::launcher::press_key(key) {
        Ok(())
    } else {
        Err("no such key, or no launcher".into())
    }
}

/// The private memory of the app's own process, in bytes: the native dock,
/// the launcher and everything else in Rust, without the WebView2 processes
/// (M1 of study 0002). What the suite's memory guard reads.
#[tauri::command]
pub async fn e2e_memory() -> Result<u64, String> {
    private_bytes().ok_or_else(|| "the process memory could not be read".into())
}

#[cfg(windows)]
fn private_bytes() -> Option<u64> {
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged_pool: usize,
        quota_paged_pool: usize,
        quota_peak_non_paged_pool: usize,
        quota_non_paged_pool: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, size: u32) -> i32;
    }

    let mut counters = Counters {
        cb: size_of::<Counters>() as u32,
        ..Default::default()
    };
    // SAFETY: `counters` is a `PROCESS_MEMORY_COUNTERS_EX` of the size `cb`
    // says, and the pseudo handle of the current process needs no closing.
    let read = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    (read != 0).then_some(counters.private_usage as u64)
}

#[cfg(not(windows))]
fn private_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IDENTIFIER: &str = "dev.maxsueleinstein.dockmax.e2e";

    fn absolute_dir() -> PathBuf {
        std::env::temp_dir().join("dockyard-e2e-test")
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
        for value in ["dockyard-e2e", "$TEMP/dockyard-e2e"] {
            let error = data_root(TEST_IDENTIFIER, Some(value.into())).unwrap_err();
            assert!(error.contains("absolute"), "{value}: {error}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn reads_the_private_memory_of_the_process() {
        let before = private_bytes().expect("the memory must be readable");
        let block = vec![1u8; 32 << 20];
        let after = private_bytes().expect("the memory must be readable");
        assert!(after >= before + (16 << 20), "{before} -> {after}");
        drop(block);
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
