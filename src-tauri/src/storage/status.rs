//! Whether the history database is usable, as the webview sees it (IPC `get_storage_status`
//! and the event `storage://changed`). Only an error code travels, never a path or a key.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum StorageState {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct StorageStatus {
    pub state: StorageState,
    /// The `AppError` code of the failure that made the database unavailable, `null` when ready.
    pub code: Option<String>,
    /// Whether the history may be reset: only when the key is gone or does not open the file.
    /// Any other failure can be temporary (a Keychain prompt that was denied, a locked
    /// Keychain): a reset would then throw away a history that is only out of reach for now.
    /// The front shows its button from this flag, never from its own list of codes.
    pub can_reset: bool,
}

impl StorageStatus {
    pub fn ready() -> Self {
        Self {
            state: StorageState::Ready,
            code: None,
            can_reset: false,
        }
    }

    pub fn unavailable(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            state: StorageState::Unavailable,
            can_reset: matches!(
                code.as_str(),
                "storage_key_missing"
                    | "storage_wrong_key"
                    | "storage_key_malformed"
                    | "storage_migration_failed"
            ),
            code: Some(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_ready_status_has_no_code() {
        assert_eq!(
            json!(StorageStatus::ready()),
            json!({ "state": "ready", "code": null, "canReset": false })
        );
    }

    #[test]
    fn only_a_failure_that_cannot_pass_can_be_reset() {
        assert!(StorageStatus::unavailable("storage_key_missing").can_reset);
        assert!(StorageStatus::unavailable("storage_wrong_key").can_reset);
        assert!(StorageStatus::unavailable("storage_key_malformed").can_reset);
        assert!(StorageStatus::unavailable("storage_migration_failed").can_reset);
        assert!(!StorageStatus::unavailable("storage_error").can_reset);
        assert!(!StorageStatus::unavailable("storage_unavailable").can_reset);
        assert!(!StorageStatus::ready().can_reset);
    }

    #[test]
    fn the_codes_of_the_errors_that_lose_the_key_are_the_ones_that_can_be_reset() {
        use crate::crypto::KeyStoreError;
        use crate::error::AppError;
        use crate::storage::StorageError;
        for (error, resettable) in [
            (StorageError::KeyMissing, true),
            (StorageError::WrongKey, true),
            (StorageError::KeyStore(KeyStoreError::Malformed), true),
            (StorageError::KeyStore(KeyStoreError::Unavailable), false),
            (
                StorageError::Migration(rusqlite_migration::Error::InvalidUserVersion),
                true,
            ),
            (
                StorageError::Migration(rusqlite_migration::Error::with_sql(
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                        None,
                    ),
                    "",
                )),
                false,
            ),
            (StorageError::InvalidCursor, false),
            (StorageError::Io(std::io::ErrorKind::Other.into()), false),
        ] {
            // The core decides from the error, the front from the code: they must agree.
            assert_eq!(error.is_key_lost(), resettable);
            let code = AppError::from(error).code;
            assert_eq!(StorageStatus::unavailable(code).can_reset, resettable);
        }
    }

    #[test]
    fn an_unavailable_status_carries_the_failure_code() {
        assert_eq!(
            json!(StorageStatus::unavailable("storage_key_missing")),
            json!({ "state": "unavailable", "code": "storage_key_missing", "canReset": true })
        );
    }
}
