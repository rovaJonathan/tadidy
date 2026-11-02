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
#[ts(export)]
pub struct StorageStatus {
    pub state: StorageState,
    /// The `AppError` code of the failure that made the database unavailable, `null` when ready.
    pub code: Option<String>,
}

impl StorageStatus {
    pub fn ready() -> Self {
        Self {
            state: StorageState::Ready,
            code: None,
        }
    }

    pub fn unavailable(code: impl Into<String>) -> Self {
        Self {
            state: StorageState::Unavailable,
            code: Some(code.into()),
        }
    }
}

impl StorageStatus {
    /// Whether the history may be reset: only when the key is gone or does not open the file.
    /// Any other failure can be temporary (a Keychain prompt that was denied, a locked
    /// Keychain): a reset would then throw away a history that is only out of reach for now.
    pub fn can_reset(&self) -> bool {
        matches!(
            self.code.as_deref(),
            Some("storage_key_missing" | "storage_wrong_key")
        )
    }
}

impl Default for StorageStatus {
    /// Before the first open: nothing is promised yet.
    fn default() -> Self {
        Self::unavailable("storage_unavailable")
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
            json!({ "state": "ready", "code": null })
        );
    }

    #[test]
    fn only_a_missing_or_wrong_key_can_be_reset() {
        assert!(StorageStatus::unavailable("storage_key_missing").can_reset());
        assert!(StorageStatus::unavailable("storage_wrong_key").can_reset());
        assert!(!StorageStatus::unavailable("storage_error").can_reset());
        assert!(!StorageStatus::unavailable("storage_unavailable").can_reset());
        assert!(!StorageStatus::ready().can_reset());
    }

    #[test]
    fn an_unavailable_status_carries_the_failure_code() {
        assert_eq!(
            json!(StorageStatus::unavailable("storage_key_missing")),
            json!({ "state": "unavailable", "code": "storage_key_missing" })
        );
    }
}
