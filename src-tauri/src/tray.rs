//! The tray icon's menu model (MNU-03) and click rules (MNU-02). They are pure, so the labels,
//! the order, the enabled state and the clicks are tested without AppKit; `macos::tray` builds
//! the native menu and the status item from them.

use serde_json::Value;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};

use crate::events::{EventSink, MENU_CLEAR_HISTORY};
use crate::windows::{self, PopoverWindow};

pub const TRAY_ID: &str = "tadidy";

/// One row of the tray menu, in the order the prototype's `#tray-menu` shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayItem {
    Entry {
        id: &'static str,
        label: &'static str,
        enabled: bool,
        /// Tauri accelerator string, shown as the menu's keyboard equivalent.
        accelerator: Option<&'static str>,
    },
    Separator,
}

const fn entry(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    accelerator: Option<&'static str>,
) -> TrayItem {
    TrayItem::Entry {
        id,
        label,
        enabled,
        accelerator,
    }
}

/// Id of the About entry: the native about panel shows itself, so it has no [`TrayAction`].
pub const ABOUT_ID: &str = "about";

/// The tray menu. Check for updates… is wired by the P2 updater ticket (SYS-04): until then it
/// is shown but disabled.
pub fn menu_items() -> Vec<TrayItem> {
    vec![
        entry("preferences", "Preferences…", true, Some("CmdOrCtrl+,")),
        entry("clear-history", "Clear history…", true, None),
        TrayItem::Separator,
        entry(ABOUT_ID, "About Tadidy", true, None),
        entry("check-updates", "Check for updates…", false, None),
        TrayItem::Separator,
        entry("quit", "Quit Tadidy", true, Some("CmdOrCtrl+Q")),
    ]
}

/// What choosing a menu item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Preferences,
    ClearHistory,
    Quit,
}

pub fn action_for(id: &str) -> Option<TrayAction> {
    match id {
        "preferences" => Some(TrayAction::Preferences),
        "clear-history" => Some(TrayAction::ClearHistory),
        "quit" => Some(TrayAction::Quit),
        _ => None,
    }
}

/// Clear history… (HIS-06): shows the popover, without toggling it, and asks its webview to open
/// the confirmation. The history is cleared by the webview's `clear_history` call, only once the
/// user confirmed.
pub fn request_clear_history(popover: &impl PopoverWindow, sink: &impl EventSink) {
    windows::show(popover);
    sink.emit(MENU_CLEAR_HISTORY, Value::Null);
}

/// A left click acts when the button is released (MNU-02). The right click opens the native
/// menu by itself, so it is not handled here.
pub fn toggles_the_popover(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RecordingEventSink;
    use serde_json::json;

    fn labels(items: &[TrayItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                TrayItem::Entry { label, .. } => (*label).to_owned(),
                TrayItem::Separator => "-".to_owned(),
            })
            .collect()
    }

    #[test]
    fn menu_lists_the_prototype_items_in_order() {
        let items = menu_items();

        assert_eq!(
            labels(&items),
            [
                "Preferences…",
                "Clear history…",
                "-",
                "About Tadidy",
                "Check for updates…",
                "-",
                "Quit Tadidy",
            ]
        );
        let ids: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                TrayItem::Entry { id, .. } => Some(*id),
                TrayItem::Separator => None,
            })
            .collect();
        assert_eq!(
            ids,
            [
                "preferences",
                "clear-history",
                "about",
                "check-updates",
                "quit"
            ]
        );
        let accelerators: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                TrayItem::Entry {
                    id, accelerator, ..
                } => accelerator.map(|accelerator| (*id, accelerator)),
                TrayItem::Separator => None,
            })
            .collect();
        assert_eq!(
            accelerators,
            [("preferences", "CmdOrCtrl+,"), ("quit", "CmdOrCtrl+Q")]
        );
    }

    #[test]
    fn all_mvp_items_enabled_except_updates() {
        let enabled: Vec<_> = menu_items()
            .into_iter()
            .filter_map(|item| match item {
                TrayItem::Entry {
                    id, enabled: true, ..
                } => Some(id),
                _ => None,
            })
            .collect();

        assert_eq!(enabled, ["preferences", "clear-history", "about", "quit"]);
    }

    #[test]
    fn menu_ids_map_to_actions() {
        assert_eq!(action_for("preferences"), Some(TrayAction::Preferences));
        assert_eq!(action_for("clear-history"), Some(TrayAction::ClearHistory));
        assert_eq!(action_for("quit"), Some(TrayAction::Quit));
        // The about panel is native and Check for updates… waits for the P2 updater: no action.
        for id in ["about", "check-updates", "?"] {
            assert_eq!(action_for(id), None, "{id}");
        }
    }

    #[test]
    fn clear_history_shows_the_popover_then_asks_for_the_dialog() {
        let popover = FakePopover::hidden();
        let sink = RecordingEventSink::default();

        request_clear_history(&popover, &sink);

        assert_eq!(popover.reveals.get(), 1);
        assert_eq!(sink.events(), vec![(MENU_CLEAR_HISTORY, json!(null))]);
    }

    #[test]
    fn clear_history_leaves_a_visible_popover_open() {
        let popover = FakePopover::shown();
        let sink = RecordingEventSink::default();

        request_clear_history(&popover, &sink);

        assert_eq!(popover.reveals.get(), 0);
        assert_eq!(popover.conceals.get(), 0);
        assert_eq!(sink.events(), vec![(MENU_CLEAR_HISTORY, json!(null))]);
    }

    struct FakePopover {
        shown: bool,
        reveals: std::cell::Cell<u32>,
        conceals: std::cell::Cell<u32>,
    }

    impl FakePopover {
        fn hidden() -> Self {
            Self {
                shown: false,
                reveals: std::cell::Cell::default(),
                conceals: std::cell::Cell::default(),
            }
        }

        fn shown() -> Self {
            Self {
                shown: true,
                ..Self::hidden()
            }
        }
    }

    impl PopoverWindow for FakePopover {
        fn is_shown(&self) -> bool {
            self.shown
        }

        fn reveal(&self) {
            self.reveals.set(self.reveals.get() + 1);
        }

        fn conceal(&self) {
            self.conceals.set(self.conceals.get() + 1);
        }
    }

    fn click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: tauri::tray::TrayIconId::new(TRAY_ID),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect: tauri::Rect::default(),
            button,
            button_state,
        }
    }

    #[test]
    fn a_left_click_release_toggles_the_popover() {
        assert!(toggles_the_popover(&click(
            MouseButton::Left,
            MouseButtonState::Up
        )));
    }

    #[test]
    fn other_clicks_do_not_toggle_the_popover() {
        // The press is ignored (the release acts), and the right click opens the menu.
        for (button, state) in [
            (MouseButton::Left, MouseButtonState::Down),
            (MouseButton::Right, MouseButtonState::Up),
            (MouseButton::Right, MouseButtonState::Down),
        ] {
            assert!(
                !toggles_the_popover(&click(button, state)),
                "{button:?} {state:?}"
            );
        }
    }

    #[test]
    fn pointer_movement_does_not_toggle_the_popover() {
        let event = TrayIconEvent::Leave {
            id: tauri::tray::TrayIconId::new(TRAY_ID),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect: tauri::Rect::default(),
        };

        assert!(!toggles_the_popover(&event));
    }
}
