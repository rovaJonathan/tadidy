//! `storage::open` against a real SQLCipher database in a temporary folder, with a fixed test
//! key. Never the Keychain, never the user's data folder (AGENTS.md §13).

// `allow-unwrap-in-tests` covers the `#[test]` functions, not the helpers around them.
#![allow(clippy::unwrap_used)]

use rusqlite::Connection;
use tadidy_lib::crypto::DbKey;
use tadidy_lib::storage::{Db, StorageError, open};
use tempfile::TempDir;

fn key_a() -> DbKey {
    DbKey::from_bytes([0xA; 32])
}

fn key_b() -> DbKey {
    DbKey::from_bytes([0xB; 32])
}

fn new_db_path() -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    (dir, path)
}

fn pragma(db: &Db, name: &str) -> String {
    db.with_conn(|conn| {
        conn.query_row(&format!("PRAGMA {name}"), [], |row| {
            row.get::<_, rusqlite::types::Value>(0)
        })
    })
    .map(|value| match value {
        rusqlite::types::Value::Integer(n) => n.to_string(),
        rusqlite::types::Value::Text(s) => s,
        other => format!("{other:?}"),
    })
    .unwrap()
}

#[test]
fn opening_twice_succeeds_and_migrates_to_version_one() {
    let (_dir, path) = new_db_path();
    open(&path, &key_a()).unwrap();

    let db = open(&path, &key_a()).unwrap();

    assert_eq!(pragma(&db, "user_version"), "1");
    let tables: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'settings'",
                [],
                |row| row.get(0),
            )
        })
        .unwrap();
    assert_eq!(tables, 1);
}

#[test]
fn opening_with_another_key_is_a_wrong_key() {
    let (_dir, path) = new_db_path();
    open(&path, &key_a()).unwrap();

    let result = open(&path, &key_b());

    assert!(matches!(result, Err(StorageError::WrongKey)));
}

#[test]
fn opening_without_a_key_fails_with_not_a_database() {
    let (_dir, path) = new_db_path();
    open(&path, &key_a()).unwrap();

    let conn = Connection::open(&path).unwrap();
    let error = conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_err();

    assert!(
        error.to_string().contains("file is not a database"),
        "{error}"
    );
}

#[test]
fn the_file_holds_no_plain_text() {
    let (_dir, path) = new_db_path();
    let db = open(&path, &key_a()).unwrap();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO settings VALUES ('marker', 'plain-marker-value')",
            [],
        )
    })
    .unwrap();
    drop(db);

    let bytes = std::fs::read(&path).unwrap();

    assert!(!bytes.starts_with(b"SQLite format 3"));
    assert!(
        !bytes
            .windows(18)
            .any(|window| window == b"plain-marker-value")
    );
}

#[test]
fn the_connection_is_configured_for_wal_incremental_vacuum_and_foreign_keys() {
    let (_dir, path) = new_db_path();

    let db = open(&path, &key_a()).unwrap();

    assert_eq!(pragma(&db, "journal_mode"), "wal");
    assert_eq!(pragma(&db, "auto_vacuum"), "2");
    assert_eq!(pragma(&db, "foreign_keys"), "1");
}
