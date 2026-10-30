//! State managed by Tauri and shared by the commands. Later tickets add fields.

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::clip::HistoryLimit;
use crate::clock::SystemClock;
use crate::error::AppError;
use crate::events::EventSink;
use crate::login_item::{self, LoginItem, NoLoginItem};
use crate::paster::{AccessibilityMonitor, PASTE_DELAY, Paster, PasterServices, WindowHider};
use crate::platform::{AccessibilityProbe, KeySender, Pasteboard, SourceApp, Workspace};
use crate::settings::Settings;
use crate::shortcuts::{self, NoShortcuts, ShortcutRegistrar, ShortcutStatus};
use crate::storage::{self, Db};
use crate::watcher::WatcherHandle;

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
    /// `None` when the database could not be opened (e.g. its key is missing from the Keychain).
    pub db: Option<Db>,
    /// How many clips the history keeps (HIS-02), shared with the settings and the purger.
    pub history_limit: HistoryLimit,
    /// The user's preferences, loaded from the database at startup.
    pub settings: RwLock<Settings>,
    /// Serializes `set_settings` without holding `settings` across the system effects.
    pub settings_gate: Mutex<()>,
    /// The clipboard watcher thread, `None` until it starts (and when there is no database).
    pub watcher: Mutex<Option<WatcherHandle>>,
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
            db: None,
            history_limit: HistoryLimit::default(),
            settings: RwLock::default(),
            settings_gate: Mutex::default(),
            watcher: Mutex::default(),
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
        let settings = db.as_ref().map_or_else(Settings::default, |db| {
            db.with_conn(storage::settings::load)
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        code = %AppError::from(error).code,
                        "the settings could not be read, using the defaults"
                    );
                    Settings::default()
                })
        });
        self.history_limit.set(settings.history_limit());
        Self {
            db,
            settings: RwLock::new(settings),
            ..self
        }
    }

    /// Brings the system in line with the stored settings at startup: the login item is read back
    /// (the system wins) and the stored Quick Panel shortcut is registered, not the default.
    pub fn apply_stored_settings(&self) {
        if let Some(db) = &self.db {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::FakeKeyStore;
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
        assert!(state().db.is_none());
    }

    #[test]
    fn the_state_holds_the_opened_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        assert!(state().with_db(Some(db)).db.is_some());
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
}
