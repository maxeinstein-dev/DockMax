//! The text of note widgets, in the `notes` table (spec 0003 §2.2, Note).
//! A note's text never travels with the instance list: the panel reads it
//! and saves it on its own.

use rusqlite::{Connection, OptionalExtension, params};

use super::database::{begin, now_ms, write_failed};
use super::widgets_repo::kind_of;
use crate::domain::widgets::{MAX_NOTE_BYTES, WidgetError, WidgetId, WidgetKind};

/// The text of the note `id`; empty if it was never written.
pub fn read(conn: &Connection, id: &WidgetId) -> Result<String, WidgetError> {
    ensure_note(conn, id)?;
    let body: Option<String> = conn
        .query_row(
            "SELECT body FROM notes WHERE instance_id = ?1",
            params![id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(write_failed)?;
    Ok(body.unwrap_or_default())
}

/// Replaces the text of the note `id`. Over [`MAX_NOTE_BYTES`] in UTF-8 it
/// is rejected and the text saved before stays.
pub fn save(conn: &mut Connection, id: &WidgetId, body: &str) -> Result<(), WidgetError> {
    if body.len() > MAX_NOTE_BYTES {
        return Err(WidgetError::NoteTooLarge);
    }
    let tx = begin(conn)?;
    ensure_note(&tx, id)?;
    tx.execute(
        "INSERT INTO notes (instance_id, body, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT (instance_id) DO UPDATE \
         SET body = excluded.body, updated_at = excluded.updated_at",
        params![id.as_str(), body, now_ms()],
    )
    .map_err(write_failed)?;
    tx.commit().map_err(write_failed)
}

/// [`WidgetError::NotFound`] unless `id` is an existing note.
fn ensure_note(conn: &Connection, id: &WidgetId) -> Result<(), WidgetError> {
    match kind_of(conn, id)? {
        Some(WidgetKind::Note) => Ok(()),
        _ => Err(WidgetError::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::persistence::database::Storage;
    use crate::persistence::widgets_repo;

    /// The storage and the IDs of the seeded clock and note.
    fn open(dir: &TempDir) -> (Storage, WidgetId, WidgetId) {
        let storage = Storage::open(dir.path());
        let widgets = storage.snapshot().unwrap().widgets;
        assert_eq!(widgets[2].kind(), WidgetKind::Note);
        let clock = widgets[0].id.clone();
        let note = widgets[2].id.clone();
        (storage, clock, note)
    }

    fn updated_at(conn: &Connection, id: &WidgetId) -> i64 {
        conn.query_row(
            "SELECT updated_at FROM notes WHERE instance_id = ?1",
            [id.as_str()],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_new_note_is_empty() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);

        assert_eq!(
            read(storage.connection().unwrap(), &note),
            Ok(String::new())
        );
    }

    #[test]
    fn saved_text_reads_back_with_the_time_it_was_saved() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        let conn = storage.connection().unwrap();
        let before = now_ms();

        save(conn, &note, "Comprar café\nLigar às 15h ✓").unwrap();

        assert_eq!(read(conn, &note).unwrap(), "Comprar café\nLigar às 15h ✓");
        let saved_at = updated_at(conn, &note);
        assert!(saved_at >= before && saved_at <= now_ms(), "{saved_at}");
    }

    #[test]
    fn text_survives_closing_and_reopening() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        save(storage.connection().unwrap(), &note, "keep me").unwrap();
        drop(storage);

        let (mut reopened, _, _) = open(&dir);

        assert_eq!(
            read(reopened.connection().unwrap(), &note).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn only_existing_notes_have_text() {
        let dir = tempdir().unwrap();
        let (mut storage, clock, _) = open(&dir);
        let conn = storage.connection().unwrap();
        let stranger = WidgetId::new();

        for id in [&clock, &stranger] {
            assert_eq!(read(conn, id), Err(WidgetError::NotFound));
            assert_eq!(save(conn, id, "text"), Err(WidgetError::NotFound));
        }
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_removed_note_is_not_found() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        let conn = storage.connection().unwrap();
        save(conn, &note, "gone soon").unwrap();

        widgets_repo::remove(conn, &note).unwrap();

        assert_eq!(read(conn, &note), Err(WidgetError::NotFound));
    }

    #[test]
    fn text_over_64_kib_is_rejected_and_the_saved_text_stays() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        let conn = storage.connection().unwrap();
        save(conn, &note, "before").unwrap();
        let saved_at = updated_at(conn, &note);

        // 2-byte characters: the limit is in UTF-8 bytes, not characters.
        let over = format!("{}a", "é".repeat(MAX_NOTE_BYTES / 2));
        assert_eq!(over.len(), MAX_NOTE_BYTES + 1);
        assert_eq!(save(conn, &note, &over), Err(WidgetError::NoteTooLarge));

        assert_eq!(read(conn, &note).unwrap(), "before");
        assert_eq!(updated_at(conn, &note), saved_at);
    }

    #[test]
    fn text_of_exactly_64_kib_is_accepted() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        let conn = storage.connection().unwrap();
        let full = "é".repeat(MAX_NOTE_BYTES / 2);

        save(conn, &note, &full).unwrap();

        assert_eq!(read(conn, &note).unwrap(), full);
    }

    #[test]
    fn a_note_whose_text_row_is_missing_reads_empty_and_saves() {
        let dir = tempdir().unwrap();
        let (mut storage, _, note) = open(&dir);
        let conn = storage.connection().unwrap();
        conn.execute("DELETE FROM notes", []).unwrap();

        assert_eq!(read(conn, &note), Ok(String::new()));
        save(conn, &note, "again").unwrap();
        assert_eq!(read(conn, &note).unwrap(), "again");
    }
}
