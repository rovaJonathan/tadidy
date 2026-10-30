//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

use std::sync::{Arc, PoisonError};

use tauri::{AppHandle, Manager, Runtime, State, WebviewWindow};

use crate::clip::{ClipQuery, ClipSummary, Page};
use crate::error::AppError;
use crate::events::TauriEventSink;
use crate::history;
use crate::paster::{PasteMode, PasteOrigin, PasteResult, Paster};
use crate::settings::{self, Settings, SettingsPatch};
use crate::shortcuts::ShortcutStatus;
use crate::state::AppState;
use crate::storage::{self, Db};
use crate::windows;

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
    settings::update(
        database(&state)?,
        &state.settings,
        &state.history_limit,
        &TauriEventSink::new(app),
        &settings::Effects {
            login_item: state.login_item.as_ref(),
            shortcuts: state.shortcuts.as_ref(),
            shortcut_status: &state.shortcut,
        },
        patch,
    )
}

/// How the Quick Panel shortcut was registered, for the Shortcuts tab: the accelerator and the
/// code of the registration error, if any.
#[tauri::command]
pub fn get_shortcut_status(state: State<'_, AppState>) -> ShortcutStatus {
    state.shortcut_status()
}

/// Shows the Preferences window, creating it when it does not exist yet. A sync command: it runs
/// on the main thread, where AppKit windows and the activation policy belong.
#[tauri::command]
pub fn open_preferences<R: Runtime>(app: AppHandle<R>) -> Result<(), AppError> {
    windows::open_preferences(&app)
        .map_err(|_| AppError::new("window", "The Preferences window could not be opened."))
}

/// The history database, or `storage_unavailable` when it could not be opened at startup.
fn database(state: &AppState) -> Result<&Db, AppError> {
    state.db.as_ref().ok_or_else(|| {
        AppError::new(
            "storage_unavailable",
            "The history database is unavailable.",
        )
    })
}

/// One page of the history, newest first, filtered by the search when there is one.
#[tauri::command(async)]
pub fn list_clips(state: State<'_, AppState>, q: ClipQuery) -> Result<Page<ClipSummary>, AppError> {
    Ok(database(&state)?.with_conn(|conn| storage::clips::list(conn, &q))?)
}

/// Removes clips and announces it with `clips://changed`.
#[tauri::command(async)]
pub fn delete_clips<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), AppError> {
    history::delete_clips(database(&state)?, &TauriEventSink::new(app), &ids)
}

/// Empties the history and returns how many clips went; pinned clips stay when `keep_pinned`.
#[tauri::command(async)]
pub fn clear_history<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    keep_pinned: bool,
) -> Result<u32, AppError> {
    history::clear_history(database(&state)?, &TauriEventSink::new(app), keep_pinned)
}

/// Puts a clip back on the pasteboard and into the app that was in front (PST-01), or only on
/// the pasteboard when Accessibility is not granted. The window that calls it is hidden.
/// `async` keeps the wait for the target app and the SQLite work off the main thread.
#[tauri::command(async)]
pub fn paste_clip<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    id: String,
    mode: PasteMode,
) -> Result<PasteResult, AppError> {
    let origin = PasteOrigin::from_label(window.label())?;
    paster(window.app_handle(), &state)?.paste(&id, mode, origin)
}

/// Puts a clip back on the pasteboard and hides the window that calls it (PST-02).
#[tauri::command(async)]
pub fn copy_clip<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let origin = PasteOrigin::from_label(window.label())?;
    paster(window.app_handle(), &state)?.copy(&id, origin)
}

/// The paster over the app's real windows and event channel.
fn paster<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> Result<Paster, AppError> {
    Ok(state.paster(
        database(state)?.clone(),
        Arc::new(TauriEventSink::new(app.clone())),
        Arc::new(app.clone()),
    ))
}

/// The cached icon of a source app as raw PNG bytes (no JSON round trip), `not_found` when the
/// app has none.
#[tauri::command(async)]
pub fn get_app_icon(
    state: State<'_, AppState>,
    bundle_id: String,
) -> Result<tauri::ipc::Response, AppError> {
    database(&state)?
        .with_conn(|conn| storage::app_icons::get(conn, &bundle_id))?
        .map(tauri::ipc::Response::new)
        .ok_or_else(|| AppError::new("not_found", "No icon is cached for this app."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender, FakePasteboard, FakeWorkspace};
    use crate::state::AppState;

    use tauri::Manager;

    /// A mock app whose state holds the given probe and a key sender that is never used here.
    fn app_with(accessibility: &Arc<FakeAccessibility>) -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(AppState::new(
            accessibility.clone(),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
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
                Arc::new(FakePasteboard::new()),
                Arc::new(FakeWorkspace::new()),
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
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
        ));

        let error = set_settings(app.handle().clone(), app.state(), patch(false)).unwrap_err();

        assert_eq!(error.code, "storage_unavailable");
        assert_eq!(get_settings(app.state()), Settings::default());
    }

    // --- preferences -----------------------------------------------------------------------

    use crate::login_item::FakeLoginItem;
    use crate::shortcuts::{DEFAULT_QUICK_PANEL_SHORTCUT, FakeShortcutRegistrar};
    use crate::windows::{PolicyState, SETTINGS_LABEL};

    /// A mock app with a database and a login item and registrar it can inspect.
    fn app_with_system(
        dir: &tempfile::TempDir,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        Arc<FakeLoginItem>,
        Arc<FakeShortcutRegistrar>,
    ) {
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let login_item = Arc::new(FakeLoginItem::new(false));
        let shortcuts = Arc::new(FakeShortcutRegistrar::default());
        let state = AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
        )
        .with_db(Some(db))
        .with_login_item(login_item.clone())
        .with_shortcuts(shortcuts.clone());
        crate::shortcuts::register_at_startup(
            shortcuts.as_ref(),
            &state.shortcut,
            DEFAULT_QUICK_PANEL_SHORTCUT,
        );
        let app = tauri::test::mock_app();
        app.manage(state);
        (app, login_item, shortcuts)
    }

    #[test]
    fn get_shortcut_status_reports_the_registered_accelerator() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _login, _shortcuts) = app_with_system(&dir);

        assert_eq!(
            get_shortcut_status(app.state()),
            ShortcutStatus {
                accelerator: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
                error: None,
            }
        );
    }

    #[test]
    fn get_shortcut_status_reports_a_registration_that_failed_at_startup() {
        let app = app_without_db();
        app.state::<AppState>().shortcut.lock().unwrap().error = Some("unavailable".to_owned());

        assert_eq!(
            get_shortcut_status(app.state()).error.as_deref(),
            Some("unavailable")
        );
    }

    #[test]
    fn open_preferences_creates_the_settings_window() {
        let app = tauri::test::mock_app();
        app.manage(PolicyState::default());

        open_preferences(app.handle().clone()).unwrap();

        assert!(app.get_webview_window(SETTINGS_LABEL).is_some());
    }

    #[test]
    fn open_preferences_twice_leaves_one_window() {
        let app = tauri::test::mock_app();
        app.manage(PolicyState::default());

        open_preferences(app.handle().clone()).unwrap();
        open_preferences(app.handle().clone()).unwrap();

        let count = app
            .webview_windows()
            .keys()
            .filter(|label| label.as_str() == SETTINGS_LABEL)
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn open_preferences_without_the_policy_state_is_a_window_error() {
        let app = tauri::test::mock_app();

        let error = open_preferences(app.handle().clone()).unwrap_err();

        assert_eq!(error.code, "window");
    }

    #[test]
    fn set_settings_enables_the_login_item_of_the_state() {
        let dir = tempfile::tempdir().unwrap();
        let (app, login_item, _shortcuts) = app_with_system(&dir);

        let updated = set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                launch_at_login: Some(true),
                ..SettingsPatch::default()
            },
        )
        .unwrap();

        assert!(updated.launch_at_login);
        assert!(login_item.enabled());
    }

    #[test]
    fn set_settings_reports_a_login_item_that_fails() {
        let dir = tempfile::tempdir().unwrap();
        let (app, login_item, _shortcuts) = app_with_system(&dir);
        login_item.fail();

        let error = set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                launch_at_login: Some(true),
                ..SettingsPatch::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "login_item_failed");
        assert!(!get_settings(app.state()).launch_at_login);
    }

    #[test]
    fn set_settings_moves_the_shortcut_and_the_status_follows() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _login, shortcuts) = app_with_system(&dir);

        set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                quick_panel_shortcut: Some("Ctrl+Alt+V".to_owned()),
                ..SettingsPatch::default()
            },
        )
        .unwrap();

        assert!(shortcuts.is_registered("Ctrl+Alt+V"));
        assert_eq!(get_shortcut_status(app.state()).accelerator, "Ctrl+Alt+V");
    }

    #[test]
    fn set_settings_reports_a_shortcut_the_system_refuses_and_keeps_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _login, shortcuts) = app_with_system(&dir);
        shortcuts.take("Ctrl+Alt+V");

        let error = set_settings(
            app.handle().clone(),
            app.state(),
            SettingsPatch {
                quick_panel_shortcut: Some("Ctrl+Alt+V".to_owned()),
                ..SettingsPatch::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "shortcut_unavailable");
        assert!(shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(
            get_settings(app.state()).quick_panel_shortcut,
            DEFAULT_QUICK_PANEL_SHORTCUT
        );
    }

    use crate::clip::{ClipKind, NewClip, content_hash};
    use crate::events::CLIPS_CHANGED;
    use crate::platform::{Representation, UTI_PLAIN_TEXT, UTI_TADIDY_MARKER};
    use crate::storage::clips::{self, InsertOutcome};
    use tauri::ipc::{InvokeResponseBody, IpcResponse};

    /// Stores a text clip and returns its id.
    fn add(db: &Db, text: &str, now: i64) -> String {
        let clip = NewClip {
            kind: ClipKind::Text,
            content_hash: content_hash(text.as_bytes()),
            plain_text: Some(text.to_owned()),
            preview: Some(text.to_owned()),
            source_bundle: None,
            source_name: None,
            byte_size: text.len() as i64,
            representations: vec![Representation::new(UTI_PLAIN_TEXT, text.as_bytes())],
            now,
        };
        match db
            .with_conn(|conn| clips::insert_or_bump(conn, &clip))
            .unwrap()
        {
            InsertOutcome::Inserted(id) | InsertOutcome::Bumped(id) => id,
        }
    }

    fn app_without_db() -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
        ));
        app
    }

    /// Collects the payloads of `clips://changed` the app emits.
    fn clips_changed_payloads(
        app: &tauri::App<tauri::test::MockRuntime>,
    ) -> std::sync::mpsc::Receiver<String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        app.listen_any(CLIPS_CHANGED, move |event| {
            sender.send(event.payload().to_owned()).unwrap();
        });
        receiver
    }

    #[test]
    fn list_clips_returns_the_page_the_query_asks_for() {
        let dir = tempfile::tempdir().unwrap();
        let (app, db) = app_with_db(&dir);
        add(&db, "SELECT * FROM users", 1_000);
        add(&db, "vessel", 2_000);
        let selected = add(&db, "Selected items", 3_000);

        let page = list_clips(
            app.state(),
            ClipQuery {
                search: Some("sel".to_owned()),
                ..ClipQuery::default()
            },
        )
        .unwrap();

        assert_eq!(page.total, 2);
        assert_eq!(
            page.items.first().map(|clip| clip.id.clone()),
            Some(selected)
        );
    }

    #[test]
    fn list_clips_maps_an_invalid_cursor_to_its_code() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _db) = app_with_db(&dir);

        let error = list_clips(
            app.state(),
            ClipQuery {
                cursor: Some("nonsense".to_owned()),
                ..ClipQuery::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "invalid_cursor");
    }

    #[test]
    fn the_history_commands_fail_without_a_database() {
        let app = app_without_db();
        let handle = || app.handle().clone();

        let codes = [
            list_clips(app.state(), ClipQuery::default())
                .unwrap_err()
                .code,
            delete_clips(handle(), app.state(), vec!["a".to_owned()])
                .unwrap_err()
                .code,
            clear_history(handle(), app.state(), true).unwrap_err().code,
            get_app_icon(app.state(), "com.example.Editor".to_owned())
                .map(|_| ())
                .unwrap_err()
                .code,
        ];

        assert_eq!(codes, ["storage_unavailable"; 4]);
    }

    #[test]
    fn delete_clips_removes_the_clips_and_emits_their_ids() {
        let dir = tempfile::tempdir().unwrap();
        let (app, db) = app_with_db(&dir);
        let a = add(&db, "first", 1_000);
        let b = add(&db, "second", 2_000);
        let receiver = clips_changed_payloads(&app);

        delete_clips(
            app.handle().clone(),
            app.state(),
            vec![a.clone(), b.clone()],
        )
        .unwrap();

        assert_eq!(db.with_conn(clips::count).unwrap(), 0);
        let payload = receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).unwrap(),
            serde_json::json!({ "added": null, "updated": null, "removed": [a, b] })
        );
    }

    #[test]
    fn clear_history_returns_how_many_went_and_emits_a_reload() {
        let dir = tempfile::tempdir().unwrap();
        let (app, db) = app_with_db(&dir);
        let pinned = add(&db, "pinned", 1_000);
        add(&db, "other", 2_000);
        db.with_conn(|conn| conn.execute("UPDATE clips SET pinned = 1 WHERE id = ?1", [&pinned]))
            .unwrap();
        let receiver = clips_changed_payloads(&app);

        let removed = clear_history(app.handle().clone(), app.state(), true).unwrap();

        assert_eq!(removed, 1);
        assert_eq!(db.with_conn(clips::count).unwrap(), 1);
        let payload = receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).unwrap(),
            serde_json::json!({ "added": null, "updated": null, "removed": null })
        );
    }

    #[test]
    fn get_app_icon_returns_the_cached_png_as_raw_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let (app, db) = app_with_db(&dir);
        let png = vec![0x89, b'P', b'N', b'G', 0, 1, 2];
        db.with_conn(|conn| storage::app_icons::put(conn, "com.example.Editor", &png))
            .unwrap();

        let response = get_app_icon(app.state(), "com.example.Editor".to_owned()).unwrap();

        match response.body().unwrap() {
            InvokeResponseBody::Raw(bytes) => assert_eq!(bytes, png),
            InvokeResponseBody::Json(_) => panic!("the icon must travel as raw bytes"),
        }
    }

    #[test]
    fn get_app_icon_without_a_cached_icon_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let (app, _db) = app_with_db(&dir);

        let error = get_app_icon(app.state(), "com.example.Unknown".to_owned())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(error.code, "not_found");
    }

    // --- paste and copy ------------------------------------------------------------------

    struct PasteApp {
        app: tauri::App<tauri::test::MockRuntime>,
        db: Db,
        pasteboard: Arc<FakePasteboard>,
        keys: Arc<FakeKeySender>,
        accessibility: Arc<FakeAccessibility>,
        clip_id: String,
        _dir: tempfile::TempDir,
    }

    fn paste_app() -> PasteApp {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let pasteboard = Arc::new(FakePasteboard::new());
        let keys = Arc::new(FakeKeySender::new());
        let accessibility = Arc::new(FakeAccessibility::new());
        accessibility.set_trusted(true);
        let app = tauri::test::mock_app();
        app.manage(
            AppState::new(
                accessibility.clone(),
                keys.clone(),
                pasteboard.clone(),
                Arc::new(FakeWorkspace::new()),
            )
            .with_db(Some(db.clone())),
        );
        let clip_id = add(&db, "pasted text", 1_000);
        PasteApp {
            app,
            db,
            pasteboard,
            keys,
            accessibility,
            clip_id,
            _dir: dir,
        }
    }

    fn window(
        app: &tauri::App<tauri::test::MockRuntime>,
        label: &str,
    ) -> tauri::WebviewWindow<tauri::test::MockRuntime> {
        tauri::WebviewWindowBuilder::new(app, label, Default::default())
            .build()
            .unwrap()
    }

    #[test]
    fn paste_clip_writes_the_clip_with_the_marker_and_sends_command_v() {
        let paste = paste_app();

        // The Quick Panel is an `NSPanel` the mock runtime cannot hide: its paste is manual QA.
        let result = paste_clip(
            window(&paste.app, "popover"),
            paste.app.state(),
            paste.clip_id.clone(),
            PasteMode::Default,
        )
        .unwrap();

        assert_eq!(result, PasteResult::Pasted);
        assert_eq!(
            paste.pasteboard.writes(),
            vec![vec![
                Representation::new(UTI_PLAIN_TEXT, b"pasted text".as_slice()),
                Representation::new(UTI_TADIDY_MARKER, Vec::new()),
            ]]
        );
        assert_eq!(paste.keys.paste_count(), 1);
    }

    #[test]
    fn paste_clip_without_accessibility_only_copies() {
        let paste = paste_app();
        paste.accessibility.set_trusted(false);

        let result = paste_clip(
            window(&paste.app, "popover"),
            paste.app.state(),
            paste.clip_id.clone(),
            PasteMode::Plain,
        )
        .unwrap();

        assert_eq!(result, PasteResult::CopiedOnly);
        assert_eq!(paste.pasteboard.writes().len(), 1);
        assert_eq!(paste.keys.paste_count(), 0);
    }

    #[test]
    fn copy_clip_writes_the_clip_and_sends_no_key() {
        let paste = paste_app();

        copy_clip(
            window(&paste.app, "popover"),
            paste.app.state(),
            paste.clip_id.clone(),
        )
        .unwrap();

        assert_eq!(paste.pasteboard.writes().len(), 1);
        assert_eq!(paste.keys.paste_count(), 0);
        let used = paste
            .db
            .with_conn(|conn| clips::get(conn, &paste.clip_id))
            .unwrap()
            .unwrap()
            .clip
            .last_used_at;
        assert!(used.is_some());
    }

    #[test]
    fn paste_and_copy_refuse_a_window_that_is_not_the_popover_or_the_panel() {
        let paste = paste_app();
        let settings = window(&paste.app, "settings");

        let pasted = paste_clip(
            settings.clone(),
            paste.app.state(),
            paste.clip_id.clone(),
            PasteMode::Default,
        )
        .unwrap_err();
        let copied = copy_clip(settings, paste.app.state(), paste.clip_id.clone()).unwrap_err();

        assert_eq!(pasted.code, "invalid_window");
        assert_eq!(copied.code, "invalid_window");
        assert!(paste.pasteboard.writes().is_empty());
    }

    #[test]
    fn paste_and_copy_of_an_unknown_id_fail_with_not_found_and_touch_nothing() {
        let paste = paste_app();

        let pasted = paste_clip(
            window(&paste.app, "panel"),
            paste.app.state(),
            "no-such-id".to_owned(),
            PasteMode::Default,
        )
        .unwrap_err();
        let copied = copy_clip(
            window(&paste.app, "popover"),
            paste.app.state(),
            "no-such-id".to_owned(),
        )
        .unwrap_err();

        assert_eq!(pasted.code, "not_found");
        assert_eq!(copied.code, "not_found");
        assert!(paste.pasteboard.writes().is_empty());
        assert_eq!(paste.keys.paste_count(), 0);
    }

    #[test]
    fn paste_and_copy_without_a_database_fail_with_storage_unavailable() {
        let app = app_without_db();

        let pasted = paste_clip(
            window(&app, "panel"),
            app.state(),
            "a".to_owned(),
            PasteMode::Default,
        )
        .unwrap_err();
        let copied = copy_clip(window(&app, "popover"), app.state(), "a".to_owned()).unwrap_err();

        assert_eq!(pasted.code, "storage_unavailable");
        assert_eq!(copied.code, "storage_unavailable");
    }
}
