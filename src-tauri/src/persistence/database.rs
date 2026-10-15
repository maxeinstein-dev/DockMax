//! Opens the widget database, `dockmax.db` (spec 0003 §2.4).
//!
//! Opening never fails: whatever happens, the caller gets a [`Storage`] with
//! a [`StorageStatus`] and, unless the status is `unavailable`, a connection
//! ready to use. Every problem is logged with only the kind of failure (ADR
//! 0007): never a path, a label, a config or note text.
//!
//! - A missing file is created, migrated and seeded once with a Clock, a Time
//!   progress and a Note, schema and seed (and the recovery notice, after a
//!   recovery) in one transaction.
//! - An existing file at an older schema version is copied to
//!   `dockmax.db.bak-v<n>` with `VACUUM INTO` and then migrated, all
//!   migrations in one transaction. If the copy fails, nothing is migrated.
//! - A file from a newer DockMax, or one whose migration fails, is left as it
//!   was and the widgets are unavailable in this run.
//! - A file SQLite says is not a database or is corrupt is renamed to
//!   `dockmax.db.corrompido-<UTC date>` (never deleted), along with its
//!   `-wal`, `-shm` and `-journal` files, and a new seeded database takes its
//!   place with a recovery notice in `meta` until the user dismisses it.
//! - Companion files with no database (missing, or an empty file) are set
//!   aside the same way: SQLite would delete them, and the last changes can
//!   be only in the `-wal`.

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::config::DbConfig;
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension, Transaction};
use rusqlite::{TransactionBehavior, params};
use rusqlite_migration::{HookResult, M, Migrations};

use super::widgets_repo;
use crate::domain::widgets::{
    RECOVERY_NOTICE_KEY, RECOVERY_NOTICE_PENDING, StorageStatus, UnavailableReason, WidgetError,
    WidgetKind, WidgetsSnapshot,
};

/// Name of the widget database inside the app data folder.
pub const FILE_NAME: &str = "dockmax.db";

/// How long a statement waits for a lock held by another connection.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// SQL of migration 1, also used by the tests to build migration sets.
const MIGRATION_1: &str = include_str!("../../migrations/0001_widgets.sql");

/// The SQL of the schema migrations, in order; the database's
/// `user_version` is the number of them applied. Never change or reorder one
/// that has shipped: add the next file in `src-tauri/migrations/` instead.
pub const MIGRATIONS: &[&str] = &[MIGRATION_1];

/// `application_id` of a new database that replaces one set aside, until
/// the transaction that makes its schema commits with the recovery notice.
/// It survives a stop before that commit, so the next start still shows the
/// notice. ("DMRC" in ASCII.)
const RECOVERY_MARK: i32 = 0x444D_5243;

/// The kinds a new database starts with, in dock order (spec 0003 §2.1
/// item 4).
const SEED: [WidgetKind; 3] = [
    WidgetKind::Clock,
    WidgetKind::TimeProgress,
    WidgetKind::Note,
];

/// The widget database for this run: the connection, if the database could
/// be used, and its status.
#[derive(Debug)]
pub struct Storage {
    conn: Option<Connection>,
    status: StorageStatus,
}

impl Storage {
    /// Opens (creating, migrating or recovering as needed) `dockmax.db` in
    /// `dir`, the app data folder, creating the folder if it is missing.
    /// Never fails; see the module docs.
    pub fn open(dir: &Path) -> Storage {
        open_with(dir, MIGRATIONS)
    }

    /// A storage that can't be used in this run, for when there is not even
    /// a folder to open the database in.
    pub fn unavailable(reason: UnavailableReason) -> Storage {
        Storage {
            conn: None,
            status: StorageStatus::Unavailable { reason },
        }
    }

    /// Only the tests need it: the app reads the status in the snapshot.
    #[cfg(test)]
    pub fn status(&self) -> StorageStatus {
        self.status
    }

    /// The connection, for the repositories; [`WidgetError::Unavailable`]
    /// when the database can't be used in this run.
    pub fn connection(&mut self) -> Result<&mut Connection, WidgetError> {
        self.conn.as_mut().ok_or(WidgetError::Unavailable)
    }

    /// The status and every widget in dock order. Without a database the
    /// list is empty: the dock still shows Settings and About.
    pub fn snapshot(&self) -> Result<WidgetsSnapshot, WidgetError> {
        let widgets = match &self.conn {
            Some(conn) => widgets_repo::list(conn)?,
            None => Vec::new(),
        };
        Ok(WidgetsSnapshot {
            storage: self.status,
            widgets,
        })
    }

    /// Removes the recovery notice: the status goes from `recovered` to
    /// `ok`, in this run and in the next ones.
    pub fn dismiss_recovery_notice(&mut self) -> Result<(), WidgetError> {
        let conn = self.connection()?;
        conn.execute(
            "DELETE FROM meta WHERE key = ?1",
            params![RECOVERY_NOTICE_KEY],
        )
        .map_err(write_failed)?;
        if self.status == StorageStatus::Recovered {
            self.status = StorageStatus::Ok;
        }
        Ok(())
    }
}

/// [`Storage::open`] with the given migrations: a seam for the tests, which
/// need a database "from a newer version" and a migration that fails.
pub(super) fn open_with(dir: &Path, migrations: &[&'static str]) -> Storage {
    let path = dir.join(FILE_NAME);
    let found = match fs::symlink_metadata(&path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return unavailable(
                UnavailableReason::OpenFailed,
                format_args!("could not check the file: {}", Cause::Io(error.kind())),
            );
        }
    };

    // Companions with no database to go with them (no file, or an empty
    // one) hold changes SQLite would delete on opening: SQLite itself never
    // leaves them so, so they are set aside like a corrupt database.
    let empty = found
        .as_ref()
        .is_none_or(|metadata| metadata.is_file() && metadata.len() == 0);
    if empty {
        match has_companions(&path) {
            Ok(false) => {}
            Ok(true) => return recover(dir, &path, migrations, Cause::OrphanCompanions),
            Err(error) => {
                return unavailable(
                    UnavailableReason::OpenFailed,
                    format_args!("could not check the file: {}", Cause::Io(error.kind())),
                );
            }
        }
    }

    if found.is_none() {
        return create(dir, &path, migrations, false);
    }
    match open_existing(&path, migrations) {
        Ok(storage) => storage,
        Err(cause) => recover(dir, &path, migrations, cause),
    }
}

/// Makes a new database at `path`: migrated, seeded and, after a recovery,
/// with the notice pending.
fn create(dir: &Path, path: &Path, migrations: &[&'static str], recovered: bool) -> Storage {
    if let Err(error) = fs::create_dir_all(dir) {
        return unavailable(
            UnavailableReason::OpenFailed,
            format_args!("could not create the folder: {}", Cause::Io(error.kind())),
        );
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connected = connect(path, flags).and_then(|conn| {
        if recovered {
            conn.pragma_update(None, "application_id", RECOVERY_MARK)?;
        }
        Ok(conn)
    });
    match connected {
        Ok(conn) => prepare(conn, path, 0, migrations),
        Err(error) => unavailable(
            UnavailableReason::OpenFailed,
            format_args!("could not create the file: {}", Cause::sqlite(&error)),
        ),
    }
}

/// Opens a file that already exists. Nothing is written until the file is
/// known to be a sound database of a version this build can handle: `Err`
/// means SQLite found it is not a database or is corrupt, and the caller
/// sets it aside.
fn open_existing(path: &Path, migrations: &[&'static str]) -> Result<Storage, Cause> {
    // SQLite deletes the `-wal` of a file that turns out not to be a
    // database (seen in the tests), so such a file is caught before SQLite
    // opens it, while its companions are still there to set aside with it.
    match has_foreign_header(path) {
        Ok(false) => {}
        Ok(true) => return Err(Cause::NotSqlite),
        Err(error) => {
            return Ok(unavailable(
                UnavailableReason::OpenFailed,
                format_args!("could not read the file: {}", Cause::Io(error.kind())),
            ));
        }
    }
    let had_companions = match has_companions(path) {
        Ok(had) => had,
        Err(error) => {
            return Ok(unavailable(
                UnavailableReason::OpenFailed,
                format_args!("could not check the file: {}", Cause::Io(error.kind())),
            ));
        }
    };

    // No CREATE: if the file vanished meanwhile, that's a failure to open,
    // not a new database.
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let checked = connect(path, flags).and_then(|conn| {
        // A `-wal` found on disk may hold changes not yet in the file. Until
        // the file proves sound, closing must not checkpoint them into it
        // nor delete the `-wal`: they stay with a corrupt or newer file.
        // Without companions there is nothing to keep, and closing removes
        // the ones this connection made.
        if had_companions {
            conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
        }
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        Ok((conn, version))
    });
    let (conn, version) = match checked {
        Ok(checked) => checked,
        Err(error) if is_corrupt(&error) => return Err(Cause::sqlite(&error)),
        Err(error) => {
            return Ok(unavailable(
                UnavailableReason::OpenFailed,
                format_args!("could not open the file: {}", Cause::sqlite(&error)),
            ));
        }
    };

    // Checked before the integrity check: a newer database is never set
    // aside, whatever this build thinks of its content.
    let latest = migrations.len();
    let version = match usize::try_from(version) {
        Ok(version) if version > latest => {
            return Ok(unavailable(
                UnavailableReason::NewerVersion,
                format_args!("written by a newer version"),
            ));
        }
        Ok(version) => version,
        Err(_) => {
            return Ok(unavailable(
                UnavailableReason::OpenFailed,
                format_args!("invalid schema version"),
            ));
        }
    };

    match quick_check(&conn) {
        Ok(true) => {}
        Ok(false) => return Err(Cause::IntegrityCheck),
        Err(error) if is_corrupt(&error) => return Err(Cause::sqlite(&error)),
        Err(error) => {
            return Ok(unavailable(
                UnavailableReason::OpenFailed,
                format_args!("could not check the file: {}", Cause::sqlite(&error)),
            ));
        }
    }

    Ok(prepare(conn, path, version, migrations))
}

/// Sets up the connection, backs up and migrates a database at `version`
/// (0 for a new one, which migration 1 seeds) and reads the recovery notice.
fn prepare(
    mut conn: Connection,
    path: &Path,
    version: usize,
    migrations: &[&'static str],
) -> Storage {
    if let Err(error) = configure(&conn) {
        return unavailable(
            UnavailableReason::OpenFailed,
            format_args!("could not set up the connection: {}", Cause::sqlite(&error)),
        );
    }

    if version < migrations.len() {
        // Version 0 has no schema of ours, so nothing worth a copy.
        if version > 0
            && let Err(cause) = backup(&conn, path, version)
        {
            return unavailable(
                UnavailableReason::MigrationFailed,
                format_args!("backup before migrating failed: {cause}"),
            );
        }
        if let Err(error) = Migrations::new(with_seed(migrations)).to_latest(&mut conn) {
            return unavailable(
                UnavailableReason::MigrationFailed,
                format_args!("migration failed: {}", Cause::migration(&error)),
            );
        }
    }

    // Sound and up to date: closing checkpoints again, as by default.
    let ready = conn
        .set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, false)
        .and_then(|_| recovery_pending(&conn));
    match ready {
        Ok(pending) => Storage {
            conn: Some(conn),
            status: if pending {
                StorageStatus::Recovered
            } else {
                StorageStatus::Ok
            },
        },
        Err(error) => unavailable(
            UnavailableReason::OpenFailed,
            format_args!(
                "could not read the database state: {}",
                Cause::sqlite(&error)
            ),
        ),
    }
}

/// Sets the corrupt database at `path` aside and makes a new one.
fn recover(dir: &Path, path: &Path, migrations: &[&'static str], cause: Cause) -> Storage {
    log::warn!("widgets: database can't be used, setting it aside: {cause}");
    let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S").to_string();
    match set_aside(path, &format!(".corrompido-{stamp}")) {
        Ok(()) => create(dir, path, migrations, true),
        Err(error) => unavailable(
            UnavailableReason::OpenFailed,
            format_args!(
                "could not set the corrupt file aside: {}",
                Cause::Io(error.kind())
            ),
        ),
    }
}

/// Logs why the widgets are unavailable and returns that storage.
fn unavailable(reason: UnavailableReason, message: fmt::Arguments<'_>) -> Storage {
    log::warn!("widgets: database unavailable: {message}");
    Storage::unavailable(reason)
}

fn connect(path: &Path, flags: OpenFlags) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(path, flags)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// WAL and foreign keys. Foreign keys are off by default in every new
/// connection, and without them removing a note would leave its text.
fn configure(conn: &Connection) -> rusqlite::Result<()> {
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        // Still usable, only less resilient to a crash mid-write.
        log::warn!("widgets: database not in WAL mode");
    }
    conn.pragma_update(None, "foreign_keys", true)
}

/// `PRAGMA quick_check`: true when SQLite finds nothing wrong.
fn quick_check(conn: &Connection) -> rusqlite::Result<bool> {
    let first: String = conn.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
    Ok(first == "ok")
}

/// Copies the database to `dockmax.db.bak-v<version>` (or that name with a
/// number after it, if taken: `VACUUM INTO` refuses an existing file).
fn backup(conn: &Connection, path: &Path, version: usize) -> Result<(), Cause> {
    let target =
        free_name(path, &format!(".bak-v{version}"), &[]).map_err(|e| Cause::Io(e.kind()))?;
    let target = target
        .to_str()
        .ok_or(Cause::Io(io::ErrorKind::InvalidInput))?;
    conn.execute("VACUUM INTO ?1", params![target])
        .map_err(|error| Cause::sqlite(&error))?;
    Ok(())
}

/// The companion files SQLite keeps next to a database: `-wal` and `-shm`
/// in WAL mode, `-journal` in rollback mode (a file switching modes, or
/// from another tool). They belong to it: left behind, they would be read
/// into the new database.
const COMPANIONS: [&str; 3] = ["-wal", "-shm", "-journal"];

/// The first bytes of every SQLite database file.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// True when the file has content that doesn't start like a SQLite
/// database. An empty file is a database with nothing in it yet.
fn has_foreign_header(path: &Path) -> io::Result<bool> {
    let mut start = Vec::with_capacity(SQLITE_HEADER.len());
    fs::File::open(path)?
        .take(SQLITE_HEADER.len() as u64)
        .read_to_end(&mut start)?;
    Ok(!start.is_empty() && start != SQLITE_HEADER)
}

fn has_companions(path: &Path) -> io::Result<bool> {
    for companion in COMPANIONS {
        if with_suffix(path, companion).try_exists()? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Renames `path` and its companion files, those that exist, to `path` +
/// `suffix` (+ a number, if taken). All or nothing: if one rename fails, the
/// ones done are undone.
fn set_aside(path: &Path, suffix: &str) -> io::Result<()> {
    let target = free_name(path, suffix, &COMPANIONS)?;
    let mut moves = Vec::new();
    for companion in std::iter::once("").chain(COMPANIONS) {
        let from = with_suffix(path, companion);
        if from.try_exists()? {
            moves.push((from, with_suffix(&target, companion)));
        }
    }
    let mut done: Vec<&(PathBuf, PathBuf)> = Vec::new();
    for pair in &moves {
        if let Err(error) = fs::rename(&pair.0, &pair.1) {
            for (from, to) in done.iter().rev() {
                // Best effort: put back what was moved.
                let _ = fs::rename(to, from);
            }
            return Err(error);
        }
        done.push(pair);
    }
    Ok(())
}

/// `path` + `suffix`, or with `-2`, `-3`... after it, the first whose name
/// and companion names are all free. Renaming over an existing file would
/// replace it on Windows.
fn free_name(path: &Path, suffix: &str, companions: &[&str]) -> io::Result<PathBuf> {
    let base = with_suffix(path, suffix);
    for n in 1..=1000 {
        let candidate = if n == 1 {
            base.clone()
        } else {
            with_suffix(&base, &format!("-{n}"))
        };
        let mut taken = candidate.try_exists()?;
        for companion in companions {
            taken = taken || with_suffix(&candidate, companion).try_exists()?;
        }
        if !taken {
            return Ok(candidate);
        }
    }
    Err(io::Error::from(io::ErrorKind::AlreadyExists))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

/// The migrations to run, with the seed in migration 1: a database at
/// version 0 has no tables of ours, so it is new, and it gets its schema and
/// its first widgets (and the recovery notice) in the same transaction, or
/// neither. A database emptied by the user stays empty.
fn with_seed(migrations: &[&'static str]) -> Vec<M<'static>> {
    migrations
        .iter()
        .enumerate()
        .map(|(index, sql)| {
            if index == 0 {
                M::up_with_hook(sql, |tx: &Transaction<'_>| -> HookResult { Ok(seed(tx)?) })
            } else {
                M::up(sql)
            }
        })
        .collect()
}

/// Inserts the first widgets of a new database and, if it replaces one set
/// aside ([`RECOVERY_MARK`]), the recovery notice.
fn seed(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    let now = now_ms();
    for (position, kind) in SEED.into_iter().enumerate() {
        widgets_repo::insert(tx, kind, position, now)?;
    }
    let mark: i32 = tx.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if mark == RECOVERY_MARK {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![RECOVERY_NOTICE_KEY, RECOVERY_NOTICE_PENDING],
        )?;
        tx.pragma_update(None, "application_id", 0)?;
    }
    Ok(())
}

fn recovery_pending(conn: &Connection) -> rusqlite::Result<bool> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![RECOVERY_NOTICE_KEY],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value.as_deref() == Some(RECOVERY_NOTICE_PENDING))
}

/// Starts a write transaction, taking the write lock up front so it can't
/// fail halfway on a lock held by another connection.
pub(super) fn begin(conn: &mut Connection) -> Result<Transaction<'_>, WidgetError> {
    conn.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(write_failed)
}

/// Logs a SQLite error by its code and turns it into
/// [`WidgetError::WriteFailed`]. SQLite's message is dropped: it can quote
/// the file name.
pub(super) fn write_failed(error: rusqlite::Error) -> WidgetError {
    log::warn!("widgets: database error: {}", Cause::sqlite(&error));
    WidgetError::WriteFailed
}

/// Now, in milliseconds since the Unix epoch.
pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

fn is_corrupt(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
    )
}

/// The kind of an open failure, safe to log: SQLite's result code or the
/// I/O error kind, never a message (which can name the file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cause {
    Sqlite(Option<ErrorCode>),
    Io(io::ErrorKind),
    NotSqlite,
    IntegrityCheck,
    OrphanCompanions,
    Migration(&'static str),
}

impl Cause {
    fn sqlite(error: &rusqlite::Error) -> Cause {
        Cause::Sqlite(error.sqlite_error_code())
    }

    fn migration(error: &rusqlite_migration::Error) -> Cause {
        match error {
            rusqlite_migration::Error::RusqliteError { err, .. } => Cause::sqlite(err),
            rusqlite_migration::Error::MigrationDefinition(_) => Cause::Migration("definition"),
            rusqlite_migration::Error::ForeignKeyCheck(_) => Cause::Migration("foreign key check"),
            rusqlite_migration::Error::InvalidUserVersion => Cause::Migration("invalid version"),
            _ => Cause::Migration("other"),
        }
    }
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::Sqlite(Some(code)) => write!(f, "SQLite {code:?}"),
            Cause::Sqlite(None) => f.write_str("SQLite interface error"),
            Cause::Io(kind) => write!(f, "{kind}"),
            Cause::NotSqlite => f.write_str("not a SQLite file"),
            Cause::IntegrityCheck => f.write_str("integrity check failed"),
            Cause::OrphanCompanions => f.write_str("companion files without a database"),
            Cause::Migration(kind) => write!(f, "migration {kind} error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::domain::widgets::{WidgetConfig, WidgetInstance};
    use crate::persistence::notes_repo;

    /// A second migration, for a database "from a newer version" and for
    /// migrating an existing one.
    const MIGRATION_2: &str = "ALTER TABLE meta ADD COLUMN extra TEXT;";

    fn up_to_v2() -> Vec<&'static str> {
        vec![MIGRATION_1, MIGRATION_2]
    }

    /// Migration 2 creates a table and then fails: the whole migration must
    /// be rolled back.
    fn failing_v2() -> Vec<&'static str> {
        vec![
            MIGRATION_1,
            "CREATE TABLE half_done (x INTEGER) STRICT; \
             INSERT INTO no_such_table VALUES (1);",
        ]
    }

    /// Migration 1 and a trigger that refuses every widget: the seed fails
    /// right after the schema is made, as if the app stopped between them.
    const SEED_REFUSED: &str = concat!(
        include_str!("../../migrations/0001_widgets.sql"),
        "
CREATE TRIGGER refuse_seed BEFORE INSERT ON widget_instances          BEGIN SELECT RAISE(ABORT, 'stopped'); END;"
    );

    fn db_path(dir: &TempDir) -> PathBuf {
        dir.path().join(FILE_NAME)
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn widgets(storage: &Storage) -> Vec<WidgetInstance> {
        storage.snapshot().unwrap().widgets
    }

    fn kinds(storage: &Storage) -> Vec<WidgetKind> {
        widgets(storage).iter().map(WidgetInstance::kind).collect()
    }

    /// A raw connection to inspect a file the storage has closed.
    fn inspect(path: &Path) -> Connection {
        Connection::open(path).unwrap()
    }

    fn user_version(path: &Path) -> i64 {
        inspect(path)
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn modified(path: &Path) -> SystemTime {
        fs::metadata(path).unwrap().modified().unwrap()
    }

    /// Opens with the real migrations, adds a widget and closes, leaving a
    /// version 1 database with four widgets.
    fn v1_database(dir: &TempDir) -> Vec<WidgetInstance> {
        let mut storage = Storage::open(dir.path());
        widgets_repo::add(storage.connection().unwrap(), WidgetKind::Clock).unwrap();
        widgets(&storage)
    }

    fn unavailable_with(reason: UnavailableReason) -> StorageStatus {
        StorageStatus::Unavailable { reason }
    }

    #[test]
    fn real_migrations_validate() {
        assert_eq!(Migrations::new(with_seed(MIGRATIONS)).validate(), Ok(()));
        assert_eq!(Migrations::new(with_seed(&up_to_v2())).validate(), Ok(()));
    }

    #[test]
    fn new_database_is_created_migrated_and_seeded() {
        let dir = tempdir().unwrap();

        let mut storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Ok);
        let seeded = widgets(&storage);
        assert_eq!(
            kinds(&storage),
            [
                WidgetKind::Clock,
                WidgetKind::TimeProgress,
                WidgetKind::Note
            ]
        );
        for instance in &seeded {
            assert_eq!(instance.label, None);
            assert_eq!(instance.config, WidgetConfig::default_for(instance.kind()));
        }
        let conn = storage.connection().unwrap();
        let positions: Vec<i64> = conn
            .prepare("SELECT position FROM widget_instances ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(positions, [0, 1, 2]);
        let (note_id, body): (String, String) = conn
            .query_row("SELECT instance_id, body FROM notes", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(note_id, seeded[2].id.as_str());
        assert_eq!(body, "");
        let meta: i64 = conn
            .query_row("SELECT count(*) FROM meta", [], |row| row.get(0))
            .unwrap();
        assert_eq!(meta, 0);
        drop(storage);
        assert_eq!(user_version(&db_path(&dir)), 1);
    }

    #[test]
    fn stopping_between_schema_and_seed_leaves_a_database_to_seed() {
        let dir = tempdir().unwrap();
        let stopped = open_with(dir.path(), &[SEED_REFUSED]);
        assert!(matches!(
            stopped.status(),
            StorageStatus::Unavailable { .. }
        ));
        drop(stopped);

        let reopened = Storage::open(dir.path());

        assert_eq!(reopened.status(), StorageStatus::Ok);
        assert_eq!(kinds(&reopened), SEED);
    }

    #[test]
    fn stopping_between_schema_and_seed_after_a_recovery_keeps_the_notice() {
        let dir = tempdir().unwrap();
        fs::write(db_path(&dir), "garbage".repeat(100)).unwrap();
        let stopped = open_with(dir.path(), &[SEED_REFUSED]);
        assert!(matches!(
            stopped.status(),
            StorageStatus::Unavailable { .. }
        ));
        drop(stopped);
        // Nothing half done: no schema without its seed and notice.
        let path = db_path(&dir);
        assert_eq!(user_version(&path), 0);
        let tables: i64 = inspect(&path)
            .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
            .unwrap();
        assert_eq!(tables, 0);

        let reopened = Storage::open(dir.path());

        assert_eq!(reopened.status(), StorageStatus::Recovered);
        assert_eq!(kinds(&reopened), SEED);
        drop(reopened);
        let mark: i32 = inspect(&path)
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .unwrap();
        assert_eq!(mark, 0, "the mark goes once the notice is stored");
    }

    #[test]
    fn connection_uses_wal_foreign_keys_and_a_busy_timeout() {
        let dir = tempdir().unwrap();
        let mut storage = Storage::open(dir.path());
        let conn = storage.connection().unwrap();

        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let foreign_keys: i64 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        let timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |row| row.get(0))
            .unwrap();

        assert_eq!(mode, "wal");
        assert_eq!(foreign_keys, 1);
        assert_eq!(timeout, 5000);
    }

    #[test]
    fn missing_data_folder_is_created() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("dev.maxsueleinstein.dockmax").join("data");

        let storage = Storage::open(&nested);

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert!(nested.join(FILE_NAME).is_file());
    }

    #[test]
    fn reopening_keeps_the_widgets_and_does_not_seed_again() {
        let dir = tempdir().unwrap();
        let first = widgets(&Storage::open(dir.path()));

        let reopened = Storage::open(dir.path());

        assert_eq!(reopened.status(), StorageStatus::Ok);
        assert_eq!(widgets(&reopened), first);
    }

    #[test]
    fn database_emptied_by_the_user_stays_empty() {
        let dir = tempdir().unwrap();
        let mut storage = Storage::open(dir.path());
        for instance in widgets(&storage) {
            widgets_repo::remove(storage.connection().unwrap(), &instance.id).unwrap();
        }
        drop(storage);

        let reopened = Storage::open(dir.path());

        assert_eq!(reopened.status(), StorageStatus::Ok);
        assert_eq!(widgets(&reopened), []);
    }

    #[test]
    fn older_database_is_backed_up_and_then_migrated() {
        let dir = tempdir().unwrap();
        let before = v1_database(&dir);
        let path = db_path(&dir);
        let original = fs::read(&path).unwrap();
        let migrations = up_to_v2();

        let storage = open_with(dir.path(), &migrations);

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert_eq!(widgets(&storage), before, "same widgets, not seeded again");
        drop(storage);
        assert_eq!(user_version(&path), 2);
        let backup = dir.path().join("dockmax.db.bak-v1");
        assert_eq!(user_version(&backup), 1);
        let backed_up: i64 = inspect(&backup)
            .query_row("SELECT count(*) FROM widget_instances", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(backed_up, 4);
        assert_ne!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn backup_never_replaces_an_earlier_one() {
        let dir = tempdir().unwrap();
        v1_database(&dir);
        let earlier = dir.path().join("dockmax.db.bak-v1");
        fs::write(&earlier, "an earlier backup").unwrap();
        let migrations = up_to_v2();

        let storage = open_with(dir.path(), &migrations);

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert_eq!(fs::read_to_string(&earlier).unwrap(), "an earlier backup");
        assert_eq!(user_version(&dir.path().join("dockmax.db.bak-v1-2")), 1);
    }

    #[test]
    fn failed_migration_leaves_the_file_as_it_was() {
        let dir = tempdir().unwrap();
        v1_database(&dir);
        let path = db_path(&dir);
        let original = fs::read(&path).unwrap();
        let migrations = failing_v2();

        let mut storage = open_with(dir.path(), &migrations);

        let status = unavailable_with(UnavailableReason::MigrationFailed);
        assert_eq!(storage.status(), status);
        assert_eq!(storage.connection().unwrap_err(), WidgetError::Unavailable);
        assert_eq!(
            storage.snapshot(),
            Ok(WidgetsSnapshot {
                storage: status,
                widgets: vec![],
            })
        );
        drop(storage);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(user_version(&path), 1);
        let half_done: i64 = inspect(&path)
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'half_done'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(half_done, 0);
    }

    #[test]
    fn newer_database_is_left_untouched_and_unavailable() {
        let dir = tempdir().unwrap();
        let migrations = up_to_v2();
        drop(open_with(dir.path(), &migrations));
        let path = db_path(&dir);
        let bytes = fs::read(&path).unwrap();
        let mtime = modified(&path);
        let names = file_names(dir.path());

        let mut storage = Storage::open(dir.path());

        assert_eq!(
            storage.status(),
            unavailable_with(UnavailableReason::NewerVersion)
        );
        assert_eq!(storage.connection().unwrap_err(), WidgetError::Unavailable);
        drop(storage);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(modified(&path), mtime);
        assert_eq!(file_names(dir.path()), names);
        assert_eq!(user_version(&path), 2);
    }

    #[test]
    fn newer_database_keeps_its_wal_unapplied() {
        // A copy taken while the database is open: the newer changes are
        // still in the `-wal`, not in the file.
        let live = tempdir().unwrap();
        let migrations = up_to_v2();
        let storage = open_with(live.path(), &migrations);
        let dir = tempdir().unwrap();
        for name in [FILE_NAME, "dockmax.db-wal", "dockmax.db-shm"] {
            fs::copy(live.path().join(name), dir.path().join(name)).unwrap();
        }
        drop(storage);
        let path = db_path(&dir);
        let wal = with_suffix(&path, "-wal");
        let bytes = fs::read(&path).unwrap();
        let wal_bytes = fs::read(&wal).unwrap();
        assert!(!wal_bytes.is_empty());

        let storage = Storage::open(dir.path());

        assert_eq!(
            storage.status(),
            unavailable_with(UnavailableReason::NewerVersion)
        );
        drop(storage);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&wal).unwrap(), wal_bytes);
        assert_eq!(
            file_names(dir.path()),
            [FILE_NAME, "dockmax.db-shm", "dockmax.db-wal"]
        );
    }

    /// The `dockmax.db.corrompido-<stamp>` file next to the new database.
    fn set_aside_name(dir: &Path) -> String {
        let names: Vec<String> = file_names(dir)
            .into_iter()
            .filter(|name| name.starts_with("dockmax.db.corrompido-"))
            .filter(|name| {
                !["-wal", "-shm", "-journal"]
                    .iter()
                    .any(|c| name.ends_with(c))
            })
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        let stamp = names[0].trim_start_matches("dockmax.db.corrompido-");
        let (date, time) = stamp.split_once('-').expect("AAAAMMDD-HHMMSS");
        assert!(
            date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()),
            "{stamp}"
        );
        assert!(
            time.len() == 6 && time.bytes().all(|b| b.is_ascii_digit()),
            "{stamp}"
        );
        names[0].clone()
    }

    #[test]
    fn file_that_is_not_a_database_is_set_aside_and_replaced() {
        let dir = tempdir().unwrap();
        let path = db_path(&dir);
        let garbage = "this is not a SQLite database\n".repeat(200);
        fs::write(&path, &garbage).unwrap();
        fs::write(with_suffix(&path, "-wal"), "old wal").unwrap();
        fs::write(with_suffix(&path, "-shm"), "old shm").unwrap();
        fs::write(with_suffix(&path, "-journal"), "old journal").unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Recovered);
        assert_eq!(
            kinds(&storage),
            [
                WidgetKind::Clock,
                WidgetKind::TimeProgress,
                WidgetKind::Note
            ]
        );
        drop(storage);
        let aside = set_aside_name(dir.path());
        assert_eq!(
            fs::read_to_string(dir.path().join(&aside)).unwrap(),
            garbage
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("{aside}-wal"))).unwrap(),
            "old wal"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("{aside}-shm"))).unwrap(),
            "old shm"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("{aside}-journal"))).unwrap(),
            "old journal"
        );
        assert_eq!(user_version(&path), 1);
    }

    /// The `-wal` and `-shm` of a live database with a saved note, copied
    /// into `dir` while it is open: the note is only in the `-wal`.
    fn copy_live_companions(dir: &TempDir) -> Vec<(&'static str, Vec<u8>)> {
        let live = tempdir().unwrap();
        let mut storage = Storage::open(live.path());
        let note = widgets(&storage)[2].id.clone();
        notes_repo::save(storage.connection().unwrap(), &note, "only in the wal").unwrap();
        let copied: Vec<(&str, Vec<u8>)> = ["-wal", "-shm"]
            .into_iter()
            .map(|companion| {
                let bytes = fs::read(with_suffix(&db_path(&live), companion)).unwrap();
                fs::write(with_suffix(&db_path(dir), companion), &bytes).unwrap();
                (companion, bytes)
            })
            .collect();
        assert!(!copied[0].1.is_empty(), "the -wal holds the note");
        drop(storage);
        copied
    }

    /// Each companion's bytes are still in some file of `dir`.
    fn assert_companions_kept(dir: &Path, companions: &[(&str, Vec<u8>)]) {
        let contents: Vec<Vec<u8>> = file_names(dir)
            .iter()
            .map(|name| fs::read(dir.join(name)).unwrap())
            .collect();
        for (companion, bytes) in companions {
            assert!(
                contents.contains(bytes),
                "the {companion} is gone: {:?}",
                file_names(dir)
            );
        }
    }

    #[test]
    fn orphan_wal_without_a_database_is_set_aside_and_recovered() {
        let dir = tempdir().unwrap();
        let companions = copy_live_companions(&dir);

        let storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Recovered);
        assert_eq!(widgets(&storage).len(), 3, "a new, seeded database");
        drop(storage);
        assert_companions_kept(dir.path(), &companions);
        let aside: Vec<String> = file_names(dir.path())
            .into_iter()
            .filter(|name| name.starts_with("dockmax.db.corrompido-"))
            .collect();
        assert_eq!(aside.len(), 2, "the -wal and the -shm: {aside:?}");
    }

    #[test]
    fn empty_database_file_with_a_wal_is_set_aside_and_recovered() {
        let dir = tempdir().unwrap();
        let companions = copy_live_companions(&dir);
        fs::write(db_path(&dir), "").unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Recovered);
        assert_eq!(widgets(&storage).len(), 3, "a new, seeded database");
        drop(storage);
        assert_companions_kept(dir.path(), &companions);
        let aside = set_aside_name(dir.path());
        assert_eq!(fs::read(dir.path().join(&aside)).unwrap(), b"");
    }

    #[test]
    fn empty_database_file_alone_is_a_new_database() {
        let dir = tempdir().unwrap();
        fs::write(db_path(&dir), "").unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Ok);
        assert_eq!(widgets(&storage).len(), 3);
        drop(storage);
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn corrupt_pages_are_detected_and_set_aside() {
        let dir = tempdir().unwrap();
        v1_database(&dir);
        let path = db_path(&dir);
        let mut bytes = fs::read(&path).unwrap();
        let page_size = usize::from(u16::from_be_bytes([bytes[16], bytes[17]]));
        assert!(
            bytes.len() >= 2 * page_size,
            "the database has a second page"
        );
        bytes[page_size..2 * page_size].fill(0xA5);
        fs::write(&path, &bytes).unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(storage.status(), StorageStatus::Recovered);
        assert_eq!(widgets(&storage).len(), 3, "a new, seeded database");
        drop(storage);
        let aside = set_aside_name(dir.path());
        assert_eq!(fs::read(dir.path().join(aside)).unwrap(), bytes);
    }

    #[test]
    fn recovery_notice_stays_until_dismissed() {
        let dir = tempdir().unwrap();
        fs::write(db_path(&dir), "garbage".repeat(100)).unwrap();
        drop(Storage::open(dir.path()));

        let mut reopened = Storage::open(dir.path());
        assert_eq!(reopened.status(), StorageStatus::Recovered);
        assert_eq!(
            reopened.snapshot().unwrap().storage,
            StorageStatus::Recovered
        );

        reopened.dismiss_recovery_notice().unwrap();
        assert_eq!(reopened.status(), StorageStatus::Ok);
        drop(reopened);

        assert_eq!(Storage::open(dir.path()).status(), StorageStatus::Ok);
    }

    #[test]
    fn dismissing_without_a_notice_keeps_ok() {
        let dir = tempdir().unwrap();
        let mut storage = Storage::open(dir.path());

        assert_eq!(storage.dismiss_recovery_notice(), Ok(()));
        assert_eq!(storage.status(), StorageStatus::Ok);
    }

    /// Holds the file open without `FILE_SHARE_DELETE`, so SQLite can read
    /// it but Windows refuses to rename it.
    #[cfg(windows)]
    #[test]
    fn corrupt_file_that_cannot_be_renamed_makes_nothing_new() {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;

        let dir = tempdir().unwrap();
        let path = db_path(&dir);
        fs::write(&path, "garbage".repeat(100)).unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&path)
            .unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(
            storage.status(),
            unavailable_with(UnavailableReason::OpenFailed)
        );
        drop(storage);
        drop(held);
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
        assert_eq!(fs::read_to_string(&path).unwrap(), "garbage".repeat(100));
    }

    #[test]
    fn folder_in_place_of_the_file_is_open_failed_and_untouched() {
        let dir = tempdir().unwrap();
        let path = db_path(&dir);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep.txt"), "kept").unwrap();

        let storage = Storage::open(dir.path());

        assert_eq!(
            storage.status(),
            unavailable_with(UnavailableReason::OpenFailed)
        );
        assert_eq!(file_names(dir.path()), [FILE_NAME]);
        assert_eq!(fs::read_to_string(path.join("keep.txt")).unwrap(), "kept");
    }

    #[test]
    fn unavailable_storage_refuses_every_use() {
        let mut storage = Storage::unavailable(UnavailableReason::OpenFailed);
        let status = unavailable_with(UnavailableReason::OpenFailed);

        assert_eq!(storage.status(), status);
        assert_eq!(storage.connection().unwrap_err(), WidgetError::Unavailable);
        assert_eq!(
            storage.dismiss_recovery_notice(),
            Err(WidgetError::Unavailable)
        );
        assert_eq!(
            storage.snapshot(),
            Ok(WidgetsSnapshot {
                storage: status,
                widgets: vec![],
            })
        );
    }

    #[test]
    fn free_name_skips_names_taken_by_a_file_or_a_companion() {
        let dir = tempdir().unwrap();
        let path = db_path(&dir);

        assert_eq!(
            free_name(&path, ".x", &COMPANIONS).unwrap(),
            dir.path().join("dockmax.db.x")
        );
        fs::write(dir.path().join("dockmax.db.x"), "").unwrap();
        fs::write(dir.path().join("dockmax.db.x-2-shm"), "").unwrap();
        assert_eq!(
            free_name(&path, ".x", &COMPANIONS).unwrap(),
            dir.path().join("dockmax.db.x-3")
        );
        assert_eq!(
            free_name(&path, ".x", &[]).unwrap(),
            dir.path().join("dockmax.db.x-2")
        );
    }

    #[test]
    fn causes_name_only_the_kind_of_failure() {
        let causes = [
            Cause::Sqlite(Some(ErrorCode::NotADatabase)),
            Cause::Sqlite(None),
            Cause::Io(io::ErrorKind::PermissionDenied),
            Cause::NotSqlite,
            Cause::IntegrityCheck,
            Cause::OrphanCompanions,
            Cause::Migration("definition"),
        ];
        for cause in causes {
            let message = cause.to_string();
            assert!(!message.is_empty());
            assert!(
                !message.contains('/') && !message.contains('\\') && !message.contains(FILE_NAME),
                "{message}"
            );
        }
    }
}
