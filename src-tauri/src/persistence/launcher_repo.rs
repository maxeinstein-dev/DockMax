//! What the launcher opened, in the `launcher_usage` table (spec 0012 §5):
//! each entry's weight, which halves every 14 days, and when it was last
//! opened. Only the launcher writes here.
//!
//! Reading never fails on a bad row (edited by hand, by another tool or by a
//! newer Dockyard): a row with a key that is not one the launcher makes, or a
//! weight that is not a number, is left out and stays as it is.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::database::{begin, read_failed, write_failed};
use crate::domain::apps::is_valid_app_id;
use crate::domain::launcher::{decay, opened};
use crate::domain::launcher_files::hit_of_key;
use crate::domain::launcher_pages::page;
use crate::domain::widgets::WidgetError;

/// Most entries kept: the ones opened longest ago go first.
pub const MAX_ENTRIES: usize = 500;

/// Whether `key` is one the launcher keeps the use of: `app:` and a program
/// of the Apps folder, `setting:` and a page of the fixed list, or `file:`
/// and a full path on a drive (L4; a folder's ends in `\`).
pub fn is_valid_key(key: &str) -> bool {
    key.strip_prefix("app:").is_some_and(is_valid_app_id)
        || key
            .strip_prefix("setting:")
            .is_some_and(|id| page(id).is_some())
        || hit_of_key(key).is_some()
}

/// Each entry's weight at `now_ms`, by key.
pub fn usage(conn: &Connection, now_ms: i64) -> Result<HashMap<String, f64>, WidgetError> {
    let mut statement = conn
        .prepare_cached("SELECT key, score, last_used FROM launcher_usage")
        .map_err(read_failed)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).ok(),
                row.get::<_, f64>(1).ok(),
                row.get::<_, i64>(2).ok(),
            ))
        })
        .map_err(read_failed)?;
    let mut found = HashMap::new();
    for row in rows {
        match row.map_err(read_failed)? {
            (Some(key), Some(score), Some(last))
                if is_valid_key(&key) && score.is_finite() && score >= 0.0 =>
            {
                found.insert(key, decay(score, last, now_ms));
            }
            _ => log::warn!("launcher: ignored a use with an invalid key or weight"),
        }
    }
    Ok(found)
}

/// One more opening of `key` at `now_ms`; past `MAX_ENTRIES`, the entries
/// opened longest ago are forgotten.
pub fn record(conn: &mut Connection, key: &str, now_ms: i64) -> Result<(), WidgetError> {
    if !is_valid_key(key) {
        return Err(WidgetError::InvalidTarget);
    }
    let tx = begin(conn)?;
    let before: Option<(f64, i64)> = tx
        .query_row(
            "SELECT score, last_used FROM launcher_usage WHERE key = ?1",
            params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let score = match before {
        Some((score, last)) if score.is_finite() && score >= 0.0 => opened(score, last, now_ms),
        _ => 1.0,
    };
    tx.execute(
        "INSERT INTO launcher_usage (key, score, last_used) VALUES (?1, ?2, ?3) \
         ON CONFLICT (key) DO UPDATE SET score = excluded.score, last_used = excluded.last_used",
        params![key, score, now_ms],
    )
    .map_err(write_failed)?;
    tx.execute(
        "DELETE FROM launcher_usage WHERE key NOT IN \
         (SELECT key FROM launcher_usage ORDER BY last_used DESC, key LIMIT ?1)",
        params![MAX_ENTRIES as i64],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)
}

/// Forgets everything the launcher opened ("Limpar o histórico do
/// lançador").
pub fn clear(conn: &Connection) -> Result<(), WidgetError> {
    conn.execute("DELETE FROM launcher_usage", [])
        .map_err(write_failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::domain::launcher::HALF_LIFE_MS;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0003_launcher.sql"))
            .unwrap();
        conn
    }

    #[test]
    fn an_opening_weighs_one_and_halves_every_fourteen_days() {
        let mut conn = database();
        record(&mut conn, "app:Paint", 0).unwrap();
        assert_eq!(usage(&conn, 0).unwrap()["app:Paint"], 1.0);
        assert!((usage(&conn, HALF_LIFE_MS).unwrap()["app:Paint"] - 0.5).abs() < 1e-9);
        // Opened again fourteen days later: half of the first, plus one.
        record(&mut conn, "app:Paint", HALF_LIFE_MS).unwrap();
        assert!((usage(&conn, HALF_LIFE_MS).unwrap()["app:Paint"] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn a_key_the_launcher_does_not_make_is_refused() {
        let mut conn = database();
        for key in [
            "Paint",
            "app:",
            "app: Paint",
            "widget:1",
            "app:a\nb",
            "setting:",
            "setting:developers",
            "file:",
            "file:a.txt",
            r"file:\\server\a.txt",
        ] {
            assert!(record(&mut conn, key, 0).is_err(), "{key:?}");
        }
        assert!(usage(&conn, 0).unwrap().is_empty());
        record(&mut conn, "setting:wifi", 0).unwrap();
        assert!(usage(&conn, 0).unwrap().contains_key("setting:wifi"));
        record(&mut conn, r"file:C:\Docs\a.pdf", 0).unwrap();
        record(&mut conn, r"file:C:\Docs\", 0).unwrap();
        assert_eq!(usage(&conn, 0).unwrap().len(), 3);
    }

    #[test]
    fn a_bad_row_is_left_out() {
        let conn = database();
        conn.execute(
            "INSERT INTO launcher_usage VALUES ('nothing', 1.0, 0), ('app:Paint', -1.0, 0), \
             ('app:Calculadora', 2.0, 0)",
            [],
        )
        .unwrap();
        let found = usage(&conn, 0).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found["app:Calculadora"], 2.0);
    }

    #[test]
    fn past_the_most_the_oldest_are_forgotten() {
        let mut conn = database();
        for n in 0..=MAX_ENTRIES {
            record(&mut conn, &format!("app:App {n}"), n as i64).unwrap();
        }
        let found = usage(&conn, 0).unwrap();
        assert_eq!(found.len(), MAX_ENTRIES);
        assert!(!found.contains_key("app:App 0"));
        assert!(found.contains_key(&format!("app:App {MAX_ENTRIES}")));
    }

    #[test]
    fn clearing_forgets_everything() {
        let mut conn = database();
        record(&mut conn, "app:Paint", 0).unwrap();
        clear(&conn).unwrap();
        assert!(usage(&conn, 0).unwrap().is_empty());
    }
}
