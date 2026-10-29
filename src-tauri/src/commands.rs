//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

use std::sync::PoisonError;

use tauri::{AppHandle, Runtime, State, WebviewWindow};

use crate::error::AppError;
use crate::events::TauriEventSink;
use crate::settings::{self, Settings, SettingsPatch};
use crate::state::AppState;

/// The version of the running app, from the crate manifest.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Hides the window that calls it (`Esc` in the popover or the Quick Panel). The window is
/// hidden, not destroyed.
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
/// user click only: the permission is never requested silently. `async` keeps the wait for
/// `open` off the main thread, where a plain command would run.
#[tauri::command(async)]
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

/// The user's preferences.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state
        .settings
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Validates and stores a partial update of the preferences, applies the history size at once,
/// and returns the full settings. `async` keeps the SQLite work off the main thread.
#[tauri::command(async)]
pub fn set_settings<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, AppError> {
    let db = state.db.as_ref().ok_or_else(|| {
        AppError::new(
            "storage_unavailable",
            "The history database is unavailable.",
        )
    })?;
    settings::update(
        db,
        &state.settings,
        &state.history_limit,
        &TauriEventSink::new(app),
        patch,
    )
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
        for label in ["popover", "panel"] {
            let app = tauri::test::mock_app();
            let window = tauri::WebviewWindowBuilder::new(&app, label, Default::default())
                .build()
                .unwrap();

            // Tauri's mock runtime ignores `hide`: that it hides is checked by the manual QA.
            assert!(hide_window(window).is_ok(), "{label}");

            assert!(app.get_webview_window(label).is_some(), "{label}");
        }
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

    use crate::crypto::FakeKeyStore;
    use crate::events::SETTINGS_CHANGED;
    use crate::storage::{Db, open_in_dir};
    use tauri::Listener;

    /// A mock app whose state holds a database in `dir`, with the fakes of the platform.
    fn app_with_db(dir: &tempfile::TempDir) -> (tauri::App<tauri::test::MockRuntime>, Db) {
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let app = tauri::test::mock_app();
        app.manage(
            AppState::new(
                Arc::new(FakeAccessibility::new()),
                Arc::new(FakeKeySender::new()),
            )
            .with_db(Some(db.clone())),
        );
        (app, db)
    }

    fn patch(confirm_before_delete: bool) -> SettingsPatch {
        SettingsPatch {
            confirm_before_delete: Some(confirm_before_delete),
            ..SettingsPatch::default()
        }
    }

    #[test]
    fn get_settings_returns_the_defaults_on_an_empty_database() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _db) = app_with_db(&dir);

        assert_eq!(get_settings(app.state()), Settings::default());
    }

    #[test]
    fn set_settings_returns_stores_and_announces_the_new_settings() {
        let dir = tempfile::tempdir().unwrap();
        let (app, db) = app_with_db(&dir);
        let (sender, receiver) = std::sync::mpsc::channel();
        app.listen_any(SETTINGS_CHANGED, move |event| {
            sender.send(event.payload().to_owned()).unwrap();
        });

        let returned = set_settings(app.handle().clone(), app.state(), patch(false)).unwrap();

        let expected = Settings {
            confirm_before_delete: false,
            ..Settings::default()
        };
        assert_eq!(returned, expected);
        assert_eq!(get_settings(app.state()), expected);
        assert_eq!(
            db.with_conn(crate::storage::settings::load).unwrap(),
            expected
        );
        let payload = receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Settings>(&payload).unwrap(),
            expected
        );
        assert!(receiver.try_recv().is_err(), "announced once");
    }

    #[test]
    fn set_settings_lowers_the_shared_history_limit() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _db) = app_with_db(&dir);

        set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                history_size: Some(10),
                ..SettingsPatch::default()
            },
        )
        .unwrap();

        assert_eq!(app.state::<AppState>().history_limit.get(), Some(10));
    }

    #[test]
    fn set_settings_refuses_an_invalid_patch() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _db) = app_with_db(&dir);

        let error = set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                history_size: Some(5),
                ..SettingsPatch::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "invalid_settings");
        assert_eq!(get_settings(app.state()), Settings::default());
    }

    #[test]
    fn set_settings_without_a_database_fails_and_changes_nothing() {
        let app = tauri::test::mock_app();
        app.manage(AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
        ));

        let error = set_settings(app.handle().clone(), app.state(), patch(false)).unwrap_err();

        assert_eq!(error.code, "storage_unavailable");
        assert_eq!(get_settings(app.state()), Settings::default());
    }
}
