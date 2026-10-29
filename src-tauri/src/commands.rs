//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

use tauri::{Runtime, State, WebviewWindow};

use crate::error::AppError;
use crate::state::AppState;

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

/// Whether Tadidy may post the paste keystroke. Never shows the system prompt.
#[tauri::command]
pub fn check_accessibility(state: State<'_, AppState>) -> bool {
    state.accessibility.is_trusted()
}

/// Lists Tadidy under *Privacy & Security → Accessibility* and opens that pane. Called from a
/// user click only: the permission is never requested silently.
#[tauri::command]
pub fn open_accessibility_settings(state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .accessibility
        .request_and_open_settings()
        .map_err(|_| {
            AppError::new(
                "accessibility_settings_unavailable",
                "The Accessibility settings could not be opened.",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender};
    use crate::state::AppState;
    use std::sync::Arc;
    use tauri::Manager;

    /// A mock app whose state holds the given probe and a key sender that is never used here.
    fn app_with(accessibility: &Arc<FakeAccessibility>) -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(AppState::new(
            accessibility.clone(),
            Arc::new(FakeKeySender::new()),
        ));
        app
    }

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

    #[test]
    fn check_accessibility_reflects_the_probe() {
        let accessibility = Arc::new(FakeAccessibility::new());
        let app = app_with(&accessibility);

        accessibility.set_trusted(true);
        assert!(check_accessibility(app.state()));

        accessibility.set_trusted(false);
        assert!(!check_accessibility(app.state()));
    }

    #[test]
    fn check_accessibility_never_prompts() {
        let accessibility = Arc::new(FakeAccessibility::new());
        let app = app_with(&accessibility);

        check_accessibility(app.state());

        assert_eq!(accessibility.requests(), 0);
    }

    #[test]
    fn open_accessibility_settings_requests_once() {
        let accessibility = Arc::new(FakeAccessibility::new());
        let app = app_with(&accessibility);

        open_accessibility_settings(app.state()).unwrap();

        assert_eq!(accessibility.requests(), 1);
    }

    #[test]
    fn open_accessibility_settings_maps_failure_to_app_error() {
        let accessibility = Arc::new(FakeAccessibility::new());
        accessibility.fail_requests();
        let app = app_with(&accessibility);

        let error = open_accessibility_settings(app.state()).unwrap_err();

        assert_eq!(error.code, "accessibility_settings_unavailable");
        assert_eq!(accessibility.requests(), 1);
    }
}
