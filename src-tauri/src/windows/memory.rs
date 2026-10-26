//! How much memory each window's WebView2 aims to use (spec 0003 §2.12).
//! A window at rest asks for the low target, which lets WebView2 drop
//! caches and trim the working set of its processes; it asks for the normal
//! target again before it is used:
//!
//! - the panel is at rest while hidden: normal before its page renders a
//!   panel, low once it is hidden (`panel::apply`);
//! - the dock is at rest while the cursor is off it: normal while it
//!   captures the mouse (cursor on the bar or a magnified icon), low once it
//!   lets the mouse through, and applied again after each layout its page
//!   reports, the first at startup included (`hit_test`). Its tiles still
//!   change once a minute at rest.
//!
//! Once every WebView2 of the app is low, the shared browser, GPU and
//! utility processes may be trimmed too. Measured (spec 0003 §2.12): the
//! renderer's private working set at rest falls from 15 to 3 MB every
//! time, the shared processes' in part of the runs; the committed memory
//! does not change, and the first interaction after a rest costs about
//! 10 ms more while the pages come back.
//!
//! The two windows also share one renderer process (`--process-per-site`
//! in `tauri.conf.json`, pinned by the test below): both pages come from
//! the same origin, and the second renderer cost 20 to 30 MB.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow};

use super::{dock, hit_test, panel};

/// How long after a page loads its window is taken to be at rest, with its
/// first render done, and the target is applied again (`page_loaded`).
const SETTLE_DELAY: Duration = Duration::from_secs(3);

/// The WebView2 memory usage target level (`ICoreWebView2_19`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTarget {
    Normal,
    Low,
}

impl MemoryTarget {
    /// Normal while the window is in use, low at rest.
    pub fn in_use(in_use: bool) -> Self {
        if in_use { Self::Normal } else { Self::Low }
    }
}

/// Asks the window's WebView2 for `target`, on the main thread. Best
/// effort: a WebView2 runtime without `ICoreWebView2_19` keeps its level,
/// and a failure is only logged. WebView2 trims the processes when the
/// level changes to low, so a window loads its page at the normal level
/// (the default) and asks for low only once rendered (`page_loaded`):
/// asked for low from the start, or for normal and low back to back, it
/// kept most of its working set (measured).
pub fn set(window: &WebviewWindow, target: MemoryTarget) {
    #[cfg(windows)]
    {
        let label = window.label().to_owned();
        let scheduled = window.with_webview(move |webview| {
            if let Err(error) = apply(&webview, target) {
                log::debug!("{label}: memory target {target:?} not applied: {error}");
            }
        });
        if let Err(error) = scheduled {
            log::debug!("memory target {target:?} not scheduled: {error}");
        }
    }
    #[cfg(not(windows))]
    let _ = (window, target);
}

/// A window's page finished loading: once its first render has settled,
/// applies the target, so a window nobody touches after startup is at rest
/// too. The dock's is applied by its cursor thread, which knows whether it
/// holds the mouse; the panel is at rest unless a panel is open.
pub fn page_loaded(app: &AppHandle, label: &str) {
    let app = app.clone();
    let label = label.to_owned();
    let name = format!("{label}-memory-settle");
    let spawned = thread::Builder::new().name(name).spawn(move || {
        thread::sleep(SETTLE_DELAY);
        match label.as_str() {
            dock::LABEL => hit_test::tracker(&app).settle(),
            panel::LABEL => {
                let at_rest = panel::lock(&app).open().is_none();
                if let (true, Some(window)) = (at_rest, app.get_webview_window(&label)) {
                    set(&window, MemoryTarget::Low);
                }
            }
            _ => {}
        }
    });
    if let Err(error) = spawned {
        log::debug!("memory settle not scheduled: {error}");
    }
}

#[cfg(windows)]
fn apply(
    webview: &tauri::webview::PlatformWebview,
    target: MemoryTarget,
) -> windows::core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        ICoreWebView2_19,
    };
    use windows::core::Interface;

    let level = match target {
        MemoryTarget::Normal => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        MemoryTarget::Low => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    };
    // SAFETY: `with_webview` runs this on the main thread, where the
    // controller lives, and keeps the webview alive for the call.
    unsafe {
        webview
            .controller()
            .CoreWebView2()?
            .cast::<ICoreWebView2_19>()?
            .SetMemoryUsageTargetLevel(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_in_use_is_normal_and_one_at_rest_is_low() {
        assert_eq!(MemoryTarget::in_use(true), MemoryTarget::Normal);
        assert_eq!(MemoryTarget::in_use(false), MemoryTarget::Low);
    }

    /// The browser arguments of each window in `tauri.conf.json`.
    fn browser_args() -> Vec<(String, Option<String>)> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = std::fs::read_to_string(path).expect("tauri.conf.json must exist");
        let config: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        config["app"]["windows"]
            .as_array()
            .expect("windows")
            .iter()
            .map(|window| {
                (
                    window["label"].as_str().unwrap_or_default().to_owned(),
                    window["additionalBrowserArgs"].as_str().map(str::to_owned),
                )
            })
            .collect()
    }

    #[test]
    fn every_window_shares_one_renderer_with_wrys_default_arguments() {
        // The dock is the only window of `tauri.conf.json`; the panel and the
        // Configurações app are made on demand with the same arguments
        // (spec 0011). One WebView2 environment for all: arguments that
        // differ would need another data folder, and another browser process.
        let windows = browser_args();
        assert_eq!(windows.len(), 1, "{windows:?}");
        assert_eq!(
            windows[0].1.as_deref(),
            Some(super::super::BROWSER_ARGS),
            "{windows:?}"
        );
        let args = super::super::BROWSER_ARGS;
        // Setting them replaces wry's defaults, which must stay.
        assert!(
            args.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"),
            "{args}"
        );
        // The dock and the panel, one site, in one renderer process.
        assert!(
            args.split(' ').any(|arg| arg == "--process-per-site"),
            "{args}"
        );
        // The GPU work in the browser process (ADR 0010), with the restart
        // of `windows/recovery.rs` for when it fails.
        assert!(
            args.split(' ').any(|arg| arg == "--in-process-gpu"),
            "{args}"
        );
    }
}
