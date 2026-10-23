//! The programs pinned to the dock as app state (spec 0006): kept in the same
//! database as the widgets, read by both windows and changed only by the
//! panel. Rust is the source of truth; the windows apply what `apps-changed`
//! carries.
//!
//! The commands only adapt: the rules live in the functions below, which take
//! a [`Storage`] and are tested without Tauri or Windows. Logs name only the
//! capability and the kind of error, never an id or a name.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::widgets::lock;
use crate::domain::apps::{
    AppError, AppErrorCode, AppsSnapshot, PinnedApp, is_valid_app_id, is_valid_app_name,
};
use crate::domain::widgets::{MoveDirection, WidgetId};
use crate::persistence::apps_repo;
use crate::persistence::database::Storage;
use crate::windows::apps;

/// Event sent to every window with the full [`AppsSnapshot`] after a change.
pub const CHANGED_EVENT: &str = "apps-changed";

/// How an app command fails: `{ "code": "notFound" }`, matching
/// `AppCommandError` in `src/core/apps.ts`. Carries no id and no name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCommandError {
    code: AppErrorCode,
}

impl From<AppError> for AppCommandError {
    fn from(error: AppError) -> Self {
        AppCommandError { code: error.code() }
    }
}

/// One program of the catalog, for the list the panel offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub app_id: String,
    pub name: String,
}

/// The pinned programs in dock order, with the storage status.
pub fn snapshot(storage: &Storage) -> Result<AppsSnapshot, AppError> {
    storage.apps_snapshot()
}

/// Pins the program `app_id`, named `name` (read from the shell by the
/// command, never taken from the page).
pub fn pin(storage: &mut Storage, app_id: &str, name: &str) -> Result<PinnedApp, AppError> {
    apps_repo::pin(storage.connection()?, app_id, name)
}

/// Unpins the program `id`.
pub fn unpin(storage: &mut Storage, id: &str) -> Result<(), AppError> {
    apps_repo::unpin(storage.connection()?, &WidgetId::parse(id)?)
}

/// Moves the program `id` one place up or down.
pub fn shift(storage: &mut Storage, id: &str, direction: MoveDirection) -> Result<(), AppError> {
    apps_repo::shift(storage.connection()?, &WidgetId::parse(id)?, direction)
}

/// The parsing name of the pinned program `id`, to open it.
pub fn target_of(storage: &mut Storage, id: &str) -> Result<String, AppError> {
    let id = WidgetId::parse(id)?;
    apps_repo::get(storage.connection()?, &id)?
        .map(|app| app.app_id)
        .ok_or(AppError::NotFound)
}

/// Runs `change` and, if it was accepted, sends `apps-changed` to every
/// window. `what` names the change in the log. The lock is released before
/// emitting.
fn change<T>(
    app: &AppHandle,
    state: &State<'_, Mutex<Storage>>,
    what: &str,
    change: impl FnOnce(&mut Storage) -> Result<T, AppError>,
) -> Result<T, AppCommandError> {
    let (value, snapshot) = {
        let mut storage = lock(state);
        let value = change(&mut storage).inspect_err(|error| {
            log::warn!("apps: {what} rejected: {error}");
        })?;
        let snapshot = storage
            .apps_snapshot()
            .inspect_err(|error| {
                log::warn!("apps: {what} saved, but the list could not be read: {error}");
            })
            .ok();
        (value, snapshot)
    };
    if let Some(snapshot) = snapshot
        && let Err(error) = app.emit(CHANGED_EVENT, &snapshot)
    {
        log::warn!("apps: {what} saved, but failed to notify the windows: {error}");
    }
    Ok(value)
}

/// Called by both windows at startup and after `apps-changed` is missed.
#[tauri::command]
pub fn list_apps(state: State<'_, Mutex<Storage>>) -> Result<AppsSnapshot, AppCommandError> {
    snapshot(&lock(&state)).map_err(|error| {
        log::warn!("apps: list failed: {error}");
        AppCommandError::from(error)
    })
}

/// Every program of the Apps folder, by name. Runs off the main thread:
/// listing takes about a second the first time.
#[tauri::command]
pub async fn list_catalog() -> Result<Vec<CatalogEntry>, AppCommandError> {
    let listed = tauri::async_runtime::spawn_blocking(apps::catalog).await;
    match listed {
        Ok(Ok(catalog)) => Ok(catalog
            .into_iter()
            .map(|app| CatalogEntry {
                app_id: app.id,
                name: app.name,
            })
            .collect()),
        Ok(Err(error)) => {
            log::warn!("apps: the catalog could not be read: {error}");
            Err(AppError::Unavailable.into())
        }
        Err(_) => {
            log::warn!("apps: the catalog task did not finish");
            Err(AppError::Unavailable.into())
        }
    }
}

/// The sizes of an icon the dock asks for: the panel's list and the dock.
const ICON_SIZES: [u32; 2] = [32, 96];

/// The icons already read, so a program's icon is decoded once per run. A
/// cap keeps the cache small: the panel's catalog can ask for a few hundred.
const ICON_CACHE_LIMIT: usize = 300;

/// The icon of a program at a size; `None` for a program with no icon, which
/// is also remembered.
type IconCache = HashMap<(String, u32), Option<String>>;

static ICONS: Mutex<Option<IconCache>> = Mutex::new(None);

fn cached_icon(key: &(String, u32)) -> Option<Option<String>> {
    let cache = ICONS.lock().unwrap_or_else(PoisonError::into_inner);
    cache.as_ref().and_then(|icons| icons.get(key).cloned())
}

fn remember_icon(key: (String, u32), icon: Option<String>) {
    let mut cache = ICONS.lock().unwrap_or_else(PoisonError::into_inner);
    let icons = cache.get_or_insert_with(HashMap::new);
    if icons.len() >= ICON_CACHE_LIMIT {
        icons.clear();
    }
    icons.insert(key, icon);
}

/// The icon of a program as a PNG `data:` URL, or `null` when Windows has
/// none (the dock then shows the first letter of its name). `size` is 32 or
/// 96; any other value is the nearest of the two.
#[tauri::command]
pub async fn app_icon(app_id: String, size: u32) -> Result<Option<String>, AppCommandError> {
    if !is_valid_app_id(&app_id) {
        return Err(AppError::InvalidApp.into());
    }
    let size = if size <= ICON_SIZES[0] {
        ICON_SIZES[0]
    } else {
        ICON_SIZES[1]
    };
    let key = (app_id, size);
    if let Some(icon) = cached_icon(&key) {
        return Ok(icon);
    }
    let lookup = key.clone();
    let read = tauri::async_runtime::spawn_blocking(move || apps::icon_png(&lookup.0, lookup.1))
        .await
        .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
    let icon = read.map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)));
    remember_icon(key, icon.clone());
    Ok(icon)
}

/// Pins a program of the Apps folder at the end of the dock. The name is the
/// one the shell shows for the id, so the page can't invent one.
#[tauri::command]
pub async fn pin_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    app_id: String,
) -> Result<PinnedApp, AppCommandError> {
    if !is_valid_app_id(&app_id) {
        return Err(AppError::InvalidApp.into());
    }
    let id = app_id.clone();
    let name = tauri::async_runtime::spawn_blocking(move || apps::name_of(&id))
        .await
        .ok()
        .flatten()
        .filter(|name| is_valid_app_name(name))
        .ok_or(AppError::InvalidApp)?;
    change(&app, &state, "pin", |storage| pin(storage, &app_id, &name))
}

#[tauri::command]
pub fn unpin_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<(), AppCommandError> {
    change(&app, &state, "unpin", |storage| unpin(storage, &id))
}

#[tauri::command]
pub fn move_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
) -> Result<(), AppCommandError> {
    change(&app, &state, "move", |storage| {
        shift(storage, &id, direction)
    })
}

/// Opens the pinned program `id`.
#[tauri::command]
pub async fn launch_app(
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<(), AppCommandError> {
    let target = target_of(&mut lock(&state), &id).inspect_err(|error| {
        log::warn!("apps: launch rejected: {error}");
    })?;
    let opened = tauri::async_runtime::spawn_blocking(move || apps::launch(&target)).await;
    match opened {
        Ok(Ok(())) => Ok(()),
        _ => {
            log::warn!("apps: a program could not be opened");
            Err(AppError::LaunchFailed.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;

    fn open() -> (TempDir, Storage) {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path());
        (dir, storage)
    }

    #[test]
    fn a_pinned_program_is_in_the_snapshot_and_can_be_opened_by_its_id() {
        let (_dir, mut storage) = open();

        let pinned = pin(&mut storage, "Some.App!App", "Some App").unwrap();

        let snapshot = snapshot(&storage).unwrap();
        assert_eq!(snapshot.apps, vec![pinned.clone()]);
        assert_eq!(
            target_of(&mut storage, pinned.id.as_str()),
            Ok("Some.App!App".to_owned())
        );
    }

    #[test]
    fn an_id_that_is_not_one_rust_made_is_not_found() {
        let (_dir, mut storage) = open();
        pin(&mut storage, "Some.App!App", "Some App").unwrap();

        assert_eq!(unpin(&mut storage, "1"), Err(AppError::NotFound));
        assert_eq!(
            shift(&mut storage, "../x", MoveDirection::Up),
            Err(AppError::NotFound)
        );
        assert_eq!(
            target_of(&mut storage, "Some.App!App"),
            Err(AppError::NotFound)
        );
    }

    #[test]
    fn a_removed_program_cannot_be_opened() {
        let (_dir, mut storage) = open();
        let pinned = pin(&mut storage, "Some.App!App", "Some App").unwrap();

        unpin(&mut storage, pinned.id.as_str()).unwrap();

        assert_eq!(
            target_of(&mut storage, pinned.id.as_str()),
            Err(AppError::NotFound)
        );
    }

    #[test]
    fn without_a_database_nothing_is_pinned_and_nothing_can_be() {
        let mut storage =
            Storage::unavailable(crate::domain::widgets::UnavailableReason::OpenFailed);

        assert_eq!(snapshot(&storage).unwrap().apps, []);
        assert_eq!(
            pin(&mut storage, "Some.App!App", "Some App"),
            Err(AppError::Unavailable)
        );
    }

    #[test]
    fn the_command_error_is_only_the_code() {
        let error = AppCommandError::from(AppError::AlreadyPinned);

        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"code":"alreadyPinned"}"#
        );
    }

    #[test]
    fn the_icon_cache_is_bounded() {
        for n in 0..=ICON_CACHE_LIMIT {
            remember_icon((format!("bounded-{n}"), 32), None);
        }

        let len = ICONS.lock().unwrap().as_ref().map_or(0, HashMap::len);
        assert!(len <= ICON_CACHE_LIMIT, "{len}");
        assert_eq!(
            cached_icon(&(format!("bounded-{ICON_CACHE_LIMIT}"), 32)),
            Some(None)
        );
    }
}
