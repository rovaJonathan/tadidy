//! The global shortcut that opens the Quick Panel (QPN-01). Registration goes through the
//! Rust side of `tauri-plugin-global-shortcut` only: the webview never registers a shortcut.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::Shortcut;
use ts_rs::TS;

pub const DEFAULT_QUICK_PANEL_SHORTCUT: &str = "Cmd+Shift+V";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShortcutError {
    /// The accelerator string does not describe a key combination.
    #[error("invalid accelerator: {0}")]
    Invalid(String),
    /// The system refused the registration, typically because another app owns the combination.
    #[error("shortcut unavailable: {0}")]
    Unavailable(String),
}

impl ShortcutError {
    /// Stable code kept in `ShortcutStatus` and logged: never the accelerator text.
    pub fn code(&self) -> &'static str {
        "unknown"
    }
}

/// The state of the Quick Panel shortcut, kept in `AppState` for the preferences (TAD-017).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShortcutStatus {
    pub accelerator: String,
    /// Code of the registration error, `None` when the shortcut is registered.
    pub error: Option<String>,
}

impl Default for ShortcutStatus {
    fn default() -> Self {
        Self {
            accelerator: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
            error: None,
        }
    }
}

pub fn parse(_accelerator: &str) -> Result<Shortcut, ShortcutError> {
    Err(ShortcutError::Unavailable(String::new()))
}

/// What registering needs from the plugin, so the startup rules run against a fake.
pub trait ShortcutRegistrar {
    fn register(&self, accelerator: &str) -> Result<(), ShortcutError>;
}

/// Registers the default shortcut at startup. A failure does not stop the app: it is logged
/// (code only) and kept in `status` for the preferences.
pub fn register_at_startup(_registrar: &impl ShortcutRegistrar, _status: &Mutex<ShortcutStatus>) {}

pub fn register_quick_panel<R: Runtime>(
    _app: &AppHandle<R>,
    accelerator: &str,
) -> Result<(), ShortcutError> {
    parse(accelerator).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_parses() {
        assert!(parse(DEFAULT_QUICK_PANEL_SHORTCUT).is_ok());
    }

    #[test]
    fn invalid_accelerator_is_a_typed_error() {
        let app = tauri::test::mock_app();

        let result = register_quick_panel(app.handle(), "Cmd+Shift+");

        assert!(
            matches!(result, Err(ShortcutError::Invalid(_))),
            "{result:?}"
        );
    }

    struct FakeRegistrar(Result<(), ShortcutError>);

    impl ShortcutRegistrar for FakeRegistrar {
        fn register(&self, _accelerator: &str) -> Result<(), ShortcutError> {
            self.0.clone()
        }
    }

    #[test]
    fn startup_failure_is_recorded_in_status() {
        let status = Mutex::new(ShortcutStatus::default());
        let taken = FakeRegistrar(Err(ShortcutError::Unavailable("taken".to_owned())));

        register_at_startup(&taken, &status);

        let status = status.lock().unwrap();
        assert_eq!(status.accelerator, DEFAULT_QUICK_PANEL_SHORTCUT);
        assert_eq!(status.error.as_deref(), Some("unavailable"));
    }

    #[test]
    fn startup_success_leaves_no_error() {
        let status = Mutex::new(ShortcutStatus::default());

        register_at_startup(&FakeRegistrar(Ok(())), &status);

        assert_eq!(status.lock().unwrap().error, None);
    }

    #[test]
    fn an_error_has_a_stable_code() {
        assert_eq!(ShortcutError::Invalid("x".into()).code(), "invalid");
        assert_eq!(ShortcutError::Unavailable("x".into()).code(), "unavailable");
    }
}
