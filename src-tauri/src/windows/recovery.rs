//! Coming back by itself when the WebView2 browser process dies (ADR 0010).
//!
//! The GPU work runs inside the browser process (`--in-process-gpu`), which
//! saves a process and tens of megabytes; the price is that a failing video
//! driver takes the browser process down, and both windows with it. Then the
//! DockMax gives the taskbar back the way it does on exit and starts again.
//! Once: a second failure within `QUIET` of the first leaves it closed, so a
//! driver that keeps failing does not make it blink forever (the taskbar is
//! back either way).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager, WebviewWindow};

/// The least time between two restarts.
const QUIET: Duration = Duration::from_secs(120);

/// The file that keeps when it last restarted, in the local data folder.
const LAST_RESTART: &str = "last-recovery";

/// Whether the browser process is being watched (one window is enough: they
/// share it).
static WATCHING: AtomicBool = AtomicBool::new(false);

/// Whether it may restart now: the last restart, if any, is older than
/// `QUIET`.
fn may_restart(app: &AppHandle) -> bool {
    let Ok(dir) = app.path().app_local_data_dir() else {
        return false;
    };
    let path = dir.join(LAST_RESTART);
    let recent = std::fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|when| SystemTime::now().duration_since(when).ok())
        .is_some_and(|age| age < QUIET);
    if recent {
        return false;
    }
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(&path, b"").is_ok()
}

/// The browser process exited: gives the edge and the taskbar back, as the
/// exit does, and starts again.
fn browser_gone(app: &AppHandle) {
    log::error!("webview: the browser process exited");
    super::tray_host::stop();
    super::appbar::remove();
    super::taskbar::release();
    super::hit_test::tracker(app).stop();
    if !may_restart(app) {
        log::error!("webview: it exited again soon after a restart; closing");
        app.exit(1);
        return;
    }
    log::warn!("webview: restarting the DockMax");
    // The new process must not find this one's single-instance lock.
    tauri_plugin_single_instance::destroy(app);
    app.restart();
}

/// Starts watching the browser process of `window`'s WebView2. Called when
/// the dock's page loads; once per run.
pub fn watch(window: &WebviewWindow) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = window.app_handle().clone();
    let scheduled = window.with_webview(move |webview| {
        if let Err(error) = subscribe(&webview, app) {
            log::warn!("webview: the browser process can not be watched: {error}");
            WATCHING.store(false, Ordering::SeqCst);
        }
    });
    if let Err(error) = scheduled {
        log::warn!("webview: the browser process watch was not scheduled: {error}");
        WATCHING.store(false, Ordering::SeqCst);
    }
}

fn subscribe(
    webview: &tauri::webview::PlatformWebview,
    app: AppHandle,
) -> windows::core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    };
    use webview2_com::ProcessFailedEventHandler;

    let handler = ProcessFailedEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
        // SAFETY: the arguments are alive for the call.
        unsafe { args.ProcessFailedKind(&mut kind) }?;
        if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED {
            browser_gone(&app);
        }
        Ok(())
    }));
    let mut token = 0i64;
    // SAFETY: `with_webview` runs this on the main thread, where the
    // controller lives, and keeps the webview alive for the call.
    unsafe {
        webview
            .controller()
            .CoreWebView2()?
            .add_ProcessFailed(&handler, &mut token)
    }
}
