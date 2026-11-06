//! The encrypted database (SQLCipher). The connection is opened with the key first, checked, set
//! up, then migrated. Clip content never appears in an error or a log (AGENTS.md §2).

pub mod app_icons;
pub mod clips;
pub mod fts;
mod migrations;
pub mod reset;
pub mod settings;
pub mod status;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, ErrorCode};

use crate::crypto::{DbKey, KeyStore, KeyStoreError, load_or_create_key};
use crate::error::AppError;

/// The database file inside the data folder.
pub const DB_FILE_NAME: &str = "tadidy.db";

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The database exists but the Keychain has no key for it: it can never be read again.
    #[error("the database key is missing")]
    KeyMissing,
    /// The key does not decrypt the file (SQLCipher reports "file is not a database").
    #[error("the database key does not match the database")]
    WrongKey,
    #[error("migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("key store failed: {0}")]
    KeyStore(#[from] KeyStoreError),
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    /// The cursor of a list query is not one this app handed out.
    #[error("the list cursor is not valid")]
    InvalidCursor,
}

impl StorageError {
    /// The key is gone, is not a key, or does not open the file: the history can never be read
    /// again, so it may be started over. Any other failure can pass (a denied Keychain prompt, a
    /// locked Keychain).
    pub fn is_key_lost(&self) -> bool {
        matches!(
            self,
            Self::KeyMissing | Self::WrongKey | Self::KeyStore(KeyStoreError::Malformed)
        )
    }
}

/// The one connection, shared between threads. Services borrow it through [`Db::with_conn`];
/// they never block the main thread on it (AGENTS.md §13).
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> T) -> T {
        // A panic in another closure does not corrupt SQLite: keep serving the connection.
        let conn = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Db")
    }
}

/// Opens (or creates) the database at `path` with `key` and brings its schema up to date.
pub fn open(path: &Path, key: &DbKey) -> Result<Db, StorageError> {
    let mut conn = Connection::open(path)?;
    // `PRAGMA key` takes no bound parameter; the value is `x'` + 64 hex digits, nothing else.
    conn.execute_batch(&format!("PRAGMA key = \"{}\";", key.to_pragma_value()))?;
    check_key(&conn)?;
    // Only effective before the first table exists, hence before the migrations.
    conn.execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrations::MIGRATIONS.to_latest(&mut conn)?;
    Ok(Db(Arc::new(Mutex::new(conn))))
}

/// The first read decrypts the first page: with a wrong key (or none) SQLCipher answers
/// "file is not a database".
fn check_key(conn: &Connection) -> Result<(), StorageError> {
    match conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())) {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == ErrorCode::NotADatabase => {
            Err(StorageError::WrongKey)
        }
        Err(error) => Err(error.into()),
    }
}

/// Opens `tadidy.db` in the data folder, creating the folder and the key on the first launch.
/// A database whose key is gone is left untouched ([`StorageError::KeyMissing`]).
pub fn open_in_dir(dir: &Path, store: &dyn KeyStore) -> Result<Db, StorageError> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(DB_FILE_NAME);
    let key = load_or_create_key(store, path.exists())?;
    open(&path, &key)
}

/// What `setup` does at startup: the database, or the error to report (logged by its code only).
/// A database whose key is gone is left untouched (spec *Encryption*): the app runs without it
/// and offers a reset. Only the error code is logged, never a path or a key.
pub fn open_for_startup(dir: &Path, store: &dyn KeyStore) -> Result<Db, AppError> {
    open_in_dir(dir, store).map_err(|error| {
        let error = AppError::from(error);
        tracing::error!(code = %error.code, "the history database is unavailable");
        error
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{DbKey, FakeKeyStore};
    use crate::error::AppError;

    #[test]
    fn open_in_dir_creates_the_database_and_the_key_on_first_launch() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();

        open_in_dir(dir.path(), &store).unwrap();

        assert!(dir.path().join(DB_FILE_NAME).exists());
        assert!(store.get().unwrap().is_some());
    }

    #[test]
    fn open_in_dir_creates_the_data_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("app.tadidy");

        open_in_dir(&data_dir, &FakeKeyStore::default()).unwrap();

        assert!(data_dir.join(DB_FILE_NAME).exists());
    }

    #[test]
    fn open_in_dir_reopens_with_the_stored_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        open_in_dir(dir.path(), &store).unwrap();

        assert!(open_in_dir(dir.path(), &store).is_ok());
    }

    #[test]
    fn open_in_dir_refuses_to_replace_a_database_whose_key_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let before = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();

        let result = open_in_dir(dir.path(), &FakeKeyStore::default());

        assert!(matches!(result, Err(StorageError::KeyMissing)));
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
    }

    #[test]
    fn open_for_startup_returns_the_database() {
        let dir = tempfile::tempdir().unwrap();

        assert!(open_for_startup(dir.path(), &FakeKeyStore::default()).is_ok());
    }

    #[test]
    fn open_for_startup_reports_the_code_when_the_key_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        let error = open_for_startup(dir.path(), &FakeKeyStore::default()).unwrap_err();

        assert_eq!(error.code, "storage_key_missing");
    }

    #[test]
    fn open_for_startup_reports_the_code_of_a_wrong_key() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let other = FakeKeyStore::with_key(DbKey::from_bytes([9; 32]));

        let error = open_for_startup(dir.path(), &other).unwrap_err();

        assert_eq!(error.code, "storage_wrong_key");
    }

    #[test]
    fn open_for_startup_reports_the_code_of_a_malformed_key() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        let error = open_for_startup(dir.path(), &FakeKeyStore::malformed()).unwrap_err();

        assert_eq!(error.code, "storage_key_malformed");
    }

    /// A database created by this app, then given another schema version (`user_version`).
    fn database_at_version(version: i64) -> (tempfile::TempDir, FakeKeyStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        let db = open_in_dir(dir.path(), &store).unwrap();
        db.with_conn(|conn| conn.pragma_update(None, "user_version", version))
            .unwrap();
        (dir, store)
    }

    /// A migration stopped by SQLite with `code`.
    fn migration_error(code: std::ffi::c_int) -> StorageError {
        StorageError::Migration(rusqlite_migration::Error::with_sql(
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None),
            "",
        ))
    }

    #[test]
    fn open_for_startup_reports_the_code_of_a_migration_that_fails() {
        // Version 1 makes the second migration run again: its tables exist already.
        let (dir, store) = database_at_version(1);

        let error = open_for_startup(dir.path(), &store).unwrap_err();

        assert_eq!(error.code, "storage_migration_failed");
    }

    #[test]
    fn a_migration_stopped_by_a_damaged_file_or_a_bad_version_fails_for_good() {
        let code = |error: StorageError| AppError::from(error).code;

        assert_eq!(
            code(migration_error(rusqlite::ffi::SQLITE_CORRUPT)),
            "storage_migration_failed"
        );
        assert_eq!(
            code(StorageError::Migration(
                rusqlite_migration::Error::InvalidUserVersion
            )),
            "storage_migration_failed"
        );
    }

    #[test]
    fn a_migration_stopped_by_a_failure_that_can_pass_is_a_storage_error() {
        for sqlite_code in [
            rusqlite::ffi::SQLITE_BUSY,
            rusqlite::ffi::SQLITE_LOCKED,
            rusqlite::ffi::SQLITE_FULL,
            rusqlite::ffi::SQLITE_IOERR,
            rusqlite::ffi::SQLITE_READONLY,
            rusqlite::ffi::SQLITE_CANTOPEN,
            rusqlite::ffi::SQLITE_NOMEM,
        ] {
            let error = migration_error(sqlite_code);
            assert!(!error.is_key_lost());
            assert_eq!(AppError::from(error).code, "storage_error");
        }
    }

    #[test]
    fn open_in_dir_reports_a_wrong_key() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let other = FakeKeyStore::with_key(DbKey::from_bytes([9; 32]));

        let result = open_in_dir(dir.path(), &other);

        assert!(matches!(result, Err(StorageError::WrongKey)));
    }

    #[test]
    fn storage_errors_map_to_stable_codes_without_leaking_details() {
        let code = |error: StorageError| AppError::from(error).code;

        assert_eq!(code(StorageError::KeyMissing), "storage_key_missing");
        assert_eq!(code(StorageError::WrongKey), "storage_wrong_key");
        assert_eq!(
            code(StorageError::KeyStore(KeyStoreError::Malformed)),
            "storage_key_malformed"
        );
        assert_eq!(
            code(StorageError::KeyStore(KeyStoreError::Unavailable)),
            "storage_error"
        );
        assert_eq!(
            code(StorageError::Sqlite(rusqlite::Error::InvalidQuery)),
            "storage_error"
        );
    }

    #[test]
    fn with_conn_runs_on_the_shared_connection() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        db.with_conn(|conn| conn.execute("INSERT INTO settings VALUES ('k', '1')", []))
            .unwrap();
        let value: String = db
            .clone()
            .with_conn(|conn| {
                conn.query_row("SELECT value FROM settings WHERE key = 'k'", [], |row| {
                    row.get(0)
                })
            })
            .unwrap();

        assert_eq!(value, "1");
    }

    #[test]
    fn debug_does_not_print_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(format!("{db:?}"), "Db");
    }
}
