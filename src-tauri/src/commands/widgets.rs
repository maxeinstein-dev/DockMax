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
