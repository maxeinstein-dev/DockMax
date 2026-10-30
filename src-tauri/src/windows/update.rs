//! The automatic update (spec 0019 G3): looks for a new version in the
//! GitHub Releases (`plugins.updater` in `tauri.conf.json`) a moment after
//! the app opens and every 6 hours, announces it once with a Windows notice,
//! and installs it only when the user clicks Atualizar in Sobre.
//!
//! On Windows the updater's `install` runs the NSIS installer and ends the
//! process with `std::process::exit`, which skips `RunEvent::Exit`. So the
//! install runs on the main thread, where the docks live, and its
//! `on_before_exit` does what Sair does (`crate::release_on_exit`): the
//! taskbar comes back, the dock stops and the database is checkpointed
//! before the installer starts.

use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard, Once};
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::domain::update::{self, Notices, UpdateState};

/// The event every window gets when the state changes.
pub const EVENT: &str = "update-changed";

/// The test build never looks for updates (its updater is not even
/// registered).
pub const ENABLED: bool = cfg!(not(feature = "e2e"));
/// Looks by itself only in the release build: a debug build is the
/// author's, run from the code.
const AUTOMATIC: bool = ENABLED && !cfg!(debug_assertions);

static STATE: Mutex<UpdateState> = Mutex::new(UpdateState::None);
static NOTICES: Mutex<Notices> = Mutex::new(Notices::new());
static STARTED: Once = Once::new();

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What Sobre shows now.
pub fn state() -> UpdateState {
    lock(&STATE).clone()
}

fn set(app: &AppHandle, state: UpdateState) {
    *lock(&STATE) = state.clone();
    if let Err(error) = app.emit(EVENT, state) {
        log::warn!("update: the windows could not be told: {error}");
    }
}

/// Starts looking: a moment after the app opens, then every 6 hours. Only
/// in the release build; once.
pub fn start(app: &AppHandle) {
    if !AUTOMATIC {
        return;
    }
    STARTED.call_once(|| {
        let app = app.clone();
        let spawned = std::thread::Builder::new()
            .name("update".into())
            .spawn(move || run(&app));
        if let Err(error) = spawned {
            log::warn!("update: the thread could not start: {error}");
        }
    });
}

fn run(app: &AppHandle) {
    let mut last: Option<Instant> = None;
    loop {
        std::thread::sleep(update::wait(last.map(|at| at.elapsed())));
        // A failure (no network, GitHub down) is only logged by `check`.
        let _ = tauri::async_runtime::block_on(check(app, true));
        last = Some(Instant::now());
    }
}

/// The newest version, if newer than this one. The updater carries what to
/// do right before the installer takes over.
async fn find(app: &AppHandle) -> Result<Option<Update>, String> {
    let handle = app.clone();
    let updater = app
        .updater_builder()
        .on_before_exit(move || {
            crate::release_on_exit(&handle);
            // What Tauri does itself on its way out (the tray icon, among
            // others): the plugin's own hook, which this one replaces.
            handle.cleanup_before_exit();
        })
        .build()
        .map_err(|error| error.to_string())?;
    updater.check().await.map_err(|error| error.to_string())
}

/// Looks for a new version now. `announce`: a look of its own, which shows
/// the Windows notice the first time a version is found; a look the user
/// asked for in Sobre shows the answer there, and the version counts as
/// announced.
pub async fn check(app: &AppHandle, announce: bool) -> Result<UpdateState, String> {
    if !ENABLED {
        return Err("updates are off in this build".into());
    }
    if state().is_busy() {
        return Ok(state());
    }
    let found = find(app).await.inspect_err(|error| {
        log::warn!("update: the look failed: {error}");
    })?;
    // From the state now: an Atualizar may have started meanwhile.
    let next = state().after_check(found.map(|update| (update.version, update.body)));
    if let UpdateState::Available { version, .. } = &next {
        let first = lock(&NOTICES).first_time(version);
        if first {
            log::info!("update: version {version} available");
            if announce {
                notify(version);
            }
        }
    }
    set(app, next.clone());
    Ok(next)
}

/// The Windows notice of a new version.
fn notify(version: &str) {
    #[cfg(windows)]
    {
        let text = &crate::i18n::messages().dock;
        let title = text.update_notice_title.replace("{version}", version);
        match super::toast::show(&title, text.update_notice_line) {
            Ok(true) => {}
            Ok(false) => log::info!("update: Windows has the notices off"),
            Err(error) => log::warn!("update: the notice failed: {error}"),
        }
    }
    #[cfg(not(windows))]
    let _ = version;
}

/// Atualizar in Sobre: downloads the new version, showing the progress,
/// then installs it on the main thread. The app ends there; the passive
/// installer opens it again when it is done (`/R`).
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if !ENABLED {
        return Err("updates are off in this build".into());
    }
    if state().is_busy() {
        return Err("an update is already under way".into());
    }
    let Some(found) = find(app).await? else {
        set(app, UpdateState::None);
        return Err("there is no newer version".into());
    };
    let version = found.version.clone();
    let available = UpdateState::Available {
        version: version.clone(),
        notes: found.body.clone(),
    };
    set(
        app,
        UpdateState::Downloading {
            version: version.clone(),
            downloaded: 0,
            total: None,
        },
    );
    let mut downloaded = 0u64;
    let mut shown = 0u64;
    let bytes = found
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                if progress_worth_showing(shown, downloaded, total) {
                    shown = downloaded;
                    set(
                        app,
                        UpdateState::Downloading {
                            version: version.clone(),
                            downloaded,
                            total,
                        },
                    );
                }
            },
            || {},
        )
        .await;
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("update: the download failed: {error}");
            set(app, available);
            return Err(error.to_string());
        }
    };
    set(
        app,
        UpdateState::Installing {
            version: version.clone(),
        },
    );
    log::info!("update: installing {version}");

    // The docks and the launcher live in the main thread's storage, and the
    // exit lets go of them there, as `RunEvent::Exit` does.
    let (sent, answer) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        // Returns only when it failed: on success the process ends inside.
        let result = found.install(bytes);
        let _ = sent.send(result.map_err(|error| error.to_string()));
        if crate::released() {
            // The installer could not start after the dock let go of
            // everything: open the app again rather than leave it half.
            log::warn!("update: the installer did not start; restarting");
            handle.restart();
        }
    })
    .map_err(|error| error.to_string())?;
    let result = tauri::async_runtime::spawn_blocking(move || answer.recv())
        .await
        .map_err(|error| error.to_string())?
        .unwrap_or(Ok(()));
    if let Err(error) = &result {
        log::warn!("update: the install failed: {error}");
        set(app, available);
    }
    result
}

/// Whether the download moved enough since `shown` to tell the windows:
/// each 1% with a known size, each 512 KiB without one, and at the end.
fn progress_worth_showing(shown: u64, downloaded: u64, total: Option<u64>) -> bool {
    match total {
        Some(total) if total > 0 => {
            downloaded >= total || (downloaded - shown).saturating_mul(100) >= total
        }
        _ => downloaded - shown >= 512 * 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_progress_is_told_each_percent() {
        assert!(!progress_worth_showing(0, 9, Some(1000)));
        assert!(progress_worth_showing(0, 10, Some(1000)));
        assert!(!progress_worth_showing(10, 19, Some(1000)));
        assert!(progress_worth_showing(995, 1000, Some(1000)));
        assert!(!progress_worth_showing(0, 1000, None));
        assert!(progress_worth_showing(0, 512 * 1024, None));
    }

    #[test]
    fn only_the_release_build_looks_by_itself() {
        assert_eq!(AUTOMATIC, ENABLED && !cfg!(debug_assertions));
        assert_eq!(ENABLED, cfg!(not(feature = "e2e")));
    }
}
