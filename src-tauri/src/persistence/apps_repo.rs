//! The pinned programs in the `pinned_apps` table (spec 0006 §3.2).
//!
//! Positions are 0-based and contiguous after every write, which runs in one
//! transaction. Reading never fails on a bad row (edited by hand, by another
//! tool or by a newer DockMax): a row with an id that is not one Rust makes,
//! or an id or a name that is not valid text, is left out of the list and
//! stays as it is; the log gets only the kind of problem.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::database::{begin, now_ms, read_failed, write_failed};
use crate::domain::apps::{AppError, MAX_APPS, PinnedApp, is_valid_app_id, is_valid_app_name};
#[cfg(test)]
use crate::domain::widgets::MoveDirection;
use crate::domain::widgets::WidgetId;

/// Every pinned program the interface can see, in dock order.
pub fn list(conn: &Connection) -> Result<Vec<PinnedApp>, AppError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, app_id, name FROM pinned_apps ORDER BY position, created_at, id",
        )
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                text(row.get_ref(0)?),
                text(row.get_ref(1)?),
                text(row.get_ref(2)?),
            ))
        })
        .map_err(read_failed)?;
    let mut apps = Vec::new();
    for row in rows {
        let (id, app_id, name) = row.map_err(read_failed)?;
        match shown(id, app_id, name) {
            Some(app) => apps.push(app),
            None => log::warn!("apps: ignored a pinned app with an invalid id or name"),
        }
    }
    Ok(apps)
}

/// The pinned program `id`, if it exists and the interface sees it.
pub fn get(conn: &Connection, id: &WidgetId) -> Result<Option<PinnedApp>, AppError> {
    let row = conn
        .query_row(
            "SELECT id, app_id, name FROM pinned_apps WHERE id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    text(row.get_ref(0)?),
                    text(row.get_ref(1)?),
                    text(row.get_ref(2)?),
                ))
            },
        )
        .optional()
        .map_err(read_failed)?;
    Ok(row.and_then(|(id, app_id, name)| shown(id, app_id, name)))
}

/// Pins the program `app_id`, named `name`, at the end of the dock. The same
/// program twice (without regard to case) is `alreadyPinned`.
pub fn pin(conn: &mut Connection, app_id: &str, name: &str) -> Result<PinnedApp, AppError> {
    if !is_valid_app_id(app_id) || !is_valid_app_name(name) {
        return Err(AppError::InvalidApp);
    }
    let tx = begin(conn)?;
    let ids = order(&tx)?;
    if ids.len() >= MAX_APPS {
        return Err(AppError::LimitReached);
    }
    let taken: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pinned_apps WHERE app_id = ?1)",
            params![app_id],
            |row| row.get(0),
        )
        .map_err(read_failed)?;
    if taken {
        return Err(AppError::AlreadyPinned);
    }
    let app = PinnedApp {
        id: WidgetId::new(),
        app_id: app_id.to_owned(),
        name: name.to_owned(),
    };
    tx.execute(
        "INSERT INTO pinned_apps (id, app_id, name, position, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![app.id.as_str(), app_id, name, ids.len() as i64, now_ms()],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    Ok(app)
}

/// Unpins a program; the ones after it move up.
pub fn unpin(conn: &mut Connection, id: &WidgetId) -> Result<(), AppError> {
    let tx = begin(conn)?;
    let removed = tx
        .execute(
            "DELETE FROM pinned_apps WHERE id = ?1",
            params![id.as_str()],
        )
        .map_err(write_failed)?;
    if removed == 0 {
        return Err(AppError::NotFound);
    }
    let ids = order(&tx)?;
    renumber(&tx, &ids)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Moves a program one place towards the start (`up`) or the end (`down`)
/// of the dock. At either end nothing changes.
#[cfg(test)]
pub fn shift(
    conn: &mut Connection,
    id: &WidgetId,
    direction: MoveDirection,
) -> Result<(), AppError> {
    let tx = begin(conn)?;
    let mut ids = order(&tx)?;
    let index = ids
        .iter()
        .position(|stored| stored == id.as_str())
        .ok_or(AppError::NotFound)?;
    let neighbor = match direction {
        MoveDirection::Up => index.checked_sub(1),
        MoveDirection::Down => (index + 1 < ids.len()).then_some(index + 1),
    };
    let Some(neighbor) = neighbor else {
        return Ok(());
    };
    ids.swap(index, neighbor);
    renumber(&tx, &ids)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Puts a program at `index` among the pinned ones the interface sees, the
/// others closing up around it. An index past the end is the end.
pub fn place(conn: &mut Connection, id: &WidgetId, index: usize) -> Result<(), AppError> {
    let tx = begin(conn)?;
    let mut ids = order(&tx)?;
    let from = ids
        .iter()
        .position(|stored| stored == id.as_str())
        .ok_or(AppError::NotFound)?;
    let moved = ids.remove(from);
    ids.insert(index.min(ids.len()), moved);
    renumber(&tx, &ids)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// The ids of every row in dock order, hidden ones included: they keep
/// their place.
fn order(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement = conn
        .prepare_cached("SELECT id FROM pinned_apps ORDER BY position, created_at, id")
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(read_failed)?;
    Ok(rows.collect::<Result<Vec<_>, _>>().map_err(read_failed)?)
}

/// Writes the positions 0, 1, 2... in the order of `ids`.
fn renumber(tx: &Transaction<'_>, ids: &[String]) -> Result<(), AppError> {
    let mut statement = tx
        .prepare_cached("UPDATE pinned_apps SET position = ?1 WHERE id = ?2")
        .map_err(write_failed)?;
    for (position, id) in ids.iter().enumerate() {
        statement
            .execute(params![position as i64, id])
            .map_err(write_failed)?;
    }
    Ok(())
}

/// A TEXT column as stored: `None` for anything that is not valid UTF-8
/// text.
fn text(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes).ok().map(str::to_owned),
        _ => None,
    }
}

/// A stored row as the interface sees it, if it is one Rust could have made.
fn shown(id: Option<String>, app_id: Option<String>, name: Option<String>) -> Option<PinnedApp> {
    let id = WidgetId::parse(&id?).ok()?;
    let (app_id, name) = (app_id?, name?);
    (is_valid_app_id(&app_id) && is_valid_app_name(&name)).then_some(PinnedApp { id, app_id, name })
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::persistence::database::Storage;

    fn open() -> (TempDir, Storage) {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path());
        (dir, storage)
    }

    fn names(conn: &Connection) -> Vec<String> {
        list(conn)
            .unwrap()
            .into_iter()
            .map(|app| app.name)
            .collect()
    }

    fn pin_n(conn: &mut Connection, names: &[&str]) -> Vec<PinnedApp> {
        names
            .iter()
            .map(|name| pin(conn, &format!("app\\{name}.exe"), name).unwrap())
            .collect()
    }

    #[test]
    fn a_new_database_has_no_pinned_apps() {
        let (_dir, mut storage) = open();

        assert_eq!(list(storage.connection().unwrap()).unwrap(), []);
    }

    #[test]
    fn pinning_adds_at_the_end_and_keeps_the_name() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();

        let a = pin(
            conn,
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            "Calculadora",
        )
        .unwrap();
        pin(conn, r"C:\tools\x.exe", "X").unwrap();

        assert_eq!(names(conn), ["Calculadora", "X"]);
        assert_eq!(get(conn, &a.id).unwrap(), Some(a));
    }

    #[test]
    fn the_same_program_twice_is_refused_without_regard_to_case() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        pin(conn, r"C:\Tools\X.exe", "X").unwrap();

        assert_eq!(
            pin(conn, r"c:\tools\x.EXE", "X"),
            Err(AppError::AlreadyPinned)
        );
        assert_eq!(names(conn), ["X"]);
    }

    #[test]
    fn an_invalid_id_or_name_never_reaches_the_table() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();

        assert_eq!(pin(conn, "", "X"), Err(AppError::InvalidApp));
        assert_eq!(pin(conn, "a\nb", "X"), Err(AppError::InvalidApp));
        assert_eq!(pin(conn, "ok", "  "), Err(AppError::InvalidApp));
        assert_eq!(list(conn).unwrap(), []);
    }

    #[test]
    fn the_limit_is_enforced() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        for n in 0..MAX_APPS {
            pin(conn, &format!("app{n}"), "A").unwrap();
        }

        assert_eq!(pin(conn, "one-more", "A"), Err(AppError::LimitReached));
        assert_eq!(list(conn).unwrap().len(), MAX_APPS);
    }

    #[test]
    fn unpinning_closes_the_gap() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        let apps = pin_n(conn, &["A", "B", "C"]);

        unpin(conn, &apps[1].id).unwrap();

        assert_eq!(names(conn), ["A", "C"]);
        let positions: Vec<i64> = conn
            .prepare("SELECT position FROM pinned_apps ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(positions, [0, 1]);
    }

    #[test]
    fn unpinning_what_is_not_there_is_not_found() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();

        assert_eq!(unpin(conn, &WidgetId::new()), Err(AppError::NotFound));
    }

    #[test]
    fn moving_swaps_with_the_neighbor_and_stops_at_the_ends() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        let apps = pin_n(conn, &["A", "B", "C"]);

        shift(conn, &apps[2].id, MoveDirection::Up).unwrap();
        assert_eq!(names(conn), ["A", "C", "B"]);
        shift(conn, &apps[0].id, MoveDirection::Up).unwrap();
        assert_eq!(names(conn), ["A", "C", "B"], "already first");
        shift(conn, &apps[1].id, MoveDirection::Down).unwrap();
        assert_eq!(names(conn), ["A", "C", "B"], "already last");
        shift(conn, &apps[0].id, MoveDirection::Down).unwrap();
        assert_eq!(names(conn), ["C", "A", "B"]);
        assert_eq!(
            shift(conn, &WidgetId::new(), MoveDirection::Up),
            Err(AppError::NotFound)
        );
    }

    #[test]
    fn placing_moves_a_program_to_the_index_and_the_others_close_up() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        let apps = pin_n(conn, &["A", "B", "C", "D"]);

        place(conn, &apps[3].id, 0).unwrap();
        assert_eq!(names(conn), ["D", "A", "B", "C"]);
        place(conn, &apps[0].id, 2).unwrap();
        assert_eq!(names(conn), ["D", "B", "A", "C"]);
        place(conn, &apps[1].id, 3).unwrap();
        assert_eq!(names(conn), ["D", "A", "C", "B"]);
    }

    #[test]
    fn placing_in_the_same_place_or_past_the_end_is_harmless() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        let apps = pin_n(conn, &["A", "B", "C"]);

        place(conn, &apps[1].id, 1).unwrap();
        assert_eq!(names(conn), ["A", "B", "C"]);
        place(conn, &apps[0].id, 99).unwrap();
        assert_eq!(names(conn), ["B", "C", "A"]);
        assert_eq!(place(conn, &WidgetId::new(), 0), Err(AppError::NotFound));
        let positions: Vec<i64> = conn
            .prepare("SELECT position FROM pinned_apps ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(positions, [0, 1, 2]);
    }

    #[test]
    fn a_row_edited_by_hand_is_left_out_but_keeps_its_place() {
        let (_dir, mut storage) = open();
        let conn = storage.connection().unwrap();
        let apps = pin_n(conn, &["A", "B"]);
        conn.execute(
            "INSERT INTO pinned_apps (id, app_id, name, position, created_at) \
             VALUES ('not-an-id', 'x', 'Hidden', 1, 0)",
            [],
        )
        .unwrap();

        assert_eq!(names(conn), ["A", "B"]);
        // The hidden row is not the interface's: it can't be moved or removed.
        assert_eq!(unpin(conn, &WidgetId::new()), Err(AppError::NotFound));
        unpin(conn, &apps[0].id).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM pinned_apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 2, "A is gone, the hidden row stays");
    }

    #[test]
    fn the_pins_survive_a_restart() {
        let dir = tempdir().unwrap();
        {
            let mut storage = Storage::open(dir.path());
            pin_n(storage.connection().unwrap(), &["A", "B"]);
        }

        let mut again = Storage::open(dir.path());

        assert_eq!(names(again.connection().unwrap()), ["A", "B"]);
    }
}
