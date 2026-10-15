//! The widget database as app state: opened once at startup, read by both
//! windows and changed only by the panel (spec 0003 §2.3 item 3, §2.7, ADR
//! 0008). Rust is the source of truth; the windows apply what
//! `widgets-changed` carries.
//!
//! The commands only adapt: the rules live in the functions below, which take
//! a [`Storage`] and are tested without Tauri. Logs name only the capability
//! and the kind of error, never an ID, a label, a config or note text.

use std::fmt;
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
pub fn open_storage<E: fmt::Display>(data_dir: Result<PathBuf, E>) -> Storage {
    match data_dir {
        Ok(dir) => Storage::open(&dir),
        Err(error) => {
            log::warn!("widgets: data folder unavailable, widgets disabled: {error}");
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

/// Adds an instance of `kind` at the end of the dock. A name that is not a
/// kind can't be created either: `notCreatable`.
pub fn add(storage: &mut Storage, kind: &str) -> Result<WidgetInstance, WidgetError> {
    let conn = storage.connection()?;
    let kind = WidgetKind::from_name(kind).ok_or(WidgetError::NotCreatable)?;
    widgets_repo::add(conn, kind)
}

/// Removes an instance; a note's text goes with it.
pub fn remove(storage: &mut Storage, id: &str) -> Result<(), WidgetError> {
    let conn = storage.connection()?;
    widgets_repo::remove(conn, &parse_id(id)?)
}

/// Moves an instance one place up or down; at either end nothing changes.
pub fn shift(storage: &mut Storage, id: &str, direction: MoveDirection) -> Result<(), WidgetError> {
    let conn = storage.connection()?;
    widgets_repo::move_widget(conn, &parse_id(id)?, direction)
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

fn lock<'a>(state: &'a State<'_, Mutex<Storage>>) -> MutexGuard<'a, Storage> {
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
) -> Result<WidgetInstance, WidgetCommandError> {
    change(&app, &state, "add", |storage| add(storage, &kind))
}

#[tauri::command]
pub fn remove_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "remove", |storage| remove(storage, &id))
}

#[tauri::command]
pub fn move_widget(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    direction: MoveDirection,
) -> Result<(), WidgetCommandError> {
    change(&app, &state, "move", |storage| {
        shift(storage, &id, direction)
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
    type AddCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
    ) -> Result<WidgetInstance, WidgetCommandError>;
    type IdCommand =
        fn(AppHandle, State<'static, Mutex<Storage>>, String) -> Result<(), WidgetCommandError>;
    type MoveCommand = fn(
        AppHandle,
        State<'static, Mutex<Storage>>,
        String,
        MoveDirection,
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

    fn ids(storage: &Storage) -> Vec<String> {
        snapshot(storage)
            .unwrap()
            .widgets
            .iter()
            .map(|instance| instance.id.to_string())
            .collect()
    }

    fn clock_config() -> Value {
        json!({
            "kind": "clock",
            "config": { "timeZone": "America/Sao_Paulo", "hourCycle": "h12", "showSeconds": true }
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
    fn storage_opens_in_the_data_folder_with_the_seed() {
        let dir = tempdir().unwrap();
        let (storage, _) = open(&dir);

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert!(dir.path().join("dockmax.db").is_file());
    }

    #[test]
    fn without_a_data_folder_the_app_opens_with_the_widgets_unavailable() {
        let storage = open_storage(Err("unknown path"));

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

        assert_eq!(add(&mut storage, "clock").map(|_| ()), expected);
        assert_eq!(remove(&mut storage, STRANGER), expected);
        assert_eq!(shift(&mut storage, STRANGER, MoveDirection::Up), expected);
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
            assert_eq!(remove(&mut storage, id), not_found, "{id:?}");
            assert_eq!(
                shift(&mut storage, id, MoveDirection::Down),
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
                add(&mut storage, kind).map(|_| ()),
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

        let added = add(&mut storage, "clock").unwrap().id.to_string();
        assert_eq!(
            ids(&storage),
            [
                clock.as_str(),
                progress.as_str(),
                note.as_str(),
                added.as_str()
            ]
        );

        shift(&mut storage, &added, MoveDirection::Up).unwrap();
        shift(&mut storage, &clock, MoveDirection::Down).unwrap();
        assert_eq!(
            ids(&storage),
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
            ids(&storage),
            [
                progress.as_str(),
                clock.as_str(),
                added.as_str(),
                note.as_str()
            ]
        );

        remove(&mut storage, &progress).unwrap();
        assert_eq!(
            ids(&storage),
            [clock.as_str(), added.as_str(), note.as_str()]
        );

        // The order and the change survive reopening.
        drop(storage);
        let reopened = open_storage::<String>(Ok(dir.path().to_path_buf()));
        assert_eq!(
            ids(&reopened),
            [clock.as_str(), added.as_str(), note.as_str()]
        );
        let listed = snapshot(&reopened).unwrap().widgets;
        assert_eq!(listed[1], updated);
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
            json!({ "kind": "clock", "config": { "hourCycle": "h12", "showSeconds": true } }),
            json!({ "kind": "clock", "config": { "timeZone": null, "showSeconds": true } }),
            json!({ "kind": "clock", "config": {} }),
            json!({ "kind": "clock" }),
            json!({ "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false } }),
            // Unknown fields: never dropped.
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false,
                                "format": "long" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "show_seconds": false } }),
            json!({ "kind": "clock", "label": "x",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": false } }),
            // Mistyped fields and unknown names.
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h23", "showSeconds": "yes" } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": 3, "hourCycle": "h23", "showSeconds": false } }),
            json!({ "kind": "clock",
                    "config": { "timeZone": null, "hourCycle": "h24", "showSeconds": false } }),
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
            "config": { "timeZone": "Mars/Olympus", "hourCycle": "h23", "showSeconds": false }
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

        let changed = apply(&mut storage, "add", |storage| add(storage, "note")).unwrap();

        let snapshot = changed.snapshot.expect("a snapshot to emit");
        assert_eq!(snapshot.storage, StorageStatus::Ok);
        assert_eq!(snapshot.widgets.len(), 4);
        assert_eq!(snapshot.widgets[3], changed.value);
    }

    #[test]
    fn a_rejected_change_sends_nothing() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        let result = apply(&mut storage, "remove", |storage| remove(storage, STRANGER));

        assert_eq!(
            result.map(|changed| changed.value),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn dismissing_the_notice_of_a_healthy_database_changes_nothing() {
        let dir = tempdir().unwrap();
        let (mut storage, _) = open(&dir);

        assert_eq!(dismiss_notice(&mut storage), Ok(()));
        assert_eq!(storage.status(), StorageStatus::Ok);
    }
}
