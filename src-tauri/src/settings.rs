//! The user's preferences: a typed, validated object shared by Rust and the front. Only the
//! stored settings live here; applying them (autostart, shortcut registration) is the job of
//! the preferences window's ticket.

use std::sync::{PoisonError, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::clip::{DEFAULT_HISTORY_SIZE, HistoryLimit};
use crate::error::AppError;
use crate::events::{CLIPS_CHANGED, ClipsChanged, EventSink, SETTINGS_CHANGED};
use crate::shortcuts::DEFAULT_QUICK_PANEL_SHORTCUT;
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

/// Validates `patch`, stores the result, then makes it live: the shared settings, the history
/// limit, and a `settings://changed` event with the full settings. When the history size
/// changed, the history is trimmed at once and `clips://changed` lists what went.
pub fn update(
    db: &Db,
    current: &RwLock<Settings>,
    limit: &HistoryLimit,
    events: &dyn EventSink,
    patch: SettingsPatch,
) -> Result<Settings, AppError> {
    // Held until `settings://changed` so two updates cannot interleave their writes and their
    // events, then released: `get_settings` runs on the main thread and must not wait for the
    // trim. The trim reads only `limit` and is idempotent, so interleaved trims still end right.
    let mut guard = current.write().unwrap_or_else(PoisonError::into_inner);
    let next = guard.apply(patch)?;
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
