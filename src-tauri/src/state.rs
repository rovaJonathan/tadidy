//! State managed by Tauri and shared by the commands. Later tickets add fields
//! (settings, last active app).

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::clip::HistoryLimit;
use crate::platform::{AccessibilityProbe, KeySender};
use crate::settings::Settings;
use crate::shortcuts::ShortcutStatus;
use crate::storage::Db;
use crate::watcher::WatcherHandle;

/// The platform services as trait objects: the real ones in `setup`, fakes in the tests.
pub struct AppState {
    pub accessibility: Arc<dyn AccessibilityProbe>,
    /// Used by the paster (TAD-015).
    pub key_sender: Arc<dyn KeySender>,
    /// Result of the Quick Panel shortcut registration, read by the preferences (TAD-017).
    pub shortcut: Mutex<ShortcutStatus>,
    /// `None` when the database could not be opened (e.g. its key is missing from the Keychain).
    pub db: Option<Db>,
    /// How many clips the history keeps (HIS-02), shared with the settings and the purger.
    pub history_limit: HistoryLimit,
    /// The user's preferences, loaded from the database at startup.
    pub settings: RwLock<Settings>,
    /// The clipboard watcher thread, `None` until it starts (and when there is no database).
    pub watcher: Mutex<Option<WatcherHandle>>,
}

impl AppState {
    pub fn new(accessibility: Arc<dyn AccessibilityProbe>, key_sender: Arc<dyn KeySender>) -> Self {
        Self {
            accessibility,
            key_sender,
            shortcut: Mutex::default(),
            db: None,
            history_limit: HistoryLimit::default(),
            settings: RwLock::default(),
            watcher: Mutex::default(),
        }
    }

    pub fn with_db(self, db: Option<Db>) -> Self {
        Self { db, ..self }
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
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender};
    use crate::shortcuts::DEFAULT_QUICK_PANEL_SHORTCUT;
    use crate::storage::open_in_dir;
    use std::sync::PoisonError;

    fn state() -> AppState {
        AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
        )
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
}
