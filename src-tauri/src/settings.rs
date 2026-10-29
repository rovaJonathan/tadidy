//! The user's preferences: a typed, validated object shared by Rust and the front. Only the
//! stored settings live here; applying them (autostart, shortcut registration) is the job of
//! the preferences window's ticket.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

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
    fn from(_error: SettingsError) -> Self {
        todo!()
    }
}

impl Default for Settings {
    fn default() -> Self {
        todo!()
    }
}

impl Settings {
    /// The settings after `patch`, or the first rule it breaks; `self` is left untouched.
    pub fn apply(&self, _patch: SettingsPatch) -> Result<Settings, SettingsError> {
        todo!()
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
