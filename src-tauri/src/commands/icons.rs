//! The icon packs as app state (spec 0007): the packs found in the packs
//! folder, kept until the panel asks to read the folder again, and the commands
//! that list them and open the folder. The pack that is in use is the
//! `dock.iconPack` setting; `app_icon` (`commands/apps.rs`) asks this module
//! for the image of a program.
//!
//! Nothing here sends a path to a window, and logs name only the capability
//! and the kind of error.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::apps::{AppCommandError, forget_icons};
use super::widgets::lock;
use crate::domain::apps::{AppError, PinnedApp};
use crate::domain::icon_pack::PackError;
use crate::persistence::database::Storage;
use crate::persistence::icon_packs::{self, Found, Pack};
use crate::windows::apps;

/// Event sent to every window when the icons the dock shows may have changed
/// (a pack was added, removed or replaced).
pub const CHANGED_EVENT: &str = "icons-changed";

/// The packs found last, `None` until the folder is first read.
static PACKS: Mutex<Option<Arc<Vec<Found>>>> = Mutex::new(None);

fn folder(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| icon_packs::root(&dir))
}

fn stored(found: Option<Arc<Vec<Found>>>) -> Option<Arc<Vec<Found>>> {
    *PACKS.lock().unwrap_or_else(PoisonError::into_inner) = found.clone();
    found
}

/// Reads the folder again and keeps what it finds. Returns the new packs and
/// whether they differ from the ones kept before.
fn rescan(app: &AppHandle) -> (Arc<Vec<Found>>, bool) {
    let found = Arc::new(
        folder(app)
            .map(|root| icon_packs::scan(&root))
            .unwrap_or_default(),
    );
    let before = PACKS.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let changed = before.is_none_or(|before| *before != *found);
    stored(Some(found.clone()));
    (found, changed)
}

/// The packs kept, reading the folder the first time.
fn packs(app: &AppHandle) -> Arc<Vec<Found>> {
    let kept = PACKS.lock().unwrap_or_else(PoisonError::into_inner).clone();
    kept.unwrap_or_else(|| rescan(app).0)
}

/// The pack `id`, if it is in the folder and valid.
pub fn pack_named(app: &AppHandle, id: &str) -> Option<Pack> {
    packs(app)
        .iter()
        .find(|found| found.id == id)
        .and_then(|found| found.pack.clone().ok())
}

/// The image `pack_id` has for a program known by `names`, as the bytes of a
/// PNG; `None` if the pack is gone, does not cover it and has no fallback, or
/// the image is not good. Reads a file: off the event loop.
pub fn pack_icon(app: &AppHandle, pack_id: &str, names: &[String]) -> Option<Vec<u8>> {
    let pack = pack_named(app, pack_id)?;
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let image = pack.manifest.image_for(&names)?;
    icon_packs::read_image(&pack, image)
}

/// A pack of the folder, for the panel: `{"id": "...", "name": "...", ...}`.
/// `error` is set (and the rest but the id empty) for a folder that is not a
/// usable pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    /// `#RRGGBB`.
    pub accent: Option<String>,
    pub icon_count: usize,
    /// How many of the pinned programs the pack has an image for, of
    /// `total` (the fallback does not count).
    pub covers: usize,
    pub total: usize,
    pub error: Option<PackError>,
}

/// The names a program is looked up by: the name the shell shows for it, and
/// the name of the file for a program dropped on the dock.
pub fn names_of(app_id: &str, shown: &str) -> Vec<String> {
    let mut names = vec![shown.to_owned()];
    if let Some(stem) = apps::file_of(app_id)
        .as_deref()
        .and_then(std::path::Path::file_stem)
        .and_then(std::ffi::OsStr::to_str)
    {
        names.push(stem.to_owned());
    }
    names
}

fn info_of(found: &Found, pinned: &[PinnedApp]) -> PackInfo {
    match &found.pack {
        Ok(pack) => {
            let manifest = &pack.manifest;
            PackInfo {
                id: found.id.clone(),
                name: manifest.name.clone(),
                author: manifest.author.clone(),
                version: manifest.version.clone(),
                description: manifest.description.clone(),
                accent: manifest.accent.clone(),
                icon_count: manifest.icons.len(),
                covers: pinned
                    .iter()
                    .filter(|app| {
                        let names = names_of(&app.app_id, &app.name);
                        let names: Vec<&str> = names.iter().map(String::as_str).collect();
                        manifest.covers(&names)
                    })
                    .count(),
                total: pinned.len(),
                error: None,
            }
        }
        Err(error) => PackInfo {
            id: found.id.clone(),
            name: found.id.clone(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            accent: None,
            icon_count: 0,
            covers: 0,
            total: pinned.len(),
            error: Some(*error),
        },
    }
}

/// Reads the packs folder and lists what is in it. `force` drops the icons
/// already read and tells every window (`icons-changed`) even when no manifest
/// changed (an image may have been replaced); without it that happens only
/// when the packs found differ from the ones kept.
async fn listing(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    force: bool,
) -> Result<Vec<PackInfo>, AppCommandError> {
    let pinned = {
        lock(&state)
            .apps_snapshot()
            .map(|snapshot| snapshot.apps)
            .unwrap_or_default()
    };
    let handle = app.clone();
    let (found, changed) = tauri::async_runtime::spawn_blocking(move || rescan(&handle))
        .await
        .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
    if changed || force {
        forget_icons();
        if let Err(error) = app.emit(CHANGED_EVENT, ()) {
            log::warn!("icons: could not announce a change: {error}");
        }
        crate::windows::dock::forget_icons();
    }
    Ok(found.iter().map(|found| info_of(found, &pinned)).collect())
}

/// Lists the packs. Reads the folder again, as a change there is what the
/// panel wants to see when it opens.
#[tauri::command]
pub async fn list_icon_packs(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
) -> Result<Vec<PackInfo>, AppCommandError> {
    listing(app, state, false).await
}

/// "Recarregar pacotes": like [`list_icon_packs`], and the icons the dock
/// shows are read again whatever changed.
#[tauri::command]
pub async fn reload_icon_packs(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
) -> Result<Vec<PackInfo>, AppCommandError> {
    listing(app, state, true).await
}

/// Opens the packs folder in File Explorer, making it if it is not there.
#[tauri::command]
pub fn open_icon_packs_folder(app: AppHandle) -> Result<(), AppCommandError> {
    let folder = folder(&app).ok_or(AppError::Unavailable)?;
    std::fs::create_dir_all(&folder).map_err(|error| {
        log::warn!("icons: could not make the packs folder: {}", error.kind());
        AppCommandError::from(AppError::Unavailable)
    })?;
    apps::open_folder(&folder).map_err(|error| {
        log::warn!("icons: could not open the packs folder: {error}");
        AppCommandError::from(AppError::Unavailable)
    })
}
