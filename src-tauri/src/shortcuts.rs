//! The global shortcut that opens the Quick Panel (QPN-01). Registration goes through the
//! Rust side of `tauri-plugin-global-shortcut` only: the webview never registers a shortcut.

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use ts_rs::TS;

use crate::error::AppError;
use crate::windows;

pub const DEFAULT_QUICK_PANEL_SHORTCUT: &str = "Cmd+Shift+V";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShortcutError {
    /// The accelerator string does not describe a key combination.
    #[error("invalid accelerator: {0}")]
    Invalid(String),
    /// The system refused the registration. macOS does not refuse a combination another app
    /// already holds (the plugin registers it non-exclusively), so this stays rare.
    #[error("shortcut unavailable: {0}")]
    Unavailable(String),
}

impl ShortcutError {
    /// Stable code kept in `ShortcutStatus` and logged: never the accelerator text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid",
            Self::Unavailable(_) => "unavailable",
        }
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

pub fn parse(accelerator: &str) -> Result<Shortcut, ShortcutError> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|error| ShortcutError::Invalid(error.to_string()))
}

/// What registering needs from the plugin, so the rules run against a fake.
pub trait ShortcutRegistrar: Send + Sync {
    fn register(&self, accelerator: &str) -> Result<(), ShortcutError>;
    fn unregister(&self, accelerator: &str) -> Result<(), ShortcutError>;
}

/// Stands in until the real registrar is wired: every call fails, so a state built without one
/// reports `shortcut_unavailable` instead of pretending it worked.
pub struct NoShortcuts;

impl ShortcutRegistrar for NoShortcuts {
    fn register(&self, accelerator: &str) -> Result<(), ShortcutError> {
        Err(ShortcutError::Unavailable(accelerator.to_owned()))
    }

    fn unregister(&self, accelerator: &str) -> Result<(), ShortcutError> {
        Err(ShortcutError::Unavailable(accelerator.to_owned()))
    }
}

/// An in-memory registrar for the tests of everything that registers a shortcut. `take` makes an
/// accelerator refuse to register, as one the system has reserved does.
#[derive(Default)]
pub struct FakeShortcutRegistrar {
    registered: Mutex<HashSet<String>>,
    taken: Mutex<HashSet<String>>,
}

impl FakeShortcutRegistrar {
    pub fn take(&self, accelerator: &str) {
        self.taken
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(accelerator.to_owned());
    }

    pub fn is_registered(&self, accelerator: &str) -> bool {
        self.registered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(accelerator)
    }

    pub fn registered_count(&self) -> usize {
        self.registered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl ShortcutRegistrar for FakeShortcutRegistrar {
    fn register(&self, accelerator: &str) -> Result<(), ShortcutError> {
        parse(accelerator)?;
        if self
            .taken
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(accelerator)
        {
            return Err(ShortcutError::Unavailable(accelerator.to_owned()));
        }
        self.registered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(accelerator.to_owned());
        Ok(())
    }

    fn unregister(&self, accelerator: &str) -> Result<(), ShortcutError> {
        let removed = self
            .registered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(accelerator);
        removed
            .then_some(())
            .ok_or_else(|| ShortcutError::Unavailable(accelerator.to_owned()))
    }
}

impl From<ShortcutError> for AppError {
    /// Fixed messages: the accelerator is the user's choice, not something to echo back.
    fn from(error: ShortcutError) -> Self {
        match error {
            ShortcutError::Invalid(_) => Self::new(
                "invalid_settings",
                "quickPanelShortcut is not a valid key combination",
            ),
            ShortcutError::Unavailable(_) => Self::new(
                "shortcut_unavailable",
                "This shortcut is already used by macOS or another app.",
            ),
        }
    }
}

/// Registers the stored shortcut at startup. A failure does not stop the app: it is logged
/// (code only) and kept in `status` for the preferences.
pub fn register_at_startup(
    registrar: &dyn ShortcutRegistrar,
    status: &Mutex<ShortcutStatus>,
    accelerator: &str,
) {
    let error = registrar.register(accelerator).err().map(|error| {
        tracing::warn!(
            code = error.code(),
            "the Quick Panel shortcut is not registered"
        );
        error.code().to_owned()
    });
    *status.lock().unwrap_or_else(PoisonError::into_inner) = ShortcutStatus {
        accelerator: accelerator.to_owned(),
        error,
    };
}

/// Moves the Quick Panel to `accelerator`. The new one is registered first: when that fails the
/// old one stays registered and `status` is untouched. Once it succeeds the old one is released.
pub fn switch(
    registrar: &dyn ShortcutRegistrar,
    status: &Mutex<ShortcutStatus>,
    accelerator: &str,
) -> Result<(), ShortcutError> {
    registrar.register(accelerator)?;
    let mut status = status.lock().unwrap_or_else(PoisonError::into_inner);
    // A shortcut that never registered has nothing to release.
    if status.error.is_none() {
        if let Err(error) = registrar.unregister(&status.accelerator) {
            tracing::warn!(
                code = error.code(),
                "the old Quick Panel shortcut was not released"
            );
        }
    }
    *status = ShortcutStatus {
        accelerator: accelerator.to_owned(),
        error: None,
    };
    Ok(())
}

/// Registers `accelerator` to toggle the Quick Panel.
pub fn register_quick_panel<R: Runtime>(
    app: &AppHandle<R>,
    accelerator: &str,
) -> Result<(), ShortcutError> {
    let shortcut = parse(accelerator)?;
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                windows::toggle_quick_panel(app);
            }
        })
        .map_err(|error| ShortcutError::Unavailable(error.to_string()))
}

/// The plugin-backed registrar, for the real app.
pub struct AppShortcuts<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> ShortcutRegistrar for AppShortcuts<R> {
    fn register(&self, accelerator: &str) -> Result<(), ShortcutError> {
        register_quick_panel(&self.0, accelerator)
    }

    fn unregister(&self, accelerator: &str) -> Result<(), ShortcutError> {
        let shortcut = parse(accelerator)?;
        self.0
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| ShortcutError::Unavailable(error.to_string()))
    }
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

    fn status_of(accelerator: &str, error: Option<&str>) -> Mutex<ShortcutStatus> {
        Mutex::new(ShortcutStatus {
            accelerator: accelerator.to_owned(),
            error: error.map(str::to_owned),
        })
    }

    fn read(status: &Mutex<ShortcutStatus>) -> ShortcutStatus {
        status.lock().unwrap().clone()
    }

    #[test]
    fn startup_registers_the_stored_shortcut_not_the_default() {
        let registrar = FakeShortcutRegistrar::default();
        let status = Mutex::new(ShortcutStatus::default());

        register_at_startup(&registrar, &status, "Ctrl+Alt+V");

        assert!(registrar.is_registered("Ctrl+Alt+V"));
        assert!(!registrar.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(
            read(&status),
            status_of("Ctrl+Alt+V", None).into_inner().unwrap()
        );
    }

    #[test]
    fn startup_failure_is_recorded_in_status() {
        let registrar = FakeShortcutRegistrar::default();
        registrar.take(DEFAULT_QUICK_PANEL_SHORTCUT);
        let status = Mutex::new(ShortcutStatus::default());

        register_at_startup(&registrar, &status, DEFAULT_QUICK_PANEL_SHORTCUT);

        let status = status.lock().unwrap();
        assert_eq!(status.accelerator, DEFAULT_QUICK_PANEL_SHORTCUT);
        assert_eq!(status.error.as_deref(), Some("unavailable"));
    }

    #[test]
    fn startup_success_leaves_no_error() {
        let status = Mutex::new(ShortcutStatus::default());

        register_at_startup(
            &FakeShortcutRegistrar::default(),
            &status,
            DEFAULT_QUICK_PANEL_SHORTCUT,
        );

        assert_eq!(status.lock().unwrap().error, None);
    }

    #[test]
    fn switching_registers_the_new_shortcut_and_releases_the_old_one() {
        let registrar = FakeShortcutRegistrar::default();
        let status = Mutex::new(ShortcutStatus::default());
        register_at_startup(&registrar, &status, DEFAULT_QUICK_PANEL_SHORTCUT);

        switch(&registrar, &status, "Ctrl+Alt+V").unwrap();

        assert!(registrar.is_registered("Ctrl+Alt+V"));
        assert!(!registrar.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(registrar.registered_count(), 1);
        assert_eq!(
            read(&status),
            status_of("Ctrl+Alt+V", None).into_inner().unwrap()
        );
    }

    #[test]
    fn a_refused_shortcut_keeps_the_old_one_and_the_status() {
        let registrar = FakeShortcutRegistrar::default();
        let status = Mutex::new(ShortcutStatus::default());
        register_at_startup(&registrar, &status, DEFAULT_QUICK_PANEL_SHORTCUT);
        registrar.take("Ctrl+Alt+V");

        let result = switch(&registrar, &status, "Ctrl+Alt+V");

        assert!(matches!(result, Err(ShortcutError::Unavailable(_))));
        assert!(registrar.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert!(!registrar.is_registered("Ctrl+Alt+V"));
        assert_eq!(read(&status), ShortcutStatus::default());
    }

    #[test]
    fn a_malformed_shortcut_is_refused_as_invalid_and_keeps_the_old_one() {
        let registrar = FakeShortcutRegistrar::default();
        let status = Mutex::new(ShortcutStatus::default());
        register_at_startup(&registrar, &status, DEFAULT_QUICK_PANEL_SHORTCUT);

        let result = switch(&registrar, &status, "Cmd+Shift+");

        assert!(
            matches!(result, Err(ShortcutError::Invalid(_))),
            "{result:?}"
        );
        assert!(registrar.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
    }

    #[test]
    fn switching_away_from_a_shortcut_that_never_registered_clears_the_error() {
        let registrar = FakeShortcutRegistrar::default();
        registrar.take(DEFAULT_QUICK_PANEL_SHORTCUT);
        let status = Mutex::new(ShortcutStatus::default());
        register_at_startup(&registrar, &status, DEFAULT_QUICK_PANEL_SHORTCUT);

        switch(&registrar, &status, "Ctrl+Alt+V").unwrap();

        assert!(registrar.is_registered("Ctrl+Alt+V"));
        assert_eq!(
            read(&status),
            status_of("Ctrl+Alt+V", None).into_inner().unwrap()
        );
    }

    #[test]
    fn a_shortcut_error_maps_to_a_code_without_the_accelerator() {
        let unavailable = AppError::from(ShortcutError::Unavailable("Cmd+Shift+V".into()));
        let invalid = AppError::from(ShortcutError::Invalid("Cmd+Shift+V".into()));

        assert_eq!(unavailable.code, "shortcut_unavailable");
        assert_eq!(invalid.code, "invalid_settings");
        assert!(
            !unavailable.message.contains("Cmd"),
            "{}",
            unavailable.message
        );
        assert!(!invalid.message.contains("Cmd"), "{}", invalid.message);
    }

    #[test]
    fn the_stand_in_registrar_always_refuses() {
        assert!(NoShortcuts.register(DEFAULT_QUICK_PANEL_SHORTCUT).is_err());
        assert!(
            NoShortcuts
                .unregister(DEFAULT_QUICK_PANEL_SHORTCUT)
                .is_err()
        );
    }

    #[test]
    fn an_error_has_a_stable_code() {
        assert_eq!(ShortcutError::Invalid("x".into()).code(), "invalid");
        assert_eq!(ShortcutError::Unavailable("x".into()).code(), "unavailable");
    }
}
