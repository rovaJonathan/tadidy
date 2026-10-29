//! State managed by Tauri and shared by the commands. Later tickets add fields
//! (settings, last active app).

use std::sync::{Arc, Mutex, PoisonError};

use crate::platform::{AccessibilityProbe, KeySender};
use crate::shortcuts::ShortcutStatus;
use crate::storage::Db;

/// The platform services as trait objects: the real ones in `setup`, fakes in the tests.
pub struct AppState {
    pub accessibility: Arc<dyn AccessibilityProbe>,
    /// Used by the paster (TAD-015).
    pub key_sender: Arc<dyn KeySender>,
    /// Result of the Quick Panel shortcut registration, read by the preferences (TAD-017).
    pub shortcut: Mutex<ShortcutStatus>,
    /// `None` when the database could not be opened (e.g. its key is missing from the Keychain).
    pub db: Option<Db>,
}

impl AppState {
    pub fn new(accessibility: Arc<dyn AccessibilityProbe>, key_sender: Arc<dyn KeySender>) -> Self {
        Self {
            accessibility,
            key_sender,
            shortcut: Mutex::default(),
            db: None,
        }
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
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender};
    use crate::shortcuts::DEFAULT_QUICK_PANEL_SHORTCUT;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::FakeKeyStore;
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender};
    use crate::storage::open_in_dir;

    fn state() -> AppState {
        AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
        )
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
}
