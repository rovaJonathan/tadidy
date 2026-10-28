//! The error every IPC command returns. It reaches the webview as `{ code, message }`:
//! the message never contains clip content or a path under the user's home (AGENTS.md §13).

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
