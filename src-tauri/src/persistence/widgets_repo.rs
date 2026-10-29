//! Widget instances in the `widget_instances` table (spec 0003 §2.1, §2.4).
//!
//! Positions are 0-based and contiguous after every write, which runs in one
//! transaction. Reading never fails on a bad row (edited by hand, by another
//! tool or by a newer DockMax):
//!
//! - a config that can't be read gives the kind's default;
//! - a label that isn't text or breaks the label rules reads as no label;
//! - a row of an unknown kind, of a kind not creatable in this version
//!   (`shortcut`, `files`) or with a malformed ID is hidden: left out of the
//!   list, it keeps its place, doesn't count towards the limit and can't be
//!   moved, changed or removed.
//!
//! Either way the row stays as it is until the user changes it (§2.4 item
//! 6), and the log gets only the kind of problem.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use super::database::{begin, now_ms, read_failed, write_failed};
#[cfg(test)]
use crate::domain::widgets::MoveDirection;
use crate::domain::widgets::{
    MAX_WIDGETS, WidgetConfig, WidgetError, WidgetId, WidgetInstance, WidgetKind, decode_config,
    encode_config, validate_label,
};

/// Every instance the interface can see, in dock order.
pub fn list(conn: &Connection) -> Result<Vec<WidgetInstance>, WidgetError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, kind, label, config, config_version FROM widget_instances \
             ORDER BY position, created_at, id",
        )
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredRow {
                id: Text::read(row, 0)?,
                kind: Text::read(row, 1)?,
                label: Text::read(row, 2)?,
                config: Text::read(row, 3)?,
                config_version: match row.get_ref(4)? {
                    ValueRef::Integer(version) => Some(version),
                    _ => None,
                },
            })
        })
        .map_err(read_failed)?;
    let mut instances = Vec::new();
    for row in rows {
        if let Some(instance) = row.map_err(read_failed)?.into_instance() {
            instances.push(instance);
        }
    }
    Ok(instances)
}

/// Adds an instance of `kind` with the default config at the end of the
/// dock.
pub fn add(conn: &mut Connection, kind: WidgetKind) -> Result<WidgetInstance, WidgetError> {
    if !kind.creatable() {
        return Err(WidgetError::NotCreatable);
    }
    let tx = begin(conn)?;
    let order = order(&tx).map_err(read_failed)?;
    if order.iter().filter(|entry| entry.shown.is_some()).count() >= MAX_WIDGETS {
        return Err(WidgetError::LimitReached);
    }
    renumber(&tx, &order).map_err(write_failed)?;
    let instance = insert(&tx, kind, order.len(), now_ms()).map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    Ok(instance)
}

/// Removes an instance; a note's text goes with it (`ON DELETE CASCADE`).
pub fn remove(conn: &mut Connection, id: &WidgetId) -> Result<(), WidgetError> {
    let tx = begin(conn)?;
    let mut order = order(&tx).map_err(read_failed)?;
    let index = find(&order, id)?;
    tx.execute(
        "DELETE FROM widget_instances WHERE id = ?1",
        params![id.as_str()],
    )
    .map_err(write_failed)?;
    order.remove(index);
    renumber(&tx, &order).map_err(write_failed)?;
    tx.commit().map_err(write_failed)
}

/// Moves an instance one place up or down the dock, past the next instance
/// the interface sees. At the start or the end, nothing changes.
#[cfg(test)]
pub fn move_widget(
    conn: &mut Connection,
    id: &WidgetId,
    direction: MoveDirection,
) -> Result<(), WidgetError> {
    let tx = begin(conn)?;
    let mut order = order(&tx).map_err(read_failed)?;
    let index = find(&order, id)?;
    let neighbor = match direction {
        MoveDirection::Up => order[..index]
            .iter()
            .rposition(|entry| entry.shown.is_some()),
        MoveDirection::Down => order[index + 1..]
            .iter()
            .position(|entry| entry.shown.is_some())
            .map(|offset| index + 1 + offset),
    };
    let Some(neighbor) = neighbor else {
        return Ok(());
    };
    order.swap(index, neighbor);
    renumber(&tx, &order).map_err(write_failed)?;
    tx.commit().map_err(write_failed)
}

/// Replaces an instance's label and config. The config must be of the
/// instance's kind and both must be valid; the config is stored at its
/// kind's current version.
pub fn update(
    conn: &mut Connection,
    id: &WidgetId,
    label: Option<String>,
    config: WidgetConfig,
) -> Result<WidgetInstance, WidgetError> {
    let tx = begin(conn)?;
    let kind = kind_of(&tx, id)?.ok_or(WidgetError::NotFound)?;
    if config.kind() != kind {
        return Err(WidgetError::KindMismatch);
    }
    validate_label(label.as_deref())?;
    config.validate()?;
    tx.execute(
        "UPDATE widget_instances SET label = ?1, config = ?2, config_version = ?3 WHERE id = ?4",
        params![
            label,
            encode_config(&config),
            kind.config_version(),
            id.as_str()
        ],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    Ok(WidgetInstance {
        id: id.clone(),
        label,
        config,
    })
}

/// The kind of the instance `id`, if it exists and the interface sees it.
pub(super) fn kind_of(conn: &Connection, id: &WidgetId) -> Result<Option<WidgetKind>, WidgetError> {
    let kind = conn
        .query_row(
            "SELECT kind FROM widget_instances WHERE id = ?1",
            params![id.as_str()],
            |row| Text::read(row, 0),
        )
        .optional()
        .map_err(read_failed)?;
    Ok(kind.and_then(|kind| shown_kind(&kind).ok()))
}

/// Inserts a new instance of `kind` with the default config at `position`,
/// plus the empty text of a note.
pub(super) fn insert(
    tx: &Transaction<'_>,
    kind: WidgetKind,
    position: usize,
    now: i64,
) -> rusqlite::Result<WidgetInstance> {
    let instance = WidgetInstance {
        id: WidgetId::new(),
        label: None,
        config: WidgetConfig::default_for(kind),
    };
    tx.execute(
        "INSERT INTO widget_instances \
         (id, kind, label, position, config, config_version, created_at) \
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
        params![
            instance.id.as_str(),
            kind.as_str(),
            position as i64,
            encode_config(&instance.config),
            kind.config_version(),
            now
        ],
    )?;
    if kind == WidgetKind::Note {
        tx.execute(
            "INSERT INTO notes (instance_id, body, updated_at) VALUES (?1, '', ?2)",
            params![instance.id.as_str(), now],
        )?;
    }
    Ok(instance)
}

/// A TEXT column as stored. SQLite keeps whatever bytes it is given, so
/// text that is not UTF-8 is possible and must not fail the whole read.
enum Text {
    Null,
    Valid(String),
    /// Not UTF-8, or not text at all.
    Invalid,
}

impl Text {
    fn read(row: &Row<'_>, index: usize) -> rusqlite::Result<Text> {
        Ok(match row.get_ref(index)? {
            ValueRef::Null => Text::Null,
            ValueRef::Text(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) => Text::Valid(text.to_owned()),
                Err(_) => Text::Invalid,
            },
            _ => Text::Invalid,
        })
    }
}

/// Why a row is hidden, for the log: never the stored value.
#[derive(Debug, Clone, Copy)]
enum Hidden {
    UnreadableKind,
    UnknownKind,
    NotCreatable,
    InvalidId,
}

impl Hidden {
    fn log(self) {
        let why = match self {
            Hidden::UnreadableKind => "a kind that is not text",
            Hidden::UnknownKind => "an unknown kind",
            Hidden::NotCreatable => "a kind not available in this version",
            Hidden::InvalidId => "an invalid ID",
        };
        log::warn!("widgets: ignored an instance with {why}");
    }
}

/// The kind of a stored row, if the interface sees rows of it: known and
/// creatable in this version.
fn shown_kind(kind: &Text) -> Result<WidgetKind, Hidden> {
    let Text::Valid(name) = kind else {
        return Err(Hidden::UnreadableKind);
    };
    match WidgetKind::from_name(name) {
        Some(kind) if kind.creatable() => Ok(kind),
        Some(_) => Err(Hidden::NotCreatable),
        None => Err(Hidden::UnknownKind),
    }
}

/// The ID of a stored row, if it is one Rust makes.
fn shown_id(id: &Text) -> Result<WidgetId, Hidden> {
    match id {
        Text::Valid(id) => WidgetId::parse(id).map_err(|_| Hidden::InvalidId),
        _ => Err(Hidden::InvalidId),
    }
}

/// A row as stored, before it is checked.
struct StoredRow {
    id: Text,
    kind: Text,
    label: Text,
    config: Text,
    config_version: Option<i64>,
}

impl StoredRow {
    /// The instance the interface gets, or `None` for a row it can't use.
    /// Logs only the kind of problem, never the stored values.
    fn into_instance(self) -> Option<WidgetInstance> {
        let shown = shown_kind(&self.kind).and_then(|kind| Ok((kind, shown_id(&self.id)?)));
        let (kind, id) = match shown {
            Ok(shown) => shown,
            Err(hidden) => {
                hidden.log();
                return None;
            }
        };
        let label = match self.label {
            Text::Null => None,
            Text::Valid(label) if validate_label(Some(&label)).is_ok() => Some(label),
            Text::Valid(_) | Text::Invalid => {
                log::warn!("widgets: ignored an invalid {kind} label");
                None
            }
        };
        let config = match (self.config_version, &self.config) {
            (Some(version), Text::Valid(json)) => u32::try_from(version)
                .map_err(|_| WidgetError::InvalidConfig)
                .and_then(|version| decode_config(kind, version, json)),
            _ => Err(WidgetError::InvalidConfig),
        }
        .unwrap_or_else(|error| {
            log::warn!("widgets: using the default {kind} config: {error}");
            WidgetConfig::default_for(kind)
        });
        Some(WidgetInstance { id, label, config })
    }
}

/// A row in dock order: its `rowid`, which names it even when its `id` is
/// not valid text, and its ID if the interface sees it.
struct Entry {
    rowid: i64,
    shown: Option<WidgetId>,
}

/// Every row, in dock order, including the ones the list leaves out: they
/// keep a place, so positions stay contiguous over the whole table.
fn order(conn: &Connection) -> rusqlite::Result<Vec<Entry>> {
    let mut statement = conn.prepare_cached(
        "SELECT rowid, id, kind FROM widget_instances ORDER BY position, created_at, id",
    )?;
    let rows = statement.query_map([], |row| {
        let id = Text::read(row, 1)?;
        let kind = Text::read(row, 2)?;
        Ok(Entry {
            rowid: row.get(0)?,
            shown: shown_kind(&kind).and_then(|_| shown_id(&id)).ok(),
        })
    })?;
    rows.collect()
}

/// Index of the instance `id` in `order`, if the interface can see it.
fn find(order: &[Entry], id: &WidgetId) -> Result<usize, WidgetError> {
    order
        .iter()
        .position(|entry| entry.shown.as_ref() == Some(id))
        .ok_or(WidgetError::NotFound)
}

/// Writes positions 0..n-1 in the order given, touching only the rows whose
/// position changes.
fn renumber(tx: &Transaction<'_>, order: &[Entry]) -> rusqlite::Result<()> {
    let mut statement = tx.prepare_cached(
        "UPDATE widget_instances SET position = ?1 WHERE rowid = ?2 AND position <> ?1",
    )?;
    for (position, entry) in order.iter().enumerate() {
        statement.execute(params![position as i64, entry.rowid])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::domain::widgets::ClockConfig;
    use crate::persistence::database::Storage;

    fn open(dir: &TempDir) -> Storage {
        Storage::open(dir.path())
    }

    fn ids(conn: &Connection) -> Vec<WidgetId> {
        list(conn).unwrap().into_iter().map(|w| w.id).collect()
    }

    /// Every row's position, in position order.
    fn positions(conn: &Connection) -> Vec<i64> {
        conn.prepare("SELECT position FROM widget_instances ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn assert_contiguous(conn: &Connection) {
        let positions = positions(conn);
        let expected: Vec<i64> = (0..positions.len() as i64).collect();
        assert_eq!(positions, expected);
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    /// `config` and `config_version` of the row `id`, as stored.
    fn stored_config(conn: &Connection, id: &str) -> (String, i64) {
        conn.query_row(
            "SELECT config, config_version FROM widget_instances WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    fn insert_raw(conn: &Connection, id: &str, kind: &str, position: i64) {
        conn.execute(
            "INSERT INTO widget_instances \
             (id, kind, label, position, config, config_version, created_at) \
             VALUES (?1, ?2, NULL, ?3, '{}', 1, 0)",
            params![id, kind, position],
        )
        .unwrap();
    }

    #[test]
    fn add_appends_a_new_instance_with_the_default_config() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);

        let added = add(conn, WidgetKind::Clock).unwrap();

        assert_eq!(added.label, None);
        assert_eq!(added.config, WidgetConfig::default_for(WidgetKind::Clock));
        assert!(!before.contains(&added.id));
        let mut expected = before;
        expected.push(added.id.clone());
        assert_eq!(ids(conn), expected);
        assert_eq!(list(conn).unwrap().last(), Some(&added));
        assert_contiguous(conn);
    }

    #[test]
    fn adding_a_note_creates_its_empty_text() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();

        let note = add(conn, WidgetKind::Note).unwrap();

        let body: String = conn
            .query_row(
                "SELECT body FROM notes WHERE instance_id = ?1",
                [note.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(body, "");
        assert_eq!(count(conn, "notes"), 2);
    }

    #[test]
    fn shortcut_and_files_cannot_be_created() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();

        for kind in [WidgetKind::Shortcut, WidgetKind::Files] {
            assert_eq!(add(conn, kind), Err(WidgetError::NotCreatable), "{kind}");
        }
        assert_eq!(count(conn, "widget_instances"), 3);
    }

    #[test]
    fn the_dock_holds_at_most_twelve_widgets() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();

        for _ in 3..MAX_WIDGETS {
            add(conn, WidgetKind::TimeProgress).unwrap();
        }
        assert_eq!(list(conn).unwrap().len(), MAX_WIDGETS);

        for kind in [WidgetKind::Clock, WidgetKind::Note] {
            assert_eq!(add(conn, kind), Err(WidgetError::LimitReached), "{kind}");
        }
        assert_eq!(count(conn, "widget_instances"), MAX_WIDGETS as i64);
        assert_contiguous(conn);
    }

    #[test]
    fn remove_closes_the_gap_and_keeps_the_other_ids() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);

        remove(conn, &before[1]).unwrap();

        assert_eq!(ids(conn), [before[0].clone(), before[2].clone()]);
        assert_contiguous(conn);
    }

    #[test]
    fn removing_a_note_removes_its_text() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let note = ids(conn)[2].clone();
        conn.execute("UPDATE notes SET body = 'shopping list'", [])
            .unwrap();

        remove(conn, &note).unwrap();

        assert_eq!(count(conn, "notes"), 0);
    }

    #[test]
    fn unknown_ids_are_not_found() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let stranger = WidgetId::new();
        let clock = ClockConfig::default();

        assert_eq!(remove(conn, &stranger), Err(WidgetError::NotFound));
        assert_eq!(
            move_widget(conn, &stranger, MoveDirection::Up),
            Err(WidgetError::NotFound)
        );
        assert_eq!(
            update(conn, &stranger, None, WidgetConfig::Clock(clock)),
            Err(WidgetError::NotFound)
        );
        assert_eq!(count(conn, "widget_instances"), 3);
    }

    #[test]
    fn move_swaps_with_the_neighbor_and_keeps_the_ids() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let [a, b, c]: [WidgetId; 3] = ids(conn).try_into().unwrap();

        move_widget(conn, &c, MoveDirection::Up).unwrap();
        assert_eq!(ids(conn), [a.clone(), c.clone(), b.clone()]);

        move_widget(conn, &a, MoveDirection::Down).unwrap();
        assert_eq!(ids(conn), [c, a, b]);
        assert_contiguous(conn);
    }

    #[test]
    fn moving_past_either_end_changes_nothing() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);

        move_widget(conn, &before[0], MoveDirection::Up).unwrap();
        move_widget(conn, &before[2], MoveDirection::Down).unwrap();

        assert_eq!(ids(conn), before);
        assert_contiguous(conn);
    }

    #[test]
    fn update_replaces_label_and_config_in_place() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);
        let config = WidgetConfig::Clock(ClockConfig {
            show_seconds: true,
            ..ClockConfig::default()
        });

        let updated = update(conn, &before[0], Some("Lisboa".to_owned()), config.clone()).unwrap();

        assert_eq!(updated.id, before[0]);
        assert_eq!(updated.label.as_deref(), Some("Lisboa"));
        assert_eq!(updated.config, config);
        assert_eq!(list(conn).unwrap()[0], updated);
        assert_eq!(ids(conn), before);

        let cleared = update(conn, &before[0], None, config).unwrap();
        assert_eq!(cleared.label, None);
        assert_eq!(list(conn).unwrap()[0].label, None);
    }

    #[test]
    fn update_rejects_a_config_of_another_kind() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let clock = ids(conn)[0].clone();
        let before = list(conn).unwrap();

        assert_eq!(
            update(
                conn,
                &clock,
                Some("x".to_owned()),
                WidgetConfig::default_for(WidgetKind::Note)
            ),
            Err(WidgetError::KindMismatch)
        );
        assert_eq!(list(conn).unwrap(), before);
    }

    #[test]
    fn update_stores_the_current_config_version() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let clock = ids(conn)[0].clone();
        conn.execute(
            "UPDATE widget_instances SET config_version = 99 WHERE id = ?1",
            [clock.as_str()],
        )
        .unwrap();

        update(
            conn,
            &clock,
            None,
            WidgetConfig::default_for(WidgetKind::Clock),
        )
        .unwrap();

        let (_, version) = stored_config(conn, clock.as_str());
        assert_eq!(version, i64::from(WidgetKind::Clock.config_version()));
    }

    #[test]
    fn order_ids_and_changes_survive_closing_and_reopening() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let first = ids(conn)[0].clone();
        add(conn, WidgetKind::Clock).unwrap();
        move_widget(conn, &first, MoveDirection::Down).unwrap();
        update(
            conn,
            &first,
            Some("Sede".to_owned()),
            WidgetConfig::default_for(WidgetKind::Clock),
        )
        .unwrap();
        let before = list(conn).unwrap();
        drop(storage);

        let reopened = open(&dir);

        assert_eq!(reopened.snapshot().unwrap().widgets, before);
    }

    #[test]
    fn unreadable_config_gives_the_default_without_rewriting_the_row() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let [clock, progress, _]: [WidgetId; 3] = ids(conn).try_into().unwrap();
        conn.execute(
            "UPDATE widget_instances SET config = '{\"showSeconds\": tru' WHERE id = ?1",
            [clock.as_str()],
        )
        .unwrap();
        conn.execute(
            "UPDATE widget_instances SET config_version = 7 WHERE id = ?1",
            [progress.as_str()],
        )
        .unwrap();
        let clock_row = stored_config(conn, clock.as_str());
        let progress_row = stored_config(conn, progress.as_str());

        let listed = list(conn).unwrap();

        assert_eq!(listed[0].id, clock);
        assert_eq!(
            listed[0].config,
            WidgetConfig::default_for(WidgetKind::Clock)
        );
        assert_eq!(listed[1].id, progress);
        assert_eq!(
            listed[1].config,
            WidgetConfig::default_for(WidgetKind::TimeProgress)
        );
        assert_eq!(stored_config(conn, clock.as_str()), clock_row);
        assert_eq!(stored_config(conn, progress.as_str()), progress_row);
    }

    #[test]
    fn rows_the_interface_cannot_use_are_left_out_but_kept() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);
        // An unknown kind and a malformed ID, between the first and the
        // second widget: positions 0, [1, 2], 3, 4.
        conn.execute(
            "UPDATE widget_instances SET position = position + 2 WHERE position >= 1",
            [],
        )
        .unwrap();
        let radio = WidgetId::new();
        insert_raw(conn, radio.as_str(), "radio", 1);
        insert_raw(conn, "not-an-id", "clock", 2);

        assert_eq!(ids(conn), before);
        assert_eq!(count(conn, "widget_instances"), 5);
        assert_eq!(remove(conn, &radio), Err(WidgetError::NotFound));

        // Moves skip the hidden rows: one step up is one place in the dock.
        move_widget(conn, &before[1], MoveDirection::Up).unwrap();
        assert_eq!(
            ids(conn),
            [before[1].clone(), before[0].clone(), before[2].clone()]
        );
        remove(conn, &before[0]).unwrap();
        add(conn, WidgetKind::Note).unwrap();

        assert_eq!(count(conn, "widget_instances"), 5);
        let kept: i64 = conn
            .query_row(
                "SELECT count(*) FROM widget_instances WHERE id IN (?1, 'not-an-id')",
                [radio.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, 2);
        assert_contiguous(conn);
    }

    /// Text that is not UTF-8, as a hand edit or another tool can store it.
    const NOT_UTF8: &str = "CAST(x'ff' AS TEXT)";

    #[test]
    fn text_that_is_not_utf8_never_breaks_the_list() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let [clock, progress, note]: [WidgetId; 3] = ids(conn).try_into().unwrap();
        conn.execute(
            &format!("UPDATE widget_instances SET label = {NOT_UTF8} WHERE id = ?1"),
            [clock.as_str()],
        )
        .unwrap();
        conn.execute(
            &format!("UPDATE widget_instances SET config = {NOT_UTF8} WHERE id = ?1"),
            [progress.as_str()],
        )
        .unwrap();
        // A bad ID and a bad kind between the first and the second widget.
        conn.execute(
            "UPDATE widget_instances SET position = position + 2 WHERE position >= 1",
            [],
        )
        .unwrap();
        conn.execute(
            &format!(
                "INSERT INTO widget_instances \
                 (id, kind, label, position, config, config_version, created_at) \
                 VALUES ({NOT_UTF8}, 'clock', NULL, 1, '{{}}', 1, 0), \
                 (?1, {NOT_UTF8}, NULL, 2, '{{}}', 1, 0)"
            ),
            [WidgetId::new().as_str()],
        )
        .unwrap();

        let listed = list(conn).unwrap();

        assert_eq!(ids(conn), [clock.clone(), progress.clone(), note.clone()]);
        assert_eq!(listed[0].label, None);
        assert_eq!(
            listed[1].config,
            WidgetConfig::default_for(WidgetKind::TimeProgress)
        );
        // Moving, removing and adding still work, around the hidden rows.
        move_widget(conn, &note, MoveDirection::Up).unwrap();
        assert_eq!(ids(conn), [clock.clone(), note, progress]);
        remove(conn, &clock).unwrap();
        add(conn, WidgetKind::Clock).unwrap();
        assert_eq!(list(conn).unwrap().len(), 3);
        assert_eq!(count(conn, "widget_instances"), 5);
        assert_contiguous(conn);
        let kept: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM widget_instances \
                     WHERE id = {NOT_UTF8} OR kind = {NOT_UTF8}"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, 2);
    }

    #[test]
    fn stored_label_that_breaks_the_rules_reads_as_none_and_stays() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        add(conn, WidgetKind::Clock).unwrap();
        let ids = ids(conn);
        let long = "x".repeat(10 * 1024);
        let labels = ["", "\n", long.as_str()];
        for (id, label) in ids.iter().zip(labels) {
            conn.execute(
                "UPDATE widget_instances SET label = ?1 WHERE id = ?2",
                params![label, id.as_str()],
            )
            .unwrap();
        }

        let listed = list(conn).unwrap();

        for (instance, label) in listed.iter().zip(labels) {
            assert_eq!(instance.label, None, "{:?}", &label[..label.len().min(8)]);
            let stored: String = conn
                .query_row(
                    "SELECT label FROM widget_instances WHERE id = ?1",
                    [instance.id.as_str()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(stored, label, "not rewritten");
        }
        // What the interface got can be saved back.
        let clock = &listed[0];
        assert_eq!(
            update(conn, &clock.id, clock.label.clone(), clock.config.clone()),
            Ok(clock.clone())
        );
    }

    #[test]
    fn kinds_not_creatable_yet_are_hidden_like_unknown_ones() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        let before = ids(conn);
        conn.execute(
            "UPDATE widget_instances SET position = position + 2 WHERE position >= 1",
            [],
        )
        .unwrap();
        let shortcut = WidgetId::new();
        let files = WidgetId::new();
        insert_raw(conn, shortcut.as_str(), "shortcut", 1);
        insert_raw(conn, files.as_str(), "files", 2);

        assert_eq!(ids(conn), before);
        for (id, kind) in [
            (&shortcut, WidgetKind::Shortcut),
            (&files, WidgetKind::Files),
        ] {
            assert_eq!(remove(conn, id), Err(WidgetError::NotFound), "{kind}");
            assert_eq!(
                move_widget(conn, id, MoveDirection::Up),
                Err(WidgetError::NotFound),
                "{kind}"
            );
            assert_eq!(
                update(conn, id, None, WidgetConfig::default_for(kind)),
                Err(WidgetError::NotFound),
                "{kind}"
            );
        }
        move_widget(conn, &before[1], MoveDirection::Up).unwrap();
        assert_eq!(
            ids(conn),
            [before[1].clone(), before[0].clone(), before[2].clone()]
        );
        for _ in 3..MAX_WIDGETS {
            add(conn, WidgetKind::Clock).unwrap();
        }
        assert_eq!(add(conn, WidgetKind::Clock), Err(WidgetError::LimitReached));
        assert_eq!(count(conn, "widget_instances"), MAX_WIDGETS as i64 + 2);
        assert_contiguous(conn);
    }

    #[test]
    fn hidden_rows_do_not_count_towards_the_limit() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        insert_raw(conn, WidgetId::new().as_str(), "radio", 3);

        for _ in 3..MAX_WIDGETS {
            add(conn, WidgetKind::Clock).unwrap();
        }

        assert_eq!(list(conn).unwrap().len(), MAX_WIDGETS);
        assert_eq!(add(conn, WidgetKind::Clock), Err(WidgetError::LimitReached));
    }
}
