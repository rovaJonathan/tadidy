//! State managed by Tauri and shared by the commands. Later tickets add fields.

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::clip::HistoryLimit;
use crate::clock::SystemClock;
use crate::crypto::KeyStore;
use crate::error::AppError;
use crate::events::{CLIPS_CHANGED, ClipsChanged, EventSink, SETTINGS_CHANGED, STORAGE_CHANGED};
use crate::login_item::{self, LoginItem, NoLoginItem};
use crate::paster::{AccessibilityMonitor, PASTE_DELAY, Paster, PasterServices, WindowHider};
use crate::platform::{AccessibilityProbe, KeySender, Pasteboard, SourceApp, Workspace};
use crate::settings::Settings;
use crate::shortcuts::{self, NoShortcuts, ShortcutRegistrar, ShortcutStatus};
use crate::storage::status::StorageStatus;
use crate::storage::{self, Db};
use crate::watcher::{self, WatcherHandle};
use serde_json::json;
use std::path::PathBuf;

/// Tadidy's own bundle identifier (AGENTS.md §3).
pub const OWN_BUNDLE_ID: &str = "app.tadidy";

/// The platform services as trait objects: the real ones in `setup`, fakes in the tests.
pub struct AppState {
    pub accessibility: Arc<dyn AccessibilityProbe>,
    /// Remembers the permission's last known state and announces its changes.
    pub accessibility_monitor: Arc<AccessibilityMonitor>,
    pub key_sender: Arc<dyn KeySender>,
    pub pasteboard: Arc<dyn Pasteboard>,
    pub workspace: Arc<dyn Workspace>,
    /// The app that was in front just before the popover opened: the paster gives it the focus
    /// back before sending `⌘V`.
    pub last_active_app: Arc<Mutex<Option<SourceApp>>>,
    /// Result of the Quick Panel shortcut registration, read by the preferences (TAD-017).
    pub shortcut: Mutex<ShortcutStatus>,
    /// Registers and releases the Quick Panel shortcut.
    pub shortcuts: Arc<dyn ShortcutRegistrar>,
    /// The system's "open at login" entry for the app (SYS-01).
    pub login_item: Arc<dyn LoginItem>,
    /// `None` when the database could not be opened (e.g. its key is missing from the Keychain);
    /// the history reset sets it later. Read it with [`AppState::db`].
    db: RwLock<Option<Db>>,
    /// How many clips the history keeps (HIS-02), shared with the settings and the purger.
    pub history_limit: HistoryLimit,
    /// The user's preferences, loaded from the database at startup.
    pub settings: RwLock<Settings>,
    /// Serializes `set_settings` without holding `settings` across the system effects.
    pub settings_gate: Mutex<()>,
    /// The clipboard watcher thread, `None` until it starts (and when there is no database).
    pub watcher: Mutex<Option<WatcherHandle>>,
    /// The code of the failure that kept the database from opening, last seen. The status the
    /// webview gets is derived from it and from `db`, so the two cannot disagree.
    storage_failure: RwLock<Option<String>>,
    /// What the history reset needs to start over; `None` where it cannot run.
    storage_reset: Option<StorageReset>,
    /// Serializes the history reset: two windows can ask at once.
    reset_gate: Mutex<()>,
}

/// Where the database lives and what restarts around a new one.
struct StorageReset {
    dir: PathBuf,
    store: Arc<dyn KeyStore>,
    watcher: watcher::Services,
}

impl AppState {
    pub fn new(
        accessibility: Arc<dyn AccessibilityProbe>,
        key_sender: Arc<dyn KeySender>,
        pasteboard: Arc<dyn Pasteboard>,
        workspace: Arc<dyn Workspace>,
    ) -> Self {
        Self {
            accessibility_monitor: Arc::new(AccessibilityMonitor::new(accessibility.clone())),
            accessibility,
            key_sender,
            pasteboard,
            workspace,
            last_active_app: Arc::default(),
            shortcut: Mutex::default(),
            shortcuts: Arc::new(NoShortcuts),
            login_item: Arc::new(NoLoginItem),
            db: RwLock::default(),
            history_limit: HistoryLimit::default(),
            settings: RwLock::default(),
            settings_gate: Mutex::default(),
            watcher: Mutex::default(),
            storage_failure: RwLock::default(),
            storage_reset: None,
            reset_gate: Mutex::default(),
        }
    }

    /// Replaces the stand-in that refuses every shortcut with the real registrar (or a fake).
    pub fn with_shortcuts(self, shortcuts: Arc<dyn ShortcutRegistrar>) -> Self {
        Self { shortcuts, ..self }
    }

    /// Replaces the stand-in that fails every call with the real login item (or a fake).
    pub fn with_login_item(self, login_item: Arc<dyn LoginItem>) -> Self {
        Self { login_item, ..self }
    }

    /// Attaches the database and loads the stored settings from it, so the history limit is
    /// right before the watcher's first tick. Without a database the defaults stay.
    pub fn with_db(self, db: Option<Db>) -> Self {
        let settings = db
            .as_ref()
            .map_or_else(Settings::default, load_settings_or_defaults);
        self.history_limit.set(settings.history_limit());
        Self {
            db: RwLock::new(db),
            settings: RwLock::new(settings),
            ..self
        }
    }

    /// The database that opened at startup, or the error that kept it from opening.
    pub fn with_storage(self, opened: Result<Db, AppError>) -> Self {
        if let Err(error) = &opened {
            self.remember_failure(&error.code);
        }
        self.with_db(opened.ok())
    }

    /// Lets [`AppState::reset_history`] move the database in `dir` aside and start over.
    pub fn with_storage_reset(
        self,
        dir: PathBuf,
        store: Arc<dyn KeyStore>,
        watcher: watcher::Services,
    ) -> Self {
        Self {
            storage_reset: Some(StorageReset {
                dir,
                store,
                watcher,
            }),
            ..self
        }
    }

    /// Ready with a database; otherwise unavailable, with the code of the failure.
    pub fn storage_status(&self) -> StorageStatus {
        if self.db().is_some() {
            return StorageStatus::ready();
        }
        let failure = self
            .storage_failure
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        StorageStatus::unavailable(failure.unwrap_or_else(|| "storage_unavailable".to_owned()))
    }

    fn remember_failure(&self, code: &str) {
        *self
            .storage_failure
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(code.to_owned());
    }

    /// Starts over with an empty history when its key is gone or does not open it. The database
    /// is tried again first, because the startup failure may have passed (a Keychain item
    /// restored while the app ran): then the history is simply opened and nothing moves.
    /// Refused with `invalid_state` when a database is open, or when it fails to open for a
    /// reason that may pass (a locked Keychain): the history may be intact.
    pub fn reset_history(&self) -> Result<(), AppError> {
        let _gate = self
            .reset_gate
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if self.db().is_some() {
            return Err(AppError::new(
                "invalid_state",
                "The history database works: it cannot be reset.",
            ));
        }
        let Some(reset) = &self.storage_reset else {
            return Err(AppError::new(
                "invalid_state",
                "The history database cannot be reset here.",
            ));
        };
        let db = match storage::open_in_dir(&reset.dir, reset.store.as_ref()) {
            Ok(db) => db,
            Err(error) => {
                // One rule for the button and for the core: `StorageStatus::can_reset`.
                let failure = AppError::from(error);
                if !StorageStatus::unavailable(failure.code.as_str()).can_reset {
                    self.remember_failure(&failure.code);
                    reset
                        .watcher
                        .events
                        .emit(STORAGE_CHANGED, json!(self.storage_status()));
                    return Err(AppError::new(
                        "invalid_state",
                        "The history database may be only out of reach for now: it cannot be reset.",
                    ));
                }
                storage::reset::reset(&reset.dir, reset.store.as_ref())?
            }
        };
        {
            // A `set_settings` call must not interleave with the new settings, the login item
            // and the shortcut being brought in line.
            let _settings = self
                .settings_gate
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            self.attach(db.clone());
            login_item::reconcile(self.login_item.as_ref(), &self.settings, &db);
            self.follow_stored_shortcut();
        }
        watcher::start(self, reset.watcher.clone());
        let events = &reset.watcher.events;
        let settings = self
            .settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        events.emit(SETTINGS_CHANGED, json!(settings));
        events.emit(STORAGE_CHANGED, json!(self.storage_status()));
        events.emit(CLIPS_CHANGED, json!(ClipsChanged::default()));
        Ok(())
    }

    /// Startup registered the default Quick Panel shortcut, having no database to read the stored
    /// one from. A reopened history may hold another: it replaces the default. A fresh database
    /// holds the default, so nothing is registered twice.
    fn follow_stored_shortcut(&self) {
        let wanted = self
            .settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .quick_panel_shortcut
            .clone();
        if self.shortcut_status().accelerator == wanted {
            return;
        }
        // As at startup: the default one is released, then the stored one registered; when another
        // app holds it, nothing is registered and the status says so, which is what is true.
        let current = self.shortcut_status();
        if current.error.is_none()
            && let Err(error) = self.shortcuts.unregister(&current.accelerator)
        {
            tracing::warn!(
                code = error.code(),
                "the default Quick Panel shortcut was not released"
            );
        }
        shortcuts::register_at_startup(self.shortcuts.as_ref(), &self.shortcut, &wanted);
    }

    /// Makes `db` the history database and loads its settings.
    fn attach(&self, db: Db) {
        let settings = load_settings_or_defaults(&db);
        self.history_limit.set(settings.history_limit());
        *self
            .settings
            .write()
            .unwrap_or_else(PoisonError::into_inner) = settings;
        *self.db.write().unwrap_or_else(PoisonError::into_inner) = Some(db);
    }

    /// The history database, `None` while it is unavailable. A clone: the handle is shared.
    pub fn db(&self) -> Option<Db> {
        self.db
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Brings the system in line with the stored settings at startup: the login item is read back
    /// (the system wins) and the stored Quick Panel shortcut is registered, not the default.
    pub fn apply_stored_settings(&self) {
        if let Some(db) = &self.db() {
            login_item::reconcile(self.login_item.as_ref(), &self.settings, db);
        }
        let accelerator = self
            .settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .quick_panel_shortcut
            .clone();
        shortcuts::register_at_startup(self.shortcuts.as_ref(), &self.shortcut, &accelerator);
    }

    /// Remembers which app is in front. Called just before the popover is shown, while that app
    /// still has the focus. When Tadidy itself is in front (a Preferences window, say) the
    /// previous app is kept: re-activating Tadidy would send `⌘V` to its own webview.
    pub fn remember_frontmost_app(&self) {
        let frontmost = self.workspace.frontmost_app();
        if frontmost
            .as_ref()
            .is_some_and(|app| app.bundle_id == OWN_BUNDLE_ID)
        {
            return;
        }
        *self
            .last_active_app
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = frontmost;
    }

    /// The paster over this state's services, writing to `db` and reaching the windows through
    /// `hider`.
    pub fn paster(
        &self,
        db: Db,
        events: Arc<dyn EventSink>,
        hider: Arc<dyn WindowHider>,
    ) -> Paster {
        Paster::new(
            PasterServices {
                pasteboard: self.pasteboard.clone(),
                keys: self.key_sender.clone(),
                workspace: self.workspace.clone(),
                monitor: self.accessibility_monitor.clone(),
                events,
                hider,
                clock: Arc::new(SystemClock),
                last_active_app: self.last_active_app.clone(),
                db,
            },
            PASTE_DELAY,
        )
    }

    /// A copy of the shortcut's registration state.
    pub fn shortcut_status(&self) -> ShortcutStatus {
        self.shortcut
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The settings stored in `db`; the defaults when they cannot be read.
fn load_settings_or_defaults(db: &Db) -> Settings {
    db.with_conn(storage::settings::load)
        .unwrap_or_else(|error| {
            tracing::warn!(
                code = %AppError::from(error).code,
                "the settings could not be read, using the defaults"
            );
            Settings::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{DbKey, FakeKeyStore};
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender, FakePasteboard, FakeWorkspace};
    use crate::shortcuts::DEFAULT_QUICK_PANEL_SHORTCUT;
    use crate::storage::open_in_dir;
    use std::sync::PoisonError;

    fn state() -> AppState {
        AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
        )
    }

    fn textedit() -> SourceApp {
        SourceApp::new("com.apple.TextEdit", "TextEdit", 4242)
    }

    fn remembered(state: &AppState) -> Option<SourceApp> {
        state
            .last_active_app
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[test]
    fn no_app_is_remembered_at_first() {
        assert_eq!(remembered(&state()), None);
    }

    #[test]
    fn the_frontmost_app_is_remembered_for_the_paster() {
        let workspace = Arc::new(FakeWorkspace::new());
        workspace.set_frontmost(Some(textedit()));
        let state = AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            workspace,
        );

        state.remember_frontmost_app();

        assert_eq!(remembered(&state), Some(textedit()));
    }

    #[test]
    fn tadidy_in_front_does_not_replace_the_remembered_app() {
        // A Tadidy window in front (Preferences, say) when the popover opens: pasting would
        // re-activate Tadidy itself and send `⌘V` to its webview.
        let workspace = Arc::new(FakeWorkspace::new());
        workspace.set_frontmost(Some(textedit()));
        let state = AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            workspace.clone(),
        );
        state.remember_frontmost_app();

        workspace.set_frontmost(Some(SourceApp::new(OWN_BUNDLE_ID, "Tadidy", 99)));
        state.remember_frontmost_app();

        assert_eq!(remembered(&state), Some(textedit()));
    }

    #[test]
    fn an_empty_desktop_makes_the_state_forget_the_previous_app() {
        let workspace = Arc::new(FakeWorkspace::new());
        workspace.set_frontmost(Some(textedit()));
        let state = AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            workspace.clone(),
        );
        state.remember_frontmost_app();

        workspace.set_frontmost(None);
        state.remember_frontmost_app();

        assert_eq!(remembered(&state), None);
    }

    #[test]
    fn the_shortcut_starts_as_the_default_without_error() {
        assert_eq!(
            state().shortcut_status(),
            ShortcutStatus {
                accelerator: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
                error: None,
            }
        );
    }

    #[test]
    fn the_shortcut_status_reflects_a_failed_registration() {
        let state = state();
        state
            .shortcut
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .error = Some("unavailable".to_owned());

        assert_eq!(
            state.shortcut_status().error.as_deref(),
            Some("unavailable")
        );
    }

    #[test]
    fn the_state_starts_without_a_database() {
        assert!(state().db().is_none());
    }

    #[test]
    fn the_state_holds_the_opened_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        assert!(state().with_db(Some(db)).db().is_some());
    }

    #[test]
    fn the_history_limit_starts_at_the_default_size() {
        assert_eq!(state().history_limit.get(), Some(500));
    }

    #[test]
    fn the_settings_start_as_the_defaults() {
        let settings = state().settings.read().unwrap().clone();

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn the_stored_settings_are_loaded_with_the_database_and_set_the_history_limit() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let stored = Settings {
            history_size: 10,
            launch_at_login: true,
            ..Settings::default()
        };
        db.with_conn(|conn| crate::storage::settings::save(conn, &stored))
            .unwrap();

        let state = state().with_db(Some(db));

        assert_eq!(*state.settings.read().unwrap(), stored);
        assert_eq!(state.history_limit.get(), Some(10));
    }

    #[test]
    fn an_unlimited_stored_history_size_lifts_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let stored = Settings {
            history_size: 0,
            ..Settings::default()
        };
        db.with_conn(|conn| crate::storage::settings::save(conn, &stored))
            .unwrap();

        let state = state().with_db(Some(db));

        assert_eq!(state.history_limit.get(), None);
    }
    use crate::login_item::FakeLoginItem;
    use crate::shortcuts::FakeShortcutRegistrar;

    struct Wired {
        state: AppState,
        login_item: Arc<FakeLoginItem>,
        shortcuts: Arc<FakeShortcutRegistrar>,
        _dir: tempfile::TempDir,
    }

    /// A state over a database holding `stored`, with a login item that reads `system`.
    fn wired(stored: Settings, system: bool) -> Wired {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        db.with_conn(|conn| crate::storage::settings::save(conn, &stored))
            .unwrap();
        let login_item = Arc::new(FakeLoginItem::new(system));
        let shortcuts = Arc::new(FakeShortcutRegistrar::default());
        let state = state()
            .with_db(Some(db))
            .with_login_item(login_item.clone())
            .with_shortcuts(shortcuts.clone());
        Wired {
            state,
            login_item,
            shortcuts,
            _dir: dir,
        }
    }

    #[test]
    fn startup_registers_the_stored_shortcut() {
        let wired = wired(
            Settings {
                quick_panel_shortcut: "Ctrl+Alt+V".to_owned(),
                ..Settings::default()
            },
            false,
        );

        wired.state.apply_stored_settings();

        assert!(wired.shortcuts.is_registered("Ctrl+Alt+V"));
        assert!(!wired.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(wired.state.shortcut_status().accelerator, "Ctrl+Alt+V");
        assert_eq!(wired.state.shortcut_status().error, None);
    }

    #[test]
    fn startup_records_a_stored_shortcut_the_system_refuses() {
        let wired = wired(Settings::default(), false);
        wired.shortcuts.take(DEFAULT_QUICK_PANEL_SHORTCUT);

        wired.state.apply_stored_settings();

        assert_eq!(
            wired.state.shortcut_status().error.as_deref(),
            Some("unavailable")
        );
    }

    #[test]
    fn startup_lets_the_system_win_over_the_stored_login_item() {
        let wired = wired(
            Settings {
                launch_at_login: true,
                ..Settings::default()
            },
            false,
        );

        wired.state.apply_stored_settings();

        assert!(!wired.state.settings.read().unwrap().launch_at_login);
        assert_eq!(wired.login_item.changes(), 0);
    }

    #[test]
    fn startup_without_a_database_still_registers_the_shortcut() {
        let shortcuts = Arc::new(FakeShortcutRegistrar::default());
        let state = state().with_shortcuts(shortcuts.clone());

        state.apply_stored_settings();

        assert!(shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
    }

    // --- history reset (TAD-022) ---

    use crate::clock::FixedClock;
    use crate::events::RecordingEventSink;
    use crate::platform::fakes::FakeIdle;
    use crate::storage::{DB_FILE_NAME, status::StorageStatus};

    /// A data folder holding a database whose key `FakeKeyStore::default()` does not have.
    fn orphaned_dir() -> (tempfile::TempDir, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let bytes = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();
        (dir, bytes)
    }

    fn watcher_services(events: &Arc<RecordingEventSink>) -> watcher::Services {
        watcher::Services {
            pasteboard: Arc::new(FakePasteboard::new()),
            workspace: Arc::new(FakeWorkspace::new()),
            idle: Arc::new(FakeIdle::new()),
            events: events.clone(),
            clock: Arc::new(FixedClock::new(1_000)),
        }
    }

    fn key_missing() -> AppError {
        AppError::new("storage_key_missing", "The history key is missing.")
    }

    /// A state whose database did not open, ready to reset in `dir`.
    fn unavailable_state(
        dir: &std::path::Path,
        store: FakeKeyStore,
        events: &Arc<RecordingEventSink>,
    ) -> AppState {
        state().with_storage(Err(key_missing())).with_storage_reset(
            dir.to_path_buf(),
            Arc::new(store),
            watcher_services(events),
        )
    }

    #[test]
    fn an_opened_database_makes_the_storage_ready() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        let state = state().with_storage(Ok(db));

        assert_eq!(state.storage_status(), StorageStatus::ready());
        assert!(state.db().is_some());
    }

    #[test]
    fn a_database_attached_by_itself_is_a_ready_storage() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        let state = state().with_db(Some(db));

        assert_eq!(state.storage_status(), StorageStatus::ready());
    }

    #[test]
    fn a_failed_open_makes_the_storage_unavailable_with_its_code() {
        let state = state().with_storage(Err(key_missing()));

        assert_eq!(
            state.storage_status(),
            StorageStatus::unavailable("storage_key_missing")
        );
        assert!(state.db().is_none());
    }

    #[test]
    fn the_reset_is_refused_while_the_database_works_and_moves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        let db = open_in_dir(dir.path(), &store).unwrap();
        let before = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();
        let events = Arc::new(RecordingEventSink::default());
        let state = state().with_storage(Ok(db)).with_storage_reset(
            dir.path().to_path_buf(),
            Arc::new(store),
            watcher_services(&events),
        );

        let error = state.reset_history().unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
        assert!(!dir.path().join("tadidy.db.bak").exists());
        assert!(events.events().is_empty());
    }

    #[test]
    fn the_reset_is_refused_when_the_database_fails_to_open_for_a_reason_that_may_pass() {
        // A Keychain that fails (locked, access denied) says nothing about the key: the
        // history may be intact.
        let (dir, before) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), FakeKeyStore::failing(), &events);

        let error = state.reset_history().unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
        assert!(!dir.path().join("tadidy.db.bak").exists());
        assert!(state.db().is_none());
        // The window learns it: the reset button goes away.
        assert_eq!(
            state.storage_status(),
            StorageStatus::unavailable("storage_error")
        );
        assert_eq!(
            events.events(),
            vec![(
                "storage://changed",
                json!({ "state": "unavailable", "code": "storage_error", "canReset": false })
            )]
        );
    }

    #[test]
    fn a_key_that_came_back_opens_the_history_instead_of_resetting_it() {
        // Startup saw no key; the Keychain item was restored while the app ran.
        let dir = tempfile::tempdir().unwrap();
        let key = DbKey::from_bytes([7; 32]);
        let store = FakeKeyStore::with_key(key.clone());
        let db = open_in_dir(dir.path(), &store).unwrap();
        db.with_conn(|conn| {
            conn.execute("CREATE TABLE marker (n INTEGER)", []).unwrap();
        });
        drop(db);
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), store, &events);

        state.reset_history().unwrap();

        assert!(!dir.path().join("tadidy.db.bak").exists());
        assert!(state.watcher.lock().unwrap().is_some());
        let state_db = state.db().unwrap();
        assert!(
            state_db
                .with_conn(|conn| conn
                    .query_row("SELECT count(*) FROM marker", [], |r| r.get::<_, i64>(0)))
                .is_ok()
        );
        assert_eq!(state.storage_status(), StorageStatus::ready());
        assert!(
            events
                .events()
                .iter()
                .any(|(name, _)| *name == "storage://changed")
        );
    }

    #[test]
    fn the_reset_is_allowed_when_the_key_does_not_open_the_database() {
        let (dir, _) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = state()
            .with_storage(Err(AppError::new("storage_wrong_key", "no match")))
            .with_storage_reset(
                dir.path().to_path_buf(),
                Arc::new(FakeKeyStore::default()),
                watcher_services(&events),
            );

        assert!(state.reset_history().is_ok());
    }

    #[test]
    fn the_reset_never_moves_a_database_that_is_open() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        let db = open_in_dir(dir.path(), &store).unwrap();
        let before = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();
        let events = Arc::new(RecordingEventSink::default());
        // A status that says "resettable" next to an open database: the open one wins.
        let state = state()
            .with_storage(Err(key_missing()))
            .with_db(Some(db))
            .with_storage_reset(
                dir.path().to_path_buf(),
                Arc::new(store),
                watcher_services(&events),
            );

        let error = state.reset_history().unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
        assert!(!dir.path().join("tadidy.db.bak").exists());
    }

    #[test]
    fn the_reset_is_refused_where_it_was_not_set_up() {
        let state = state().with_storage(Err(key_missing()));

        assert_eq!(state.reset_history().unwrap_err().code, "invalid_state");
    }

    #[test]
    fn a_reset_gives_the_state_a_new_database_and_starts_the_watcher() {
        let (dir, before) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), FakeKeyStore::default(), &events);

        state.reset_history().unwrap();

        assert_eq!(state.storage_status(), StorageStatus::ready());
        assert!(state.db().is_some());
        assert!(state.watcher.lock().unwrap().is_some());
        assert_eq!(
            std::fs::read(dir.path().join("tadidy.db.bak")).unwrap(),
            before
        );
    }

    #[test]
    fn a_reset_announces_the_new_status_and_a_reloaded_history() {
        let (dir, _) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), FakeKeyStore::default(), &events);

        state.reset_history().unwrap();

        assert_eq!(
            events.events(),
            vec![
                ("settings://changed", json!(Settings::default())),
                (
                    "storage://changed",
                    json!({ "state": "ready", "code": null, "canReset": false })
                ),
                (
                    CLIPS_CHANGED,
                    json!({ "added": null, "updated": null, "removed": null })
                ),
            ]
        );
    }

    #[test]
    fn a_reopened_history_gets_its_stored_quick_panel_shortcut_back() {
        // Startup had no database, so the default shortcut was registered; the key came back.
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        let db = open_in_dir(dir.path(), &store).unwrap();
        let stored = Settings {
            quick_panel_shortcut: "Ctrl+Alt+V".to_owned(),
            ..Settings::default()
        };
        db.with_conn(|conn| crate::storage::settings::save(conn, &stored))
            .unwrap();
        drop(db);
        let events = Arc::new(RecordingEventSink::default());
        let shortcuts = Arc::new(FakeShortcutRegistrar::default());
        let state = unavailable_state(dir.path(), store, &events).with_shortcuts(shortcuts.clone());
        state.apply_stored_settings();
        assert!(shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));

        state.reset_history().unwrap();

        assert!(shortcuts.is_registered("Ctrl+Alt+V"));
        assert!(!shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(state.shortcut_status().accelerator, "Ctrl+Alt+V");
        assert_eq!(state.shortcut_status().error, None);
    }

    #[test]
    fn a_stored_shortcut_that_another_app_holds_leaves_the_status_true_to_what_is_registered() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        let db = open_in_dir(dir.path(), &store).unwrap();
        let stored = Settings {
            quick_panel_shortcut: "Ctrl+Alt+V".to_owned(),
            ..Settings::default()
        };
        db.with_conn(|conn| crate::storage::settings::save(conn, &stored))
            .unwrap();
        drop(db);
        let events = Arc::new(RecordingEventSink::default());
        let shortcuts = Arc::new(FakeShortcutRegistrar::default());
        let state = unavailable_state(dir.path(), store, &events).with_shortcuts(shortcuts.clone());
        state.apply_stored_settings();
        shortcuts.take("Ctrl+Alt+V");

        state.reset_history().unwrap();

        // Neither shortcut is registered, and the status says exactly that: the stored one, failed.
        assert_eq!(shortcuts.registered_count(), 0);
        assert_eq!(state.shortcut_status().accelerator, "Ctrl+Alt+V");
        assert!(state.shortcut_status().error.is_some());
    }

    #[test]
    fn a_reset_waits_for_a_settings_change_in_progress_before_it_attaches_the_history() {
        let (dir, _) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), FakeKeyStore::default(), &events);

        std::thread::scope(|scope| {
            // A `set_settings` call holds the gate: the reset must not interleave with it.
            let change = state.settings_gate.lock().unwrap();
            let reset = scope.spawn(|| state.reset_history());
            std::thread::sleep(std::time::Duration::from_millis(200));
            assert!(state.db().is_none());
            drop(change);
            assert!(reset.join().unwrap().is_ok());
        });

        assert!(state.db().is_some());
    }

    #[test]
    fn a_reset_announces_the_settings_the_login_item_changed() {
        let (dir, _) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        // The system login item is on, the new database says off: the system wins.
        let state = unavailable_state(dir.path(), FakeKeyStore::default(), &events)
            .with_login_item(Arc::new(FakeLoginItem::new(true)));

        state.reset_history().unwrap();

        let settings = state.settings.read().unwrap().clone();
        assert!(settings.launch_at_login);
        assert_eq!(
            events.events().first(),
            Some(&("settings://changed", json!(settings)))
        );
    }

    #[test]
    fn a_failed_reset_leaves_the_storage_unavailable_and_the_files_in_place() {
        let (dir, before) = orphaned_dir();
        let events = Arc::new(RecordingEventSink::default());
        let state = unavailable_state(dir.path(), FakeKeyStore::refusing_replace(), &events);

        let error = state.reset_history().unwrap_err();

        assert_eq!(error.code, "storage_error");
        assert_eq!(
            state.storage_status(),
            StorageStatus::unavailable("storage_key_missing")
        );
        assert!(state.db().is_none());
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
        assert!(events.events().is_empty());
    }
}
