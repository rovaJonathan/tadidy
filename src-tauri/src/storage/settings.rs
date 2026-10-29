//! The `settings` table: one row per field, the key is the field's camelCase name and the value
//! its JSON. Nothing here holds clip content.

use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use super::StorageError;
use crate::settings::{Settings, SettingsPatch};

/// The stored settings. A field that is missing, unreadable or breaks a rule falls back to its
/// default, and an unknown key is ignored: a bad row never reaches the user.
pub fn load(conn: &Connection) -> Result<Settings, StorageError> {
    let mut stmt = conn.prepare_cached("SELECT key, value FROM settings")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut settings = Settings::default();
    for (key, value) in rows {
        let Ok(value) = serde_json::from_str::<Value>(&value) else {
            continue;
        };
        // One row is one field: reading it as a patch checks its type, and applying the patch
        // checks its rules. An unknown key reads as an empty patch.
        let Ok(patch) =
            serde_json::from_value::<SettingsPatch>(Value::Object(Map::from_iter([(key, value)])))
        else {
            continue;
        };
        if let Ok(next) = settings.apply(patch) {
            settings = next;
        }
    }
    Ok(settings)
}

/// Writes every field in one transaction.
pub fn save(conn: &Connection, settings: &Settings) -> Result<(), StorageError> {
    let Value::Object(fields) = serde_json::to_value(settings).unwrap_or_default() else {
        return Ok(());
    };
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )?;
        for (key, value) in fields {
            stmt.execute(params![key, value.to_string()])?;
        }
    }
    tx.commit()?;
    Ok(())
}
