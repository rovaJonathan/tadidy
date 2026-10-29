//! State managed by Tauri and shared by the commands. Later tickets add fields
//! (database, settings, last active app).

use std::sync::{Arc, Mutex};

use crate::platform::{AccessibilityProbe, KeySender};
use crate::shortcuts::ShortcutStatus;

/// The platform services as trait objects: the real ones in `setup`, fakes in the tests.
pub struct AppState {
    pub accessibility: Arc<dyn AccessibilityProbe>,
    /// Used by the paster (TAD-015).
    pub key_sender: Arc<dyn KeySender>,
    /// Result of the Quick Panel shortcut registration, read by the preferences (TAD-017).
    pub shortcut: Mutex<ShortcutStatus>,
}

impl AppState {
    pub fn new(accessibility: Arc<dyn AccessibilityProbe>, key_sender: Arc<dyn KeySender>) -> Self {
        Self {
            accessibility,
            key_sender,
            shortcut: Mutex::default(),
        }
    }

    /// A copy of the shortcut's registration state.
    pub fn shortcut_status(&self) -> ShortcutStatus {
        ShortcutStatus {
            accelerator: String::new(),
            error: Some("stub".to_owned()),
        }
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
