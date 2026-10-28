//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

use tauri::{Runtime, WebviewWindow};

use crate::error::AppError;

/// The version of the running app, from the crate manifest.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Hides the window that calls it (`Esc` in the popover). The window is hidden, not destroyed.
#[tauri::command]
pub fn hide_window<R: Runtime>(window: WebviewWindow<R>) -> Result<(), AppError> {
    window
        .hide()
        .map_err(|_| AppError::new("window", "The window could not be hidden."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[test]
    fn app_version_returns_the_crate_version() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn hide_window_succeeds_and_keeps_the_window_alive() {
        let app = tauri::test::mock_app();
        let window = tauri::WebviewWindowBuilder::new(&app, "popover", Default::default())
            .build()
            .unwrap();

        // Tauri's mock runtime ignores `hide`: that it hides is checked by the manual QA.
        assert!(hide_window(window).is_ok());

        assert!(app.get_webview_window("popover").is_some());
    }
}
