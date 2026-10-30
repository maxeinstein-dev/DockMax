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
use tauri::{AppHandle, Emitter, Manager, State};

use super::icons;
use super::widgets::lock;
use crate::domain::apps::{
    AppError, AppErrorCode, AppsSnapshot, PinnedApp, is_pinnable_path, is_valid_app_id,
    is_valid_app_name,
};
use crate::domain::icon_pack::is_valid_pack_id;
use crate::domain::open_apps::OpenSnapshot;
use crate::domain::settings::DockStyle;
use crate::domain::widgets::{MoveDirection, WidgetId};
use crate::persistence::apps_repo;
use crate::persistence::database::Storage;
use crate::windows::apps;
use crate::windows::jumplist::{self, Runnable, Section, SectionKind};

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

/// The key a workspace knows a pinned program by.
fn key_of(id: &WidgetId) -> String {
    format!("app:{}", id.as_str())
}

/// Pins the program `app_id`, named `name` (read from the shell by the
/// command, never taken from the page), at the end of the workspace
/// `workspace` (or the one in use). A program pinned in another workspace
/// is only put in this one; `alreadyPinned` when this one has it.
pub fn pin(
    storage: &mut Storage,
    app_id: &str,
    name: &str,
    workspace: Option<&str>,
) -> Result<PinnedApp, AppError> {
    let workspace = crate::commands::widgets::workspace_of(storage, workspace)?;
    let app = match apps_repo::pin(storage.connection()?, app_id, name) {
        Err(AppError::AlreadyPinned) => {
            let known = storage
                .apps_snapshot()?
                .apps
                .into_iter()
                .find(|app| app.app_id.eq_ignore_ascii_case(app_id))
                .ok_or(AppError::AlreadyPinned)?;
            let has = storage
                .workspaces()?
                .iter()
                .any(|w| w.id == workspace && w.items.contains(&key_of(&known.id)));
            if has {
                return Err(AppError::AlreadyPinned);
            }
            known
        }
        other => other?,
    };
    storage.add_to_workspace(&workspace, &key_of(&app.id))?;
    Ok(app)
}

/// Takes the program `id` out of the workspace `workspace` (or the one in
/// use); with no other workspace showing it, it is unpinned for good.
pub fn unpin(storage: &mut Storage, id: &str, workspace: Option<&str>) -> Result<(), AppError> {
    let id = WidgetId::parse(id)?;
    let workspace = crate::commands::widgets::workspace_of(storage, workspace)?;
    let key = key_of(&id);
    crate::commands::widgets::in_workspace(storage, &workspace, &key)?;
    if storage.remove_from_workspace(&workspace, &key)? {
        return Ok(());
    }
    apps_repo::unpin(storage.connection()?, &id)?;
    storage.forget_item(&key)?;
    Ok(())
}

/// The dock's id of the program `app_id` of the Apps folder when the
/// workspace in use has it pinned; `None` when it does not (pinned only in
/// another workspace, or not at all).
pub fn pinned_here(storage: &mut Storage, app_id: &str) -> Result<Option<WidgetId>, AppError> {
    let Some(known) = storage
        .apps_snapshot()?
        .apps
        .into_iter()
        .find(|app| app.app_id.eq_ignore_ascii_case(app_id))
    else {
        return Ok(None);
    };
    let workspace = crate::commands::widgets::workspace_of(storage, None)?;
    let has = storage
        .workspaces()?
        .iter()
        .any(|w| w.id == workspace && w.items.contains(&key_of(&known.id)));
    Ok(has.then_some(known.id))
}

/// Pins (`pin`) the program `app_id`, named `name`, to the workspace in use,
/// or takes it out of it: the launcher's "Fixar no dock" and "Desafixar"
/// (spec 0012 L6), through what the panel's `pin_app` and `unpin_app` do.
/// The name is the one the Apps folder gave the launcher's catalog.
pub fn pin_from_launcher(
    app: &AppHandle,
    app_id: &str,
    name: &str,
    wanted: bool,
) -> Result<(), AppCommandError> {
    if !is_valid_app_id(app_id) || !is_valid_app_name(name) {
        return Err(AppError::InvalidApp.into());
    }
    let state = app.state::<Mutex<Storage>>();
    change(app, &state, "launcher pin", |storage| {
        if wanted {
            pin(storage, app_id, name, None).map(|_| ())
        } else {
            let id = pinned_here(storage, app_id)?.ok_or(AppError::NotFound)?;
            unpin(storage, id.as_str(), None)
        }
    })
}

/// Moves the program `id` one place up or down among the programs of the
/// workspace `workspace` (or the one in use).
pub fn shift(
    storage: &mut Storage,
    id: &str,
    direction: MoveDirection,
    workspace: Option<&str>,
) -> Result<(), AppError> {
    let id = WidgetId::parse(id)?;
    let workspace = crate::commands::widgets::workspace_of(storage, workspace)?;
    let key = key_of(&id);
    crate::commands::widgets::in_workspace(storage, &workspace, &key)?;
    storage.shift_in_workspace(&workspace, &key, direction == MoveDirection::Up)?;
    Ok(())
}

/// Puts the program `id` at `index` among the pinned ones.
pub fn place(storage: &mut Storage, id: &str, index: usize) -> Result<(), AppError> {
    apps_repo::place(storage.connection()?, &WidgetId::parse(id)?, index)
}

/// The paths of what was dropped on the dock that can be pinned, with no
/// repeats, in the order dropped: a program or a shortcut on a drive that
/// exists (`is_pinnable_path`, then the disk).
pub fn droppable(paths: &[std::path::PathBuf]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for path in paths {
        let Some(text) = path.to_str() else { continue };
        let known = found.iter().any(|seen| seen.eq_ignore_ascii_case(text));
        if is_pinnable_path(text) && path.is_file() && !known {
            found.push(text.to_owned());
        }
    }
    found
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
    // What each workspace shows may have changed with it.
    crate::commands::workspaces::notify(app);
    crate::windows::dock::refresh();
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

/// What an icon is read for: the icon pack in use (`None` for the icons
/// Windows gives), the program and the size.
type IconKey = (Option<String>, String, u32, bool);

/// The icon of a program at a size; `None` for a program with no icon, which
/// is also remembered.
type IconCache = HashMap<IconKey, Option<String>>;

static ICONS: Mutex<Option<IconCache>> = Mutex::new(None);

/// Drops the icons read, for a pack that changed on disk (spec 0007).
pub fn forget_icons() {
    *ICONS.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

fn cached_icon(key: &IconKey) -> Option<Option<String>> {
    let cache = ICONS.lock().unwrap_or_else(PoisonError::into_inner);
    cache.as_ref().and_then(|icons| icons.get(key).cloned())
}

fn remember_icon(key: IconKey, icon: Option<String>) {
    let mut cache = ICONS.lock().unwrap_or_else(PoisonError::into_inner);
    let icons = cache.get_or_insert_with(HashMap::new);
    if icons.len() >= ICON_CACHE_LIMIT {
        icons.clear();
    }
    icons.insert(key, icon);
}

/// The icon of a program as a PNG `data:` URL, or `null` when there is none
/// (the dock then shows the first letter of its name). `size` is 32 or 96; any
/// other value is the nearest of the two. At the dock's size the icon pack in
/// use (spec 0007) answers first, then Windows; the panel's list (32) always
/// shows the program's own icon.
#[tauri::command]
pub async fn app_icon(
    app: AppHandle,
    app_id: String,
    size: u32,
) -> Result<Option<String>, AppCommandError> {
    if !is_valid_app_id(&app_id) {
        return Err(AppError::InvalidApp.into());
    }
    let size = if size <= ICON_SIZES[0] {
        ICON_SIZES[0]
    } else {
        ICON_SIZES[1]
    };
    let pack = if size == ICON_SIZES[1] {
        super::settings::current(&app).dock.icon_pack
    } else {
        None
    };
    let dark = pack.is_some() && super::settings::icons_dark(&app, None);
    let key = (pack, app_id, size, dark);
    if let Some(icon) = cached_icon(&key) {
        return Ok(icon);
    }
    let lookup = key.clone();
    let read = tauri::async_runtime::spawn_blocking(move || {
        let (pack, app_id, size, dark) = lookup;
        let from_pack = pack.and_then(|pack| {
            let shown = apps::name_of(&app_id).unwrap_or_default();
            icons::pack_icon(&app, &pack, &icons::names_of(&app_id, &shown), dark)
        });
        from_pack.or_else(|| apps::icon_png(&app_id, size))
    })
    .await
    .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
    let icon = read.map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)));
    remember_icon(key, icon.clone());
    Ok(icon)
}

/// The most programs a miniature of an icon pack shows.
pub const PREVIEW_APPS: usize = 6;

/// The icons `pack` gives the programs `app_ids`, in that order, as PNG
/// `data:` URLs (`null` for a program with no icon): the miniature of the
/// pack's card in Configurações (spec 0007 §3.5). On the bar of `style` (the
/// dock's own when `None`) in the dock's color mode, and at its size; without
/// a pack, the programs' own icons. At most
/// [`PREVIEW_APPS`] programs.
#[tauri::command]
pub async fn icon_pack_preview(
    app: AppHandle,
    pack: Option<String>,
    app_ids: Vec<String>,
    style: Option<DockStyle>,
) -> Result<Vec<Option<String>>, AppCommandError> {
    if app_ids.len() > PREVIEW_APPS || !app_ids.iter().all(|id| is_valid_app_id(id)) {
        return Err(AppError::InvalidApp.into());
    }
    if pack.as_deref().is_some_and(|id| !is_valid_pack_id(id)) {
        return Err(AppError::InvalidApp.into());
    }
    let dark = pack.is_some() && super::settings::icons_dark(&app, style);
    let mut icons = Vec::with_capacity(app_ids.len());
    for app_id in app_ids {
        let key = (pack.clone(), app_id, ICON_SIZES[1], dark);
        if let Some(icon) = cached_icon(&key) {
            icons.push(icon);
            continue;
        }
        let app = app.clone();
        let lookup = key.clone();
        let read = tauri::async_runtime::spawn_blocking(move || {
            let (pack, app_id, size, dark) = lookup;
            let from_pack = pack.and_then(|pack| {
                let shown = apps::name_of(&app_id).unwrap_or_default();
                icons::pack_icon(&app, &pack, &icons::names_of(&app_id, &shown), dark)
            });
            from_pack.or_else(|| apps::icon_png(&app_id, size))
        })
        .await
        .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
        let icon = read.map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)));
        remember_icon(key, icon.clone());
        icons.push(icon);
    }
    Ok(icons)
}

/// The icon the dock draws for a program (spec 0011 H3), as PNG: the icon
/// pack in use answers first (its dark image while `dark`), then Windows.
/// Reads the shell: off the main thread.
pub fn dock_icon_png(app: &AppHandle, app_id: &str, dark: bool) -> Option<Vec<u8>> {
    if !is_valid_app_id(app_id) {
        return None;
    }
    let pack = super::settings::current(app).dock.icon_pack;
    let from_pack = pack.and_then(|pack| {
        let shown = apps::name_of(app_id).unwrap_or_default();
        icons::pack_icon(app, &pack, &icons::names_of(app_id, &shown), dark)
    });
    from_pack.or_else(|| apps::icon_png(app_id, ICON_SIZES[1]))
}

/// Opens the pinned program `id` (a click on the dock), off the main thread;
/// `failed` runs when it could not be opened.
pub fn launch_pinned(app: &AppHandle, id: &str, failed: impl FnOnce() + Send + 'static) {
    let target = {
        let state = app.state::<Mutex<Storage>>();
        target_of(&mut lock(&state), id)
    };
    let Ok(target) = target.inspect_err(|error| log::warn!("apps: launch rejected: {error}"))
    else {
        failed();
        return;
    };
    launch_off_thread(target, failed);
}

/// Opens a new instance of the open program `key` that is not pinned (the
/// middle button on the dock); `failed` runs when it could not be opened.
pub fn launch_other(key: &str, failed: impl FnOnce() + Send + 'static) {
    #[cfg(windows)]
    let target = crate::windows::open_apps::app_id_of_other(key);
    #[cfg(not(windows))]
    let target: Option<String> = {
        let _ = key;
        None
    };
    match target {
        Some(target) => launch_off_thread(target, failed),
        None => failed(),
    }
}

fn launch_off_thread(target: String, failed: impl FnOnce() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("dock-launch".into())
        .spawn(move || {
            if apps::launch(&target).is_err() {
                log::warn!("apps: a program could not be opened");
                failed();
            }
        });
    if let Err(error) = spawned {
        log::warn!("apps: the program could not be opened: {error}");
    }
}

/// Pins a program of the Apps folder at the end of the dock. The name is the
/// one the shell shows for the id, so the page can't invent one.
#[tauri::command]
pub async fn pin_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    app_id: String,
    workspace: Option<String>,
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
    change(&app, &state, "pin", |storage| {
        pin(storage, &app_id, &name, workspace.as_deref())
    })
}

#[tauri::command]
pub fn unpin_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    workspace: Option<String>,
) -> Result<(), AppCommandError> {
    change(&app, &state, "unpin", |storage| {
        unpin(storage, &id, workspace.as_deref())
    })
}

/// Puts a program where the user dropped it among the others (the dock's
/// drag). The dock may ask: it only orders what is already pinned.
#[tauri::command]
pub fn place_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    index: usize,
) -> Result<(), AppCommandError> {
    change(&app, &state, "place", |storage| place(storage, &id, index))
}

/// Pins what was dropped on the dock, at the end of the workspace its
/// monitor (`monitor`, its device name) shows. Comes from the dock's own drop
/// target (`windows::drop_target`), not from the page, so only what the user
/// really dropped can be pinned. Reading the names takes the shell's time:
/// off the event loop.
pub fn pin_dropped(app: &AppHandle, paths: Vec<std::path::PathBuf>, monitor: Option<String>) {
    let wanted = droppable(&paths);
    if wanted.is_empty() {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("dock-drop".into())
        .spawn(move || {
            let state = app.state::<Mutex<Storage>>();
            for id in wanted {
                let Some(name) = apps::name_of(&id).filter(|name| is_valid_app_name(name)) else {
                    log::warn!("apps: a dropped file has no name the shell knows");
                    continue;
                };
                // An error is already in the log; the rest still go in.
                let _ = change(&app, &state, "drop", |storage| {
                    let workspace = match &monitor {
                        Some(monitor) => Some(storage.shown_on(monitor)?.id),
                        None => None,
                    };
                    pin(storage, &id, &name, workspace.as_deref())
                });
            }
        });
    if let Err(error) = spawned {
        log::warn!("apps: the drop could not be handled: {error}");
    }
}

#[tauri::command]
pub fn move_app(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
    workspace: Option<String>,
) -> Result<(), AppCommandError> {
    change(&app, &state, "move", |storage| {
        shift(storage, &id, direction, workspace.as_deref())
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

/// The workspace in use (the dock used last): the windows a panel's menu
/// acts on are the ones of its workspace (spec 0013 C2).
#[cfg(windows)]
fn in_use(app: &AppHandle) -> Option<String> {
    let state = app.state::<Mutex<Storage>>();
    let mut storage = lock(&state);
    storage.in_use().ok().map(|workspace| workspace.id)
}

/// The pinned programs that have windows open (spec 0009), for the dock's
/// first read; after that `open-windows-changed` carries the same list. No
/// window handle leaves Rust.
#[tauri::command]
pub fn list_open_apps() -> OpenSnapshot {
    #[cfg(windows)]
    {
        crate::windows::open_apps::snapshot()
    }
    #[cfg(not(windows))]
    {
        OpenSnapshot::default()
    }
}

/// A click on a pinned program that is open (spec 0009): brings its window
/// to the front, minimizes it if the user is in it, or passes to its next
/// window. `false` when it has no window, so the dock opens it instead. The
/// page names the pinned item by its id; the window is Rust's to find.
#[tauri::command]
pub fn activate_app(app: AppHandle, id: String) -> Result<bool, AppCommandError> {
    let id = WidgetId::parse(&id).map_err(|_| AppCommandError::from(AppError::NotFound))?;
    #[cfg(windows)]
    {
        let workspace = in_use(&app);
        Ok(crate::windows::open_apps::activate(
            &app,
            id.as_str(),
            workspace.as_deref(),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (app, id);
        Ok(false)
    }
}

/// A click on an open program that is not pinned (spec 0009 D2), named by
/// the `key` the dock was given for it (an opaque name, never a path or a
/// window). `false` when it is no longer open.
#[tauri::command]
pub fn activate_open(app: AppHandle, key: String) -> Result<bool, AppCommandError> {
    if !is_open_key_text(&key) {
        return Err(AppError::NotFound.into());
    }
    #[cfg(windows)]
    {
        let workspace = in_use(&app);
        Ok(crate::windows::open_apps::activate_other(
            &app,
            &key,
            workspace.as_deref(),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (app, key);
        Ok(false)
    }
}

/// A middle click on an open program that is not pinned (spec 0009 D2): opens
/// a new instance of it, as the Windows taskbar does, by the `key` the dock
/// was given for it. Fails with `notFound` when it is no longer open.
#[tauri::command]
pub async fn launch_open(key: String) -> Result<(), AppCommandError> {
    if !is_open_key_text(&key) {
        return Err(AppError::NotFound.into());
    }
    #[cfg(windows)]
    {
        let target = crate::windows::open_apps::app_id_of_other(&key).ok_or(AppError::NotFound)?;
        let opened = tauri::async_runtime::spawn_blocking(move || apps::launch(&target)).await;
        match opened {
            Ok(Ok(())) => Ok(()),
            _ => {
                log::warn!("apps: a program could not be opened");
                Err(AppError::LaunchFailed.into())
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        Err(AppError::LaunchFailed.into())
    }
}

/// The badge of an open program that is not pinned (spec 0009): the photo of
/// the Chrome profile its windows belong to, as a small PNG `data:` URL, drawn
/// on the corner of its icon the way the Windows taskbar does. `null` for a
/// program with none. The program is named by the `key` the dock was given.
#[tauri::command]
pub async fn open_badge(key: String) -> Result<Option<String>, AppCommandError> {
    if !is_open_key_text(&key) {
        return Err(AppError::NotFound.into());
    }
    #[cfg(windows)]
    {
        let Some(aumid) = crate::windows::open_apps::aumid_of_other(&key) else {
            return Ok(None);
        };
        let made = tauri::async_runtime::spawn_blocking(move || {
            crate::windows::profiles::badge_png(&aumid)
        })
        .await
        .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
        Ok(made.map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png))))
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

/// Asks the windows of the open pinned program `id` to close, as the close
/// button of each does (spec 0009 D3). How many it asked; 0 when it has none.
#[tauri::command]
pub fn close_app(app: AppHandle, id: String) -> Result<usize, AppCommandError> {
    let id = WidgetId::parse(&id).map_err(|_| AppCommandError::from(AppError::NotFound))?;
    #[cfg(windows)]
    {
        let workspace = in_use(&app);
        Ok(crate::windows::open_apps::close_pinned(
            &app,
            id.as_str(),
            workspace.as_deref(),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (app, id);
        Ok(0)
    }
}

/// The same for an open program that is not pinned, by its `key`.
#[tauri::command]
pub fn close_open(app: AppHandle, key: String) -> Result<usize, AppCommandError> {
    if !is_open_key_text(&key) {
        return Err(AppError::NotFound.into());
    }
    #[cfg(windows)]
    {
        let workspace = in_use(&app);
        Ok(crate::windows::open_apps::close_other(
            &app,
            &key,
            workspace.as_deref(),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(0)
    }
}

/// Pins the open program `key` that is not pinned (spec 0009 D3), at the end
/// of the pinned ones. The id and the name are the ones Rust described when it
/// showed the program, so the page names nothing but the opaque key.
#[tauri::command]
pub fn pin_open(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    key: String,
) -> Result<PinnedApp, AppCommandError> {
    if !is_open_key_text(&key) {
        return Err(AppError::NotFound.into());
    }
    #[cfg(windows)]
    {
        let (app_id, name) =
            crate::windows::open_apps::pin_target_of_other(&key).ok_or(AppError::NotFound)?;
        if !is_valid_app_id(&app_id) || !is_valid_app_name(&name) {
            return Err(AppError::InvalidApp.into());
        }
        let pinned = change(&app, &state, "pin open", |storage| {
            pin(storage, &app_id, &name, None)
        })?;
        // It is pinned now: the dock shows it with the pinned ones.
        crate::windows::open_apps::wake(&app);
        Ok(pinned)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, state);
        Err(AppError::NotFound.into())
    }
}

/// What the dock was given as the key of an open program: 16 hexadecimal
/// digits.
fn is_open_key_text(key: &str) -> bool {
    key.len() == 16 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One entry of the menu of a program, as the panel shows it: the key names
/// it for `run_app_entry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuEntryView {
    pub key: u32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuSectionView {
    pub kind: SectionKind,
    /// The name of the program's own category.
    pub title: Option<String>,
    pub entries: Vec<MenuEntryView>,
}

/// What `app_menu` returns: the Jump List of the program (spec 0006 §6) and
/// whether its file can be shown in Explorer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMenuView {
    pub sections: Vec<MenuSectionView>,
    pub can_reveal: bool,
}

/// The menu last shown and what each of its keys runs. Only an entry of this
/// menu runs, so the page never names a link or a path: it names a key.
struct ShownMenu {
    app: WidgetId,
    runs: Vec<Runnable>,
}

static MENU: Mutex<Option<ShownMenu>> = Mutex::new(None);

/// The sections as the page gets them, with a key for each entry, and what
/// each key runs.
fn view_of(sections: Vec<Section>) -> (Vec<MenuSectionView>, Vec<Runnable>) {
    let mut runs = Vec::new();
    let views = sections
        .into_iter()
        .map(|section| MenuSectionView {
            kind: section.kind,
            title: section.title,
            entries: section
                .entries
                .into_iter()
                .map(|entry| {
                    let key = runs.len() as u32;
                    runs.push(entry.run);
                    MenuEntryView {
                        key,
                        title: entry.title,
                    }
                })
                .collect(),
        })
        .collect();
    (views, runs)
}

/// The run of `key` in the menu last shown for the pinned program `app`.
fn run_of(app: &WidgetId, key: u32) -> Result<Runnable, AppError> {
    let shown = MENU.lock().unwrap_or_else(PoisonError::into_inner);
    shown
        .as_ref()
        .filter(|menu| &menu.app == app)
        .and_then(|menu| menu.runs.get(key as usize))
        .cloned()
        .ok_or(AppError::NotFound)
}

/// The menu of the pinned program `id`, as the taskbar shows it: its
/// categories, the recent and frequent destinations and its tasks. Runs off
/// the main thread: it reads the program's list from disk.
#[tauri::command]
pub async fn app_menu(
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<AppMenuView, AppCommandError> {
    let pinned = WidgetId::parse(&id).map_err(AppError::from)?;
    let target = target_of(&mut lock(&state), &id).inspect_err(|error| {
        log::warn!("apps: menu rejected: {error}");
    })?;
    let can_reveal = apps::file_of(&target).is_some();
    let read = tauri::async_runtime::spawn_blocking(move || jumplist::menu(&target)).await;
    let Ok(sections) = read else {
        log::warn!("apps: the menu task did not finish");
        return Err(AppError::Unavailable.into());
    };
    let (sections, runs) = view_of(sections);
    *MENU.lock().unwrap_or_else(PoisonError::into_inner) = Some(ShownMenu { app: pinned, runs });
    Ok(AppMenuView {
        sections,
        can_reveal,
    })
}

/// Runs an entry of the menu `app_menu` last returned for the pinned program
/// `id`: a task, a recent document or a category's item.
#[tauri::command]
pub async fn run_app_entry(id: String, key: u32) -> Result<(), AppCommandError> {
    let pinned = WidgetId::parse(&id).map_err(AppError::from)?;
    let run = run_of(&pinned, key).inspect_err(|error| {
        log::warn!("apps: entry rejected: {error}");
    })?;
    let opened = tauri::async_runtime::spawn_blocking(move || jumplist::run(&run)).await;
    match opened {
        Ok(Ok(())) => Ok(()),
        _ => {
            log::warn!("apps: a menu entry could not be opened");
            Err(AppError::LaunchFailed.into())
        }
    }
}

/// Shows the file of the pinned program `id` in File Explorer, for the
/// programs that have one (`app_menu` says which).
#[tauri::command]
pub fn reveal_app(state: State<'_, Mutex<Storage>>, id: String) -> Result<(), AppCommandError> {
    let target = target_of(&mut lock(&state), &id).inspect_err(|error| {
        log::warn!("apps: reveal rejected: {error}");
    })?;
    let path = apps::file_of(&target).ok_or(AppError::NotFound)?;
    apps::reveal(&path).map_err(|_| {
        log::warn!("apps: a file could not be shown");
        AppCommandError::from(AppError::LaunchFailed)
    })
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

        let pinned = pin(&mut storage, "Some.App!App", "Some App", None).unwrap();

        let snapshot = snapshot(&storage).unwrap();
        assert_eq!(snapshot.apps, vec![pinned.clone()]);
        assert_eq!(
            target_of(&mut storage, pinned.id.as_str()),
            Ok("Some.App!App".to_owned())
        );
    }

    #[test]
    fn a_program_is_placed_where_it_was_dropped() {
        let (_dir, mut storage) = open();
        let a = pin(&mut storage, "a!App", "A", None).unwrap();
        pin(&mut storage, "b!App", "B", None).unwrap();
        pin(&mut storage, "c!App", "C", None).unwrap();

        place(&mut storage, a.id.as_str(), 2).unwrap();

        let names: Vec<_> = snapshot(&storage)
            .unwrap()
            .apps
            .into_iter()
            .map(|app| app.name)
            .collect();
        assert_eq!(names, ["B", "C", "A"]);
        assert_eq!(place(&mut storage, "1", 0), Err(AppError::NotFound));
    }

    #[test]
    fn only_programs_and_shortcuts_that_exist_are_taken_from_a_drop() {
        let dir = tempdir().unwrap();
        let exe = dir.path().join("Tool.exe");
        let lnk = dir.path().join("Other.LNK");
        let text = dir.path().join("notes.txt");
        for file in [&exe, &lnk, &text] {
            std::fs::write(file, b"x").unwrap();
        }
        let missing = dir.path().join("Gone.exe");

        let taken = droppable(&[
            exe.clone(),
            text,
            missing,
            lnk.clone(),
            // The same file twice, in another case.
            std::path::PathBuf::from(exe.to_string_lossy().to_uppercase()),
            dir.path().to_path_buf(),
        ]);

        let as_text = |path: &std::path::Path| path.to_string_lossy().into_owned();
        assert_eq!(taken, [as_text(&exe), as_text(&lnk)]);
    }

    #[test]
    fn an_id_that_is_not_one_rust_made_is_not_found() {
        let (_dir, mut storage) = open();
        pin(&mut storage, "Some.App!App", "Some App", None).unwrap();

        assert_eq!(unpin(&mut storage, "1", None), Err(AppError::NotFound));
        assert_eq!(
            shift(&mut storage, "../x", MoveDirection::Up, None),
            Err(AppError::NotFound)
        );
        assert_eq!(
            target_of(&mut storage, "Some.App!App"),
            Err(AppError::NotFound)
        );
    }

    #[test]
    fn the_launcher_knows_whether_the_workspace_in_use_has_a_program_pinned() {
        let (_dir, mut storage) = open();
        assert_eq!(pinned_here(&mut storage, "Some.App!App"), Ok(None));

        let pinned = pin(&mut storage, "Some.App!App", "Some App", None).unwrap();
        // "Desafixar": the same program, whatever the case of its id.
        assert_eq!(
            pinned_here(&mut storage, "some.app!app"),
            Ok(Some(pinned.id.clone()))
        );
        assert_eq!(pinned_here(&mut storage, "Other!App"), Ok(None));

        unpin(&mut storage, pinned.id.as_str(), None).unwrap();
        // "Fixar no dock" again.
        assert_eq!(pinned_here(&mut storage, "Some.App!App"), Ok(None));
    }

    #[test]
    fn a_removed_program_cannot_be_opened() {
        let (_dir, mut storage) = open();
        let pinned = pin(&mut storage, "Some.App!App", "Some App", None).unwrap();

        unpin(&mut storage, pinned.id.as_str(), None).unwrap();

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
            pin(&mut storage, "Some.App!App", "Some App", None),
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
            remember_icon((None, format!("bounded-{n}"), 32, false), None);
        }

        let len = ICONS.lock().unwrap().as_ref().map_or(0, HashMap::len);
        assert!(len <= ICON_CACHE_LIMIT, "{len}");
        assert_eq!(
            cached_icon(&(None, format!("bounded-{ICON_CACHE_LIMIT}"), 32, false)),
            Some(None)
        );
    }

    use crate::windows::jumplist::Entry;

    fn section(kind: SectionKind, titles: &[&str]) -> Section {
        Section {
            kind,
            title: None,
            entries: titles
                .iter()
                .map(|title| Entry {
                    title: (*title).to_owned(),
                    run: Runnable::Open(format!("open {title}")),
                })
                .collect(),
        }
    }

    #[test]
    fn every_entry_gets_a_key_that_runs_what_it_shows() {
        let (views, runs) = view_of(vec![
            section(SectionKind::Recent, &["a", "b"]),
            section(SectionKind::Tasks, &["c"]),
        ]);

        let keys: Vec<Vec<u32>> = views
            .iter()
            .map(|view| view.entries.iter().map(|entry| entry.key).collect())
            .collect();
        assert_eq!(keys, vec![vec![0, 1], vec![2]]);
        assert_eq!(runs[2], Runnable::Open("open c".into()));
    }

    #[test]
    fn only_the_last_menu_of_that_program_runs() {
        let first = WidgetId::new();
        let other = WidgetId::new();
        *MENU.lock().unwrap() = Some(ShownMenu {
            app: first.clone(),
            runs: vec![Runnable::Open("x".into())],
        });

        assert_eq!(run_of(&first, 0), Ok(Runnable::Open("x".into())));
        assert_eq!(run_of(&first, 1), Err(AppError::NotFound));
        assert_eq!(run_of(&other, 0), Err(AppError::NotFound));
        *MENU.lock().unwrap() = None;
        assert_eq!(run_of(&first, 0), Err(AppError::NotFound));
    }

    #[test]
    fn the_menu_goes_to_the_page_as_the_contract_file_says() {
        let entry = |title: &str| Entry {
            title: title.to_owned(),
            run: Runnable::Open(format!("never sent: {title}")),
        };
        let (sections, _) = view_of(vec![
            Section {
                kind: SectionKind::Category,
                title: Some("Recent chats".into()),
                entries: vec![entry("First"), entry("Second")],
            },
            Section {
                kind: SectionKind::Recent,
                title: None,
                entries: vec![entry("notes.txt")],
            },
            Section {
                kind: SectionKind::Tasks,
                title: None,
                entries: vec![entry("New window")],
            },
        ]);

        let json = serde_json::to_value(AppMenuView {
            sections,
            can_reveal: true,
        })
        .unwrap();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/apps/menu-sample.json");
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(json, expected);
        // What an entry runs never goes to the page.
        assert!(!json.to_string().contains("never sent"));
    }
}
