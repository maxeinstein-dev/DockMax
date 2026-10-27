//! The workspaces in the `workspaces` and `workspace_items` tables (spec
//! 0013 C1), and the one each monitor shows, in `meta`.
//!
//! Positions are 0-based and contiguous after every write, which runs in one
//! transaction. An item key is kept only if it is one a workspace can hold
//! (`is_item_key`); a key whose program or widget is gone is skipped by the
//! dock, not an error.

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::database::{begin, now_ms, read_failed, write_failed};
use crate::domain::widgets::WidgetId;
use crate::domain::workspaces::{
    FIRST_NAME, MAX_ITEMS, MAX_WORKSPACES, PALETTE, Workspace, WorkspaceError, initial_order,
    is_item_key, is_palette_color, is_valid_name, next_color, next_name,
};

/// Prefix in `meta` of the workspace a monitor shows: `workspace:` and the
/// monitor's device name (`\\.\DISPLAY1`).
const ACTIVE_PREFIX: &str = "workspace:";

/// What follows `ACTIVE_PREFIX` for the workspace in use (no monitor has
/// this name).
const IN_USE_KEY: &str = "in-use";

/// Every workspace, in order, with its items.
pub fn list(conn: &Connection) -> Result<Vec<Workspace>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT id, name, color FROM workspaces ORDER BY position")
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                items: Vec::new(),
            })
        })
        .map_err(read_failed)?;
    let mut workspaces = Vec::new();
    for row in rows {
        let mut workspace = row.map_err(read_failed)?;
        workspace.items = items(conn, &workspace.id)?;
        workspaces.push(workspace);
    }
    Ok(workspaces)
}

/// The keys of the workspace `id`, in dock order.
fn items(conn: &Connection, id: &str) -> Result<Vec<String>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT item_key FROM workspace_items WHERE workspace_id = ?1 ORDER BY position")
        .map_err(read_failed)?;
    let keys = statement
        .query_map(params![id], |row| row.get::<_, String>(0))
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .filter(|key| is_item_key(key))
        .collect();
    Ok(keys)
}

/// Makes the first workspace when there is none: "Principal", with the
/// programs and the widgets there are, in the order the dock showed them
/// (`dock_order`, the order of before the workspaces).
pub fn ensure_first(conn: &mut Connection, dock_order: &[String]) -> Result<(), WorkspaceError> {
    let any: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM workspaces)", [], |row| {
            row.get(0)
        })
        .map_err(read_failed)?;
    if any {
        return Ok(());
    }
    let tx = begin(conn)?;
    let keys = |sql: &str, kind: &str| -> Result<Vec<String>, WorkspaceError> {
        let mut statement = tx.prepare(sql).map_err(read_failed)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .map(|id| format!("{kind}:{id}"))
            .collect();
        Ok(ids)
    };
    let apps = keys("SELECT id FROM pinned_apps ORDER BY position", "app")?;
    let widgets = keys(
        "SELECT id FROM widget_instances ORDER BY position",
        "widget",
    )?;
    let id = WidgetId::new();
    tx.execute(
        "INSERT INTO workspaces (id, name, color, position, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        params![id.as_str(), FIRST_NAME, PALETTE[0], now_ms()],
    )
    .map_err(write_failed)?;
    write_items(
        &tx,
        id.as_str(),
        &initial_order(dock_order, &apps, &widgets),
    )?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Replaces the items of the workspace `id` with `keys`, in that order.
fn write_items(tx: &Transaction<'_>, id: &str, keys: &[String]) -> Result<(), WorkspaceError> {
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1",
        params![id],
    )
    .map_err(write_failed)?;
    let mut seen = std::collections::HashSet::new();
    let kept = keys
        .iter()
        .filter(|key| is_item_key(key) && seen.insert(key.as_str()))
        .take(MAX_ITEMS);
    for (position, key) in kept.enumerate() {
        tx.execute(
            "INSERT INTO workspace_items (workspace_id, item_key, position) VALUES (?1, ?2, ?3)",
            params![id, key, position as i64],
        )
        .map_err(write_failed)?;
    }
    Ok(())
}

/// The ids of the workspaces, in order.
fn ids(conn: &Connection) -> Result<Vec<String>, WorkspaceError> {
    let mut statement = conn
        .prepare("SELECT id FROM workspaces ORDER BY position")
        .map_err(read_failed)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(read_failed)?
        .filter_map(Result::ok)
        .collect();
    Ok(ids)
}

fn renumber(tx: &Transaction<'_>, ids: &[String]) -> Result<(), WorkspaceError> {
    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE workspaces SET position = ?1 WHERE id = ?2",
            params![position as i64, id],
        )
        .map_err(write_failed)?;
    }
    Ok(())
}

fn exists(conn: &Connection, id: &str) -> Result<bool, WorkspaceError> {
    let found = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
            params![id],
            |row| row.get(0),
        )
        .map_err(read_failed)?;
    Ok(found)
}

/// Makes a workspace at the end, empty: named `name`, or the next free
/// "Espaço N"; with the next free color of the palette.
pub fn create(conn: &mut Connection, name: Option<&str>) -> Result<Workspace, WorkspaceError> {
    if name.is_some_and(|name| !is_valid_name(name)) {
        return Err(WorkspaceError::InvalidName);
    }
    let tx = begin(conn)?;
    let existing = list(&tx)?;
    if existing.len() >= MAX_WORKSPACES {
        return Err(WorkspaceError::LimitReached);
    }
    let names: Vec<&str> = existing.iter().map(|w| w.name.as_str()).collect();
    let colors: Vec<&str> = existing.iter().map(|w| w.color.as_str()).collect();
    let workspace = Workspace {
        id: WidgetId::new().as_str().to_owned(),
        name: name.map_or_else(|| next_name(&names), str::to_owned),
        color: next_color(&colors).to_owned(),
        items: Vec::new(),
    };
    tx.execute(
        "INSERT INTO workspaces (id, name, color, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            workspace.id,
            workspace.name,
            workspace.color,
            existing.len() as i64,
            now_ms()
        ],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)?;
    Ok(workspace)
}

pub fn rename(conn: &mut Connection, id: &str, name: &str) -> Result<(), WorkspaceError> {
    if !is_valid_name(name) {
        return Err(WorkspaceError::InvalidName);
    }
    let changed = conn
        .execute(
            "UPDATE workspaces SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
        .map_err(write_failed)?;
    if changed == 0 {
        return Err(WorkspaceError::NotFound);
    }
    Ok(())
}

pub fn recolor(conn: &mut Connection, id: &str, color: &str) -> Result<(), WorkspaceError> {
    if !is_palette_color(color) {
        return Err(WorkspaceError::InvalidColor);
    }
    let color = PALETTE
        .iter()
        .find(|known| known.eq_ignore_ascii_case(color))
        .copied()
        .unwrap_or(PALETTE[0]);
    let changed = conn
        .execute(
            "UPDATE workspaces SET color = ?1 WHERE id = ?2",
            params![color, id],
        )
        .map_err(write_failed)?;
    if changed == 0 {
        return Err(WorkspaceError::NotFound);
    }
    Ok(())
}

/// Moves the workspace one place up or down; at either end nothing changes.
pub fn shift(conn: &mut Connection, id: &str, up: bool) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    let mut order = ids(&tx)?;
    let index = order
        .iter()
        .position(|known| known == id)
        .ok_or(WorkspaceError::NotFound)?;
    let other = if up {
        index.checked_sub(1)
    } else {
        (index + 1 < order.len()).then_some(index + 1)
    };
    let Some(other) = other else {
        return Ok(());
    };
    order.swap(index, other);
    renumber(&tx, &order)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Deletes the workspace `id` (never the last one) and returns the keys of
/// its items that no other workspace has: those programs and widgets are to
/// be removed for good. The monitors that showed it go to the first one.
pub fn delete(conn: &mut Connection, id: &str) -> Result<Vec<String>, WorkspaceError> {
    let tx = begin(conn)?;
    let mut order = ids(&tx)?;
    let index = order
        .iter()
        .position(|known| known == id)
        .ok_or(WorkspaceError::NotFound)?;
    if order.len() == 1 {
        return Err(WorkspaceError::LastWorkspace);
    }
    let gone = items(&tx, id)?;
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1",
        params![id],
    )
    .map_err(write_failed)?;
    tx.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
        .map_err(write_failed)?;
    order.remove(index);
    renumber(&tx, &order)?;
    tx.execute(
        "DELETE FROM meta WHERE key LIKE ?1 AND value = ?2",
        params![format!("{ACTIVE_PREFIX}%"), id],
    )
    .map_err(write_failed)?;
    let mut orphans = Vec::new();
    for key in gone {
        if !held(&tx, &key)? {
            orphans.push(key);
        }
    }
    tx.commit().map_err(write_failed)?;
    Ok(orphans)
}

/// Whether any workspace has the item `key`.
pub fn held(conn: &Connection, key: &str) -> Result<bool, WorkspaceError> {
    let found = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspace_items WHERE item_key = ?1)",
            params![key],
            |row| row.get(0),
        )
        .map_err(read_failed)?;
    Ok(found)
}

/// Puts the item `key` at the end of the workspace `id`, unless it is there.
pub fn add_item(conn: &mut Connection, id: &str, key: &str) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    if !exists(&tx, id)? {
        return Err(WorkspaceError::NotFound);
    }
    let mut keys = items(&tx, id)?;
    if keys.iter().any(|known| known == key) {
        return Ok(());
    }
    if keys.len() >= MAX_ITEMS {
        return Err(WorkspaceError::LimitReached);
    }
    keys.push(key.to_owned());
    write_items(&tx, id, &keys)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Takes the item `key` out of the workspace `id`; returns whether another
/// workspace still has it.
pub fn remove_item(conn: &mut Connection, id: &str, key: &str) -> Result<bool, WorkspaceError> {
    let tx = begin(conn)?;
    tx.execute(
        "DELETE FROM workspace_items WHERE workspace_id = ?1 AND item_key = ?2",
        params![id, key],
    )
    .map_err(write_failed)?;
    let keys = items(&tx, id)?;
    write_items(&tx, id, &keys)?;
    let elsewhere = held(&tx, key)?;
    tx.commit().map_err(write_failed)?;
    Ok(elsewhere)
}

/// Takes the item `key` out of every workspace (its program or widget was
/// removed for good).
pub fn forget_item(conn: &mut Connection, key: &str) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    let holders: Vec<String> = {
        let mut statement = tx
            .prepare("SELECT workspace_id FROM workspace_items WHERE item_key = ?1")
            .map_err(read_failed)?;
        statement
            .query_map(params![key], |row| row.get::<_, String>(0))
            .map_err(read_failed)?
            .filter_map(Result::ok)
            .collect()
    };
    for id in holders {
        let keys: Vec<String> = items(&tx, &id)?
            .into_iter()
            .filter(|known| known != key)
            .collect();
        write_items(&tx, &id, &keys)?;
    }
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// Puts the items of the workspace `id` in the order of `keys` (the dock's
/// drag): its items `keys` does not name keep their order, at the end, and
/// keys of items it does not have are left out.
pub fn reorder(conn: &mut Connection, id: &str, keys: &[String]) -> Result<(), WorkspaceError> {
    let tx = begin(conn)?;
    if !exists(&tx, id)? {
        return Err(WorkspaceError::NotFound);
    }
    let current = items(&tx, id)?;
    let mut ordered: Vec<String> = keys
        .iter()
        .filter(|key| current.contains(key))
        .cloned()
        .collect();
    ordered.extend(current.into_iter().filter(|key| !keys.contains(key)));
    write_items(&tx, id, &ordered)?;
    tx.commit().map_err(write_failed)?;
    Ok(())
}

/// The workspace the monitor `monitor` shows: the one kept for it, or the
/// first one.
pub fn active(conn: &Connection, monitor: &str) -> Result<Option<String>, WorkspaceError> {
    let kept: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![format!("{ACTIVE_PREFIX}{monitor}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_failed)?;
    if let Some(id) = kept
        && exists(conn, &id)?
    {
        return Ok(Some(id));
    }
    Ok(ids(conn)?.into_iter().next())
}

/// The workspace in use: the one of the dock used last, whose items
/// Configurações lists and where a program pinned or a widget added goes;
/// the first one when none was kept or it is gone.
pub fn in_use(conn: &Connection) -> Result<Option<String>, WorkspaceError> {
    active(conn, IN_USE_KEY)
}

/// Keeps `id` as the workspace in use; returns whether it changed.
pub fn set_in_use(conn: &mut Connection, id: &str) -> Result<bool, WorkspaceError> {
    if in_use(conn)?.as_deref() == Some(id) {
        return Ok(false);
    }
    set_active(conn, IN_USE_KEY, id)?;
    Ok(true)
}

/// Keeps `id` as the workspace the monitor `monitor` shows.
pub fn set_active(conn: &mut Connection, monitor: &str, id: &str) -> Result<(), WorkspaceError> {
    if !exists(conn, id)? {
        return Err(WorkspaceError::NotFound);
    }
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![format!("{ACTIVE_PREFIX}{monitor}"), id],
    )
    .map_err(write_failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::MIGRATIONS;

    fn database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        for migration in MIGRATIONS {
            conn.execute_batch(migration).unwrap();
        }
        let tx = conn.transaction().unwrap();
        tx.execute_batch(
            "INSERT INTO pinned_apps VALUES ('a1', 'App1', 'App 1', 0, 0);
             INSERT INTO pinned_apps VALUES ('a2', 'App2', 'App 2', 1, 0);
             INSERT INTO widget_instances VALUES ('w1', 'clock', NULL, 0, '{}', 1, 0);",
        )
        .unwrap();
        tx.commit().unwrap();
        conn
    }

    fn keys(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    #[test]
    fn the_first_workspace_holds_what_the_dock_showed() {
        let mut conn = database();
        ensure_first(&mut conn, &keys(&["widget:w1", "app:a1"])).unwrap();
        // Once only.
        ensure_first(&mut conn, &[]).unwrap();

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Principal");
        assert_eq!(all[0].color, PALETTE[0]);
        assert_eq!(all[0].items, keys(&["widget:w1", "app:a1", "app:a2"]));
    }

    #[test]
    fn workspaces_are_made_renamed_recolored_and_moved() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let second = create(&mut conn, None).unwrap();
        assert_eq!(second.name, "Espaço 2");
        assert_eq!(second.color, PALETTE[1]);
        let third = create(&mut conn, Some("Foco")).unwrap();

        rename(&mut conn, &second.id, "Trabalho").unwrap();
        recolor(&mut conn, &third.id, "#30d158").unwrap();
        shift(&mut conn, &third.id, true).unwrap();

        let all = list(&conn).unwrap();
        let names: Vec<&str> = all.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, ["Principal", "Foco", "Trabalho"]);
        assert_eq!(all[1].color, "#30D158");
        assert_eq!(
            rename(&mut conn, "nope", "X"),
            Err(WorkspaceError::NotFound)
        );
        assert_eq!(
            rename(&mut conn, &third.id, " X"),
            Err(WorkspaceError::InvalidName)
        );
        assert_eq!(
            recolor(&mut conn, &third.id, "#123456"),
            Err(WorkspaceError::InvalidColor)
        );
    }

    #[test]
    fn no_more_than_nine() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        for _ in 1..MAX_WORKSPACES {
            create(&mut conn, None).unwrap();
        }
        assert_eq!(create(&mut conn, None), Err(WorkspaceError::LimitReached));
    }

    #[test]
    fn an_item_in_two_workspaces_stays_until_the_last_lets_it_go() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "widget:w1").unwrap();
        add_item(&mut conn, &second, "widget:w1").unwrap();
        assert_eq!(list(&conn).unwrap()[1].items, keys(&["widget:w1"]));

        assert!(remove_item(&mut conn, &first, "widget:w1").unwrap());
        assert!(!remove_item(&mut conn, &second, "widget:w1").unwrap());
        assert!(!held(&conn, "widget:w1").unwrap());
    }

    #[test]
    fn deleting_a_workspace_tells_what_no_other_has() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "app:a1").unwrap();
        set_active(&mut conn, r"\\.\DISPLAY1", &second).unwrap();

        let orphans = delete(&mut conn, &first).unwrap();
        assert_eq!(orphans, keys(&["app:a2", "widget:w1"]));
        assert_eq!(
            delete(&mut conn, &second),
            Err(WorkspaceError::LastWorkspace)
        );
        // The monitor that showed the second one still does; one that showed
        // the deleted one goes to the first.
        assert_eq!(
            active(&conn, r"\\.\DISPLAY1").unwrap(),
            Some(second.clone())
        );
        assert_eq!(active(&conn, r"\\.\DISPLAY2").unwrap(), Some(second));
    }

    #[test]
    fn the_drag_reorders_only_what_the_workspace_has() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        reorder(&mut conn, &first, &keys(&["widget:w1", "app:zz", "app:a2"])).unwrap();
        assert_eq!(
            list(&conn).unwrap()[0].items,
            keys(&["widget:w1", "app:a2", "app:a1"])
        );
    }

    #[test]
    fn a_removed_item_leaves_every_workspace() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let second = create(&mut conn, None).unwrap().id;
        add_item(&mut conn, &second, "app:a1").unwrap();
        forget_item(&mut conn, "app:a1").unwrap();
        assert!(!held(&conn, "app:a1").unwrap());
    }

    #[test]
    fn each_monitor_keeps_its_workspace() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        assert_eq!(active(&conn, "A").unwrap(), Some(first.clone()));
        set_active(&mut conn, "A", &second).unwrap();
        assert_eq!(active(&conn, "A").unwrap(), Some(second));
        assert_eq!(active(&conn, "B").unwrap(), Some(first));
        assert_eq!(
            set_active(&mut conn, "A", "nope"),
            Err(WorkspaceError::NotFound)
        );
    }

    #[test]
    fn the_workspace_in_use_starts_as_the_first() {
        let mut conn = database();
        ensure_first(&mut conn, &[]).unwrap();
        let first = list(&conn).unwrap()[0].id.clone();
        let second = create(&mut conn, None).unwrap().id;
        assert_eq!(in_use(&conn).unwrap(), Some(first));
        assert!(set_in_use(&mut conn, &second).unwrap());
        assert!(!set_in_use(&mut conn, &second).unwrap());
        assert_eq!(in_use(&conn).unwrap(), Some(second));
    }
}
