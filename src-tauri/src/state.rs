//! State managed by Tauri and shared by the commands. Later tickets add fields
//! (database, settings, last active app).

use std::sync::Arc;

use crate::platform::{AccessibilityProbe, KeySender};

/// The platform services as trait objects: the real ones in `setup`, fakes in the tests.
pub struct AppState {
    pub accessibility: Arc<dyn AccessibilityProbe>,
    /// Used by the paster (TAD-015).
    pub key_sender: Arc<dyn KeySender>,
}

impl AppState {
    pub fn new(accessibility: Arc<dyn AccessibilityProbe>, key_sender: Arc<dyn KeySender>) -> Self {
        Self {
            accessibility,
            key_sender,
        }
    }
}
