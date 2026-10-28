//! The widget database as app state: opened once at startup, read by both
//! windows and changed only by the panel (spec 0003 §2.3 item 3, §2.7, ADR
//! 0008). Rust is the source of truth; the windows apply what
//! `widgets-changed` carries.
//!
//! The commands only adapt: the rules live in the functions below, which take
//! a [`Storage`] and are tested without Tauri. Logs name only the capability
//! and the kind of error, never an ID, a label, a config or note text.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::domain::widgets::{
    MoveDirection, UnavailableReason, WidgetConfig, WidgetError, WidgetErrorCode, WidgetId,
    WidgetInstance, WidgetKind, WidgetsSnapshot,
};
use crate::persistence::database::Storage;
use crate::persistence::{notes_repo, widgets_repo};

/// Event sent to every window with the full [`WidgetsSnapshot`] after a
/// change. Saving a note's text does not send it: the text is not part of the
/// snapshot.
pub const CHANGED_EVENT: &str = "widgets-changed";

/// How a widget command fails: `{ "code": "notFound" }`, matching
/// `WidgetCommandError` in `src/core/widgets.ts`. The error carries no path
/// and no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetCommandError {
    code: WidgetErrorCode,
}

impl From<WidgetError> for WidgetCommandError {
    fn from(error: WidgetError) -> Self {
        WidgetCommandError { code: error.code() }
    }
}

/// Opens the widget database in the app data folder, `data_dir` as Tauri
/// resolved it (redirected in the e2e build). Never fails: without a folder
/// the widgets are unavailable in this run, with a warning in the log.
///
/// The error is not logged, and `E` has no bound so it can't be: its text
/// may hold the folder, which the e2e override can point anywhere.
pub fn open_storage<E>(data_dir: Result<PathBuf, E>) -> Storage {
    match data_dir {
        Ok(dir) => Storage::open(&dir),
        Err(_) => {
            log::warn!("widgets: data folder unavailable, widgets disabled");
            Storage::unavailable(UnavailableReason::OpenFailed)
        }
    }
}

/// Opens the database and manages it as state. Runs in the `dockmax-startup`
/// plugin, after the settings, so both windows find it.
pub fn setup(app: &AppHandle) {
    let storage = open_storage(app.path().app_data_dir());
    app.manage(Mutex::new(storage));
}

/// The text of an ID from the interface as a [`WidgetId`]: anything but the
/// exact form Rust makes can't name an instance, so it is `notFound`.
fn parse_id(id: &str) -> Result<WidgetId, WidgetError> {
    WidgetId::parse(id)
}

/// The label `update_widget` takes: `null` (the kind's name) or text. The
/// rules of the text are checked when it is saved.
fn parse_label(label: Value) -> Result<Option<String>, WidgetError> {
    match label {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        _ => Err(WidgetError::InvalidLabel),
    }
}

/// The config `update_widget` takes: `{ "kind": ..., "config": {...} }` with
/// every field and nothing more.
///
/// Stricter than [`WidgetConfig`]'s own deserialization, which fills missing
/// fields with defaults and ignores unknown ones so a stored config from an
/// older build still reads. From the interface, a missing field would
/// silently reset that setting and an unknown one would be dropped, so the
/// payload must be exactly what the parsed config serializes back to
/// (`invalidConfig` otherwise).
fn parse_config(config: Value) -> Result<WidgetConfig, WidgetError> {
    let parsed: WidgetConfig =
        serde_json::from_value(config.clone()).map_err(|_| WidgetError::InvalidConfig)?;
    let canonical = serde_json::to_value(&parsed).map_err(|_| WidgetError::InvalidConfig)?;
    if canonical != config {
        return Err(WidgetError::InvalidConfig);
    }
    Ok(parsed)
}

/// Every instance in dock order, with the storage status. Without a database
/// the list is empty and the status says why.
pub fn snapshot(storage: &Storage) -> Result<WidgetsSnapshot, WidgetError> {
    storage.snapshot()
}

/// The workspace a change goes to (spec 0013): the one named, which must
/// exist, or the one in use (the dock used last).
pub(crate) fn workspace_of(
    storage: &mut Storage,
    workspace: Option<&str>,
) -> Result<String, WidgetError> {
    match workspace {
        Some(id) => storage
            .workspaces()?
            .into_iter()
            .find(|known| known.id == id)
            .map(|known| known.id)
            .ok_or(WidgetError::NotFound),
        None => Ok(storage.in_use()?.id),
    }
}

/// `notFound` unless the workspace `workspace` has the item `key`.
pub(crate) fn in_workspace(
    storage: &mut Storage,
    workspace: &str,
    key: &str,
) -> Result<(), WidgetError> {
    let has = storage
        .workspaces()?
        .iter()
        .any(|known| known.id == workspace && known.items.iter().any(|item| item == key));
    if has {
        Ok(())
    } else {
        Err(WidgetError::NotFound)
    }
}

/// The key a workspace knows a widget by.
fn key_of(id: &WidgetId) -> String {
    format!("widget:{}", id.as_str())
}

/// Adds an instance of `kind` at the end of the workspace `workspace` (or
/// the one in use). A name that is not a kind can't be created either:
/// `notCreatable`.
pub fn add(
    storage: &mut Storage,
    kind: &str,
    workspace: Option<&str>,
) -> Result<WidgetInstance, WidgetError> {
    let kind = WidgetKind::from_name(kind).ok_or(WidgetError::NotCreatable)?;
    let workspace = workspace_of(storage, workspace)?;
    let instance = widgets_repo::add(storage.connection()?, kind)?;
    storage.add_to_workspace(&workspace, &key_of(&instance.id))?;
    Ok(instance)
}

/// Takes an instance out of the workspace `workspace` (or the one in use);
/// with no other workspace showing it, it is removed for good, a note's text
/// with it.
pub fn remove(storage: &mut Storage, id: &str, workspace: Option<&str>) -> Result<(), WidgetError> {
    let id = parse_id(id)?;
    let workspace = workspace_of(storage, workspace)?;
    let key = key_of(&id);
    in_workspace(storage, &workspace, &key)?;
    if storage.remove_from_workspace(&workspace, &key)? {
        return Ok(());
    }
    widgets_repo::remove(storage.connection()?, &id)?;
    storage.forget_item(&key)?;
    Ok(())
}

/// Shows the instance `id` in the workspace `workspace` too, at its end
/// (spec 0013 C3): still one instance, a note's text the same in each.
/// Taking it out of one is `remove`.
pub fn share(storage: &mut Storage, id: &str, workspace: &str) -> Result<(), WidgetError> {
    let id = parse_id(id)?;
    let workspace = workspace_of(storage, Some(workspace))?;
    let key = key_of(&id);
    let known = storage
        .workspaces()?
        .iter()
        .any(|each| each.items.contains(&key));
    if !known {
        return Err(WidgetError::NotFound);
    }
    storage.add_to_workspace(&workspace, &key)?;
    Ok(())
}

/// Moves an instance one place up or down among the widgets of the
/// workspace `workspace` (or the one in use); at either end nothing changes.
pub fn shift(
    storage: &mut Storage,
    id: &str,
    direction: MoveDirection,
    workspace: Option<&str>,
) -> Result<(), WidgetError> {
    let id = parse_id(id)?;
    let workspace = workspace_of(storage, workspace)?;
    let key = key_of(&id);
    in_workspace(storage, &workspace, &key)?;
    storage.shift_in_workspace(&workspace, &key, direction == MoveDirection::Up)?;
    Ok(())
}

/// Replaces an instance's label and config, both given in full.
pub fn update(
    storage: &mut Storage,
    id: &str,
    label: Value,
    config: Value,
) -> Result<WidgetInstance, WidgetError> {
    let conn = storage.connection()?;
    let id = parse_id(id)?;
    let label = parse_label(label)?;
    let config = parse_config(config)?;
    widgets_repo::update(conn, &id, label, config)
}

/// Sets how many icons wide the instance `id` is, keeping the rest of its
/// label and config (spec 0017 F1b). A width its kind doesn't take is
/// [`WidgetError::InvalidWidth`].
pub fn set_width(
    storage: &mut Storage,
    id: &WidgetId,
    width: u8,
) -> Result<WidgetInstance, WidgetError> {
    let instance = storage
        .snapshot()?
        .widgets
        .into_iter()
        .find(|instance| &instance.id == id)
        .ok_or(WidgetError::NotFound)?;
    let config = instance
        .config
        .with_width(width)
        .ok_or(WidgetError::InvalidWidth)?;
    config.validate()?;
    widgets_repo::update(storage.connection()?, id, instance.label, config)
}

/// The width chosen in the dock's menu: saved, and every window and dock
/// told. A refusal is only logged; the dock stays as it was.
pub fn resize(app: &AppHandle, id: &WidgetId, width: u8) {
    let state = app.state::<Mutex<Storage>>();
    // `change` logs a refusal.
    let _ = change(app, &state, "resize", |storage| {
        set_width(storage, id, width)
    });
}

/// The text of the note `id`.
pub fn read_note(storage: &mut Storage, id: &str) -> Result<String, WidgetError> {
    let conn = storage.connection()?;
    notes_repo::read(conn, &parse_id(id)?)
}

/// Replaces the text of the note `id`.
pub fn write_note(storage: &mut Storage, id: &str, body: &str) -> Result<(), WidgetError> {
    let conn = storage.connection()?;
    notes_repo::save(conn, &parse_id(id)?, body)
}

/// The kind of the instance `id`, for opening its panel: `notFound` when
/// no instance has this ID (or the storage can't be used in this run, so
/// there is no instance at all), `notCreatable` for a kind that can't be
/// used in this phase. Reads the list, at most `MAX_WIDGETS` rows.
pub fn panel_kind(storage: &Storage, id: &WidgetId) -> Result<WidgetKind, WidgetError> {
    let kind = storage
        .snapshot()?
        .widgets
        .into_iter()
        .find(|instance| &instance.id == id)
        .map(|instance| instance.config.kind())
        .ok_or(WidgetError::NotFound)?;
    if kind.creatable() {
        Ok(kind)
    } else {
        Err(WidgetError::NotCreatable)
    }
}

/// Dismisses the recovery notice: the status goes from `recovered` to `ok`.
pub fn dismiss_notice(storage: &mut Storage) -> Result<(), WidgetError> {
    storage.dismiss_recovery_notice()
}

/// What a change returns, and the snapshot to send to the windows: `None`
/// when the change was saved but the list could not be read back.
#[derive(Debug)]
struct Changed<T> {
    value: T,
    snapshot: Option<WidgetsSnapshot>,
}

/// Runs `change` and, if it was accepted, reads the snapshot every window
/// gets. `what` names the change in the log.
fn apply<T>(
    storage: &mut Storage,
    what: &str,
    change: impl FnOnce(&mut Storage) -> Result<T, WidgetError>,
) -> Result<Changed<T>, WidgetError> {
    let value = change(storage).inspect_err(|error| {
        log::warn!("widgets: {what} rejected: {error}");
    })?;
    let snapshot = storage
        .snapshot()
        .inspect_err(|error| {
            log::warn!("widgets: {what} saved, but the list could not be read: {error}");
        })
        .ok();
    Ok(Changed { value, snapshot })
}

pub(crate) fn lock<'a>(state: &'a State<'_, Mutex<Storage>>) -> MutexGuard<'a, Storage> {
    // Every write runs in a transaction that rolls back if its holder
    // panicked, so the storage stays consistent.
    state
        .inner()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// [`apply`] on the managed storage, then `widgets-changed` to every window.
/// The lock is released before emitting.
fn change<T>(
    app: &AppHandle,
    state: &State<'_, Mutex<Storage>>,
    what: &str,
    change: impl FnOnce(&mut Storage) -> Result<T, WidgetError>,
) -> Result<T, WidgetCommandError> {
    let changed = apply(&mut lock(state), what, change)?;
    if let Some(snapshot) = changed.snapshot
        && let Err(error) = app.emit(CHANGED_EVENT, &snapshot)
    {
        log::warn!("widgets: {what} saved, but failed to notify the windows: {error}");
    }
    // What each workspace shows may have changed with it.
    crate::commands::workspaces::notify(app);
    crate::windows::dock::refresh();
    Ok(changed.value)
}

/// Called by both windows at startup. With the database unavailable, the
/// list is empty and the status says why.
#[tauri::command]
pub fn list_widgets(
    state: State<'_, Mutex<Storage>>,
) -> Result<WidgetsSnapshot, WidgetCommandError> {
    snapshot(&lock(&state)).map_err(|error| {
        log::warn!("widgets: list failed: {error}");
        WidgetCommandError::from(error)
    })
}

/// Sync commands run on the main thread, one at a time, so the events go out
/// in the same order as the changes.
#[tauri::command]
pub fn add_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    kind: String,
    workspace: Option<String>,
) -> Result<WidgetInstance, WidgetCommandError> {
    change(&app, &state, "add", |storage| {
        add(storage, &kind, workspace.as_deref())
    })
}

#[tauri::command]
pub fn remove_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    workspace: Option<String>,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "remove", |storage| {
        remove(storage, &id, workspace.as_deref())
    })
}

#[tauri::command]
pub fn share_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    workspace: String,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "share", |storage| {
        share(storage, &id, &workspace)
    })
}

#[tauri::command]
pub fn move_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
    workspace: Option<String>,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "move", |storage| {
        shift(storage, &id, direction, workspace.as_deref())
    })
}

/// `label` and `config` arrive as JSON so that a missing label is refused
/// (with `Option<String>`, Tauri would read it as `null` and clear the label)
/// and the config is checked strictly by [`parse_config`].
#[tauri::command]
pub fn update_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    label: Value,
    config: Value,
) -> Result<WidgetInstance, WidgetCommandError> {
    change(&app, &state, "update", |storage| {
        update(storage, &id, label, config)
    })
}

#[tauri::command]
pub fn get_note(
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<String, WidgetCommandError> {
    read_note(&mut lock(&state), &id).map_err(|error| {
        log::warn!("widgets: note read rejected: {error}");
        WidgetCommandError::from(error)
    })
}

/// Emits nothing: the text is not part of the snapshot.
#[tauri::command]
pub fn save_note(
    state: State<'_, Mutex<Storage>>,
    id: String,
    body: String,
) -> Result<(), WidgetCommandError> {
    write_note(&mut lock(&state), &id, &body).map_err(|error| {
        log::warn!("widgets: note save rejected: {error}");
        WidgetCommandError::from(error)
    })
}

#[tauri::command]
pub fn dismiss_storage_notice(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "notice dismissal", dismiss_notice)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::domain::widgets::{MAX_NOTE_BYTES, StorageStatus};

    // The signatures the interface relies on. IDs and the update payload must
    // stay plain JSON: typed, a malformed ID or a partial config would fail in
    // Tauri's deserialization, before the rules here. `save_note` and
    // `get_note` take no `AppHandle`: they can't emit.
    type ListCommand =
        fn(State<'static, Mutex<Storage>>) -> Result<WidgetsSnapshot, WidgetCommandError>;
    // The workspace (spec 0013) is optional: without it, the one in use.
    type AddCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
        Option<String>,
    ) -> Result<WidgetInstance, WidgetCommandError>;
    type IdCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
        Option<String>,
    ) -> Result<(), WidgetCommandError>;
    type MoveCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
        MoveDirection,
        Option<String>,
    ) -> Result<(), WidgetCommandError>;
    type UpdateCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
        Value,
        Value,
    ) -> Result<WidgetInstance, WidgetCommandError>;
    type GetNoteCommand =
        fn(State<'static, Mutex<Storage>>, String) -> Result<String, WidgetCommandError>;
    type SaveNoteCommand =
        fn(State<'static, Mutex<Storage>>, String, String) -> Result<(), WidgetCommandError>;
    type DismissCommand =
        fn(AppHandle, State<'static, Mutex<Storage>>) -> Result<(), WidgetCommandError>;
    const _: ListCommand = list_widgets;
    const _: AddCommand = add_widget;
    const _: IdCommand = remove_widget;
    const _: MoveCommand = move_widget;
    const _: UpdateCommand = update_widget;
    const _: GetNoteCommand = get_note;
    const _: SaveNoteCommand = save_note;
    const _: DismissCommand = dismiss_storage_notice;

    /// A canonical v7 ID that names no instance.
    const STRANGER: &str = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071";

    const MALFORMED_IDS: [&str; 4] = [
        "",
        "clock",
        "019A1B2C-3D4E-7F60-8A1B-2C3D4E5F6071",
        "019a1b2c-3d4e-4f60-8a1b-2c3d4e5f6071",
    ];

    /// A new database and the IDs of its seeded clock, time progress and
    /// note, in dock order.
    fn open(dir: &TempDir) -> (Storage, [String; 3]) {
        let storage = open_storage::<String>(Ok(dir.path().to_path_buf()));
        let widgets = snapshot(&storage).unwrap().widgets;
        let kinds: Vec<_> = widgets.iter().map(WidgetInstance::kind).collect();
        assert_eq!(
            kinds,
            [
                WidgetKind::Clock,
                WidgetKind::TimeProgress,
                WidgetKind::Note
            ]
        );
        let ids = [0, 1, 2].map(|index| widgets[index].id.to_string());
        (storage, ids)
    }

    /// The widgets of the workspace in use, in its order (spec 0013).
    fn ids(storage: &mut Storage) -> Vec<String> {
        storage
            .in_use()
            .unwrap()
            .items
            .iter()
            .filter_map(|key| key.strip_prefix("widget:").map(str::to_owned))
            .collect()
    }

    fn clock_config() -> Value {
        json!({
            "kind": "clock",
            "config": { "timeZone": "America/Sao_Paulo", "hourCycle": "h12", "showSeconds": true, "iconStyle": "analog" }
        })
    }

    fn unavailable() -> Storage {
        Storage::unavailable(UnavailableReason::OpenFailed)
    }

    #[test]
    fn command_error_is_only_the_code() {
        let cases = [
            (WidgetError::LimitReached, "limitReached"),
            (WidgetError::NotCreatable, "notCreatable"),
            (WidgetError::NotFound, "notFound"),
            (WidgetError::KindMismatch, "kindMismatch"),
            (WidgetError::InvalidLabel, "invalidLabel"),
            (WidgetError::InvalidTimeZone, "invalidTimeZone"),
            (WidgetError::InvalidPeriods, "invalidPeriods"),
            (WidgetError::InvalidWorkday, "invalidWorkday"),
            (WidgetError::InvalidTarget, "invalidTarget"),
            (WidgetError::InvalidFolder, "invalidFolder"),
            (WidgetError::InvalidWidth, "invalidWidth"),
            (WidgetError::InvalidCalendars, "invalidCalendars"),
            (WidgetError::InvalidCycle, "invalidCycle"),
            (WidgetError::NoteTooLarge, "noteTooLarge"),
            (WidgetError::InvalidConfig, "invalidConfig"),
            (WidgetError::Unavailable, "unavailable"),
            (WidgetError::WriteFailed, "writeFailed"),
        ];
        assert_eq!(cases.len(), WidgetErrorCode::ALL.len());
        for (error, code) in cases {
            assert_eq!(
                serde_json::to_value(WidgetCommandError::from(error)).unwrap(),
                json!({ "code": code }),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_widget_shown_in_two_workspaces_is_one_instance() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, ..]) = open(&dir);
        let first = storage.in_use().unwrap().id;
        let second = storage.create_workspace(None).unwrap().id;

        share(&mut storage, &clock, &second).unwrap();
        // Twice is once.
        share(&mut storage, &clock, &second).unwrap();
        let key = format!("widget:{clock}");
        let holding = |storage: &mut Storage| {
            storage
                .workspaces()
                .unwrap()
                .into_iter()
                .filter(|each| each.items.contains(&key))
                .map(|each| each.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(holding(&mut storage), [first.clone(), second.clone()]);

        // Out of the first, it stays in the second.
        remove(&mut storage, &clock, Some(&first)).unwrap();
        assert_eq!(holding(&mut storage), std::slice::from_ref(&second));
        assert!(
            snapshot(&storage)
                .unwrap()
                .widgets
                .iter()
                .any(|widget| widget.id.to_string() == clock)
        );

        assert_eq!(
            share(&mut storage, STRANGER, &first),
            Err(WidgetError::NotFound)
        );
        assert_eq!(
            share(&mut storage, &clock, STRANGER),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn storage_opens_in_the_data_folder_with_the_seed() {
        let dir = tempdir().unwrap();
        let (storage, _) = open(&dir);

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert!(dir.path().join("dockmax.db").is_file());
    }

    #[test]
    fn without_a_data_folder_the_app_opens_with_the_widgets_unavailable() {
        // Neither `Display` nor `Debug`: this only compiles while the error,
        // whose text may hold the redirected data folder, is never logged.
        struct Unprintable;
        let storage = open_storage(Err::<PathBuf, _>(Unprintable));

        assert_eq!(
            snapshot(&storage),
            Ok(WidgetsSnapshot {
                storage: StorageStatus::Unavailable {
                    reason: UnavailableReason::OpenFailed,
                },
                widgets: Vec::new(),
            })
        );
    }

    #[test]
    fn with_the_storage_unavailable_every_change_is_unavailable() {
        let mut storage = unavailable();
        let expected = Err(WidgetError::Unavailable);

        assert_eq!(add(&mut storage, "clock", None).map(|_| ()), expected);
        assert_eq!(remove(&mut storage, STRANGER, None), expected);
        assert_eq!(
            shift(&mut storage, STRANGER, MoveDirection::Up, None),
            expected
        );
        assert_eq!(
            update(&mut storage, STRANGER, Value::Null, clock_config()).map(|_| ()),
            expected
        );
        assert_eq!(read_note(&mut storage, STRANGER).map(|_| ()), expected);
        assert_eq!(write_note(&mut storage, STRANGER, "text"), expected);
        assert_eq!(dismiss_notice(&mut storage), expected);
        // Still listable, empty, with the status.
        let listed = snapshot(&storage).unwrap();
        assert!(listed.widgets.is_empty());
        assert_eq!(
            listed.storage,
            StorageStatus::Unavailable {
                reason: UnavailableReason::OpenFailed
            }
        );
    }

    #[test]
    fn malformed_ids_are_not_found() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);
        let not_found = Err(WidgetError::NotFound);

        for id in MALFORMED_IDS.into_iter().chain([STRANGER]) {
            assert_eq!(remove(&mut storage, id, None), not_found, "{id:?}");
            assert_eq!(
                shift(&mut storage, id, MoveDirection::Down, None),
                not_found,
                "{id:?}"
            );
            assert_eq!(
                update(&mut storage, id, Value::Null, clock_config()).map(|_| ()),
                not_found,
                "{id:?}"
            );
            assert_eq!(read_note(&mut storage, id).map(|_| ()), not_found, "{id:?}");
            assert_eq!(write_note(&mut storage, id, "text"), not_found, "{id:?}");
        }
        assert_eq!(snapshot(&storage).unwrap().widgets.len(), 3);
    }

    #[test]
    fn only_creatable_kinds_are_added() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        for kind in ["shortcut", "files", "weather", "Clock", "timeProgress", ""] {
            assert_eq!(
                add(&mut storage, kind, None).map(|_| ()),
                Err(WidgetError::NotCreatable),
                "{kind:?}"
            );
        }
        assert_eq!(snapshot(&storage).unwrap().widgets.len(), 3);
    }

    #[test]
    fn add_move_update_remove_keep_the_ids_and_the_order() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, progress, note]) = open(&dir);

        let added = add(&mut storage, "clock", None).unwrap().id.to_string();
        assert_eq!(
            ids(&mut storage),
            [
                clock.as_str(),
                progress.as_str(),
                note.as_str(),
                added.as_str()
            ]
        );

        shift(&mut storage, &added, MoveDirection::Up, None).unwrap();
        shift(&mut storage, &clock, MoveDirection::Down, None).unwrap();
        assert_eq!(
            ids(&mut storage),
            [
                progress.as_str(),
                clock.as_str(),
                added.as_str(),
                note.as_str()
            ]
        );

        let updated = update(&mut storage, &added, json!("São Paulo"), clock_config()).unwrap();
        assert_eq!(updated.id.as_str(), added);
        assert_eq!(updated.label.as_deref(), Some("São Paulo"));
        assert_eq!(
            ids(&mut storage),
            [
                progress.as_str(),
                clock.as_str(),
                added.as_str(),
                note.as_str()
            ]
        );

        remove(&mut storage, &progress, None).unwrap();
        assert_eq!(
            ids(&mut storage),
            [clock.as_str(), added.as_str(), note.as_str()]
        );

        // The order and the change survive reopening.
        drop(storage);
        let mut reopened = open_storage::<String>(Ok(dir.path().to_path_buf()));
        assert_eq!(
            ids(&mut reopened),
            [clock.as_str(), added.as_str(), note.as_str()]
        );
        let listed = snapshot(&reopened).unwrap().widgets;
        assert_eq!(
            listed.iter().find(|widget| widget.id == updated.id),
            Some(&updated)
        );
    }

    #[test]
    fn update_takes_a_null_label_and_rejects_other_shapes() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, ..]) = open(&dir);

        let cleared = update(&mut storage, &clock, Value::Null, clock_config()).unwrap();
        assert_eq!(cleared.label, None);
        for label in [json!(1), json!(true), json!(["a"]), json!({})] {
            assert_eq!(
                update(&mut storage, &clock, label.clone(), clock_config()).map(|_| ()),
                Err(WidgetError::InvalidLabel),
                "{label}"
            );
        }
        assert_eq!(
            update(&mut storage, &clock, json!(""), clock_config()).map(|_| ()),
            Err(WidgetError::InvalidLabel)
        );
    }

    #[test]
    fn update_rejects_a_config_with_missing_unknown_or_mistyped_fields() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, progress, _]) = open(&dir);
        let before = snapshot(&storage).unwrap();

        let rejected = [
            // Missing fields: never filled with their defaults.
            json!({ "kind": "clock", "config": { "hourCycle": "h12", "showSeconds": true, "iconStyle": "digital" } }),
            json!({ "kind": "clock", "config": { "timeZone": null, "showSeconds": true, "iconStyle": "digital" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false } }),
            json!({ "kind": "clock", "config": {} }),
            json!({ "kind": "clock" }),
            json!({ "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false, "iconStyle": "digital" } }),
            // Unknown fields: never dropped.
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false,
                                "iconStyle": "digital", "format": "long" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "show_seconds": false, "iconStyle": "digital" } }),
            json!({ "kind": "clock", "label": "x",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false, "iconStyle": "digital" } }),
            // Mistyped fields and unknown names.
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": "yes", "iconStyle": "digital" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": 3, "hourCycle": "h23", "showSeconds": false, "iconStyle": "digital" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h24", "showSeconds": false, "iconStyle": "digital" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false,
                                "iconStyle": "Analog" } }),
            json!({ "kind": "weather", "config": {} }),
            json!(null),
            json!([]),
            json!("clock"),
        ];
        for config in rejected {
            assert_eq!(
                update(&mut storage, &clock, Value::Null, config.clone()).map(|_| ()),
                Err(WidgetError::InvalidConfig),
                "{config}"
            );
        }
        let partial_workday = json!({
            "kind": "time-progress",
            "config": { "periods": ["day"], "weekStart": "monday", "workday": { "start": "09:00" } }
        });
        assert_eq!(
            update(&mut storage, &progress, Value::Null, partial_workday).map(|_| ()),
            Err(WidgetError::InvalidConfig)
        );
        assert_eq!(snapshot(&storage).unwrap(), before);
    }

    #[test]
    fn update_checks_the_kind_and_the_rules_of_the_config() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, _, note]) = open(&dir);

        assert_eq!(
            update(&mut storage, &note, Value::Null, clock_config()).map(|_| ()),
            Err(WidgetError::KindMismatch)
        );
        let unknown_zone = json!({
            "kind": "clock",
            "config": { "timeZone": "Mars/Olympus", "hourCycle": "h23", "showSeconds": false, "iconStyle": "digital" }
        });
        assert_eq!(
            update(&mut storage, &clock, Value::Null, unknown_zone).map(|_| ()),
            Err(WidgetError::InvalidTimeZone)
        );
        let note_config = json!({ "kind": "note", "config": {} });
        assert!(update(&mut storage, &note, json!("Mercado"), note_config).is_ok());
    }

    #[test]
    fn note_text_reads_back_and_over_64_kib_is_rejected() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, _, note]) = open(&dir);

        assert_eq!(read_note(&mut storage, &note), Ok(String::new()));
        write_note(&mut storage, &note, "Comprar café\nLigar às 15h").unwrap();
        assert_eq!(
            read_note(&mut storage, &note).unwrap(),
            "Comprar café\nLigar às 15h"
        );

        let over = "a".repeat(MAX_NOTE_BYTES + 1);
        assert_eq!(
            write_note(&mut storage, &note, &over),
            Err(WidgetError::NoteTooLarge)
        );
        assert_eq!(
            read_note(&mut storage, &note).unwrap(),
            "Comprar café\nLigar às 15h"
        );
        // Only a note has text.
        assert_eq!(
            write_note(&mut storage, &clock, "text"),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn an_accepted_change_comes_with_the_full_snapshot() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        let changed = apply(&mut storage, "add", |storage| add(storage, "note", None)).unwrap();

        let snapshot = changed.snapshot.expect("a snapshot to emit");
        assert_eq!(snapshot.storage, StorageStatus::Ok);
        assert_eq!(snapshot.widgets.len(), 4);
        assert_eq!(snapshot.widgets[3], changed.value);
    }

    #[test]
    fn a_rejected_change_sends_nothing() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        let result = apply(&mut storage, "remove", |storage| {
            remove(storage, STRANGER, None)
        });

        assert_eq!(
            result.map(|changed| changed.value),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn the_width_changes_only_to_one_the_kind_takes() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, progress, _]) = open(&dir);
        let parse = |id: &str| WidgetId::parse(id).unwrap();

        let wider = set_width(&mut storage, &parse(&progress), 3).unwrap();
        assert_eq!(wider.config.width(), 3);
        assert_eq!(snapshot(&storage).unwrap().widgets[1].config.width(), 3);

        assert_eq!(
            set_width(&mut storage, &parse(&progress), 4),
            Err(WidgetError::InvalidWidth)
        );
        assert_eq!(
            set_width(&mut storage, &parse(&clock), 2),
            Err(WidgetError::InvalidWidth)
        );
        assert_eq!(
            set_width(&mut storage, &WidgetId::new(), 2),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn panel_kind_is_the_kind_of_an_existing_instance() {
        let dir = tempdir().unwrap();
        let (mut storage, [clock, progress, note]) = open(&dir);
        let parse = |id: &str| WidgetId::parse(id).unwrap();

        assert_eq!(panel_kind(&storage, &parse(&clock)), Ok(WidgetKind::Clock));
        assert_eq!(
            panel_kind(&storage, &parse(&progress)),
            Ok(WidgetKind::TimeProgress)
        );
        assert_eq!(panel_kind(&storage, &parse(&note)), Ok(WidgetKind::Note));

        remove(&mut storage, &note, None).unwrap();
        assert_eq!(
            panel_kind(&storage, &parse(&note)),
            Err(WidgetError::NotFound)
        );
        assert_eq!(
            panel_kind(&storage, &WidgetId::new()),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn panel_kind_finds_nothing_without_a_database() {
        assert_eq!(
            panel_kind(&unavailable(), &WidgetId::new()),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn panel_kind_never_opens_a_kind_that_cannot_be_used_yet() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);
        let shortcut = WidgetId::new();
        storage
            .connection()
            .unwrap()
            .execute(
                "INSERT INTO widget_instances \
                 (id, kind, label, position, config, config_version, created_at) \
                 VALUES (?1, 'shortcut', NULL, 3, '{}', 1, 0)",
                [shortcut.as_str()],
            )
            .unwrap();

        // The list hides it (`widgets_repo`), so there is no such instance
        // to open; `creatable` stays as a second check.
        assert_eq!(panel_kind(&storage, &shortcut), Err(WidgetError::NotFound));
    }

    #[test]
    fn dismissing_the_notice_of_a_healthy_database_changes_nothing() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        assert_eq!(dismiss_notice(&mut storage), Ok(()));
        assert_eq!(storage.status(), StorageStatus::Ok);
    }
}

/// The names the frontend uses, pinned in `tests/fixtures/widgets/ipc.json`
/// (spec 0003 §2.6): the event, each widget command and its arguments, as
/// `src/core/widgets.ts` sends them. `widgets.contract.test.ts` checks the
/// same file against the TS constants, so renaming one on one side alone
/// breaks one of the two tests.
///
/// The commands are read from the source, as `tests/guards.rs` does: the
/// `#[tauri::command]` functions of this file, the app manifest in
/// `build.rs` and `generate_handler!` in `lib.rs`.
#[cfg(test)]
mod ipc_contract {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use serde_json::Value;

    use super::CHANGED_EVENT;

    /// Command name to its argument names, as the frontend sends them.
    type Commands = BTreeMap<String, Vec<String>>;

    fn read(relative: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{relative}: {error}"))
    }

    fn fixture() -> Value {
        let text = read("tests/fixtures/widgets/ipc.json");
        serde_json::from_str(&text).expect("ipc.json is valid JSON")
    }

    fn fixture_commands() -> Commands {
        let commands = fixture()["commands"]
            .as_object()
            .expect("ipc.json has a `commands` object")
            .clone();
        commands
            .into_iter()
            .map(|(name, args)| {
                let args = args
                    .as_array()
                    .unwrap_or_else(|| panic!("ipc.json: `{name}` lists its arguments"))
                    .iter()
                    .map(|arg| arg.as_str().expect("argument names are text").to_owned())
                    .collect();
                (name, args)
            })
            .collect()
    }

    /// `snake_case` to `camelCase`, as Tauri names a command's arguments by
    /// default.
    fn camel_case(name: &str) -> String {
        let mut camel = String::with_capacity(name.len());
        let mut upper = false;
        for c in name.chars() {
            if c == '_' {
                upper = true;
            } else if upper {
                camel.extend(c.to_uppercase());
                upper = false;
            } else {
                camel.push(c);
            }
        }
        camel
    }

    /// Splits `text` at the commas outside `<>`, `()` and `[]`.
    fn split_top_level(text: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        let mut start = 0;
        for (index, c) in text.char_indices() {
            match c {
                '<' | '(' | '[' => depth += 1,
                '>' | ')' | ']' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    parts.push(&text[start..index]);
                    start = index + 1;
                }
                _ => {}
            }
        }
        parts.push(&text[start..]);
        parts
            .into_iter()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect()
    }

    /// The text between the `(` at the start of `text` and its matching `)`.
    fn parenthesized(text: &str) -> &str {
        let mut depth = 0usize;
        for (index, c) in text.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return &text[1..index];
                    }
                }
                _ => {}
            }
        }
        panic!("unbalanced parentheses in {text:?}");
    }

    /// The code of `source` above the tests, without comment lines.
    fn code_above_tests(source: &str) -> String {
        source
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The `#[tauri::command]` functions of this file with the arguments the
    /// frontend sends.
    fn source_commands() -> Commands {
        commands_in(&read("src/commands/widgets.rs"))
    }

    /// The `#[tauri::command]` functions of `source` with the arguments the
    /// frontend sends: what Tauri injects (`AppHandle`, `State`) is left out.
    /// Also `#[tauri::command(...)]`: arguments to the attribute
    /// (`rename_all`, `async`) don't hide a command from the contract.
    fn commands_in(source: &str) -> Commands {
        let code = code_above_tests(source);
        code.split("#[tauri::command")
            .skip(1)
            .map(|item| {
                let (attribute, item) = item
                    .split_once(']')
                    .expect("a #[tauri::command] attribute is closed");
                assert!(
                    attribute.is_empty() || attribute.starts_with('('),
                    "not a #[tauri::command] attribute: `#[tauri::command{attribute}]`"
                );
                let rename = argument_case(attribute);
                let after_fn = item
                    .split_once("fn ")
                    .expect("a #[tauri::command] marks a function")
                    .1;
                let open = after_fn.find('(').expect("the function has parameters");
                let name = after_fn[..open].trim().to_owned();
                let args = split_top_level(parenthesized(&after_fn[open..]))
                    .into_iter()
                    .filter_map(|param| {
                        let (arg, ty) = param.split_once(':').expect("`name: Type`");
                        let ty = ty.trim();
                        let injected = ty.starts_with("AppHandle") || ty.starts_with("State<");
                        (!injected).then(|| rename(arg.trim()))
                    })
                    .collect();
                (name, args)
            })
            .collect()
    }

    /// How Tauri names the arguments of a command whose attribute takes
    /// `attribute` (`""` or `"(...)"`): `camelCase` unless `rename_all` says
    /// otherwise.
    fn argument_case(attribute: &str) -> fn(&str) -> String {
        let Some((_, rest)) = attribute.split_once("rename_all") else {
            return camel_case;
        };
        let case = rest
            .split('"')
            .nth(1)
            .unwrap_or_else(|| panic!("`rename_all` without a quoted case: {attribute}"));
        match case {
            "camelCase" => camel_case,
            "snake_case" => str::to_owned,
            other => panic!("ipc contract: `rename_all = \"{other}\"` is not supported here"),
        }
    }

    /// The commands the app manifest in `build.rs` declares.
    fn manifest_commands() -> Vec<String> {
        let source = read("build.rs");
        let list = source
            .split(".commands(&[")
            .nth(1)
            .and_then(|rest| rest.split("])").next())
            .expect("build.rs declares the commands with `.commands(&[...])`");
        list.split(',')
            .map(|entry| entry.trim().trim_matches('"').to_owned())
            .filter(|entry| !entry.is_empty())
            .collect()
    }

    /// The widget commands `generate_handler!` registers in `lib.rs`.
    fn registered_commands() -> Vec<String> {
        widget_commands_in(&read("src/lib.rs"), &read("src/commands/widgets.rs"))
    }

    /// The functions of `widgets` (the source of this file) that
    /// `generate_handler!` registers in `lib`, by any path
    /// (`commands::widgets::x`, `crate::commands::widgets::x`, `x`): each
    /// entry counts by its last segment, as `tests/guards.rs` reads it.
    fn widget_commands_in(lib: &str, widgets: &str) -> Vec<String> {
        let code = code_above_tests(widgets);
        let words: Vec<&str> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| !word.is_empty())
            .collect();
        let functions: Vec<&str> = words
            .windows(2)
            .filter(|pair| pair[0] == "fn")
            .map(|pair| pair[1])
            .collect();
        let list = lib
            .split("generate_handler![")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("lib.rs registers the commands with `generate_handler![...]`");
        list.split(',')
            .map(|entry| entry.trim())
            .filter(|entry| !entry.is_empty())
            .map(|path| path.rsplit("::").next().unwrap_or(path).trim())
            .filter(|name| functions.contains(name))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn commands_in_reads_a_command_attribute_with_arguments() {
        let source = r#"
            #[tauri::command]
            pub fn plain(app: AppHandle, time_zone: String) {}

            #[tauri::command(rename_all = "snake_case")]
            pub fn renamed(state: State<'_, Store>, time_zone: String) {}
        "#;
        let expected: Commands = [
            ("plain".to_owned(), vec!["timeZone".to_owned()]),
            ("renamed".to_owned(), vec!["time_zone".to_owned()]),
        ]
        .into();
        assert_eq!(commands_in(source), expected);
    }

    #[test]
    fn widget_commands_in_reads_any_path_to_this_file() {
        let widgets = "
            #[tauri::command]
            pub fn first() {}
            #[tauri::command]
            pub fn second() {}
            #[tauri::command]
            pub fn third() {}
        ";
        let lib = "
            tauri::generate_handler![
                commands::settings::get_settings,
                commands::widgets::first,
                crate::commands::widgets::second,
                widgets::third
            ]
        ";
        let mut registered = widget_commands_in(lib, widgets);
        registered.sort();
        assert_eq!(registered, ["first", "second", "third"]);
    }

    #[test]
    fn camel_case_follows_tauri() {
        assert_eq!(camel_case("id"), "id");
        assert_eq!(camel_case("time_zone"), "timeZone");
        assert_eq!(camel_case("a_b_c"), "aBC");
    }

    #[test]
    fn the_event_is_the_one_in_the_fixture() {
        assert_eq!(fixture()["event"], CHANGED_EVENT);
    }

    #[test]
    fn every_command_and_argument_is_the_one_in_the_fixture() {
        let expected = fixture_commands();
        assert!(!expected.is_empty(), "ipc.json lists no command");
        assert_eq!(source_commands(), expected);
    }

    #[test]
    fn every_widget_command_is_declared_and_registered() {
        let manifest = manifest_commands();
        let mut registered = registered_commands();
        registered.sort();
        let expected: Vec<String> = fixture_commands().into_keys().collect();

        for command in &expected {
            assert!(
                manifest.contains(command),
                "build.rs: `{command}` is not in the app manifest"
            );
        }
        assert_eq!(
            registered, expected,
            "lib.rs: generate_handler! must register exactly the widget commands"
        );
    }
}
