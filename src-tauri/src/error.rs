//! The error every IPC command returns. It reaches the webview as `{ code, message }`:
//! the message never contains clip content or a path under the user's home (AGENTS.md §13).

use serde::Serialize;
use ts_rs::TS;

use crate::storage::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[ts(export)]
#[error("{code}: {message}")]
pub struct AppError {
    /// Stable, machine-readable code the front can branch on, e.g. `not_found`.
    pub code: String,
    /// Human-readable explanation, safe to show and to log.
    pub message: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<StorageError> for AppError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::KeyMissing => Self::new(
                "storage_key_missing",
                "The history key is missing from the Keychain.",
            ),
            StorageError::WrongKey => Self::new(
                "storage_wrong_key",
                "The history key does not match the database.",
            ),
            _ => Self::new("storage_error", "The history database could not be used."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn app_error_serializes_to_code_and_message() {
        let error = AppError::new("not_found", "The clip does not exist.");

        let value = serde_json::to_value(&error).unwrap();

        assert_eq!(
            value,
            json!({ "code": "not_found", "message": "The clip does not exist." })
        );
    }
}
