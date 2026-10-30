//! The user's preferences: a typed, validated object shared by Rust and the front. Only the
//! stored settings live here; applying them (autostart, shortcut registration) is the job of
//! the preferences window's ticket.

use std::sync::{Mutex, PoisonError, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::clip::{DEFAULT_HISTORY_SIZE, HistoryLimit};
use crate::error::AppError;
use crate::events::{CLIPS_CHANGED, ClipsChanged, EventSink, SETTINGS_CHANGED};
use crate::login_item::LoginItem;
use crate::shortcuts::{self, DEFAULT_QUICK_PANEL_SHORTCUT, ShortcutRegistrar, ShortcutStatus};
use crate::storage::clips::enforce_limit;
use crate::storage::{self, Db};

/// The smallest history a user can pick (HIS-02); `0` is the separate "unlimited" value.
const MIN_HISTORY_SIZE: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Settings {
    /// How many unpinned clips the history keeps; `0` means unlimited, otherwise at least 10.
    pub history_size: u32,
    /// Ask before deleting a clip (HIS-05).
    pub confirm_before_delete: bool,
    /// Start Tadidy when the user logs in (SYS-01).
    pub launch_at_login: bool,
    /// Accelerator that opens the Quick Panel (QPN-01), e.g. `Cmd+Shift+V`.
    pub quick_panel_shortcut: String,
}

/// A partial update of the settings: only the fields that are set change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct SettingsPatch {
    pub history_size: Option<u32>,
    pub confirm_before_delete: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub quick_panel_shortcut: Option<String>,
}

/// Why a patch was refused. The variant names the field, never its value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("historySize must be 0 (unlimited) or at least 10")]
    HistorySize,
    #[error("quickPanelShortcut must not be empty")]
    QuickPanelShortcut,
}

impl From<SettingsError> for AppError {
    fn from(error: SettingsError) -> Self {
        Self::new("invalid_settings", error.to_string())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            history_size: DEFAULT_HISTORY_SIZE,
            confirm_before_delete: true,
            launch_at_login: false,
            quick_panel_shortcut: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
        }
    }
}

impl Settings {
    /// The history size as the purger reads it: `None` for "unlimited".
    pub fn history_limit(&self) -> Option<u32> {
        (self.history_size != 0).then_some(self.history_size)
    }

    /// The settings after `patch`, or the first rule it breaks; `self` is left untouched.
    pub fn apply(&self, patch: SettingsPatch) -> Result<Settings, SettingsError> {
        let mut next = self.clone();
        if let Some(size) = patch.history_size {
            if (1..MIN_HISTORY_SIZE).contains(&size) {
                return Err(SettingsError::HistorySize);
            }
            next.history_size = size;
        }
        if let Some(shortcut) = patch.quick_panel_shortcut {
            if shortcut.trim().is_empty() {
                return Err(SettingsError::QuickPanelShortcut);
            }
            next.quick_panel_shortcut = shortcut;
        }
        next.confirm_before_delete = patch
            .confirm_before_delete
            .unwrap_or(next.confirm_before_delete);
        next.launch_at_login = patch.launch_at_login.unwrap_or(next.launch_at_login);
        Ok(next)
    }
}

/// What a change of setting has to do outside the database: the system's login item and the
/// global shortcut.
pub struct Effects<'a> {
    pub login_item: &'a dyn LoginItem,
    pub shortcuts: &'a dyn ShortcutRegistrar,
    pub shortcut_status: &'a Mutex<ShortcutStatus>,
}

impl Effects<'_> {
    /// Brings the system in line with `to`, for the fields that differ from `from`. All or
    /// nothing: when the shortcut is refused, the login item goes back to where it was.
    fn apply(&self, from: &Settings, to: &Settings) -> Result<(), AppError> {
        let login_changed = from.launch_at_login != to.launch_at_login;
        if login_changed {
            self.login_item
                .set_enabled(to.launch_at_login)
                .map_err(|_| {
                    AppError::new("login_item_failed", "Launch at login could not be changed.")
                })?;
        }
        if from.quick_panel_shortcut != to.quick_panel_shortcut
            && let Err(error) = shortcuts::switch(
                self.shortcuts,
                self.shortcut_status,
                &to.quick_panel_shortcut,
            )
        {
            if login_changed && self.login_item.set_enabled(from.launch_at_login).is_err() {
                tracing::warn!("the login item could not be put back after a refused shortcut");
            }
            return Err(error.into());
        }
        Ok(())
    }
}

/// Validates `patch`, stores the result, then makes it live: the shared settings, the history
/// limit, and a `settings://changed` event with the full settings. When the history size
/// changed, the history is trimmed at once and `clips://changed` lists what went.
pub fn update(
    db: &Db,
    current: &RwLock<Settings>,
    limit: &HistoryLimit,
    events: &dyn EventSink,
    effects: &Effects<'_>,
    patch: SettingsPatch,
) -> Result<Settings, AppError> {
    // Held until `settings://changed` so two updates cannot interleave their writes and their
    // events, then released: `get_settings` runs on the main thread and must not wait for the
    // trim. The trim reads only `limit` and is idempotent, so interleaved trims still end right.
    let mut guard = current.write().unwrap_or_else(PoisonError::into_inner);
    let next = guard.apply(patch)?;
    // Before the write: a refusal must leave both the system and the stored value as they were.
    effects.apply(&guard, &next)?;
    db.with_conn(|conn| storage::settings::save(conn, &next))?;
    let history_size_changed = next.history_size != guard.history_size;
    *guard = next.clone();
    limit.set(next.history_limit());
    events.emit(SETTINGS_CHANGED, json!(next));
    drop(guard);
    if history_size_changed {
        trim_history(db, limit, events);
    }
    Ok(next)
}

/// Applies the new limit to the clips already stored. The settings are saved by now, so a trim
/// that fails is logged by code and the next capture trims again, as in the watcher.
fn trim_history(db: &Db, limit: &HistoryLimit, events: &dyn EventSink) {
    match db.with_conn(|conn| enforce_limit(conn, limit.get())) {
        Ok(removed) if !removed.is_empty() => {
            let changed = ClipsChanged {
                removed: Some(removed),
                ..ClipsChanged::default()
            };
            events.emit(CLIPS_CHANGED, json!(changed));
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(code = %AppError::from(error).code, "the history could not be trimmed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patch(history_size: u32) -> SettingsPatch {
        SettingsPatch {
            history_size: Some(history_size),
            ..SettingsPatch::default()
        }
    }

    #[test]
    fn the_defaults_match_the_spec() {
        assert_eq!(
            Settings::default(),
            Settings {
                history_size: 500,
                confirm_before_delete: true,
                launch_at_login: false,
                quick_panel_shortcut: "Cmd+Shift+V".to_owned(),
            }
        );
    }

    #[test]
    fn settings_serialize_to_camel_case() {
        assert_eq!(
            json!(Settings::default()),
            json!({
                "historySize": 500,
                "confirmBeforeDelete": true,
                "launchAtLogin": false,
                "quickPanelShortcut": "Cmd+Shift+V",
            })
        );
    }

    #[test]
    fn a_patch_reads_missing_fields_as_unset() {
        let parsed: SettingsPatch =
            serde_json::from_value(json!({ "launchAtLogin": true })).unwrap();

        assert_eq!(
            parsed,
            SettingsPatch {
                launch_at_login: Some(true),
                ..SettingsPatch::default()
            }
        );
    }

    #[test]
    fn a_partial_patch_changes_only_its_fields() {
        let settings = Settings::default();

        let updated = settings
            .apply(SettingsPatch {
                confirm_before_delete: Some(false),
                ..SettingsPatch::default()
            })
            .unwrap();

        assert_eq!(
            updated,
            Settings {
                confirm_before_delete: false,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn an_empty_patch_changes_nothing() {
        let settings = Settings::default();

        assert_eq!(settings.apply(SettingsPatch::default()), Ok(settings));
    }

    #[test]
    fn apply_does_not_modify_the_original() {
        let settings = Settings::default();

        settings.apply(patch(20)).unwrap();

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn a_history_size_from_one_to_nine_is_refused() {
        for size in 1..=9 {
            assert_eq!(
                Settings::default().apply(patch(size)),
                Err(SettingsError::HistorySize),
                "{size}"
            );
        }
    }

    #[test]
    fn zero_and_ten_are_valid_history_sizes() {
        for size in [0, 10, 500] {
            let updated = Settings::default().apply(patch(size)).unwrap();

            assert_eq!(updated.history_size, size);
        }
    }

    #[test]
    fn an_empty_shortcut_is_refused() {
        for shortcut in ["", "   "] {
            let result = Settings::default().apply(SettingsPatch {
                quick_panel_shortcut: Some(shortcut.to_owned()),
                ..SettingsPatch::default()
            });

            assert_eq!(
                result,
                Err(SettingsError::QuickPanelShortcut),
                "{shortcut:?}"
            );
        }
    }

    #[test]
    fn a_refused_patch_applies_none_of_its_fields() {
        let result = Settings::default().apply(SettingsPatch {
            history_size: Some(5),
            launch_at_login: Some(true),
            ..SettingsPatch::default()
        });

        assert_eq!(result, Err(SettingsError::HistorySize));
    }

    // --- side effects of `update` ----------------------------------------------------------

    use crate::crypto::FakeKeyStore;
    use crate::events::RecordingEventSink;
    use crate::login_item::FakeLoginItem;
    use crate::shortcuts::{FakeShortcutRegistrar, register_at_startup};
    use crate::storage::open_in_dir;

    struct Harness {
        db: Db,
        current: RwLock<Settings>,
        limit: HistoryLimit,
        events: RecordingEventSink,
        login_item: FakeLoginItem,
        shortcuts: FakeShortcutRegistrar,
        shortcut_status: Mutex<ShortcutStatus>,
        _dir: tempfile::TempDir,
    }

    /// Defaults stored, the default shortcut registered, a login item that is off.
    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let shortcuts = FakeShortcutRegistrar::default();
        let shortcut_status = Mutex::new(ShortcutStatus::default());
        register_at_startup(&shortcuts, &shortcut_status, DEFAULT_QUICK_PANEL_SHORTCUT);
        Harness {
            db,
            current: RwLock::new(Settings::default()),
            limit: HistoryLimit::default(),
            events: RecordingEventSink::default(),
            login_item: FakeLoginItem::new(false),
            shortcuts,
            shortcut_status,
            _dir: dir,
        }
    }

    impl Harness {
        fn update(&self, patch: SettingsPatch) -> Result<Settings, AppError> {
            update(
                &self.db,
                &self.current,
                &self.limit,
                &self.events,
                &Effects {
                    login_item: &self.login_item,
                    shortcuts: &self.shortcuts,
                    shortcut_status: &self.shortcut_status,
                },
                patch,
            )
        }

        fn live(&self) -> Settings {
            self.current.read().unwrap().clone()
        }

        fn stored(&self) -> Settings {
            self.db.with_conn(crate::storage::settings::load).unwrap()
        }

        fn announced(&self) -> usize {
            self.events
                .events()
                .iter()
                .filter(|(name, _)| *name == SETTINGS_CHANGED)
                .count()
        }
    }

    fn launch(on: bool) -> SettingsPatch {
        SettingsPatch {
            launch_at_login: Some(on),
            ..SettingsPatch::default()
        }
    }

    fn shortcut(accelerator: &str) -> SettingsPatch {
        SettingsPatch {
            quick_panel_shortcut: Some(accelerator.to_owned()),
            ..SettingsPatch::default()
        }
    }

    #[test]
    fn turning_launch_at_login_on_enables_the_login_item_and_stores_it() {
        let h = harness();

        let updated = h.update(launch(true)).unwrap();

        assert!(updated.launch_at_login);
        assert!(h.login_item.enabled());
        assert!(h.stored().launch_at_login);
        assert!(h.live().launch_at_login);
    }

    #[test]
    fn turning_launch_at_login_off_disables_the_login_item() {
        let h = harness();
        h.update(launch(true)).unwrap();
        assert!(h.login_item.enabled());

        h.update(launch(false)).unwrap();

        assert!(!h.login_item.enabled());
        assert!(!h.stored().launch_at_login);
    }

    #[test]
    fn a_failing_login_item_refuses_the_change_and_keeps_the_setting() {
        let h = harness();
        h.login_item.fail();

        let error = h.update(launch(true)).unwrap_err();

        assert_eq!(error.code, "login_item_failed");
        assert!(!h.live().launch_at_login);
        assert!(!h.stored().launch_at_login);
        assert_eq!(h.announced(), 0);
    }

    #[test]
    fn a_patch_that_leaves_launch_at_login_alone_does_not_touch_the_login_item() {
        let h = harness();
        h.login_item.fail();

        let result = h.update(SettingsPatch {
            confirm_before_delete: Some(false),
            ..SettingsPatch::default()
        });

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(h.login_item.changes(), 0);
    }

    #[test]
    fn sending_the_stored_launch_at_login_again_does_not_touch_the_login_item() {
        let h = harness();

        h.update(launch(false)).unwrap();

        assert_eq!(h.login_item.changes(), 0);
    }

    #[test]
    fn a_new_shortcut_is_registered_and_the_old_one_released() {
        let h = harness();

        let updated = h.update(shortcut("Ctrl+Alt+V")).unwrap();

        assert_eq!(updated.quick_panel_shortcut, "Ctrl+Alt+V");
        assert!(h.shortcuts.is_registered("Ctrl+Alt+V"));
        assert!(!h.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(h.stored().quick_panel_shortcut, "Ctrl+Alt+V");
        let status = h.shortcut_status.lock().unwrap().clone();
        assert_eq!(status.accelerator, "Ctrl+Alt+V");
        assert_eq!(status.error, None);
    }

    #[test]
    fn a_shortcut_the_system_refuses_keeps_the_old_one_and_the_stored_value() {
        let h = harness();
        h.shortcuts.take("Ctrl+Alt+V");

        let error = h.update(shortcut("Ctrl+Alt+V")).unwrap_err();

        assert_eq!(error.code, "shortcut_unavailable");
        assert!(h.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(h.live().quick_panel_shortcut, DEFAULT_QUICK_PANEL_SHORTCUT);
        assert_eq!(
            h.stored().quick_panel_shortcut,
            DEFAULT_QUICK_PANEL_SHORTCUT
        );
        assert_eq!(
            *h.shortcut_status.lock().unwrap(),
            ShortcutStatus::default()
        );
        assert_eq!(h.announced(), 0);
    }

    #[test]
    fn a_shortcut_that_does_not_parse_is_refused_as_invalid_settings() {
        let h = harness();

        let error = h.update(shortcut("Cmd+Shift+")).unwrap_err();

        assert_eq!(error.code, "invalid_settings");
        assert!(h.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
    }

    #[test]
    fn sending_the_stored_shortcut_again_registers_nothing() {
        let h = harness();
        // Taken now: a second registration of the same accelerator would be refused.
        h.shortcuts.take(DEFAULT_QUICK_PANEL_SHORTCUT);

        let result = h.update(shortcut(DEFAULT_QUICK_PANEL_SHORTCUT));

        assert!(result.is_ok(), "{result:?}");
        assert!(h.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
    }

    #[test]
    fn sending_the_stored_shortcut_again_retries_one_that_failed_at_startup() {
        let h = harness();
        // The stored shortcut was refused at startup; the user has freed it since.
        h.shortcuts
            .unregister(DEFAULT_QUICK_PANEL_SHORTCUT)
            .unwrap();
        *h.shortcut_status.lock().unwrap() = ShortcutStatus {
            accelerator: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
            error: Some("shortcut_unavailable".to_owned()),
        };

        let result = h.update(shortcut(DEFAULT_QUICK_PANEL_SHORTCUT));

        assert!(result.is_ok(), "{result:?}");
        assert!(h.shortcuts.is_registered(DEFAULT_QUICK_PANEL_SHORTCUT));
        assert_eq!(h.shortcut_status.lock().unwrap().error, None);
    }

    #[test]
    fn a_retry_that_is_refused_again_reports_it() {
        let h = harness();
        h.shortcuts
            .unregister(DEFAULT_QUICK_PANEL_SHORTCUT)
            .unwrap();
        h.shortcuts.take(DEFAULT_QUICK_PANEL_SHORTCUT);
        *h.shortcut_status.lock().unwrap() = ShortcutStatus {
            accelerator: DEFAULT_QUICK_PANEL_SHORTCUT.to_owned(),
            error: Some("shortcut_unavailable".to_owned()),
        };

        let error = h
            .update(shortcut(DEFAULT_QUICK_PANEL_SHORTCUT))
            .unwrap_err();

        assert_eq!(error.code, "shortcut_unavailable");
    }

    /// A registrar that records whether the settings could be read while it was called, as
    /// `get_settings` does on the main thread while the real plugin waits for that same thread.
    struct ReadsSettings<'a> {
        current: &'a RwLock<Settings>,
        readable: std::sync::atomic::AtomicBool,
    }

    impl ShortcutRegistrar for ReadsSettings<'_> {
        fn register(&self, _accelerator: &str) -> Result<(), shortcuts::ShortcutError> {
            self.readable.store(
                self.current.try_read().is_ok(),
                std::sync::atomic::Ordering::SeqCst,
            );
            Ok(())
        }

        fn unregister(&self, _accelerator: &str) -> Result<(), shortcuts::ShortcutError> {
            Ok(())
        }
    }

    #[test]
    fn the_settings_stay_readable_while_the_system_effects_run() {
        let h = harness();
        let registrar = ReadsSettings {
            current: &h.current,
            readable: std::sync::atomic::AtomicBool::new(false),
        };

        update(
            &h.db,
            &h.current,
            &h.limit,
            &h.events,
            &Effects {
                login_item: &h.login_item,
                shortcuts: &registrar,
                shortcut_status: &h.shortcut_status,
            },
            shortcut("Ctrl+Alt+V"),
        )
        .unwrap();

        assert!(registrar.readable.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn a_refused_shortcut_undoes_the_login_item_change_of_the_same_patch() {
        let h = harness();
        h.shortcuts.take("Ctrl+Alt+V");

        let error = h
            .update(SettingsPatch {
                launch_at_login: Some(true),
                quick_panel_shortcut: Some("Ctrl+Alt+V".to_owned()),
                ..SettingsPatch::default()
            })
            .unwrap_err();

        assert_eq!(error.code, "shortcut_unavailable");
        assert!(!h.login_item.enabled());
        assert!(!h.stored().launch_at_login);
    }

    #[test]
    fn a_refused_patch_has_no_side_effect() {
        let h = harness();

        let error = h
            .update(SettingsPatch {
                history_size: Some(5),
                launch_at_login: Some(true),
                quick_panel_shortcut: Some("Ctrl+Alt+V".to_owned()),
                ..SettingsPatch::default()
            })
            .unwrap_err();

        assert_eq!(error.code, "invalid_settings");
        assert_eq!(h.login_item.changes(), 0);
        assert!(!h.shortcuts.is_registered("Ctrl+Alt+V"));
    }

    #[test]
    fn a_settings_error_maps_to_invalid_settings_naming_the_field() {
        let history = AppError::from(SettingsError::HistorySize);
        let shortcut = AppError::from(SettingsError::QuickPanelShortcut);

        assert_eq!(history.code, "invalid_settings");
        assert!(
            history.message.contains("historySize"),
            "{}",
            history.message
        );
        assert_eq!(shortcut.code, "invalid_settings");
        assert!(
            shortcut.message.contains("quickPanelShortcut"),
            "{}",
            shortcut.message
        );
    }
}
