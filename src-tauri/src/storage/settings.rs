//! The `settings` table: one row per field, the key is the field's camelCase name and the value
//! its JSON. Nothing here holds clip content.

use rusqlite::Connection;

use super::StorageError;
use crate::settings::Settings;

/// The stored settings. A field that is missing, unreadable or breaks a rule falls back to its
/// default, and an unknown key is ignored: a bad row never reaches the user.
pub fn load(_conn: &Connection) -> Result<Settings, StorageError> {
    todo!()
}

/// Writes every field in one transaction.
pub fn save(_conn: &Connection, _settings: &Settings) -> Result<(), StorageError> {
    todo!()
}
