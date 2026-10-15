//! Widget instances in the `widget_instances` table (spec 0003 §2.1, §2.4).
//!
//! Positions are 0-based and contiguous after every write, which runs in one
//! transaction. Reading never fails on a bad row: a config that can't be
//! read gives the kind's default, and a row of an unknown kind or with a
//! malformed ID is left out of the list. Either way the row stays as it is
//! until the user changes it (§2.4 item 6).

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::database::{begin, now_ms, write_failed};
use crate::domain::widgets::{
    MAX_WIDGETS, MoveDirection, WidgetConfig, WidgetError, WidgetId, WidgetInstance, WidgetKind,
    decode_config, encode_config, validate_label,
};

/// Every instance the interface can see, in dock order.
pub fn list(conn: &Connection) -> Result<Vec<WidgetInstance>, WidgetError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, kind, label, config, config_version FROM widget_instances \
             ORDER BY position, created_at, id",
        )
        .map_err(write_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                label: row.get(2)?,
                config: row.get(3)?,
                config_version: row.get(4)?,
            })
        })
        .map_err(write_failed)?;
    let mut instances = Vec::new();
    for row in rows {
        if let Some(instance) = row.map_err(write_failed)?.into_instance() {
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
    let order = order(&tx).map_err(write_failed)?;
    if order.iter().filter(|entry| entry.visible).count() >= MAX_WIDGETS {
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
    let mut order = order(&tx).map_err(write_failed)?;
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
pub fn move_widget(
    conn: &mut Connection,
    id: &WidgetId,
    direction: MoveDirection,
) -> Result<(), WidgetError> {
    let tx = begin(conn)?;
    let mut order = order(&tx).map_err(write_failed)?;
    let index = find(&order, id)?;
    let neighbor = match direction {
        MoveDirection::Up => order[..index].iter().rposition(|entry| entry.visible),
        MoveDirection::Down => order[index + 1..]
            .iter()
            .position(|entry| entry.visible)
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

/// The kind of the instance `id`, if it exists and its kind is known.
pub(super) fn kind_of(conn: &Connection, id: &WidgetId) -> Result<Option<WidgetKind>, WidgetError> {
    let name: Option<String> = conn
        .query_row(
            "SELECT kind FROM widget_instances WHERE id = ?1",
            params![id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(write_failed)?;
    Ok(name.as_deref().and_then(WidgetKind::from_name))
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

/// A row as stored, before it is checked.
struct StoredRow {
    id: String,
    kind: String,
    label: Option<String>,
    config: String,
    config_version: i64,
}

impl StoredRow {
    /// The instance the interface gets, or `None` for a row it can't use.
    /// Logs only the kind of problem, never the stored values.
    fn into_instance(self) -> Option<WidgetInstance> {
        let Some(kind) = WidgetKind::from_name(&self.kind) else {
            log::warn!("widgets: ignored an instance of an unknown kind");
            return None;
        };
        let Ok(id) = WidgetId::parse(&self.id) else {
            log::warn!("widgets: ignored an instance with an invalid ID");
            return None;
        };
        let config = u32::try_from(self.config_version)
            .map_err(|_| WidgetError::InvalidConfig)
            .and_then(|version| decode_config(kind, version, &self.config))
            .unwrap_or_else(|error| {
                log::warn!("widgets: using the default {kind} config: {error}");
                WidgetConfig::default_for(kind)
            });
        Some(WidgetInstance {
            id,
            label: self.label,
            config,
        })
    }
}

/// A row in dock order, and whether the interface sees it.
struct Entry {
    id: String,
    visible: bool,
}

/// Every row, in dock order, including the ones the list leaves out: they
/// keep a place, so positions stay contiguous over the whole table.
fn order(conn: &Connection) -> rusqlite::Result<Vec<Entry>> {
    let mut statement = conn.prepare_cached(
        "SELECT id, kind FROM widget_instances ORDER BY position, created_at, id",
    )?;
    let rows = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let visible = WidgetKind::from_name(&kind).is_some() && WidgetId::parse(&id).is_ok();
        Ok(Entry { id, visible })
    })?;
    rows.collect()
}

/// Index of the instance `id` in `order`, if the interface can see it.
fn find(order: &[Entry], id: &WidgetId) -> Result<usize, WidgetError> {
    order
        .iter()
        .position(|entry| entry.visible && entry.id == id.as_str())
        .ok_or(WidgetError::NotFound)
}

/// Writes positions 0..n-1 in the order given, touching only the rows whose
/// position changes.
fn renumber(tx: &Transaction<'_>, order: &[Entry]) -> rusqlite::Result<()> {
    let mut statement = tx.prepare_cached(
        "UPDATE widget_instances SET position = ?1 WHERE id = ?2 AND position <> ?1",
    )?;
    for (position, entry) in order.iter().enumerate() {
        statement.execute(params![position as i64, entry.id])?;
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
        let weather = WidgetId::new();
        insert_raw(conn, weather.as_str(), "weather", 1);
        insert_raw(conn, "not-an-id", "clock", 2);

        assert_eq!(ids(conn), before);
        assert_eq!(count(conn, "widget_instances"), 5);
        assert_eq!(remove(conn, &weather), Err(WidgetError::NotFound));

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
                [weather.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, 2);
        assert_contiguous(conn);
    }

    #[test]
    fn hidden_rows_do_not_count_towards_the_limit() {
        let dir = tempdir().unwrap();
        let mut storage = open(&dir);
        let conn = storage.connection().unwrap();
        insert_raw(conn, WidgetId::new().as_str(), "weather", 3);

        for _ in 3..MAX_WIDGETS {
            add(conn, WidgetKind::Clock).unwrap();
        }

        assert_eq!(list(conn).unwrap().len(), MAX_WIDGETS);
        assert_eq!(add(conn, WidgetKind::Clock), Err(WidgetError::LimitReached));
    }
}
