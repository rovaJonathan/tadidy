//! The cache of source app icons, keyed by bundle id.

use rusqlite::{Connection, OptionalExtension};

use super::StorageError;

pub fn has(conn: &Connection, bundle_id: &str) -> Result<bool, StorageError> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM app_icons WHERE bundle_id = ?1)",
        [bundle_id],
        |row| row.get(0),
    )?)
}

/// Stores the icon, replacing the one already cached for this app.
pub fn put(conn: &Connection, bundle_id: &str, png: &[u8]) -> Result<(), StorageError> {
    conn.execute(
        "INSERT OR REPLACE INTO app_icons (bundle_id, png) VALUES (?1, ?2)",
        rusqlite::params![bundle_id, png],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, bundle_id: &str) -> Result<Option<Vec<u8>>, StorageError> {
    Ok(conn
        .query_row(
            "SELECT png FROM app_icons WHERE bundle_id = ?1",
            [bundle_id],
            |row| row.get(0),
        )
        .optional()?)
}
